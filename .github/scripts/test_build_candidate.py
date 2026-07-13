import json, pathlib, subprocess, sys, unittest
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import build_candidate as bc


def _completed(*, returncode: int = 0, stdout: str = "", stderr: str = "") -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess(args=["aomi-build", "manifest"], returncode=returncode, stdout=stdout, stderr=stderr)


class ReadPluginSecretsTests(unittest.TestCase):
    def setUp(self):
        # Every test overrides these before calling read_plugin_secrets; restore
        # the real functions afterwards so tests can't leak into each other.
        self.addCleanup(setattr, bc, "run", bc.run)
        self.addCleanup(setattr, bc, "run_capture", bc.run_capture)
        # Default: cargo install "succeeds" (most tests only care about the
        # `aomi-build manifest` step). Tests that need to simulate a cargo
        # install failure override this explicitly.
        bc.run = lambda cmd, **kw: ""

    def test_returns_slots_from_the_manifest_command(self):
        bc.run_capture = lambda cmd, **kw: _completed(
            stdout=json.dumps(
                {"name": "binance",
                 "secrets": [{"name": "BINANCE_API_KEY", "description": "d", "required": True}]}
            )
        )
        slots = bc.read_plugin_secrets(pathlib.Path("/tmp/libbinance.so"), "3.0.2")
        self.assertEqual(slots[0]["name"], "BINANCE_API_KEY")

    def test_returns_empty_when_the_sdk_lacks_the_subcommand(self):
        # Real-world failure path for an older SDK: clap rejects `manifest` as
        # an unrecognized subcommand. This is the ONE legitimate reason to
        # fall back to [] -- the app simply can't be secret-gated with this
        # SDK version.
        bc.run_capture = lambda cmd, **kw: _completed(
            returncode=2, stderr="error: unrecognized subcommand 'manifest'"
        )
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.1"), [])

    def test_returns_empty_when_the_manifest_has_no_secrets(self):
        bc.run_capture = lambda cmd, **kw: _completed(stdout=json.dumps({"name": "hello"}))
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])

    def test_raises_when_cargo_install_fails(self):
        # A flaky/transient `cargo install` (network hiccup, crates.io blip,
        # etc.) must fail the build, not silently return []. run() raises
        # SystemExit via fail() on a non-zero exit.
        def boom(cmd, **kw):
            raise SystemExit("error: command failed (cargo install ...): connection reset")
        bc.run = boom
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_on_transient_manifest_failure(self):
        # A non-zero exit from `aomi-build manifest` that is NOT the
        # unrecognized-subcommand case (e.g. a panic, OOM, transient IO
        # error) must fail the build rather than silently return [].
        bc.run_capture = lambda cmd, **kw: _completed(
            returncode=1, stderr="thread 'main' panicked at 'unexpected plugin ABI'"
        )
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_when_manifest_json_is_malformed(self):
        # Not valid JSON at all from an SDK that DOES support `manifest`.
        bc.run_capture = lambda cmd, **kw: _completed(stdout="{not json")
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_when_manifest_json_is_null(self):
        bc.run_capture = lambda cmd, **kw: _completed(stdout="null")
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_when_manifest_json_is_a_string(self):
        bc.run_capture = lambda cmd, **kw: _completed(stdout='"a string"')
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_when_manifest_json_is_a_bare_list(self):
        bc.run_capture = lambda cmd, **kw: _completed(stdout="[]")
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")

    def test_raises_when_secrets_field_is_not_a_list(self):
        bc.run_capture = lambda cmd, **kw: _completed(
            stdout=json.dumps({"name": "hello", "secrets": "oops"})
        )
        with self.assertRaises(SystemExit):
            bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2")


class IsUnsupportedManifestErrorTests(unittest.TestCase):
    def test_recognizes_common_clap_phrasings(self):
        self.assertTrue(bc.is_unsupported_manifest_error("error: unrecognized subcommand 'manifest'"))
        self.assertTrue(bc.is_unsupported_manifest_error("error: no such subcommand: `manifest`"))
        self.assertTrue(bc.is_unsupported_manifest_error("error: unexpected argument 'manifest' found"))
        self.assertTrue(
            bc.is_unsupported_manifest_error(
                "error: Found argument 'manifest' which wasn't expected, or isn't valid in this context"
            )
        )

    def test_does_not_recognize_unrelated_failures(self):
        self.assertFalse(bc.is_unsupported_manifest_error("thread 'main' panicked at 'index out of bounds'"))
        self.assertFalse(bc.is_unsupported_manifest_error("error: connection reset by peer"))
        self.assertFalse(bc.is_unsupported_manifest_error(""))


if __name__ == "__main__":
    unittest.main()
