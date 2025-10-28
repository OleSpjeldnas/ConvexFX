//! Comprehensive SP1 integration tests for ConvexFX local laws
//!
//! This test suite validates the SP1 zkVM integration, including proof generation,
//! verification key extraction, and end-to-end proving flow.

use convexfx_clearing::{Diagnostics, EpochInstance, ObjectiveTerms, ScpClearing};
use convexfx_delta::sp1_prover::{ConvexFxSp1Prover, ClearingProofInput};
use convexfx_delta::DemoApp;
use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AssetId, Amount, PairOrder};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper to create reference prices
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

/// Helper to create initial inventory
fn create_initial_inventory() -> BTreeMap<AssetId, f64> {
    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 100000.0);
    }
    inventory
}

#[test]
fn test_sp1_prover_creation() {
    let prover = ConvexFxSp1Prover::new();
    assert_eq!(prover.get_vkey().len(), 32);
    println!("✅ SP1 prover created successfully");
}

#[test]
fn test_sp1_vkey_deterministic() {
    let prover1 = ConvexFxSp1Prover::new();
    let prover2 = ConvexFxSp1Prover::new();
    
    let vkey1 = prover1.get_vkey();
    let vkey2 = prover2.get_vkey();
    
    // Verification keys should be deterministic
    assert_eq!(vkey1, vkey2);
    println!("✅ SP1 vkeys are deterministic");
}

#[tokio::test]
async fn test_sp1_proof_generation_valid_clearing() {
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();
    
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
    
    let instance = EpochInstance::new(
        1,
        inventory.clone(),
        orders,
        ref_prices,
        risk_params,
    );
    
    let solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    // Generate SP1 proof
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_ok(), "Proof generation should succeed");
    let proof = result.unwrap();
    assert!(!proof.is_empty(), "Proof should not be empty");
    
    println!("✅ SP1 proof generated for valid clearing");
    println!("   Proof size: {} bytes", proof.len());
}

#[tokio::test]
async fn test_sp1_proof_reject_non_convergent() {
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();
    
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
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let mut solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    // Manually set convergence to false to simulate failure
    solution.diagnostics.convergence_achieved = false;
    
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_err(), "Should reject non-convergent solution");
    assert!(result.unwrap_err().to_string().contains("did not converge"));
    
    println!("✅ SP1 proof correctly rejects non-convergent solution");
}

#[tokio::test]
async fn test_sp1_proof_reject_high_step_norm() {
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();
    
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
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let mut solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    // Set step norm too high
    solution.diagnostics.final_step_norm_y = 1.0;  // Way above tolerance
    
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_err(), "Should reject high step norm");
    assert!(result.unwrap_err().to_string().contains("exceeds tolerance"));
    
    println!("✅ SP1 proof correctly rejects high step norm");
}

#[tokio::test]
async fn test_sp1_with_demo_app() {
    let app = DemoApp::new().expect("Failed to create demo app");
    
    let orders = vec![PairOrder {
        id: "sp1_test".to_string(),
        trader: "alice".to_string().into(),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_units(1000),
        limit_ratio: Some(1.1),
        min_fill_fraction: Some(0.5),
        metadata: serde_json::json!({}),
    }];
    
    // Execute orders - this will generate SP1 proof internally
    let result = app.execute_orders(orders);
    
    assert!(result.is_ok(), "Demo app execution should succeed with SP1 proving");
    let (fills, state_diffs) = result.unwrap();
    
    assert!(!fills.is_empty(), "Should have fills");
    assert!(!state_diffs.is_empty(), "Should have state diffs");
    
    println!("✅ SP1 proving integrated successfully with demo app");
    println!("   Fills: {}", fills.len());
    println!("   State diffs: {}", state_diffs.len());
}

#[tokio::test]
async fn test_sp1_proof_empty_batch() {
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();
    
    // Empty order batch
    let orders = Vec::new();
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_ok(), "Empty batch should still be provable");
    
    println!("✅ SP1 proof handles empty batch correctly");
}

#[tokio::test]
async fn test_sp1_proof_large_batch() {
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
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_ok(), "Large batch should be provable");
    
    println!("✅ SP1 proof handles large batch (20 orders)");
    println!("   Fills: {}", solution.fills.len());
}

#[tokio::test]
async fn test_sp1_proof_multi_asset() {
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
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    let prover = ConvexFxSp1Prover::new();
    let result = prover.prove_clearing(&solution, &inventory);
    
    assert!(result.is_ok(), "Multi-asset trading should be provable");
    
    println!("✅ SP1 proof handles multi-asset trading");
    println!("   Assets involved: USD, EUR, GBP, JPY");
}

#[test]
fn test_clearing_proof_input_serialization() {
    // Test that ClearingProofInput can be serialized/deserialized
    let input = ClearingProofInput {
        y_star: vec![(0, 0.0), (1, 0.1)],
        prices: vec![(0, 1.0), (1, 1.105)],
        fills: vec![],
        initial_inventory: vec![(0, 10000.0), (1, 10000.0)],
        final_inventory: vec![(0, 10000.0), (1, 10000.0)],
        convergence_achieved: true,
        final_step_norm_y: 1e-6,
        final_step_norm_alpha: 1e-7,
        inventory_risk: 100.0,
        price_tracking: 50.0,
        fill_incentive: -20.0,
        total_objective: 130.0,
    };
    
    let serialized = serde_json::to_string(&input).unwrap();
    let deserialized: ClearingProofInput = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.y_star.len(), 2);
    assert_eq!(deserialized.convergence_achieved, true);
    
    println!("✅ ClearingProofInput serialization works correctly");
}

#[tokio::test]
async fn test_sp1_proof_determinism() {
    // Same input should produce same proof (with deterministic prover)
    let clearing_engine = ScpClearing::new();
    let ref_prices = create_ref_prices();
    let inventory = create_initial_inventory();
    let risk_params = RiskParams::default_demo();
    
    let orders = vec![PairOrder {
        id: "order1".to_string(),
        trader: "alice".to_string().into(),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_units(1000),
        limit_ratio: Some(1.1),
        min_fill_fraction: Some(0.5),
        metadata: serde_json::json!({}),
    }];
    
    let instance = EpochInstance::new(1, inventory.clone(), orders, ref_prices, risk_params);
    let solution = clearing_engine.clear_epoch(&instance).unwrap();
    
    let prover = ConvexFxSp1Prover::new();
    let proof1 = prover.prove_clearing(&solution, &inventory).unwrap();
    let proof2 = prover.prove_clearing(&solution, &inventory).unwrap();
    
    assert_eq!(proof1, proof2, "Proofs should be deterministic");
    
    println!("✅ SP1 proofs are deterministic");
}

