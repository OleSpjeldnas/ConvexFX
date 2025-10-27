//! Integration tests for Delta integration
//! 
//! These tests verify that the core components work together properly

use convexfx_delta::SdlGenerator;
use delta_primitives::diff::StateDiff;
use convexfx_exchange::{Exchange, ExchangeConfig};

#[test]
fn test_basic_integration() {
    println!("🧪 Testing basic Delta integration components");

    // Test 1: SDL Generator can be created
    println!("📋 Test 1: SDL Generator Creation");
    let _generator = SdlGenerator::new();
    println!("✅ SDL Generator created successfully");

    // Test 2: Exchange Integration
    println!("🔄 Test 2: Exchange Integration");
    let exchange = Exchange::new(ExchangeConfig::default()).unwrap();
    
    // This tests that the exchange can be created and basic operations work
    let assets = exchange.list_assets().unwrap();
    assert!(!assets.is_empty(), "Exchange should have assets");
    println!("✅ Exchange integration works with {} assets", assets.len());

    println!("🎉 All basic integration tests passed!");
}

#[test]
fn test_sdl_validation() {
    println!("✅ Testing State Diffs Validation");

    let generator = SdlGenerator::new();

    // Empty state diffs list is valid (represents batch with no fills)
    let empty_state_diffs: Vec<StateDiff> = vec![];
    assert!(generator.validate_state_diffs(&empty_state_diffs).is_ok());

    // Note: Full state diffs validation requires registered accounts and vaults
    // which is tested in the unit tests

    println!("✅ State diffs validation tests passed!");
}
