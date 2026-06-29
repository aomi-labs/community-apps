use anyhow::{bail, Context, Result};
use reqwest::{header, Client};
use serde::Deserialize;

pub const ZORA_BASE_URL: &str = "https://api-sdk.zora.engineering";

#[derive(Debug, Clone)]
pub struct ZoraClient {
    http: Client,
    base_url: String,
}

impl ZoraClient {
    pub fn new(api_key: impl AsRef<str>) -> Result<Self> {
        Self::with_base_url(api_key, ZORA_BASE_URL)
    }

    pub fn with_base_url(api_key: impl AsRef<str>, base_url: impl Into<String>) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert("api-key", header::HeaderValue::from_str(api_key.as_ref())?);

        let http = Client::builder().default_headers(headers).build()?;

        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    pub fn from_env() -> Result<Self> {
        crate::load_client_from_env()
    }

    pub async fn get_coin(&self, address: &str, chain: u64) -> Result<CoinDetailResponse> {
        let url = format!("{}/coin", self.base_url);
        let chain = chain.to_string();

        let response: CoinDetailResponse = send_json(
            self.http
                .get(url)
                .query(&[("address", address), ("chain", chain.as_str())]),
        )
        .await?;

        Ok(response.normalize())
    }

    pub async fn get_coin_swaps(
        &self,
        address: &str,
        chain: u64,
        first: u32,
        after: Option<&str>,
    ) -> Result<CoinSwapsResponse> {
        let url = format!("{}/coinSwaps", self.base_url);
        let mut query = vec![
            ("address", address.to_string()),
            ("chain", chain.to_string()),
            ("first", first.to_string()),
        ];

        if let Some(cursor) = after {
            query.push(("after", cursor.to_string()));
        }

        let response: CoinSwapsResponse = send_json(self.http.get(url).query(&query)).await?;

        Ok(response.normalize())
    }

    pub async fn get_profile_coins(
        &self,
        identifier: &str,
        count: u32,
        chain_ids: &[u64],
    ) -> Result<ProfileCoinsResponse> {
        let url = format!("{}/profileCoins", self.base_url);
        let mut query = vec![
            ("identifier", identifier.to_string()),
            ("count", count.to_string()),
        ];

        for chain_id in chain_ids {
            query.push(("chainIds", chain_id.to_string()));
        }

        send_json(self.http.get(url).query(&query)).await
    }

    pub async fn get_coin_price_history(
        &self,
        address: &str,
        chain: u64,
    ) -> Result<CoinPriceHistoryResponse> {
        let url = format!("{}/coinPriceHistory", self.base_url);
        let chain = chain.to_string();

        send_json(
            self.http
                .get(url)
                .query(&[("address", address), ("chain", chain.as_str())]),
        )
        .await
    }

    pub async fn get_coin_holders(
        &self,
        address: &str,
        chain: u64,
        count: u32,
    ) -> Result<CoinHoldersResponse> {
        let url = format!("{}/coinHolders", self.base_url);
        let query = vec![
            ("address", address.to_string()),
            ("chainId", chain.to_string()),
            ("count", count.to_string()),
        ];

        send_json(self.http.get(url).query(&query)).await
    }
}

async fn send_json<T>(request: reqwest::RequestBuilder) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        bail!("Zora API returned {status}: {body}");
    }

    serde_json::from_str(&body).with_context(|| format!("failed to decode Zora response: {body}"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinDetailResponse {
    #[serde(default, rename = "zora20Token")]
    root_zora20_token: Option<Zora20Token>,
    #[serde(default)]
    pub data: CoinDetailData,
}

impl CoinDetailResponse {
    fn normalize(mut self) -> Self {
        if self.data.zora20_token.is_none() {
            self.data.zora20_token = self.root_zora20_token.take();
        }
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinDetailData {
    pub zora20_token: Option<Zora20Token>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zora20Token {
    pub name: String,
    pub symbol: String,
    pub address: String,
    #[serde(default)]
    pub unique_holders: Option<u64>,
    #[serde(default, rename = "volume24h")]
    pub volume_24h: Option<String>,
    #[serde(default)]
    pub market_cap: Option<String>,
    #[serde(default, rename = "marketCapDelta24h")]
    pub market_cap_delta_24h: Option<String>,
    #[serde(default)]
    pub total_volume: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinSwapsResponse {
    #[serde(default, rename = "zora20Token")]
    root_zora20_token: Option<CoinSwapsToken>,
    #[serde(default)]
    pub data: CoinSwapsData,
}

impl CoinSwapsResponse {
    fn normalize(mut self) -> Self {
        if self.data.zora20_token.is_none() {
            self.data.zora20_token = self.root_zora20_token.take();
        }
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinSwapsData {
    pub zora20_token: Option<CoinSwapsToken>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinSwapsToken {
    pub swap_activities: Option<SwapActivities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapActivities {
    #[serde(default)]
    pub edges: Vec<SwapEdge>,
    pub page_info: Option<PageInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwapEdge {
    pub node: SwapNode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapNode {
    pub activity_type: String,
    pub coin_amount: Option<String>,
    pub sender_address: Option<String>,
    pub block_timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub end_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileCoinsResponse {
    pub profile: Option<ProfileWithCoins>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileWithCoins {
    pub handle: String,
    pub avatar: Option<Avatar>,
    pub creator_coin: Option<CreatorCoinRef>,
    pub created_coins: Option<CreatedCoins>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Avatar {
    #[serde(rename = "previewImage")]
    pub preview_image: PreviewImage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewImage {
    pub small: Option<String>,
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatorCoinRef {
    pub address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedCoins {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub edges: Vec<CreatedCoinEdge>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedCoinEdge {
    pub node: CreatedCoin,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedCoin {
    pub id: String,
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub total_volume: String,
    #[serde(default, rename = "volume24h")]
    pub volume_24h: String,
    #[serde(default, rename = "marketCapDelta24h")]
    pub market_cap_delta_24h: String,
    #[serde(default)]
    pub unique_holders: u64,
    pub token_price: Option<TokenPrice>,
    pub media_content: Option<MediaContent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPrice {
    pub price_in_pool_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaContent {
    pub preview_image: Option<PreviewImage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinPriceHistoryResponse {
    pub zora20_token: Option<CoinPriceHistoryToken>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinPriceHistoryToken {
    #[serde(default)]
    pub one_month: Vec<PricePoint>,
    #[serde(default)]
    pub one_week: Vec<PricePoint>,
    #[serde(default)]
    pub all: Vec<PricePoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PricePoint {
    pub timestamp: String,
    #[serde(rename = "closePrice")]
    pub close_price: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinHoldersResponse {
    pub zora20_token: Option<CoinHoldersToken>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinHoldersToken {
    pub token_balances: HolderBalances,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HolderBalances {
    #[serde(default)]
    pub edges: Vec<HolderEdge>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HolderEdge {
    pub node: HolderNode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HolderNode {
    pub balance: String,
    pub owner_address: String,
}
