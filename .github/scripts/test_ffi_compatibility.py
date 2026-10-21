from dataclasses import replace
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import build_candidate as build
import deploy_project as deploy


def compiled_ffi(manifest, sdk_version):
    return build.PluginManifest.parse(json.dumps(manifest), "demo.so", sdk_version).ffi_version


class FfiCompatibility(unittest.TestCase):
    def test_compiled_metadata_is_independent_of_sdk_release(self):
        self.assertEqual(compiled_ffi({"sdk_version": "99.0.0", "ffi_version": 1}, "99.0.0"), 1)
        self.assertEqual(compiled_ffi({"sdk_version": "5.1.1"}, "5.1.1"), 1)
        self.assertIsNone(compiled_ffi({"sdk_version": "3.0.1"}, "3.0.1"))
        for value in (True, "1", 0, -1):
            with self.subTest(value=value), self.assertRaises(SystemExit):
                compiled_ffi({"sdk_version": "99.0.0", "ffi_version": value}, "99.0.0")
        with self.assertRaises(SystemExit):
            compiled_ffi({"sdk_version": "other", "ffi_version": 1}, "99.0.0")
        with self.assertRaises(SystemExit):
            compiled_ffi({"sdk_version": "99.0.0", "ffi_version": 1,
                          "required_interfaces": ["future_permission_v2"]}, "99.0.0")

    def test_publisher_accepts_cross_sdk_bundle_but_preserves_integrity_gates(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = b"compiled fixture"
            app = build.App.from_json({"name": "demo", "release_tag": "apps-fixture", "path": "apps/12/r0123456789/demo", "aomi_toml_path": "aomi.toml"})
            deployment = build.Deployment.from_json({
                "id": "dep_12_r0123456789_" + "a" * 12, "sdk_version": "6.0.0", "ffi_version": 1,
                "source": {"project_id": 42, "commit_hash": "a" * 40, "installation_id": 12, "repository_link": "https://github.com/partner/apps"},
                "platform": {"repository": "partner/apps", "platform": "partner", "commit_hash": "b" * 40,
                             "platform_branch": "partner/apps/12/" + "a" * 12, "apps": [{"name": "demo", "release_tag": "apps-fixture", "path": "apps/12/r0123456789/demo", "aomi_toml_path": "aomi.toml"}]},
            })
            manifest = {"sdk_version": "5.1.1", "ffi_version": 1, "target": deploy.TARGET,
                        "plugins": {"demo": {"file": "demo.so", "sha256": hashlib.sha256(binary).hexdigest()}}}
            metadata = {"source": {"commit": "a" * 40}, "candidate": {"commit": "b" * 40},
                        "release": {"tag": "apps-fixture"}, "app": {"name": "demo"}}
            (root / "aomi-release.json").write_text(json.dumps(metadata))
            with tarfile.open(root / f"aomi-plugins-apps-fixture-{deploy.TARGET}.tar.gz", "w:gz") as archive:
                for name, data in (("plugins/manifest.json", json.dumps(manifest).encode()), ("plugins/demo.so", binary)):
                    member = tarfile.TarInfo(name)
                    member.size = len(data)
                    archive.addfile(member, io.BytesIO(data))
            deploy.verify_bundle(root, deployment, app, manifest)
            with self.assertRaisesRegex(RuntimeError, "FFI compatibility"):
                deploy.verify_bundle(root, replace(deployment, ffi_version=2), app, manifest)
            with self.assertRaisesRegex(RuntimeError, "FFI compatibility"):
                deploy.verify_bundle(root, replace(deployment, ffi_version=None), app, manifest)
            with self.assertRaisesRegex(RuntimeError, "FFI compatibility"):
                deploy.verify_bundle(root, deployment, app, {**manifest, "ffi_version": True})
            corrupted = json.loads(json.dumps(manifest))
            corrupted["plugins"]["demo"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(RuntimeError, "manifests do not agree|checksum"):
                deploy.verify_bundle(root, deployment, app, corrupted)


if __name__ == "__main__":
    unittest.main()
