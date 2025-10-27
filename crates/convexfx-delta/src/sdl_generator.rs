use crate::{DeltaIntegrationError, Result};
use convexfx_types::{AssetId, Fill, AccountId};
use delta_base_sdk::{
    vaults::{OwnerId, VaultId, TokenKind, TokenId},
    crypto::HashDigest,
};
use delta_crypto::{
    signature::Signature,
    ed25519::{PubKey, SignatureScheme},
    messages::BaseSignedMessage,
};
use delta_primitives::{
    diff::{StateDiff, types::StateDiffOperation},
};
// Simplified SDL generator for demo purposes
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;

/// SDL Generator that converts ConvexFX clearing results to Delta SDL format
#[derive(Debug)]
pub struct SdlGenerator {
    /// Mapping from ConvexFX AccountId to Delta OwnerId
    account_to_owner: BTreeMap<AccountId, OwnerId>,
    /// Mapping from VaultId to current nonce
    vault_nonces: BTreeMap<VaultId, u64>,
    /// Mapping from AssetId to TokenId for Delta
    asset_to_token: BTreeMap<AssetId, TokenId>,
}

impl SdlGenerator {
    /// Create a new SDL generator
    pub fn new() -> Self {
        let mut asset_to_token = BTreeMap::new();
        
        // Initialize asset to token mappings
        asset_to_token.insert(AssetId::USD, TokenId::new_base(b"USD"));
        asset_to_token.insert(AssetId::EUR, TokenId::new_base(b"EUR"));
        asset_to_token.insert(AssetId::JPY, TokenId::new_base(b"JPY"));
        asset_to_token.insert(AssetId::GBP, TokenId::new_base(b"GBP"));
        asset_to_token.insert(AssetId::CHF, TokenId::new_base(b"CHF"));
        asset_to_token.insert(AssetId::AUD, TokenId::new_base(b"AUD"));
        
        Self {
            account_to_owner: BTreeMap::new(),
            vault_nonces: BTreeMap::new(),
            asset_to_token,
        }
    }

    /// Register an account-to-owner mapping
    pub fn register_account(&mut self, account: AccountId, owner: OwnerId) {
        self.account_to_owner.insert(account, owner);
    }

    /// Register a vault with initial nonce
    pub fn register_vault(&mut self, vault_id: VaultId, initial_nonce: u64) {
        self.vault_nonces.insert(vault_id, initial_nonce);
    }

    /// Get the current nonce for a vault
    pub fn get_vault_nonce(&self, vault_id: &VaultId) -> u64 {
        self.vault_nonces.get(vault_id).copied().unwrap_or(0)
    }

    /// Increment vault nonce after a transaction
    pub fn increment_vault_nonce(&mut self, vault_id: &VaultId) -> u64 {
        let current_nonce = self.get_vault_nonce(vault_id);
        let new_nonce = current_nonce + 1;
        self.vault_nonces.insert(*vault_id, new_nonce);
        new_nonce
    }

    /// Get vault ID for an account
    pub fn get_vault_id(&self, account: &AccountId) -> Option<VaultId> {
        self.account_to_owner.get(account).map(|owner| {
            VaultId::from((*owner, 0)) // Using vault index 0 for simplicity
        })
    }

    /// Convert ConvexFX fills to Delta state diffs
    pub fn generate_sdl_from_fills(
        &mut self,
        fills: Vec<Fill>,
        epoch_id: u64,
    ) -> Result<Vec<StateDiff>> {
        let mut state_diffs = Vec::new();

        for fill in fills {
            let fill_diffs = self.fill_to_state_diffs(&fill)?;
            state_diffs.extend(fill_diffs);
        }

        Ok(state_diffs)
    }

    /// Convert a single ConvexFX fill to Delta state diffs
    /// A fill represents a trade between two assets, so we need to create
    /// two state diffs: one to debit the pay asset and one to credit the receive asset
    fn fill_to_state_diffs(&mut self, fill: &Fill) -> Result<Vec<StateDiff>> {
        let mut state_diffs = Vec::new();

        // Get the vault ID for the trader
        let vault_id = self.get_vault_id(&fill.trader)
            .ok_or_else(|| DeltaIntegrationError::InvalidMessage(
                format!("No vault found for trader: {}", fill.trader)
            ))?;

        // Get current nonce and increment it
        let new_nonce = self.increment_vault_nonce(&vault_id);

        // Create token diffs for the trade
        let mut token_diffs = BTreeMap::new();

        // Debit the pay asset
        let pay_token_id = self.asset_to_token.get(&fill.pay_asset)
            .ok_or_else(|| DeltaIntegrationError::AssetNotFound(
                format!("Token not found for asset: {:?}", fill.pay_asset)
            ))?;
        let pay_token_kind = TokenKind::Fungible(*pay_token_id);
        let pay_amount_planck = delta_primitives::type_aliases::Planck::from(fill.pay_units as u64);
        token_diffs.insert(pay_token_kind, -pay_amount_planck);

        // Credit the receive asset
        let recv_token_id = self.asset_to_token.get(&fill.recv_asset)
            .ok_or_else(|| DeltaIntegrationError::AssetNotFound(
                format!("Token not found for asset: {:?}", fill.recv_asset)
            ))?;
        let recv_token_kind = TokenKind::Fungible(*recv_token_id);
        let recv_amount_planck = delta_primitives::type_aliases::Planck::from(fill.recv_units as u64);
        token_diffs.insert(recv_token_kind, recv_amount_planck);

        // Create the state diff
        let state_diff = StateDiff {
            vault_id,
            new_nonce: Some(new_nonce),
            operation: StateDiffOperation::TokenDiffs(token_diffs),
        };

        state_diffs.push(state_diff);
        Ok(state_diffs)
    }

    /// Convert ConvexFX AssetId to Delta asset identifier
    fn asset_id_to_delta(&self, asset_id: &AssetId) -> Result<String> {
        // Simple mapping - in reality this would be more sophisticated
        Ok(match asset_id {
            AssetId::USD => "USD".to_string(),
            AssetId::EUR => "EUR".to_string(),
            AssetId::JPY => "JPY".to_string(),
            AssetId::GBP => "GBP".to_string(),
            AssetId::CHF => "CHF".to_string(),
            AssetId::AUD => "AUD".to_string(),
        })
    }

    /// Generate SDL hash (simplified implementation)
    pub fn calculate_sdl_hash(&self, state_diffs: &[StateDiff]) -> Result<HashDigest> {
        // In a real implementation, this would properly hash the SDL content
        // For now, return a mock hash based on the number of diffs
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&state_diffs.len(), &mut hasher);
        Ok(HashDigest::default()) // Simplified for demo
    }

    /// Validate state diffs before submission
    pub fn validate_state_diffs(&self, state_diffs: &[StateDiff]) -> Result<()> {
        for diff in state_diffs {
            // Validate vault exists
            if !self.vault_nonces.contains_key(&diff.vault_id) {
                return Err(DeltaIntegrationError::InvalidMessage(
                    format!("Vault not found: {:?}", diff.vault_id)
                ));
            }

            // Validate nonce is properly set
            if diff.new_nonce.is_none() {
                return Err(DeltaIntegrationError::InvalidMessage(
                    "State diff missing new_nonce".to_string()
                ));
            }

            // Validate token diffs are not empty
            match &diff.operation {
                StateDiffOperation::TokenDiffs(token_diffs) => {
                    if token_diffs.is_empty() {
                        return Err(DeltaIntegrationError::InvalidMessage(
                            "Token diffs cannot be empty".to_string()
                        ));
                    }
                }
                _ => {
                    return Err(DeltaIntegrationError::InvalidMessage(
                        "Unsupported state diff operation".to_string()
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for SdlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// SDL batch processor for handling multiple fills
pub struct SdlBatchProcessor {
    generator: SdlGenerator,
    batch_size: usize,
}

impl SdlBatchProcessor {
    /// Create a new batch processor
    pub fn new(batch_size: usize) -> Self {
        Self {
            generator: SdlGenerator::new(),
            batch_size,
        }
    }

    /// Process a batch of fills into state diffs
    pub fn process_batch(
        &mut self,
        fills: Vec<Fill>,
        epoch_id: u64,
    ) -> Result<Vec<StateDiff>> {
        let mut all_state_diffs = Vec::new();
        let mut current_batch = Vec::new();

        for fill in fills {
            current_batch.push(fill);

            if current_batch.len() >= self.batch_size {
                let state_diffs = self.generator.generate_sdl_from_fills(current_batch, epoch_id)?;
                all_state_diffs.extend(state_diffs);
                current_batch = Vec::new();
            }
        }

        // Process remaining fills
        if !current_batch.is_empty() {
            let state_diffs = self.generator.generate_sdl_from_fills(current_batch, epoch_id)?;
            all_state_diffs.extend(state_diffs);
        }

        // Validate all state diffs
        self.generator.validate_state_diffs(&all_state_diffs)?;

        Ok(all_state_diffs)
    }

    /// Register an account with the generator
    pub fn register_account(&mut self, account: AccountId, owner: OwnerId) {
        self.generator.register_account(account, owner);
    }

    /// Register a vault with the generator
    pub fn register_vault(&mut self, vault_id: VaultId, initial_nonce: u64) {
        self.generator.register_vault(vault_id, initial_nonce);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_types::{AccountId, Amount};
    use delta_base_sdk::{vaults::{OwnerId as DeltaOwnerId}, crypto::ed25519::PubKey, crypto::Hash256};

    #[test]
    fn test_sdl_generator_creation() {
        let generator = SdlGenerator::new();
        assert!(generator.account_to_owner.is_empty());
        assert!(generator.vault_nonces.is_empty());
        assert_eq!(generator.asset_to_token.len(), 6); // All 6 assets
    }

    #[test]
    fn test_account_registration() {
        let mut generator = SdlGenerator::new();
        let account = AccountId::new("test_account".to_string());
        let owner = OwnerId::from(PubKey::generate().hash_sha256());

        generator.register_account(account.clone(), owner);
        assert_eq!(generator.get_vault_id(&account), Some(VaultId::from((owner, 0))));
    }

    #[test]
    fn test_vault_nonce_management() {
        let mut generator = SdlGenerator::new();
        let vault_id = VaultId::from((OwnerId::from(PubKey::generate().hash_sha256()), 0));

        // Initial nonce should be 0
        assert_eq!(generator.get_vault_nonce(&vault_id), 0);

        // Register vault with nonce 5
        generator.register_vault(vault_id, 5);
        assert_eq!(generator.get_vault_nonce(&vault_id), 5);

        // Increment nonce
        let new_nonce = generator.increment_vault_nonce(&vault_id);
        assert_eq!(new_nonce, 6);
        assert_eq!(generator.get_vault_nonce(&vault_id), 6);
    }

    #[test]
    fn test_fill_to_state_diffs() {
        let mut generator = SdlGenerator::new();
        let account = AccountId::new("trader".to_string());
        let owner = OwnerId::from(PubKey::generate().hash_sha256());
        let vault_id = VaultId::from((owner, 0));

        // Register account and vault
        generator.register_account(account.clone(), owner);
        generator.register_vault(vault_id, 0);

        // Create a test fill
        let fill = Fill {
            order_id: "test_order".to_string(),
            trader: account,
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        // Convert fill to state diffs
        let state_diffs = generator.fill_to_state_diffs(&fill).unwrap();
        assert_eq!(state_diffs.len(), 1);

        let state_diff = &state_diffs[0];
        assert_eq!(state_diff.vault_id, vault_id);
        assert_eq!(state_diff.new_nonce, Some(1)); // Should be incremented from 0

        // Check token diffs
        match &state_diff.operation {
            StateDiffOperation::TokenDiffs(token_diffs) => {
                assert_eq!(token_diffs.len(), 2); // Pay and receive assets
                
                // Check USD debit (negative amount)
                let usd_token = TokenKind::Fungible(TokenId::new_base(b"USD"));
                assert_eq!(token_diffs.get(&usd_token), Some(&(-1000 as delta_primitives::type_aliases::Planck)));
                
                // Check EUR credit (positive amount)
                let eur_token = TokenKind::Fungible(TokenId::new_base(b"EUR"));
                assert_eq!(token_diffs.get(&eur_token), Some(&(900 as delta_primitives::type_aliases::Planck)));
            }
            _ => panic!("Expected TokenDiffs operation"),
        }
    }

    #[test]
    fn test_state_diffs_validation() {
        let generator = SdlGenerator::new();
        let vault_id = VaultId::from((OwnerId::from(PubKey::generate().hash_sha256()), 0));

        // Create a valid state diff
        let mut token_diffs = BTreeMap::new();
        token_diffs.insert(TokenKind::Fungible(TokenId::new_base(b"USD")), 1000 as delta_primitives::type_aliases::Planck);
        
        let valid_diff = StateDiff {
            vault_id,
            new_nonce: Some(1),
            operation: StateDiffOperation::TokenDiffs(token_diffs),
        };

        // Should fail validation because vault is not registered
        assert!(generator.validate_state_diffs(&[valid_diff]).is_err());
    }

    #[test]
    fn test_batch_processor() {
        let mut processor = SdlBatchProcessor::new(2);
        let account = AccountId::new("trader".to_string());
        let owner = OwnerId::from(PubKey::generate().hash_sha256());
        let vault_id = VaultId::from((owner, 0));

        // Register account and vault
        processor.register_account(account.clone(), owner);
        processor.register_vault(vault_id, 0);

        // Create test fills
        let fills = vec![
            Fill {
                order_id: "order1".to_string(),
                trader: account.clone(),
                fill_frac: 1.0,
                pay_asset: AssetId::USD,
                recv_asset: AssetId::EUR,
                pay_units: 1000.0,
                recv_units: 900.0,
                fees_paid: BTreeMap::new(),
            },
            Fill {
                order_id: "order2".to_string(),
                trader: account.clone(),
                fill_frac: 0.5,
                pay_asset: AssetId::EUR,
                recv_asset: AssetId::JPY,
                pay_units: 900.0,
                recv_units: 100000.0,
                fees_paid: BTreeMap::new(),
            },
        ];

        let state_diffs = processor.process_batch(fills, 1).unwrap();
        assert_eq!(state_diffs.len(), 2); // One state diff per fill
    }
}
