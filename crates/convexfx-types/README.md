# ConvexFX Types

Core type definitions for the ConvexFX exchange system.

## Overview

This crate provides the fundamental types used throughout the ConvexFX ecosystem, including assets, accounts, amounts, orders, prices, and epochs.

## Key Types

### Assets

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AssetId {
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    AUD,
}
```

Represents the supported currencies in the exchange.

### Accounts

```rust
pub type AccountId = String;
```

User account identifiers.

### Amounts

```rust
pub type Amount = i64;
```

Fixed-point representation of currency amounts (with implicit decimal places).

### Orders

```rust
pub struct PairOrder {
    pub trader: AccountId,
    pub pay_asset: AssetId,
    pub recv_asset: AssetId,
    pub pay_units: f64,
    pub limit_ratio: Option<f64>,
    pub min_fill_fraction: Option<f64>,
    pub metadata: serde_json::Value,
}
```

Represents a user's order to exchange one asset for another with optional constraints.

### Fills

```rust
pub struct Fill {
    pub order_id: String,
    pub trader: AccountId,
    pub pay_asset: AssetId,
    pub recv_asset: AssetId,
    pub pay_units: f64,
    pub recv_units: f64,
}
```

Represents the result of clearing an order.

### Prices

```rust
pub struct Prices {
    pub prices: BTreeMap<AssetId, f64>,
}
```

Exchange rates for all assets.

### Epochs

```rust
pub type EpochId = u64;
```

Identifier for batch clearing epochs.

## Features

- **Type Safety**: Strong typing prevents mixing incompatible types
- **Serialization**: All types implement `Serialize` and `Deserialize` for JSON/binary serialization
- **Ordering**: AssetId implements `Ord` for consistent ordering in maps
- **Error Types**: Comprehensive error handling with `ConvexFxError`

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
convexfx-types = { path = "../convexfx-types" }
```

Example:

```rust
use convexfx_types::{AssetId, PairOrder, AccountId};

let order = PairOrder {
    trader: "alice".to_string().into(),
    pay_asset: AssetId::USD,
    recv_asset: AssetId::EUR,
    pay_units: 1000.0,
    limit_ratio: Some(1.1),
    min_fill_fraction: Some(0.9),
    metadata: serde_json::json!({}),
};
```

## Error Handling

The crate defines `ConvexFxError` for all exchange-related errors:

```rust
pub enum ConvexFxError {
    InsufficientBalance,
    InvalidAsset(String),
    InvalidAmount,
    OrderNotFound(String),
    // ... more variants
}
```

## Testing

```bash
cargo test -p convexfx-types
```

## Dependencies

- `serde`: Serialization support
- `serde_json`: JSON handling
- `thiserror`: Error derive macros

