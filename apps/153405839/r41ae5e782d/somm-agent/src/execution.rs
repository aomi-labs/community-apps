//! Aurora deposit execution — the agent-locus write path.
//!
//! Three tools form one route-driven flow. Each is a separate tool invocation;
//! the runtime resumes the chain when the wallet resolves a signature or a
//! transaction, so no tool ever blocks waiting on the user.
//!
//!   somm_aurora_deposit          -> POST /api/execution {action:"request"}
//!                                   then routes host::EvmCommitMessage (ERC-191)
//!   somm_aurora_submit_signature -> POST /api/execution {action:"submit"}
//!                                   then routes StageTx/SimulateBatch/CommitTxs
//!   somm_aurora_report_deposit   -> POST /api/execution {action:"deposit"}
//!                                   then polls /api/execution-status
//!
//! ## Every guard stays server-side
//!
//! The deposit cap, the deposits kill-switch, the per-wallet/IP rate limit, the
//! folded-sizing drift check, the 409/staleLock, and the settlement-presence
//! check all live in `/api/execution`. This module re-implements none of them.
//! A guard duplicated in Rust is the highest-risk failure mode available here —
//! it drifts silently out of sync with the one that actually protects funds.
//!
//! ## Values are bound, never re-derived
//!
//! The deposit's recipient and amount come verbatim from Aurora's own quote
//! (`quote.depositAddress`, `quote.amountIn`), mirroring the hardened frontend
//! orchestrator in `IntentPreviewModal.sendOneClickDeposit`. Re-scaling a human
//! amount crate-side would be a second, drift-prone source of truth for the
//! number of tokens that actually move.

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::Deserialize;
use serde_json::{Value, json};

pub use crate::client::SommApp;
use crate::session::require_session_wallet;

// ── V1 scope gate ────────────────────────────────────────────────────────────
//
// Base -> Base, aave-v3, USDC. This mirrors `V1_EXECUTABLE_VENUES` in
// src/lib/yields.ts and the Base-only refusal in the frontend orchestrator.
// The endpoint enforces its own venue gate too; this is defense in depth, and
// it keeps the agent from proposing a route it cannot finish.

const V1_CHAIN: &str = "Base";
const V1_CHAIN_ID: u64 = 8453;
const V1_PROJECT: &str = "aave-v3";
/// Canonical Base USDC (`CANONICAL_BASE_USDC` in src/lib/c6HumanGated.ts).
const BASE_USDC: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// `domain.app` shown in the wallet's signing prompt. ERC-191 has no native
/// domain separation, so this is display/audit provenance only — never
/// signing-relevant, and never a trust boundary.
const ERC191_DOMAIN_APP: &str = "Agentic Somm";
/// Distinguishes an Aurora execution authorization from any other ERC-191
/// message this app might request.
const ERC191_REQUEST_KIND: &str = "aurora_intent";

/// Pull a string out of a route-injected `Value`, which may arrive as a bare
/// string or as an object carrying it under one of several common keys. The
/// runtime types bound values as opaque JSON, so consumers unwrap defensively.
fn unwrap_route_str<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a str> {
    if let Some(s) = v.as_str() {
        return Some(s);
    }
    for key in keys {
        if let Some(s) = v.get(key).and_then(Value::as_str) {
            return Some(s);
        }
    }
    None
}

fn str_field(v: &Value, path: &[&str]) -> Option<String> {
    let mut cur = v;
    for key in path {
        cur = cur.get(key)?;
    }
    cur.as_str().map(str::to_string)
}

/// Reject anything outside the V1 lane, before any execution is created.
fn enforce_v1_scope(chain: &str, project: &str, symbol: &str) -> Result<(), String> {
    if !chain.eq_ignore_ascii_case(V1_CHAIN) {
        return Err(format!(
            "out_of_scope: V1 deposits are {V1_CHAIN}-only (got {chain}). Say so plainly rather \
             than implying another chain can be executed."
        ));
    }
    if !project.eq_ignore_ascii_case(V1_PROJECT) {
        return Err(format!(
            "out_of_scope: V1 executes {V1_PROJECT} only (got {project}). Other venues are \
             read-only for now."
        ));
    }
    if !symbol.eq_ignore_ascii_case("USDC") {
        return Err(format!(
            "out_of_scope: V1 deposits are USDC-only (got {symbol})."
        ));
    }
    Ok(())
}

/// Render a raw USDC amount (6 decimals) as human units for display.
///
/// Used for the wallet's signing prompt so the number the user is asked to
/// authorize is derived from the same value that actually moves
/// (`quote.amountIn`), not from what they typed. The two can differ — Aurora
/// quotes the exact input it wants — and a prompt that disagrees with the
/// transfer is how a user ends up authorizing something they did not read.
///
/// USDC-only is enforced by `enforce_v1_scope`, so 6 decimals is fixed here
/// rather than plumbed. Trailing zeros are trimmed: "100.500000" reads as
/// "100.5", and a whole amount as "100".
fn format_usdc(raw: &str) -> Option<String> {
    let amount: u128 = raw.parse().ok()?;
    let whole = amount / 1_000_000;
    let frac = amount % 1_000_000;
    if frac == 0 {
        return Some(whole.to_string());
    }
    let frac = format!("{frac:06}");
    Some(format!("{whole}.{}", frac.trim_end_matches('0')))
}

/// Minimal ERC-20 `transfer(address,uint256)` calldata.
fn erc20_transfer_calldata(to: &str, raw_amount: &str) -> Result<String, String> {
    let addr = to.strip_prefix("0x").unwrap_or(to);
    if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("deposit_error: malformed deposit address {to}"));
    }
    let amount: u128 = raw_amount
        .parse()
        .map_err(|_| format!("deposit_error: non-numeric raw amount {raw_amount}"))?;
    Ok(format!(
        "0xa9059cbb{:0>64}{:064x}",
        addr.to_lowercase(),
        amount
    ))
}

// ─── somm_aurora_deposit ─────────────────────────────────────────────────────

pub struct AuroraDeposit;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuroraDepositArgs {
    /// Optional. Normally OMIT this — the funding wallet is the connected
    /// wallet, resolved from the session. If supplied it must name that same
    /// address; anything else is refused.
    #[serde(default)]
    pub wallet: Option<String>,
    /// Stablecoin symbol. V1 is USDC only.
    pub symbol: String,
    /// Source chain. V1 is Base only.
    pub chain: String,
    /// Destination venue. V1 is aave-v3 only.
    pub project: String,
    /// Human-units amount to deposit, e.g. "100" or "100.5".
    pub amount: String,
}

impl DynAomiTool for AuroraDeposit {
    type App = SommApp;
    type Args = AuroraDepositArgs;

    const NAME: &'static str = "somm_aurora_deposit";
    const DESCRIPTION: &'static str = "Execute a USDC deposit into Aave v3 on Base over the Aurora Intents rail, after the user \
         has explicitly asked to deploy a specific amount. Requests the execution, then asks the \
         user's wallet to sign the Aurora authorization message and to send the deposit transfer. \
         The deposit cap, kill-switch and rate limit are enforced by the Somm API, not here. Do \
         not call this to explore or compare options — use propose_intent for recommendations.";

    fn run_with_routes(
        app: &SommApp,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        // The connected wallet is authoritative and comes from the host, never
        // from the model. Resolved before anything else so a mismatch costs no
        // Aurora call.
        let wallet = require_session_wallet(&ctx, args.wallet.as_deref())?;
        enforce_v1_scope(&args.chain, &args.project, &args.symbol)?;

        // The endpoint resolves canonical Aurora asset ids and decimals itself
        // when they are omitted, and takes the HUMAN amount so its cap check
        // runs against a value we cannot pre-scale out from under it.
        let resp = app.post(
            &ctx,
            "/api/execution",
            json!({
                "action": "request",
                "wallet": wallet,
                "source": {
                    "chainLabel": args.chain,
                    "symbol": args.symbol,
                    "amount": args.amount,
                },
                "destination": {
                    "chainLabel": args.chain,
                    "symbol": args.symbol,
                },
                "slippageBps": 50,
                "type": format!("bridge-in:{}", args.project),
                "metadata": { "venue": args.project },
            }),
        )?;

        let execution = resp
            .get("execution")
            .ok_or("[somm] execution request returned no `execution`")?;

        let execution_id =
            str_field(execution, &["id"]).ok_or("[somm] execution response missing id")?;
        let message_to_sign = str_field(execution, &["details", "messageToSign"])
            .ok_or("[somm] execution response missing details.messageToSign")?;

        // Bound verbatim from Aurora's quote — never re-derived. If either is
        // absent we stop here rather than guessing a recipient or an amount.
        let deposit_address = str_field(execution, &["quote", "depositAddress"]).ok_or(
            "[somm] execution quote missing depositAddress — refusing to guess a recipient",
        )?;
        let amount_in = str_field(execution, &["quote", "amountIn"])
            .ok_or("[somm] execution quote missing amountIn — refusing to re-derive the amount")?;

        // The prompt must state the amount that will actually move. `amount_in`
        // is Aurora's own quoted input and is what the transfer sends; the
        // user's requested figure is only the ask.
        let display_amount = format_usdc(&amount_in).ok_or_else(|| {
            format!("[somm] execution quote amountIn is not a valid raw amount: {amount_in}")
        })?;

        ToolReturn::route(json!({
            "status": "awaiting_signature",
            "message": format!(
                "Please sign the Aurora authorization message in your wallet to authorize \
                 {display_amount} {}. Nothing has moved yet.",
                args.symbol
            ),
            "execution_id": execution_id,
            "amount_to_deposit": display_amount,
        }))
        .next(|next| {
            next.add::<host::EvmCommitMessage>(json!({
                "non_typed_data": message_to_sign,
                "description": format!(
                    "Authorize a {} {} deposit into {} on {} over the Aurora Intents rail.",
                    display_amount, args.symbol, args.project, args.chain
                ),
                "domain": {
                    "app": ERC191_DOMAIN_APP,
                    "chain_id": V1_CHAIN_ID,
                    "purpose": format!("aurora_intent:{execution_id}"),
                },
                "request_kind": ERC191_REQUEST_KIND,
            }))
            .bind_as("signature");
        })
        .after_named(
            AuroraSubmitSignature::NAME,
            json!({
                "execution_id": execution_id,
                "wallet": wallet,
                "deposit_address": deposit_address,
                "amount_in": amount_in,
            }),
        )
        .awaits("signature")
        .try_build()
    }
}

// ─── somm_aurora_submit_signature (internal) ─────────────────────────────────

pub struct AuroraSubmitSignature;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuroraSubmitSignatureArgs {
    pub execution_id: String,
    pub wallet: String,
    /// Aurora's `quote.depositAddress` — the only valid transfer recipient.
    pub deposit_address: String,
    /// Aurora's `quote.amountIn`, raw units — the only valid transfer amount.
    pub amount_in: String,
    /// The ERC-191 signature bound from the wallet by the route engine.
    #[serde(default)]
    pub signature: Value,
}

impl DynAomiTool for AuroraSubmitSignature {
    type App = SommApp;
    type Args = AuroraSubmitSignatureArgs;

    const NAME: &'static str = "somm_aurora_submit_signature";
    const DESCRIPTION: &'static str = "Internal tool — called automatically by the route engine after the user signs the Aurora \
         authorization message. Submits the signature, then stages the source-chain USDC transfer \
         for the user to approve. Do not call directly.";

    fn run_with_routes(
        app: &SommApp,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        // args.wallet was set by somm_aurora_deposit (already session-resolved),
        // not by the model. Re-checking it against the session anyway catches the
        // case the route args cannot: the user switching wallets mid-flow, which
        // would otherwise submit and fund against an address no longer connected.
        let wallet = require_session_wallet(&ctx, Some(&args.wallet))?;

        let signature_hex =
            unwrap_route_str(&args.signature, &["signature", "result", "data", "hex"])
                .ok_or("signature_error: could not read the signature from the wallet")?
                .to_string();

        // Send the RAW hex signature: `/api/execution` applies Aurora's
        // `secp256k1:<base58>` encoding itself, so encoding here too would
        // double-encode and Aurora would reject it.
        app.post(
            &ctx,
            "/api/execution",
            json!({
                "action": "submit_steps",
                "wallet": wallet,
                "executionId": args.execution_id,
                "signature": signature_hex,
            }),
        )?;

        let calldata = erc20_transfer_calldata(&args.deposit_address, &args.amount_in)?;

        ToolReturn::route(json!({
            "status": "awaiting_deposit",
            "message": "Authorization accepted. Please approve the USDC transfer in your wallet.",
            "execution_id": args.execution_id,
        }))
        .next(|next| {
            next.add::<host::StageTx>(json!({
                "to": BASE_USDC,
                "data": { "raw": calldata },
                "value": "0",
                "chain_id": V1_CHAIN_ID,
            }))
            .enforce(EnforcementPolicy::Stop, |enforce| {
                enforce.add::<host::SimulateBatch>(json!({}));
                enforce
                    .add::<host::CommitTxs>(json!({ "aa_preference": "auto" }))
                    .bind_as("deposit_tx_hash");
            });
        })
        .after_named(
            AuroraReportDeposit::NAME,
            json!({
                "execution_id": args.execution_id,
                "wallet": wallet,
            }),
        )
        .awaits("deposit_tx_hash")
        .try_build()
    }
}

// ─── somm_aurora_report_deposit (internal) ───────────────────────────────────

pub struct AuroraReportDeposit;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuroraReportDepositArgs {
    pub execution_id: String,
    pub wallet: String,
    /// The broadcast deposit tx hash, bound by the route engine.
    #[serde(default)]
    pub deposit_tx_hash: Value,
}

impl DynAomiTool for AuroraReportDeposit {
    type App = SommApp;
    type Args = AuroraReportDepositArgs;

    const NAME: &'static str = "somm_aurora_report_deposit";
    const DESCRIPTION: &'static str = "Internal tool — called automatically after the deposit transaction is confirmed. Reports \
         the deposit hash and returns the execution's current status. Do not call directly.";

    fn run(app: &SommApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        // Same mid-flow wallet-switch check as the submit step.
        let wallet = require_session_wallet(&ctx, Some(&args.wallet))?;

        let tx_hash = unwrap_route_str(
            &args.deposit_tx_hash,
            &["deposit_tx_hash", "hash", "transactionHash", "result"],
        )
        .ok_or("deposit_error: could not read the deposit transaction hash")?
        .to_string();

        // The endpoint sources depositAddress from a fresh execution read and
        // fails closed if the execution has none, so we do not pass one.
        app.post(
            &ctx,
            "/api/execution",
            json!({
                "action": "deposit",
                "wallet": wallet,
                "executionId": args.execution_id,
                "txHash": tx_hash,
                "chain": V1_CHAIN,
            }),
        )?;

        let status_resp = app.get(
            &ctx,
            &format!(
                "/api/execution-status?wallet={}&id={}",
                wallet, args.execution_id
            ),
        )?;
        let status = str_field(&status_resp, &["execution", "status"])
            .unwrap_or_else(|| "UNKNOWN".to_string());

        // SUCCESS is asserted by the API from an on-chain delta, never by this
        // tool and never from the transaction's own claimed status. Anything
        // else is reported as the literal status — never as "deployed" or
        // "earning".
        Ok(json!({
            "execution_id": args.execution_id,
            "deposit_tx_hash": tx_hash,
            "status": status,
            "message": match status.as_str() {
                "SUCCESS" => "Deposit settled — the position is live.".to_string(),
                other => format!(
                    "Deposit transaction sent and reported. Execution status is {other}; \
                     it is not settled yet. Do not describe the position as live or earning."
                ),
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_scope_accepts_the_supported_lane() {
        assert!(enforce_v1_scope("Base", "aave-v3", "USDC").is_ok());
        assert!(enforce_v1_scope("base", "AAVE-V3", "usdc").is_ok());
    }

    #[test]
    fn v1_scope_rejects_other_chains_venues_and_assets() {
        assert!(enforce_v1_scope("Arbitrum", "aave-v3", "USDC").is_err());
        assert!(enforce_v1_scope("Base", "compound-v3", "USDC").is_err());
        assert!(enforce_v1_scope("Base", "aave-v3", "USDT").is_err());
    }

    #[test]
    fn format_usdc_renders_the_amount_that_actually_moves() {
        assert_eq!(format_usdc("100000000").as_deref(), Some("100"));
        assert_eq!(format_usdc("100500000").as_deref(), Some("100.5"));
        assert_eq!(format_usdc("1").as_deref(), Some("0.000001"));
        assert_eq!(format_usdc("0").as_deref(), Some("0"));
        // A quote that differs from the request must display the QUOTE.
        assert_eq!(format_usdc("99750000").as_deref(), Some("99.75"));
    }

    #[test]
    fn format_usdc_rejects_non_numeric_input() {
        assert!(format_usdc("1e6").is_none());
        assert!(format_usdc("").is_none());
        assert!(format_usdc("-1").is_none());
    }

    #[test]
    fn transfer_calldata_shape() {
        let cd = erc20_transfer_calldata("0x1111111111111111111111111111111111111111", "1000000")
            .unwrap();
        assert!(cd.starts_with("0xa9059cbb"));
        assert_eq!(cd.len(), 2 + 8 + 64 + 64);
        assert!(cd.contains("1111111111111111111111111111111111111111"));
        assert!(cd.ends_with(&format!("{:064x}", 1_000_000u128)));
    }

    #[test]
    fn transfer_calldata_rejects_malformed_inputs() {
        assert!(erc20_transfer_calldata("0xnothex", "1").is_err());
        assert!(erc20_transfer_calldata("0x1111", "1").is_err());
        assert!(
            erc20_transfer_calldata("0x1111111111111111111111111111111111111111", "1e6").is_err()
        );
    }

    #[test]
    fn str_field_reads_nested_paths_and_misses_safely() {
        let v = json!({ "quote": { "amountIn": "500", "empty": 3 } });
        assert_eq!(
            str_field(&v, &["quote", "amountIn"]).as_deref(),
            Some("500")
        );
        assert_eq!(str_field(&v, &["quote", "missing"]), None);
        assert_eq!(str_field(&v, &["quote", "empty"]), None);
        assert_eq!(str_field(&v, &["nope", "deep"]), None);
    }
}
