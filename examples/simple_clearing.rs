use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_oracle::{MockOracle, Oracle};
use convexfx_risk::RiskParams;
use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use std::collections::BTreeMap;

fn main() {
    println!("=== ConvexFX Simple Clearing Demo ===\n");

    // Setup: Create mock oracle with default FX prices
    let oracle = MockOracle::new();
    let ref_prices = oracle.current_prices().unwrap();

    println!("Reference prices (linear):");
    for asset in AssetId::all() {
        let y = ref_prices.get_ref(*asset);
        let p = y.exp();
        println!("  {}: {:.4}", asset, p);
    }
    println!();

    // Setup initial inventory: 10M units each
    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 10.0); // in millions
    }

    println!("Initial inventory (millions):");
    for (asset, amount) in &inventory {
        println!("  {}: {:.2}", asset, amount);
    }
    println!();

    // Create orders: 5 EUR buy orders
    let mut orders = Vec::new();
    for i in 0..5 {
        orders.push(PairOrder {
            id: format!("order_{}", i),
            trader: AccountId::new(format!("trader_{}", i)),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_f64(0.5).unwrap(), // 0.5M USD each
            limit_ratio: Some(1.15), // Max EURUSD = 1.15
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        });
    }

    println!("Orders to clear: {} EUR buy orders", orders.len());
    println!("  Total budget: {:.2}M USD", orders.len() as f64 * 0.5);
    println!();

    // Setup risk parameters
    let risk = RiskParams::default_demo();

    // Create epoch instance
    let instance = EpochInstance::new(1, inventory, orders, ref_prices, risk);

    // Run clearing
    println!("Running SCP clearing algorithm...\n");
    let clearing = ScpClearing::new(); // Uses Clarabel by default

    match clearing.clear_epoch(&instance) {
        Ok(solution) => {
            println!("✓ Clearing succeeded!\n");

            println!("Diagnostics:");
            println!("  Iterations: {}", solution.diagnostics.iterations);
            println!("  Converged: {}", solution.diagnostics.convergence_achieved);
            println!("  QP status: {}", solution.diagnostics.qp_status);
            println!();

            println!("Cleared prices (linear):");
            for asset in AssetId::all() {
                let price = solution.prices.get(asset).copied().unwrap_or(1.0);
                println!("  {}: {:.4}", asset, price);
            }
            println!();

            println!("Fills: {} orders filled", solution.fills.len());
            for fill in &solution.fills {
                println!(
                    "  {} - {:.2}% filled: pay {:.4}M {}, receive {:.4}M {}",
                    fill.order_id,
                    fill.fill_frac * 100.0,
                    fill.pay_units,
                    fill.pay_asset,
                    fill.recv_units,
                    fill.recv_asset
                );
            }
            println!();

            println!("Post-trade inventory (millions):");
            for (asset, amount) in &solution.q_post {
                println!("  {}: {:.2}", asset, amount);
            }
            println!();

            println!("Objective terms:");
            println!("  Inventory risk: {:.6}", solution.objective_terms.inventory_risk);
            println!("  Price tracking: {:.6}", solution.objective_terms.price_tracking);
            println!("  Fill incentive: {:.6}", solution.objective_terms.fill_incentive);
            println!("  Total objective: {:.6}", solution.objective_terms.total);
            println!();

            // Check no-arbitrage
            let y_eur = solution.y_star.get(&AssetId::EUR).copied().unwrap_or(0.0);
            let y_jpy = solution.y_star.get(&AssetId::JPY).copied().unwrap_or(0.0);
            let y_usd = solution.y_star.get(&AssetId::USD).copied().unwrap_or(0.0);

            let eurusd = (y_eur - y_usd).exp();
            let usdjpy = 1.0 / (y_jpy - y_usd).exp();
            let eurjpy_direct = 1.0 / (y_jpy - y_eur).exp();
            let eurjpy_cross = eurusd * usdjpy;

            println!("No-arbitrage check:");
            println!("  EURUSD: {:.4}", eurusd);
            println!("  USDJPY: {:.4}", usdjpy);
            println!("  EURJPY (direct): {:.4}", eurjpy_direct);
            println!("  EURJPY (via USD): {:.4}", eurjpy_cross);
            println!("  Arbitrage error: {:.8}", (eurjpy_direct - eurjpy_cross).abs());
            println!();

            println!("✓ Demo complete!");
        }
        Err(e) => {
            println!("✗ Clearing failed: {:?}", e);
        }
    }
}


