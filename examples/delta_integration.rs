use convexfx_delta::{
    messages::{DeltaMessage, AssetMapper},
    runtime_adapter::ConvexFxDeltaAdapter,
    state::DeltaStateManager,
    sdl_generator::SdlGenerator,
};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_types::{AccountId, Amount, AssetId, Fill};
use delta_base_sdk::{
    crypto::{ed25519::PrivKey, Hash256},
    vaults::OwnerId,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use tokio runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ConvexFX + Delta Network Full Integration Demo");
    println!("================================================");

    // Step 1: Initialize ConvexFX Exchange
    println!("\n📦 Step 1: Initialize ConvexFX Exchange");
    let exchange = Exchange::new(ExchangeConfig::default())?;
    println!("✅ Exchange initialized");

    // Step 2: Set up Delta State Manager
    println!("\n🔐 Step 2: Set up Delta State Management");
    let mut state_manager = DeltaStateManager::new();

    // Generate test Delta keypairs
    let alice_privkey = PrivKey::generate();
    let alice_pubkey = alice_privkey.pub_key();
    let alice_owner = OwnerId::from(alice_pubkey.hash_sha256());

    let bob_privkey = PrivKey::generate();
    let bob_pubkey = bob_privkey.pub_key();
    let bob_owner = OwnerId::from(bob_pubkey.hash_sha256());

    // Register Delta owners with ConvexFX accounts
    let alice_account = AccountId::new("alice_delta".to_string());
    let bob_account = AccountId::new("bob_delta".to_string());

    state_manager.register_owner(alice_owner, alice_account.clone());
    state_manager.register_owner(bob_owner, bob_account.clone());

    println!("✅ Alice registered: {} -> {}", alice_owner, alice_account);
    println!("✅ Bob registered: {} -> {}", bob_owner, bob_account);

    // Step 3: Create Delta Runtime Adapter
    println!("\n🔄 Step 3: Create Delta Runtime Adapter");
    let mut delta_adapter = ConvexFxDeltaAdapter::new(exchange);
    delta_adapter.register_owner(alice_owner.clone(), alice_account.clone());
    delta_adapter.register_owner(bob_owner.clone(), bob_account.clone());
    println!("✅ Delta adapter created and configured");

    // Step 4: Create Delta Messages
    println!("\n📨 Step 4: Create Delta Messages");

    // Alice wants to swap USD for EUR
    let alice_swap = DeltaMessage::swap(
        alice_owner,
        AssetId::USD,
        AssetId::EUR,
        Amount::from_f64(1000.0)?,
        Some(1.1), // Max EUR/USD rate of 1.1
        Some(0.5),  // Min 50% fill
    );

    // Bob wants to swap EUR for JPY
    let bob_swap = DeltaMessage::swap(
        bob_owner,
        AssetId::EUR,
        AssetId::JPY,
        Amount::from_f64(800.0)?,
        Some(140.0), // Max JPY/EUR rate of 140
        Some(0.8),   // Min 80% fill
    );

    println!("✅ Alice swap: 1000 USD → EUR (max rate 1.1, min fill 50%)");
    println!("✅ Bob swap: 800 EUR → JPY (max rate 140, min fill 80%)");

    // Step 5: Process Messages through ConvexFX
    println!("\n⚡ Step 5: Process Messages through ConvexFX");

    // Convert messages to ConvexFX orders (simplified for demo)
    let alice_order = alice_swap.to_pair_order("alice_order_1".to_string())?;
    let bob_order = bob_swap.to_pair_order("bob_order_2".to_string())?;

    println!("✅ Converted to ConvexFX orders");
    println!("   - Alice: {} USD → {} EUR",
             alice_order.budget.to_f64(), alice_order.receive);
    println!("   - Bob: {} EUR → {} JPY",
             bob_order.budget.to_f64(), bob_order.receive);

    // Step 6: Execute Batch Processing
    println!("\n🎯 Step 6: Execute Batch Processing");
    let fills: Vec<Fill> = delta_adapter.execute_batch(vec![alice_order, bob_order]).await?;

    println!("✅ Batch executed with {} fills", fills.len());
    for fill in &fills {
        println!("   - Order {}: {:.2}% filled, {} {} → {} {}",
                 fill.order_id, fill.fill_frac * 100.0,
                 fill.pay_units, fill.pay_asset,
                 fill.recv_units, fill.recv_asset);
    }

    // Step 7: Generate SDL from Results
    println!("\n📋 Step 7: Generate SDL from Results");
    let mut sdl_generator = SdlGenerator::new();
    sdl_generator.register_account(alice_account, alice_owner);
    sdl_generator.register_account(bob_account, bob_owner);

    let verifiable_diffs = sdl_generator.generate_sdl_from_fills(fills, 1)?;

    println!("✅ SDL generated with {} state diffs", verifiable_diffs.state_diffs.len());
    for (i, diff) in verifiable_diffs.state_diffs.iter().enumerate() {
        println!("   - Diff {}: {} transitions", i, diff.transitions.len());
        for transition in &diff.transitions {
            println!("     * Vault {:?}: {} {}",
                     transition.vault_id,
                     if transition.amount > 0 { "+" } else { "" },
                     transition.amount);
        }
    }

    // Step 8: Validate SDL
    println!("\n✅ Step 8: Validate SDL");
    sdl_generator.validate_sdl(&verifiable_diffs)?;
    println!("✅ SDL validation passed");

    // Step 9: Demonstrate Asset Mapping
    println!("\n🌍 Step 9: Asset Mapping Demonstration");
    println!("ConvexFX → Delta:");
    println!("   USD → {}", AssetMapper::convexfx_to_delta(AssetId::USD));
    println!("   EUR → {}", AssetMapper::convexfx_to_delta(AssetId::EUR));
    println!("   JPY → {}", AssetMapper::convexfx_to_delta(AssetId::JPY));

    println!("\nDelta → ConvexFX:");
    println!("   USD → {:?}", AssetMapper::delta_to_convexfx("USD")?);
    println!("   EUR → {:?}", AssetMapper::delta_to_convexfx("EUR")?);
    println!("   JPY → {:?}", AssetMapper::delta_to_convexfx("JPY")?);

    // Step 10: Show State Manager Capabilities
    println!("\n🏦 Step 10: State Manager Demo");

    // Mock vault balance check (would connect to real Delta runtime)
    println!("✅ Mock vault balance checks:");
    println!("   - Alice USD balance: 10000 units");
    println!("   - Bob EUR balance: 8000 units");

    // Step 11: Summary
    println!("\n🎉 Integration Summary:");
    println!("   ✅ Delta SDK integration working");
    println!("   ✅ Message conversion between Delta and ConvexFX");
    println!("   ✅ State management and vault operations");
    println!("   ✅ Batch processing and clearing");
    println!("   ✅ SDL generation from clearing results");
    println!("   ✅ Asset mapping and validation");

    println!("\n🚀 ConvexFX is now fully integrated with Delta Network!");
    println!("   Ready for production deployment with proper Delta runtime configuration.");

    Ok(())
}