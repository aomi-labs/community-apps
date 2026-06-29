use serde::{Deserialize, Serialize};

#[cfg(feature = "aomi-plugin")]
use schemars::JsonSchema;

pub fn default_chain() -> u64 {
    std::env::var("DEFAULT_CHAIN_ID")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8453)
}

fn default_top_n() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct CoinInput {
    pub address: String,
    #[serde(default = "default_chain")]
    pub chain: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct HolderCountOutput {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub unique_holders: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct Volume24hOutput {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub volume_24h: String,
    pub market_cap_delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct TopBuyer {
    pub rank: u32,
    pub address: String,
    pub total_bought: String,
    pub trade_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct TopBuyersInput {
    pub address: String,
    #[serde(default = "default_chain")]
    pub chain: u64,
    #[serde(default = "default_top_n")]
    pub top_n: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct MessageBuyerInput {
    pub address: String,
    #[serde(default = "default_chain")]
    pub chain: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "aomi-plugin", derive(JsonSchema))]
pub struct MessageBuyerOutput {
    pub buyer_address: String,
    pub coin_address: String,
    pub message_sent: String,
    pub tx_hash: Option<String>,
}
