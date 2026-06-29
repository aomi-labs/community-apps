use aomi_sdk::{DynAomiTool, DynToolCallCtx, Secret};
use serde_json::Value;

use crate::{
    load_client_from_env, tools,
    types::{CoinInput, TopBuyersInput},
};

#[derive(Clone, Default)]
pub struct ZoraAomiApp;

pub struct GetHolderCountTool;
pub struct Get24hVolumeTool;
pub struct GetTopBuyersTool;
pub struct MessageRecentBuyerTool;

const SECRET_ZORA_API_KEY: Secret = Secret::new(
    "ZORA_API_KEY",
    "Zora API key for api-sdk.zora.engineering market data requests.",
    true,
);

impl DynAomiTool for GetHolderCountTool {
    type App = ZoraAomiApp;
    type Args = CoinInput;

    const NAME: &'static str = "get_holder_count";
    const DESCRIPTION: &'static str =
        "Returns the number of unique wallet holders for a Zora coin.";

    fn run(_app: &ZoraAomiApp, args: CoinInput, _ctx: DynToolCallCtx) -> Result<Value, String> {
        block_on_tool(async move {
            let client = load_client_from_env()?;
            let output = tools::get_holder_count(args, &client).await?;
            serde_json::to_value(output).map_err(anyhow::Error::from)
        })
    }
}

impl DynAomiTool for Get24hVolumeTool {
    type App = ZoraAomiApp;
    type Args = CoinInput;

    const NAME: &'static str = "get_24h_volume";
    const DESCRIPTION: &'static str =
        "Returns the 24-hour trading volume and market cap delta for a Zora coin.";

    fn run(_app: &ZoraAomiApp, args: CoinInput, _ctx: DynToolCallCtx) -> Result<Value, String> {
        block_on_tool(async move {
            let client = load_client_from_env()?;
            let output = tools::get_24h_volume(args, &client).await?;
            serde_json::to_value(output).map_err(anyhow::Error::from)
        })
    }
}

impl DynAomiTool for GetTopBuyersTool {
    type App = ZoraAomiApp;
    type Args = TopBuyersInput;

    const NAME: &'static str = "get_top_buyers";
    const DESCRIPTION: &'static str =
        "Returns the top N buyers of a Zora coin ranked by total amount bought.";

    fn run(
        _app: &ZoraAomiApp,
        args: TopBuyersInput,
        _ctx: DynToolCallCtx,
    ) -> Result<Value, String> {
        block_on_tool(async move {
            let client = load_client_from_env()?;
            let output = tools::get_top_buyers(args, &client).await?;
            serde_json::to_value(output).map_err(anyhow::Error::from)
        })
    }
}

impl DynAomiTool for MessageRecentBuyerTool {
    type App = ZoraAomiApp;
    type Args = crate::types::MessageBuyerInput;

    const NAME: &'static str = "message_recent_buyer";
    const DESCRIPTION: &'static str =
        "Finds the most recent buyer of a Zora coin and posts an on-chain comment to them. \
         Requires the Aomi wallet execution adapter to be wired in by the host runtime.";

    fn run(
        _app: &ZoraAomiApp,
        args: crate::types::MessageBuyerInput,
        _ctx: DynToolCallCtx,
    ) -> Result<Value, String> {
        block_on_tool(async move {
            let client = load_client_from_env()?;

            // Step 1: find the most recent buyer
            let swaps = client
                .get_coin_swaps(&args.address, args.chain, 10, None)
                .await?;

            let recent_buyer = swaps
                .data
                .zora20_token
                .and_then(|t| t.swap_activities)
                .map(|a| a.edges)
                .unwrap_or_default()
                .into_iter()
                .find(|e| e.node.activity_type == "BUY")
                .and_then(|e| e.node.sender_address)
                .ok_or_else(|| anyhow::anyhow!("no recent buyer found"))?;

            // Step 2: return the composed message for the agent to deliver
            serde_json::to_value(serde_json::json!({
                "buyer_address": recent_buyer,
                "coin_address":  args.address,
                "message":       args.message,
                "status":        "ready_to_send",
                "note":          "on-chain delivery pending wallet adapter"
            }))
            .map_err(anyhow::Error::from)
        })
    }
}

fn block_on_tool<F>(future: F) -> Result<Value, String>
where
    F: std::future::Future<Output = anyhow::Result<Value>>,
{
    let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    runtime.block_on(future).map_err(|err| err.to_string())
}

aomi_sdk::dyn_aomi_app!(
    app = ZoraAomiApp,
    name = "zora-aomi-tools",
    version = "0.1.0",
    preamble = "You are a creator analytics assistant for Zora Coins on Base. \
        Think Daily Wrapped for on-chain creators. \
        When a creator asks about their coin, respond with sharp, human-readable insights — \
        not raw numbers. Examples: '12 new holders today, your top buyer is 0xabc…ef.' \
        or 'Volume is up 40% in the last 24h — your biggest trade was 2.1 ETH.' \
        Use get_holder_count for audience size, get_24h_volume for momentum, \
        get_top_buyers to spotlight whales, and message_recent_buyer to engage fans. \
        Never claim custody of wallets. Never execute trades without explicit confirmation. \
        Keep responses concise — one or two sentences per insight.",
    tools = [
        GetHolderCountTool,
        Get24hVolumeTool,
        GetTopBuyersTool,
        MessageRecentBuyerTool
    ],
    secrets = [SECRET_ZORA_API_KEY],
    namespaces = ["evm-core"],
);
