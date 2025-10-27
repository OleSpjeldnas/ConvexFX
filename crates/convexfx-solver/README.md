# ConvexFX Solver

Quadratic Programming (QP) solver backend for the SCP clearing algorithm.

## Overview

Provides solver implementations for the convex optimization problems in ConvexFX clearing. The solver takes a QP model and returns optimal decision variables.

## Solver Trait

```rust
pub trait Solver: Send + Sync {
    fn solve(&self, model: &QpModel) -> Result<Solution, SolverError>;
}
```

## Implementations

### SimpleBackend

Basic gradient descent solver for prototyping:

```rust
let solver = SimpleBackend::new();
let solution = solver.solve(&qp_model)?;
```

- **Pros**: No external dependencies, easy to understand
- **Cons**: Slower convergence, less robust for large problems
- **Use case**: Development, testing, small-scale demos

### OSQPBackend (Optional)

High-performance solver using OSQP library:

```rust
let solver = OSQPBackend::new();
let solution = solver.solve(&qp_model)?;
```

- **Pros**: Fast, production-ready, handles large problems
- **Cons**: External C dependency
- **Use case**: Production deployments

## QP Model

The QP model represents:

```
minimize: 0.5 * x' * P * x + q' * x
subject to: A * x = b    (equality constraints)
            l ≤ x ≤ u    (bounds)
```

Where:
- `x`: Decision variables (prices, flows)
- `P`: Quadratic cost matrix
- `q`: Linear cost vector
- `A`, `b`: Equality constraints
- `l`, `u`: Variable bounds

## Usage

```rust
use convexfx_solver::{QpModel, SimpleBackend, Solver};

// Build QP model
let model = QpModel {
    n_vars: 10,
    P: sparse_matrix_P,
    q: cost_vector,
    A: constraint_matrix,
    b: constraint_rhs,
    l: lower_bounds,
    u: upper_bounds,
};

// Solve
let solver = SimpleBackend::new();
let solution = solver.solve(&model)?;

// Extract results
let optimal_x = solution.x;
let objective_value = solution.obj_val;
```

## Configuration

### SimpleBackend

```rust
SimpleBackend {
    max_iterations: 1000,
    tolerance: 1e-6,
    step_size: 0.01,
}
```

### OSQPBackend

```rust
OSQPBackend {
    eps_abs: 1e-5,
    eps_rel: 1e-5,
    max_iter: 10000,
    verbose: false,
}
```

## Testing

```bash
cargo test -p convexfx-solver
```

## Performance Comparison

For a typical 6-asset, 100-order clearing problem:

| Solver | Time | Iterations |
|--------|------|------------|
| SimpleBackend | ~50ms | 200-500 |
| OSQPBackend | ~5ms | 10-30 |

## Dependencies

- Core: `nalgebra`, `sprs` (sparse matrices)
- Optional: `osqp` (for OSQPBackend)

