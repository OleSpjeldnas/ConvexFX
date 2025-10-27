# ConvexFX Orders

Order book management with commit-reveal support for MEV protection.

## Overview

Manages order submission, validation, and epoch-based batching with optional commit-reveal mechanism to prevent MEV attacks.

## Key Types

### OrderBook

```rust
pub struct OrderBook {
    pending_orders: Vec<PairOrder>,
    commit_reveal: bool,
    // ...
}

impl OrderBook {
    pub fn submit_order(&mut self, order: PairOrder) -> Result<String, OrderError>;
    pub fn get_pending_orders(&self) -> Vec<PairOrder>;
    pub fn clear_epoch(&mut self);
}
```

### Order Validation

```rust
pub fn validate_order(order: &PairOrder) -> Result<(), ValidationError> {
    // Check asset validity
    // Check amount > 0
    // Check limit_ratio if specified
    // Check min_fill_fraction in [0, 1]
}
```

## Features

### Standard Order Submission

```rust
let mut orderbook = OrderBook::new(false);  // No commit-reveal

let order = PairOrder {
    trader: "alice".into(),
    pay_asset: AssetId::USD,
    recv_asset: AssetId::EUR,
    pay_units: 1000.0,
    limit_ratio: Some(1.1),
    min_fill_fraction: Some(0.9),
    metadata: serde_json::json!({}),
};

let order_id = orderbook.submit_order(order)?;
```

### Commit-Reveal (MEV Protection)

Two-phase submission to prevent front-running:

```rust
let mut orderbook = OrderBook::new(true);  // Enable commit-reveal

// Phase 1: Commit (hash only)
let order_hash = orderbook.commit_order(hash)?;

// Wait for commit phase to close...

// Phase 2: Reveal (actual order)
orderbook.reveal_order(order_id, order)?;
```

## Order Lifecycle

```
1. Submit → 2. Validate → 3. Batch → 4. Clear → 5. Execute
```

1. **Submit**: User submits order to orderbook
2. **Validate**: Check order parameters
3. **Batch**: Collect orders during epoch window
4. **Clear**: Run SCP clearing algorithm
5. **Execute**: Apply fills to ledger

## Order Types

### Market Orders

No limit ratio (accept any price):

```rust
PairOrder {
    limit_ratio: None,  // No price limit
    min_fill_fraction: Some(0.0),  // Accept partial fills
    // ...
}
```

### Limit Orders

With maximum price:

```rust
PairOrder {
    limit_ratio: Some(1.1),  // Accept up to 1.1 EUR per USD
    min_fill_fraction: Some(1.0),  // All or nothing
    // ...
}
```

### Partial Fill Orders

```rust
PairOrder {
    limit_ratio: Some(1.1),
    min_fill_fraction: Some(0.5),  // Accept ≥ 50% fill
    // ...
}
```

## Order Metadata

Orders can carry arbitrary JSON metadata:

```rust
PairOrder {
    metadata: serde_json::json!({
        "client_id": "mobile-app-v2",
        "reference": "payroll-batch-2024-03",
        "priority": "high",
    }),
    // ...
}
```

## Epoch Management

```rust
// During epoch: Collect orders
orderbook.submit_order(order1)?;
orderbook.submit_order(order2)?;
orderbook.submit_order(order3)?;

// At epoch end: Extract orders for clearing
let orders_to_clear = orderbook.get_pending_orders();

// After clearing: Reset for next epoch
orderbook.clear_epoch();
```

## Testing

```bash
cargo test -p convexfx-orders
```

Tests include:
- Order validation
- Commit-reveal flow
- Epoch batching
- Order cancellation

## Security

### MEV Protection

Commit-reveal prevents:
- **Front-running**: Attackers can't see order details during commit phase
- **Sandwich attacks**: Orders revealed simultaneously
- **Just-in-time MEV**: No order priority based on submission time

### Order Authentication

Orders should be signed:

```rust
// In production, verify signatures
verify_signature(&order.trader, &order_hash, &signature)?;
```

## Dependencies

- `convexfx-types`: Order types
- `serde_json`: Metadata handling
- `sha2`: Commit hashing (for commit-reveal)

