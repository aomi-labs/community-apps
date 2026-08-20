//! Agentic Somm tool implementations.
//!
//! Each tool forwards to one route on the deployed Somm app, mirroring the
//! TypeScript MCP proxy (`mcp/src/handlers.ts` + `TOOL_ROUTES`):
//!
//!   get_idle_assets   -> GET  /api/portfolio?wallet=…
//!   get_risk_snapshot -> GET  /api/risk/snapshot
//!   assess_position   -> POST /api/risk/assess
//!   get_credit_balance-> GET  /api/account
//!   propose_intent    -> (no HTTP) the args ARE the recommendation card the
//!                        frontend tool-UI renders; the tool just acknowledges.

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::Deserialize;
use serde_json::{json, Value};

pub use crate::client::SommApp;

/// Resolve the app's API key for this call: host-injected secret vault first,
/// then the `SOMM_API_KEY` env var as a local/CLI fallback.
fn api_key(ctx: &DynToolCallCtx) -> Result<String, String> {
    resolve_secret_value(
        ctx,
        None,
        crate::SOMM_API_KEY.name,
        "SOMM_API_KEY is not set — add it in the app's settings to use this tool.",
    )
}

// ─── get_idle_assets ─────────────────────────────────────────────────────────

pub struct GetIdleAssets;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIdleAssetsArgs {
    /// The connected wallet address (0x…) to inspect for idle, uninvested assets.
    pub wallet: String,
}

impl DynAomiTool for GetIdleAssets {
    type App = SommApp;
    type Args = GetIdleAssetsArgs;

    const NAME: &'static str = "get_idle_assets";
    const DESCRIPTION: &'static str =
        "List the connected wallet's idle, uninvested holdings (balances, USD value, chain) \
         that could be put to work earning yield. Call this first to see what the user holds.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.wallet.is_empty() {
            return Err("wallet (non-empty string) is required".to_string());
        }
        app.get(&format!("/api/portfolio?wallet={}", args.wallet), &api_key(&ctx)?)
    }
}

// ─── get_risk_snapshot ───────────────────────────────────────────────────────

pub struct GetRiskSnapshot;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRiskSnapshotArgs {}

impl DynAomiTool for GetRiskSnapshot {
    type App = SommApp;
    type Args = GetRiskSnapshotArgs;

    const NAME: &'static str = "get_risk_snapshot";
    const DESCRIPTION: &'static str =
        "Get the current venue/asset risk snapshot used to reason about where idle assets can \
         be deployed safely. Read-only context for recommendations.";

    fn run(app: &SommApp, _args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        app.get("/api/risk/snapshot", &api_key(&ctx)?)
    }
}

// ─── assess_position ─────────────────────────────────────────────────────────

pub struct AssessPosition;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessPositionArgs {
    /// Destination chain (e.g. "Base", "Arbitrum").
    pub chain: String,
    /// Destination protocol (e.g. "aave-v3").
    pub project: String,
    /// Asset symbol (e.g. "USDC").
    pub symbol: String,
    /// Position size in USD; must be a positive number.
    pub amount_usd: f64,
}

impl DynAomiTool for AssessPosition {
    type App = SommApp;
    type Args = AssessPositionArgs;

    const NAME: &'static str = "assess_position";
    const DESCRIPTION: &'static str =
        "Assess the risk of a specific candidate position (chain + protocol + asset + USD size) \
         before recommending it. Call before proposing an intent when the user asks about risk.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.chain.is_empty() || args.project.is_empty() || args.symbol.is_empty() {
            return Err("chain, project, symbol (non-empty strings) are required".to_string());
        }
        if !args.amount_usd.is_finite() || args.amount_usd <= 0.0 {
            return Err("amountUsd (positive finite number) is required".to_string());
        }
        app.post(
            "/api/risk/assess",
            json!({
                "chain": args.chain,
                "project": args.project,
                "symbol": args.symbol,
                "amountUsd": args.amount_usd,
            }),
            &api_key(&ctx)?,
        )
    }
}

// ─── get_credit_balance ──────────────────────────────────────────────────────

pub struct GetCreditBalance;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCreditBalanceArgs {}

const LOW_CREDIT_THRESHOLD: i128 = 50_000;

impl DynAomiTool for GetCreditBalance {
    type App = SommApp;
    type Args = GetCreditBalanceArgs;

    const NAME: &'static str = "get_credit_balance";
    const DESCRIPTION: &'static str =
        "Check the account's remaining paid-call credit balance. Warn the user when it is low.";

    fn run(app: &SommApp, _args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let body = app.get("/api/account", &api_key(&ctx)?)?;
        // balance may arrive as a number or a string; normalize to a decimal string.
        let bal_str = match body.get("balance") {
            None | Some(Value::Null) => "0".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        };
        // Low check mirrors the MCP handler (BigInt < 50000n); non-numeric => not low.
        let low = bal_str.parse::<i128>().map(|n| n < LOW_CREDIT_THRESHOLD).unwrap_or(false);
        Ok(json!({
            "balance": bal_str,
            "low": low,
            "message": if low {
                "Balance is LOW — top up to continue making paid calls."
            } else {
                "Balance OK."
            },
        }))
    }
}

// ─── propose_intent ──────────────────────────────────────────────────────────
// The args ARE the recommendation card. Their shape mirrors `parseRecommendationCard`
// (src/lib/recommendation.ts) one-to-one (camelCase keys), so the assistant-ui
// tool-UI renders the card directly from the tool-call args. No HTTP call.

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    BlueChip,
    Standard,
    Experimental,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReachableVia {
    Planner,
    Steps,
}

/// The source holding the recommendation deploys from (prefills the preview modal).
/// Fields are schema/contract only — the frontend tool-UI reads them, not `run`.
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub asset_id: String,
    pub symbol: String,
    pub chain_key: String,
    pub chain_label: String,
    /// 0x… ERC-20 contract address on the source chain.
    pub contract_address: String,
    pub decimals: u8,
    /// USD price per token.
    pub price: f64,
    /// Raw on-chain balance in base units, as a decimal string.
    pub raw_balance: String,
    /// Human-readable balance, as a decimal string.
    pub balance: String,
    pub usd_value: f64,
}

/// Fields mirror `parseRecommendationCard` and are read by the frontend tool-UI,
/// not by `run` — schema/contract only on the Rust side.
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposeIntentArgs {
    /// Destination pool id.
    pub pool: String,
    /// Destination protocol (e.g. "aave-v3").
    pub project: String,
    /// Destination chain.
    pub chain: String,
    /// Asset symbol.
    pub symbol: String,
    /// Total APY (percent).
    pub apy: f64,
    /// Base APY (percent), or null.
    #[serde(default)]
    pub apy_base: Option<f64>,
    /// Reward APY (percent), or null.
    #[serde(default)]
    pub apy_reward: Option<f64>,
    /// Pool TVL in USD.
    pub tvl_usd: f64,
    /// Optional pool metadata label.
    #[serde(default)]
    pub pool_meta: Option<String>,
    /// Risk tier: blue-chip | standard | experimental.
    pub tier: Tier,
    /// Whether the destination is reachable for execution today.
    pub reachable: bool,
    /// How it is reachable: planner | steps | null.
    #[serde(default)]
    pub reachable_via: Option<ReachableVia>,
    /// Short caveats/notes shown on the card.
    #[serde(default)]
    pub notes: Vec<String>,
    /// The source holding this recommendation deploys from.
    pub source: Holding,
    /// Estimated yearly yield in USD at the quoted APY.
    pub est_yearly_usd: f64,
    /// One- or two-sentence plain-language rationale for the user.
    #[serde(default)]
    pub rationale: String,
}

pub struct ProposeIntent;

impl DynAomiTool for ProposeIntent {
    type App = SommApp;
    type Args = ProposeIntentArgs;

    const NAME: &'static str = "propose_intent";
    const DESCRIPTION: &'static str =
        "Surface ONE yield recommendation to the user as a card. Provide a complete, accurate \
         destination (pool/project/chain/symbol/apy/tvl/tier/reachability), the exact source \
         holding it deploys from, the estimated yearly USD yield, and a plain-language rationale. \
         This only RECOMMENDS — the user reviews and approves deployment; you never execute.";

    fn run(_app: &SommApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        // The frontend renders the card from these args; the tool just acknowledges.
        Ok(json!({
            "status": "surfaced",
            "pool": args.pool,
            "project": args.project,
            "message": "Recommendation surfaced to the user for review and approval.",
        }))
    }
}
