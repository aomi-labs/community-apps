use serde::{Deserialize, Serialize};

// ─── Avantis API response types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundingRate {
    pub pair: String,
    pub rate: f64,        // funding rate as a decimal (e.g. 0.0003 = 0.03%)
    pub interval: String, // "8h"
    pub direction: String,// "positive" (longs pay shorts) or "negative"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketDepth {
    pub pair: String,
    pub open_interest_long: f64,  // USD value
    pub open_interest_short: f64, // USD value
    pub long_short_ratio: f64,    // OI long / OI short
    pub spread_bps: f64,          // bid/ask spread in basis points
    pub liquidity_score: f64,     // 0–1, higher = more liquid
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionPayload {
    pub pair: String,
    pub direction: String,    // "long" | "short"
    pub collateral_usdc: f64,
    pub leverage: u32,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub entry_price: f64,
    pub is_signed: bool,      // always false — user signs on their device
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivePosition {
    pub position_id: String,
    pub pair: String,
    pub direction: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub leverage: u32,
    pub collateral_usdc: f64,
    pub unrealized_pnl: f64,
    pub margin_ratio: f64,     // 0–1; below 0.1 = danger zone
    pub liquidation_price: f64,
    pub opened_at: String,     // ISO timestamp
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClosePayload {
    pub position_id: String,
    pub pair: String,
    pub direction: String,
    pub size_to_close: f64,   // 1.0 = full close
    pub is_signed: bool,      // always false
}

// ─── Tool input/output wrappers ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct GetFundingRatesInput {
    /// Comma-separated pairs, e.g. "BTC-USD,ETH-USD" or "all"
    pub pairs: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetFundingRatesOutput {
    pub rates: Vec<FundingRate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMarketDepthInput {
    pub pair: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMarketDepthOutput {
    pub depth: MarketDepth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionInput {
    pub pair: String,
    pub direction: String,
    pub collateral_usdc: f64,
    pub leverage: u32,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionOutput {
    pub payload: PositionPayload,
    pub estimated_gas_usd: f64,
    pub simulation_passed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPositionsInput {
    /// Wallet address to fetch positions for
    pub wallet_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPositionsOutput {
    pub positions: Vec<ActivePosition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExitPositionInput {
    pub position_id: String,
    /// Fraction to close: 1.0 = full, 0.5 = half
    pub size_fraction: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExitPositionOutput {
    pub payload: ClosePayload,
    pub estimated_pnl: f64,
    pub estimated_gas_usd: f64,
}
