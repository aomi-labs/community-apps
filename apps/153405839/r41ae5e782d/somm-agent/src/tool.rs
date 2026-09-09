//! Agentic Somm tool implementations.
//!
//! Each tool forwards to one route on the deployed Somm app, mirroring the
//! TypeScript MCP proxy (`mcp/src/handlers.ts` + `TOOL_ROUTES`):
//!
//!   get_idle_assets   -> GET  /api/portfolio?wallet=…
//!   get_ymax_position -> GET  /api/ymax/position?wallet=…
//!   get_risk_snapshot -> GET  /api/risk/snapshot
//!   assess_position   -> POST /api/risk/assess
//!   get_credit_balance-> GET  /api/account
//!   propose_intent    -> (no HTTP) the args ARE the recommendation card the
//!                        frontend tool-UI renders; the tool just acknowledges.

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use crate::client::SommApp;
use crate::session::{read_wallet, resolve_connected_wallet};

// ─── get_idle_assets ─────────────────────────────────────────────────────────

pub struct GetIdleAssets;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIdleAssetsArgs {
    /// Optional. Normally OMIT this — the connected wallet is resolved
    /// automatically from the session. Only set it to override.
    #[serde(default)]
    pub wallet: Option<String>,
}

impl DynAomiTool for GetIdleAssets {
    type App = SommApp;
    type Args = GetIdleAssetsArgs;

    const NAME: &'static str = "get_idle_assets";
    const DESCRIPTION: &'static str = "List the connected wallet's idle, uninvested holdings (balances, USD value, chain) \
         that could be put to work earning yield. ALWAYS call this first. The connected wallet \
         is resolved automatically from the session — do NOT pass a wallet and do NOT ask the \
         user for an address.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        // Authoritative: the user's actual connected wallet from the host session
        // context. Fall back to an explicit override only if one was passed.
        let wallet = resolve_connected_wallet(&ctx).or_else(|| {
            args.wallet
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        });

        let Some(wallet) = wallet else {
            // No address in the session → genuinely not connected. Return a clear
            // signal (plus the available attribute keys to aid diagnosis).
            return Ok(json!({
                "connected": false,
                "message": "No wallet is connected. Ask the user to connect their wallet using the button in the app.",
                "_debug_state_attribute_keys": ctx.state_attributes.keys().cloned().collect::<Vec<String>>(),
            }));
        };

        app.get(&ctx, &format!("/api/portfolio?wallet={}", wallet))
    }
}

// ─── get_ymax_position ───────────────────────────────────────────────────────

pub struct GetYmaxPosition;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetYmaxPositionArgs {
    /// Optional. Normally OMIT this — resolved from the session. If supplied it
    /// must name the connected address.
    #[serde(default)]
    pub wallet: Option<String>,
}

impl DynAomiTool for GetYmaxPosition {
    type App = SommApp;
    type Args = GetYmaxPositionArgs;

    const NAME: &'static str = "get_ymax_position";
    const DESCRIPTION: &'static str = "Read the wallet's Ymax (Agoric) managed-portfolio positions: which venues and chains \
         each portfolio holds, and whether an agent may rebalance it under a delegation. \
         Ymax publishes cumulative flows rather than positions marked to market, and no rate, \
         so this returns NO APY and NO position value — never state or estimate either; point \
         the user to ymax.app for live figures. A wallet may own several portfolios. \
         If the result status is \"unavailable\", the data could not be read: do NOT conclude \
         the wallet holds nothing, say it could not be checked and offer to retry.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let wallet = read_wallet(&ctx, args.wallet.as_deref())?;
        app.get(&ctx, &format!("/api/ymax/position?wallet={}", wallet))
    }
}

// ─── zero-argument tools ─────────────────────────────────────────────────────

/// Args type for every tool that takes no input.
///
/// A genuinely empty `properties: {}` map survives the Rust descriptor but is
/// omitted by the plugin/provider serialization boundary. Keep one optional,
/// ignored field so the schema remains an object with a non-empty `properties`
/// map all the way to the model provider. Callers should omit it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyArgs {
    /// Reserved schema anchor; omit from tool calls.
    #[serde(default, rename = "reserved")]
    pub _reserved: Option<bool>,
}

// ─── get_risk_snapshot ───────────────────────────────────────────────────────

pub struct GetRiskSnapshot;

impl DynAomiTool for GetRiskSnapshot {
    type App = SommApp;
    type Args = EmptyArgs;

    const NAME: &'static str = "get_risk_snapshot";
    const DESCRIPTION: &'static str = "Get the current venue/asset risk snapshot used to reason about where idle assets can \
         be deployed safely. Read-only context for recommendations.";

    fn run(app: &SommApp, _args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        app.get(&ctx, "/api/risk/snapshot")
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
    const DESCRIPTION: &'static str = "Assess the risk of a specific candidate position (chain + protocol + asset + USD size) \
         before recommending it. Call before proposing an intent when the user asks about risk.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.chain.is_empty() || args.project.is_empty() || args.symbol.is_empty() {
            return Err("chain, project, symbol (non-empty strings) are required".to_string());
        }
        if !args.amount_usd.is_finite() || args.amount_usd <= 0.0 {
            return Err("amountUsd (positive finite number) is required".to_string());
        }
        app.post(
            &ctx,
            "/api/risk/assess",
            json!({
                "chain": args.chain,
                "project": args.project,
                "symbol": args.symbol,
                "amountUsd": args.amount_usd,
            }),
        )
    }
}

// ─── get_credit_balance ──────────────────────────────────────────────────────

pub struct GetCreditBalance;

const LOW_CREDIT_THRESHOLD: i128 = 50_000;

impl DynAomiTool for GetCreditBalance {
    type App = SommApp;
    type Args = EmptyArgs;

    const NAME: &'static str = "get_credit_balance";
    const DESCRIPTION: &'static str =
        "Check the account's remaining paid-call credit balance. Warn the user when it is low.";

    fn run(app: &SommApp, _args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let body = app.get(&ctx, "/api/account")?;
        // balance may arrive as a number or a string; normalize to a decimal string.
        let bal_str = match body.get("balance") {
            None | Some(Value::Null) => "0".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        };
        // Low check mirrors the MCP handler (BigInt < 50000n); non-numeric => not low.
        let low = bal_str
            .parse::<i128>()
            .map(|n| n < LOW_CREDIT_THRESHOLD)
            .unwrap_or(false);
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

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    BlueChip,
    Standard,
    Experimental,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReachableVia {
    Planner,
    Steps,
}

/// The source holding the recommendation deploys from (prefills the preview modal).
/// Fields are schema/contract only — the frontend tool-UI reads them, not `run`.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
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

/// Fields mirror `parseRecommendationCard` and are read by the frontend tool-UI.
/// `run` echoes the full card back as the tool RESULT so the frontend renders it
/// even on runtimes that do not forward tool-call args to the client.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
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
    /// The source holding this recommendation deploys from. OPTIONAL: when
    /// omitted (models often skip it), it is reconstructed server-side from the
    /// connected wallet's portfolio by matching the symbol on the target chain.
    #[serde(default)]
    pub source: Option<Holding>,
    /// Estimated yearly yield in USD at the quoted APY.
    pub est_yearly_usd: f64,
    /// One- or two-sentence plain-language rationale for the user.
    #[serde(default)]
    pub rationale: String,
}

/// Map a human chain label ("Base", "Ethereum", …) to Aurora's portfolio chain
/// key ("base", "eth", …) for the /api/portfolio?chains= filter.
fn aurora_chain_key(label: &str) -> Option<&'static str> {
    match label.trim().to_lowercase().as_str() {
        "base" => Some("base"),
        "ethereum" | "eth" | "mainnet" => Some("eth"),
        "arbitrum" | "arb" => Some("arb"),
        "optimism" | "op" => Some("op"),
        "polygon" | "pol" | "matic" => Some("pol"),
        _ => None,
    }
}

pub struct ProposeIntent;

impl DynAomiTool for ProposeIntent {
    type App = SommApp;
    type Args = ProposeIntentArgs;

    const NAME: &'static str = "propose_intent";
    const DESCRIPTION: &'static str = "Surface ONE yield recommendation to the user as a card. Provide a complete, accurate \
         destination (pool/project/chain/symbol/apy/tvl/tier/reachability), the estimated yearly \
         USD yield, and a plain-language rationale. The `source` holding is optional — when \
         omitted it is reconstructed from the connected wallet's portfolio automatically. \
         This only RECOMMENDS — the user reviews and approves deployment; you never execute.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut args = args;

        // Reconstruct the source holding server-side when the model omits it —
        // deterministic, from the same portfolio get_idle_assets serves, instead
        // of relying on the model to copy 10 fields verbatim.
        if args.source.is_none() {
            let wallet = resolve_connected_wallet(&ctx).ok_or_else(|| {
                "propose_intent: no connected wallet in the session to reconstruct the source holding from"
                    .to_string()
            })?;
            let path = match aurora_chain_key(&args.chain) {
                Some(k) => format!("/api/portfolio?wallet={}&chains={}", wallet, k),
                None => format!("/api/portfolio?wallet={}", wallet),
            };
            let body = app.get(&ctx, &path)?;
            let holdings = body
                .get("holdings")
                .and_then(|h| h.as_array())
                .cloned()
                .unwrap_or_default();
            let sym = args.symbol.trim().to_uppercase();
            let found = holdings.into_iter().find(|h| {
                h.get("symbol")
                    .and_then(|s| s.as_str())
                    .map(|s| s.trim().to_uppercase() == sym)
                    .unwrap_or(false)
            });
            let Some(hjson) = found else {
                return Err(format!(
                    "propose_intent: the connected wallet holds no {} on {} to deploy from",
                    args.symbol, args.chain
                ));
            };
            let holding: Holding = serde_json::from_value(hjson)
                .map_err(|e| format!("propose_intent: portfolio holding shape mismatch: {e}"))?;
            args.source = Some(holding);
        }

        // Echo the FULL card back as the tool result. The assistant-ui tool-UI
        // renders the card from args when available, but falls back to the
        // RESULT — which is the only channel that reaches the client on runtimes
        // that do not forward tool-call args.
        let mut out = serde_json::to_value(&args).map_err(|e| e.to_string())?;
        if let Some(o) = out.as_object_mut() {
            o.insert("status".into(), json!("surfaced"));
            o.insert(
                "message".into(),
                json!("Recommendation surfaced to the user for review and approval."),
            );
        }
        Ok(out)
    }
}
