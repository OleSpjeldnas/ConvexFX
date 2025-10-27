# ConvexFX Oracle

Price feed and reference price management.

## Overview

Provides oracle abstractions for fetching external price data and managing reference prices for the clearing algorithm.

## Key Types

### Oracle Trait

```rust
pub trait Oracle: Send + Sync {
    fn get_price(&self, asset: AssetId) -> Result<f64, OracleError>;
    fn get_all_prices(&self) -> Result<BTreeMap<AssetId, f64>, OracleError>;
}
```

### RefPrices

Log-space price representation for clearing:

```rust
pub struct RefPrices {
    pub y_ref: BTreeMap<AssetId, f64>,  // log(price)
    pub band_bps: f64,                   // price band in basis points
}

impl RefPrices {
    pub fn new(prices: BTreeMap<AssetId, f64>, band_bps: f64) -> Self;
    pub fn get_ref(&self, asset: AssetId) -> f64;
}
```

## Implementations

### MockOracle

Fixed prices for testing:

```rust
let mut oracle = MockOracle::new();
oracle.set_price(AssetId::USD, 1.0);
oracle.set_price(AssetId::EUR, 0.86);  // 1 USD = 0.86 EUR
oracle.set_price(AssetId::GBP, 0.77);
oracle.set_price(AssetId::JPY, 149.0);

let ref_prices = oracle.get_ref_prices(5.0)?;  // 5 bps band
```

## Usage

```rust
use convexfx_oracle::{MockOracle, Oracle, RefPrices};

// Set up oracle
let mut oracle = MockOracle::new();
oracle.set_price(AssetId::USD, 1.0);
oracle.set_price(AssetId::EUR, 1.0 / 0.86);  // USD per EUR

// Get reference prices for clearing
let ref_prices = RefPrices::new(
    oracle.get_all_prices()?,
    5.0,  // 5 basis point price band
);

// Use in clearing
let instance = EpochInstance {
    ref_prices,
    // ... other fields
};
```

## Log-Space Pricing

ConvexFX uses log-space prices for numerical stability:

```rust
// If 1 USD = 0.86 EUR, then:
let y_usd = 0.0;                    // USD is reference
let y_eur = -(0.86_f64).ln();       // log(0.86) ≈ -0.15

// Conversion back:
let eur_per_usd = (y_usd - y_eur).exp();  // = 0.86
```

## Price Bands

Price bands prevent excessive deviation from reference:

```rust
let ref_prices = RefPrices::new(prices, 50.0);  // 50 bps = 0.5%

// Clearing prices constrained to:
// reference_price * (1 - 0.005) ≤ clearing_price ≤ reference_price * (1 + 0.005)
```

## Production Oracles

Implement the `Oracle` trait for real price feeds:

```rust
struct ChainlinkOracle { /* ... */ }

impl Oracle for ChainlinkOracle {
    fn get_price(&self, asset: AssetId) -> Result<f64, OracleError> {
        // Fetch from Chainlink price feed
    }
}
```

## Testing

```bash
cargo test -p convexfx-oracle
```

## Dependencies

- `convexfx-types`: Asset definitions
- Standard library collections

