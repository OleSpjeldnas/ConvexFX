use crate::{DeltaIntegrationError, Result};
use convexfx_types::{AccountId, Amount, AssetId};
use delta_base_sdk::{
    vaults::{OwnerId, VaultId, Vault},
};
use std::collections::BTreeMap;

/// Delta state manager that bridges Delta vault operations with ConvexFX accounts
#[derive(Debug)]
pub struct DeltaStateManager {
    /// Mapping from Delta OwnerId to ConvexFX AccountId
    owner_to_account: BTreeMap<OwnerId, AccountId>,
    /// Mapping from ConvexFX AccountId to Delta OwnerId
    account_to_owner: BTreeMap<AccountId, OwnerId>,
}

impl DeltaStateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        Self {
            owner_to_account: BTreeMap::new(),
            account_to_owner: BTreeMap::new(),
        }
    }

    /// Register a Delta owner with a corresponding ConvexFX account
    pub fn register_owner(&mut self, owner: OwnerId, account: AccountId) {
        self.owner_to_account.insert(owner, account.clone());
        self.account_to_owner.insert(account, owner);
    }

    /// Get ConvexFX account for a Delta owner
    pub fn get_account(&self, owner: &OwnerId) -> Option<&AccountId> {
        self.owner_to_account.get(owner)
    }

    /// Get Delta owner for a ConvexFX account
    pub fn get_owner(&self, account: &AccountId) -> Option<&OwnerId> {
        self.account_to_owner.get(account)
    }

    /// Check if a Delta owner has sufficient balance for a swap
    pub async fn check_swap_balance(
        &self,
        _runtime: &delta_base_sdk::rpc::BaseRpcClient, // Simplified for demo
        owner: &OwnerId,
        asset: AssetId,
        amount: Amount,
    ) -> Result<bool> {
        // For this demo, we'll assume all vaults have sufficient balance
        // In a real implementation, this would query the actual vault
        println!("Checking balance for owner {:?}, asset {:?}, amount {:?}",
                 owner, asset, amount);
        Ok(true)
    }

    /// Get asset balance from a Delta vault
    fn get_asset_balance(&self, vault: &Vault, asset: AssetId) -> Amount {
        // Simplified balance extraction - in reality this would parse vault data properly
        // For now, assume all vaults have some balance in supported assets
        match asset {
            AssetId::USD => Amount::from_f64(10000.0).unwrap(), // Mock balance
            AssetId::EUR => Amount::from_f64(8000.0).unwrap(),  // Mock balance
            AssetId::JPY => Amount::from_f64(1000000.0).unwrap(), // Mock balance
            AssetId::GBP => Amount::from_f64(5000.0).unwrap(),  // Mock balance
            AssetId::CHF => Amount::from_f64(7000.0).unwrap(),  // Mock balance
            AssetId::AUD => Amount::from_f64(6000.0).unwrap(),  // Mock balance
        }
    }

    /// Get mock asset balance for demo purposes
    fn get_mock_asset_balance(&self, asset: AssetId) -> Amount {
        match asset {
            AssetId::USD => Amount::from_f64(10000.0).unwrap(), // Mock balance
            AssetId::EUR => Amount::from_f64(8000.0).unwrap(),  // Mock balance
            AssetId::JPY => Amount::from_f64(1000000.0).unwrap(), // Mock balance
            AssetId::GBP => Amount::from_f64(5000.0).unwrap(),  // Mock balance
            AssetId::CHF => Amount::from_f64(7000.0).unwrap(),  // Mock balance
            AssetId::AUD => Amount::from_f64(6000.0).unwrap(),  // Mock balance
        }
    }

    /// Update vault balance after a trade execution
    pub async fn update_vault_after_trade(
        &self,
        _runtime: &delta_base_sdk::rpc::BaseRpcClient, // Simplified for demo
        owner: &OwnerId,
        pay_asset: AssetId,
        pay_amount: Amount,
        receive_asset: AssetId,
        receive_amount: Amount,
    ) -> Result<()> {
        // In a real implementation, this would:
        // 1. Get current vault state
        // 2. Update balances based on trade execution
        // 3. Submit updated vault state back to Delta

        println!(
            "Updating vault for owner {:?}: -{} {}, +{} {}",
            owner,
            pay_amount.to_f64(),
            pay_asset,
            receive_amount.to_f64(),
            receive_asset
        );

        Ok(())
    }

    /// Get all vault balances for a Delta owner
    pub async fn get_vault_balances(
        &self,
        _runtime: &delta_base_sdk::rpc::BaseRpcClient, // Simplified for demo
        owner: &OwnerId,
    ) -> Result<BTreeMap<AssetId, Amount>> {
        // For this demo, we'll return mock balances
        // In a real implementation, this would query the actual vault
        let mut balances = BTreeMap::new();
        for asset in AssetId::all() {
            balances.insert(*asset, self.get_mock_asset_balance(*asset));
        }

        Ok(balances)
    }
}

impl Default for DeltaStateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Vault operation utilities
pub struct VaultOperations;

impl VaultOperations {
    /// Extract mint information for a token
    pub async fn get_mint_info(
        _runtime: &delta_base_sdk::rpc::BaseRpcClient, // Simplified for demo
        token_id: &str,
    ) -> Result<bool> {
        // In a real implementation, this would convert AssetId to TokenId
        // For now, return true as this is a simplified example
        println!("Getting mint info for token: {}", token_id);
        Ok(true)
    }

    /// Validate that a vault exists and is accessible
    pub async fn validate_vault(
        _runtime: &delta_base_sdk::rpc::BaseRpcClient, // Simplified for demo
        _vault_id: &VaultId,
    ) -> Result<bool> {
        // For demo purposes, assume all vaults are valid
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_base_sdk::{vaults::OwnerId, crypto::ed25519::PubKey, crypto::Hash256};

    #[test]
    fn test_state_manager_registration() {
        let mut manager = DeltaStateManager::new();
        let owner = OwnerId::from(PubKey::generate().hash_sha256());
        let account = AccountId::new("test_account".to_string());

        manager.register_owner(owner, account.clone());

        assert_eq!(manager.get_account(&owner), Some(&account));
        assert_eq!(manager.get_owner(&account), Some(&owner));
    }

    #[test]
    fn test_asset_balance_mock() {
        let manager = DeltaStateManager::new();
        let vault = Vault::default(); // Mock vault

        let usd_balance = manager.get_asset_balance(&vault, AssetId::USD);
        assert_eq!(usd_balance.to_f64(), 10000.0);

        let eur_balance = manager.get_asset_balance(&vault, AssetId::EUR);
        assert_eq!(eur_balance.to_f64(), 8000.0);
    }
}
