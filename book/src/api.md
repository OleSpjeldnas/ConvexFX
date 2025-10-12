# HTTP API Reference

ConvexFX exposes an Axum-based HTTP service for submitting FX orders, inspecting market data, and monitoring the clearing engine. The server starts on `http://127.0.0.1:3000` when you run `cargo run -p convexfx-api`. 【F:crates/convexfx-api/src/main.rs†L1-L21】

All examples in this chapter assume the API is reachable at the default base URL and that you are using the development in-memory backends provided by `AppState`. 【F:crates/convexfx-api/src/state.rs†L12-L34】

## Common data types

### Assets
The API accepts ISO-style asset symbols that map to the `AssetId` enum used internally. Supported assets are:

- `USD`
- `EUR`
- `JPY`
- `GBP`
- `CHF`
- `AUD`

Any other value will return `400 Bad Request` during order submission. 【F:crates/convexfx-types/src/asset.rs†L5-L45】【F:crates/convexfx-api/src/handlers.rs†L60-L82】

### Dynamic Asset Registration
The system now supports dynamic asset registration at runtime via the API. New assets can be added with custom names and decimal precision. All registered assets participate in the coherent pricing optimization. 【F:crates/convexfx-types/src/asset.rs†L6-L72】【F:crates/convexfx-oracle/src/mock.rs†L73-L78】

### Amounts
Budget amounts are encoded as decimal strings and parsed into the fixed-point `Amount` type with 9 decimal places. Invalid strings result in a `400 Bad Request` response. 【F:crates/convexfx-types/src/amount.rs†L6-L88】【F:crates/convexfx-api/src/handlers.rs†L84-L95】

### Order lifecycle
`POST /v1/orders/submit` creates a commitment hash for a new order and stores it in the in-memory order book. The subsequent `POST /v1/orders/reveal` call would normally reveal the signed order; in the reference implementation it always returns success and places the order into epoch `1`. 【F:crates/convexfx-api/src/handlers.rs†L97-L154】【F:crates/convexfx-api/src/handlers.rs†L156-L169】

## Health and metadata

### `GET /health`
Returns service status and the running crate version.

```bash
curl http://127.0.0.1:3000/health
```

Example response:

```json
{"status":"ok","version":"0.1.0"}
```

【F:crates/convexfx-api/src/handlers.rs†L22-L31】

### `GET /v1/info`
Provides a human-readable service description.

```bash
curl http://127.0.0.1:3000/v1/info
```

Example response:

```json
{"name":"ConvexFX","description":"Sequential Convex Programming FX AMM"}
```

【F:crates/convexfx-api/src/handlers.rs†L33-L41】

## Order endpoints

### `POST /v1/orders/submit`
Creates a commitment for a new FX order. Fields:

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `pay_asset` | string | ✅ | Asset symbol to pay. |
| `receive_asset` | string | ✅ | Asset symbol to receive. |
| `budget` | string | ✅ | Budget amount in decimal string form. |
| `limit_ratio` | float | optional | Maximum acceptable receive/pay ratio. |
| `min_fill_fraction` | float | optional | Minimum acceptable fill percentage. |

Example request:

```bash
curl -X POST http://127.0.0.1:3000/v1/orders/submit \
  -H "Content-Type: application/json" \
  -d '{
        "pay_asset": "EUR",
        "receive_asset": "USD",
        "budget": "1_000_000.0",
        "limit_ratio": 1.08,
        "min_fill_fraction": 0.5
      }'
```

Example response (values will vary per request):

```json
{
  "order_id": "order_1700000000000000000",
  "commitment_hash": "c6c1…",
  "accepted": true
}
```

On validation failure the endpoint returns `400` with an `error` message. 【F:crates/convexfx-api/src/handlers.rs†L97-L154】

### `POST /v1/orders/reveal`
Reveals a previously committed order. The current implementation accepts any payload and echoes that the order was added to epoch `1`.

```bash
curl -X POST http://127.0.0.1:3000/v1/orders/reveal \
  -H "Content-Type: application/json" \
  -d '{
        "order_id": "order_1700000000000000000",
        "pay_asset": "EUR",
        "receive_asset": "USD",
        "budget": "1000000",
        "trader": "alice",
        "limit_ratio": 1.08,
        "min_fill_fraction": 0.5
      }'
```

Response:

```json
{"accepted":true,"epoch_id":1}
```

【F:crates/convexfx-api/src/handlers.rs†L156-L169】

### `POST /v1/orders/commit`
Legacy commitment endpoint that simply acknowledges the epoch hint. Useful for compatibility tests.

```bash
curl -X POST http://127.0.0.1:3000/v1/orders/commit \
  -H "Content-Type: application/json" \
  -d '{"epoch_hint": 1}'
```

Response:

```json
{"accepted":true,"epoch":1}
```

【F:crates/convexfx-api/src/handlers.rs†L205-L217】

## Market data

### `GET /v1/prices`
Returns the latest oracle log-prices and their exponentiated spot prices for each supported asset alongside the epoch identifier.

```bash
curl http://127.0.0.1:3000/v1/prices
```

Example response:

```json
{
  "prices": [
    {"asset":"USD","price":1.0,"log_price":0.0},
    {"asset":"EUR","price":1.1025,"log_price":0.0976},
    …
  ],
  "epoch_id": 1
}
```

【F:crates/convexfx-api/src/handlers.rs†L171-L189】

## Epoch management

### `GET /v1/epochs`
Lists recent epochs with summary statistics. The development implementation returns a static example.

```bash
curl http://127.0.0.1:3000/v1/epochs
```

Response:

```json
{"epochs":[{"epoch_id":1,"state":"COMPLETED","order_count":5,"start_time":"2025-01-01T00:00:00Z","end_time":"2025-01-01T00:01:00Z"}]}
```

【F:crates/convexfx-api/src/handlers.rs†L191-L208】

### `GET /v1/epochs/current`
Returns the currently active epoch identifier and state.

```bash
curl http://127.0.0.1:3000/v1/epochs/current
```

Response:

```json
{"epoch_id":1,"state":"COLLECT"}
```

【F:crates/convexfx-api/src/handlers.rs†L231-L239】

### `GET /v1/epochs/:epoch_id`
Fetches a single epoch record. The placeholder implementation always returns epoch `1` data regardless of input.

```bash
curl http://127.0.0.1:3000/v1/epochs/42
```

Response:

```json
{"epoch_id":1,"state":"COMPLETED","order_count":0,"start_time":null,"end_time":null}
```

【F:crates/convexfx-api/src/handlers.rs†L210-L229】

## System status

### `GET /v1/status`
Reports aggregate system metrics collected from the in-memory ledger, orderbook, and solver configuration.

```bash
curl http://127.0.0.1:3000/v1/status
```

Response:

```json
{
  "status": "healthy",
  "current_epoch": 1,
  "total_accounts": 0,
  "total_orders_pending": 0,
  "solver_backend": "clarabel",
  "uptime_seconds": 3600
}
```

【F:crates/convexfx-api/src/handlers.rs†L183-L204】

## Asset management

### `GET /v1/assets`
Returns a list of all registered assets with their metadata and current prices.

```bash
curl http://127.0.0.1:3000/v1/assets
```

Example response:

```json
{
  "assets": [
    {
      "symbol": "USD",
      "name": "US Dollar",
      "decimals": 2,
      "is_base_currency": true,
      "current_price": 1.0
    },
    {
      "symbol": "EUR",
      "name": "Euro",
      "decimals": 2,
      "is_base_currency": false,
      "current_price": 1.1025
    }
  ]
}
```

【F:crates/convexfx-api/src/handlers.rs†L262-L279】

### `POST /v1/assets`
Registers a new asset in the system. Fields:

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `symbol` | string | ✅ | Asset symbol (2-10 characters, uppercase). |
| `name` | string | ✅ | Human-readable asset name. |
| `decimals` | integer | ✅ | Number of decimal places for the asset. |
| `is_base_currency` | boolean | ✅ | Whether this is a base currency (e.g., USD). |
| `initial_price` | float | ✅ | Initial price in USD (for non-base currencies). |

Example request:

```bash
curl -X POST http://127.0.0.1:3000/v1/assets \
  -H "Content-Type: application/json" \
  -d '{
        "symbol": "BTC",
        "name": "Bitcoin",
        "decimals": 8,
        "is_base_currency": false,
        "initial_price": 50000.0
      }'
```

Response:

```json
{
  "success": true,
  "asset_id": "BTC",
  "message": "Asset BTC added successfully"
}
```

【F:crates/convexfx-api/src/handlers.rs†L244-L261】

## Liquidity management

### `GET /v1/liquidity`
Returns current liquidity/balances for all accounts in the system.

```bash
curl http://127.0.0.1:3000/v1/liquidity
```

Example response:

```json
{
  "account_1": {
    "USD": "1000000.000000000",
    "EUR": "500000.000000000"
  },
  "liquidity_provider_1": {
    "BTC": "1000000000000.000000000"
  }
}
```

【F:crates/convexfx-api/src/handlers.rs†L305-L320】

### `POST /v1/liquidity`
Provides liquidity by depositing assets into an account.

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `account_id` | string | ✅ | Account identifier. |
| `asset_symbol` | string | ✅ | Asset symbol to deposit. |
| `amount` | string | ✅ | Amount to deposit (decimal string). |

Example request:

```bash
curl -X POST http://127.0.0.1:3000/v1/liquidity \
  -H "Content-Type: application/json" \
  -d '{
        "account_id": "liquidity_provider_1",
        "asset_symbol": "BTC",
        "amount": "1000000000000"
      }'
```

Response:

```json
{
  "success": true,
  "message": "Liquidity provided successfully",
  "new_balance": "1000000000000.000000000"
}
```

【F:crates/convexfx-api/src/handlers.rs†L281-L304】

## CORS
The API enables a permissive CORS layer so that browser-based clients can issue cross-origin requests without additional configuration. 【F:crates/convexfx-api/src/server.rs†L5-L33】
