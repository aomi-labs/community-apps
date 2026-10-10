# somm-agent — Agentic Somm aomi app

A thin Rust `cdylib` aomi app (`dyn_aomi_app!`, `aomi-sdk = "=3.1.0"`),
bound to `somm.finance` by `.aomi/config.json`, that exposes the Agentic Somm
yield agent's tools to aomi's runtime. Each tool forwards to the **deployed Somm HTTP API** — it
re-implements nothing. It is the one-to-one Rust mirror of the TypeScript MCP
proxy in [`../mcp`](../mcp) (`mcp/src/{client,contract,handlers}.ts`), which
aomi's runtime cannot load directly (the runtime loads only compiled Rust
plugins, not MCP servers — see
[`../docs/superpowers/specs/2026-06-25-aomi-upstream-contracts-verified.md`](../docs/superpowers/specs/2026-06-25-aomi-upstream-contracts-verified.md)).

## Tools

| Tool | Route on the Somm API | Notes |
|---|---|---|
| `get_idle_assets` | `GET /api/portfolio?wallet=…` | idle holdings for a wallet |
| `get_risk_snapshot` | `GET /api/risk/snapshot` | venue/asset risk context |
| `assess_position` | `POST /api/risk/assess` | risk of a candidate position |
| `get_credit_balance` | `GET /api/account` | paid-call credit + low warning |
| `propose_intent` | _(no HTTP)_ | args **are** the recommendation card the frontend tool-UI renders; mirrors `parseRecommendationCard` (`src/lib/recommendation.ts`) |

The agent **observes and recommends only** — it never signs, submits, or
executes. The user approves every deployment through the app's preview flow. See
`PREAMBLE` in `src/lib.rs`.

## Runtime env

| Var | Purpose | Default |
|---|---|---|
| `SOMM_API_BASE_URL` | base URL of the deployed Somm API | `https://agentic.somm.finance` |
| `SOMM_API_KEY` | `Authorization: Bearer` for the API | _(required for live data)_ |

These must be provisioned to the app at deploy/activation time.

## Build / validate

```bash
cargo build --lib            # compiles the cdylib (libsomm_agent.dylib/.so)
nm -gU target/debug/libsomm_agent.dylib | grep aomi_   # plugin ABI + SDK stamp
```

For the full plugin-manifest validation aomi's CI runs, drop this crate into a
`somm-finance-apps` workspace and `cargo run -p xtask -- build-aomi --app somm-agent`.

## Deploy

This crate is authored here for review and targets the `somm.finance` platform. To go live it is pushed through aomi's
GitHub-App onboarding/deploy pipeline (its own repo → `somm-finance-apps` CI builds
the release tag `apps-<installation>-<repo-key>-somm-agent-<commit>` → activate),
then chat is addressed at the activated bot's session. See the verified-contracts
spec linked above.
