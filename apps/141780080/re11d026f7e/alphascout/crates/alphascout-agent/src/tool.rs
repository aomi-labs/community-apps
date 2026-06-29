use crate::engine::process_message;
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Default)]
pub(crate) struct AlphaScoutApp;

pub(crate) struct AnalyzeAlpha;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AnalyzeAlphaArgs {
    /// User request to route through AlphaScout, such as "scan markets", "analyze odds", or "find high conviction alpha".
    pub(crate) message: String,
}

impl DynAomiTool for AnalyzeAlpha {
    type App = AlphaScoutApp;
    type Args = AnalyzeAlphaArgs;
    const NAME: &'static str = "analyze_alpha";
    const DESCRIPTION: &'static str = "Run AlphaScout's prediction-market scanner for scan, odds-analysis, or high-conviction-alpha requests. Read-only.";

    fn run(_app: &AlphaScoutApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("failed to start AlphaScout runtime: {e}"))?;
        let reply = runtime.block_on(process_message(args.message.clone()));
        Ok(json!({
            "source": "alphascout",
            "message": args.message,
            "reply": reply,
        }))
    }
}
