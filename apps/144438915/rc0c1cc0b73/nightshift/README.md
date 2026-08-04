# Nightshift

A bounded Solana agent that buys a little SOL on a schedule, inside a box you can read before you let it run.

Nightshift is a real Aomi app. On Solana, every transaction is routed by two decisions the agent never makes, who signs and who submits, and an app's powers are fixed at build time by namespace permissions. Nightshift makes that box legible: you read exactly what it may do, then grant it.

The point that separates this from a chat that trades is in `DIFFERENTIATION.md`: a chat needs you present to approve each transaction; Nightshift is the bound you read and grant once, so an agent can act on a schedule while you are not watching and still never leave the box. They guard the spend, we guard the build.

## Layout

Canonical Aomi app layout: the plugin crate at the root, the web dashboard in `ui/`.

- `Cargo.toml`, `aomi.toml`, `src/` — the Aomi SDK plugin (Rust `cdylib`). The permission envelope is its real compile-time surface: namespaces `svm-reads` + `svm-tx-broadcast`, USDC into SOL only, a $50 per-action ceiling checked in code, Jupiter as the venue, and `svm_commit_tx({mode: "wallet"})` so the connected wallet signs. Two tools: `quote_sol_buy` (read-only) and `stage_sol_buy` (stage for the wallet to sign). See `PLUGIN.md`.
- `ui/` — the web dashboard: the readable front-end for that same envelope. Live Jupiter quotes, the box drawn as a spec, the boundary refusing anything outside it. Works standalone with zero env vars. See `ui/README.md`.

## Build and test the plugin

```bash
cargo build --release      # target/release/libnightshift.dylib (.so on Linux)
cargo test                 # envelope + manifest tests
```

## Run the plugin live (terminal, no backend)

```bash
ANTHROPIC_API_KEY=sk-... aomi-run target/release/libnightshift.dylib
```

Try `quote a $20 SOL buy`, then `put $500 into SOL` and watch it refused with the exact bound it broke.

## Run the dashboard

```bash
cd ui && npm install && npm run dev
```

## Deploy (community platform)

```bash
aomi-build deploy run --repo victorchimakanu/nightshift --fix-sdk
aomi-build deploy activate
aomi-build deploy status
```

Then attach the deployed app to a Telegram bot from build.aomi.dev/integrations.
