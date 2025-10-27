use crate::{DeltaIntegrationError, Result};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_types::{AccountId, Amount, AssetId, Fill, PairOrder};
use delta_base_sdk::{
    vaults::{OwnerId},
};
use serde_json;
use std::{collections::BTreeMap, num::NonZero};

/// Delta runtime adapter that uses ConvexFX as the execution engine
pub struct ConvexFxDeltaAdapter {
    /// The underlying ConvexFX exchange
    exchange: Exchange,
    /// Mapping between Delta owners and ConvexFX accounts
    state_manager: crate::state::DeltaStateManager,
    /// SDL generator for creating Delta state diffs
    sdl_generator: crate::sdl_generator::SdlGenerator,
}

impl ConvexFxDeltaAdapter {
    /// Create a new adapter with the given ConvexFX exchange
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            state_manager: crate::state::DeltaStateManager::new(),
            sdl_generator: crate::sdl_generator::SdlGenerator::new(),
        }
    }

    /// Register a Delta owner with a ConvexFX account
    pub fn register_owner(&mut self, owner: OwnerId, account: AccountId) {
        self.state_manager.register_owner(owner, account.clone());
        self.sdl_generator.register_account(account, owner);
    }

    /// Process Delta verifiable messages through ConvexFX execution
    pub async fn process_messages(
        &mut self,
        _messages: Vec<delta_verifiable::types::VerifiableType>,
    ) -> Result<Vec<delta_verifiable::types::VerifiableWithDiffs>> {
        // For this simplified example, we'll create some mock orders
        // In a real implementation, this would convert Delta messages to ConvexFX orders
        let mut orders = Vec::new();

        // Create a sample order for demonstration
        orders.push(PairOrder {
            id: "delta_demo_order".to_string(),
            trader: AccountId::new("delta_user".to_string()),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_f64(1000.0).unwrap(),
            limit_ratio: Some(1.1),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({
                "source": "delta_integration"
            }),
        });

        // Execute orders through ConvexFX batch processing
        let fills = self.execute_batch(orders).await?;

        // Generate SDL from fills using the SDL generator
        let epoch_id = 1; // Would be properly managed in real implementation
        let sdl = self.sdl_generator.generate_sdl_from_fills(fills, epoch_id)?;

        // Validate the SDL before returning
        self.sdl_generator.validate_sdl(&sdl)?;

        Ok(vec![sdl])
    }

    /// Process a single liquidity operation
    async fn process_liquidity_operation(
        &mut self,
        _liquidity_msg: &str, // Simplified for demo - would be proper type
    ) -> Result<()> {
        // In a real implementation, this would:
        // 1. Extract owner, asset, and amount from the liquidity message
        // 2. Call exchange.add_liquidity() or exchange.remove_liquidity()
        // 3. Update vault balances accordingly

        println!("Processing liquidity operation (mock implementation)");
        Ok(())
    }

    /// Execute a batch of orders through ConvexFX
    pub async fn execute_batch(&mut self, orders: Vec<PairOrder>) -> Result<Vec<Fill>> {
        use convexfx_clearing::{EpochInstance, ScpClearing};
        use std::collections::BTreeMap;
        use std::str::FromStr;
        
        if orders.is_empty() {
            println!("⚠️  No orders to execute");
            return Ok(Vec::new());
        }
        
        println!("📦 Processing {} orders through ConvexFX clearing...", orders.len());
        
        // Get current prices from the exchange oracle
        let prices = self.exchange.get_current_prices()
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Failed to get prices: {:?}", e)))?;
        
        // Convert linear prices to log-prices (y_ref)
        let mut y_ref = BTreeMap::new();
        for (asset_str, price) in prices {
            if let Some(asset_id) = AssetId::from_str(&asset_str) {
                let log_price = if asset_id == AssetId::USD {
                    0.0 // USD is numeraire
                } else {
                    price.ln()
                };
                y_ref.insert(asset_id, log_price);
            }
        }
        
        // Get current liquidity inventory
        let total_liquidity = self.exchange.get_total_liquidity()
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Failed to get liquidity: {:?}", e)))?;
        
        let mut inventory = BTreeMap::new();
        for (asset_str, amount) in total_liquidity {
            if let Some(asset_id) = AssetId::from_str(&asset_str) {
                inventory.insert(asset_id, amount);
            }
        }
        
        // Create epoch instance with the actual orders
        let epoch_id = 1; // In real implementation, would track this properly
        let risk_params = convexfx_risk::RiskParams::default_demo();
        
        // Create RefPrices object with log-prices
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let ref_prices_obj = convexfx_oracle::RefPrices::new(
            y_ref,
            20.0, // band_bps: ±20 basis points
            timestamp_ms,
            vec!["delta_exchange".to_string()],
        );
        
        let instance = EpochInstance::new(
            epoch_id,
            inventory,
            orders.clone(),
            ref_prices_obj,
            risk_params,
        );
        
        // Run SCP clearing algorithm
        let clearing_engine = ScpClearing::with_simple_solver();
        let clearing_result = clearing_engine.clear_epoch(&instance)
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Clearing failed: {:?}", e)))?;
        
        println!("✅ Clearing complete: {} fills generated", clearing_result.fills.len());
        
        // Print summary of fills
        if !clearing_result.fills.is_empty() {
            println!("\n📊 Fill Summary:");
            for fill in &clearing_result.fills {
                let rate = fill.recv_units / fill.pay_units;
                println!("   {} ({:.1}% filled): {:.2} {} → {:.2} {} @ {:.4}",
                         fill.order_id,
                         fill.fill_frac * 100.0,
                         fill.pay_units,
                         fill.pay_asset,
                         fill.recv_units,
                         fill.recv_asset,
                         rate);
            }
            println!();
        }
        
        Ok(clearing_result.fills)
    }

}

/// Factory for creating Delta runtime with ConvexFX execution engine
pub struct DeltaRuntimeFactory;

impl DeltaRuntimeFactory {
    /// Create a ConvexFX execution engine
    pub fn create_execution_engine(exchange: Exchange) -> ConvexFxExecutionEngine {
        // Create the execution engine wrapper
        ConvexFxExecutionEngine::new(exchange)
    }
}

/// ConvexFX execution engine that implements Delta's Execution trait
pub struct ConvexFxExecutionEngine {
    exchange: Exchange,
}

impl ConvexFxExecutionEngine {
    /// Create a new execution engine with the given ConvexFX exchange
    pub fn new(exchange: Exchange) -> Self {
        Self { exchange }
    }
}

// Note: The Execution trait implementation is simplified for this example
// In a real implementation, this would properly implement the Delta Execution trait
impl ConvexFxExecutionEngine {
    /// Execute messages using ConvexFX (simplified implementation)
    pub async fn execute_messages(
        &mut self,
        messages: Vec<delta_verifiable::types::VerifiableType>,
    ) -> Result<Vec<delta_verifiable::types::VerifiableWithDiffs>> {
        // For this simplified example, we'll create a new exchange
        // In a real implementation, this would be handled more efficiently
        let exchange = Exchange::new(ExchangeConfig::default())?;
        let mut adapter = ConvexFxDeltaAdapter::new(exchange);
        adapter.process_messages(messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_convexfx_execution_engine_creation() {
        let exchange = Exchange::new(ExchangeConfig::default()).unwrap();
        let engine = ConvexFxExecutionEngine::new(exchange);

        // Test that the engine can be created
        let _ = engine; // creation succeeded
    }

    #[test]
    fn test_delta_runtime_adapter_creation() {
        let exchange = Exchange::new(ExchangeConfig::default()).unwrap();
        let adapter = ConvexFxDeltaAdapter::new(exchange);

        let _ = adapter; // creation succeeded
    }
}
