//! Tool layer for Nightshift.
//!
//! Two intent-shaped tools: a read-only quote and a bounded stage-and-sign.
//! Both run the same in-code envelope check first, so the boundary is enforced
//! identically whether the agent is only looking or about to act.

pub(crate) mod quote;
pub(crate) mod stage;

use aomi_sdk::*;
use serde_json::{Value, json};

/// Resolve the connected SVM wallet address from the host session. The wallet
/// is the fee payer and the signer; the app never holds a key.
pub(crate) fn require_svm_wallet(ctx: &DynToolCallCtx) -> Result<String, String> {
    ctx.attribute_string(&["domain", "svm", "address"]).ok_or_else(|| {
        "[nightshift] no SVM wallet connected — set SOLANA_KEYPAIR (or run \
         `aomi secret add SOLANA_KEYPAIR=…`) and re-open the session"
            .to_string()
    })
}

/// Reduce a raw Jupiter quote into the fields the model and the ledger care
/// about: what you receive, the route, the price impact, and the floor.
pub(crate) fn summarize_quote(q: &Value) -> Value {
    let out_lamports = q.get("outAmount").and_then(Value::as_str).unwrap_or("0");
    let min_lamports = q
        .get("otherAmountThreshold")
        .and_then(Value::as_str)
        .unwrap_or("0");
    let out_sol = lamports_to_sol(out_lamports);
    let min_sol = lamports_to_sol(min_lamports);
    let impact = q
        .get("priceImpactPct")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let usd_value = q.get("swapUsdValue").and_then(Value::as_str);

    let route = q
        .get("routePlan")
        .and_then(Value::as_array)
        .map(|legs| {
            let mut labels: Vec<String> = legs
                .iter()
                .filter_map(|l| {
                    l.get("swapInfo")
                        .and_then(|s| s.get("label"))
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                })
                .collect();
            labels.dedup();
            labels
        })
        .unwrap_or_default();

    json!({
        "receive_sol": format!("{out_sol:.6}"),
        "min_receive_sol": format!("{min_sol:.6}"),
        "price_impact_pct": format!("{:.4}", impact * 100.0),
        "route": route,
        "quoted_usd_value": usd_value,
    })
}

fn lamports_to_sol(lamports: &str) -> f64 {
    lamports.parse::<f64>().unwrap_or(0.0) / 1_000_000_000.0
}
