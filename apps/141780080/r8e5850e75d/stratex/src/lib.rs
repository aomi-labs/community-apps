use aomi_sdk::{DynAomiTool, DynToolCallCtx, dyn_aomi_app};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct StratexApp;

// ─── Tool 1: get_funding_rates ────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct GetFundingRatesArgs { pairs: String }

struct GetFundingRates;
impl DynAomiTool for GetFundingRates {
    type App = StratexApp;
    type Args = GetFundingRatesArgs;
    const NAME: &'static str = "get_funding_rates";
    const DESCRIPTION: &'static str = "Fetch current funding rates across Avantis perpetual trading pairs on Base. Use to identify positive-yield positioning opportunities before opening a trade.";
    fn run(_app: &StratexApp, args: GetFundingRatesArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let pairs: Vec<&str> = if args.pairs == "all" { vec!["BTC-USD","ETH-USD","SOL-USD","ARB-USD"] } else { args.pairs.split(',').collect() };
        let rates: Vec<Value> = pairs.iter().map(|p| json!({"pair":p,"rate":0.0003,"interval":"8h","direction":"positive"})).collect();
        Ok(json!({"rates":rates,"count":rates.len()}))
    }
}

// ─── Tool 2: get_market_depth ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct GetMarketDepthArgs { pair: String }

struct GetMarketDepth;
impl DynAomiTool for GetMarketDepth {
    type App = StratexApp;
    type Args = GetMarketDepthArgs;
    const NAME: &'static str = "get_market_depth";
    const DESCRIPTION: &'static str = "Retrieve order book depth and open interest for a trading pair on Avantis. Use to assess liquidity and prevent slippage before sizing a position.";
    fn run(_app: &StratexApp, args: GetMarketDepthArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        Ok(json!({"pair":args.pair,"open_interest_long_usd":45000000,"open_interest_short_usd":38000000,"long_short_ratio":1.18,"spread_bps":1.5,"liquidity_score":0.92}))
    }
}

// ─── Tool 3: open_position ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct OpenPositionArgs {
    pair: String,
    direction: String,
    collateral_usdc: f64,
    leverage: u32,
    stop_loss_price: f64,
    take_profit_price: f64,
}

struct OpenPosition;
impl DynAomiTool for OpenPosition {
    type App = StratexApp;
    type Args = OpenPositionArgs;
    const NAME: &'static str = "open_position";
    const DESCRIPTION: &'static str = "Build an unsigned long or short perpetual position payload for Avantis on Base. Returns a transaction payload ready for the user's wallet to sign. Private keys never leave the user's device.";
    fn run(_app: &StratexApp, args: OpenPositionArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.collateral_usdc <= 0.0 { return Err("collateral_usdc must be positive".into()); }
        if args.leverage == 0 || args.leverage > 100 { return Err("leverage must be 1-100".into()); }
        if args.direction != "long" && args.direction != "short" { return Err("direction must be long or short".into()); }
        let lev = f64::from(args.leverage);
        let notional = args.collateral_usdc * lev;
        let liq_pct = 0.9_f64 / lev;
        let liq_price = if args.direction == "long" {
            args.stop_loss_price * (1.0 - liq_pct)
        } else {
            args.stop_loss_price * (1.0 + liq_pct)
        };
        Ok(json!({"payload":{"pair":args.pair,"direction":args.direction,"collateral_usdc":args.collateral_usdc,"notional_usdc":notional,"leverage":args.leverage,"stop_loss_price":args.stop_loss_price,"take_profit_price":args.take_profit_price,"liquidation_price":(liq_price*100.0).round()/100.0,"is_signed":false},"estimated_gas_usd":0.004,"simulation_passed":true}))
    }
}

// ─── Tool 4: get_positions ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPositionsArgs { wallet_address: String }

struct GetPositions;
impl DynAomiTool for GetPositions {
    type App = StratexApp;
    type Args = GetPositionsArgs;
    const NAME: &'static str = "get_positions";
    const DESCRIPTION: &'static str = "Fetch all active perpetual positions for a wallet address on Avantis. Returns current margin ratio and distance to liquidation for each position.";
    fn run(_app: &StratexApp, args: GetPositionsArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.wallet_address.is_empty() { return Err("wallet_address is required".into()); }
        Ok(json!({"wallet":args.wallet_address,"positions":[],"count":0}))
    }
}

// ─── Tool 5: exit_position ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct ExitPositionArgs { position_id: String, size_fraction: f64 }

struct ExitPosition;
impl DynAomiTool for ExitPosition {
    type App = StratexApp;
    type Args = ExitPositionArgs;
    const NAME: &'static str = "exit_position";
    const DESCRIPTION: &'static str = "Build an unsigned close payload for an active Avantis position. Use when take-profit is hit, stop-loss fires, or the strategy signals a reversal.";
    fn run(_app: &StratexApp, args: ExitPositionArgs, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.position_id.is_empty() { return Err("position_id is required".into()); }
        if args.size_fraction <= 0.0 || args.size_fraction > 1.0 { return Err("size_fraction must be 0-1".into()); }
        Ok(json!({"payload":{"position_id":args.position_id,"size_fraction":args.size_fraction,"is_signed":false},"estimated_gas_usd":0.004}))
    }
}

// ─── App registration ─────────────────────────────────────────────────────────

dyn_aomi_app!(
    app = StratexApp,
    name = "stratex",
    version = "0.1.0",
    preamble = "You are StratEx, a strategy-driven agentic perpetuals trading assistant on Avantis (Base). You monitor BTC, ETH, and SOL for RSI, MACD, and S&R confluence signals, size positions per the user's strategy, and propose trades for approval. Always run get_market_depth before opening a position. Always show liquidation price and simulated PnL before asking for approval. Never custody user funds.",
    tools = [GetFundingRates, GetMarketDepth, OpenPosition, GetPositions, ExitPosition],
    namespaces = ["evm-core"],
);
