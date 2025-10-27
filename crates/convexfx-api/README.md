# ConvexFX API

REST API server for the ConvexFX exchange.

## Overview

Provides HTTP endpoints for interacting with the ConvexFX exchange, including order submission, balance queries, and price discovery.

## Quick Start

```rust
use convexfx_api::server;
use convexfx_exchange::Exchange;

let exchange = Exchange::new(config)?;
let server = server::start(exchange, "0.0.0.0:8080").await?;
```

## Endpoints

### Health Check

```
GET /health
```

Returns 200 OK if server is running.

### Get Prices

```
GET /prices
```

Returns current clearing prices for all assets.

**Response:**
```json
{
  "USD": 1.0,
  "EUR": 1.163,
  "GBP": 1.299,
  "JPY": 0.0067,
  "CHF": 1.136,
  "AUD": 0.667
}
```

### Get Balance

```
GET /balance/{account}/{asset}
```

Returns account balance for specific asset.

**Response:**
```json
{
  "account": "alice",
  "asset": "USD",
  "balance": 10000
}
```

### Submit Order

```
POST /orders
Content-Type: application/json

{
  "trader": "alice",
  "pay_asset": "USD",
  "recv_asset": "EUR",
  "pay_units": 1000.0,
  "limit_ratio": 1.1,
  "min_fill_fraction": 0.9
}
```

**Response:**
```json
{
  "order_id": "ord_abc123",
  "status": "pending"
}
```

### Get Order Status

```
GET /orders/{order_id}
```

Returns order status and fill details if executed.

### Get Epoch Report

```
GET /epochs/{epoch_id}
```

Returns full epoch clearing report.

**Response:**
```json
{
  "epoch_id": 42,
  "timestamp": 1709845234,
  "orders_count": 157,
  "fills": [...],
  "clearing_prices": {...},
  "report_hash": "0xabc..."
}
```

## WebSocket Support

Real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  if (update.type === 'fill') {
    console.log('New fill:', update.data);
  } else if (update.type === 'prices') {
    console.log('Price update:', update.data);
  }
};
```

## Authentication

For production, implement authentication middleware:

```rust
use actix_web::middleware::auth;

server::start_with_auth(
    exchange,
    "0.0.0.0:8080",
    JwtAuth::new(secret_key)
).await?;
```

## Rate Limiting

Built-in rate limiting to prevent abuse:

```rust
server::Config {
    rate_limit: RateLimit {
        requests_per_minute: 60,
        burst: 10,
    },
    // ...
}
```

## CORS

Configure CORS for web clients:

```rust
server::Config {
    cors: CorsConfig {
        allow_origins: vec!["https://app.convexfx.com"],
        allow_methods: vec!["GET", "POST"],
    },
    // ...
}
```

## Monitoring

Prometheus metrics endpoint:

```
GET /metrics
```

Exports:
- Request counts
- Response times
- Order volumes
- Fill rates
- Active connections

## Error Responses

Standard error format:

```json
{
  "error": "InsufficientBalance",
  "message": "Account alice has insufficient USD balance",
  "details": {
    "required": 1000,
    "available": 500
  }
}
```

HTTP status codes:
- 200: Success
- 400: Bad request (invalid parameters)
- 404: Not found
- 429: Rate limit exceeded
- 500: Server error

## Testing

```bash
cargo test -p convexfx-api
```

## Example Client

```rust
use reqwest::Client;

let client = Client::new();

// Submit order
let response = client
    .post("http://localhost:8080/orders")
    .json(&order)
    .send()
    .await?;

let order_id: String = response.json().await?;
```

## Dependencies

- `actix-web`: Web framework
- `serde_json`: JSON handling
- `tokio`: Async runtime
- `convexfx-exchange`: Exchange core

