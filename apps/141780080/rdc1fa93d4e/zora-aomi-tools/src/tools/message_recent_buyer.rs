use anyhow::{Context, Result};

use crate::{
    client::ZoraClient,
    runtime::WalletRuntime,
    types::{MessageBuyerInput, MessageBuyerOutput},
};

pub async fn message_recent_buyer<R>(
    input: MessageBuyerInput,
    client: &ZoraClient,
    runtime: &R,
) -> Result<MessageBuyerOutput>
where
    R: WalletRuntime,
{
    let swaps = client
        .get_coin_swaps(&input.address, input.chain, 10, None)
        .await?;

    let recent_buyer = swaps
        .data
        .zora20_token
        .and_then(|token| token.swap_activities)
        .map(|activities| activities.edges)
        .unwrap_or_default()
        .into_iter()
        .filter(|edge| edge.node.activity_type.eq_ignore_ascii_case("BUY"))
        .filter_map(|edge| {
            edge.node
                .sender_address
                .map(|address| (address, edge.node.block_timestamp))
        })
        .max_by(|(_, left_time), (_, right_time)| left_time.cmp(right_time))
        .map(|(address, _)| address)
        .context("no recent buyer found")?;

    let receipt = runtime
        .post_zora_comment(&input.address, &input.message)
        .await?;

    Ok(MessageBuyerOutput {
        buyer_address: recent_buyer,
        coin_address: input.address,
        message_sent: input.message,
        tx_hash: Some(receipt.tx_hash),
    })
}
