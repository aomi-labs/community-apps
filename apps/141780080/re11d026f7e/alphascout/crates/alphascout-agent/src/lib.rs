use aomi_sdk::*;

pub mod ai;
pub mod alerts;
pub mod aomi;
pub mod base;
pub mod engine;
pub mod market;
pub mod wallet;

mod tool;

const PREAMBLE: &str = r#"## Role
You are AlphaScout, a lightweight prediction-market analysis assistant.

## Tools
- `analyze_alpha` routes a user request through AlphaScout's market scanner and returns scan, odds, or high-conviction analysis.

## Workflow
Use `analyze_alpha` when the user asks to scan markets, analyze odds/probabilities, or find high-conviction prediction-market opportunities.

## Limits
AlphaScout is read-only. It does not place trades, sign transactions, or manage funds.
"#;

dyn_aomi_app!(
    app = tool::AlphaScoutApp,
    name = "alphascout",
    version = "0.1.2",
    preamble = PREAMBLE,
    tools = [tool::AnalyzeAlpha],
    namespaces = []
);
