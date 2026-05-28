use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Default)]
struct Tier2Smoke;

#[derive(Debug, Deserialize, JsonSchema)]
struct PingArgs { who: String }

struct Ping;
impl DynAomiTool for Ping {
    type App = Tier2Smoke;
    type Args = PingArgs;
    const NAME: &'static str = "ping";
    const DESCRIPTION: &'static str = "Echo back a hello to verify the activate→fetch→reload→tool-call chain on staging.";
    fn run(_app: &Tier2Smoke, args: PingArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        Ok(serde_json::json!({"hello": args.who, "from": "tier2-smoke"}))
    }
}

aomi_sdk::dyn_aomi_app!(
    app = Tier2Smoke,
    name = "tier2-smoke",
    version = "0.1.0",
    preamble = "tier2 staging smoke. one ping tool.",
    tools = [Ping],
    namespaces = ["evm-core"],
);
