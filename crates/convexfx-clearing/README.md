# ConvexFX Clearing

Sequential Convex Programming (SCP) clearing engine for batch order execution.

## Overview

The clearing crate implements the core ConvexFX algorithm that takes a batch of orders and solves a convex optimization problem to find optimal clearing prices and fills.

## Key Concepts

### Epoch-Based Clearing

Orders are collected into epochs (e.g., every 200ms) and cleared together:

1. **Collect**: Gather all orders submitted during the epoch
2. **Optimize**: Solve convex program to find optimal prices
3. **Execute**: Fill orders at the computed clearing prices
4. **Report**: Generate execution report with fills and prices

### SCP Algorithm

The Sequential Convex Programming approach:

```
while not converged:
    1. Linearize constraints around current point
    2. Solve Quadratic Program (QP)
    3. Update prices
    4. Check convergence
```

## Main Components

### ScpClearing

```rust
pub struct ScpClearing {
    solver: Box<dyn Solver>,
    // Configuration...
}

impl ScpClearing {
    pub fn clear_epoch(&self, instance: &EpochInstance) 
        -> Result<EpochSolution, ClearingError>
}
```

The main clearing engine that orchestrates the SCP algorithm.

### EpochInstance

```rust
pub struct EpochInstance {
    pub epoch_id: EpochId,
    pub inventory_q: BTreeMap<AssetId, f64>,
    pub orders: Vec<PairOrder>,
    pub ref_prices: RefPrices,
    pub risk: RiskParams,
}
```

Input data for an epoch clearing:
- Current inventory across all assets
- Submitted orders
- Reference prices from oracle
- Risk parameters (Γ, W matrices)

### EpochSolution

```rust
pub struct EpochSolution {
    pub epoch_id: EpochId,
    pub fills: Vec<Fill>,
    pub clearing_prices: Prices,
    pub new_inventory: BTreeMap<AssetId, f64>,
    pub objective_value: f64,
}
```

Output from clearing:
- Executed fills
- Clearing prices for all assets
- Updated inventory
- Objective function value

## Features

- **Optimal Execution**: Finds prices that maximize trader satisfaction while managing risk
- **Inventory Management**: Tracks and manages exchange inventory with risk-aware pricing
- **Price Discovery**: Automatic price computation based on supply/demand
- **Coherent Pricing**: Guarantees no triangular arbitrage opportunities
- **Partial Fills**: Supports partial order fills when full execution isn't feasible

## Usage

```rust
use convexfx_clearing::{ScpClearing, EpochInstance};
use convexfx_solver::SimpleBackend;

// Create clearing engine with simple solver
let clearing = ScpClearing::with_simple_solver();

// Set up epoch data
let instance = EpochInstance {
    epoch_id: 1,
    inventory_q: initial_inventory(),
    orders: vec![order1, order2, order3],
    ref_prices: oracle.get_prices(),
    risk: risk_params,
};

// Clear the epoch
let solution = clearing.clear_epoch(&instance)?;

// Process fills
for fill in solution.fills {
    ledger.execute_fill(&fill)?;
}
```

## Configuration

The SCP algorithm can be tuned via parameters:

```rust
let clearing = ScpClearing {
    max_iterations: 50,
    tolerance: 1e-6,
    solver: Box::new(SimpleBackend::new()),
};
```

## Objective Function

The clearing engine minimizes:

```
minimize: 
    Σ (deviation from reference prices)²  // Price tracking
  + Σ (unfilled order penalty)           // Fill incentive
  + inventory' * Γ * inventory           // Inventory risk
```

Subject to:
- Inventory balance constraints
- Order limit prices
- Minimum fill requirements
- Non-negativity constraints

## Testing

```bash
cargo test -p convexfx-clearing
```

Includes tests for:
- Single pair clearing
- Multi-asset clearing
- Partial fills
- Price discovery
- Inventory management

## Performance

- **Speed**: Clears 100-500 orders in < 100ms
- **Scalability**: Handles 6+ assets efficiently
- **Convergence**: Typically converges in 10-30 iterations

## Dependencies

- `convexfx-types`: Core types
- `convexfx-solver`: QP solver backend
- `convexfx-risk`: Risk parameters
- `convexfx-oracle`: Price references

