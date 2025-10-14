use convexfx_exchange::{Exchange, ExchangeConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ConvexFX Exchange Demo");
    println!("========================");

    // Create exchange with default configuration
    let mut exchange = Exchange::new(ExchangeConfig::default())?;

    println!("✅ Exchange initialized");

    // Display available assets
    println!("\n💰 Available Assets:");
    for asset in exchange.list_assets()? {
        println!("   {}: {} ({} decimals) - Price: ${:.4}",
                 asset.symbol,
                 asset.name,
                 asset.decimals,
                 asset.current_price.unwrap_or(0.0));
    }

    // Demonstrate adding liquidity
    println!("\n🏦 Adding initial liquidity...");
    exchange.add_liquidity("lp_1", "USD", 1_000_000.0)?;
    exchange.add_liquidity("lp_1", "EUR", 850_000.0)?;
    exchange.add_liquidity("lp_2", "JPY", 100_000_000.0)?;

    println!("\n✅ Liquidity added");
    println!("\n💧 Total Liquidity:");
    for (asset, amount) in exchange.get_total_liquidity()? {
        println!("   {}: {:.2}", asset, amount);
    }

    // Demonstrate submitting orders
    println!("\n📈 Submitting sample orders...");
    let order1 = exchange.submit_order("trader_1", "USD", "EUR", 1000.0, Some(1.10), None)?;
    let order2 = exchange.submit_order("trader_2", "EUR", "USD", 800.0, Some(0.95), None)?;

    println!("✅ Orders submitted:");
    println!("   Order {}: {} -> {} for ${}", order1.order_id, order1.pay_asset, order1.receive_asset, order1.budget);
    println!("   Order {}: {} -> {} for ${}", order2.order_id, order2.pay_asset, order2.receive_asset, order2.budget);

    // Execute a batch
    println!("\n⚡ Executing clearing batch...");
    let batch_result = exchange.execute_batch()?;
    println!("✅ Batch #{} executed", batch_result.epoch_id);
    println!("   Fills processed: {}", batch_result.fills.len());

    // Show final status
    let status = exchange.get_status();
    println!("\n📊 Final System Status:");
    println!("   Status: {:?}", status.status);
    println!("   Current Epoch: {}", status.current_epoch);
    println!("   Total Accounts: {}", status.total_accounts);
    println!("   Total Orders Pending: {}", status.total_orders_pending);
    println!("   Uptime: {}s", status.uptime_seconds);

    println!("\n✅ Demo complete! Exchange is ready for production use.");

    Ok(())
}
