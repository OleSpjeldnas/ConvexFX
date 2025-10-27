# ConvexFX Exchange

High-level exchange orchestration combining ledger, oracle, clearing, and order management.

## Overview

The Exchange struct provides a complete, ready-to-use FX exchange system that coordinates all ConvexFX components.

## Architecture

```
┌──────────────────────────────────────┐
│           Exchange                   │
│  ┌────────────────────────────────┐  │
│  │  Ledger (Account balances)     │  │
│  ├────────────────────────────────┤  │
│  │  OrderBook (Pending orders)    │  │
│  ├────────────────────────────────┤  │
│  │  Oracle (Price feeds)          │  │
│  ├────────────────────────────────┤  │
│  │  RiskParams (Γ, W matrices)    │  │
│  ├────────────────────────────────┤  │
│  │  ScpClearing (Batch clearing)  │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

## Key Features

- **Account Management**: Track balances across all supported assets
- **Order Submission**: Accept and validate user orders
- **Epoch Clearing**: Batch orders and execute via SCP clearing
- **Price Discovery**: Automatic price computation
- **Risk Management**: Inventory-aware pricing and risk controls

## Usage

### Initialization

```rust
use convexfx_exchange::{Exchange, ExchangeConfig};

let config = ExchangeConfig {
    initial_inventory: initial_balances(),
    risk_params: RiskParams::default(),
    clearing_interval_ms: 200,
};

let mut exchange = Exchange::new(config)?;
```

### Submit Orders

```rust
let order = PairOrder {
    trader: "alice".into(),
    pay_asset: AssetId::USD,
    recv_asset: AssetId::EUR,
    pay_units: 1000.0,
    limit_ratio: Some(1.1),
    min_fill_fraction: Some(0.9),
    metadata: serde_json::json!({}),
};

exchange.submit_order(order)?;
```

### Process Epoch

```rust
// Wait for epoch to fill with orders
tokio::time::sleep(Duration::from_millis(200)).await;

// Clear the epoch
let epoch_result = exchange.process_epoch()?;

println!("Fills: {:?}", epoch_result.fills);
println!("Clearing prices: {:?}", epoch_result.clearing_prices);
```

### Query State

```rust
// Get account balance
let balance = exchange.get_balance("alice", AssetId::USD)?;

// Get current prices
let prices = exchange.get_current_prices()?;

// Get total liquidity
let liquidity = exchange.get_total_liquidity()?;
```

## Configuration

```rust
pub struct ExchangeConfig {
    /// Initial exchange inventory
    pub initial_inventory: BTreeMap<AssetId, f64>,
    
    /// Risk parameters for clearing
    pub risk_params: RiskParams,
    
    /// Epoch duration in milliseconds
    pub clearing_interval_ms: u64,
    
    /// Enable commit-reveal for MEV protection
    pub use_commit_reveal: bool,
}
```

## State Management

The exchange maintains:

1. **Ledger State**: All account balances
2. **Order State**: Pending orders per epoch
3. **Inventory State**: Exchange's asset holdings
4. **Price State**: Latest clearing prices

## Error Handling

```rust
pub enum ExchangeError {
    InsufficientBalance { account: String, asset: AssetId },
    InvalidOrder(String),
    ClearingFailed(String),
    LedgerError(LedgerError),
    // ...
}
```

## Testing

```bash
cargo test -p convexfx-exchange
```

Includes integration tests for:
- Multi-user trading scenarios
- Partial fill handling
- Price discovery
- Inventory management

## Examples

See `examples/simple_exchange.rs`:

```bash
cargo run --example simple_exchange
```

## Performance

- Handles 100+ concurrent traders
- Processes 200-500 orders per epoch
- < 100ms clearing latency
- Supports 6+ asset pairs

## Dependencies

- `convexfx-ledger`: Account management
- `convexfx-orders`: Order book
- `convexfx-oracle`: Price feeds
- `convexfx-clearing`: SCP clearing engine
- `convexfx-risk`: Risk parameters

