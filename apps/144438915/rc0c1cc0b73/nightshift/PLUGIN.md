# Nightshift plugin

The real Aomi app behind Nightshift: a Rust SDK plugin (`cdylib`) whose permission envelope is its actual, compile-time surface. The agent proposes; the boundary is fixed in code; the connected wallet signs.

This is not a mock. It compiles against the Aomi SDK, registers two tools, and drives the host's SVM stage/commit pipeline. The web panel in the parent folder is the readable front-end for this same envelope.

## The envelope, in code

- **Namespaces** (fixed at build time, in `src/lib.rs`): `svm-reads`, `svm-tx-broadcast`. Nothing else. It reads routes and stages a swap for the wallet to sign. It holds no key.
- **Pair**: hard-wired USDC into SOL.
- **Per action**: `MAX_PER_ACTION_USDC = 50`, checked in `client::enforce_envelope` before anything is quoted or staged. A larger amount is refused, not clamped.
- **Venue**: the Jupiter aggregator.
- **Who signs / submits**: the host wallet. `stage_sol_buy` ends in `svm_commit_tx({mode: "wallet"})`, so the wallet signs and broadcasts. The app never signs.

## Tools

| Tool | Kind | Effect |
|---|---|---|
| `quote_sol_buy({ usdc_amount })` | read-only | Live Jupiter quote for a bounded USDC to SOL buy. Signs nothing. |
| `stage_sol_buy({ usdc_amount })` | execution | Checks the envelope, has Jupiter build the swap for the wallet, emits `svm_stage_tx` to `svm_commit_tx({mode: "wallet"})`. |

The write path:

```
stage_sol_buy -> svm_stage_tx -> svm_commit_tx(mode="wallet")
```

Jupiter is the producer-of-record for the transaction blob (the venue-blob pattern). The host's `svm_stage_tx` decodes it, validates the payer, and mints a `pending_tx_id`; `svm_commit_tx` routes the stored blob to the wallet for signing. Simulate-before-sign is the host pipeline, not something the app fakes.

## Build

```bash
cd plugin
cargo build --release      # produces target/release/libnightshift.dylib (.so on Linux)
cargo test                 # envelope + manifest tests
```

The `aomi-sdk` dependency is a path reference to the local SDK checkout at `../../aomi-sdk/sdk`. The host-plugin compatibility gate is an exact-match SDK version, so build against the SDK the runtime is on.

## Run it live (terminal, no backend)

`aomi-run` loads the plugin and chats to it against an LLM. It needs an API key.

```bash
# with a key in the environment
ANTHROPIC_API_KEY=sk-... aomi-run target/release/libnightshift.dylib

# or from a dotenv file
aomi-run --env-file .env target/release/libnightshift.dylib
```

Then try:

- `quote a $20 SOL buy` — a live route, no signing.
- `put $500 into SOL` — refused, with the exact bound it broke.
- `stage a $20 SOL buy` (needs a connected SVM wallet, `SOLANA_KEYPAIR`) — stages the swap and routes it to the wallet to sign.

## Deploy it (so the real Aomi agent drives it)

The plugin is a standard Aomi app. Compile and deploy it through the platform with `aomi-build compile` / `aomi-build deploy`, then point a frontend (the panel here, or `AomiFrame`) at the deployed backend. The envelope you read in the panel is the envelope the deployed app enforces.
