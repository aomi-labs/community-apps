import json, pathlib, sys, types, unittest
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
        def boom(cmd, **kw):
            raise RuntimeError("unrecognized subcommand 'manifest'")
        bc.run = boom
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.1"), [])

    def test_returns_empty_when_the_manifest_has_no_secrets(self):
        bc.run = lambda cmd, **kw: json.dumps({"name": "hello"})
        self.assertEqual(bc.read_plugin_secrets(pathlib.Path("/tmp/x.so"), "3.0.2"), [])


if __name__ == "__main__":
    unittest.main()
