//! `stage_sol_buy` — the bounded execution path. Checks the envelope, gets a
//! live Jupiter quote, asks Jupiter to build the swap transaction for the
//! connected wallet, and emits the canonical
//! `svm_stage_tx` -> `svm_commit_tx({mode: "wallet"})` route plan.
//!
//! The app never signs. Who signs and who submits are the host's decisions:
//! `svm_commit_tx({mode: "wallet"})` routes the staged blob to the connected
//! wallet, which signs and broadcasts. Simulate-before-sign is the host's
//! stage/commit pipeline. The app's only authority is to stage an action that
//! is already inside the box.

use serde::Deserialize;
use serde_json::json;

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;

use crate::client::{
    MAX_PER_ACTION_USDC, NightshiftApp, VENUE, enforce_envelope, jupiter_quote, jupiter_swap_blob,
    to_base_units,
};
use crate::tool::{require_svm_wallet, summarize_quote};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct StageSolBuyArgs {
    /// Dollars of USDC to spend on SOL. Must be inside the per-action envelope
    /// (at most 20). This app only ever buys SOL with USDC.
    pub usdc_amount: f64,
}

pub(crate) struct StageSolBuy;

impl DynAomiTool for StageSolBuy {
    type App = NightshiftApp;
    type Args = StageSolBuyArgs;
    const NAME: &'static str = "stage_sol_buy";
    const DESCRIPTION: &'static str =
        "Stage a bounded USDC -> SOL buy for the connected wallet to sign. Checks the fixed \
         per-action envelope, gets a live Jupiter quote, has Jupiter build the swap transaction \
         for the wallet, and emits the svm_stage_tx -> svm_commit_tx({mode: \"wallet\"}) route \
         plan. The wallet signs and broadcasts; the app signs nothing. Emit a one-screen \
         confirmation (spend, receive, route, price impact) and stop the turn before calling this, \
         unless the user's message contains PRE-AUTHORIZED.";

    fn run_with_routes(
        _app: &Self::App,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        enforce_envelope(args.usdc_amount)?;
        let wallet = require_svm_wallet(&ctx)?;

        let amount_base = to_base_units(args.usdc_amount);
        let quote = jupiter_quote(amount_base)?;
        let summary = summarize_quote(&quote);
        let blob_b64 = jupiter_swap_blob(&quote, &wallet)?;

        let preview = json!({
            "action_kind": "stage_sol_buy",
            "inside_envelope": true,
            "envelope": {
                "pair": "USDC -> SOL",
                "max_per_action_usdc": MAX_PER_ACTION_USDC,
                "venue": VENUE,
            },
            "preview": {
                "spend_usdc": args.usdc_amount,
                "wallet": wallet,
                "quote": summary,
            },
            "requires_user_confirmation": true,
            "confirmation_phrase": "confirm",
        });

        let description = format!(
            "Nightshift: buy ${:.2} of SOL with USDC via {} for {}",
            args.usdc_amount, VENUE, wallet
        );

        ToolReturn::route(preview)
            .next(|next| {
                next.add::<host::SvmStageTx>(json!({
                    "tx": blob_b64,
                    "description": description.clone(),
                    "kind": "nightshift.jupiter-swap",
                }))
                .bind_as("tx_id")
                .note("Stage the Jupiter-built USDC->SOL swap blob.");
            })
            .after::<host::SvmCommitTx>(json!({ "mode": "wallet" }))
            .awaits("tx_id")
            .note("Commit via the connected wallet — it signs the staged blob and broadcasts.")
            .try_build()
            .map_err(|e| format!("[nightshift] route build failed: {e}"))
    }
}
