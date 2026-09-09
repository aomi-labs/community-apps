# my-aomi-bots

A Hyperliquid perpetuals trading bot, packaged as an Aomi app for the
[`community`](https://github.com/aomi-labs/community-apps) platform.

## Tools exposed to the agent

| Tool | Auth | What it does |
|---|---|---|
| `get_meta` | none | List every Hyperliquid perp with size/price decimals & max leverage |
| `get_mid_price` | none | Current mid-price for a coin (e.g. `BTC`, `ETH`) |
| `get_user_state` | none | Read positions, margin, withdrawable for any wallet address |
| `get_open_orders` | none | List a wallet's resting orders (with oids for cancel) |
| `place_market_order` | `HL_WALLET_KEY` | IoC limit order at slipped mid (default 0.5%) |
| `cancel_order` | `HL_WALLET_KEY` | Cancel a resting order by oid |

Trading tools fail fast with a clear error when `HL_WALLET_KEY` is not
configured on the backend. **The signing path itself is a TODO** — see the
comments in `src/client.rs::Trader::submit_action` for the protocol reference
before wiring real trades.

## Layout

```
my-aomi-bots/
├── .aomi/
│   └── config.json  # V2 project configuration — platform + app manifests
├── aomi.toml         # application manifest — slug, visibility, server tags
├── Cargo.toml        # cdylib + aomi-sdk pinned to the backend-required version
├── src/
│   ├── lib.rs        # dyn_aomi_app! registration + preamble
│   ├── client.rs     # HTTP client, Trader scaffold, arg structs, action builders
│   └── tools.rs      # one impl DynAomiTool per tool
└── .gitignore        # /target, local deployment state, Cargo.lock
```

## Publishing

Authoring lives here; project creation and deployment go through `aomi-build`.

```bash
# 1. Compile check
cargo check

# 2. Create or validate the platform-bound V2 Project
aomi-build project create \
  --repo CeciliaZ030/my-aomi-bots \
  --platform community

# 3. Commit and push .aomi/config.json before deploying
git add .aomi/config.json Cargo.toml aomi.toml
git commit -m "Configure Aomi project"
git push

# 4. Deploy, activate, and verify the app
aomi-build deploy
aomi-build deploy status
```

The backend relays the pushed source revision to the community platform,
waits for its release, activates it, and verifies that the runtime loaded it.

See [`community-apps/CONTRIBUTING.md`](https://github.com/aomi-labs/community-apps/blob/main/CONTRIBUTING.md)
for the full pipeline walkthrough.

## TODOs

- [ ] Implement EIP-712 signing in `Trader::submit_action` (see protocol
      reference in the comment block). Reference impl:
      <https://github.com/hyperliquid-dex/hyperliquid-python-sdk>.
- [ ] Add a `set_leverage` tool once signing is wired up.
- [ ] Consider a `place_limit_order` variant with explicit `tif` (Gtc/Ioc/Alo).
