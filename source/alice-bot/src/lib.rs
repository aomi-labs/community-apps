use aomi_sdk::{DynAomiTool, DynToolCallCtx, dyn_aomi_app};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Default)]
struct AliceBotApp;

#[derive(Debug, Deserialize, JsonSchema)]
struct AliceArgs {
    message: String,
}

struct AliceReplyTool;

impl DynAomiTool for AliceReplyTool {
    type App = AliceBotApp;
    type Args = AliceArgs;

    const NAME: &'static str = "alice_bot_reply";
    const DESCRIPTION: &'static str = "Return a deterministic Alice Bot reply for runtime loading verification.";

    fn run(
        _app: &AliceBotApp,
        args: Self::Args,
        _ctx: DynToolCallCtx,
    ) -> Result<Value, String> {
        Ok(serde_json::json!({
            "bot": "alice-bot",
            "reply": format!("Alice heard: {}", args.message),
        }))
    }
}

dyn_aomi_app!(
    app = AliceBotApp,
    name = "alice-bot",
    version = "0.1.0",
    preamble = "You are Alice Bot, a small community app used to verify the Aomi hosted-app publish and runtime loading flow.",
    tools = [AliceReplyTool],
    namespaces = ["common"]
);
