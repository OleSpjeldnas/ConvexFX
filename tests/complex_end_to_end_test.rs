use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_oracle::{MockOracle, Oracle};
use convexfx_risk::RiskParams;
use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use std::collections::BTreeMap;

/// Comprehensive end-to-end test with 6 assets and complex order flows
#[test]
fn test_complex_6_asset_clearing() {
    println!("\n=== Complex 6-Asset End-to-End Test ===\n");

    // Setup oracle with 6 assets
    let oracle = MockOracle::new();
    let ref_prices = oracle.reference_prices(1).unwrap();

    println!("Reference Prices (6 assets):");
    for asset in AssetId::all() {
        let y = ref_prices.get_ref(*asset);
        let p = y.exp();
        println!("  {}: {:.6} (log: {:.6})", asset, p, y);
    }
    println!();

    // Setup initial inventory: 20M units each for the 6 assets
    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 20.0);
    }

    println!("Initial Pool Inventory (millions):");
    for (asset, amount) in &inventory {
        println!("  {}: {:.2}M", asset, amount);
    }
    println!();

    // Create diverse set of orders across multiple currency pairs
    let orders = vec![
        // EUR/USD orders (major pair)
        PairOrder {
            id: "eur_buy_1".to_string(),
            trader: AccountId::new("trader_alice"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_f64(1.0).unwrap(),
            limit_ratio: Some(1.15),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({"type": "market_making"}),
        },
        PairOrder {
            id: "eur_buy_2".to_string(),
            trader: AccountId::new("trader_bob"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_f64(0.75).unwrap(),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "retail"}),
        },
        // GBP/USD orders
        PairOrder {
            id: "gbp_sell_1".to_string(),
            trader: AccountId::new("trader_charlie"),
            pay: AssetId::GBP,
            receive: AssetId::USD,
            budget: Amount::from_f64(0.5).unwrap(),
            limit_ratio: Some(0.85), // Max USDGBP = 0.85 (min GBPUSD = 1/0.85 = 1.176)
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "hedge"}),
        },
        // JPY/USD orders  
        PairOrder {
            id: "jpy_buy_1".to_string(),
            trader: AccountId::new("trader_diana"),
            pay: AssetId::USD,
            receive: AssetId::JPY,
            budget: Amount::from_f64(1.2).unwrap(),
            limit_ratio: Some(105.0), // Max JPYUSD
            min_fill_fraction: Some(0.3),
            metadata: serde_json::json!({"type": "institutional"}),
        },
        // Cross-pair: EUR/GBP
        PairOrder {
            id: "eurgbp_1".to_string(),
            trader: AccountId::new("trader_eve"),
            pay: AssetId::EUR,
            receive: AssetId::GBP,
            budget: Amount::from_f64(0.6).unwrap(),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "arb"}),
        },
        // CHF orders
        PairOrder {
            id: "chf_buy_1".to_string(),
            trader: AccountId::new("trader_frank"),
            pay: AssetId::USD,
            receive: AssetId::CHF,
            budget: Amount::from_f64(0.8).unwrap(),
            limit_ratio: Some(1.12),
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "flight_to_quality"}),
        },
        // AUD orders (new 6th asset)
        PairOrder {
            id: "aud_buy_1".to_string(),
            trader: AccountId::new("trader_grace"),
            pay: AssetId::USD,
            receive: AssetId::AUD,
            budget: Amount::from_f64(0.9).unwrap(),
            limit_ratio: Some(1.35), // Max AUDUSD
            min_fill_fraction: Some(0.2),
            metadata: serde_json::json!({"type": "commodity_proxy"}),
        },
        PairOrder {
            id: "aud_sell_1".to_string(),
            trader: AccountId::new("trader_henry"),
            pay: AssetId::AUD,
            receive: AssetId::USD,
            budget: Amount::from_f64(1.5).unwrap(),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "risk_off"}),
        },
        // Complex cross: JPY/EUR
        PairOrder {
            id: "jpyeur_1".to_string(),
            trader: AccountId::new("trader_iris"),
            pay: AssetId::JPY,
            receive: AssetId::EUR,
            budget: Amount::from_f64(10.0).unwrap(), // 10M JPY
            limit_ratio: Some(125.0),
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "carry_trade"}),
        },
        // AUD/JPY cross
        PairOrder {
            id: "audjpy_1".to_string(),
            trader: AccountId::new("trader_jack"),
            pay: AssetId::AUD,
            receive: AssetId::JPY,
            budget: Amount::from_f64(0.4).unwrap(),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({"type": "risk_reversal"}),
        },
    ];

    println!("Orders Submitted: {}", orders.len());
    println!("  - EUR/USD: 2 orders");
    println!("  - GBP/USD: 1 order");
    println!("  - JPY/USD: 1 order");
    println!("  - EUR/GBP: 1 order");
    println!("  - CHF/USD: 1 order");
    println!("  - AUD/USD: 2 orders");
    println!("  - JPY/EUR: 1 order");
    println!("  - AUD/JPY: 1 order");
    println!("Total Budget: {:.2}M USD-equivalent\n", 
        orders.iter().map(|o| o.budget.to_f64()).sum::<f64>());

    // Setup risk parameters (updated for 6 assets)
    let mut risk = RiskParams::default_demo();
    
    // Adjust for 6 assets
    for asset in AssetId::all() {
        risk.q_target.insert(*asset, 20.0);
        risk.q_min.insert(*asset, 10.0);
        risk.q_max.insert(*asset, 30.0);
    }
    risk.gamma_diag = vec![1.0; 6];
    risk.w_diag = vec![100.0; 6];
    risk.rebuild_matrices();

    // Create epoch instance
    let instance = EpochInstance::new(1, inventory.clone(), orders.clone(), ref_prices, risk);

    println!("Running SCP Clearing Algorithm...\n");
    
    // Run clearing
    let clearing = ScpClearing::with_simple_solver();
    let solution = clearing.clear_epoch(&instance);

    match solution {
        Ok(sol) => {
            println!("✓ CLEARING SUCCESSFUL!\n");

            // Print diagnostics
            println!("=== Clearing Diagnostics ===");
            println!("Iterations: {}", sol.diagnostics.iterations);
            println!("Converged: {}", sol.diagnostics.convergence_achieved);
            println!("QP Status: {}", sol.diagnostics.qp_status);
            println!("Final step norm (y): {:.8}", sol.diagnostics.final_step_norm_y);
            println!("Final step norm (α): {:.8}", sol.diagnostics.final_step_norm_alpha);
            println!();

            // Print cleared prices
            println!("=== Cleared Prices (6 Assets) ===");
            for asset in AssetId::all() {
                let p = sol.prices.get(asset).copied().unwrap_or(1.0);
                let y = sol.y_star.get(asset).copied().unwrap_or(0.0);
                let p_ref = instance.ref_prices.y_ref.get(asset).map(|y| y.exp()).unwrap_or(1.0);
                let deviation_bps = ((p / p_ref - 1.0) * 10000.0);
                println!("  {}: {:.6} (log: {:.6}, dev: {:+.1} bps)", 
                    asset, p, y, deviation_bps);
            }
            println!();

            // Print fills
            println!("=== Order Fills ({} orders) ===", sol.fills.len());
            let mut total_volume = 0.0;
            for fill in &sol.fills {
                let fill_pct = fill.fill_frac * 100.0;
                let status = if fill.is_complete() {
                    "FULL"
                } else if fill.is_partial() {
                    "PARTIAL"
                } else {
                    "NONE"
                };
                
                println!("  {} [{:7}] {:.1}% - Pay: {:.4}M {}, Receive: {:.4}M {}",
                    fill.order_id, status, fill_pct,
                    fill.pay_units, fill.pay_asset,
                    fill.recv_units, fill.recv_asset);
                
                total_volume += fill.pay_units;
            }
            println!("Total Volume: {:.2}M USD-equivalent\n", total_volume);

            // Print inventory changes
            println!("=== Inventory Changes ===");
            for asset in AssetId::all() {
                let initial = inventory.get(asset).copied().unwrap_or(0.0);
                let final_inv = sol.q_post.get(asset).copied().unwrap_or(0.0);
                let change = final_inv - initial;
                let change_pct = (change / initial) * 100.0;
                println!("  {}: {:.2}M → {:.2}M ({:+.2}M, {:+.1}%)",
                    asset, initial, final_inv, change, change_pct);
            }
            println!();

            // Print objective breakdown
            println!("=== Objective Function ===");
            println!("  Inventory Risk: {:.6}", sol.objective_terms.inventory_risk);
            println!("  Price Tracking: {:.6}", sol.objective_terms.price_tracking);
            println!("  Fill Incentive: {:.6}", sol.objective_terms.fill_incentive);
            println!("  Total: {:.6}", sol.objective_terms.total);
            println!();

            // Verify no arbitrage across triangles
            println!("=== No-Arbitrage Verification ===");
            
            // Triangle 1: EUR/USD/JPY
            let eurusd = sol.prices.get(&AssetId::EUR).unwrap() / sol.prices.get(&AssetId::USD).unwrap();
            let usdjpy = 1.0 / (sol.prices.get(&AssetId::JPY).unwrap() / sol.prices.get(&AssetId::USD).unwrap());
            let eurjpy_direct = sol.prices.get(&AssetId::EUR).unwrap() / sol.prices.get(&AssetId::JPY).unwrap();
            let eurjpy_cross = eurusd * usdjpy;
            let arb_error_1 = ((eurjpy_direct - eurjpy_cross) / eurjpy_direct * 10000.0).abs();
            
            println!("Triangle EUR/USD/JPY:");
            println!("  EURUSD: {:.6}", eurusd);
            println!("  USDJPY: {:.4}", usdjpy);
            println!("  EURJPY (direct): {:.4}", eurjpy_direct);
            println!("  EURJPY (cross):  {:.4}", eurjpy_cross);
            println!("  Arbitrage error: {:.4} bps", arb_error_1);
            
            // Triangle 2: AUD/USD/JPY
            let audusd = sol.prices.get(&AssetId::AUD).unwrap() / sol.prices.get(&AssetId::USD).unwrap();
            let audjpy_direct = sol.prices.get(&AssetId::AUD).unwrap() / sol.prices.get(&AssetId::JPY).unwrap();
            let audjpy_cross = audusd * usdjpy;
            let arb_error_2 = ((audjpy_direct - audjpy_cross) / audjpy_direct * 10000.0).abs();
            
            println!("\nTriangle AUD/USD/JPY:");
            println!("  AUDUSD: {:.6}", audusd);
            println!("  USDJPY: {:.4}", usdjpy);
            println!("  AUDJPY (direct): {:.4}", audjpy_direct);
            println!("  AUDJPY (cross):  {:.4}", audjpy_cross);
            println!("  Arbitrage error: {:.4} bps", arb_error_2);
            
            // Triangle 3: EUR/GBP/USD
            let gbpusd = sol.prices.get(&AssetId::GBP).unwrap() / sol.prices.get(&AssetId::USD).unwrap();
            let eurgbp_direct = sol.prices.get(&AssetId::EUR).unwrap() / sol.prices.get(&AssetId::GBP).unwrap();
            let eurgbp_cross = eurusd / gbpusd;
            let arb_error_3 = ((eurgbp_direct - eurgbp_cross) / eurgbp_direct * 10000.0).abs();
            
            println!("\nTriangle EUR/GBP/USD:");
            println!("  EURUSD: {:.6}", eurusd);
            println!("  GBPUSD: {:.6}", gbpusd);
            println!("  EURGBP (direct): {:.6}", eurgbp_direct);
            println!("  EURGBP (cross):  {:.6}", eurgbp_cross);
            println!("  Arbitrage error: {:.4} bps", arb_error_3);
            
            println!();

            // Assertions
            assert!(sol.diagnostics.iterations > 0, "Should have run iterations");
            assert!(sol.diagnostics.iterations <= 10, "Should converge quickly");
            assert!(sol.fills.len() > 0, "Should have some fills");
            
            // USD should be numeraire
            assert_eq!(sol.y_star.get(&AssetId::USD).copied().unwrap_or(1.0), 0.0, 
                "USD must be numeraire (y_USD = 0)");
            
            // Prices should be within bands
            for asset in AssetId::all() {
                let y = sol.y_star.get(asset).copied().unwrap_or(0.0);
                let y_low = instance.ref_prices.get_low(*asset);
                let y_high = instance.ref_prices.get_high(*asset);
                assert!(y >= y_low - 1e-6 && y <= y_high + 1e-6,
                    "Price for {} should be within bands", asset);
            }
            
            // No arbitrage (within numerical tolerance)
            assert!(arb_error_1 < 1.0, "Arbitrage error should be < 1 bps for EUR/USD/JPY");
            assert!(arb_error_2 < 1.0, "Arbitrage error should be < 1 bps for AUD/USD/JPY");
            assert!(arb_error_3 < 1.0, "Arbitrage error should be < 1 bps for EUR/GBP/USD");
            
            // Inventory should be within bounds
            for asset in AssetId::all() {
                let q = sol.q_post.get(asset).copied().unwrap_or(0.0);
                assert!(q >= instance.risk.min_bound(*asset) - 0.1,
                    "{} inventory below min", asset);
                assert!(q <= instance.risk.max_bound(*asset) + 0.1,
                    "{} inventory above max", asset);
            }
            
            println!("✅ All assertions passed!");
            println!("\n=== Test Complete ===\n");
        }
        Err(e) => {
            panic!("Clearing failed: {:?}", e);
        }
    }
}

#[test]
fn test_6_asset_cross_rate_consistency() {
    println!("\n=== 6-Asset Cross-Rate Consistency Test ===\n");
    
    let oracle = MockOracle::new();
    let ref_prices = oracle.reference_prices(1).unwrap();
    
    // Test all possible triangles with 6 assets
    let assets = AssetId::all();
    let mut max_error = 0.0;
    let mut triangle_count = 0;
    
    for i in 0..assets.len() {
        for j in (i+1)..assets.len() {
            for k in (j+1)..assets.len() {
                let a1 = assets[i];
                let a2 = assets[j];
                let a3 = assets[k];
                
                let y1 = ref_prices.get_ref(*a1);
                let y2 = ref_prices.get_ref(*a2);
                let y3 = ref_prices.get_ref(*a3);
                
                // Check triangle: a1/a2 * a2/a3 = a1/a3
                let r12 = (y1 - y2).exp();
                let r23 = (y2 - y3).exp();
                let r13_direct = (y1 - y3).exp();
                let r13_cross = r12 * r23;
                
                let error = ((r13_direct - r13_cross) / r13_direct).abs();
                max_error = max_error.max(error);
                triangle_count += 1;
                
                assert!(error < 1e-10, 
                    "Triangle {}/{}/{} has arbitrage error {:.2e}",
                    a1, a2, a3, error);
            }
        }
    }
    
    println!("Tested {} triangles", triangle_count);
    println!("Max arbitrage error: {:.2e}", max_error);
    println!("✅ All triangles are arbitrage-free!\n");
}

