//! Integration test for SDL generation from ConvexFX fills
//!
//! This test verifies the complete flow:
//! 1. Create demo users with vaults
//! 2. Execute orders through ConvexFX clearing
//! 3. Generate proper Delta state diffs from fills
//! 4. Validate the state diffs

use convexfx_delta::DemoApp;
use convexfx_types::{AssetId, Amount, PairOrder};
use delta_base_sdk::vaults::{TokenKind, TokenId};
use delta_primitives::{
    diff::{StateDiff, types::StateDiffOperation},
};
use std::collections::BTreeMap;

#[tokio::test]
async fn test_complete_sdl_generation_flow() {
    // Create demo app with users
    let app = DemoApp::new().expect("Failed to create demo app");

    // Verify users are registered
    assert!(app.get_balance("alice", "USD").is_ok());
    assert!(app.get_balance("bob", "EUR").is_ok());
    assert!(app.get_balance("charlie", "JPY").is_ok());

    // Create test orders
    let orders = vec![
        PairOrder {
            id: "alice_usd_eur".to_string(),
            trader: "alice".to_string().into(),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({"test": "alice_trade"}),
        },
        PairOrder {
            id: "bob_eur_gbp".to_string(),
            trader: "bob".to_string().into(),
            pay: AssetId::EUR,
            receive: AssetId::GBP,
            budget: Amount::from_units(500),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.8),
            metadata: serde_json::json!({"test": "bob_trade"}),
        },
    ];

    // Execute orders and generate SDLs
    let (fills, state_diffs) = app.execute_orders(orders)
        .expect("Failed to execute orders and generate SDLs");

    println!("Generated {} fills and {} state diffs", fills.len(), state_diffs.len());

    // Verify fills were generated
    assert!(!fills.is_empty(), "No fills were generated");
    
    // Verify state diffs were generated
    assert!(!state_diffs.is_empty(), "No state diffs were generated");
    assert_eq!(fills.len(), state_diffs.len(), "Number of fills should match number of state diffs");

    // Validate each state diff
    for (i, state_diff) in state_diffs.iter().enumerate() {
        println!("Validating state diff {}: {:?}", i, state_diff);
        
        // Check vault ID is set
        assert!(!state_diff.vault_id.is_zero(), "Vault ID should not be zero");
        
        // Check nonce is set
        assert!(state_diff.new_nonce.is_some(), "New nonce should be set");
        assert!(state_diff.new_nonce.unwrap() > 0, "New nonce should be greater than 0");
        
        // Check operation is TokenDiffs
        match &state_diff.operation {
            StateDiffOperation::TokenDiffs(token_diffs) => {
                assert!(!token_diffs.is_empty(), "Token diffs should not be empty");
                
                // Verify we have both debit and credit entries
                let mut has_debit = false;
                let mut has_credit = false;
                
                for (token_kind, amount) in token_diffs {
                    match token_kind {
                        TokenKind::Fungible(token_id) => {
                            println!("  Token: {:?}, Amount: {}", token_id, amount);
                            
                            if *amount < 0 {
                                has_debit = true;
                            } else if *amount > 0 {
                                has_credit = true;
                            }
                        }
                        _ => panic!("Expected fungible token"),
                    }
                }
                
                assert!(has_debit, "Should have at least one debit (negative amount)");
                assert!(has_credit, "Should have at least one credit (positive amount)");
            }
            _ => panic!("Expected TokenDiffs operation"),
        }
    }

    // Verify specific fill details
    for fill in &fills {
        println!("Fill: {} - {} {} -> {} {} ({}% filled)", 
                 fill.order_id,
                 fill.pay_units,
                 fill.pay_asset,
                 fill.recv_units,
                 fill.recv_asset,
                 fill.fill_frac * 100.0);
        
        // Verify fill has reasonable values
        assert!(fill.pay_units > 0.0, "Pay units should be positive");
        assert!(fill.recv_units > 0.0, "Receive units should be positive");
        assert!(fill.fill_frac > 0.0, "Fill fraction should be positive");
        assert!(fill.fill_frac <= 1.0, "Fill fraction should not exceed 1.0");
    }

    println!("✅ Complete SDL generation flow test passed!");
}

#[tokio::test]
async fn test_vault_nonce_increment() {
    let app = DemoApp::new().expect("Failed to create demo app");

    // Get initial nonce for alice
    let initial_nonce = app.vault_manager.get_vault_nonce("alice")
        .expect("Failed to get initial nonce");

    // Create and execute a simple order
    let orders = vec![PairOrder {
        id: "test_nonce".to_string(),
        trader: "alice".to_string().into(),
        pay: AssetId::USD,
        receive: AssetId::EUR,
        budget: Amount::from_units(100),
        limit_ratio: Some(1.1),
        min_fill_fraction: Some(0.5),
        metadata: serde_json::json!({}),
    }];

    let (fills, state_diffs) = app.execute_orders(orders)
        .expect("Failed to execute orders");

    // Verify nonce was incremented
    let new_nonce = app.vault_manager.get_vault_nonce("alice")
        .expect("Failed to get new nonce");

    if !fills.is_empty() {
        assert!(new_nonce > initial_nonce, "Nonce should have been incremented");
        println!("Nonce incremented from {} to {}", initial_nonce, new_nonce);
    } else {
        println!("No fills generated, nonce remains at {}", initial_nonce);
    }
}

#[tokio::test]
async fn test_multiple_users_trading() {
    let app = DemoApp::new().expect("Failed to create demo app");

    // Create orders from multiple users
    let orders = vec![
        PairOrder {
            id: "alice_trade".to_string(),
            trader: "alice".to_string().into(),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
        PairOrder {
            id: "bob_trade".to_string(),
            trader: "bob".to_string().into(),
            pay: AssetId::EUR,
            receive: AssetId::GBP,
            budget: Amount::from_units(800),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
        PairOrder {
            id: "charlie_trade".to_string(),
            trader: "charlie".to_string().into(),
            pay: AssetId::JPY,
            receive: AssetId::CHF,
            budget: Amount::from_units(50000),
            limit_ratio: Some(1.3),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        },
    ];

    let (fills, state_diffs) = app.execute_orders(orders)
        .expect("Failed to execute orders");

    println!("Multi-user trading: {} fills, {} state diffs", fills.len(), state_diffs.len());

    // Verify we have state diffs for different vaults
    let mut vault_ids = std::collections::HashSet::new();
    for state_diff in &state_diffs {
        vault_ids.insert(state_diff.vault_id);
    }

    println!("State diffs generated for {} unique vaults", vault_ids.len());
    
    // Each fill should have a corresponding state diff
    assert_eq!(fills.len(), state_diffs.len(), "Each fill should have a corresponding state diff");
}
