import json, pathlib, sys, unittest
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import build_candidate as bc


class ReadPluginSecretsTests(unittest.TestCase):
    def test_returns_slots_from_the_manifest_command(self):
        bc.run = lambda cmd, **kw: json.dumps(
            {"name": "binance",
             "secrets": [{"name": "BINANCE_API_KEY", "description": "d", "required": True}]}
        )
        slots = bc.read_plugin_secrets(pathlib.Path("/tmp/libbinance.so"), "3.0.2")
        self.assertEqual(slots[0]["name"], "BINANCE_API_KEY")

    def test_returns_empty_when_the_sdk_lacks_the_subcommand(self):
        # Real-world failure path: run() -> fail() -> SystemExit. SystemExit does
        # NOT subclass Exception, so this is the regression test for the
        # `except (Exception, SystemExit)` clause -- a plain `except Exception`
        # would let this propagate and fail the whole CI job.
        def boom(cmd, **kw):
            raise SystemExit("unrecognized subcommand 'manifest'")
        bc.run = boom
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.1"), [])

    def test_returns_empty_on_generic_exception(self):
        def boom(cmd, **kw):
            raise RuntimeError("cargo install failed")
        bc.run = boom
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.1"), [])

    def test_returns_empty_when_the_manifest_has_no_secrets(self):
        bc.run = lambda cmd, **kw: json.dumps({"name": "hello"})
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])

    def test_returns_empty_when_manifest_json_is_null(self):
        # json.loads("null") -> None, which has no .get(); must not raise.
        bc.run = lambda cmd, **kw: "null"
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])

    def test_returns_empty_when_manifest_json_is_a_string(self):
        bc.run = lambda cmd, **kw: '"a string"'
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])

    def test_returns_empty_when_manifest_json_is_a_bare_list(self):
        bc.run = lambda cmd, **kw: "[]"
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])

    def test_returns_empty_when_secrets_field_is_not_a_list(self):
        bc.run = lambda cmd, **kw: json.dumps({"name": "hello", "secrets": "oops"})
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])


if __name__ == "__main__":
    unittest.main()
