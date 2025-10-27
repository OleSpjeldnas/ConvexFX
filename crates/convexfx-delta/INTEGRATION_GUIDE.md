# ConvexFX Delta Integration Guide

## Overview

This guide explains how to integrate ConvexFX with the Delta network as a full executor.

## Current Status

### ✅ What You Have

- **ConvexFX Clearing Engine**: Full SCP clearing algorithm implementation
- **SDL Generation**: Conversion of fills to Delta state diff format
- **Basic Types**: Mapping between ConvexFX and Delta types
- **Proof of Concept**: Demonstrates the integration pattern

### ❌ What's Missing for Production

To run ConvexFX as a real Delta executor, you need to:

1. **Implement the `Execution` Trait**
   - The `delta_executor_sdk::execution::Execution` trait defines how your custom logic executes messages
   - This trait has changed in Delta SDK v0.5.10+ and requires:
     - An associated `Error` type
     - Methods for applying verifiables and checking state
     - Different signature than older versions

2. **Use the Delta Runtime**
   - The `delta_executor_sdk::Runtime<E, P>` wraps your executor
   - Handles proving, SDL submission, and base layer sync automatically
   - Manages the HTTP API and client interactions

3. **Configure Domain Agreement**
   - Register your shard with the Delta base layer
   - Submit domain agreement with shard ID and fee
   - Wait for activation (after current epoch ends)

## Integration Approaches

### Approach 1: Full Custom Executor (Recommended for Production)

Implement the `Execution` trait to have full control over execution logic:

```rust
use delta_executor_sdk::execution::Execution;
use delta_base_sdk::verifiable::Verifiable;

pub struct ConvexFxExecutor {
    exchange: Exchange,
    // ... other state
}

impl Execution for ConvexFxExecutor {
    type Error = ConvexFxError;
    
    // Implement required methods based on SDK documentation
    // Note: API varies by SDK version
}

// Then use with Runtime:
type Runtime = delta_executor_sdk::Runtime<ConvexFxExecutor, ProvingClient>;
```

### Approach 2: Use FullDebitExecutor (Simpler, Less Control)

Use the built-in `FullDebitExecutor` and integrate ConvexFX as a pricing/clearing oracle:

```rust
use delta_executor_sdk::execution::FullDebitExecutor;
use delta_executor_sdk::proving::mock::Client;

type Runtime = delta_executor_sdk::Runtime<FullDebitExecutor, mock::Client>;

// Run standard executor, use ConvexFX for price discovery
// Submit clearing prices as oracle data
```

### Approach 3: Hybrid (Current Recommendation)

Given SDK version uncertainties:

1. **Run standard Delta executor** using the generic pattern you provided
2. **Run ConvexFX exchange separately** as a service
3. **Bridge them** via HTTP API or shared database
4. **Use ConvexFX** for:
   - Price discovery
   - Batch order matching
   - Risk management
5. **Use Delta executor** for:
   - Proving
   - Base layer settlement
   - Vault management

```text
┌─────────────────┐
│  Delta Executor │──┐
│  (Generic impl) │  │ Queries prices
└─────────────────┘  │ Submits orders
         │           │
         ├───────────┘
         │
         │ HTTP/gRPC
         │
         ▼
┌─────────────────┐
│ ConvexFX Engine │
│  - SCP Clearing │
│  - Price Oracle │
│  - Risk Mgmt    │
└─────────────────┘
```

## Implementation Checklist

### Phase 1: Verify SDK Compatibility

- [ ] Check Delta SDK documentation for your version (0.5.10)
- [ ] Review `Execution` trait requirements
- [ ] Test generic executor example runs successfully
- [ ] Understand proving options (mock vs. ZKP)

### Phase 2: Create Minimal Executor

- [ ] Create simple `Execution` impl that compiles
- [ ] Wire up to `Runtime<YourExecutor, mock::Client>`
- [ ] Test domain agreement submission
- [ ] Verify it runs and syncs with base layer

### Phase 3: Integrate ConvexFX Logic

- [ ] Convert Delta verifiables to ConvexFX orders
- [ ] Run SCP clearing on order batches
- [ ] Generate state diffs from fills
- [ ] Return results to Delta runtime

### Phase 4: Production Hardening

- [ ] Replace mock proving with real ZKP client
- [ ] Add persistent storage (RocksDB)
- [ ] Implement proper error handling
- [ ] Add monitoring and metrics
- [ ] Security audit

## SDK Version Issues

### Current Blockers

The Delta SDK v0.5.10 has API changes:

1. **`Execution` trait**:
   - Requires `type Error = ...;` associated type
   - Method signatures have changed
   - No `ExecutionError` type exported

2. **Module structure**:
   - `delta_base_sdk::verifiable` module doesn't exist
   - Verifiable types in different location

3. **Type changes**:
   - `EpochSolution` struct has different fields
   - No `fills` field (now has `y_star`, `prices`, `q_post`)

### Recommended Actions

1. **Check Delta Documentation**:
   ```bash
   cargo doc --package delta_executor_sdk --open
   cargo doc --package delta_base_sdk --open
   ```

2. **Review Examples**:
   - Look for updated examples in the Delta SDK repo
   - Check for migration guides for 0.5.x versions

3. **Contact Delta Team**:
   - Ask for integration guide for custom executors
   - Request example of `Execution` trait implementation
   - Clarify API changes in v0.5.10

4. **Consider Staying on Generic Executor**:
   - Use the provided generic executor as-is
   - Run ConvexFX as separate service
   - Integrate via API rather than trait implementation

## Alternative: Standalone Integration

If full executor integration is blocked by API issues, you can:

### Run ConvexFX as a Delta Client

Instead of being an executor, run ConvexFX as a sophisticated client:

```rust
// ConvexFX service that:
// 1. Collects orders from users
// 2. Runs SCP clearing
// 3. Generates signed verifiables
// 4. Submits to any Delta executor (including generic one)

// Users → ConvexFX → Signed Verifiables → Delta Executor → Base Layer
```

This approach:
- ✅ Doesn't require implementing `Execution` trait
- ✅ Can use any existing Delta executor
- ✅ ConvexFX provides advanced matching/clearing
- ✅ Delta provides settlement and proving
- ❌ Two separate services to run
- ❌ More network hops

## Code Structure for Full Integration

When you have the correct SDK API documentation:

```
crates/convexfx-delta/
├── src/
│   ├── lib.rs                 # Public API
│   ├── executor.rs            # Execution trait impl
│   ├── converter.rs           # Delta ↔ ConvexFX type conversion
│   ├── state_manager.rs       # Vault and account state
│   ├── clearing_service.rs    # ConvexFX clearing logic
│   └── main.rs                # Binary with CLI
├── executor.yaml.example      # Configuration template
└── README.md                  # User documentation
```

## Next Steps

1. **Immediate**: Test the generic executor runs on your system
2. **Short-term**: Run ConvexFX separately, integrate via API
3. **Medium-term**: Get clarification on Delta SDK 0.5.10 API
4. **Long-term**: Implement full `Execution` trait when API is clear

## Resources

- Delta SDK Docs: `cargo doc --package delta_executor_sdk --open`
- Generic Executor Example: (the code you provided)
- ConvexFX Clearing: `crates/convexfx-clearing/`
- Integration Tests: `crates/convexfx-delta/tests/`

## Questions to Answer

Before proceeding with full integration:

1. What Delta SDK version is officially supported/documented?
2. Is there an example of a custom `Execution` implementation for v0.5.10+?
3. What is the correct way to return state diffs in the new API?
4. How should proving client be configured for production?
5. What are the gas/fee requirements for domain agreements?

---

**Note**: The executor.rs file I created is a starting point but won't compile against SDK v0.5.10 due to API changes. You'll need to adapt it once you have the correct API documentation.

