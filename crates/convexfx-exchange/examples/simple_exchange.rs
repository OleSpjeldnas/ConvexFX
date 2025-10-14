use convexfx_exchange::{Exchange, ExchangeConfig};
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ConvexFX Exchange Demo");
    println!("========================");

    // Create exchange with default configuration
    let mut exchange = Exchange::new(ExchangeConfig::default())?;

    println!("✅ Exchange initialized with assets:");
    for asset in exchange.list_assets()? {
        println!("   - {}: {} ({} decimals)",
                 asset.symbol,
                 asset.name,
                 asset.decimals);
    }
    println!();

    // Start the exchange
    println!("🔄 Starting exchange...");
    exchange.start().await?;

    Ok(())
}
