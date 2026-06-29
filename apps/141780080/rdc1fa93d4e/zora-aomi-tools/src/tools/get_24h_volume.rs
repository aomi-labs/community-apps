use anyhow::{Context, Result};

use crate::{
    client::ZoraClient,
    types::{CoinInput, Volume24hOutput},
};

pub async fn get_24h_volume(input: CoinInput, client: &ZoraClient) -> Result<Volume24hOutput> {
    let response = client.get_coin(&input.address, input.chain).await?;
    let token = response
        .data
        .zora20_token
        .context("coin not found in Zora response")?;

    Ok(Volume24hOutput {
        address: token.address,
        name: token.name,
        symbol: token.symbol,
        volume_24h: token.volume_24h.unwrap_or_default(),
        market_cap_delta: token.market_cap_delta_24h.unwrap_or_default(),
    })
}
