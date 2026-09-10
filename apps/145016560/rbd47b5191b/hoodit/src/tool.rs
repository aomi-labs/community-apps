use crate::client::*;
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) struct SearchStockTokens;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchStockTokensArgs {
    /// Ticker, company name, token name, or ISIN. An empty query lists active assets.
    query: String,
    /// Maximum results to return. Defaults to 10 and is capped at 50.
    #[serde(default)]
    limit: Option<usize>,
    /// Exclude inactive assets. Defaults to true.
    #[serde(default)]
    active_only: Option<bool>,
}

impl DynAomiTool for SearchStockTokens {
    type App = HooditApp;
    type Args = SearchStockTokensArgs;
    const NAME: &'static str = "hoodit_search_stock_tokens";
    const DESCRIPTION: &'static str = "Search Robinhood's live Stock Token catalog by ticker, company, token name, or ISIN. Use this before trading when the user's symbol is missing or ambiguous; returns canonical Robinhood Chain contracts but does not prepare a transaction.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let assets = select_assets(
            HooditClient::new()?.assets()?,
            &args.query,
            args.active_only.unwrap_or(true),
            args.limit.unwrap_or(10),
        );
        let assets = assets.iter().map(asset_json).collect::<Vec<_>>();
        Ok(json!({
            "source": "Robinhood Stock Token API /rhj/assets",
            "chain_id": ROBINHOOD_CHAIN_ID,
            "query": args.query,
            "count": assets.len(),
            "assets": assets,
        }))
    }
}

pub(crate) struct GetStockSnapshot;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetStockSnapshotArgs {
    /// Exact Stock Token ticker, for example AAPL or NVDA. Search first when ambiguous.
    symbol: String,
}

impl DynAomiTool for GetStockSnapshot {
    type App = HooditApp;
    type Args = GetStockSnapshotArgs;
    const NAME: &'static str = "hoodit_get_stock_snapshot";
    const DESCRIPTION: &'static str = "Get trade-decision context for one Robinhood Stock Token: canonical contract, raw underlying bid/ask, corporate-action multiplier, adjusted token reference, halt status, data timestamp, and recent corporate actions. Call this before every buy or sell.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = normalize_symbol(&args.symbol)?;
        let client = HooditClient::new()?;
        let asset = client.asset(&symbol)?;
        let quote = client.quote(&symbol)?;
        let actions = client
            .corporate_actions()?
            .into_iter()
            .filter(|action| action.token_symbol.eq_ignore_ascii_case(&symbol))
            .take(5)
            .map(corporate_action_json)
            .collect::<Vec<_>>();

        let underlying_mid = decimal_midpoint(&quote.bid, &quote.ask);
        let underlying_spread = decimal_difference(&quote.ask, &quote.bid);
        let underlying_spread_bps = decimal_spread_bps(&quote.bid, &quote.ask);
        let token_bid = decimal_product(&[&quote.bid, &asset.current_multiplier]);
        let token_ask = decimal_product(&[&quote.ask, &asset.current_multiplier]);
        let token_mid = underlying_mid
            .as_deref()
            .and_then(|mid| decimal_product(&[mid, &asset.current_multiplier]));

        let mut warnings = Vec::new();
        if quote.is_trading_halt {
            warnings.push("The underlying equity is currently halted; do not prepare a trade.");
        }
        if !asset.is_active() {
            warnings.push("Robinhood marks this Stock Token inactive; do not prepare a trade.");
        }
        if !asset.pending_multiplier.is_empty() {
            warnings.push("A corporate-action multiplier change is pending; re-check immediately before trading.");
        }

        Ok(json!({
            "source": "Robinhood Stock Token API",
            "chain_id": ROBINHOOD_CHAIN_ID,
            "asset": asset_json(&asset),
            "underlying_market": quote_json(&quote),
            "underlying_spread": {
                "absolute": underlying_spread,
                "basis_points_at_mid": underlying_spread_bps,
            },
            "token_reference": {
                "method": "raw underlying price multiplied by current_multiplier",
                "currency": quote.currency,
                "bid": token_bid,
                "ask": token_ask,
                "mid": token_mid,
                "current_multiplier": asset.current_multiplier,
                "not_an_executable_quote": true
            },
            "recent_corporate_actions": actions,
            "warnings": warnings,
            "execution_note": "Use the canonical resolver and a fresh LI.FI quote for an executable trade price."
        }))
    }
}

pub(crate) struct GetCorporateActions;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetCorporateActionsArgs {
    /// Optional Stock Token ticker. Omit to list actions across all supported assets.
    #[serde(default)]
    symbol: Option<String>,
    /// When true, return only in-progress actions. Defaults to false.
    #[serde(default)]
    in_progress_only: Option<bool>,
    /// Maximum results. Defaults to 10 and is capped at 50.
    #[serde(default)]
    limit: Option<usize>,
}

impl DynAomiTool for GetCorporateActions {
    type App = HooditApp;
    type Args = GetCorporateActionsArgs;
    const NAME: &'static str = "hoodit_get_corporate_actions";
    const DESCRIPTION: &'static str = "List recent or pending Robinhood Stock Token corporate actions such as dividends and splits. Use before trading around an adjustment or when explaining a multiplier change.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = args.symbol.as_deref().map(normalize_symbol).transpose()?;
        let limit = args.limit.unwrap_or(10).clamp(1, 50);
        let actions = select_corporate_actions(
            HooditClient::new()?.corporate_actions()?,
            symbol.as_deref(),
            args.in_progress_only.unwrap_or(false),
            limit,
        )
        .into_iter()
        .map(corporate_action_json)
        .collect::<Vec<_>>();

        Ok(json!({
            "source": "Robinhood Stock Token API /rhj/corporate-actions",
            "symbol": symbol,
            "count": actions.len(),
            "actions": actions,
        }))
    }
}

fn select_assets(assets: Vec<Asset>, query: &str, active_only: bool, limit: usize) -> Vec<Asset> {
    let mut assets = assets
        .into_iter()
        .filter(|asset| !active_only || asset.is_active())
        .filter(|asset| asset.matches(query))
        .filter(|asset| asset.robinhood_chain_contract().is_some())
        .collect::<Vec<_>>();
    assets.sort_by_key(|asset| asset_rank(asset, query));
    assets.truncate(limit.clamp(1, 50));
    assets
}

fn select_corporate_actions(
    actions: Vec<CorporateAction>,
    symbol: Option<&str>,
    in_progress_only: bool,
    limit: usize,
) -> Vec<CorporateAction> {
    actions
        .into_iter()
        .filter(|action| {
            symbol.is_none_or(|symbol| action.token_symbol.eq_ignore_ascii_case(symbol))
        })
        .filter(|action| {
            !in_progress_only || action.status == "CORPORATE_ACTION_STATUS_IN_PROGRESS"
        })
        .take(limit.clamp(1, 50))
        .collect()
}

fn asset_rank(asset: &Asset, query: &str) -> u8 {
    let query = query.trim().to_ascii_lowercase();
    if asset.token_symbol.eq_ignore_ascii_case(&query) {
        0
    } else if asset
        .token_name
        .to_ascii_lowercase()
        .strip_suffix(" • robinhood token")
        .is_some_and(|name| name == query)
    {
        1
    } else if asset.token_symbol.to_ascii_lowercase().starts_with(&query) {
        2
    } else {
        3
    }
}

fn asset_json(asset: &Asset) -> Value {
    let deployment = asset
        .deployments
        .iter()
        .find(|deployment| deployment.chain_id == ROBINHOOD_CHAIN_ID);
    json!({
        "id": asset.id,
        "symbol": asset.token_symbol,
        "name": asset.token_name,
        "isin": asset.isin,
        "status": asset.status,
        "chain_id": ROBINHOOD_CHAIN_ID,
        "contract_address": deployment.map(|deployment| deployment.contract_address.as_str()),
        "network_name": deployment.and_then(|deployment| deployment.network_name.as_deref()),
        "token_decimals": asset.token_decimals,
        "current_multiplier": asset.current_multiplier,
        "pending_multiplier": asset.pending_multiplier,
        "pending_multiplier_effective_time": asset.pending_multiplier_effective_time,
        "trading_capabilities": asset.trading_capabilities,
        "logo_url": asset.logo_url,
    })
}

fn quote_json(quote: &Quote) -> Value {
    json!({
        "symbol": quote.token_symbol,
        "bid": quote.bid,
        "ask": quote.ask,
        "mid": decimal_midpoint(&quote.bid, &quote.ask),
        "currency": quote.currency,
        "daily_high": quote.daily_high,
        "daily_low": quote.daily_low,
        "daily_trading_volume": quote.daily_trading_volume,
        "mint_burn_token_volume": quote.mint_burn_token_volume,
        "mint_burn_usd_volume": quote.mint_burn_usd_volume,
        "is_trading_halt": quote.is_trading_halt,
        "generated_at": quote.generated_at,
        "deployments": quote.deployments,
        "price_semantics": "raw underlying-equity price; not multiplier-adjusted"
    })
}

fn corporate_action_json(action: CorporateAction) -> Value {
    json!({
        "id": action.id,
        "symbol": action.token_symbol,
        "type": action.action_type,
        "status": action.status,
        "process_date": action.process_date,
        "details": action.details,
        "deployments": action.deployments,
    })
}

fn normalize_symbol(symbol: &str) -> Result<String, String> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || symbol.len() > 12
        || !symbol
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err("Provide a valid Stock Token ticker such as AAPL or NVDA".to_string());
    }
    Ok(symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(symbol: &str, name: &str, status: &str, chain_id: u64) -> Asset {
        Asset {
            id: symbol.into(),
            token_symbol: symbol.into(),
            token_name: name.into(),
            deployments: vec![Deployment {
                contract_address: format!("0x{symbol:0>40}"),
                chain_id,
                network_name: None,
            }],
            current_multiplier: "1".into(),
            pending_multiplier: String::new(),
            pending_multiplier_effective_time: None,
            status: status.into(),
            logo_url: None,
            trading_capabilities: None,
            token_decimals: 18,
            isin: None,
        }
    }

    fn action(symbol: &str, status: &str) -> CorporateAction {
        CorporateAction {
            id: format!("{symbol}-{status}"),
            action_type: "CORPORATE_ACTION_TYPE_SPLIT".into(),
            status: status.into(),
            process_date: None,
            token_symbol: symbol.into(),
            deployments: Vec::new(),
            details: Value::Null,
        }
    }

    #[test]
    fn validates_stock_symbol_inputs() {
        assert_eq!(normalize_symbol(" aapl ").as_deref(), Ok("AAPL"));
        assert!(normalize_symbol("AAPL/USD").is_err());
    }

    #[test]
    fn catalog_search_filters_sorts_and_limits() {
        let selected = select_assets(
            vec![
                asset(
                    "AAPD",
                    "Apple Daily Stock Token",
                    "ASSET_STATUS_ACTIVE",
                    4663,
                ),
                asset(
                    "AAPL",
                    "Apple • Robinhood Token",
                    "ASSET_STATUS_ACTIVE",
                    4663,
                ),
                asset(
                    "OLD",
                    "Apple Old Stock Token",
                    "ASSET_STATUS_INACTIVE",
                    4663,
                ),
                asset(
                    "OFF",
                    "Apple Offchain Stock Token",
                    "ASSET_STATUS_ACTIVE",
                    1,
                ),
            ],
            "Apple",
            true,
            1,
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].token_symbol, "AAPL");
    }

    #[test]
    fn corporate_actions_filter_by_symbol_status_and_limit() {
        let selected = select_corporate_actions(
            vec![
                action("AAPL", "CORPORATE_ACTION_STATUS_COMPLETE"),
                action("NVDA", "CORPORATE_ACTION_STATUS_IN_PROGRESS"),
                action("AAPL", "CORPORATE_ACTION_STATUS_IN_PROGRESS"),
                action("AAPL", "CORPORATE_ACTION_STATUS_IN_PROGRESS"),
            ],
            Some("AAPL"),
            true,
            1,
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].token_symbol, "AAPL");
        assert_eq!(selected[0].status, "CORPORATE_ACTION_STATUS_IN_PROGRESS");
    }
}
