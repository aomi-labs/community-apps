use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::client::{get, rt, urlencode};

#[derive(Clone, Default)]
pub(crate) struct LimiApp;

// ============================================================================
// Tool 1: GetTrendingMarkets
// ============================================================================

pub(crate) struct GetTrendingMarkets;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetTrendingMarketsArgs {
    /// Category filter — pass a category name or numeric ID to narrow results
    /// (e.g. "Crypto", "Politics", "Sports"). Omit to browse all categories.
    #[serde(default)]
    pub category: Option<String>,
    /// How many markets to return. Default 10, max 50.
    #[serde(default)]
    pub limit: Option<u32>,
}

impl DynAomiTool for GetTrendingMarkets {
    type App = LimiApp;
    type Args = GetTrendingMarketsArgs;

    const NAME: &'static str = "limi_get_trending_markets";
    const DESCRIPTION: &'static str = "Browse active Limitless prediction markets. \
        Use when the user asks what markets are trending, wants to discover what's \
        tradeable right now, or wants to see markets in a specific category. \
        Returns market titles, slugs, current YES/NO odds, and 24h volume. \
        Public — no API key needed.";

    fn run(_app: &LimiApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let limit = args.limit.unwrap_or(10).min(50);
        let rt = rt()?;
        rt.block_on(async move {
            let mut path = format!("/markets/active?limit={limit}&page=1");
            if let Some(cat) = args.category {
                path.push_str(&format!("&category={}", urlencode(&cat)));
            }
            get(&path).await
        })
    }
}

// ============================================================================
// Tool 2: ExplainMarket
// ============================================================================

pub(crate) struct ExplainMarket;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ExplainMarketArgs {
    /// Market slug or contract address.
    /// Slug example: `btc-above-100k-jun-2026`
    /// Address example: `0x1234...abcd`
    pub slug_or_address: String,
}

impl DynAomiTool for ExplainMarket {
    type App = LimiApp;
    type Args = ExplainMarketArgs;

    const NAME: &'static str = "limi_explain_market";
    const DESCRIPTION: &'static str = "Fetch full detail and current orderbook for one \
        Limitless market. Use when the user asks to understand a specific market — \
        what it resolves on, current YES/NO odds, implied probability, liquidity depth, \
        and resolution date. The tool returns raw data; summarize it in plain English, \
        including what outcome is more likely and why the odds are where they are. \
        Public — no API key needed.";

    fn run(_app: &LimiApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let slug = args.slug_or_address;
        let rt = rt()?;
        rt.block_on(async move {
            let encoded = urlencode(&slug);
            let market = get(&format!("/markets/{encoded}")).await?;

            // Detect NegRisk group markets (multi-outcome, no single orderbook).
            // They have a `markets` array of child markets.
            let is_group = market
                .get("markets")
                .and_then(|v| v.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if is_group {
                return Ok(json!({
                    "market": market,
                    "market_type": "NegRisk_group",
                    "note": "This is a multi-outcome group market. \
                             Use the child market slugs in the `markets` array \
                             to explain or watch individual outcomes.",
                }));
            }

            // CLOB leaf market — fetch orderbook (sequential, rate-gated).
            let orderbook = get(&format!("/markets/{encoded}/orderbook"))
                .await
                .unwrap_or_else(|_| json!({"bids": [], "asks": [], "note": "orderbook unavailable"}));
            Ok(json!({
                "market": market,
                "orderbook": orderbook,
            }))
        })
    }
}

// ============================================================================
// Tool 3: GetUserPositions
// ============================================================================

pub(crate) struct GetUserPositions;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetUserPositionsArgs {
    /// Base network wallet address to look up (0x...).
    pub address: String,
}

impl DynAomiTool for GetUserPositions {
    type App = LimiApp;
    type Args = GetUserPositionsArgs;

    const NAME: &'static str = "limi_get_user_positions";
    const DESCRIPTION: &'static str = "Fetch open prediction market positions for a Base \
        wallet address on Limitless. Returns active positions with market title, outcome \
        held (YES/NO), size, entry price, current market value, and unrealized PnL. \
        Public endpoint — only a wallet address is needed, no API key.";

    fn run(_app: &LimiApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let address = args.address;
        let rt = rt()?;
        rt.block_on(async move {
            let path = format!(
                "/public-portfolio/positions?address={}",
                urlencode(&address)
            );
            get(&path).await
        })
    }
}

// ============================================================================
// Tool 4: WatchMarketOdds
// ============================================================================

pub(crate) struct WatchMarketOdds;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WatchMarketOddsArgs {
    /// Market slug to watch (e.g. `btc-above-100k-jun-2026`).
    pub slug: String,
    /// Alert threshold in percentage points. The user will be notified when YES
    /// odds shift by at least this amount from the current baseline.
    /// Example: 5 means alert when odds move 5pp (e.g. 64% → 59% or 69%).
    pub threshold_pct: f64,
}

impl DynAomiTool for WatchMarketOdds {
    type App = LimiApp;
    type Args = WatchMarketOddsArgs;

    const NAME: &'static str = "limi_watch_market_odds";
    const DESCRIPTION: &'static str = "Register an odds-drift alert for a Limitless market. \
        Fetches the current market data as the baseline odds snapshot and returns a watch \
        registration payload. IMPORTANT: the real-time monitoring and push notification \
        runs in the Limi Telegram bot, not in this chat. After calling this tool, tell \
        the user to also run `/watch <slug> <threshold>` in the Limi Telegram bot \
        to activate live alerts on their phone.";

    fn run(_app: &LimiApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let slug = args.slug;
        let threshold_pct = args.threshold_pct;
        let rt = rt()?;
        rt.block_on(async move {
            let market = get(&format!("/markets/{}", urlencode(&slug))).await?;
            Ok(json!({
                "watch_slug": slug,
                "threshold_pct": threshold_pct,
                "baseline_snapshot": market,
                "telegram_command": format!("/watch {} {}", slug, threshold_pct),
                "note": "Run the telegram_command in the Limi Telegram bot to enable push alerts.",
            }))
        })
    }
}

// ============================================================================
// Tool 5: DailyBriefing
// ============================================================================

pub(crate) struct DailyBriefing;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DailyBriefingArgs {
    /// Base network wallet address (0x...) for portfolio data.
    pub address: String,
    /// Number of trending markets to include in the briefing. Default 5, max 20.
    #[serde(default)]
    pub market_count: Option<u32>,
}

impl DynAomiTool for DailyBriefing {
    type App = LimiApp;
    type Args = DailyBriefingArgs;

    const NAME: &'static str = "limi_daily_briefing";
    const DESCRIPTION: &'static str = "Generate a morning briefing: top active markets \
        combined with the user's open portfolio positions. Use for `/briefing` or when \
        the user wants a full market + portfolio overview in one call. Format the response \
        as a concise morning brief: top markets with odds, then portfolio positions with \
        PnL, then 24h total PnL. Keep it tight and human — no AI-sounding copy.";

    fn run(_app: &LimiApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let address = args.address;
        let count = args.market_count.unwrap_or(5).min(20);
        let rt = rt()?;
        rt.block_on(async move {
            // Sequential calls — never concurrent — to respect the 2-request
            // concurrency limit and 300ms minimum spacing enforced by `get()`.
            let markets = get(&format!("/markets/active?limit={count}&page=1"))
                .await
                .unwrap_or_else(|e| json!({"error": e}));
            // Rate gate between the two calls is handled inside `get()`.
            let positions = get(&format!(
                "/public-portfolio/positions?address={}",
                urlencode(&address)
            ))
            .await
            .unwrap_or_else(|_| json!({"clob": [], "amm": []}));
            Ok(json!({
                "trending_markets": markets,
                "positions": positions,
            }))
        })
    }
}
