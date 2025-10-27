# ConvexFX Fees

Inventory-aware fee computation with dynamic fee policies.

## Overview

Implements fee calculation logic that can adjust fees based on inventory state, promoting balanced flows and penalizing unbalanced trades.

## Key Types

### FeePolicy

```rust
pub trait FeePolicy: Send + Sync {
    fn compute_fee(
        &self,
        order: &PairOrder,
        inventory: &BTreeMap<AssetId, f64>,
    ) -> f64;
}
```

### Implementations

#### FlatFeePolicy

Simple fixed fee:

```rust
let policy = FlatFeePolicy::new(0.003);  // 0.3% fee
let fee = policy.compute_fee(&order, &inventory);
```

#### InventoryAwareFeePolicy

Dynamic fees based on inventory:

```rust
let policy = InventoryAwareFeePolicy {
    base_fee: 0.003,        // 0.3% base
    inventory_weight: 0.01,  // Inventory sensitivity
};

// If inventory is unbalanced:
// - Fee increases for trades worsening imbalance
// - Fee decreases (or rebates) for trades improving balance
```

## Usage

```rust
use convexfx_fees::{FlatFeePolicy, InventoryAwareFeePolicy, FeePolicy};

// Simple fee
let flat = FlatFeePolicy::new(0.001);  // 0.1%
let fee_amount = flat.compute_fee(&order, &inventory);

// Inventory-aware
let inv_aware = InventoryAwareFeePolicy::new(0.003, 0.01);
let adjusted_fee = inv_aware.compute_fee(&order, &inventory);

// Apply fee to fill
let gross_recv = 860.0;
let fee = gross_recv * adjusted_fee;
let net_recv = gross_recv - fee;
```

## Fee Policy Examples

### Flat Fee (0.3%)

```rust
let policy = FlatFeePolicy::new(0.003);

// Always 0.3% regardless of state
order1: 1000 USD → fee = 3 USD
order2: 10000 USD → fee = 30 USD
```

### Inventory-Aware Fee

```rust
let policy = InventoryAwareFeePolicy::new(0.003, 0.02);

// Current inventory: Heavy USD, Light EUR
// Scenario 1: USD → EUR (helps balance)
//   Fee: 0.2% (rebate!)
//
// Scenario 2: EUR → USD (worsens imbalance)
//   Fee: 0.5% (penalty)
```

## Fee Formula

For inventory-aware fees:

```
fee = base_fee + inventory_penalty

where:
inventory_penalty = inventory_weight * (
    | new_inventory_pay | - | old_inventory_pay |
  + | new_inventory_recv | - | old_inventory_recv |
)
```

## Integration with Clearing

Fees are typically applied post-clearing:

```rust
// 1. Clear orders
let solution = clearing.clear_epoch(&instance)?;

// 2. Apply fees to fills
for mut fill in solution.fills {
    let fee = fee_policy.compute_fee(&original_order, &inventory);
    fill.recv_units *= (1.0 - fee);  // Deduct fee from received amount
    
    // Credit fee to exchange
    ledger.credit(&"fee_pool".into(), fill.recv_asset, fee_amount)?;
}
```

## Rebates

Negative fees (rebates) incentivize balancing trades:

```rust
if fee < 0.0 {
    // Rebate: Give bonus to trader
    fill.recv_units *= (1.0 + fee.abs());
}
```

## Testing

```bash
cargo test -p convexfx-fees
```

Tests cover:
- Flat fee calculation
- Inventory-aware adjustments
- Rebate scenarios
- Edge cases

## Customization

Implement custom fee policies:

```rust
struct TimeBased FeePolicy { /* ... */ }

impl FeePolicy for TimeBasedFeePolicy {
    fn compute_fee(&self, order: &PairOrder, inventory: &_) -> f64 {
        // Lower fees during off-peak hours
        if is_off_peak() {
            0.001
        } else {
            0.003
        }
    }
}
```

## Dependencies

- `convexfx-types`: Order and asset types
- Standard library collections

