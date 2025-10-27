# ConvexFX Sim

Simulation framework for testing and analysis.

## Overview

Provides tools for generating synthetic trading scenarios, running simulations, and analyzing exchange behavior under various conditions.

## Key Components

### Scenario Generator

```rust
pub struct ScenarioGenerator {
    num_traders: usize,
    num_epochs: usize,
    order_rate: f64,
}

impl ScenarioGenerator {
    pub fn generate_random_orders(&self) -> Vec<PairOrder>;
    pub fn generate_informed_traders(&self) -> Vec<PairOrder>;
    pub fn generate_market_stress(&self) -> Vec<PairOrder>;
}
```

### Simulation Runner

```rust
pub struct SimulationRunner {
    exchange: Exchange,
    scenarios: Vec<Scenario>,
}

impl SimulationRunner {
    pub fn run(&mut self) -> SimulationResults;
    pub fn run_parallel(&mut self) -> SimulationResults;
}
```

### KPI Tracker

```rust
pub struct KpiTracker {
    fills: Vec<Fill>,
    reports: Vec<EpochReport>,
}

impl KpiTracker {
    pub fn fill_rate(&self) -> f64;
    pub fn average_slippage(&self) -> f64;
    pub fn price_impact(&self) -> BTreeMap<AssetId, f64>;
    pub fn inventory_drift(&self) -> f64;
}
```

## Usage

### Basic Simulation

```rust
use convexfx_sim::{ScenarioGenerator, SimulationRunner, KpiTracker};

// Generate scenario
let gen = ScenarioGenerator {
    num_traders: 100,
    num_epochs: 1000,
    order_rate: 0.5,  // 50 orders/epoch on average
};

let orders = gen.generate_random_orders();

// Run simulation
let mut runner = SimulationRunner::new(exchange);
let results = runner.run(orders)?;

// Analyze
let kpi = KpiTracker::new(results);
println!("Fill rate: {:.2}%", kpi.fill_rate() * 100.0);
println!("Avg slippage: {:.4}%", kpi.average_slippage() * 100.0);
```

### Stress Testing

```rust
let gen = ScenarioGenerator::default();

// One-sided flow stress
let buy_pressure = gen.generate_buy_pressure(AssetId::EUR, 1000);
let results = runner.run(buy_pressure)?;

// High volatility stress
let volatile = gen.generate_volatile_market(0.05);  // 5% swings
let results = runner.run(volatile)?;

// Flash crash simulation
let crash = gen.generate_flash_crash(AssetId::JPY);
let results = runner.run(crash)?;
```

### Parameter Sensitivity

```rust
// Test different risk parameters
for gamma in [0.0001, 0.001, 0.01, 0.1] {
    let risk = RiskParams { gamma, ..default() };
    let exchange = Exchange::new_with_risk(risk);
    
    let results = runner.run(scenario.clone())?;
    println!("γ={}: fill_rate={:.2}%, slippage={:.4}%",
        gamma,
        results.fill_rate() * 100.0,
        results.avg_slippage() * 100.0
    );
}
```

## Scenario Types

### Random Orders

```rust
gen.generate_random_orders()
// - Uniform asset selection
// - Log-normal amounts
// - Random traders
```

### Informed Traders

```rust
gen.generate_informed_traders()
// - Price-aware orders
// - Limit prices near current market
// - Size based on spread
```

### Market Making

```rust
gen.generate_market_makers(5)
// - Two-sided quotes
// - Inventory management
// - Spread maintenance
```

### Arbitrage

```rust
gen.generate_arbitrageurs()
// - Multi-leg trades
// - Exploit price discrepancies
// - Triangle arbitrage attempts
```

## Analysis Tools

### Fill Rate Analysis

```rust
let analysis = kpi.fill_rate_by_size();
println!("Small orders (<$1k): {:.2}%", analysis.small);
println!("Medium orders ($1k-$10k): {:.2}%", analysis.medium);
println!("Large orders (>$10k): {:.2}%", analysis.large);
```

### Slippage Distribution

```rust
let slippage_dist = kpi.slippage_histogram();
for (bucket, count) in slippage_dist {
    println!("{:.2}%-{:.2}%: {} orders", 
        bucket.low * 100.0, 
        bucket.high * 100.0, 
        count
    );
}
```

### Price Impact

```rust
let impact = kpi.price_impact_curve();
for (trade_size, impact_bps) in impact {
    println!("${}: {:.2} bps", trade_size, impact_bps);
}
```

## Testbed

Pre-configured test environments:

```rust
use convexfx_sim::testbed;

// Small scale (quick tests)
let testbed = testbed::small_scale();  // 10 traders, 100 epochs

// Medium scale (realistic)
let testbed = testbed::medium_scale();  // 100 traders, 1000 epochs

// Large scale (stress test)
let testbed = testbed::large_scale();  // 1000 traders, 10000 epochs
```

## Benchmarking

```rust
use std::time::Instant;

let start = Instant::now();
let results = runner.run(orders)?;
let duration = start.elapsed();

println!("Throughput: {:.0} orders/sec", 
    results.total_orders as f64 / duration.as_secs_f64()
);
```

## Visualization

Export results for plotting:

```rust
// CSV export
kpi.export_csv("results.csv")?;

// JSON export
kpi.export_json("results.json")?;

// Plot with external tool
// $ python plot.py results.csv
```

## Testing

```bash
cargo test -p convexfx-sim
```

Tests include:
- Scenario generation
- Simulation runs
- KPI calculations
- Result reproducibility

## Dependencies

- `convexfx-exchange`: Exchange simulation
- `rand`: Random scenario generation
- `csv`: Data export
- `serde`: Serialization

