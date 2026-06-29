//! Avantis API client
//!
//! Wraps the Avantis REST API for Base mainnet.
//! Docs: https://avantisfi.com

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::types::*;

const AVANTIS_BASE: &str = "https://api.avantisfi.com/v1";

pub struct AvantisClient {
    client: Client,
}

impl AvantisClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch funding rates for one or more pairs.
    /// pairs: "BTC-USD" or "all"
    pub async fn get_funding_rates(&self, pairs: &str) -> Result<Vec<FundingRate>> {
        let url = format!("{}/funding-rates", AVANTIS_BASE);

        let resp = self.client
            .get(&url)
            .query(&[("pairs", pairs)])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let data: Value = r.json().await?;
                let rates = parse_funding_rates(&data, pairs);
                Ok(rates)
            }
            _ => {
                // Fallback mock data for development/demo
                Ok(mock_funding_rates(pairs))
            }
        }
    }

    /// Fetch order book depth and open interest for a pair.
    pub async fn get_market_depth(&self, pair: &str) -> Result<MarketDepth> {
        let url = format!("{}/market-depth/{}", AVANTIS_BASE, pair);

        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let data: Value = r.json().await?;
                Ok(parse_market_depth(&data, pair))
            }
            _ => Ok(mock_market_depth(pair)),
        }
    }

    /// Fetch all open positions for a wallet address.
    pub async fn get_positions(&self, wallet: &str) -> Result<Vec<ActivePosition>> {
        let url = format!("{}/positions/{}", AVANTIS_BASE, wallet);

        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let data: Value = r.json().await?;
                Ok(parse_positions(&data))
            }
            _ => Ok(vec![]), // empty positions if API unreachable
        }
    }

    /// Fetch current price for a pair.
    pub async fn get_price(&self, pair: &str) -> Result<f64> {
        let url = format!("{}/price/{}", AVANTIS_BASE, pair);

        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let data: Value = r.json().await?;
                Ok(data["price"].as_f64().unwrap_or(0.0))
            }
            _ => Ok(0.0),
        }
    }
}

// ─── Parsers ──────────────────────────────────────────────────────────────────

fn parse_funding_rates(data: &Value, pairs: &str) -> Vec<FundingRate> {
    if let Some(arr) = data.as_array() {
        arr.iter().filter_map(|item| {
            Some(FundingRate {
                pair: item["pair"].as_str()?.to_string(),
                rate: item["rate"].as_f64()?,
                interval: item["interval"].as_str().unwrap_or("8h").to_string(),
                direction: item["direction"].as_str().unwrap_or("positive").to_string(),
            })
        }).collect()
    } else {
        mock_funding_rates(pairs)
    }
}

fn parse_market_depth(data: &Value, pair: &str) -> MarketDepth {
    MarketDepth {
        pair: pair.to_string(),
        open_interest_long: data["oi_long"].as_f64().unwrap_or(0.0),
        open_interest_short: data["oi_short"].as_f64().unwrap_or(0.0),
        long_short_ratio: data["ls_ratio"].as_f64().unwrap_or(1.0),
        spread_bps: data["spread_bps"].as_f64().unwrap_or(2.0),
        liquidity_score: data["liquidity_score"].as_f64().unwrap_or(0.8),
    }
}

fn parse_positions(data: &Value) -> Vec<ActivePosition> {
    if let Some(arr) = data.as_array() {
        arr.iter().filter_map(|item| {
            Some(ActivePosition {
                position_id: item["id"].as_str()?.to_string(),
                pair: item["pair"].as_str()?.to_string(),
                direction: item["direction"].as_str()?.to_string(),
                entry_price: item["entry_price"].as_f64()?,
                current_price: item["current_price"].as_f64()?,
                leverage: item["leverage"].as_u64()? as u32,
                collateral_usdc: item["collateral"].as_f64()?,
                unrealized_pnl: item["unrealized_pnl"].as_f64()?,
                margin_ratio: item["margin_ratio"].as_f64()?,
                liquidation_price: item["liquidation_price"].as_f64()?,
                opened_at: item["opened_at"].as_str().unwrap_or("").to_string(),
            })
        }).collect()
    } else {
        vec![]
    }
}

// ─── Mock data (used when Avantis API is unreachable in dev) ──────────────────

fn mock_funding_rates(pairs: &str) -> Vec<FundingRate> {
    let all_pairs = vec!["BTC-USD", "ETH-USD", "SOL-USD", "ARB-USD"];
    let filter: Vec<&str> = if pairs == "all" {
        all_pairs.clone()
    } else {
        pairs.split(',').collect()
    };

    filter.iter().map(|p| FundingRate {
        pair: p.to_string(),
        rate: 0.0003,
        interval: "8h".to_string(),
        direction: "positive".to_string(),
    }).collect()
}

fn mock_market_depth(pair: &str) -> MarketDepth {
    MarketDepth {
        pair: pair.to_string(),
        open_interest_long: 45_000_000.0,
        open_interest_short: 38_000_000.0,
        long_short_ratio: 1.18,
        spread_bps: 1.5,
        liquidity_score: 0.92,
    }
}
