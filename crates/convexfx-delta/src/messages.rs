use crate::{DeltaIntegrationError, Result};
use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use delta_base_sdk::vaults::OwnerId;
use serde::{Deserialize, Serialize};

/// Delta message types for integration with ConvexFX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaMessage {
    /// The Delta owner (vault owner)
    pub owner: OwnerId,
    /// The asset being paid/sold
    pub pay_asset: AssetId,
    /// The asset being received/bought
    pub receive_asset: AssetId,
    /// The budget amount in the pay asset
    pub budget: Amount,
    /// Optional limit ratio (max price)
    pub limit_ratio: Option<f64>,
    /// Optional minimum fill fraction
    pub min_fill_fraction: Option<f64>,
}

impl DeltaMessage {
    /// Create a new swap message
    pub fn swap(
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

    /// Convert to a ConvexFX PairOrder
    pub fn to_pair_order(&self, order_id: String) -> Result<PairOrder> {
        // Create a default trader account from the owner
        // In a real implementation, this would use proper account mapping
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
                "source": "delta_message",
                "owner": self.owner.to_string(),
            }),
        })
    }
}

/// Asset mapping between ConvexFX and Delta Network
pub struct AssetMapper;

impl AssetMapper {
    /// Convert a ConvexFX AssetId to a Delta asset identifier
    pub fn convexfx_to_delta(asset: AssetId) -> String {
        match asset {
            AssetId::USD => "USD".to_string(),
            AssetId::EUR => "EUR".to_string(),
            AssetId::JPY => "JPY".to_string(),
            AssetId::GBP => "GBP".to_string(),
            AssetId::CHF => "CHF".to_string(),
            AssetId::AUD => "AUD".to_string(),
        }
    }

    /// Convert a Delta asset identifier to a ConvexFX AssetId
    pub fn delta_to_convexfx(asset: &str) -> Result<AssetId> {
        match asset.to_uppercase().as_str() {
            "USD" => Ok(AssetId::USD),
            "EUR" => Ok(AssetId::EUR),
            "JPY" => Ok(AssetId::JPY),
            "GBP" => Ok(AssetId::GBP),
            "CHF" => Ok(AssetId::CHF),
            "AUD" => Ok(AssetId::AUD),
            _ => Err(DeltaIntegrationError::AssetNotFound(format!(
                "Unknown asset: {}",
                asset
            ))),
        }
    }

    /// Get the Delta token identifier for a ConvexFX asset
    pub fn get_token_id(asset: AssetId) -> String {
        Self::convexfx_to_delta(asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_base_sdk::crypto::{ed25519::PrivKey, Hash256};

    #[test]
    fn test_delta_message_creation() {
        let privkey = PrivKey::generate();
        let pubkey = privkey.pub_key();
        let owner = OwnerId::from(pubkey.hash_sha256());

        let msg = DeltaMessage::swap(
            owner,
            AssetId::USD,
            AssetId::EUR,
            Amount::from_f64(1000.0).unwrap(),
            Some(1.1),
            Some(0.5),
        );

        assert_eq!(msg.pay_asset, AssetId::USD);
        assert_eq!(msg.receive_asset, AssetId::EUR);
        assert_eq!(msg.limit_ratio, Some(1.1));
        assert_eq!(msg.min_fill_fraction, Some(0.5));
    }

    #[test]
    fn test_message_to_pair_order() {
        let privkey = PrivKey::generate();
        let pubkey = privkey.pub_key();
        let owner = OwnerId::from(pubkey.hash_sha256());

        let msg = DeltaMessage::swap(
            owner,
            AssetId::USD,
            AssetId::EUR,
            Amount::from_f64(1000.0).unwrap(),
            Some(1.1),
            Some(0.5),
        );

        let order = msg.to_pair_order("test_order".to_string()).unwrap();
        assert_eq!(order.id, "test_order");
        assert_eq!(order.pay, AssetId::USD);
        assert_eq!(order.receive, AssetId::EUR);
        assert_eq!(order.limit_ratio, Some(1.1));
    }

    #[test]
    fn test_asset_mapper_convexfx_to_delta() {
        assert_eq!(AssetMapper::convexfx_to_delta(AssetId::USD), "USD");
        assert_eq!(AssetMapper::convexfx_to_delta(AssetId::EUR), "EUR");
        assert_eq!(AssetMapper::convexfx_to_delta(AssetId::JPY), "JPY");
    }

    #[test]
    fn test_asset_mapper_delta_to_convexfx() {
        assert_eq!(AssetMapper::delta_to_convexfx("USD").unwrap(), AssetId::USD);
        assert_eq!(AssetMapper::delta_to_convexfx("EUR").unwrap(), AssetId::EUR);
        assert_eq!(AssetMapper::delta_to_convexfx("JPY").unwrap(), AssetId::JPY);
        
        // Test case insensitivity
        assert_eq!(AssetMapper::delta_to_convexfx("usd").unwrap(), AssetId::USD);
        assert_eq!(AssetMapper::delta_to_convexfx("eur").unwrap(), AssetId::EUR);
    }

    #[test]
    fn test_asset_mapper_invalid_asset() {
        assert!(AssetMapper::delta_to_convexfx("INVALID").is_err());
        assert!(AssetMapper::delta_to_convexfx("BTC").is_err());
    }
}

