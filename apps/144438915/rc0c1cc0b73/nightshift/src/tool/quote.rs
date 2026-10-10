//! `quote_sol_buy` — read-only. Show what a bounded SOL buy would do right now,
//! against a live Jupiter route, after checking the envelope.

use serde::Deserialize;
use serde_json::{Value, json};

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;

use crate::client::{NightshiftApp, VENUE, enforce_envelope, jupiter_quote, to_base_units};
use crate::tool::summarize_quote;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct QuoteSolBuyArgs {
    /// Dollars of USDC to spend on SOL. Must be inside the per-action envelope
    /// (at most 20). This app only ever buys SOL with USDC.
    pub usdc_amount: f64,
}

pub(crate) struct QuoteSolBuy;

impl DynAomiTool for QuoteSolBuy {
    type App = NightshiftApp;
    type Args = QuoteSolBuyArgs;
    const NAME: &'static str = "quote_sol_buy";
    const DESCRIPTION: &'static str =
        "Preview a bounded USDC -> SOL buy with a live Jupiter quote. Read-only: it fetches the \
         real route, price impact, amount received, and the floor after slippage, but signs \
         nothing. NEEDS NO CONNECTED WALLET — call it directly whenever the user asks for a quote \
         or what the agent would do; never check wallet status or ask the user to connect a wallet \
         for a quote. Refuses any amount outside the app's fixed per-action envelope.";

    fn run(_app: &Self::App, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        enforce_envelope(args.usdc_amount)?;
        let amount_base = to_base_units(args.usdc_amount);
        let quote = jupiter_quote(amount_base)?;
        Ok(json!({
            "inside_envelope": true,
            "spend_usdc": args.usdc_amount,
            "venue": VENUE,
            "quote": summarize_quote(&quote),
        }))
    }
}
