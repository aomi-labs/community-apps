use aomi_sdk::{DynAomiTool, DynToolCallCtx, dyn_aomi_app};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Default)]
struct SmokeApp;

#[derive(Debug, Deserialize, JsonSchema)]
struct EchoArgs {
    message: String,
}

struct EchoTool;

impl DynAomiTool for EchoTool {
    type App = SmokeApp;
    type Args = EchoArgs;

    const NAME: &'static str = "community_smoke_20260524_echo";
    const DESCRIPTION: &'static str = "Echo a message for CI bundle validation.";

    fn run(
        _app: &SmokeApp,
        args: Self::Args,
        _ctx: DynToolCallCtx,
    ) -> Result<Value, String> {
        Ok(serde_json::json!({ "message": args.message }))
    }
}

dyn_aomi_app!(
    app = SmokeApp,
    name = "community-smoke-20260524",
    version = "0.1.0",
    preamble = "You are the community launch smoke app.",
    tools = [EchoTool],
    namespaces = ["common"]
);
