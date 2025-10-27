# ConvexFX Report

Epoch execution reporting and cryptographic hashing.

## Overview

Generates comprehensive reports for each clearing epoch, including fills, prices, inventory changes, and cryptographic commitments.

## Key Types

### EpochReport

```rust
pub struct EpochReport {
    pub epoch_id: EpochId,
    pub timestamp: u64,
    pub orders_count: usize,
    pub fills: Vec<Fill>,
    pub clearing_prices: Prices,
    pub initial_inventory: BTreeMap<AssetId, f64>,
    pub final_inventory: BTreeMap<AssetId, f64>,
    pub objective_value: f64,
    pub report_hash: String,
}
```

### Reporter

```rust
pub struct Reporter;

impl Reporter {
    pub fn generate_report(solution: &EpochSolution) -> EpochReport;
    pub fn hash_report(report: &EpochReport) -> String;
    pub fn verify_report(report: &EpochReport) -> bool;
}
```

## Usage

```rust
use convexfx_report::Reporter;

// After clearing
let solution = clearing.clear_epoch(&instance)?;

// Generate report
let report = Reporter::generate_report(&solution);

println!("Epoch {}: {} fills", report.epoch_id, report.fills.len());
println!("Clearing prices: {:?}", report.clearing_prices);
println!("Report hash: {}", report.report_hash);

// Verify integrity
assert!(Reporter::verify_report(&report));
```

## Report Contents

### Execution Summary

```rust
report.epoch_id: 42
report.timestamp: 1709845234
report.orders_count: 157
report.fills.len(): 143  // 91% fill rate
report.objective_value: 0.0234
```

### Price Data

```rust
report.clearing_prices = Prices {
    USD: 1.0,
    EUR: 1.163,   // USD per EUR
    GBP: 1.299,
    JPY: 0.0067,
    CHF: 1.136,
    AUD: 0.667,
}
```

### Inventory Tracking

```rust
report.initial_inventory = {
    USD: 100000.0,
    EUR: 116280.0,
    // ...
}

report.final_inventory = {
    USD: 98500.0,   // -1500 (sold USD)
    EUR: 117550.0,  // +1270 (bought EUR)
    // ...
}
```

### Fill Details

```rust
for fill in report.fills {
    println!("{} traded {} {} for {} {}",
        fill.trader,
        fill.pay_units, fill.pay_asset,
        fill.recv_units, fill.recv_asset
    );
}
```

## Cryptographic Hashing

Reports are hashed for integrity:

```rust
let report_hash = Reporter::hash_report(&report);

// Hash includes:
// - Epoch ID
// - All fills (trader, assets, amounts)
// - Clearing prices
// - Inventory changes
// - Timestamp

// Verify report hasn't been tampered with
assert_eq!(report.report_hash, report_hash);
```

## Report Serialization

```rust
// JSON export
let json = serde_json::to_string_pretty(&report)?;
std::fs::write("epoch_42_report.json", json)?;

// Binary export
let bytes = bincode::serialize(&report)?;
std::fs::write("epoch_42_report.bin", bytes)?;
```

## Audit Trail

Reports form an audit trail:

```rust
// Store reports sequentially
reports.push(epoch_1_report);
reports.push(epoch_2_report);
reports.push(epoch_3_report);

// Verify chain integrity
for report in reports {
    assert!(Reporter::verify_report(report));
}
```

## Analytics

Extract metrics from reports:

```rust
// Total volume
let total_volume: f64 = report.fills
    .iter()
    .map(|f| f.pay_units)
    .sum();

// Average fill ratio
let avg_fill = report.fills.len() as f64 / report.orders_count as f64;

// Price movements
let eur_price_change = report.clearing_prices.get(AssetId::EUR) 
    - previous_report.clearing_prices.get(AssetId::EUR);
```

## Testing

```bash
cargo test -p convexfx-report
```

Tests include:
- Report generation
- Hash computation
- Verification
- Serialization round-trips

## Dependencies

- `convexfx-types`: Core types
- `serde`: Serialization
- `sha2`: Hashing
- `hex`: Hash encoding

