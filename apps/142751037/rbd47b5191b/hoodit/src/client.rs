use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) use crate::tool::*;

pub(crate) const ROBINHOOD_CHAIN_ID: u64 = 4663;
const ROBINHOOD_API_BASE: &str = "https://api.robinhood.com/rhj";
const ASSETS_CACHE_TTL: Duration = Duration::from_secs(300);
const QUOTES_CACHE_TTL: Duration = Duration::from_secs(15);
const CORPORATE_ACTIONS_CACHE_TTL: Duration = Duration::from_secs(3600);

type Timed<T> = (Instant, T);

static ASSETS_CACHE: OnceLock<Mutex<Option<Timed<Vec<Asset>>>>> = OnceLock::new();
static QUOTES_CACHE: OnceLock<Mutex<HashMap<String, Timed<Quote>>>> = OnceLock::new();
static CORPORATE_ACTIONS_CACHE: OnceLock<Mutex<Option<Timed<Vec<CorporateAction>>>>> =
    OnceLock::new();

#[derive(Clone, Default)]
pub(crate) struct HooditApp;

#[derive(Clone)]
pub(crate) struct HooditClient {
    http: Client,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Deployment {
    pub(crate) contract_address: String,
    pub(crate) chain_id: u64,
    #[serde(default)]
    pub(crate) network_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) token_symbol: String,
    pub(crate) token_name: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    pub(crate) current_multiplier: String,
    #[serde(default)]
    pub(crate) pending_multiplier: String,
    #[serde(default)]
    pub(crate) pending_multiplier_effective_time: Option<String>,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) logo_url: Option<String>,
    #[serde(default)]
    pub(crate) trading_capabilities: Option<Value>,
    #[serde(default = "default_token_decimals")]
    pub(crate) token_decimals: u32,
    #[serde(default)]
    pub(crate) isin: Option<String>,
}

impl Asset {
    pub(crate) fn robinhood_chain_contract(&self) -> Option<&str> {
        self.deployments
            .iter()
            .find(|deployment| deployment.chain_id == ROBINHOOD_CHAIN_ID)
            .map(|deployment| deployment.contract_address.as_str())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.status == "ASSET_STATUS_ACTIVE"
    }

    pub(crate) fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return true;
        }

        self.token_symbol.to_ascii_lowercase().contains(&query)
            || self.token_name.to_ascii_lowercase().contains(&query)
            || self
                .isin
                .as_deref()
                .is_some_and(|isin| isin.to_ascii_lowercase().contains(&query))
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetsResponse {
    pub(crate) assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Quote {
    pub(crate) token_symbol: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    pub(crate) bid: String,
    pub(crate) ask: String,
    pub(crate) currency: String,
    #[serde(default)]
    pub(crate) daily_trading_volume: Option<String>,
    #[serde(default)]
    pub(crate) is_trading_halt: bool,
    pub(crate) generated_at: String,
    #[serde(default)]
    pub(crate) daily_high: Option<String>,
    #[serde(default)]
    pub(crate) daily_low: Option<String>,
    #[serde(default)]
    pub(crate) mint_burn_token_volume: Option<String>,
    #[serde(default)]
    pub(crate) mint_burn_usd_volume: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuotesResponse {
    pub(crate) quotes: Vec<Quote>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorporateAction {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) action_type: String,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) process_date: Option<Value>,
    pub(crate) token_symbol: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    #[serde(default)]
    pub(crate) details: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorporateActionsResponse {
    pub(crate) corp_actions: Vec<CorporateAction>,
}

impl HooditClient {
    pub(crate) fn new() -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("hoodit-app/0.2 (+https://github.com/aomi-labs/hoodit-apps)"),
        );
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .default_headers(headers)
            .build()
            .map_err(|error| format!("failed to initialize Hoodit HTTP client: {error}"))?;
        Ok(Self { http })
    }

    pub(crate) fn assets(&self) -> Result<Vec<Asset>, String> {
        let cache = ASSETS_CACHE.get_or_init(|| Mutex::new(None));
        if let Some((fetched_at, assets)) = cache.lock().map_err(cache_error)?.as_ref()
            && fetched_at.elapsed() < ASSETS_CACHE_TTL
        {
            return Ok(assets.clone());
        }

        let assets = self.get_robinhood::<AssetsResponse>("/assets")?.assets;
        *cache.lock().map_err(cache_error)? = Some((Instant::now(), assets.clone()));
        Ok(assets)
    }

    pub(crate) fn quote(&self, symbol: &str) -> Result<Quote, String> {
        let symbol = symbol.trim().to_ascii_uppercase();
        let cache = QUOTES_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some((fetched_at, quote)) = cache.lock().map_err(cache_error)?.get(&symbol)
            && fetched_at.elapsed() < QUOTES_CACHE_TTL
        {
            return Ok(quote.clone());
        }

        let path = format!("/prices/{symbol}");
        let quote = self
            .get_robinhood::<QuotesResponse>(&path)?
            .quotes
            .into_iter()
            .next()
            .ok_or_else(|| format!("Robinhood returned no quote for {symbol}"))?;
        cache
            .lock()
            .map_err(cache_error)?
            .insert(symbol, (Instant::now(), quote.clone()));
        Ok(quote)
    }

    pub(crate) fn asset(&self, symbol: &str) -> Result<Asset, String> {
        self.assets()?
            .into_iter()
            .find(|asset| asset.token_symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No Robinhood Stock Token matched ticker {symbol}"))
    }

    pub(crate) fn corporate_actions(&self) -> Result<Vec<CorporateAction>, String> {
        let cache = CORPORATE_ACTIONS_CACHE.get_or_init(|| Mutex::new(None));
        if let Some((fetched_at, actions)) = cache.lock().map_err(cache_error)?.as_ref()
            && fetched_at.elapsed() < CORPORATE_ACTIONS_CACHE_TTL
        {
            return Ok(actions.clone());
        }

        let actions = self
            .get_robinhood::<CorporateActionsResponse>("/corporate-actions")?
            .corp_actions;
        *cache.lock().map_err(cache_error)? = Some((Instant::now(), actions.clone()));
        Ok(actions)
    }

    fn get_robinhood<T>(&self, path: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let request = self.http.get(format!("{ROBINHOOD_API_BASE}{path}"));
        let value = self.send_json(request, "Robinhood Stock Token API")?;
        serde_json::from_value(value)
            .map_err(|error| format!("Robinhood response shape changed: {error}"))
    }

    fn send_json(
        &self,
        request: reqwest::blocking::RequestBuilder,
        source: &str,
    ) -> Result<Value, String> {
        for attempt in 0..3 {
            let response = request
                .try_clone()
                .ok_or_else(|| format!("{source} request could not be retried"))?
                .send()
                .map_err(|_| format!("{source} request failed"))?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                let retry_after = retry_delay_seconds(retry_after, attempt);
                thread::sleep(Duration::from_secs(retry_after));
                continue;
            }

            let body = response
                .text()
                .map_err(|error| format!("{source} response could not be read: {error}"))?;
            if !status.is_success() {
                if status == StatusCode::TOO_MANY_REQUESTS {
                    return Err(format!("{source} rate limited the request; retry shortly"));
                }
                return Err(format!(
                    "{source} returned HTTP {status}: {}",
                    concise_body(&body)
                ));
            }

            return serde_json::from_str(&body)
                .map_err(|error| format!("{source} returned invalid JSON: {error}"));
        }
        Err(format!("{source} rate limited the request; retry shortly"))
    }
}

pub(crate) fn decimal_product(values: &[&str]) -> Option<String> {
    values
        .iter()
        .try_fold(Decimal::ONE, |product, value| {
            Decimal::from_str(value)
                .ok()
                .and_then(|value| product.checked_mul(value))
        })
        .map(normalize_decimal)
}

pub(crate) fn decimal_midpoint(bid: &str, ask: &str) -> Option<String> {
    let bid = Decimal::from_str(bid).ok()?;
    let ask = Decimal::from_str(ask).ok()?;
    bid.checked_add(ask)
        .and_then(|sum| sum.checked_div(Decimal::from(2)))
        .map(normalize_decimal)
}

pub(crate) fn decimal_difference(high: &str, low: &str) -> Option<String> {
    Decimal::from_str(high)
        .ok()?
        .checked_sub(Decimal::from_str(low).ok()?)
        .map(normalize_decimal)
}

pub(crate) fn decimal_spread_bps(bid: &str, ask: &str) -> Option<String> {
    let mid = Decimal::from_str(decimal_midpoint(bid, ask)?.as_str()).ok()?;
    if mid.is_zero() {
        return None;
    }
    Decimal::from_str(decimal_difference(ask, bid)?.as_str())
        .ok()?
        .checked_div(mid)?
        .checked_mul(Decimal::from(10_000))
        .map(normalize_decimal)
}

fn normalize_decimal(value: Decimal) -> String {
    value.normalize().to_string()
}

fn default_token_decimals() -> u32 {
    18
}

fn concise_body(body: &str) -> String {
    const MAX_CHARS: usize = 240;
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if body.chars().count() <= MAX_CHARS {
        return body;
    }
    format!("{}…", body.chars().take(MAX_CHARS).collect::<String>())
}

fn retry_delay_seconds(retry_after: Option<u64>, attempt: u64) -> u64 {
    retry_after.unwrap_or(attempt + 1).clamp(1, 2)
}

fn cache_error<T>(_error: std::sync::PoisonError<T>) -> String {
    "Hoodit response cache is unavailable".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_reference_price_and_spread_without_float_loss() {
        assert_eq!(
            decimal_product(&["320.10", "0.25"]).as_deref(),
            Some("80.025")
        );
        assert_eq!(decimal_midpoint("300", "320.1").as_deref(), Some("310.05"));
        assert_eq!(decimal_difference("101", "99").as_deref(), Some("2"));
        assert_eq!(decimal_spread_bps("99", "101").as_deref(), Some("200"));
        assert_eq!(decimal_spread_bps("0", "0"), None);
    }

    #[test]
    fn bounds_retry_delays_and_upstream_error_bodies() {
        assert_eq!(retry_delay_seconds(Some(30), 0), 2);
        assert_eq!(retry_delay_seconds(Some(0), 0), 1);
        assert_eq!(retry_delay_seconds(None, 1), 2);

        let error = concise_body(&format!("{} secret-tail", "word ".repeat(80)));
        assert!(error.chars().count() <= 241);
        assert!(error.ends_with('…'));
        assert!(!error.contains("secret-tail"));
    }
}
