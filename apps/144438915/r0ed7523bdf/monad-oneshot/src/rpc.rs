//! Tiny JSON-RPC client for Monad mainnet. Two methods, blocking.

use serde_json::{json, Value};

use crate::chain::MONAD_RPC;

fn rpc(method: &str, params: Value) -> Result<Value, String> {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let resp = reqwest::blocking::Client::new()
        .post(MONAD_RPC)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("[monad rpc] request failed: {e}"))?
        .json::<Value>()
        .map_err(|e| format!("[monad rpc] parse failed: {e}"))?;
    if let Some(err) = resp.get("error") {
        return Err(format!("[monad rpc] {err}"));
    }
    resp.get("result")
        .cloned()
        .ok_or_else(|| "[monad rpc] no result".into())
}

pub fn eth_call(to: &str, data_hex: &str) -> Result<String, String> {
    rpc("eth_call", json!([{ "to": to, "data": data_hex }, "latest"]))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "[monad rpc] eth_call returned non-string".into())
}

pub fn eth_get_balance(address: &str) -> Result<String, String> {
    rpc("eth_getBalance", json!([address, "latest"]))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "[monad rpc] eth_getBalance returned non-string".into())
}
