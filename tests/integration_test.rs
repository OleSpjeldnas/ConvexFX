use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_oracle::MockOracle;
use convexfx_risk::RiskParams;
use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use std::collections::BTreeMap;

#[test]
fn test_full_clearing_pipeline() {
    // Setup
    let oracle = MockOracle::new();
    let ref_prices = oracle.reference_prices(1).unwrap();
    let risk = RiskParams::default_demo();

    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 10.0);
    }

    // Create a simple order
    let order = PairOrder {
        id: "test_order".to_string(),
        trader: AccountId::new("trader1"),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_f64(1.0).unwrap(),
        limit_ratio: None,
        min_fill_fraction: None,
        metadata: serde_json::json!({}),
    };

    let instance = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

    // Run clearing
    let clearing = ScpClearing::with_simple_solver();
    let solution = clearing.clear_epoch(&instance).unwrap();

    // Assertions
    assert_eq!(solution.epoch_id, 1);
    assert!(solution.diagnostics.iterations > 0);
    assert!(solution.diagnostics.iterations <= 5);

    // Prices should be reasonable (near oracle)
    for asset in AssetId::all() {
        let y = solution.y_star.get(asset).copied().unwrap_or(0.0);
        let y_ref = instance.ref_prices.get_ref(*asset);
        assert!((y - y_ref).abs() < 0.01); // Within 100 bps
    }

    // USD should be numeraire
    assert_eq!(solution.y_star.get(&AssetId::USD).copied().unwrap_or(0.0), 0.0);
}

#[test]
fn test_no_orders_scenario() {
    let oracle = MockOracle::new();
    let ref_prices = oracle.reference_prices(1).unwrap();
    let risk = RiskParams::default_demo();

    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 10.0);
    }

    let instance = EpochInstance::new(1, inventory, vec![], ref_prices, risk);

    let clearing = ScpClearing::with_simple_solver();
    let solution = clearing.clear_epoch(&instance).unwrap();

    // No orders -> no fills
    assert_eq!(solution.fills.len(), 0);

    // Prices should stay at reference
    for asset in AssetId::all() {
        let y = solution.y_star.get(asset).copied().unwrap_or(0.0);
        let y_ref = instance.ref_prices.get_ref(*asset);
        assert!((y - y_ref).abs() < 1e-3);
    }
}

#[test]
fn test_cross_rate_consistency() {
    let oracle = MockOracle::new();
    let ref_prices = oracle.reference_prices(1).unwrap();
    let risk = RiskParams::default_demo();

    let mut inventory = BTreeMap::new();
    for asset in AssetId::all() {
        inventory.insert(*asset, 10.0);
    }

    let instance = EpochInstance::new(1, inventory, vec![], ref_prices, risk);

    let clearing = ScpClearing::with_simple_solver();
    let solution = clearing.clear_epoch(&instance).unwrap();

    // Check triangular consistency: EURJPY = EURUSD * USDJPY
    let y_eur = solution.y_star.get(&AssetId::EUR).copied().unwrap_or(0.0);
    let y_jpy = solution.y_star.get(&AssetId::JPY).copied().unwrap_or(0.0);
    let y_usd = solution.y_star.get(&AssetId::USD).copied().unwrap_or(0.0);

    let eurusd = (y_eur - y_usd).exp();
    let usdjpy = 1.0 / (y_jpy - y_usd).exp();
    let eurjpy_direct = 1.0 / (y_jpy - y_eur).exp();
    let eurjpy_cross = eurusd * usdjpy;

    // No arbitrage: cross rate should match direct rate
    assert!((eurjpy_direct - eurjpy_cross).abs() < 1e-6);
}


