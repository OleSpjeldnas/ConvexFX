use crate::{DeltaIntegrationError, Result};
use convexfx_types::{AccountId, AssetId, Amount, PairOrder};
use delta_base_sdk::{
    vaults::OwnerId,
    crypto::Hash256,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Delta-compatible swap message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaSwapMessage {
    /// The vault owner initiating the swap
    pub owner: OwnerId,
    /// Asset being sold
    pub pay_asset: AssetId,
    /// Asset being bought
    pub receive_asset: AssetId,
    /// Amount being sold
    pub budget: Amount,
    /// Optional limit ratio (max receive/pay price)
    pub limit_ratio: Option<f64>,
    /// Optional minimum fill fraction (0.0-1.0)
    pub min_fill_fraction: Option<f64>,
}

impl DeltaSwapMessage {
    /// Create a new swap message
    pub fn new(
        owner: OwnerId,
        pay_asset: AssetId,
        receive_asset: AssetId,
        budget: Amount,
        limit_ratio: Option<f64>,
        min_fill_fraction: Option<f64>,
    ) -> Self {
        Self {
            owner,
            pay_asset,
            receive_asset,
            budget,
            limit_ratio,
            min_fill_fraction,
        }
    }

    /// Convert to ConvexFX PairOrder
    pub fn to_pair_order(&self, order_id: String) -> Result<PairOrder> {
        // Convert OwnerId to AccountId (this is a simplified mapping)
        // In practice, this would need proper mapping logic
        let trader = AccountId::new(format!("delta_{}", self.owner));

        Ok(PairOrder {
            id: order_id,
            trader,
            pay: self.pay_asset,
            receive: self.receive_asset,
            budget: self.budget,
            limit_ratio: self.limit_ratio,
            min_fill_fraction: self.min_fill_fraction,
            metadata: serde_json::json!({
                "delta_owner": self.owner,
                "message_type": "swap"
            }),
        })
    }
}

/// Delta-compatible liquidity provision message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLiquidityMessage {
    /// The vault owner providing liquidity
    pub owner: OwnerId,
    /// Asset for liquidity provision
    pub asset: AssetId,
    /// Amount of liquidity to add (positive) or remove (negative)
    pub amount: Amount,
}

impl DeltaLiquidityMessage {
    /// Create a new liquidity message
    pub fn new(owner: OwnerId, asset: AssetId, amount: Amount) -> Self {
        Self { owner, asset, amount }
    }
}

/// Enum of all Delta-compatible message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DeltaMessage {
    /// Swap operation
    Swap(DeltaSwapMessage),
    /// Liquidity provision/removal
    Liquidity(DeltaLiquidityMessage),
}

impl DeltaMessage {
    /// Create a swap message
    pub fn swap(
        owner: OwnerId,
        pay_asset: AssetId,
        receive_asset: AssetId,
        budget: Amount,
        limit_ratio: Option<f64>,
        min_fill_fraction: Option<f64>,
    ) -> Self {
        Self::Swap(DeltaSwapMessage::new(
            owner,
            pay_asset,
            receive_asset,
            budget,
            limit_ratio,
            min_fill_fraction,
        ))
    }

    /// Create a liquidity message
    pub fn liquidity(owner: OwnerId, asset: AssetId, amount: Amount) -> Self {
        Self::Liquidity(DeltaLiquidityMessage::new(owner, asset, amount))
    }

    /// Get the owner of this message
    pub fn owner(&self) -> &OwnerId {
        match self {
            DeltaMessage::Swap(msg) => &msg.owner,
            DeltaMessage::Liquidity(msg) => &msg.owner,
        }
    }

    /// Convert to ConvexFX order for swap messages
    pub fn to_pair_order(&self, order_id: String) -> Result<PairOrder> {
        match self {
            DeltaMessage::Swap(swap_msg) => swap_msg.to_pair_order(order_id),
            DeltaMessage::Liquidity(_) => Err(DeltaIntegrationError::InvalidMessage(
                "Liquidity messages cannot be converted to pair orders".to_string(),
            )),
        }
    }
}

/// Asset identifier mapping between Delta and ConvexFX
/// In a real implementation, this would be more sophisticated
pub struct AssetMapper;

impl AssetMapper {
    /// Convert Delta asset string to ConvexFX AssetId
    pub fn delta_to_convexfx(asset_str: &str) -> Result<AssetId> {
        // Simple mapping for demo - in reality this would be configurable
        match asset_str.to_uppercase().as_str() {
            "USD" | "USDC" => Ok(AssetId::USD),
            "EUR" | "EURO" => Ok(AssetId::EUR),
            "JPY" | "YEN" => Ok(AssetId::JPY),
            _ => Err(DeltaIntegrationError::AssetNotFound(asset_str.to_string())),
        }
    }

    /// Convert ConvexFX AssetId to Delta asset string
    pub fn convexfx_to_delta(asset_id: AssetId) -> String {
        asset_id.to_string()
    }
}

/// Message factory for creating Delta messages from various inputs
pub struct DeltaMessageFactory;

impl DeltaMessageFactory {
    /// Create a swap message from string parameters
    pub fn create_swap_from_strings(
        owner_str: &str,
        pay_asset_str: &str,
        receive_asset_str: &str,
        budget_str: &str,
        limit_ratio: Option<f64>,
        min_fill_fraction: Option<f64>,
    ) -> Result<DeltaMessage> {
        // Parse owner - simplified for demo
        let owner = OwnerId::from_str(owner_str)
            .map_err(|_| DeltaIntegrationError::InvalidMessage("Invalid owner format".to_string()))?;

        let pay_asset = AssetMapper::delta_to_convexfx(pay_asset_str)?;
        let receive_asset = AssetMapper::delta_to_convexfx(receive_asset_str)?;

        let budget = Amount::from_string(budget_str)
            .map_err(|_| DeltaIntegrationError::InvalidMessage("Invalid budget format".to_string()))?;

        Ok(DeltaMessage::swap(
            owner,
            pay_asset,
            receive_asset,
            budget,
            limit_ratio,
            min_fill_fraction,
        ))
    }

    /// Create a liquidity message from string parameters
    pub fn create_liquidity_from_strings(
        owner_str: &str,
        asset_str: &str,
        amount_str: &str,
    ) -> Result<DeltaMessage> {
        let owner = OwnerId::from_str(owner_str)
            .map_err(|_| DeltaIntegrationError::InvalidMessage("Invalid owner format".to_string()))?;

        let asset = AssetMapper::delta_to_convexfx(asset_str)?;

        let amount = Amount::from_string(amount_str)
            .map_err(|_| DeltaIntegrationError::InvalidMessage("Invalid amount format".to_string()))?;

        Ok(DeltaMessage::liquidity(owner, asset, amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_base_sdk::{vaults::OwnerId, crypto::{ed25519::{PrivKey}}};

    #[test]
    fn test_delta_swap_message_creation() {
        let privkey = PrivKey::generate();
        let owner = OwnerId::from(privkey.pub_key().hash_sha256());
        let pay_asset = AssetId::USD;
        let receive_asset = AssetId::EUR;
        let budget = Amount::from_f64(1000.0).unwrap();

        let message = DeltaSwapMessage::new(owner, pay_asset, receive_asset, budget, Some(1.1), Some(0.5));

        assert_eq!(message.pay_asset, AssetId::USD);
        assert_eq!(message.receive_asset, AssetId::EUR);
        assert_eq!(message.budget.to_f64(), 1000.0);
        assert_eq!(message.limit_ratio, Some(1.1));
        assert_eq!(message.min_fill_fraction, Some(0.5));
    }

    #[test]
    fn test_delta_message_to_pair_order() {
        let owner = OwnerId::from(PrivKey::generate().pub_key().hash_sha256());
        let message = DeltaMessage::swap(
            owner,
            AssetId::USD,
            AssetId::EUR,
            Amount::from_f64(1000.0).unwrap(),
            Some(1.1),
            Some(0.5),
        );

        let order_id = "test_order_1".to_string();
        let pair_order = message.to_pair_order(order_id.clone()).unwrap();

        assert_eq!(pair_order.id, order_id);
        assert_eq!(pair_order.pay, AssetId::USD);
        assert_eq!(pair_order.receive, AssetId::EUR);
        assert_eq!(pair_order.budget.to_f64(), 1000.0);
        assert_eq!(pair_order.limit_ratio, Some(1.1));
        assert_eq!(pair_order.min_fill_fraction, Some(0.5));
    }

    #[test]
    fn test_asset_mapper() {
        assert_eq!(AssetMapper::delta_to_convexfx("USD").unwrap(), AssetId::USD);
        assert_eq!(AssetMapper::delta_to_convexfx("EUR").unwrap(), AssetId::EUR);
        assert_eq!(AssetMapper::convexfx_to_delta(AssetId::USD), "USD");
        assert_eq!(AssetMapper::convexfx_to_delta(AssetId::EUR), "EUR");

        assert!(AssetMapper::delta_to_convexfx("INVALID").is_err());
    }
}
