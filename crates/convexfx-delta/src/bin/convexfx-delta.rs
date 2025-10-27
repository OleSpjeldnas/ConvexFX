//! ConvexFX Delta Executor Binary
//!
//! This binary demonstrates how to run ConvexFX as a full Delta executor.
//! It shows the integration pattern between ConvexFX clearing and Delta runtime.

use convexfx_delta::{ConvexFxExecutor};
use delta_executor_sdk::execution::Execution;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🧪 Testing ConvexFX Delta Executor Integration");

    // Create the ConvexFX executor
    let executor = ConvexFxExecutor::new()?;
    tracing::info!("✅ ConvexFX executor created successfully");

    // Test basic execution
    use delta_verifiable::types::VerifiableType;
    let empty_verifiables: Vec<VerifiableType> = Vec::new();

    match executor.execute(&empty_verifiables) {
        Ok(_results) => {
            tracing::info!("✅ Execution test passed");
        }
        Err(e) => {
            tracing::warn!("⚠️  Execution test failed (expected for empty input): {}", e);
        }
    }

    tracing::info!("🎉 ConvexFX Delta executor integration test complete");
    Ok(())
}