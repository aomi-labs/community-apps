//! Yield discovery — the read side of the Aurora work.
//!
//! Absorbed from the standalone `somm-aurora` app (`aurora-aomi-app`) so there
//! is one deployed app rather than two overlapping ones. What came across is the
//! *tool surface*; what did not is that app's implementation.
//!
//! ## Why these are forwarders and not a port
//!
//! `somm-aurora` fetched DeFiLlama directly and carried a faithful Rust re-port
//! of `src/lib/yields.ts` — tier classification, blue-chip TVL thresholds,
//! experimental-stable demotions, rail reachability. That is a second source of
//! truth for **risk tier**, in a different language from the authoritative copy,
//! with nothing keeping the two in sync. Risk tier is what users actually act
//! on, so it gets exactly one implementation.
//!
//! `/api/yields` already returns ranked, V1-gated opportunities from that
//! authoritative copy, and `/api/surface` already groups them against the user's
//! real holdings. Both are cached server-side. So these tools carry no scoring,
//! no filtering and no thresholds of their own — matching how every other tool
//! in this app forwards to exactly one route.

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::Deserialize;
use serde_json::{Value, json};

pub use crate::client::SommApp;
use crate::session::read_wallet;

// ─── list_yields ─────────────────────────────────────────────────────────────

pub struct ListYields;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListYieldsArgs {
    /// Optional chain filter, e.g. ["Base"] or ["Ethereum","Arbitrum"]. Omit to
    /// list every in-scope chain.
    #[serde(default)]
    pub chains: Vec<String>,
}

impl DynAomiTool for ListYields {
    type App = SommApp;
    type Args = ListYieldsArgs;

    const NAME: &'static str = "list_yields";
    const DESCRIPTION: &'static str = "List in-scope stablecoin yield opportunities across Aave V3, Compound V3, Sky, Morpho and \
         Spark on Ethereum, Arbitrum and Base — each with APY, TVL, risk tier and whether it is \
         actually deployable today. Call this to compare venues or answer 'where can I earn yield'. \
         Tier and deployability come from the Somm API; report them as given rather than \
         recalculating or second-guessing them.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let path = if args.chains.is_empty() {
            "/api/yields".to_string()
        } else {
            // Chain labels are simple alphanumerics ("Base", "Arbitrum"); reject
            // anything else rather than hand-rolling percent-encoding for a case
            // that should not arise.
            for c in &args.chains {
                if !c.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                    return Err(format!("invalid chain label: {c}"));
                }
            }
            format!("/api/yields?chains={}", args.chains.join(","))
        };
        app.get(&ctx, &path)
    }
}

// ─── surface_opportunities ───────────────────────────────────────────────────

pub struct SurfaceOpportunities;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SurfaceOpportunitiesArgs {
    /// Optional. Normally OMIT this — the board is built for the connected
    /// wallet, resolved from the session. If supplied it must name that same
    /// address.
    #[serde(default)]
    pub wallet: Option<String>,
    /// Optional chain filter. Omit for all chains.
    #[serde(default)]
    pub chains: Vec<String>,
}

impl DynAomiTool for SurfaceOpportunities {
    type App = SommApp;
    type Args = SurfaceOpportunitiesArgs;

    const NAME: &'static str = "surface_opportunities";
    const DESCRIPTION: &'static str = "Show the user an interactive board of yield opportunities grouped by chain, matched \
         against what they actually hold. Rows the wallet can fund are marked deployable; the rest \
         are view-only. Call this when the user wants to see their options laid out rather than a \
         single recommendation. Describe only what the result contains — never imply a row is \
         deployable when it is marked view-only.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let wallet = read_wallet(&ctx, args.wallet.as_deref())?;
        let mut body = json!({ "wallet": wallet });
        if !args.chains.is_empty() {
            body["chains"] = json!(args.chains);
        }

        // The frontend tool-UI bound to this tool name re-fetches `/api/surface`
        // itself using the wallet from React context, because the runtime does
        // not reliably forward tool args to the client. So the board the user
        // sees does not depend on this result. We still fetch it, because the
        // model needs the same data to talk about what it just put on screen.
        app.post(&ctx, "/api/surface", body)
    }
}

// ─── get_intermediary ────────────────────────────────────────────────────────

pub struct GetIntermediary;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIntermediaryArgs {
    /// Optional. Normally OMIT this — resolved from the session. If supplied it
    /// must name the connected address.
    #[serde(default)]
    pub wallet: Option<String>,
}

impl DynAomiTool for GetIntermediary {
    type App = SommApp;
    type Args = GetIntermediaryArgs;

    const NAME: &'static str = "get_intermediary";
    const DESCRIPTION: &'static str = "Look up the Aurora MPC intermediary account for a wallet — the address funds route \
         through during a cross-chain execution. Read-only. Useful when a user asks where their \
         funds go or wants to verify a deposit address they were shown.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let wallet = read_wallet(&ctx, args.wallet.as_deref())?;
        app.get(&ctx, &format!("/api/intermediary?wallet={}", wallet))
    }
}
