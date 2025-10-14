use convexfx_delta::{
    messages::{DeltaMessage, AssetMapper},
    state::DeltaStateManager,
    sdl_generator::SdlGenerator,
    VerifiableWithDiffs, VerifiableType, SwapMessage,
};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_types::{AccountId, Amount, AssetId, Fill};
use delta_base_sdk::vaults::{OwnerId};
use delta_base_sdk::crypto::{ed25519::{PrivKey}, Hash256};
use std::collections::BTreeMap;

#[test]
fn test_delta_integration_basic_functionality() {
    println!("🧪 Testing Delta Integration Basic Functionality");

    // Test 1: Asset Mapping
    println!("📍 Test 1: Asset Mapping");
    assert_eq!(AssetMapper::delta_to_convexfx("USD").unwrap(), AssetId::USD);
    assert_eq!(AssetMapper::delta_to_convexfx("EUR").unwrap(), AssetId::EUR);
    assert_eq!(AssetMapper::convexfx_to_delta(AssetId::USD), "USD");
    assert_eq!(AssetMapper::convexfx_to_delta(AssetId::EUR), "EUR");
    println!("✅ Asset mapping works correctly");

    // Test 2: State Manager
    println!("🏦 Test 2: State Manager");
    let mut state_manager = DeltaStateManager::new();

    let alice_privkey = PrivKey::generate();
    let alice_owner = OwnerId::from(alice_privkey.pub_key().hash_sha256());
    let alice_account = AccountId::new("alice_test".to_string());

    state_manager.register_owner(alice_owner, alice_account.clone());
    println!("✅ State manager registration works");

    // Test 3: SDL Generation
    println!("📋 Test 3: SDL Generation");
    let mut sdl_generator = SdlGenerator::new();
    sdl_generator.register_account(alice_account, alice_owner);

    // Create mock fills
    let fills = vec![Fill {
        order_id: "test_order_1".to_string(),
        fill_frac: 1.0,
        pay_asset: AssetId::USD,
        recv_asset: AssetId::EUR,
        pay_units: 1000.0,
        recv_units: 900.0,
        fees_paid: BTreeMap::new(),
    }];

    let verifiable_diffs = sdl_generator.generate_sdl_from_fills(fills, 1).unwrap();
    assert_eq!(verifiable_diffs.state_diffs.len(), 1);
    println!("✅ SDL generation works correctly");

    // Test 4: Message Creation
    println!("📨 Test 4: Message Creation");
    let bob_privkey = PrivKey::generate();
    let bob_owner = OwnerId::from(bob_privkey.pub_key().hash_sha256());

    let swap_message = DeltaMessage::swap(
        bob_owner,
        AssetId::EUR,
        AssetId::JPY,
        Amount::from_f64(500.0).unwrap(),
        Some(140.0),
        Some(0.8),
    );

    let order_id = "test_order_2".to_string();
    let pair_order = swap_message.to_pair_order(order_id.clone()).unwrap();

    assert_eq!(pair_order.id, order_id);
    assert_eq!(pair_order.pay, AssetId::EUR);
    assert_eq!(pair_order.receive, AssetId::JPY);
    assert_eq!(pair_order.budget.to_f64(), 500.0);
    println!("✅ Message creation and conversion works");

    // Test 5: Exchange Integration
    println!("🔄 Test 5: Exchange Integration");
    let exchange = Exchange::new(ExchangeConfig::default()).unwrap();

    // This tests that the exchange can be created and basic operations work
    let assets = exchange.list_assets().unwrap();
    assert!(!assets.is_empty());
    println!("✅ Exchange integration works");

    println!("🎉 All Delta integration tests passed!");
}

#[test]
fn test_delta_message_conversion() {
    println!("🔄 Testing Delta Message Conversion");

    let owner = OwnerId::from(PrivKey::generate().pub_key().hash_sha256());

    // Test swap message
    let swap_msg = DeltaMessage::swap(
        owner,
        AssetId::USD,
        AssetId::EUR,
        Amount::from_f64(1000.0).unwrap(),
        Some(1.1),
        Some(0.5),
    );

    let order_id = "swap_test".to_string();
    let pair_order = swap_msg.to_pair_order(order_id.clone()).unwrap();

    assert_eq!(pair_order.id, order_id);
    assert_eq!(pair_order.pay, AssetId::USD);
    assert_eq!(pair_order.receive, AssetId::EUR);
    assert_eq!(pair_order.limit_ratio, Some(1.1));
    assert_eq!(pair_order.min_fill_fraction, Some(0.5));

    // Test that liquidity messages properly fail conversion to orders
    let liquidity_msg = DeltaMessage::liquidity(
        owner,
        AssetId::USD,
        Amount::from_f64(500.0).unwrap(),
    );

    assert!(liquidity_msg.to_pair_order("test".to_string()).is_err());

    println!("✅ Message conversion tests passed!");
}

#[test]
fn test_sdl_validation() {
    println!("✅ Testing SDL Validation");

    let generator = SdlGenerator::new();

    // Test empty SDL validation
    let empty_sdl = VerifiableWithDiffs {
        verifiable: VerifiableType {
            swap_message: Some(SwapMessage {
                owner: OwnerId::default(),
                pay_asset: "USD".to_string(),
                receive_asset: "EUR".to_string(),
                budget: "1000".to_string(),
                limit_ratio: None,
                min_fill_fraction: None,
            }),
        },
        state_diffs: vec![],
    };

    // Empty SDL is now valid (represents batch with no fills)
    assert!(generator.validate_sdl(&empty_sdl).is_ok());

    // Test valid SDL
    let fills = vec![Fill {
        order_id: "test_order".to_string(),
        fill_frac: 1.0,
        pay_asset: AssetId::USD,
        recv_asset: AssetId::EUR,
        pay_units: 1000.0,
        recv_units: 900.0,
        fees_paid: BTreeMap::new(),
    }];

    let valid_sdl = generator.generate_sdl_from_fills(fills, 1).unwrap();
    assert!(generator.validate_sdl(&valid_sdl).is_ok());

    println!("✅ SDL validation tests passed!");
}
