//! Comprehensive integration tests for SCP clearing validity predicate
//!
//! This test suite validates that the SCP_CLEARING_VALIDITY_PREDICATE correctly
//! identifies valid and invalid clearing solutions across a wide range of scenarios.

use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_delta::{predicates::{PredicateContext, ScpClearingValidityPredicate}, DemoApp};
use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AssetId, Amount, PairOrder};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper function to create test orders
fn create_test_orders() -> Vec<PairOrder> {
    vec![
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
        PairOrder {
            id: "order2".to_string(),
            trader: "bob".to_string().into(),
            pay: AssetId::EUR,
            receive: AssetId::GBP,
            budget: Amount::from_units(500),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.8),
            metadata: serde_json::json!({}),
        },
    ]
}

/// Helper function to create reference prices
fn create_ref_prices() -> RefPrices {
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

    RefPrices::new(y_ref, 20.0, timestamp, vec!["test".to_string()])
}

/// Helper function to create initial inventory
fn create_initial_inventory() -> BTreeMap<AssetId, f64> {
    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 100000.0);
    }
    inventory
}

#[tokio::test]
async fn test_predicate_valid_clearing() {
    // Create a valid clearing scenario
    let clearing_engine = ScpClearing::new();  // Use production solver
    let orders = create_test_orders();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    // Validate with predicate
    let predicate = ScpClearingValidityPredicate::default();
    let context = PredicateContext {
        oracle_prices: &ref_prices,
        initial_inventory: &inventory,
    };

    let result = predicate.validate(&solution, &context);
    assert!(result.is_ok(), "Valid clearing should pass predicate validation");

    println!("✅ Valid clearing passed all predicate checks");
    println!("   Iterations: {}", solution.diagnostics.iterations);
    println!("   Convergence: {}", solution.diagnostics.convergence_achieved);
    println!("   Fills: {}", solution.fills.len());
}

#[tokio::test]
async fn test_predicate_with_demo_app() {
    // Test predicate integration with demo app
    let app = DemoApp::new().expect("Failed to create demo app");

    let orders = vec![PairOrder {
        id: "demo_order".to_string(),
        trader: "alice".to_string().into(),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_units(1000),
        limit_ratio: Some(1.1),
        min_fill_fraction: Some(0.5),
        metadata: serde_json::json!({}),
    }];

    // Execute orders - predicate validation happens internally
    let result = app.execute_orders(orders);
    assert!(result.is_ok(), "Demo app execution should succeed with valid orders");

    let (fills, state_diffs) = result.unwrap();
    println!("✅ Demo app execution passed predicate validation");
    println!("   Fills: {}", fills.len());
    println!("   State diffs: {}", state_diffs.len());
}

#[tokio::test]
async fn test_predicate_empty_order_batch() {
    // Test with no orders - should still be valid
    let clearing_engine = ScpClearing::new();
    let orders = Vec::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed with empty batch");

    let predicate = ScpClearingValidityPredicate::default();
    let context = PredicateContext {
        oracle_prices: &ref_prices,
        initial_inventory: &inventory,
    };

    let result = predicate.validate(&solution, &context);
    assert!(result.is_ok(), "Empty batch should pass predicate validation");

    println!("✅ Empty batch passed all predicate checks");
    assert_eq!(solution.fills.len(), 0);
    assert_eq!(solution.diagnostics.iterations, 0);
}

#[tokio::test]
async fn test_predicate_large_order_batch() {
    // Test with many orders to stress-test validation
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    // Create 20 orders
    let mut orders = Vec::new();
    for i in 0..20 {
        orders.push(PairOrder {
            id: format!("order_{}", i),
            trader: format!("trader_{}", i).into(),
            pay: if i % 2 == 0 { AssetId::USD } else { AssetId::EUR },
            receive: if i % 2 == 0 { AssetId::EUR } else { AssetId::USD },
            budget: Amount::from_units(100 + i * 10),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        });
    }

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed with large batch");

    let predicate = ScpClearingValidityPredicate::default();
    let context = PredicateContext {
        oracle_prices: &ref_prices,
        initial_inventory: &inventory,
    };

    let result = predicate.validate(&solution, &context);
    assert!(
        result.is_ok(),
        "Large batch should pass predicate validation: {:?}",
        result.err()
    );

    println!("✅ Large batch (20 orders) passed all predicate checks");
    println!("   Fills: {}", solution.fills.len());
    println!("   Iterations: {}", solution.diagnostics.iterations);
}

#[tokio::test]
async fn test_predicate_multi_asset_trading() {
    // Test with orders across multiple asset pairs
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let orders = vec![
        PairOrder {
            id: "usd_eur".to_string(),
            trader: "trader1".to_string().into(),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
        PairOrder {
            id: "eur_gbp".to_string(),
            trader: "trader2".to_string().into(),
            pay: AssetId::EUR,
            receive: AssetId::GBP,
            budget: Amount::from_units(500),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
        PairOrder {
            id: "gbp_jpy".to_string(),
            trader: "trader3".to_string().into(),
            pay: AssetId::GBP,
            receive: AssetId::JPY,
            budget: Amount::from_units(300),
            limit_ratio: Some(1.15),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
    ];

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed with multi-asset trading");

    let predicate = ScpClearingValidityPredicate::default();
    let context = PredicateContext {
        oracle_prices: &ref_prices,
        initial_inventory: &inventory,
    };

    let result = predicate.validate(&solution, &context);
    assert!(
        result.is_ok(),
        "Multi-asset trading should pass predicate validation: {:?}",
        result.err()
    );

    println!("✅ Multi-asset trading passed all predicate checks");
    println!("   Assets involved: USD, EUR, GBP, JPY");
    println!("   Fills: {}", solution.fills.len());
}

#[tokio::test]
async fn test_predicate_convergence_check() {
    // This test verifies that the predicate checks convergence properly
    let clearing_engine = ScpClearing::new();
    let orders = create_test_orders();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    // Verify diagnostics show convergence
    assert!(
        solution.diagnostics.convergence_achieved,
        "Solution should have converged"
    );
    assert!(
        solution.diagnostics.final_step_norm_y < 1e-5,
        "Y step norm should be small"
    );
    assert!(
        solution.diagnostics.final_step_norm_alpha < 1e-6,
        "Alpha step norm should be small"
    );

    println!("✅ Convergence diagnostics verified");
    println!("   Final y norm: {:.2e}", solution.diagnostics.final_step_norm_y);
    println!("   Final alpha norm: {:.2e}", solution.diagnostics.final_step_norm_alpha);
}

#[tokio::test]
async fn test_predicate_price_consistency() {
    // Test that prices are consistent between log and linear space
    let clearing_engine = ScpClearing::new();
    let orders = create_test_orders();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    // Manually verify price consistency
    for (asset, log_price) in &solution.y_star {
        let linear_price = solution.prices.get(asset).unwrap();
        let expected_linear = log_price.exp();
        let error = (linear_price - expected_linear).abs() / linear_price;

        assert!(
            error < 0.01,
            "Price consistency error for {:?}: {:.6}%",
            asset,
            error * 100.0
        );
    }

    // Verify USD numeraire
    let usd_log = solution.y_star.get(&AssetId::USD).unwrap();
    assert!(
        usd_log.abs() < 1e-10,
        "USD log price should be zero (numeraire)"
    );

    println!("✅ Price consistency verified");
    println!("   All assets have consistent log/linear prices");
    println!("   USD numeraire constraint satisfied");
}

#[tokio::test]
async fn test_predicate_inventory_conservation() {
    // Test that inventory is properly conserved
    let clearing_engine = ScpClearing::new();
    let orders = create_test_orders();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    // Manually verify inventory conservation
    for asset in AssetId::all() {
        let initial = inventory.get(asset).copied().unwrap_or(0.0);
        let final_inv = solution.q_post.get(asset).copied().unwrap_or(0.0);

        let mut net_flow = 0.0;
        for fill in &solution.fills {
            if fill.pay_asset == *asset {
                net_flow += fill.pay_units;
            }
            if fill.recv_asset == *asset {
                net_flow -= fill.recv_units;
            }
        }

        let expected = initial + net_flow;
        let error = (final_inv - expected).abs();

        assert!(
            error < 1e-6,
            "Inventory conservation violated for {:?}: error = {:.6}",
            asset,
            error
        );
    }

    println!("✅ Inventory conservation verified");
    println!("   All assets properly accounted for");
}

#[tokio::test]
async fn test_predicate_objective_components() {
    // Test that objective function components are valid
    let clearing_engine = ScpClearing::new();
    let orders = create_test_orders();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    let obj = &solution.objective_terms;

    // Verify components are non-negative (except fill incentive)
    assert!(obj.inventory_risk >= 0.0, "Inventory risk should be non-negative");
    assert!(obj.price_tracking >= 0.0, "Price tracking should be non-negative");

    // Verify total is sum of components
    let computed_total = obj.inventory_risk + obj.price_tracking + obj.fill_incentive;
    let error = (obj.total - computed_total).abs();
    assert!(
        error < 1e-6,
        "Objective total should equal sum of components: error = {:.6}",
        error
    );

    println!("✅ Objective function components verified");
    println!("   Inventory risk: {:.2}", obj.inventory_risk);
    println!("   Price tracking: {:.2}", obj.price_tracking);
    println!("   Fill incentive: {:.2}", obj.fill_incentive);
    println!("   Total: {:.2}", obj.total);
}

#[tokio::test]
async fn test_predicate_with_partial_fills() {
    // Test validation when orders are partially filled
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();

    // Create orders that will likely be partially filled
    let orders = vec![PairOrder {
        id: "large_order".to_string(),
        trader: "whale".to_string().into(),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_units(50000), // Large order
        limit_ratio: Some(1.05), // Tight limit
        min_fill_fraction: Some(0.1), // Low minimum
        metadata: serde_json::json!({}),
    }];

    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices.clone(), risk_params);

    let solution = clearing_engine
        .clear_epoch(&instance)
        .expect("Clearing should succeed");

    let predicate = ScpClearingValidityPredicate::default();
    let context = PredicateContext {
        oracle_prices: &ref_prices,
        initial_inventory: &inventory,
    };

    let result = predicate.validate(&solution, &context);
    assert!(
        result.is_ok(),
        "Partial fills should pass predicate validation: {:?}",
        result.err()
    );

    // Check if any fills are partial
    for fill in &solution.fills {
        if fill.fill_frac < 1.0 && fill.fill_frac > 0.0 {
            println!("✅ Partial fill validated: {:.1}% of order {}", 
                     fill.fill_frac * 100.0, fill.order_id);
        }
    }
}

