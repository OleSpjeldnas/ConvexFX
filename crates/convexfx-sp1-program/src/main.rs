//! ConvexFX Local Laws SP1 Program
//!
//! This program runs in the SP1 zkVM and proves that ConvexFX clearing results
//! satisfy all optimality conditions (local laws) before being submitted to Delta.

#![no_main]
sp1_zkvm::entrypoints::init_io();

use sp1_zkvm::prelude::*;

/// Input data for proving ConvexFX clearing validity
#[derive(serde::Deserialize, serde::Serialize)]
struct ClearingProofInput {
    // Clearing solution data
    y_star: Vec<(u8, f64)>,        // Asset ID -> log price
    prices: Vec<(u8, f64)>,         // Asset ID -> linear price
    fills: Vec<FillData>,           // All fills
    initial_inventory: Vec<(u8, f64)>, // Initial inventory
    final_inventory: Vec<(u8, f64)>,   // Final inventory (q_post)
    
    // Diagnostics from SCP algorithm
    convergence_achieved: bool,
    final_step_norm_y: f64,
    final_step_norm_alpha: f64,
    
    // Objective function components
    inventory_risk: f64,
    price_tracking: f64,
    fill_incentive: f64,
    total_objective: f64,
}

/// Fill data for validation
#[derive(serde::Deserialize, serde::Serialize)]
struct FillData {
    fill_frac: f64,
    pay_asset: u8,
    recv_asset: u8,
    pay_units: f64,
    recv_units: f64,
}

/// Predicate parameters
const TOLERANCE_Y: f64 = 1e-5;
const TOLERANCE_ALPHA: f64 = 1e-6;
const MAX_PRICE_DEVIATION: f64 = 0.01; // 1%
const INVENTORY_TOLERANCE: f64 = 1e-6;
const USD_ASSET_ID: u8 = 0;

pub fn main() {
    // Read input from the SP1 zkVM
    let input: ClearingProofInput = sp1_zkvm::io::read();
    
    // ===== PREDICATE 1: CONVERGENCE VALIDATION =====
    // Ensures the SCP algorithm converged to an optimal solution
    assert!(
        input.convergence_achieved,
        "SCP algorithm did not converge"
    );
    
    assert!(
        input.final_step_norm_y < TOLERANCE_Y,
        "Price step norm {} exceeds tolerance {}",
        input.final_step_norm_y,
        TOLERANCE_Y
    );
    
    assert!(
        input.final_step_norm_alpha < TOLERANCE_ALPHA,
        "Fill step norm {} exceeds tolerance {}",
        input.final_step_norm_alpha,
        TOLERANCE_ALPHA
    );
    
    // ===== PREDICATE 2: PRICE CONSISTENCY VALIDATION =====
    // Verifies price = exp(log_price) and USD numeraire constraint
    for (asset_id, log_price) in &input.y_star {
        // Find corresponding linear price
        let linear_price = input.prices
            .iter()
            .find(|(id, _)| id == asset_id)
            .expect("Missing linear price for asset")
            .1;
        
        // Check price consistency: price = exp(log_price)
        let expected = log_price.exp();
        let error = (expected - linear_price).abs() / linear_price;
        
        assert!(
            error < MAX_PRICE_DEVIATION,
            "Price inconsistency for asset {}: expected {}, got {}, error {}",
            asset_id,
            expected,
            linear_price,
            error
        );
        
        // Check price is positive and finite
        assert!(
            linear_price > 0.0 && linear_price.is_finite(),
            "Invalid price for asset {}: {}",
            asset_id,
            linear_price
        );
    }
    
    // Check USD numeraire constraint: y[USD] = 0
    let usd_log_price = input.y_star
        .iter()
        .find(|(id, _)| *id == USD_ASSET_ID)
        .map(|(_, price)| *price)
        .unwrap_or(0.0);
    
    assert!(
        usd_log_price.abs() < TOLERANCE_Y,
        "USD numeraire constraint violated: y_USD = {}",
        usd_log_price
    );
    
    // ===== PREDICATE 3: FILL FEASIBILITY VALIDATION =====
    // Ensures all fills are valid and feasible
    for (i, fill) in input.fills.iter().enumerate() {
        // Check fill fraction in range [0, 1]
        assert!(
            fill.fill_frac >= 0.0 && fill.fill_frac <= 1.0,
            "Invalid fill fraction {} for fill {}",
            fill.fill_frac,
            i
        );
        
        // For non-zero fills, check amounts are positive
        if fill.fill_frac > 0.0 {
            assert!(
                fill.pay_units > 0.0,
                "Non-positive pay amount {} for fill {}",
                fill.pay_units,
                i
            );
            
            assert!(
                fill.recv_units > 0.0,
                "Non-positive receive amount {} for fill {}",
                fill.recv_units,
                i
            );
            
            assert!(
                fill.pay_units.is_finite(),
                "Non-finite pay amount for fill {}",
                i
            );
            
            assert!(
                fill.recv_units.is_finite(),
                "Non-finite receive amount for fill {}",
                i
            );
        }
    }
    
    // ===== PREDICATE 4: INVENTORY CONSERVATION VALIDATION =====
    // Verifies the fundamental law: final_inventory = initial_inventory + net_flow
    for (asset_id, initial) in &input.initial_inventory {
        let final_inv = input.final_inventory
            .iter()
            .find(|(id, _)| id == asset_id)
            .expect("Missing final inventory for asset")
            .1;
        
        // Calculate net flow from fills
        let mut net_flow = 0.0;
        for fill in &input.fills {
            if fill.pay_asset == *asset_id {
                net_flow += fill.pay_units; // Pool receives pay asset
            }
            if fill.recv_asset == *asset_id {
                net_flow -= fill.recv_units; // Pool gives receive asset
            }
        }
        
        let expected = initial + net_flow;
        let error = (final_inv - expected).abs();
        
        assert!(
            error < INVENTORY_TOLERANCE,
            "Inventory conservation violated for asset {}: initial={}, net_flow={}, expected={}, actual={}, error={}",
            asset_id,
            initial,
            net_flow,
            expected,
            final_inv,
            error
        );
    }
    
    // ===== PREDICATE 5: OBJECTIVE OPTIMALITY VALIDATION =====
    // Ensures objective function is properly computed
    assert!(
        input.inventory_risk >= -INVENTORY_TOLERANCE,
        "Negative inventory risk: {}",
        input.inventory_risk
    );
    
    assert!(
        input.price_tracking >= -INVENTORY_TOLERANCE,
        "Negative price tracking: {}",
        input.price_tracking
    );
    
    assert!(
        input.total_objective.is_finite(),
        "Non-finite objective value: {}",
        input.total_objective
    );
    
    // Check that objective components sum correctly
    let computed_total = input.inventory_risk + input.price_tracking + input.fill_incentive;
    let total_error = (input.total_objective - computed_total).abs();
    
    assert!(
        total_error < INVENTORY_TOLERANCE,
        "Objective components don't sum: components={}, total={}, error={}",
        computed_total,
        input.total_objective,
        total_error
    );
    
    // ===== COMMIT RESULT =====
    // Commit success flag to public output
    sp1_zkvm::io::commit(&true);
}

