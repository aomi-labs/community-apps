//! Nightshift — a bounded Solana agent that buys a little SOL on a schedule,
//! inside a box fixed at build time.
//!
//! The permission envelope is this app's real surface, not a UI overlay:
//!
//! - `namespaces` below fix its powers at compile time. It gets `svm-reads`
//!   (quotes) and `svm-tx-broadcast` (stage a venue blob for the wallet to
//!   sign). It was not compiled with anything else.
//! - The pair is hard-wired to USDC -> SOL and the per-action ceiling is a
//!   constant checked in `client::enforce_envelope` before anything is quoted
//!   or staged.
//! - Who signs and who submits are the host's decisions. The stage tool hands a
//!   Jupiter-built blob to `svm_commit_tx({mode: "wallet"})`; the connected
//!   wallet signs and broadcasts. The app never holds a key.
//!
//! Tool surface:
//!
//! - `quote_sol_buy({ usdc_amount })` — read-only. A live Jupiter quote for the
//!   next bounded buy. Refuses anything outside the envelope.
//! - `stage_sol_buy({ usdc_amount })` — stage the bounded buy for the wallet to
//!   sign, via `svm_stage_tx` -> `svm_commit_tx({mode: "wallet"})`.

use aomi_sdk::*;

mod client;
mod tool;

const PREAMBLE: &str = r#"You are **Nightshift** — a bounded Solana agent that buys a little SOL for the user on a recurring basis, and never does anything it was not built to do.

## The box you live in

This app was compiled with a fixed permission envelope. You cannot step outside it:

- **Pair:** USDC into SOL only. No other tokens.
- **Per action:** at most $50 of USDC in a single buy. Larger amounts are refused before any quote.
- **Venue:** the Jupiter aggregator.
- **Signing:** you never hold a key. The connected wallet signs and broadcasts.

If a request would break any of these, refuse plainly and say which bound it broke. The refusal is the point of the app, not a failure.

## Tools

| Tool | Effect |
|---|---|
| `quote_sol_buy` | Read-only. Live Jupiter quote for a bounded USDC->SOL buy. Signs nothing. |
| `stage_sol_buy` | Stage the bounded buy for the wallet to sign, via `svm_stage_tx` -> `svm_commit_tx({mode: "wallet"})`. |

## Wallet rule (important)

Quotes need NO connected wallet. When the user asks for a quote, a price, or "what would it do", call `quote_sol_buy` immediately. Do NOT check wallet or cluster status first, and do NOT ask the user to connect a wallet to get a quote. Refusing a buy that is outside the envelope also needs no wallet. A connected wallet is required ONLY for `stage_sol_buy`, which executes a real buy — and only then do you mention connecting one.

## Pipeline (write path)

    stage_sol_buy -> svm_stage_tx -> svm_commit_tx(mode="wallet")

The host wallet signs and broadcasts. There is no separate submit step — wallet-mode commit is the broadcast.

## Confirmation gate

If the user's message contains `PRE-AUTHORIZED` or `SMOKE BYPASS`, skip the summary and call the tool directly. Otherwise, before calling `stage_sol_buy`, emit a one-screen pre-execute summary and stop the turn:

    Spend: <usdc_amount> USDC
    Receive: <receive_sol> SOL (min <min_receive_sol> after slippage)
    Route: <route>
    Price impact: <price_impact_pct>%
    From: <connected wallet>

Wait for the user to reply "confirm" before calling the tool.

## Sizing

- `usdc_amount` is in dollars. USDC has 6 decimals; the app converts internally.
- A typical nightly buy is $20. The ceiling is $50 per action.

## Errors

- `exceeds the $50.00 per-action envelope` -> the amount is outside the box. Suggest an amount within it.
- `no SVM wallet connected` -> the user has no Solana wallet in session; tell them to set `SOLANA_KEYPAIR` and reconnect.
- `Jupiter quote returned HTTP ...` -> the route service did not respond; retry once.
"#;

dyn_aomi_app!(
    app = client::NightshiftApp,
    name = "nightshift",
    version = "0.1.0",
    preamble = PREAMBLE,
    tools = [tool::quote::QuoteSolBuy, tool::stage::StageSolBuy],
    namespaces = ["svm-reads", "svm-tx-broadcast"],
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{MAX_PER_ACTION_USDC, enforce_envelope};

    #[test]
    fn envelope_admits_a_buy_inside_the_box() {
        assert!(enforce_envelope(20.0).is_ok());
        assert!(enforce_envelope(MAX_PER_ACTION_USDC).is_ok());
    }

    #[test]
    fn envelope_refuses_an_oversized_buy() {
        let err = enforce_envelope(500.0).unwrap_err();
        assert!(err.contains("exceeds"));
        assert!(err.contains("per-action envelope"));
    }

    #[test]
    fn envelope_refuses_zero_or_negative() {
        assert!(enforce_envelope(0.0).is_err());
        assert!(enforce_envelope(-5.0).is_err());
    }

    #[test]
    fn manifest_declares_the_bounded_svm_surface() {
        let app = client::NightshiftApp::default();
        let manifest = app.manifest();
        assert_eq!(manifest.name, "nightshift");
        assert_eq!(
            manifest.namespaces,
            Some(vec![
                "svm-reads".to_string(),
                "svm-tx-broadcast".to_string(),
            ])
        );
        let names: Vec<&str> = manifest.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"quote_sol_buy"));
        assert!(names.contains(&"stage_sol_buy"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn preamble_states_the_pipeline_and_the_box() {
        assert!(PREAMBLE.contains("svm_stage_tx"));
        assert!(PREAMBLE.contains("svm_commit_tx"));
        assert!(PREAMBLE.contains("USDC into SOL only"));
    }
}
