# aomi-monad-oneshot

A sample Aomi app that turns one English sentence into one signed action on **Monad mainnet** (chain 143). Type *"wrap 0.5 MON"* or *"send 2 MON to 0x…"*. The agent reads your balance, builds the unsigned transaction, and the Aomi runtime simulates it on a forked chain before your wallet signs. You sign once.

```
┌────────────────────────────────────────┬──────────────────────────────┐
│  One-Shot pipeline                     │  aomi × Monad One-Shot       │
│                                        │                              │
│  1 · Read balance                      │  > wrap 0.5 MON              │
│      get_balance                       │                              │
│  2 · Build the action                  │  ⚡ get_balance               │
│      wrap_mon / unwrap_wmon / send_mon │     0.5 MON available        │
│  3 · Simulate on a fork                │  ⚡ wrap_mon                  │
│      (runtime owns this)               │     to WMON, value 0.5 MON   │
│  4 · Sign once                         │     simulated, sign to send  │
└────────────────────────────────────────┴──────────────────────────────┘
```

## Why this exists

Most agent demos show a chat window calling an LLM. This one shows the part that is actually hard: turning *intent* into an *unsigned EVM transaction*, simulating it before the user pays gas, and routing the signature through a wallet that already knows about the chain.

The backend is a single Rust plugin. The frontend is a single Next.js page. Everything else is provided by Aomi: the runtime that loads the plugin, the chat protocol, the simulation pipeline, and the wallet handshake.

## What's in the box

```
aomi-monad-oneshot/
├── app/                              ← Rust Aomi plugin (the backend)
│   ├── Cargo.toml
│   ├── aomi.toml                     ← deploy manifest (community platform)
│   └── src/
│       ├── lib.rs                    ← dyn_aomi_app! manifest + preamble
│       ├── chain.rs                  ← Monad mainnet chain id + WMON address
│       ├── abi.rs                    ← minimal ABI encoder (no crypto deps)
│       ├── rpc.rs                    ← JSON-RPC client
│       └── tool.rs                   ← the 4 tools
└── ui/                               ← Next.js 15 app
    ├── app/
    │   ├── layout.tsx
    │   ├── page.tsx                  ← split layout: pipeline + chat
    │   ├── providers.tsx             ← wagmi + react-query
    │   └── globals.css
    ├── components/
    │   ├── chat.tsx                  ← chat panel + sign flow
    │   ├── header.tsx
    │   ├── StarterChips.tsx
    │   ├── banners.tsx
    │   └── AomiMark.tsx
    ├── lib/
    │   ├── aomi-chat.ts              ← wrapper around the runtime /chat endpoint
    │   ├── wagmi.ts                  ← wagmi config (injected connector)
    │   └── chains.ts                 ← Monad mainnet defineChain
    └── package.json
```

The plugin exposes four tools. One reads, three build unsigned transactions:

| Tool           | What it does                                                                 |
| -------------- | --------------------------------------------------------------------------- |
| `get_balance`  | Reads MON (native) or WMON (wrapped) balance via `eth_getBalance`/`eth_call`. |
| `wrap_mon`     | Builds a tx that calls `deposit()` on WMON. Returns `{ to, data, value, chainId }`. |
| `unwrap_wmon`  | Builds a tx that calls `withdraw(uint256)` on WMON.                          |
| `send_mon`     | Builds a plain native MON transfer.                                          |

The agent loop is described in the preamble at [app/src/lib.rs](app/src/lib.rs).

## Heads up: this is mainnet

The app targets **Monad mainnet (chain 143)**. Transactions move **real MON**. The preamble tells the agent to confirm amounts and recipients before handing off, but you are signing real value. The chain constants live in [app/src/chain.rs](app/src/chain.rs):

- `MONAD_CHAIN_ID = 143`
- `MONAD_RPC = https://rpc.monad.xyz`
- `WMON_ADDRESS = 0x3bd359C1119dA7Da1D913D1C4D2B7c461115433A` (verified on-chain; source of truth is https://docs.monad.xyz/developer-essentials/network-information)

Re-verify the WMON address against the Monad docs before pointing anything at real funds.

## Build the plugin

```bash
cd app
cargo build --release
```

That produces `target/release/libmonad_oneshot.dylib` on macOS or `.so` on Linux. It builds against `aomi-sdk = "=3.0.0"`, the version the Aomi community platform runs. Load the artifact into any Aomi runtime.

To drive it live in the terminal with no backend, use `aomi-run` (the runner SDK version must match the pin above):

```bash
aomi-run --plugin target/release/libmonad_oneshot.dylib
```

## Run the UI

```bash
cd ui
cp .env.example .env.local
pnpm install
pnpm dev
```

Open http://localhost:3000. Connect an injected wallet (MetaMask or similar) on Monad. Ask the agent to wrap, unwrap, or send.

The environment variables:

| Variable                        | Purpose                                                    |
| ------------------------------- | ---------------------------------------------------------- |
| `NEXT_PUBLIC_AOMI_BACKEND_URL`  | Runtime endpoint that hosts the plugin. Defaults to localhost:8080. |
| `NEXT_PUBLIC_AOMI_APP_ID`       | Must match the `name` in [app/aomi.toml](app/aomi.toml). Default `monad-oneshot`. |

## Try these

| Ask                                | What happens                                                              |
| ---------------------------------- | ------------------------------------------------------------------------ |
| "What's my MON balance?"           | Calls `get_balance` with the connected wallet. Step 1 pulses.            |
| "Wrap 0.5 MON."                    | Reads balance, then builds a `deposit()` tx. Runtime simulates, you sign. |
| "Unwrap 0.2 WMON."                 | Builds a `withdraw(uint256)` tx on WMON.                                  |
| "Send 1 MON to 0x…"                | Builds a native transfer. The agent confirms the recipient first.        |

## Design decisions

**One plugin, four tools.** The "one-shot" name refers to the user side: one message, one action. The agent still calls a small chain of tools because that is how the simulation pipeline gets useful inputs.

**No crypto dependency in the plugin.** The three function selectors (`balanceOf`, `deposit`, `withdraw`) are hardcoded. ABI encoding for `uint256` and `address` is written by hand in [app/src/abi.rs](app/src/abi.rs). That keeps the plugin readable and the dep tree small. A production app should use `alloy-sol-types`.

**Build tools are pure.** No tool signs or broadcasts. `wrap_mon`, `unwrap_wmon`, and `send_mon` return an unsigned transaction. The Aomi runtime owns simulation and signing. The plugin owns intent to calldata. Keeping that boundary clean is what lets the same plugin run under different wallets and different simulation backends.

**Wrap and send, not swap.** An earlier draft targeted a DEX router. On Monad that path depends on a specific DEX deployment, which is fragile. Wrapping and native sends only depend on WMON, which is chain infrastructure. That makes the demo reliable.

**Next.js + wagmi.** The UI connects an injected wallet through wagmi and talks to the Aomi runtime over its `/chat` endpoint ([ui/lib/aomi-chat.ts](ui/lib/aomi-chat.ts)). The left panel pulses the step that matches the current tool call, so chat and UI move together because they share the plugin.

## What this is not

- Not a DEX. It wraps, unwraps, and sends. No routing, no swaps.
- Not a wallet. The user's injected wallet owns the keys. Aomi never sees them.
- Not custodial. The plugin returns an unsigned transaction; signing happens in the user's wallet.

## License

MIT.
