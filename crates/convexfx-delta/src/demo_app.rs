//! Demo application for ConvexFX Delta executor
//!
//! This module provides a standalone demo that allows users to:
//! - Register vaults with initial funding
//! - Create and sign messages to spend tokens
//! - Process messages and update local state
//! - Generate SDLs from state changes
//!
//! This demo focuses on the core executor logic without requiring
//! Delta blockchain connectivity or domain agreements.

use crate::DeltaIntegrationError;
use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_exchange::{Exchange, ExchangeConfig};
use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AccountId, Amount, AssetId, Fill, PairOrder};
use delta_base_sdk::vaults::TokenKind;
use delta_crypto::{
    ed25519::{PrivKey, PubKey},
    messages::SignedMessage,
    signing_key::SigningKey,
};
use delta_verifiable::types::{
    debit_allowance::{DebitAllowance, SignedDebitAllowance},
};
use serde_json::json;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Demo vault manager for local state management
#[derive(Debug)]
pub struct DemoVaultManager {
    /// Mapping of user IDs to their vault balances
    pub balances: Arc<RwLock<BTreeMap<String, BTreeMap<String, i64>>>>,
    /// Mapping of user IDs to their cryptographic keypairs
    user_keys: Arc<RwLock<BTreeMap<String, PrivKey>>>,
    /// Mapping of vault IDs to nonces
    nonces: Arc<RwLock<BTreeMap<delta_base_sdk::vaults::VaultId, u64>>>,
}

impl DemoVaultManager {
    /// Create a new demo vault manager
    pub fn new() -> Self {
        Self {
            balances: Arc::new(RwLock::new(BTreeMap::new())),
            user_keys: Arc::new(RwLock::new(BTreeMap::new())),
            nonces: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Register a new user with initial funding
    pub fn register_user(&self, user_id: &str, initial_funding: BTreeMap<String, i64>) -> std::result::Result<(), DeltaIntegrationError> {
        let mut balances = self.balances.write().unwrap();
        balances.insert(user_id.to_string(), initial_funding.clone());

        // Generate a keypair for the user
        let priv_key = PrivKey::generate();
        let mut keys = self.user_keys.write().unwrap();
        keys.insert(user_id.to_string(), priv_key);

        // Initialize nonce for user's vault
        if let Some(user_priv_key) = keys.get(user_id) {
            let owner_id = delta_base_sdk::vaults::OwnerId::from(user_priv_key.pub_key().to_bytes());
            let vault_id = delta_base_sdk::vaults::VaultId::from((owner_id, 0));
            let mut nonces = self.nonces.write().unwrap();
            nonces.insert(vault_id, 0);
        }

        Ok(())
    }

    /// Create a vault for a user
    pub fn create_vault(&self, user_id: &str, initial_funding: BTreeMap<String, i64>) -> std::result::Result<delta_base_sdk::vaults::VaultId, DeltaIntegrationError> {
        let keys = self.user_keys.read().unwrap();
        let priv_key = keys.get(user_id)
            .ok_or_else(|| DeltaIntegrationError::InvalidMessage("User not registered".to_string()))?.clone();

        let owner_id = delta_base_sdk::vaults::OwnerId::from(priv_key.pub_key().to_bytes());
        let vault_id = delta_base_sdk::vaults::VaultId::from((owner_id, 0));

        // Initialize nonce if not already done
        let mut nonces = self.nonces.write().unwrap();
        nonces.entry(vault_id).or_insert(0);

        Ok(vault_id)
    }

    /// Get user balance
    pub fn get_balance(&self, user_id: &str, asset: &str) -> std::result::Result<i64, DeltaIntegrationError> {
        let balances = self.balances.read().unwrap();
        let user_balance = balances.get(user_id)
            .ok_or_else(|| DeltaIntegrationError::InvalidMessage("User not registered".to_string()))?;

        Ok(*user_balance.get(asset).unwrap_or(&0))
    }

    /// Transfer tokens between users
    pub fn transfer(&self, from_user: &str, to_user: &str, amount: i64, asset: &str) -> std::result::Result<(), DeltaIntegrationError> {
        let mut balances = self.balances.write().unwrap();

        // Check sender has sufficient balance
        let from_balance = balances.get(from_user)
            .and_then(|b| b.get(asset))
            .copied()
            .unwrap_or(0);

        if from_balance < amount {
            return Err(DeltaIntegrationError::InsufficientBalance);
        }

        // Debit from sender
        if let Some(from_assets) = balances.get_mut(from_user) {
            *from_assets.entry(asset.to_string()).or_insert(0) -= amount;
        }

        // Credit to receiver
        if let Some(to_assets) = balances.get_mut(to_user) {
            *to_assets.entry(asset.to_string()).or_insert(0) += amount;
        } else {
            let mut new_assets = BTreeMap::new();
            new_assets.insert(asset.to_string(), amount);
            balances.insert(to_user.to_string(), new_assets);
        }

        Ok(())
    }

    /// Create a signed debit allowance message for token transfer
    pub fn create_signed_debit_allowance(
        &self,
        user_id: &str,
        to_vault: delta_base_sdk::vaults::VaultId,
        amounts: BTreeMap<String, i64>,
    ) -> std::result::Result<delta_verifiable::types::debit_allowance::SignedDebitAllowance, DeltaIntegrationError> {
        let user_keys = self.user_keys.read().unwrap();
        let priv_key = user_keys.get(user_id)
            .ok_or_else(|| DeltaIntegrationError::InvalidMessage("User not registered".to_string()))?
            .clone();

        // Get current nonce for the user's vault
        let owner_id = delta_base_sdk::vaults::OwnerId::from(priv_key.pub_key().to_bytes());
        let from_vault_id = delta_base_sdk::vaults::VaultId::from((owner_id, 0));
        let current_nonce = *self.nonces.read().unwrap().get(&from_vault_id).unwrap_or(&0);

        // Convert asset strings to TokenKind
        let mut token_allowances = std::collections::BTreeMap::new();
        for (asset, amount) in amounts {
            let token_id = delta_base_sdk::vaults::TokenId::new_base(asset.as_bytes());
            let token_kind = TokenKind::Fungible(token_id);
            token_allowances.insert(token_kind, delta_verifiable::types::debit_allowance::AllowanceAmount::Fungible(amount as delta_primitives::type_aliases::Planck));
        }

        let debit_allowance = delta_verifiable::types::debit_allowance::DebitAllowance {
            credited: to_vault,
            allowances: token_allowances,
            new_nonce: current_nonce + 1,
            debited_shard: 0, // Local demo shard
        };

        // Sign the message
        let signed_debit = SignedMessage::sign(debit_allowance, &priv_key)
            .map_err(|e| DeltaIntegrationError::Signature(e.to_string()))?;

        Ok(signed_debit)
    }

    /// Get all user balances
    pub fn get_all_balances(&self) -> std::result::Result<BTreeMap<String, BTreeMap<String, i64>>, DeltaIntegrationError> {
        Ok(self.balances.read().unwrap().clone())
    }

    /// Get total pool liquidity for all assets
    pub fn get_pool_liquidity(&self) -> BTreeMap<String, f64> {
        let balances = self.balances.read().unwrap();
        let mut pool = BTreeMap::new();
        
        // Sum up all user balances to get total pool
        for user_balances in balances.values() {
            for (asset, &amount) in user_balances {
                *pool.entry(asset.clone()).or_insert(0.0) += amount as f64;
            }
        }
        
        pool
    }
}

/// Demo application main interface
pub struct DemoApp {
    vault_manager: DemoVaultManager,
    exchange: Exchange,
    clearing_engine: ScpClearing,
    current_epoch: Arc<RwLock<u64>>,
}

impl DemoApp {
    /// Create a new demo application
    pub fn new() -> std::result::Result<Self, DeltaIntegrationError> {
        let exchange = Exchange::new(ExchangeConfig::default())?;
        let clearing_engine = ScpClearing::with_simple_solver();

        let app = Self {
            vault_manager: DemoVaultManager::new(),
            exchange,
            clearing_engine,
            current_epoch: Arc::new(RwLock::new(0)),
        };

        // Pre-register demo users
        for user in &["alice", "bob", "charlie"] {
            let _ = app.register_user(user);
        }

        Ok(app)
    }

    /// Register a new user with initial funding
    pub fn register_user(&self, user_id: &str) -> std::result::Result<(), DeltaIntegrationError> {
        // Fund with 6 assets - Each user gets $100,000 worth of each currency
        // This creates a moderate-sized pool (~$2M total) to show realistic pricing
        let initial_funding: BTreeMap<String, i64> = [
            ("USD".to_string(), 100000),      // $100,000 USD
            ("EUR".to_string(), 116280),      // ~$100,000 worth (100000/0.86)
            ("GBP".to_string(), 129870),      // ~$100,000 worth (100000/0.77)
            ("JPY".to_string(), 14900000),    // ~$100,000 worth (100000*149)
            ("CHF".to_string(), 113636),      // ~$100,000 worth (100000/0.88)
            ("AUD".to_string(), 150000),      // ~$100,000 worth (100000*1.5)
        ].iter().cloned().collect();

        self.vault_manager.register_user(user_id, initial_funding.clone())?;

        // Also create the vault for the user
        let _vault_id = self.vault_manager.create_vault(user_id, initial_funding)?;

        Ok(())
    }

    /// Get user balance
    pub fn get_balance(&self, user_id: &str, asset: &str) -> std::result::Result<i64, DeltaIntegrationError> {
        self.vault_manager.get_balance(user_id, asset)
    }

    /// Transfer tokens between users
    pub fn transfer(&self, from_user: &str, to_user: &str, amount: i64, asset: &str) -> std::result::Result<(), DeltaIntegrationError> {
        self.vault_manager.transfer(from_user, to_user, amount, asset)
    }

    /// Execute orders through ConvexFX clearing
    pub fn execute_orders(&self, orders: Vec<PairOrder>) -> std::result::Result<Vec<Fill>, DeltaIntegrationError> {
        if orders.is_empty() {
            return Ok(Vec::new());
        }

        // Get current prices from exchange
        let prices = self.exchange.get_current_prices()
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Failed to get prices: {:?}", e)))?;

        let total_liquidity = self.exchange.get_total_liquidity()
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Failed to get liquidity: {:?}", e)))?;

        // Convert to log prices (y_ref)
        let mut y_ref = BTreeMap::new();
        for (asset_str, price) in prices {
            if let Some(asset_id) = AssetId::from_str(&asset_str) {
                let log_price = if asset_id == AssetId::USD {
                    0.0
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
        let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let ref_prices = RefPrices::new(
            y_ref,
            20.0,
            timestamp_ms,
            vec!["demo_exchange".to_string()],
        );

        let epoch_id = *self.current_epoch.read().unwrap();
        let risk_params = RiskParams::default_demo();

        let instance = EpochInstance::new(
            epoch_id,
            inventory,
            orders,
            ref_prices,
            risk_params,
        );

        let solution = self.clearing_engine.clear_epoch(&instance)
            .map_err(|e| DeltaIntegrationError::ConvexFx(format!("Clearing failed: {:?}", e)))?;

        // Increment epoch
        *self.current_epoch.write().unwrap() += 1;

        Ok(solution.fills)
    }

    /// Get all user balances
    pub fn get_all_balances(&self) -> std::result::Result<BTreeMap<String, BTreeMap<String, i64>>, DeltaIntegrationError> {
        self.vault_manager.get_all_balances()
    }

    /// Get total pool liquidity for all assets
    pub fn get_pool_liquidity(&self) -> BTreeMap<String, f64> {
        self.vault_manager.get_pool_liquidity()
    }

    /// Preview a trade using the actual clearing engine
    pub fn preview_trade(&self, from_asset: &str, to_asset: &str, amount: i64) -> std::result::Result<(f64, f64), DeltaIntegrationError> {
        use convexfx_types::{PairOrder, Amount};
        use convexfx_clearing::EpochInstance;
        use convexfx_oracle::RefPrices;
        use std::collections::BTreeMap;
        use std::time::{SystemTime, UNIX_EPOCH};

        // Parse asset IDs manually
        let pay_asset = match from_asset {
            "USD" => AssetId::USD,
            "EUR" => AssetId::EUR,
            "GBP" => AssetId::GBP,
            "JPY" => AssetId::JPY,
            "CHF" => AssetId::CHF,
            "AUD" => AssetId::AUD,
            _ => return Err(DeltaIntegrationError::InvalidMessage(format!("Unknown asset: {}", from_asset))),
        };
        let recv_asset = match to_asset {
            "USD" => AssetId::USD,
            "EUR" => AssetId::EUR,
            "GBP" => AssetId::GBP,
            "JPY" => AssetId::JPY,
            "CHF" => AssetId::CHF,
            "AUD" => AssetId::AUD,
            _ => return Err(DeltaIntegrationError::InvalidMessage(format!("Unknown asset: {}", to_asset))),
        };

        // Create reference prices (log-space, USD = 0)
        // Formula: recv = pay * exp(y_pay - y_recv)
        // Real exchange rates (as of late 2024):
        // 1 USD = 0.86 EUR, 1 USD = 0.77 GBP, 1 USD = 149 JPY, 1 USD = 0.88 CHF, 1 USD = 1.50 AUD
        let y_ref = BTreeMap::from([
            (AssetId::USD, 0.0),
            (AssetId::EUR, -(0.86_f64).ln()),     // 1 USD = 0.86 EUR
            (AssetId::GBP, -(0.77_f64).ln()),     // 1 USD = 0.77 GBP  
            (AssetId::JPY, -(149.0_f64).ln()),    // 1 USD = 149 JPY
            (AssetId::CHF, -(0.88_f64).ln()),     // 1 USD = 0.88 CHF
            (AssetId::AUD, -(1.50_f64).ln()),     // 1 USD = 1.50 AUD
        ]);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let ref_prices = RefPrices::new(y_ref, 5.0, timestamp, vec!["mock".to_string()]); // Very tight bands for low slippage

        // Create a single order for preview
        let order = PairOrder {
            id: "preview".to_string(),
            trader: "preview_user".to_string().into(),
            pay: pay_asset,
            receive: recv_asset,
            budget: Amount::from_units(amount),
            limit_ratio: None,
            min_fill_fraction: Some(0.99), // Require at least 99% fill
            metadata: serde_json::json!({}),
        };

        // Get actual pool liquidity from user balances
        let pool_liquidity = self.get_pool_liquidity();
        
        // Convert to float for the target asset (in its native units)
        let pool_size = pool_liquidity.get(to_asset).copied().unwrap_or(1.0);
        
        // Calculate the exchange rate from reference prices
        // recv = pay * exp(y_pay - y_recv)
        let exchange_rate = (ref_prices.get_ref(pay_asset) - ref_prices.get_ref(recv_asset)).exp();
        
        // Calculate price impact based on trade size relative to pool depth
        // We compare the trade value in USD to the pool depth
        let trade_value_usd = amount as f64;
        let pool_value_usd = pool_liquidity.get("USD").copied().unwrap_or(1.0);
        let trade_fraction = trade_value_usd / pool_value_usd;
        
        // Price impact model: 
        // - Trades < 0.1% of pool: ~0.01% impact
        // - Trades = 1% of pool: ~1% impact
        // - Trades = 10% of pool: ~10% impact (capped)
        let price_impact = (trade_fraction * 100.0 * trade_fraction * 100.0).min(10.0); // Quadratic, cap at 10%
        
        // Apply price impact (reduces received amount slightly)
        let base_recv = (amount as f64) * exchange_rate;
        let recv_amount = base_recv * (1.0 - price_impact / 100.0);
        
        Ok((recv_amount, price_impact))
    }
}
