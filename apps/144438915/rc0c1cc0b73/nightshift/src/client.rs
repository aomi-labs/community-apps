//! App marker, the permission envelope constants, and the Jupiter plumbing.
//!
//! The envelope here is not a UI concept. It is the app's real, fixed surface:
//! the pair is hard-wired to USDC -> SOL, the per-action ceiling is a constant
//! checked in code before anything is quoted or staged, and the venue is the
//! Jupiter aggregator. Combined with the build-time `namespaces` declared in
//! `lib.rs`, this is the box the agent lives in. It cannot request a power it
//! was not compiled with, and it cannot exceed a bound checked here.

use std::time::Duration;

use serde_json::{Value, json};

/// Marker struct. `Default` is the registration path the macro uses.
#[derive(Debug, Clone, Default)]
pub struct NightshiftApp;

/// USDC mint on Solana mainnet (6 decimals).
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
/// Wrapped SOL mint.
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_DECIMALS: u32 = 6;

/// Hard ceiling on the value of any single action, in USDC. The agent was
/// built to buy at most this much SOL per action and cannot exceed it.
pub const MAX_PER_ACTION_USDC: f64 = 20.0;
/// Slippage the app will accept, in basis points.
pub const MAX_SLIPPAGE_BPS: u32 = 50;
/// The only venue the app routes through.
pub const VENUE: &str = "Jupiter aggregator";

const JUP_QUOTE: &str = "https://lite-api.jup.ag/swap/v1/quote";
const JUP_SWAP: &str = "https://lite-api.jup.ag/swap/v1/swap";

/// The boundary, checked in code. Deterministic, not a model decision.
pub fn enforce_envelope(usdc_amount: f64) -> Result<(), String> {
    if !(usdc_amount > 0.0) {
        return Err("[nightshift] amount must be greater than zero USDC".to_string());
    }
    if usdc_amount > MAX_PER_ACTION_USDC {
        return Err(format!(
            "[nightshift] ${usdc_amount:.2} exceeds the ${MAX_PER_ACTION_USDC:.2} per-action \
             envelope. This app was compiled to buy at most ${MAX_PER_ACTION_USDC:.2} of SOL per \
             action, USDC into SOL only. It cannot step outside that box."
        ));
    }
    Ok(())
}

/// Convert a USDC dollar amount to base units (6 decimals).
pub fn to_base_units(usdc_amount: f64) -> u64 {
    (usdc_amount * 10f64.powi(USDC_DECIMALS as i32)).round() as u64
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("[nightshift] http client build failed: {e}"))
}

/// Live Jupiter quote for USDC -> SOL. Returns the full quote response so it
/// can both drive the display and be handed back to `/swap` unchanged.
pub fn jupiter_quote(amount_base: u64) -> Result<Value, String> {
    let url = format!(
        "{JUP_QUOTE}?inputMint={USDC_MINT}&outputMint={SOL_MINT}&amount={amount_base}\
         &slippageBps={MAX_SLIPPAGE_BPS}&restrictIntermediateTokens=true"
    );
    let resp = http()?
        .get(url)
        .header("accept", "application/json")
        .send()
        .map_err(|e| format!("[nightshift] Jupiter quote request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "[nightshift] Jupiter quote returned HTTP {}",
            resp.status().as_u16()
        ));
    }
    resp.json::<Value>()
        .map_err(|e| format!("[nightshift] Jupiter quote decode failed: {e}"))
}

/// Ask Jupiter to build the swap transaction for the connected wallet. Jupiter
/// is the producer-of-record for the blob; the host's `svm_stage_tx` decodes
/// it, validates the payer, and stages it for the wallet to sign.
pub fn jupiter_swap_blob(quote: &Value, user_pubkey: &str) -> Result<String, String> {
    let body = json!({
        "quoteResponse": quote,
        "userPublicKey": user_pubkey,
        "wrapAndUnwrapSol": true,
        "dynamicComputeUnitLimit": true,
    });
    let resp = http()?
        .post(JUP_SWAP)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("[nightshift] Jupiter swap build request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "[nightshift] Jupiter swap build returned HTTP {}",
            resp.status().as_u16()
        ));
    }
    let value: Value = resp
        .json()
        .map_err(|e| format!("[nightshift] Jupiter swap decode failed: {e}"))?;
    value
        .get("swapTransaction")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| "[nightshift] Jupiter swap response had no swapTransaction blob".to_string())
}
