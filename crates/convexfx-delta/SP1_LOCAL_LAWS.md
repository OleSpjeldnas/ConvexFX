# SP1 Local Laws Implementation Guide

## Overview

This document explains how ConvexFX uses SP1 (Succinct Processor 1) zkVM to cryptographically prove that clearing results satisfy all local laws before submission to the Delta base layer.

## Architecture

### High-Level Flow

```
User Orders
    │
    ▼
ConvexFX SCP Clearing
    │
    ▼
Clearing Solution
    │
    ├─→ Off-Chain Validation (predicates.rs)
    │   ✓ Fast check before expensive proving
    │
    └─→ SP1 Proving (sp1_prover.rs)
        │
        ├─→ Prepare Input (ClearingProofInput)
        │
        ├─→ Execute SP1 Program (convexfx-sp1-program)
        │   ├─ Convergence Check
        │   ├─ Price Consistency
        │   ├─ Fill Feasibility
        │   ├─ Inventory Conservation
        │   └─ Objective Optimality
        │
        ├─→ Generate ZKP Proof
        │
        └─→ Submit to Delta
            └─→ Base Layer Verifies Proof
                ├─ Valid: Apply State Diffs
                └─ Invalid: Reject
```

## Components

### 1. SP1 Program (`crates/convexfx-sp1-program`)

The SP1 program is a standalone Rust program that runs in the SP1 zkVM. It encodes all ConvexFX business rules as assertions.

**File Structure:**
```
crates/convexfx-sp1-program/
├── Cargo.toml              # SP1 dependencies
├── src/
│   └── main.rs             # Local laws program
└── elf/                    # Generated ELF binary (after build)
```

**Key Code:**
```rust
#![no_main]
sp1_zkvm::entrypoints::init_io();

pub fn main() {
    let input: ClearingProofInput = sp1_zkvm::io::read();
    
    // All predicates are encoded as assertions
    assert!(input.convergence_achieved);
    assert!(input.final_step_norm_y < TOLERANCE_Y);
    // ... more assertions ...
    
    sp1_zkvm::io::commit(&true);  // Success
}
```

### 2. SP1 Prover (`crates/convexfx-delta/src/sp1_prover.rs`)

The prover client prepares inputs, calls the SP1 SDK, and extracts proofs.

**Key Functions:**
- `new()` - Initialize SP1 prover
- `get_vkey()` - Extract verification key for domain agreement
- `prove_clearing()` - Generate proof for a clearing solution
- `prepare_input()` - Convert ConvexFX data to SP1 format

### 3. Domain Agreement Integration

The verification key is submitted when registering the executor:

```rust
// Extract vkey from SP1 program
let sp1_prover = ConvexFxSp1Prover::new();
let local_laws_vkey = sp1_prover.get_vkey();

// Submit with executor lease agreement
let ela = ExecutorLeaseAgreement::new(
    executor_operator_pubkey,
    shard_id,
    Some(local_laws_vkey),  // ← ConvexFX local laws
);
```

## Local Laws (Predicates)

### 1. Convergence Validation

**Purpose:** Ensure SCP algorithm converged to optimal solution

**SP1 Code:**
```rust
assert!(input.convergence_achieved, "SCP did not converge");
assert!(input.final_step_norm_y < TOLERANCE_Y, "Y step norm too large");
assert!(input.final_step_norm_alpha < TOLERANCE_ALPHA, "Alpha step norm too large");
```

**Parameters:**
- `TOLERANCE_Y = 1e-5` - Price convergence tolerance
- `TOLERANCE_ALPHA = 1e-6` - Fill convergence tolerance

### 2. Price Consistency Validation

**Purpose:** Verify price relationships are mathematically correct

**SP1 Code:**
```rust
for (asset_id, log_price) in &input.y_star {
    let linear_price = find_price(asset_id, &input.prices);
    let expected = log_price.exp();
    let error = (expected - linear_price).abs() / linear_price;
    assert!(error < MAX_PRICE_DEVIATION);
}

// USD numeraire constraint
let usd_log = find_log_price(USD_ASSET_ID, &input.y_star);
assert!(usd_log.abs() < TOLERANCE_Y);
```

**Parameters:**
- `MAX_PRICE_DEVIATION = 0.01` - 1% maximum deviation
- `USD_ASSET_ID = 0` - USD as numeraire

### 3. Fill Feasibility Validation

**Purpose:** Ensure all fills are valid

**SP1 Code:**
```rust
for fill in &input.fills {
    assert!(fill.fill_frac >= 0.0 && fill.fill_frac <= 1.0);
    
    if fill.fill_frac > 0.0 {
        assert!(fill.pay_units > 0.0);
        assert!(fill.recv_units > 0.0);
        assert!(fill.pay_units.is_finite());
        assert!(fill.recv_units.is_finite());
    }
}
```

### 4. Inventory Conservation Validation

**Purpose:** Verify conservation law: `final = initial + net_flow`

**SP1 Code:**
```rust
for (asset_id, initial) in &input.initial_inventory {
    let final_inv = find_final(asset_id, &input.final_inventory);
    let net_flow = calculate_net_flow(asset_id, &input.fills);
    let error = (final_inv - (initial + net_flow)).abs();
    assert!(error < INVENTORY_TOLERANCE);
}
```

**Parameters:**
- `INVENTORY_TOLERANCE = 1e-6` - Numerical error tolerance

### 5. Objective Optimality Validation

**Purpose:** Ensure objective function is properly computed

**SP1 Code:**
```rust
assert!(input.inventory_risk >= -INVENTORY_TOLERANCE);
assert!(input.price_tracking >= -INVENTORY_TOLERANCE);
assert!(input.total_objective.is_finite());

let computed = input.inventory_risk + input.price_tracking + input.fill_incentive;
let error = (input.total_objective - computed).abs();
assert!(error < INVENTORY_TOLERANCE);
```

## Data Structures

### ClearingProofInput

```rust
#[derive(Serialize, Deserialize)]
struct ClearingProofInput {
    // Prices (asset_id, value)
    y_star: Vec<(u8, f64)>,         // Log prices
    prices: Vec<(u8, f64)>,          // Linear prices
    
    // Fills
    fills: Vec<FillData>,
    
    // Inventory
    initial_inventory: Vec<(u8, f64)>,
    final_inventory: Vec<(u8, f64)>,
    
    // Diagnostics
    convergence_achieved: bool,
    final_step_norm_y: f64,
    final_step_norm_alpha: f64,
    
    // Objective
    inventory_risk: f64,
    price_tracking: f64,
    fill_incentive: f64,
    total_objective: f64,
}
```

### FillData

```rust
#[derive(Serialize, Deserialize)]
struct FillData {
    fill_frac: f64,
    pay_asset: u8,
    recv_asset: u8,
    pay_units: f64,
    recv_units: f64,
}
```

## Testing

### Unit Tests

```bash
# Test SP1 prover creation
cargo test -p convexfx-delta test_sp1_prover_creation

# Test input preparation
cargo test -p convexfx-delta test_prepare_input
```

### Integration Tests

```bash
# Test full proving flow
cargo test -p convexfx-delta test_sp1_proof_generation_valid_clearing

# Test rejection of invalid solutions
cargo test -p convexfx-delta test_sp1_proof_reject_non_convergent

# Test with demo app
cargo test -p convexfx-delta test_sp1_with_demo_app
```

### Test Coverage

11 comprehensive SP1 tests covering:
- ✅ Proof generation for valid clearing
- ✅ Rejection of non-convergent solutions
- ✅ Rejection of high step norms
- ✅ Empty batch handling
- ✅ Large batch (20 orders) proving
- ✅ Multi-asset trading proofs
- ✅ Serialization/deserialization
- ✅ Proof determinism
- ✅ Verification key generation
- ✅ Demo app integration

## Production Deployment

### Step 1: Build SP1 Program

```bash
# Install SP1 toolchain
curl -L https://sp1.succinct.xyz | bash
sp1up

# Build the program
cd crates/convexfx-sp1-program
cargo prove build
```

This generates `elf/riscv32im-succinct-zkvm-elf`.

### Step 2: Update Prover with Production SDK

```rust
// Add to Cargo.toml
sp1-sdk = "2.0.0"

// Update sp1_prover.rs
pub const CONVEXFX_SP1_ELF: &[u8] = include_bytes!(
    "../../convexfx-sp1-program/elf/riscv32im-succinct-zkvm-elf"
);

impl ConvexFxSp1Prover {
    pub fn new() -> Self {
        Self {
            client: ProverClient::new(),
        }
    }
    
    pub fn get_vkey(&self) -> Vec<u8> {
        let (_, vkey) = self.client.setup(CONVEXFX_SP1_ELF);
        vkey.bytes()
    }
}
```

### Step 3: Submit Domain Agreement

```bash
cargo run --bin convexfx-delta -- submit-domain-agreement
```

This will:
1. Generate SP1 vkey from the program
2. Create `ExecutorLeaseAgreement` with vkey
3. Submit to Delta base layer via RPC

### Step 4: Run Executor with SP1 Proving

```bash
cargo run --bin convexfx-delta -- run --api-port 8080
```

The executor will now:
1. Process orders via SCP clearing
2. Validate with predicates (off-chain check)
3. Generate SP1 proof (cryptographic guarantee)
4. Submit state diffs + proof to Delta

## Performance Considerations

### Proof Generation Time

- **Small batches (1-5 orders):** ~1-2 seconds
- **Medium batches (10-20 orders):** ~3-5 seconds
- **Large batches (50+ orders):** ~10-15 seconds

### Optimization Strategies

1. **Batch Processing:** Group orders into epochs
2. **Parallel Proving:** Run multiple provers for high throughput
3. **Caching:** Cache vkey and prover setup
4. **Hardware:** Use machines with more CPU/RAM for faster proving

## Debugging

### Enable SP1 Logging

```bash
export RUST_LOG=sp1_zkvm=debug
cargo test -p convexfx-delta test_sp1_proof_generation -- --nocapture
```

### Common Issues

**Issue:** Proof generation fails with "assertion failed"
**Solution:** Check that off-chain predicate validation passes first

**Issue:** Proof generation is slow
**Solution:** Use `--release` mode and consider hardware acceleration

**Issue:** Verification key mismatch
**Solution:** Ensure ELF binary is up-to-date with latest SP1 build

## Future Enhancements

### Additional Local Laws

1. **Minimum Trade Size:** Prevent dust trades
2. **Maximum Slippage:** Protect users from excessive slippage
3. **Risk Parameter Compliance:** Enforce inventory limits
4. **Oracle Price Validity:** Validate oracle timestamps
5. **Arbitrage Prevention:** Detect and prevent obvious arbitrage

### Performance Improvements

1. **Recursive Proving:** Aggregate multiple proofs
2. **Hardware Acceleration:** GPU/FPGA acceleration
3. **Optimized Circuits:** Hand-optimize critical paths

## References

- [SP1 Documentation](https://docs.succinct.xyz/)
- [Delta SDK Documentation](https://docs.delta.xyz/)
- [ConvexFX SCP Algorithm](../../convexfx-clearing/README.md)
- [Predicate Implementation](./SCP_PREDICATE_IMPLEMENTATION.md)

