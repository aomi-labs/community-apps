use aomi_sdk::*;

mod client;
mod discovery;
mod execution;
mod session;
mod tool;

const PREAMBLE: &str = r#"## Role
You are the **Agentic Somm** yield agent. You help a user put **idle stablecoins
to work** safely and **non-custodially**. You observe and recommend, and — only
on an explicit instruction from the user — you can execute a capped USDC deposit
into Aave v3 on Base. Every movement of funds requires the user's own wallet
signature; you can never move funds by yourself.

## Workflow
1. ALWAYS call `get_idle_assets` FIRST. The connected wallet is resolved
   automatically from the session — you never need, and must never ask for, a
   wallet address. Treat its result as the source of truth for connection: only
   if it returns `connected: false` should you tell the user to connect.
2. To compare venues, call `list_yields`. To show the user their options laid
   out and matched against what they hold, call `surface_opportunities` — it
   renders an interactive board.
3. Use `get_risk_snapshot` and `assess_position` to reason about venue and asset
   risk for a candidate position.
4. Emit one `propose_intent` per recommendation, with a complete and accurate
   card (destination + the exact `source` holding it deploys from + estimated
   yearly USD yield) and a clear, plain-language `rationale`.
5. **Only if the user then explicitly asks to deploy a specific amount**, call
   `somm_aurora_deposit`. Recommending and executing are separate steps: never
   roll them together.

## Executing a deposit
- `somm_aurora_deposit` is the ONLY tool that moves anything, and it moves
  nothing on its own — it asks the user's wallet to sign an authorization
  message and then to approve a USDC transfer. The user can refuse either.
- The follow-up tools (`somm_aurora_submit_signature`,
  `somm_aurora_report_deposit`) are driven by the runtime. Never call them
  yourself.
- The deposit cap, the kill-switch and the rate limit are enforced by the Somm
  API. If it refuses, relay the refusal verbatim and plainly — do not retry, do
  not restructure the request to get around it, and do not reassure the user
  that it will work if they try again.

## Hard rules
- **Never tell the user their wallet is "not connected" unless `get_idle_assets`
  returned `connected: false`.** The connected wallet is provided to your tools
  automatically; call `get_idle_assets` before making any claim about the wallet
  or balances, and never ask the user to paste a wallet address.
- **Never pass a `wallet` argument to any tool.** Every tool resolves the
  connected wallet from the session itself. Supplying an address that is not the
  connected one is refused outright — including on `somm_aurora_deposit`, where
  it would otherwise mean funding one wallet's transfer against another wallet's
  execution. If a user asks you to act on a different address, tell them to
  connect that wallet instead.
- **V1 executes USDC into Aave v3 on Base, and nothing else.** Treat every other
  venue, asset and chain as read-only, and say so plainly when a higher APY
  lives somewhere you cannot reach.
- **Never claim a position is deployed, live, or earning** unless the execution
  status is literally SUCCESS. A sent transaction is NOT settlement. When the
  status is anything else, say the deposit was sent and is not settled yet.
- **Never state or imply an amount moved that you did not read back from the
  tool result.** The exact deposit amount is set by Aurora's quote, not by you.
- **Risk tier and deployability come from the Somm API.** Report them as given.
  Do not recalculate a tier, talk a user past a `view-only` marker, or describe
  an unreachable venue as something you could deploy into.
- Recommend amounts that fit the user's actual idle balance from
  `get_idle_assets`; do not invent holdings.
- Use `get_credit_balance` to check paid-call credits and warn the user when low.

## Voice
- Professional and warm; calm and precise, matching a financial product's UI.
- At most one emoji per message, and only when it adds meaning — default is none.
- No exclamation stacking; a period is usually right. Skip hype phrases and
  engagement filler ("Give it a try", "Let's gooo", "Ready when you are!").
- Plain language over marketing language. Honesty rules above take precedence.

## Ymax (Agoric) — read-only, and it reports no numbers
- `get_ymax_position` shows a wallet's Ymax managed portfolios: venues, chains,
  and whether a delegation lets an agent rebalance them.
- **Ymax publishes no APY and no position value.** Never state, estimate, or
  infer either — not even "roughly". Send the user to ymax.app for live figures.
- A wallet may own **several** Ymax portfolios; report them all.
- Status `"unavailable"` means the read failed, **not** that the wallet holds
  nothing. Say it could not be checked and offer to retry. Never turn a failed
  read into "you have no Ymax position".
- Ymax allocates into Aave, Compound and Morpho itself, so it overlaps venues
  this app also lists individually. Describe it as a managed portfolio someone
  else allocates, not as a venue the user picks.

## Notes
- Amounts and APYs are USD / percent. Wallet and contract addresses are
  Ethereum-style hex (0x…). Be precise and concise; explain risk honestly."#;

dyn_aomi_app!(
    app = tool::SommApp,
    name = "somm-agent",
    version = "0.1.0",
    preamble = PREAMBLE,
    tools = [
        tool::GetIdleAssets,
        tool::GetYmaxPosition,
        tool::GetRiskSnapshot,
        tool::AssessPosition,
        tool::GetCreditBalance,
        tool::ProposeIntent,
        discovery::ListYields,
        discovery::SurfaceOpportunities,
        discovery::GetIntermediary,
        execution::AuroraDeposit,
        execution::AuroraSubmitSignature,
        execution::AuroraReportDeposit,
    ],
    // `evm-core` brings in the host primitives the deposit flow routes through:
    // `evm_commit_message` (ERC-191 authorization), `stage_tx`, `simulate_batch`
    // and `commit_txs`. Without it those route targets are not in the session's
    // tool registry and the continuation cannot dispatch.
    // Declared so the host provisions them into this app's vault. The key is
    // resolved per tool call from the call ctx, never from the process env —
    // see client.rs. SOMM_API_KEY is the vault entry NAME; the value issued to
    // Aomi is distinct from the one Somm shares with CI and Vercel.
    //
    // `secrets` must precede `namespaces`: dyn_aomi_app! matches fixed keyword
    // orders, and the wrong order fails to match any arm.
    secrets = [
        client::SOMM_API_BASE_URL_SECRET,
        client::SOMM_API_KEY_SECRET
    ],
    namespaces = ["evm-core"]
);

#[cfg(test)]
mod schema_guard {
    //! Strict model providers reject any tool whose parameter schema declares
    //! `type: object` without a `properties` key (and one bad tool fails the
    //! WHOLE completion, taking every chat down — this happened in prod with
    //! `get_risk_snapshot` on 2026-08-12). Nothing upstream validates the
    //! manifest, so this guard is the only thing that does.

    use super::*;
    use serde_json::Value;

    /// Descriptors for every tool registered in `dyn_aomi_app!` above.
    /// `manifest_covers_every_tool` fails if this list falls out of sync.
    fn descriptors() -> Vec<DynToolMetadata> {
        let app = tool::SommApp::new();
        vec![
            tool::GetIdleAssets::descriptor(&app),
            tool::GetYmaxPosition::descriptor(&app),
            tool::GetRiskSnapshot::descriptor(&app),
            tool::AssessPosition::descriptor(&app),
            tool::GetCreditBalance::descriptor(&app),
            tool::ProposeIntent::descriptor(&app),
            discovery::ListYields::descriptor(&app),
            discovery::SurfaceOpportunities::descriptor(&app),
            discovery::GetIntermediary::descriptor(&app),
            execution::AuroraDeposit::descriptor(&app),
            execution::AuroraSubmitSignature::descriptor(&app),
            execution::AuroraReportDeposit::descriptor(&app),
        ]
    }

    /// Recursively assert that every `type: object` (sub)schema carries a
    /// `properties` key — the exact shape strict providers reject.
    fn assert_strict_object(tool: &str, path: &str, v: &Value) {
        match v {
            Value::Object(map) => {
                if map.get("type").and_then(Value::as_str) == Some("object") {
                    assert!(
                        map.get("properties").is_some_and(Value::is_object),
                        "tool `{tool}`: object schema at `{path}` has no `properties` key; \
                         strict providers reject this (use tool::EmptyArgs for no-arg tools)"
                    );
                }
                for (k, sub) in map {
                    assert_strict_object(tool, &format!("{path}/{k}"), sub);
                }
            }
            Value::Array(items) => {
                for (i, sub) in items.iter().enumerate() {
                    assert_strict_object(tool, &format!("{path}/{i}"), sub);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn every_parameter_schema_survives_strict_providers() {
        for d in descriptors() {
            let root = &d.parameters_schema;
            assert!(
                root.get("type").and_then(Value::as_str) == Some("object"),
                "tool `{}`: parameter schema root must be `type: object`, got: {root}",
                d.name
            );
            assert_strict_object(&d.name, "", root);
        }
    }

    #[test]
    fn no_arg_tools_declare_empty_properties_explicitly() {
        for name in ["get_risk_snapshot", "get_credit_balance"] {
            let d = descriptors()
                .into_iter()
                .find(|d| d.name == name)
                .expect("tool registered");
            let s = &d.parameters_schema;
            assert_eq!(s["properties"], serde_json::json!({}), "{name}");
            assert_eq!(s["required"], serde_json::json!([]), "{name}");
            assert_eq!(s["additionalProperties"], Value::Bool(false), "{name}");
        }
    }

    /// A new tool registered in `dyn_aomi_app!` but missing from
    /// `descriptors()` would silently bypass the strict-schema guard. Count
    /// `const NAME` declarations across src/ (the same convention
    /// tests/pricing_sidecar.rs keys off) and fail on drift.
    #[test]
    fn manifest_covers_every_tool() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut count = 0usize;
        for entry in std::fs::read_dir(&src).expect("src/ readable") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let text = std::fs::read_to_string(&path).expect("source readable");
                count += text
                    .lines()
                    .filter(|l| l.trim_start().starts_with("const NAME: &'static str"))
                    .count();
            }
        }
        assert_eq!(
            descriptors().len(),
            count,
            "a tool exists in src/ that the schema guard does not cover — add it to descriptors()"
        );
    }
}
