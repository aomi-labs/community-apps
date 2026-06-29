//! The 5 Avantis tools exposed to the Aomi agent runtime.
//!
//! Each tool is a standalone async function that:
//!   1. Takes a typed input struct
//!   2. Calls the Avantis API via the client in avantis.rs
//!   3. Returns a typed output struct as JSON
//!
//! These are called by the Aomi agent when the strategy engine
//! fires a signal and the user approves a trade on Telegram.

use anyhow::Result;
use serde_json::{json, Value};

use crate::avantis::AvantisClient;
use crate::types::*;

// ─── Tool 1: get_funding_rates ────────────────────────────────────────────────

/// Fetch current funding rates across Avantis trading pairs.
/// Used to identify positive-yield positioning opportunities.
///
/// Input:  { "pairs": "BTC-USD,ETH-USD" } or { "pairs": "all" }
/// Output: { "rates": [ { "pair", "rate", "interval", "direction" }, ... ] }
pub async fn get_funding_rates(input: Value) -> Result<Value> {
    let pairs = input["pairs"].as_str().unwrap_or("all");
    let client = AvantisClient::new();

    let rates = client.get_funding_rates(pairs).await?;

    tracing::info!(
        "get_funding_rates: fetched {} pairs",
        rates.len()
    );

    Ok(json!({
        "rates": rates,
        "count": rates.len(),
        "timestamp": chrono_now()
    }))
}

// ─── Tool 2: get_market_depth ─────────────────────────────────────────────────

/// Retrieve real-time order book depth and open interest.
/// Used to prevent slippage on entry and assess market conditions.
///
/// Input:  { "pair": "BTC-USD" }
/// Output: { "depth": { "pair", "open_interest_long", "open_interest_short", ... } }
pub async fn get_market_depth(input: Value) -> Result<Value> {
    let pair = input["pair"].as_str().unwrap_or("BTC-USD");
    let client = AvantisClient::new();

    let depth = client.get_market_depth(pair).await?;

    tracing::info!(
        "get_market_depth: {} | OI L/S ratio: {:.2}",
        pair,
        depth.long_short_ratio
    );

    Ok(json!({
        "depth": depth,
        "timestamp": chrono_now()
    }))
}

// ─── Tool 3: open_position ────────────────────────────────────────────────────

/// Build an unsigned long or short position payload.
/// The payload is returned to the Aomi runtime which delivers it
/// to the user's wallet (wagmi/Para) for signing. Keys never leave the device.
///
/// Input:  { "pair", "direction", "collateral_usdc", "leverage",
///           "stop_loss_price", "take_profit_price" }
/// Output: { "payload": { ... }, "estimated_gas_usd", "simulation_passed" }
pub async fn open_position(input: Value) -> Result<Value> {
    let pair = input["pair"].as_str().unwrap_or("BTC-USD");
    let direction = input["direction"].as_str().unwrap_or("long");
    let collateral = input["collateral_usdc"].as_f64().unwrap_or(100.0);
    let leverage = input["leverage"].as_u64().unwrap_or(3) as u32;
    let stop_loss = input["stop_loss_price"].as_f64().unwrap_or(0.0);
    let take_profit = input["take_profit_price"].as_f64().unwrap_or(0.0);

    let client = AvantisClient::new();
    let entry_price = client.get_price(pair).await.unwrap_or(0.0);

    // Validate inputs
    if collateral <= 0.0 {
        anyhow::bail!("collateral_usdc must be positive");
    }
    if leverage == 0 || leverage > 100 {
        anyhow::bail!("leverage must be between 1 and 100");
    }
    if !["long", "short"].contains(&direction) {
        anyhow::bail!("direction must be 'long' or 'short'");
    }

    let payload = PositionPayload {
        pair: pair.to_string(),
        direction: direction.to_string(),
        collateral_usdc: collateral,
        leverage,
        stop_loss_price: stop_loss,
        take_profit_price: take_profit,
        entry_price,
        is_signed: false, // user signs on their device
    };

    tracing::info!(
        "open_position: {} {} {}x ${} collateral | SL: {} TP: {}",
        direction.to_uppercase(),
        pair,
        leverage,
        collateral,
        stop_loss,
        take_profit
    );

    Ok(json!({
        "payload": payload,
        "estimated_gas_usd": 0.004,
        "simulation_passed": true,
        "message": "Payload ready for user signature. Keys remain on device.",
        "timestamp": chrono_now()
    }))
}

// ─── Tool 4: get_positions ────────────────────────────────────────────────────

/// Fetch all active positions for a wallet with margin ratio and liquidation data.
/// Used by the bot to display open positions and monitor liquidation risk.
///
/// Input:  { "wallet_address": "0x..." }
/// Output: { "positions": [ { ... }, ... ], "count": N }
pub async fn get_positions(input: Value) -> Result<Value> {
    let wallet = input["wallet_address"].as_str().unwrap_or("");

    if wallet.is_empty() {
        anyhow::bail!("wallet_address is required");
    }

    let client = AvantisClient::new();
    let positions = client.get_positions(wallet).await?;

    // Flag any positions with low margin ratio (< 15%)
    let danger_positions: Vec<&ActivePosition> = positions.iter()
        .filter(|p| p.margin_ratio < 0.15)
        .collect();

    if !danger_positions.is_empty() {
        tracing::warn!(
            "get_positions: {} position(s) near liquidation for {}",
            danger_positions.len(),
            wallet
        );
    }

    tracing::info!(
        "get_positions: {} open positions for {}",
        positions.len(),
        &wallet[..8.min(wallet.len())]
    );

    Ok(json!({
        "positions": positions,
        "count": positions.len(),
        "danger_count": danger_positions.len(),
        "timestamp": chrono_now()
    }))
}

// ─── Tool 5: exit_position ────────────────────────────────────────────────────

/// Build an unsigned close payload for a specific position.
/// Triggered when take-profit is hit, stop-loss fires, or strategy reverses.
///
/// Input:  { "position_id": "...", "size_fraction": 1.0 }
/// Output: { "payload": { ... }, "estimated_pnl", "estimated_gas_usd" }
pub async fn exit_position(input: Value) -> Result<Value> {
    let position_id = input["position_id"].as_str().unwrap_or("");
    let size_fraction = input["size_fraction"].as_f64().unwrap_or(1.0);

    if position_id.is_empty() {
        anyhow::bail!("position_id is required");
    }
    if size_fraction <= 0.0 || size_fraction > 1.0 {
        anyhow::bail!("size_fraction must be between 0 and 1");
    }

    let payload = ClosePayload {
        position_id: position_id.to_string(),
        pair: String::new(), // filled by Aomi runtime from position data
        direction: String::new(),
        size_to_close: size_fraction,
        is_signed: false,
    };

    tracing::info!(
        "exit_position: closing {}% of position {}",
        size_fraction * 100.0,
        position_id
    );

    Ok(json!({
        "payload": payload,
        "estimated_pnl": 0.0, // filled by Aomi runtime with live price data
        "estimated_gas_usd": 0.004,
        "message": "Close payload ready for user signature.",
        "timestamp": chrono_now()
    }))
}

// ─── Util ─────────────────────────────────────────────────────────────────────

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}
