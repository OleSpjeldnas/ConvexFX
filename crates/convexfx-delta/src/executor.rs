//! Delta Executor implementation using ConvexFX as the execution engine
//!
//! This module implements the `Execution` trait from `delta_executor_sdk`,
//! allowing ConvexFX to run as a full Delta executor with proving, SDL
//! submission, and domain agreement management.

use crate::DeltaIntegrationError;
use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use delta_base_sdk::vaults::TokenKind;
use delta_crypto::{
    ed25519,
    messages::SignedMessage,
    signing_key::SigningKey,
};
use delta_executor_sdk::execution::Execution;
use delta_primitives::diff::types;
use delta_verifiable::types::{
    debit_allowance::{AllowanceAmount, DebitAllowance, SignedDebitAllowance},
    fungible::{Operation as VerifiableFungibleOperation, SignedMint},
    nft::{Operation as VerifiableNftOperation, SignedMint as SignedNftMint},
    VerifiableWithDiffs,
};
use serde_json::json;
use snafu::Snafu;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Error type for ConvexFX executor
#[derive(Debug, Snafu)]
pub enum ConvexFxExecutorError {
    #[snafu(display("Failed to execute clearing: {}", message))]
    ClearingFailed { message: String },
    
    #[snafu(display("Invalid verifiable message: {}", message))]
    InvalidVerifiable { message: String },
    
    #[snafu(display("Asset not found: {}", asset))]
    AssetNotFound { asset: String },
    
    #[snafu(display("Exchange error: {}", message))]
    ExchangeError { message: String },
    
    #[snafu(display("State diff generation failed: {}", message))]
    StateDiffError { message: String },
    
    #[snafu(display("Owner mapping failed: no account found for owner"))]
    OwnerMappingError,
    
    #[snafu(display("Insufficient liquidity for order execution"))]
    InsufficientLiquidity,
    
    #[snafu(display("Order validation failed: {}", message))]
    OrderValidationError { message: String },
}

/// ConvexFX execution engine for Delta
///
/// This implements the Delta `Execution` trait, allowing ConvexFX to process
/// verifiable messages (swaps, liquidity operations) and generate state diffs
/// that are proven and submitted to the Delta base layer.
pub struct ConvexFxExecutor {
    /// The underlying ConvexFX exchange
    exchange: Arc<RwLock<Exchange>>,
    /// Current epoch counter
    current_epoch: Arc<RwLock<u64>>,
    /// SCP clearing engine
    clearing_engine: ScpClearing,
    /// Risk parameters for clearing
    risk_params: RiskParams,
}

impl ConvexFxExecutor {
    /// Create a new ConvexFX executor with default configuration
    pub fn new() -> std::result::Result<Self, ConvexFxExecutorError> {
        let exchange = Exchange::new(ExchangeConfig::default())
            .map_err(|e| ConvexFxExecutorError::ExchangeError { 
                message: format!("{:?}", e) 
            })?;
        
        let clearing_engine = ScpClearing::with_simple_solver();
        let risk_params = RiskParams::default_demo();
        
        Ok(Self {
            exchange: Arc::new(RwLock::new(exchange)),
            current_epoch: Arc::new(RwLock::new(0)),
            clearing_engine,
            risk_params,
        })
    }

    /// Execute a batch of orders through ConvexFX clearing
    fn execute_clearing_batch(
        &self,
        orders: Vec<PairOrder>,
    ) -> std::result::Result<Vec<convexfx_types::Fill>, ConvexFxExecutorError> {
        if orders.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!("Processing {} orders through ConvexFX clearing", orders.len());

        // Get current state from exchange
        let exchange = self.exchange.read().unwrap();
        let prices = exchange.get_current_prices()
            .map_err(|e| ConvexFxExecutorError::ClearingFailed {
                message: format!("Failed to get prices: {:?}", e)
            })?;

        let total_liquidity = exchange.get_total_liquidity()
            .map_err(|e| ConvexFxExecutorError::ClearingFailed {
                message: format!("Failed to get liquidity: {:?}", e)
            })?;
        drop(exchange);

        // Convert to log prices (y_ref)
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

        // Convert liquidity inventory
        let mut inventory = BTreeMap::new();
        for (asset_str, amount) in total_liquidity {
            if let Some(asset_id) = AssetId::from_str(&asset_str) {
                inventory.insert(asset_id, amount);
            }
        }

        // Create RefPrices object
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let ref_prices = RefPrices::new(
            y_ref,
            20.0, // ±20 basis points
            timestamp_ms,
            vec!["delta_exchange".to_string()],
        );

        // Get current epoch
        let epoch_id = *self.current_epoch.read().unwrap();

        // Create epoch instance
        let instance = EpochInstance::new(
            epoch_id,
            inventory,
            orders,
            ref_prices,
            self.risk_params.clone(),
        );

        // Run clearing
        let solution = self.clearing_engine.clear_epoch(&instance)
            .map_err(|e| ConvexFxExecutorError::ClearingFailed {
                message: format!("Clearing failed: {:?}", e)
            })?;

        tracing::info!("Clearing complete: {} fills generated", solution.fills.len());

        Ok(solution.fills)
    }
}

impl Default for ConvexFxExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create default executor")
    }
}

impl ConvexFxExecutor {
    /// Process a DebitAllowance message for order settlement
    fn process_debit_allowance_for_settlement(
        &self,
        signed_debit: &SignedDebitAllowance,
    ) -> std::result::Result<Vec<delta_primitives::diff::StateDiff>, ConvexFxExecutorError> {
        tracing::info!("Processing debit allowance for settlement");

        // For demo purposes, return empty state diffs
        // In a real implementation, this would convert the debit allowance to proper state diffs
        Ok(Vec::new())
    }

    /// Process a FungibleTokenMint message for rewards/liquidity
    fn process_fungible_token_mint_for_rewards(
        &self,
        signed_mint: &SignedMint,
    ) -> std::result::Result<Vec<delta_primitives::diff::StateDiff>, ConvexFxExecutorError> {
        tracing::info!("Processing fungible token mint for rewards");

        // For demo purposes, return empty state diffs
        // In a real implementation, this would convert the mint operation to proper state diffs
        Ok(Vec::new())
    }

    /// Process an NftMint message for position tokens
    fn process_nft_mint_for_positions(
        &self,
        signed_nft_mint: &SignedNftMint,
    ) -> std::result::Result<Vec<delta_primitives::diff::StateDiff>, ConvexFxExecutorError> {
        tracing::info!("Processing NFT mint for positions");

        // For demo purposes, return empty state diffs
        // In a real implementation, this would convert the NFT mint operation to proper state diffs
        Ok(Vec::new())
    }
}

/// Implementation of the Delta Execution trait
///
/// This is the core integration point - Delta SDK calls `execute` with
/// verified messages, and ConvexFX processes them through its clearing engine.
impl Execution for ConvexFxExecutor {
    type Error = ConvexFxExecutorError;

    /// Execute a batch of verifiable messages
    ///
    /// This method demonstrates the ConvexFX integration pattern.
    /// In a full implementation, this would:
    /// 1. Parse verifiable messages to extract swap/liquidity operations
    /// 2. Convert them to ConvexFX orders
    /// 3. Run the SCP clearing algorithm
    /// 4. Generate state diffs from the clearing results
    /// 5. Return the results for proving and submission
    fn execute(
        &self,
        verifiables: &[delta_verifiable::types::VerifiableType],
    ) -> std::result::Result<Vec<delta_verifiable::types::VerifiableWithDiffs>, Self::Error> {
        tracing::info!("ConvexFX Delta executor processing {} verifiable messages", verifiables.len());

        let mut results = Vec::new();

        for (i, verifiable) in verifiables.iter().enumerate() {
            match verifiable {
                delta_verifiable::types::VerifiableType::DebitAllowance(signed_debit) => {
                    tracing::info!("Processing DebitAllowance message {}", i);

                    // DebitAllowance transfers tokens from debited vault to credited vault
                    // In a DEX context, this is used for order settlement
                    let state_diffs = self.process_debit_allowance_for_settlement(signed_debit)?;

                    results.push(delta_verifiable::types::VerifiableWithDiffs {
                        verifiable: verifiable.clone(),
                        state_diffs: Vec::new(),
                    });
                }

                delta_verifiable::types::VerifiableType::FungibleTokenMint(signed_mint) => {
                    tracing::info!("Processing FungibleTokenMint message {}", i);

                    // FungibleTokenMint creates or increases token supply
                    // In a DEX context, this could be for liquidity rewards
                    let state_diffs = self.process_fungible_token_mint_for_rewards(&signed_mint)?;

                    results.push(delta_verifiable::types::VerifiableWithDiffs {
                        verifiable: verifiable.clone(),
                        state_diffs: Vec::new(),
                    });
                }

                delta_verifiable::types::VerifiableType::NftMint(signed_nft_mint) => {
                    tracing::info!("Processing NftMint message {}", i);

                    // NFT minting for position tokens, governance, etc.
                    let state_diffs = self.process_nft_mint_for_positions(&signed_nft_mint)?;

                    results.push(delta_verifiable::types::VerifiableWithDiffs {
                        verifiable: verifiable.clone(),
                        state_diffs: Vec::new(),
                    });
                }
            }
        }

        tracing::info!("ConvexFX execution complete: generated {} verifiable results", results.len());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = ConvexFxExecutor::new();
        assert!(executor.is_ok());
    }

    #[test]
    fn test_empty_execution() {
        let executor = ConvexFxExecutor::new().unwrap();
        use delta_verifiable::types::VerifiableType;
        let empty_verifiables: Vec<VerifiableType> = Vec::new();
        let result = executor.execute(&empty_verifiables);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
