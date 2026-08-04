# Nightshift

A bounded, recurring Solana agent that trades while you sleep, and shows you the exact box it lives in before you let it run.

You set one rule in plain English, for example "every day, put $20 into SOL." Nightshift shows you the permission envelope it was built with, quotes the next action against a real venue, stages it with a simulate-before-sign preview, and lets you sign one yourself. It can quote and stage. It cannot color outside the box.

Built on the idea behind Aomi: on Solana, every transaction is routed by two decisions the agent never makes, who signs and who submits, and an app's powers are fixed at build time by namespace permissions. The agent proposes. The boundary decides. The boundary is set in code.

The point that separates this from a chat that trades is in `DIFFERENTIATION.md`: a chat needs you present to approve each transaction; Nightshift is the bound you read and grant once, so an agent can act on a schedule while you are not watching and still never leave the box. They guard the spend, we guard the build.

## Two parts

Nightshift is a real Aomi app, not a mock of one.

- `plugin/` — the Aomi SDK plugin (Rust `cdylib`). The permission envelope is its actual compile-time surface: namespaces `svm-reads` + `svm-tx-broadcast`, USDC into SOL only, a $50 per-action ceiling checked in code, Jupiter as the venue, and `svm_commit_tx({mode: "wallet"})` so the connected wallet signs. Two tools: `quote_sol_buy` (read-only) and `stage_sol_buy` (stage for the wallet to sign). It compiles against the Aomi SDK, its tests pass, and it runs live under `aomi-run`. See `plugin/README.md`.
- `src/` — the web panel. The readable front-end for that same envelope: it renders the box, quotes the next action live from Jupiter, and shows the boundary refusing anything outside it. It works standalone with zero env vars, and the namespaces it shows are the plugin's real manifest.

The panel is the thing you show; the plugin is the thing that is true. They describe one envelope.

## What is real, and what is simulated

Honesty is the whole point of this demo, so the line is drawn clearly.

**Real:**
- Every quote is a live Jupiter route (real AMMs, real price impact, real minimum received) fetched through a keyless API.
- Every token in the envelope is checked against Jupiter's verified token data. The allowlist is by mint, not by ticker, so a lookalike that borrows the SOL symbol is refused.
- The permission envelope is enforced in code. `evaluate()` is a deterministic function, not a model call. An action that exceeds the envelope is blocked before it ever reaches a quote.
- The "sign one now" path builds the real Jupiter swap transaction for your connected Phantom wallet and asks Phantom to sign it. Broadcasting is a separate, deliberate second click, because it is your money.

**Simulated, and labeled as such in the UI:**
- The scheduler. The demo does not run a background cron. It shows you the next action a schedule or price trigger would produce.
- The autonomous server-side signer. The panel describes how a delegated grant would sign inside the same envelope. It does not run one here. The live signing path in this demo is Phantom, in your hands.

No API keys anywhere. No custody. No private keys held by the app.

## The four surfaces

1. **The permission envelope** (hero). The tokens it may touch, the max per action, the max per day, the one venue, the max slippage, and the namespace permissions it was built with (`svm-reads`, `svm-tx-quote`, `svm-tx-stage`). It was not compiled with `svm-tx-broadcast`, so the model cannot broadcast on its own.
2. **The next planned action.** Your rule, parsed into a structured action, with a live Jupiter quote for what it would do right now.
3. **The staged log.** Every proposal kept, each with its simulate-before-sign preview, each stamped inside the envelope. Two seeded entries show the boundary refusing an oversized buy and a lookalike mint.
4. **Sign one for real.** Connect Phantom and sign the staged action, alongside a labeled explainer of how the autonomous path would sign the identical transaction within the same bounds.

## Run it

```bash
npm install
npm run dev
```

Open http://localhost:3111 (or the port Next prints). No environment variables required. Vercel ready as is.

```bash
npm run build   # production build
npm run start   # serve the production build
```

## How it works

- **Next.js App Router + TypeScript + Tailwind.** Dark, editorial, built on the Aomi design tokens (`aomi-labs/design`).
- **API routes** (`/api/quote`, `/api/token`, `/api/price`, `/api/swap`, `/api/broadcast`) proxy Jupiter and a public RPC server side, so the client holds no keys and hits no CORS walls.
- **`src/lib/envelope.ts`** is the boundary. `evaluate(action, envelope)` returns the verdict and, when refused, the exact clause that was broken.
- **`src/lib/rule.ts`** is a small deterministic parser. It reads a handful of plain English shapes and refuses the ones it does not understand rather than guessing.

## Optionally, the real Aomi agent

The panel stands alone. To drop the real Aomi agent widget alongside it:

```bash
npx shadcn add https://aomi.dev/r/aomi-frame.json
```

Read more at https://aomi.dev/docs/build.

## A note on funds

The default sign path stops at a signature and does not move money. Broadcasting is a second explicit click that sends a real mainnet transaction, and Nightshift labels it as spending real funds. Treat it accordingly.
