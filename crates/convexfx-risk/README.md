# ConvexFX Risk

Risk parameter management and inventory risk modeling.

## Overview

Provides risk parameters (Γ, W matrices) that control inventory risk aversion and price covariance in the clearing algorithm.

## Key Types

### RiskParams

```rust
pub struct RiskParams {
    pub gamma: BTreeMap<AssetId, f64>,           // Inventory risk weights
    pub w_matrix: BTreeMap<(AssetId, AssetId), f64>,  // Covariance matrix
}
```

### Predefined Risk Profiles

```rust
impl RiskParams {
    pub fn ultra_low_slippage() -> Self;  // Minimal price impact
    pub fn default() -> Self;              // Balanced risk/return
    pub fn high_risk_aversion() -> Self;   // Conservative inventory
}
```

## Usage

```rust
use convexfx_risk::RiskParams;

// Use preset
let risk = RiskParams::ultra_low_slippage();

// Or build custom
let mut risk = RiskParams::default();
risk.gamma.insert(AssetId::USD, 0.001);  // Low USD risk weight
risk.gamma.insert(AssetId::JPY, 0.0001); // Very low JPY risk weight
```

## Risk Components

### Gamma (γ) - Inventory Risk Weights

Controls how much the exchange cares about inventory risk:

- **High γ**: More conservative, smaller positions, tighter prices
- **Low γ**: More aggressive, larger positions, better fills

```rust
// Conservative EUR position
risk.gamma.insert(AssetId::EUR, 0.01);

// Aggressive USD position
risk.gamma.insert(AssetId::USD, 0.0001);
```

### W Matrix - Price Covariance

Models how asset prices move together:

```rust
// EUR and USD positively correlated
risk.w_matrix.insert((AssetId::EUR, AssetId::USD), 0.8);

// JPY and EUR weakly correlated
risk.w_matrix.insert((AssetId::JPY, AssetId::EUR), 0.2);
```

## Objective Function Impact

Risk parameters affect the clearing objective:

```
minimize:
    Σ (price deviation)²               // Price tracking
  + Σ (unfilled order penalty)         // Fill quality
  + inventory' * Γ * inventory         // Inventory risk (from gamma)
  + inventory' * W * inventory         // Covariance risk (from W)
```

## Example: Tuning for Low Slippage

```rust
let risk = RiskParams {
    gamma: AssetId::all().iter().map(|a| (*a, 0.0001)).collect(),
    w_matrix: BTreeMap::new(),  // No covariance penalties
};

// Result: Clearing prioritizes fills over inventory management
// → Lower slippage, but may accumulate larger positions
```

## Example: Conservative Risk Management

```rust
let risk = RiskParams {
    gamma: AssetId::all().iter().map(|a| (*a, 0.1)).collect(),
    w_matrix: full_covariance_matrix(),
};

// Result: Clearing strongly avoids inventory accumulation
// → Higher slippage, but safer inventory exposure
```

## Matrix Utilities

```rust
use convexfx_risk::matrix_utils;

// Build covariance from historical data
let w_matrix = matrix_utils::estimate_covariance(&price_history)?;

// Validate positive definiteness
matrix_utils::validate_matrix(&w_matrix)?;
```

## Testing

```bash
cargo test -p convexfx-risk
```

## Best Practices

1. **Start Conservative**: Begin with `RiskParams::default()`
2. **Monitor Inventory**: Track actual inventory accumulation
3. **Adjust Gradually**: Tune γ based on observed risk
4. **Update W Regularly**: Refresh covariance estimates periodically
5. **Test Parameter Changes**: Use `convexfx-sim` to test before production

## Dependencies

- `convexfx-types`: Asset definitions
- `nalgebra`: Matrix operations (optional)

