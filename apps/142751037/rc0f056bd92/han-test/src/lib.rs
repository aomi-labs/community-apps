//! Aomi Playground — a minimal starter app you can clone, edit, and redeploy.
//!
//! This is the source for the example agent you deployed during onboarding.
//! Each tool is a zero-sized type implementing `DynAomiTool`; the
//! `dyn_aomi_app!` macro at the bottom registers them and exposes the plugin to
//! the Aomi backend. To make it yours: edit a tool, add a new one, then run
//! `aomi-build deploy`.

use aomi_sdk::{DynAomiTool, DynToolCallCtx, Secret, dyn_aomi_app};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// App state. Keep it `Clone + Default` — the runtime constructs one per
/// session. Add fields here (HTTP clients, config) as your app grows.
#[derive(Clone, Default)]
struct PlaygroundApp;

const BINANCE_API_KEY: Secret = Secret::new(
    "BINANCE_API_KEY",
    "Optional Binance API key for authenticated market/account integrations.",
    false,
);
const BINANCE_API_SECRET: Secret = Secret::new(
    "BINANCE_API_SECRET",
    "Optional Binance API secret for authenticated trading integrations.",
    false,
);
const BINANCE_BASE_URL: Secret = Secret::new(
    "BINANCE_BASE_URL",
    "Optional Binance REST base URL; defaults to https://api.binance.com.",
    false,
);

// ---------------------------------------------------------------------------
// Tool 1 — echo: the simplest possible tool (one required string arg).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
struct EchoArgs {
    /// The message to echo back.
    message: String,
}

struct EchoTool;

impl DynAomiTool for EchoTool {
    type App = PlaygroundApp;
    type Args = EchoArgs;

    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back verbatim.";

    fn run(_app: &Self::App, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        Ok(json!({ "message": args.message }))
    }
}

// ---------------------------------------------------------------------------
// Tool 2 — greet: shows optional args + returning structured JSON. Copy this
// shape to build your own tools.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
struct GreetArgs {
    /// Who to greet.
    name: String,
    /// Add an exclamation mark. Defaults to false.
    #[serde(default)]
    excited: bool,
}

struct GreetTool;

impl DynAomiTool for GreetTool {
    type App = PlaygroundApp;
    type Args = GreetArgs;

    const NAME: &'static str = "greet";
    const DESCRIPTION: &'static str = "Greet someone by name, optionally with excitement.";

    fn run(_app: &Self::App, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let punct = if args.excited { "!" } else { "." };
        Ok(json!({ "greeting": format!("Hello, {}{}", args.name, punct) }))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SymbolArgs {
    /// Binance symbol, for example BTCUSDT.
    symbol: String,
}

struct MarketPriceTool;

impl DynAomiTool for MarketPriceTool {
    type App = PlaygroundApp;
    type Args = SymbolArgs;
    const NAME: &'static str = "market_price";
    const DESCRIPTION: &'static str = "Fetch the current Binance spot price for a symbol.";

    fn run(_app: &Self::App, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = args.symbol.trim().to_uppercase();
        if symbol.is_empty() || !symbol.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("symbol must contain only letters and digits, e.g. BTCUSDT".into());
        }
        let base = aomi_sdk::resolve_secret_value(&ctx, None, "BINANCE_BASE_URL", "")
            .unwrap_or_else(|_| "https://api.binance.com".into());
        let url = format!(
            "{}/api/v3/ticker/price?symbol={}",
            base.trim_end_matches('/'),
            symbol
        );
        reqwest::blocking::get(url)
            .map_err(|e| format!("Binance request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("Binance returned an error: {e}"))?
            .json::<Value>()
            .map_err(|e| format!("invalid Binance response: {e}"))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SwapArgs {
    /// Asset to sell, for example ETH.
    from_asset: String,
    /// Asset to buy, for example USDC.
    to_asset: String,
    /// Decimal quantity to sell.
    amount: f64,
    /// Optional maximum slippage percentage for a future execution step.
    #[serde(default)]
    slippage_percent: Option<f64>,
}

struct SwapIntentTool;

impl DynAomiTool for SwapIntentTool {
    type App = PlaygroundApp;
    type Args = SwapArgs;
    const NAME: &'static str = "prepare_swap";
    const DESCRIPTION: &'static str =
        "Validate and prepare a crypto swap intent; does not place an order or move funds.";

    fn run(_app: &Self::App, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if !args.amount.is_finite() || args.amount <= 0.0 {
            return Err("amount must be greater than zero".into());
        }
        let slippage = args.slippage_percent.unwrap_or(0.5);
        if !slippage.is_finite() || !(0.0..=50.0).contains(&slippage) {
            return Err("slippage_percent must be between 0 and 50".into());
        }
        let from = args.from_asset.trim().to_uppercase();
        let to = args.to_asset.trim().to_uppercase();
        if from.is_empty() || to.is_empty() || from == to {
            return Err("from_asset and to_asset must be different non-empty symbols".into());
        }
        Ok(
            json!({"status":"ready_for_quote", "from_asset":from, "to_asset":to, "amount":args.amount, "max_slippage_percent":slippage, "execution": "not_performed"}),
        )
    }
}

dyn_aomi_app!(
    app = PlaygroundApp,
    name = "han-test",
    version = "0.1.0",
    preamble = "You are Han Test, a crypto market helper. You can echo and greet, fetch Binance spot prices, and prepare validated swap intents. Never claim a swap executed: prepare_swap is non-custodial and does not place orders or move funds.",
    tools = [EchoTool, GreetTool, MarketPriceTool, SwapIntentTool],
    secrets = [BINANCE_API_KEY, BINANCE_API_SECRET, BINANCE_BASE_URL],
    namespaces = []
);
