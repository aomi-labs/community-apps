# zora-aomi-tools

Rust backend MVP that wraps Zora Coins market data as typed agentic tools.

## Tools

- `get_holder_count`: returns unique holder count for a Zora coin.
- `get_24h_volume`: returns rolling 24 hour volume and market cap delta.
- `get_top_buyers`: returns top buy-side traders ranked by aggregate coin amount.
- `message_recent_buyer`: finds a recent buyer and submits an on-chain comment through a wallet runtime adapter.

## Configuration

Copy `.env.example` and set:

```env
ZORA_API_KEY=your_zora_api_key_here
DEFAULT_CHAIN_ID=8453
```

The default Zora API base URL is `https://api-sdk.zora.engineering`. For tests or local adapters, pass a custom base URL with `ZoraClient::with_base_url`.

## Usage

```rust
use zora_aomi_tools::{ZoraClient, types::CoinInput, tools};

# async fn example() -> anyhow::Result<()> {
let client = ZoraClient::from_env()?;
let output = tools::get_holder_count(
    CoinInput {
        address: "0xcoin".to_string(),
        chain: 8453,
    },
    &client,
).await?;
println!("{output:?}");
# Ok(())
# }
```

`message_recent_buyer` is intentionally runtime-agnostic. Implement `WalletRuntime` for the Aomi wallet execution layer, then pass it to the tool.

## Aomi Plugin Build

The crate includes optional `aomi-sdk` plugin wiring behind the `aomi-plugin` feature:

```sh
cargo build --features aomi-plugin
```

The plugin exports the three read tools through `DynAomiTool`. The write tool is registered by name, but returns an explicit adapter error until the host wires Aomi wallet execution into `WalletRuntime`.

## Frontend API

Start the backend:

```sh
cargo run --bin agent
```

The server runs on `http://localhost:3001` by default.

Deployed backend URL:

```text
https://zora-wrapped-back.onrender.com
```

Use the deployed URL for production/frontend requests, and `http://localhost:3001` when running the backend locally.

### NLP Agent

Natural language queries are accepted at:

```text
POST https://zora-wrapped-back.onrender.com/query
POST http://localhost:3001/query
```

The endpoint accepts any plain-English query containing a wallet or token address and returns a formatted response. Supported intents are auto-detected from the query text.

**Request:**

```json
{ "query": "How many holders are in 0xF5735B760e2194521377A24a5c1e830aFa83aCDB" }
```

**Response:**

```json
{ "result": "📊 MYTOKEN has 1204 unique holders." }
```

**Error response:**

```json
{ "error": "Could not understand query. Try: 'How many holders in 0x...' or 'Show top 5 buyers of 0x...'" }
```

**Supported query patterns:**

| Example query | Intent |
|---|---|
| `"How many holders in 0x..."` | Holder count |
| `"What's the 24h volume for 0x..."` | 24h volume |
| `"Show top 5 buyers of 0x..."` | Top buyers |
| `"Message the buyer of 0x... with \"text\""` | Message buyer |
| `"0x..."` (address only) | Token summary |

Wallet addresses (`0x` + 40 hex chars) and ENS names (e.g. `vitalik.eth`) are both accepted anywhere in the query string.

### GraphQL

```text
POST https://zora-wrapped-back.onrender.com/graphql
POST http://localhost:3001/graphql
```

Supported query fields:

```graphql
query CreatorDashboard($wallet: String!) {
  creatorStats(wallet: $wallet) {
    wallet
    name
    avatar
    totalMints
    volumeEth
    uniqueHolders
    growth30d
  }
  volumeData(wallet: $wallet) {
    date
    volume
  }
  topBuyers(wallet: $wallet, topN: 5) {
    rank
    wallet
    percentage
    amountEth
  }
  collectors(wallet: $wallet, count: 10) {
    rank
    wallet
    coinsHeld
    firstPurchase
    totalSpentEth
    badge
  }
  collections(wallet: $wallet, count: 20) {
    id
    name
    priceEth
    volumeEth
    holders
    thumbnail
  }
}
```

### REST

The server also exposes REST compatibility routes for the current frontend hooks:

```text
GET https://zora-wrapped-back.onrender.com/api/creator/:wallet
GET https://zora-wrapped-back.onrender.com/api/creator/:wallet/volume
GET https://zora-wrapped-back.onrender.com/api/creator/:wallet/top-buyers?top_n=5
GET https://zora-wrapped-back.onrender.com/api/creator/:wallet/collectors?count=10
GET https://zora-wrapped-back.onrender.com/api/creator/:wallet/collections?count=20
```
