//! Aomi Playground — a minimal starter app you can clone, edit, and redeploy.
//!
//! This is the source for the example agent you deployed during onboarding.
//! Each tool is a zero-sized type implementing `DynAomiTool`; the
//! `dyn_aomi_app!` macro at the bottom registers them and exposes the plugin to
//! the Aomi backend. To make it yours: edit a tool, add a new one, then run
//! `aomi-build deploy`.

use aomi_sdk::{
    DynAomiTool, DynToolCallCtx, EnforcementPolicy, Secret, ToolReturn, dyn_aomi_app, host,
    resolve_secret_value,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

/// App state. Keep it `Clone + Default` — the runtime constructs one per
/// session. Add fields here (HTTP clients, config) as your app grows.
#[derive(Clone, Default)]
struct PlaygroundApp;

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

// ---------------------------------------------------------------------------
// Shared EVM helpers — chain/token resolution used by both transfer tools.
// ---------------------------------------------------------------------------

const NATIVE_TOKEN_ADDR: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

/// Pull the connected EVM wallet from the host context. `stage_tx` /
/// `commit_txs` always execute as this address — apps never need to pass a
/// `from`.
fn resolve_evm_wallet(ctx: &DynToolCallCtx) -> Result<String, String> {
    ctx.attribute_string(&["domain", "evm", "address"]).ok_or_else(|| {
        "no EVM wallet connected — connect a wallet before sending or swapping".to_string()
    })
}

fn get_chain_id(chain: &str) -> Result<i64, String> {
    match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => Ok(1),
        "polygon" | "matic" => Ok(137),
        "arbitrum" | "arb" => Ok(42161),
        "optimism" | "op" => Ok(10),
        "base" => Ok(8453),
        _ => Err(format!("unsupported chain: {chain}")),
    }
}

fn is_hex_address(s: &str) -> bool {
    s.len() == 42 && s.starts_with("0x") && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Resolve a token argument (symbol or `0x...` address) to `(address, decimals)`.
/// Recognizes a small set of common tokens per chain; anything else must be
/// passed as a literal address with an explicit `decimals` override.
fn resolve_token(chain: &str, token: &str, decimals_override: Option<u8>) -> Result<(String, u8), String> {
    let lower = token.to_lowercase();
    if lower == "eth" || lower == "native" || lower == "matic" {
        return Ok((NATIVE_TOKEN_ADDR.to_string(), 18));
    }
    let known: Option<(&str, u8)> = match (chain, lower.as_str()) {
        ("ethereum", "usdc") => Some(("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6)),
        ("ethereum", "usdt") => Some(("0xdAC17F958D2ee523a2206206994597C13D831ec7", 6)),
        ("ethereum", "dai") => Some(("0x6B175474E89094C44Da98b954EedeAC495271d0F", 18)),
        ("ethereum", "weth") => Some(("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", 18)),
        ("base", "usdc") => Some(("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6)),
        ("base", "weth") => Some(("0x4200000000000000000000000000000000000006", 18)),
        ("arbitrum", "usdc") => Some(("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6)),
        ("arbitrum", "weth") => Some(("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18)),
        _ => None,
    };
    if let Some((addr, dec)) = known {
        return Ok((addr.to_string(), decimals_override.unwrap_or(dec)));
    }
    if is_hex_address(token) {
        let decimals = decimals_override
            .ok_or_else(|| format!("unrecognized token address {token} — pass `decimals` explicitly"))?;
        return Ok((token.to_string(), decimals));
    }
    Err(format!("unknown token `{token}` on chain {chain} — pass a 0x... address and `decimals`"))
}

fn amount_to_base_units(amount: f64, decimals: u8) -> Result<String, String> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err("amount must be a finite positive number".to_string());
    }
    let scaled = amount * 10f64.powi(decimals as i32);
    if scaled > (u128::MAX as f64) {
        return Err("amount is too large to convert to base units".to_string());
    }
    Ok((scaled.round() as u128).to_string())
}

// ---------------------------------------------------------------------------
// Tool 3 — send_token: transfer native ETH or an ERC-20 to a recipient. The
// tool never signs or broadcasts itself — it stages calldata and hands off to
// the host wallet (stage_tx -> simulate_batch -> commit_txs).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
struct SendTokenArgs {
    /// Recipient EVM address.
    recipient: String,
    /// Amount to send, in human-readable units (e.g. 1.5 for 1.5 tokens).
    amount: f64,
    /// Token symbol (e.g. "ETH", "USDC") or 0x... contract address. Defaults to native ETH.
    #[serde(default)]
    token: Option<String>,
    /// Chain name (ethereum, polygon, arbitrum, optimism, base). Defaults to "ethereum".
    #[serde(default)]
    chain: Option<String>,
    /// Override token decimals. Required when `token` is an unrecognized contract address.
    #[serde(default)]
    decimals: Option<u8>,
}

struct SendTokenTool;

impl DynAomiTool for SendTokenTool {
    type App = PlaygroundApp;
    type Args = SendTokenArgs;

    const NAME: &'static str = "send_token";
    const DESCRIPTION: &'static str = "Send native ETH or an ERC-20 token to a recipient address. Stages the transfer calldata and routes it through the host wallet for simulation and signing — do not call stage_tx / simulate_batch / commit_txs yourself.";

    fn run_with_routes(
        _app: &Self::App,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        resolve_evm_wallet(&ctx)?;
        let chain = args.chain.clone().unwrap_or_else(|| "ethereum".to_string());
        let chain_id = get_chain_id(&chain)?;
        let token_label = args.token.clone().unwrap_or_else(|| "ETH".to_string());
        let (token_addr, decimals) = resolve_token(&chain, &token_label, args.decimals)?;
        let amount_wei = amount_to_base_units(args.amount, decimals)?;

        let is_native = token_addr.eq_ignore_ascii_case(NATIVE_TOKEN_ADDR);
        let description = format!(
            "Send {} {} to {} on chain {}",
            args.amount, token_label, args.recipient, chain_id
        );
        let stage_args = if is_native {
            json!({
                "to": args.recipient,
                "description": description,
                "data": { "raw": "0x" },
                "value": amount_wei,
                "kind": "native_transfer",
            })
        } else {
            json!({
                "to": token_addr,
                "description": description,
                "data": {
                    "encode": {
                        "signature": "transfer(address,uint256)",
                        "args": [args.recipient, amount_wei],
                    }
                },
                "value": "0",
                "kind": "erc20_transfer",
            })
        };

        let preview = json!({
            "status": "awaiting_wallet",
            "chain_id": chain_id,
            "token": token_label,
            "recipient": args.recipient,
            "amount": args.amount,
        });

        ToolReturn::route(preview)
            .next(|next| {
                next.add::<host::StageTx>(stage_args)
                    .note(
                        "Stage the transfer. CRITICAL: copy `to` and `data`/`value` byte-for-byte \
                         from the args above. After this step the host automatically simulates and \
                         commits the staged tx and waits for the wallet.",
                    )
                    .enforce(EnforcementPolicy::Continue, |enforce| {
                        enforce.add::<host::SimulateBatch>(json!({}));
                        enforce
                            .add::<host::CommitTxs>(json!({ "aa_preference": "auto" }))
                            .bind_as("transaction_hash");
                    });
            })
            .try_build()
    }
}

// ---------------------------------------------------------------------------
// Tool 4 — swap: quote + execute a same-chain swap via the 0x Swap API v2
// AllowanceHolder route. Requires the ZEROX_API_KEY secret. Same handoff
// pattern as send_token: the tool builds calldata, the host wallet signs it.
// ---------------------------------------------------------------------------

const ZEROX_BASE_URL: &str = "https://api.0x.org";
const ZEROX_ALLOWANCE_HOLDER_SPENDER: &str = "0x0000000000001fF3684f28c67538d4D072C22734";

#[derive(Debug, Deserialize, JsonSchema)]
struct SwapArgs {
    /// Chain name (ethereum, polygon, arbitrum, optimism, base). Defaults to "ethereum".
    #[serde(default)]
    chain: Option<String>,
    /// Sell token symbol (e.g. "ETH", "USDC") or 0x... contract address.
    sell_token: String,
    /// Buy token symbol or 0x... contract address.
    buy_token: String,
    /// Sell amount in human-readable units (e.g. 100.0 for 100 USDC).
    amount: f64,
    /// Slippage tolerance as a decimal (0.005 = 0.5%). Defaults to 0.01.
    #[serde(default)]
    slippage: Option<f64>,
    /// Override decimals for an unrecognized sell-token address.
    #[serde(default)]
    decimals: Option<u8>,
    /// Optional 0x API key. Falls back to the ZEROX_API_KEY secret when omitted.
    #[serde(default)]
    #[schemars(skip)]
    api_key: Option<String>,
}

struct SwapTool;

fn zerox_api_key(ctx: &DynToolCallCtx, arg: Option<&str>) -> Result<String, String> {
    resolve_secret_value(
        ctx,
        arg,
        "ZEROX_API_KEY",
        "missing api_key argument and no ZEROX_API_KEY secret configured — add one in app settings",
    )
}

impl DynAomiTool for SwapTool {
    type App = PlaygroundApp;
    type Args = SwapArgs;

    const NAME: &'static str = "swap";
    const DESCRIPTION: &'static str = "Swap one token for another on the same chain via 0x Swap API v2. Fetches a firm quote, checks whether an ERC-20 approval is needed, then routes the (optional) approval + swap transactions through the host wallet. Do not call stage_tx / simulate_batch / commit_txs yourself.";

    fn run_with_routes(
        _app: &Self::App,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        let wallet = resolve_evm_wallet(&ctx)?;
        let api_key = zerox_api_key(&ctx, args.api_key.as_deref())?;
        let chain = args.chain.clone().unwrap_or_else(|| "ethereum".to_string());
        let chain_id = get_chain_id(&chain)?;
        let (sell_addr, decimals) = resolve_token(&chain, &args.sell_token, args.decimals)?;
        let (buy_addr, _) = resolve_token(&chain, &args.buy_token, None)?;
        let amount_wei = amount_to_base_units(args.amount, decimals)?;
        let slippage = args.slippage.unwrap_or(0.01);
        let slippage_bps = (slippage * 10_000.0).round() as i64;

        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;

        let response = http
            .get(format!("{ZEROX_BASE_URL}/swap/allowance-holder/quote"))
            .header("0x-api-key", &api_key)
            .header("0x-version", "v2")
            .query(&[
                ("chainId", chain_id.to_string()),
                ("sellToken", sell_addr.clone()),
                ("buyToken", buy_addr.clone()),
                ("sellAmount", amount_wei.clone()),
                ("taker", wallet.clone()),
                ("slippageBps", slippage_bps.to_string()),
            ])
            .send()
            .map_err(|e| format!("0x quote request failed: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("0x quote error {status}: {body}"));
        }
        let quote: Value = response
            .json()
            .map_err(|e| format!("failed to parse 0x quote response: {e}"))?;

        let tx = quote
            .get("transaction")
            .and_then(Value::as_object)
            .ok_or_else(|| "0x quote response missing `transaction`".to_string())?;
        let tx_to = tx
            .get("to")
            .and_then(Value::as_str)
            .ok_or_else(|| "0x quote response missing `transaction.to`".to_string())?
            .to_string();
        let tx_data = tx
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| "0x quote response missing `transaction.data`".to_string())?
            .to_string();
        let tx_value = tx.get("value").and_then(Value::as_str).unwrap_or("0").to_string();

        let is_native_sell = sell_addr.eq_ignore_ascii_case(NATIVE_TOKEN_ADDR);
        let needs_approval = !is_native_sell
            && quote
                .get("issues")
                .and_then(|i| i.get("allowance"))
                .map(|a| !a.is_null())
                .unwrap_or(false);

        let mut stage_args: Vec<Value> = Vec::new();
        if needs_approval {
            stage_args.push(json!({
                "to": sell_addr,
                "description": format!("Approve 0x AllowanceHolder for {} on chain {chain_id}", args.sell_token),
                "data": {
                    "encode": {
                        "signature": "approve(address,uint256)",
                        "args": [ZEROX_ALLOWANCE_HOLDER_SPENDER, amount_wei],
                    }
                },
                "value": "0",
                "kind": "erc20_approve",
            }));
        }
        stage_args.push(json!({
            "to": tx_to,
            "description": format!(
                "Swap {} {} -> {} on chain {chain_id} (slippage {}%)",
                args.amount, args.sell_token, args.buy_token, slippage * 100.0
            ),
            "data": { "raw": tx_data },
            "value": tx_value,
            "kind": "swap",
        }));
        let last_index = stage_args.len() - 1;

        let preview = json!({
            "status": "awaiting_wallet",
            "chain_id": chain_id,
            "needs_approval": needs_approval,
            "slippage": slippage,
            "quote": &quote,
        });

        ToolReturn::route(preview)
            .next(|next| {
                for (i, step_args) in stage_args.iter().enumerate() {
                    let step = next.add::<host::StageTx>(step_args.clone());
                    if i == last_index {
                        step.note(
                            "Stage the swap. CRITICAL: copy `data.raw` and `to` byte-for-byte \
                             from the args above — do not abbreviate, reformat, or truncate the \
                             calldata. After this step the host automatically simulates and \
                             commits the staged txs and waits for the wallet.",
                        )
                        .enforce(EnforcementPolicy::Continue, |enforce| {
                            enforce.add::<host::SimulateBatch>(json!({}));
                            enforce
                                .add::<host::CommitTxs>(json!({ "aa_preference": "auto" }))
                                .bind_as("transaction_hash");
                        });
                    } else {
                        step.note(
                            "Stage the ERC-20 approval for the 0x AllowanceHolder. CRITICAL: \
                             copy `data` and `to` byte-for-byte; do not abbreviate or modify.",
                        );
                    }
                }
            })
            .try_build()
    }
}

const ZEROX_API_KEY: Secret = Secret::new(
    "ZEROX_API_KEY",
    "0x Swap API v2 key, used by the `swap` tool to fetch quotes and executable transactions.",
    false,
);

dyn_aomi_app!(
    app = PlaygroundApp,
    name = "playground-example",
    version = "0.2.0",
    preamble = "You are the Aomi Playground example agent. You can echo messages, \
                greet people, send native ETH or ERC-20 tokens, and swap tokens \
                on the same chain via 0x Swap API v2. `send_token` and `swap` \
                never sign or broadcast themselves — they stage calldata and hand \
                off to the host wallet (stage_tx -> simulate_batch -> commit_txs); \
                do not call those host tools directly, and do not modify staged \
                `to`/`data`/`value` fields. The wallet's commit step is the \
                confirmation gate, so once the user has expressed clear intent \
                (\"send 10 USDC to 0x...\", \"swap 1 ETH for USDC\") go ahead and \
                stage the transaction rather than asking again. Encourage the \
                user to clone this repo, edit `src/lib.rs`, and redeploy with \
                `aomi-build` to make it their own.",
    tools = [EchoTool, GreetTool, SendTokenTool, SwapTool],
    secrets = [ZEROX_API_KEY],
    namespaces = ["evm-core"]
);
