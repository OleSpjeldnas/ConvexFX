use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AssetId, Amount, PairOrder};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("=== CLEARING ALGORITHM DIAGNOSTIC ===\n");
    
    // Create simple test case
    let orders = vec![
        PairOrder {
            id: "order1".to_string(),
            trader: "alice".to_string().into(),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
    ];
    
    // Create reference prices
    let mut y_ref = BTreeMap::new();
    y_ref.insert(AssetId::USD, 0.0);
    y_ref.insert(AssetId::EUR, -(0.86_f64).ln());
    y_ref.insert(AssetId::GBP, -(0.77_f64).ln());
    y_ref.insert(AssetId::JPY, -(149.0_f64).ln());
    y_ref.insert(AssetId::CHF, -(0.88_f64).ln());
    y_ref.insert(AssetId::AUD, -(1.50_f64).ln());
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let ref_prices = RefPrices::new(y_ref, 20.0, timestamp, vec!["test".to_string()]);
    
    // Create inventory
    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 100000.0);
    }
    
    let risk_params = RiskParams::default_demo();
    
    let instance = EpochInstance::new(1, inventory, orders, ref_prices, risk_params);
    
    // Try with simple solver first
    println!("Testing with SimpleQpSolver:");
    let clearing_simple = ScpClearing::with_simple_solver();
    match clearing_simple.clear_epoch(&instance) {
        Ok(solution) => {
            println!("✅ Simple solver succeeded!");
            println!("   Iterations: {}", solution.diagnostics.iterations);
            println!("   Converged: {}", solution.diagnostics.convergence_achieved);
            println!("   Final step norm y: {}", solution.diagnostics.final_step_norm_y);
            println!("   Final step norm alpha: {}", solution.diagnostics.final_step_norm_alpha);
            println!("   Fills: {}", solution.fills.len());
            
            if !solution.fills.is_empty() {
                let fill = &solution.fills[0];
                println!("\n   Fill details:");
                println!("     Order ID: {}", fill.order_id);
                println!("     Fill fraction: {}", fill.fill_frac);
                println!("     Pay: {} {}", fill.pay_units, fill.pay_asset);
                println!("     Receive: {} {}", fill.recv_units, fill.recv_asset);
            }
            
            println!("\n   Prices (log-space):");
            for (asset, y) in &solution.y_star {
                println!("     {:?}: y = {:.6}, p = {:.6}", asset, y, y.exp());
            }
        }
        Err(e) => {
            println!("❌ Simple solver failed: {:?}", e);
        }
    }
    
    println!("\n\nTesting with OSQP solver:");
    let clearing_osqp = ScpClearing::with_osqp_solver();
    match clearing_osqp.clear_epoch(&instance) {
        Ok(solution) => {
            println!("✅ OSQP solver succeeded!");
            println!("   Iterations: {}", solution.diagnostics.iterations);
            println!("   Converged: {}", solution.diagnostics.convergence_achieved);
            println!("   Final step norm y: {}", solution.diagnostics.final_step_norm_y);
            println!("   Final step norm alpha: {}", solution.diagnostics.final_step_norm_alpha);
            println!("   Fills: {}", solution.fills.len());
        }
        Err(e) => {
            println!("❌ OSQP solver failed: {:?}", e);
        }
    }
}

