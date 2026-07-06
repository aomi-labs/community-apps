//! Four tools. Three reads, one write.
//!
//! Build-tx tools return `{ to, data, value, chainId }`. The runtime simulates
//! and the user's wallet signs and broadcasts.

use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use num_bigint::BigUint;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::abi;
use crate::chain::{MONAD_CHAIN_ID, WMON_ADDRESS};
use crate::rpc;

#[derive(Clone, Default)]
pub(crate) struct MonadOneShotApp;

// ─── helpers ────────────────────────────────────────────────────────────────

fn to_human(amount_wei: &BigUint, decimals: u8) -> String {
    let s = amount_wei.to_str_radix(10);
    if decimals == 0 {
        return s;
    }
    let d = decimals as usize;
    let out = if s.len() <= d {
        let zeros = "0".repeat(d - s.len());
        format!("0.{zeros}{s}")
    } else {
        let split = s.len() - d;
        let (whole, frac) = s.split_at(split);
        format!("{whole}.{frac}")
    };
    let mut out = out;
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

fn from_human(amount_human: &str, decimals: u8) -> Result<String, String> {
    let s = amount_human.trim();
    if s.is_empty() {
        return Err("amount is empty".into());
    }
    let (whole, frac) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("amount `{s}` is not a decimal number"));
    }
    let d = decimals as usize;
    let frac_padded = if frac.len() >= d {
        frac[..d].to_string()
    } else {
        format!("{frac}{}", "0".repeat(d - frac.len()))
    };
    let combined = format!("{whole}{frac_padded}");
    let trimmed = combined.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.to_string()
    })
}

fn wei_hex(decimal_wei: &str) -> Result<String, String> {
    let big = BigUint::parse_bytes(decimal_wei.as_bytes(), 10)
        .ok_or_else(|| format!("invalid wei: {decimal_wei}"))?;
    Ok(format!("0x{}", big.to_str_radix(16)))
}

fn normalize_token(token: &str) -> Result<&'static str, String> {
    match token.to_ascii_uppercase().as_str() {
        "MON" => Ok("MON"),
        "WMON" => Ok("WMON"),
        other => Err(format!("unknown token `{other}` — use MON or WMON")),
    }
}

// ─── Tool: get_balance ──────────────────────────────────────────────────────

pub(crate) struct GetBalance;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetBalanceArgs {
    /// EVM address whose balance to read.
    pub(crate) owner: String,
    /// Either "MON" (native) or "WMON" (wrapped). Defaults to MON.
    #[serde(default)]
    pub(crate) token: Option<String>,
}

impl DynAomiTool for GetBalance {
    type App = MonadOneShotApp;
    type Args = GetBalanceArgs;

    const NAME: &'static str = "get_balance";
    const DESCRIPTION: &'static str =
        "Read MON (native) or WMON (wrapped) balance for an address on Monad mainnet. Returns \
         a raw wei amount and a human-readable string. Use this before building any swap or send.";

    fn run(_app: &MonadOneShotApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let token = normalize_token(args.token.as_deref().unwrap_or("MON"))?;
        let _ = abi::parse_address(&args.owner)?; // validate

        let raw_hex = match token {
            "MON" => rpc::eth_get_balance(&args.owner)?,
            "WMON" => {
                let owner_bytes = abi::parse_address(&args.owner)?;
                let data = abi::balance_of(&owner_bytes);
                rpc::eth_call(WMON_ADDRESS, &abi::to_hex(&data))?
            }
            _ => unreachable!(),
        };

        let wei = abi::decode_uint_word(&raw_hex)?;
        Ok(json!({
            "owner": args.owner,
            "token": token,
            "decimals": 18,
            "amount_wei": wei.to_str_radix(10),
            "amount": to_human(&wei, 18),
        }))
    }
}

// ─── Tool: wrap_mon ─────────────────────────────────────────────────────────

pub(crate) struct WrapMon;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WrapMonArgs {
    /// Amount of MON to wrap, in human units (e.g. "0.5" for 0.5 MON).
    pub(crate) amount: String,
}

impl DynAomiTool for WrapMon {
    type App = MonadOneShotApp;
    type Args = WrapMonArgs;

    const NAME: &'static str = "wrap_mon";
    const DESCRIPTION: &'static str =
        "Build an unsigned transaction that wraps MON into WMON by calling `deposit()` on the \
         WMON contract. Returns `{ to, data, value, chainId }`. The caller's address (the wallet \
         that signs) receives the WMON automatically.";

    fn run(_app: &MonadOneShotApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let amount_wei = from_human(&args.amount, 18)?;
        let calldata = abi::deposit();
        let value_hex = wei_hex(&amount_wei)?;

        Ok(json!({
            "chainId": MONAD_CHAIN_ID,
            "to": WMON_ADDRESS,
            "data": abi::to_hex(&calldata),
            "value": value_hex,
            "meta": {
                "action": "wrap_mon",
                "amount": args.amount,
                "amount_wei": amount_wei,
                "contract": WMON_ADDRESS,
            }
        }))
    }
}

// ─── Tool: unwrap_wmon ──────────────────────────────────────────────────────

pub(crate) struct UnwrapWmon;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UnwrapWmonArgs {
    /// Amount of WMON to unwrap, in human units.
    pub(crate) amount: String,
}

impl DynAomiTool for UnwrapWmon {
    type App = MonadOneShotApp;
    type Args = UnwrapWmonArgs;

    const NAME: &'static str = "unwrap_wmon";
    const DESCRIPTION: &'static str =
        "Build an unsigned transaction that unwraps WMON back to MON by calling \
         `withdraw(uint256)` on the WMON contract. The caller's address receives the MON.";

    fn run(_app: &MonadOneShotApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let amount_wei = from_human(&args.amount, 18)?;
        let calldata = abi::withdraw(&amount_wei)?;

        Ok(json!({
            "chainId": MONAD_CHAIN_ID,
            "to": WMON_ADDRESS,
            "data": abi::to_hex(&calldata),
            "value": "0x0",
            "meta": {
                "action": "unwrap_wmon",
                "amount": args.amount,
                "amount_wei": amount_wei,
                "contract": WMON_ADDRESS,
            }
        }))
    }
}

// ─── Tool: send_mon ─────────────────────────────────────────────────────────

pub(crate) struct SendMon;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SendMonArgs {
    /// Amount of MON to send, in human units.
    pub(crate) amount: String,
    /// Recipient EVM address.
    pub(crate) to: String,
}

impl DynAomiTool for SendMon {
    type App = MonadOneShotApp;
    type Args = SendMonArgs;

    const NAME: &'static str = "send_mon";
    const DESCRIPTION: &'static str =
        "Build an unsigned transaction that sends native MON to another address. Returns \
         `{ to, data, value, chainId }`. No contract call — just a plain value transfer.";

    fn run(_app: &MonadOneShotApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let recipient = abi::parse_address(&args.to)?;
        let amount_wei = from_human(&args.amount, 18)?;
        let value_hex = wei_hex(&amount_wei)?;

        Ok(json!({
            "chainId": MONAD_CHAIN_ID,
            "to": abi::to_hex(&recipient),
            "data": "0x",
            "value": value_hex,
            "meta": {
                "action": "send_mon",
                "amount": args.amount,
                "amount_wei": amount_wei,
                "recipient": args.to,
            }
        }))
    }
}
