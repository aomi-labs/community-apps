#!/usr/bin/env python3
"""Typed deployment record and the release build for one of its apps.

`deploy_project.py` owns the lifecycle; this module owns the record both share
and the `cargo build` that turns one app into `plugins/manifest.json`, a
tarball and `aomi-release.json` under `dist/<release-tag>/`.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, NoReturn, NotRequired, TypedDict


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
APP_RE = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
# Backend deploys are repo-keyed: apps/<installation>/<repo-key>/<app>.
REPO_KEY_RE = re.compile(r"^r[0-9a-f]{10}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{7,40}$")
SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
# SDK releases that predate the compiled `ffi_version` field but speak FFI v1.
IMPLICIT_FFI = {"5.1.1": 1, "5.1.2": 1}
SUPPORTED_INTERFACES = frozenset({"async_tools_v1", "routes_v1", "app_skills_v1", "secret_slots_v1", "evm_sponsored_4337_v1"})


def fail(message: str) -> NoReturn:
    raise SystemExit(f"error: {message}")


def positive_int(value: object) -> int | None:
    """`value` when it is a positive int proper; bool is an int subclass and is excluded."""
    return value if type(value) is int and value > 0 else None


def implicit_ffi(sdk_version: str | None) -> int | None:
    return None if sdk_version is None else IMPLICIT_FFI.get(sdk_version)


def run_capture(cmd: list[str], *, cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess[str]:
    """Run `cmd` and hand back the result so the caller can classify a failure."""
    return subprocess.run(cmd, cwd=cwd or REPO_ROOT, env=os.environ.copy(), text=True, capture_output=True)


def run(cmd: list[str], *, cwd: pathlib.Path | None = None, capture: bool = True) -> str:
    result = run_capture(cmd, cwd=cwd)
    if not capture:
        print(result.stdout or "", end="")
        print(result.stderr or "", end="", file=sys.stderr)
    if result.returncode != 0:
        command = " ".join(cmd)
        detail = (result.stderr or "")[-6000:].strip()
        fail(f"command failed ({command}): {detail}")
    return result.stdout.strip() if capture and result.stdout else ""


def write_json(path: pathlib.Path, value: Mapping[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def sha256_file(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relpath(path: pathlib.Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def normalize_repo(value: str) -> str:
    repo = value.strip().removesuffix(".git").strip("/")
    for prefix in [
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
        "http://github.com/",
        "github.com/",
    ]:
        if repo.startswith(prefix):
            repo = repo.removeprefix(prefix)
            break
    return repo.lower()


def lib_ext(target: str) -> str:
    if "linux" in target:
        return "so"
    if "apple" in target or "darwin" in target:
        return "dylib"
    if "windows" in target:
        return "dll"
    fail(f"unsupported target triple: {target}")


class PluginEntry(TypedDict):
    file: str
    sha256: str
    secrets: NotRequired[list[object]]


class BundleManifest(TypedDict):
    """`plugins/manifest.json`: written by the build, re-read and compared by the publisher."""

    app_release_tag: str
    sdk_version: str
    target: str
    commit: str
    plugins: dict[str, PluginEntry]
    ffi_version: NotRequired[int]
    required_interfaces: NotRequired[list[str]]


@dataclass(frozen=True)
class CargoPackage:
    """The app crate as declared by its Cargo.toml."""

    manifest_path: pathlib.Path
    name: str
    lib_name: str

    @classmethod
    def load(cls, app_dir: pathlib.Path) -> CargoPackage:
        manifest_path = app_dir / "Cargo.toml"
        if not manifest_path.is_file():
            fail(f"{relpath(app_dir)} must contain Cargo.toml")
        data = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        package = data.get("package")
        if not isinstance(package, dict) or not isinstance(package.get("name"), str):
            fail(f"{relpath(manifest_path)} must define [package].name")
        name = package["name"].strip()
        lib = data.get("lib")
        if not isinstance(lib, dict):
            lib = {}
        if "cdylib" not in lib.get("crate-type", []):
            fail(f"{relpath(manifest_path)} must set [lib].crate-type = [\"cdylib\"]")
        return cls(manifest_path.resolve(), name, str(lib.get("name") or name))

    def sdk_version(self) -> str:
        """The aomi-sdk release the crate depends on directly, per the lockfile."""
        output = run(
            ["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(self.manifest_path)],
            cwd=self.manifest_path.parent,
        )
        metadata = json.loads(output)
        packages = {pkg["id"]: pkg for pkg in metadata.get("packages", [])}
        root_pkg = next(
            (pkg for pkg in packages.values() if pathlib.Path(pkg["manifest_path"]).resolve() == self.manifest_path), None
        )
        if root_pkg is None:
            fail(f"cargo metadata did not include root package for {self.manifest_path}")
        nodes = {node["id"]: node for node in metadata.get("resolve", {}).get("nodes", [])}
        root_node = nodes.get(root_pkg["id"])
        if root_node is None:
            fail("cargo metadata did not include the app dependency graph")
        for dep in root_node.get("deps", []):
            package = packages.get(dep.get("pkg"))
            if package and package.get("name") == "aomi-sdk":
                return str(package["version"])
        fail("app must depend directly on aomi-sdk")

    def built_lib(self, release_dir: pathlib.Path, target: str) -> pathlib.Path:
        normalized = self.lib_name.replace("-", "_")
        ext = lib_ext(target)
        return release_dir / (f"{normalized}.dll" if ext == "dll" else f"lib{normalized}.{ext}")


# NOTE: this is a stderr-text heuristic, not a structural check -- clap gives no
# machine-readable "unknown subcommand" signal, and the aomi-sdk version that
# introduced `manifest` is not pinned yet. Once it is, gate on `sdk_version`.
# TODO(manifest-min-version): replace this heuristic with a version comparison.
def is_unsupported_manifest_error(stderr: str) -> bool:
    """True iff clap rejected the `manifest` SUBCOMMAND itself as unknown.

    Only the quoted/named token `manifest` counts: a flag-level rejection on an
    SDK that does have the subcommand (`unexpected argument '--lib' found`) is a
    real manifest-contract bug and must fail the build, as must any other
    non-zero exit (panic, network blip, timeout).
    """
    lowered = stderr.lower()
    if "unrecognized subcommand" in lowered and "manifest" in lowered:
        return True
    if "no such subcommand" in lowered and "manifest" in lowered:
        return True
    # clap v4 may report an unregistered subcommand as an unexpected argument.
    if "unexpected argument" in lowered and "'manifest'" in lowered:
        return True
    # clap v2/v3: "Found argument 'manifest' which wasn't expected".
    if "wasn't expected" in lowered and "'manifest'" in lowered:
        return True
    return False


@dataclass(frozen=True)
class PluginManifest:
    """`aomi-build manifest` output for a compiled plugin, checked against the SDK it was built with."""

    ffi_version: int | None
    required_interfaces: list[str]
    secrets: list[object]

    @classmethod
    def read(cls, plugin_path: pathlib.Path, sdk_version: str) -> PluginManifest | None:
        """Install the app's exact pinned aomi-sdk so the reader matches the plugin ABI.

        Returns None ONLY when the installed SDK predates the `manifest`
        subcommand -- the sole legitimate reason a build may ship without
        secret metadata. Any other failure must fail the build rather than
        publish a release with no secret metadata, which would permanently
        ungate an app that declares required secrets.
        """
        bin_root = pathlib.Path(tempfile.mkdtemp(prefix="aomi-build-cli-"))
        # An older SDK still installs fine; it merely lacks the subcommand (below).
        run([
            "cargo", "install", "aomi-sdk",
            "--version", f"={sdk_version}",
            "--features", "cli", "--bin", "aomi-build",
            "--locked", "--root", str(bin_root),
        ])
        result = run_capture([str(bin_root / "bin" / "aomi-build"), "manifest", "--lib", str(plugin_path)])
        if result.returncode != 0:
            stderr = (result.stderr or "").strip()
            if is_unsupported_manifest_error(stderr):
                print(
                    f"::notice::{plugin_path.name}: installed aomi-sdk {sdk_version} has no "
                    "`manifest` subcommand (predates secret-declaration support); "
                    "this app cannot be secret-gated"
                )
                return None
            fail(
                f"aomi-build manifest failed for {plugin_path.name} (sdk {sdk_version}): "
                f"{stderr or 'no stderr captured'}"
            )
        return cls.parse((result.stdout or "").strip(), plugin_path.name, sdk_version)

    @classmethod
    def parse(cls, output: str, plugin: str, sdk_version: str) -> PluginManifest:
        """Validate the untrusted JSON; semver ranges do not establish compatibility."""
        try:
            data = json.loads(output)
        except json.JSONDecodeError as err:
            fail(f"aomi-build manifest for {plugin} (sdk {sdk_version}) produced invalid JSON: {err}")
        if not isinstance(data, dict):
            fail(f"aomi-build manifest for {plugin} (sdk {sdk_version}) returned {type(data).__name__}, expected a JSON object")
        secrets = data.get("secrets")
        if secrets is not None and not isinstance(secrets, list):
            fail(
                f"aomi-build manifest for {plugin} (sdk {sdk_version}): \"secrets\" "
                f"field is {type(secrets).__name__}, expected a list"
            )
        if data.get("sdk_version") != sdk_version:
            fail("Compiled manifest SDK does not match the resolved dependency")
        ffi_version: int | None = None
        declared = data.get("ffi_version", implicit_ffi(sdk_version))
        if declared is not None:
            ffi_version = positive_int(declared)
            if ffi_version is None:
                fail("Compiled manifest ffi_version must be a positive integer")
        interfaces = data.get("required_interfaces", [])
        if not isinstance(interfaces, list) or any(not isinstance(item, str) or item not in SUPPORTED_INTERFACES for item in interfaces):
            fail("Compiled manifest requires an unsupported FFI interface")
        return cls(ffi_version, interfaces, secrets or [])


@dataclass(frozen=True)
class App:
    """One `platform.apps[]` entry of a deployment record."""

    name: str
    path: str
    aomi_toml_path: str
    release_tag: str
    target: str | None
    # Staged-file list as recorded; verified against the source snapshot by `verify_files`.
    files: object

    @classmethod
    def from_json(cls, data: dict[str, Any]) -> App:
        return cls(data["name"], data["path"], data["aomi_toml_path"], data["release_tag"], data.get("target"), data.get("files"))

    def verify_files(self, app_dir: pathlib.Path) -> None:
        files = self.files
        if not isinstance(files, list) or not files:
            fail("deployment manifest files must be a non-empty array")
        seen: set[str] = set()
        for entry in files:
            if not isinstance(entry, dict):
                fail("deployment manifest files entries must be objects")
            path = entry.get("path")
            digest = entry.get("sha256")
            byte_count = entry.get("bytes")
            if not isinstance(path, str) or path.startswith("/"):
                fail(f"invalid staged file path: {path!r}")
            rel = pathlib.PurePosixPath(path)
            if any(part in {"", ".", ".."} for part in rel.parts):
                fail(f"invalid staged file path: {path!r}")
            if path in seen:
                fail(f"duplicate staged file path: {path}")
            seen.add(path)
            file_path = app_dir / path
            if not file_path.is_file():
                fail(f"staged file missing from app directory: {path}")
            if not isinstance(digest, str) or not SHA256_RE.match(digest):
                fail(f"invalid sha256 for staged file {path}")
            if digest != f"sha256:{sha256_file(file_path)}":
                fail(f"staged file sha256 mismatch for {path}")
            if byte_count != file_path.stat().st_size:
                fail(f"staged file byte count mismatch for {path}")


@dataclass(frozen=True)
class Deployment:
    """A platform deployment record (`deployment.json`), parsed once from the backend's JSON."""

    id: str
    sdk_version: str | None
    ffi_version: int | None
    project_id: int
    installation_id: str
    owner_repo: str
    source_commit: str
    platform: str
    repository: str
    branch: str
    short_commit: str
    candidate_commit: str
    apps: tuple[App, ...]

    @classmethod
    def load(cls, path: pathlib.Path) -> Deployment:
        return cls.from_json(json.loads(path.read_text()))

    @classmethod
    def from_json(cls, data: dict[str, Any]) -> Deployment:
        source, platform = data["source"], data["platform"]
        branch = platform["platform_branch"]
        return cls(
            id=data["id"],
            sdk_version=data.get("sdk_version"),
            ffi_version=data.get("ffi_version"),
            project_id=source["project_id"],
            installation_id=str(source["installation_id"]),
            owner_repo=normalize_repo(source["repository_link"]),
            source_commit=source["commit_hash"],
            platform=platform["platform"],
            repository=platform["repository"],
            branch=branch,
            # The deployment's branch is the canonical authority for short SHA length.
            short_commit=branch.rsplit("/", 1)[1],
            candidate_commit=platform["commit_hash"],
            apps=tuple(App.from_json(app) for app in platform["apps"]),
        )

    def accepts(self, bundle: BundleManifest) -> bool:
        """A pinned FFI version decides; without one the SDK release must match exactly."""
        if self.ffi_version is None:
            return bool(self.sdk_version) and bundle.get("sdk_version") == self.sdk_version
        expected = positive_int(self.ffi_version)
        actual = bundle.get("ffi_version", implicit_ffi(bundle.get("sdk_version")))
        return expected is not None and positive_int(actual) == expected

    def build_release(self, app: App, app_dir: pathlib.Path, target: str, dist_root: pathlib.Path, source_dir: pathlib.Path) -> None:
        """Compile `app` from `source_dir` and stage its bundle under `dist_root/<release-tag>`.

        `app_dir` is the app's slot in the platform repository and holds the
        deployment manifest the release records as provenance.
        """
        parts = app_dir.relative_to(REPO_ROOT).parts
        if len(parts) != 4 or parts[0] != "apps" or not REPO_KEY_RE.match(parts[2]):
            fail(f"candidate app dir must be apps/<installation-id>/<repo-key>/<app>, got {relpath(app_dir)}")
        installation_id, repo_key, app_name = parts[1], parts[2], parts[3]
        if installation_id != self.installation_id:
            fail(f"{relpath(app_dir)} installation id does not match branch")
        if not APP_RE.match(app_name):
            fail(f"invalid app directory name: {app_name}")
        app_path = f"apps/{installation_id}/{repo_key}/{app_name}"
        matches = [entry for entry in self.apps if entry.name.lower() == app_name and entry.path == app_path]
        if len(matches) != 1:
            fail(f"deployment manifest must contain one platform.apps entry for {app_path}")
        app = matches[0]
        if not COMMIT_RE.match(self.source_commit) or not self.source_commit.startswith(self.short_commit):
            fail("deployment manifest source commit does not match branch short commit")
        release_tag = app.release_tag
        expected_tag = f"apps-{installation_id}-{repo_key}-{app_name}-{self.short_commit}"
        if release_tag != expected_tag:
            fail(f"deployment manifest release_tag must be {expected_tag}")
        if app.target and app.target != target:
            fail(f"deployment manifest target must be {target}")
        app.verify_files(source_dir)

        package = CargoPackage.load(source_dir)
        sdk_version = package.sdk_version()
        if self.ffi_version is None and self.sdk_version and sdk_version != self.sdk_version:
            fail(f"{app_name}: aomi-sdk {sdk_version} does not match required {self.sdk_version}")

        target_dir = REPO_ROOT / ".aomi-ci-target"
        run(
            [
                "cargo",
                "build",
                "--locked",
                "--lib",
                "--release",
                "--target",
                target,
                "--target-dir",
                str(target_dir),
                "--manifest-path",
                str(source_dir / "Cargo.toml"),
            ],
            capture=False,
            cwd=source_dir,
        )
        built_lib = package.built_lib(target_dir / target / "release", target)
        if not built_lib.is_file():
            fail(f"expected built library not found: {built_lib}")

        dist_dir = dist_root / release_tag
        if dist_dir.exists():
            shutil.rmtree(dist_dir)
        plugins_dir = dist_dir / "plugins"
        plugins_dir.mkdir(parents=True)
        final_plugin = plugins_dir / f"{app_name.replace('-', '_')}.{lib_ext(target)}"
        shutil.copy2(built_lib, final_plugin)
        entry: PluginEntry = {"file": final_plugin.name, "sha256": sha256_file(final_plugin)}
        plugin = PluginManifest.read(final_plugin, sdk_version)
        ffi_version = None if plugin is None else plugin.ffi_version
        expected_ffi = self.ffi_version
        if expected_ffi is not None and (positive_int(expected_ffi) is None or ffi_version != expected_ffi):
            fail(f"{app_name}: compiled ffi_version {ffi_version} does not match required {expected_ffi}")
        if plugin is not None and plugin.secrets:
            entry["secrets"] = plugin.secrets
        bundle: BundleManifest = {
            "app_release_tag": release_tag,
            "sdk_version": sdk_version,
            "target": target,
            "commit": self.source_commit,
            "plugins": {app_name: entry},
        }
        if ffi_version is not None:
            bundle["ffi_version"] = ffi_version
        if plugin is not None and plugin.required_interfaces:
            bundle["required_interfaces"] = plugin.required_interfaces
        manifest_path = plugins_dir / "manifest.json"
        write_json(manifest_path, bundle)

        tarball = dist_dir / f"aomi-plugins-{release_tag}-{target}.tar.gz"
        with tarfile.open(tarball, "w:gz") as archive:
            archive.add(manifest_path, arcname="plugins/manifest.json")
            archive.add(final_plugin, arcname=f"plugins/{final_plugin.name}")
        with tempfile.TemporaryDirectory(prefix="aomi-candidate-") as tmp:
            tmp_path = pathlib.Path(tmp)
            with tarfile.open(tarball, "r:gz") as archive:
                archive.extractall(tmp_path, filter="data")
            packed_path = tmp_path / "plugins" / "manifest.json"
            try:
                packed = json.loads(packed_path.read_text(encoding="utf-8"))
            except FileNotFoundError:
                fail(f"missing JSON file: {packed_path}")
            except json.JSONDecodeError as err:
                fail(f"invalid JSON in {packed_path}: {err}")
            if not isinstance(packed, dict):
                fail(f"{packed_path} must contain a JSON object")
            if packed != bundle:
                fail("tarball manifest does not match generated manifest")
            for name, packed_entry in bundle["plugins"].items():
                packed_plugin = tmp_path / "plugins" / packed_entry["file"]
                if not packed_plugin.is_file():
                    fail(f"tarball plugin missing for {name}")
                if sha256_file(packed_plugin) != packed_entry["sha256"]:
                    fail(f"tarball plugin checksum mismatch for {name}")

        shutil.copy2(manifest_path, dist_dir / "manifest.json")
        write_json(
            dist_dir / "aomi-release.json",
            {
                "schema_version": 1,
                "platform": self.platform,
                "app": {
                    "name": app_name,
                    "path": app_path,
                    "package": package.name,
                },
                "source": {
                    "repository": self.owner_repo,
                    "installation_id": int(installation_id),
                    "commit": self.source_commit,
                },
                "candidate": {
                    "branch": self.branch,
                    "commit": self.candidate_commit,
                    "deployment_manifest": relpath(app_dir / ".aomi" / "deployment.json"),
                },
                "release": {
                    "tag": release_tag,
                    "target": target,
                    "sdk_version": sdk_version,
                },
                "assets": {
                    "tarball": tarball.name,
                    "manifest": "manifest.json",
                    "metadata": "aomi-release.json",
                },
            },
        )
        (dist_dir / "release-notes.md").write_text(
            "\n".join(
                [
                    f"Aomi {self.platform} candidate release.",
                    "",
                    f"- App: {app_name}",
                    f"- Release: {release_tag}",
                    f"- Source: {self.owner_repo}@{self.source_commit}",
                    f"- Candidate: {self.branch}@{self.candidate_commit}",
                    f"- SDK: {sdk_version}",
                    f"- Target: {target}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
