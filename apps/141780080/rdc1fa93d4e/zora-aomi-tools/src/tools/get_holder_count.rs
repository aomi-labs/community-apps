use anyhow::{Context, Result};

use crate::{
    client::ZoraClient,
    types::{CoinInput, HolderCountOutput},
};

pub async fn get_holder_count(input: CoinInput, client: &ZoraClient) -> Result<HolderCountOutput> {
    let response = client.get_coin(&input.address, input.chain).await?;
    let token = response
        .data
        .zora20_token
        .context("coin not found in Zora response")?;

    Ok(HolderCountOutput {
        address: token.address,
        name: token.name,
        symbol: token.symbol,
        unique_holders: token.unique_holders.unwrap_or_default(),
    })
}
