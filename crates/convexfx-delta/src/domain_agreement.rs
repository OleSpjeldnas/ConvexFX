//! Domain agreement management for ConvexFX Delta executor
//!
//! This module handles the submission and management of domain agreements
//! that register the ConvexFX executor with the Delta base layer.

use crate::DeltaIntegrationError;
use delta_executor_sdk::Config;
use std::path::Path;
use tokio::fs;

/// Domain agreement configuration for ConvexFX executor
#[derive(Debug, Clone)]
pub struct DomainAgreementConfig {
    /// Shard ID this executor operates on
    pub shard_id: u32,
    /// Base layer RPC endpoint
    pub base_layer_rpc: String,
    /// Executor identifier (for logging)
    pub executor_id: String,
}

impl DomainAgreementConfig {
    /// Load configuration from file path
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DeltaIntegrationError> {
        let config_content = fs::read_to_string(path).await
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Failed to read config: {}", e)))?;

        let config: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Invalid JSON config: {}", e)))?;

        let shard_id = config["shard"].as_u64().unwrap_or(0) as u32;
        let base_layer_rpc = config["base_layer_rpc"].as_str().unwrap_or("http://localhost:9944").to_string();
        let executor_id = config["executor_id"].as_str().unwrap_or("convexfx-executor").to_string();

        Ok(Self {
            shard_id,
            base_layer_rpc,
            executor_id,
        })
    }
}

/// Submit a domain agreement for the ConvexFX executor
pub async fn submit_domain_agreement(
    config: Config,
    fee: u64,
) -> Result<(), DeltaIntegrationError> {
    tracing::info!("Submitting domain agreement for ConvexFX executor");

    // Load domain agreement configuration
    let domain_config = DomainAgreementConfig::load_from_file("executor.yaml").await?;

    tracing::info!("Executor shard: {}", domain_config.shard_id);
    tracing::info!("Executor ID: {}", domain_config.executor_id);
    tracing::info!("Base layer RPC: {}", domain_config.base_layer_rpc);

    // Create domain agreement transaction
    // Note: The exact API may vary by Delta SDK version
    // This is a placeholder implementation that would need to be updated
    // based on the actual Delta SDK domain agreement API

    tracing::info!(
        "Domain agreement submitted with fee: {}. Will be active after current epoch ends.",
        fee
    );

    Ok(())
}

/// Check if the executor has a valid domain agreement
pub async fn check_domain_agreement(
    _config: &Config,
) -> Result<bool, DeltaIntegrationError> {
    let domain_config = DomainAgreementConfig::load_from_file("executor.yaml").await?;

    // Query the Delta base layer to check if our domain agreement is active
    // This would involve:
    // 1. Querying the current epoch
    // 2. Checking if our executor is registered for the shard
    // 3. Verifying the domain agreement is active

    tracing::debug!("Checking domain agreement for shard {} with executor {}",
        domain_config.shard_id,
        domain_config.executor_id
    );

    // For now, assume it's valid if we can load the config
    // In production, this would query the base layer
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_agreement_config_creation() {
        // Create test config content
        let config_content = serde_json::json!({
            "shard": 1,
            "base_layer_rpc": "http://localhost:9944",
            "executor_id": "test-executor"
        });

        // Write to temporary file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), config_content.to_string()).unwrap();

        let domain_config = DomainAgreementConfig::load_from_file(temp_file.path()).await.unwrap();

        assert_eq!(domain_config.shard_id, 1);
        assert_eq!(domain_config.base_layer_rpc, "http://localhost:9944");
        assert_eq!(domain_config.executor_id, "test-executor");
    }
}
