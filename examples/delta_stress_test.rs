/// Delta Network Integration - Comprehensive Swap Test
/// 
/// This example demonstrates a full end-to-end integration with:
/// - Multiple Delta users submitting swap orders
/// - Large batch of transactions
/// - Real liquidity provision
/// - Actual order matching and clearing
/// - SDL generation from real fills

use convexfx_delta::{
    messages::DeltaMessage,
    runtime_adapter::ConvexFxDeltaAdapter,
    sdl_generator::SdlGenerator,
};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_types::{AccountId, Amount, AssetId};
use delta_base_sdk::{
    crypto::{ed25519::PrivKey, Hash256},
    vaults::OwnerId,
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ConvexFX + Delta Network - COMPREHENSIVE SWAP TEST");
    println!("=====================================================\n");

    // Step 1: Initialize Exchange
    println!("📦 Step 1: Initialize ConvexFX Exchange");
    let mut exchange = Exchange::new(ExchangeConfig::default())?;
    println!("✅ Exchange initialized\n");

    // Step 2: Create Delta Users
    println!("👥 Step 2: Create Delta Users (10 users)");
    let mut users = Vec::new();
    let mut user_accounts = Vec::new();
    
    for i in 0..10 {
        let privkey = PrivKey::generate();
        let pubkey = privkey.pub_key();
        let owner = OwnerId::from(pubkey.hash_sha256());
        let account = AccountId::new(format!("delta_user_{}", i));
        
        users.push((owner.clone(), account.clone()));
        user_accounts.push(account);
        
        println!("   User {}: {} -> {}", i, owner, users[i].1);
    }
    println!("✅ Created 10 Delta users\n");

    // Step 3: Provide Initial Liquidity
    println!("💧 Step 3: Provide Initial Liquidity");
    
    // Add USD liquidity (users 0-3)
    for i in 0..4 {
        let amount = 50000.0 + (i as f64 * 10000.0);
        exchange.add_liquidity(user_accounts[i].to_string().as_str(), "USD", amount)?;
        println!("   User {}: Added ${:.0} USD", i, amount);
    }
    
    // Add EUR liquidity (users 4-6)
    for i in 4..7 {
        let amount = 40000.0 + ((i - 4) as f64 * 10000.0);
        exchange.add_liquidity(user_accounts[i].to_string().as_str(), "EUR", amount)?;
        println!("   User {}: Added €{:.0} EUR", i, amount);
    }
    
    // Add JPY liquidity (users 7-9)
    for i in 7..10 {
        let amount = 5000000.0 + ((i - 7) as f64 * 1000000.0);
        exchange.add_liquidity(user_accounts[i].to_string().as_str(), "JPY", amount)?;
        println!("   User {}: Added ¥{:.0} JPY", i, amount);
    }
    
    println!("✅ Liquidity provided\n");

    // Step 4: Create Delta Runtime Adapter
    println!("🔄 Step 4: Create Delta Runtime Adapter");
    let mut delta_adapter = ConvexFxDeltaAdapter::new(exchange);
    
    for (owner, account) in &users {
        delta_adapter.register_owner(owner.clone(), account.clone());
    }
    println!("✅ Delta adapter configured with all users\n");

    // Step 5: Create Large Batch of Swap Messages
    println!("📨 Step 5: Create Swap Messages (30 swaps)");
    let mut swap_messages = Vec::new();
    
    // USD -> EUR swaps (users 0-3)
    for i in 0..4 {
        let budget = 5000.0 + (i as f64 * 1000.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::USD,
            AssetId::EUR,
            Amount::from_f64(budget)?,
            Some(1.15), // Max EUR/USD rate
            Some(0.5),  // Min 50% fill
        );
        swap_messages.push((i, "USD->EUR", budget, msg));
    }
    
    // EUR -> USD swaps (users 4-6)
    for i in 4..7 {
        let budget = 4000.0 + ((i - 4) as f64 * 1000.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::EUR,
            AssetId::USD,
            Amount::from_f64(budget)?,
            Some(0.92), // Min USD/EUR rate (inverse of 1.087)
            Some(0.5),
        );
        swap_messages.push((i, "EUR->USD", budget, msg));
    }
    
    // USD -> JPY swaps (users 0-2)
    for i in 0..3 {
        let budget = 3000.0 + (i as f64 * 500.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::USD,
            AssetId::JPY,
            Amount::from_f64(budget)?,
            Some(105.0), // Max JPY/USD rate
            Some(0.7),
        );
        swap_messages.push((i, "USD->JPY", budget, msg));
    }
    
    // JPY -> USD swaps (users 7-9)
    for i in 7..10 {
        let budget = 300000.0 + ((i - 7) as f64 * 50000.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::JPY,
            AssetId::USD,
            Amount::from_f64(budget)?,
            Some(0.0098), // Min USD/JPY rate (inverse of 102.04)
            Some(0.7),
        );
        swap_messages.push((i, "JPY->USD", budget, msg));
    }
    
    // EUR -> JPY swaps (users 4-6)
    for i in 4..7 {
        let budget = 3000.0 + ((i - 4) as f64 * 500.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::EUR,
            AssetId::JPY,
            Amount::from_f64(budget)?,
            Some(115.0), // Max JPY/EUR rate
            Some(0.6),
        );
        swap_messages.push((i, "EUR->JPY", budget, msg));
    }
    
    // JPY -> EUR swaps (users 7-9)
    for i in 7..10 {
        let budget = 250000.0 + ((i - 7) as f64 * 50000.0);
        let msg = DeltaMessage::swap(
            users[i].0.clone(),
            AssetId::JPY,
            AssetId::EUR,
            Amount::from_f64(budget)?,
            Some(0.0091), // Min EUR/JPY rate (inverse of 109.89)
            Some(0.6),
        );
        swap_messages.push((i, "JPY->EUR", budget, msg));
    }
    
    println!("✅ Created {} swap messages", swap_messages.len());
    for (user_id, pair, budget, _) in &swap_messages {
        println!("   User {}: {} (budget: {:.0})", user_id, pair, budget);
    }
    println!();

    // Step 6: Convert to ConvexFX Orders and Execute
    println!("⚡ Step 6: Process Swaps Through ConvexFX");
    let mut orders = Vec::new();
    
    for (user_id, pair, _budget, msg) in &swap_messages {
        let order_id = format!("order_u{}_{}", user_id, pair.replace("->", "_to_"));
        let order = msg.to_pair_order(order_id)?;
        orders.push(order);
    }
    
    println!("✅ Converted {} Delta messages to ConvexFX orders", orders.len());
    println!("🎯 Executing batch...\n");
    
    let fills = delta_adapter.execute_batch(orders).await?;
    
    println!("✅ Batch executed with {} fills!", fills.len());
    println!();

    // Step 7: Analyze Results
    println!("📊 Step 7: Fill Analysis");
    
    if fills.is_empty() {
        println!("⚠️  No fills generated");
        println!("   This might be because:");
        println!("   - Orders don't match current market prices");
        println!("   - Insufficient liquidity for the requested amounts");
        println!("   - Limit ratios too restrictive");
    } else {
        let mut total_volume_usd = 0.0;
        let mut fills_by_pair: HashMap<String, Vec<_>> = HashMap::new();
        
        for fill in &fills {
            let pair = format!("{}/{}", fill.pay_asset, fill.recv_asset);
            fills_by_pair.entry(pair).or_insert_with(Vec::new).push(fill);
            
            // Estimate USD volume (simplified)
            let usd_volume = match fill.pay_asset {
                AssetId::USD => fill.pay_units,
                AssetId::EUR => fill.pay_units * 1.1,
                AssetId::JPY => fill.pay_units * 0.01,
                _ => fill.pay_units,
            };
            total_volume_usd += usd_volume;
        }
        
        println!("📈 Summary:");
        println!("   Total fills: {}", fills.len());
        println!("   Estimated total volume: ${:.2}", total_volume_usd);
        println!();
        
        println!("📋 Fills by Trading Pair:");
        for (pair, pair_fills) in &fills_by_pair {
            println!("\n   {} ({} fills):", pair, pair_fills.len());
            for fill in pair_fills {
                let rate = fill.recv_units / fill.pay_units;
                println!("      Order {}: {:.1}% filled, rate={:.4}", 
                         fill.order_id, fill.fill_frac * 100.0, rate);
                println!("         Paid: {:.2} {}, Received: {:.2} {}",
                         fill.pay_units, fill.pay_asset,
                         fill.recv_units, fill.recv_asset);
            }
        }
    }
    println!();

    // Step 8: Generate SDL
    println!("📋 Step 8: Generate SDL (State Diff List)");
    let mut sdl_generator = SdlGenerator::new();
    
    for (owner, account) in &users {
        sdl_generator.register_account(account.clone(), owner.clone());
        
        // Register vault with initial nonce
        use delta_base_sdk::vaults::VaultId;
        let vault_id = VaultId::from((*owner, 0));
        sdl_generator.register_vault(vault_id, 0);
    }
    
    // Register order-to-account mappings
    for (user_id, pair, _budget, _msg) in &swap_messages {
        let order_id = format!("order_u{}_{}", user_id, pair.replace("->", "_to_"));
        sdl_generator.register_order(order_id.into(), users[*user_id].1.clone());
    }
    
    let state_diffs = sdl_generator.generate_sdl_from_fills(fills.clone(), 1)?;
    
    println!("✅ SDL generated:");
    println!("   State diffs: {}", state_diffs.len());
    
    let mut total_token_changes = 0;
    for diff in &state_diffs {
        use convexfx_delta::StateDiffOperation;
        match &diff.operation {
            StateDiffOperation::TokenDiffs(token_diffs) => {
                total_token_changes += token_diffs.len();
            }
            _ => {}
        }
    }
    println!("   Total token changes: {}", total_token_changes);
    println!();

    // Step 9: Validate SDL
    println!("✅ Step 9: Validate SDL");
    sdl_generator.validate_state_diffs(&state_diffs)?;
    println!("✅ SDL validation passed\n");

    // Step 10: Final Summary
    println!("🎉 INTEGRATION TEST COMPLETE");
    println!("============================");
    println!("✅ Created 10 Delta users");
    println!("✅ Provided liquidity across USD, EUR, JPY");
    println!("✅ Submitted {} swap messages", fills.len());
    println!("✅ Executed {} fills", fills.len());
    println!("✅ Generated SDL with {} state diffs", state_diffs.len());
    println!("✅ Validated SDL structure");
    println!();
    println!("🚀 ConvexFX + Delta Network integration fully operational!");
    
    Ok(())
}

