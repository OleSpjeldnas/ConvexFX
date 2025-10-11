use crate::{AccountId, Amount, AssetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Order identifier
pub type OrderId = String;

/// Pair order: pay j_k to receive i_k
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairOrder {
    pub id: OrderId,
    pub trader: AccountId,
    pub pay: AssetId,            // j_k (asset being sold)
    pub receive: AssetId,        // i_k (asset being bought)
    pub budget: Amount,          // B_k (units of pay asset)
    pub limit_ratio: Option<f64>, // optional max p_i/p_j
    pub min_fill_fraction: Option<f64>, // optional minimum fill (default 0.0)
    pub metadata: serde_json::Value, // client-specific fields
}

impl PairOrder {
    /// Get effective minimum fill fraction
    pub fn min_fill(&self) -> f64 {
        self.min_fill_fraction.unwrap_or(0.0).clamp(0.0, 1.0)
    }

    /// Check if order has a limit constraint
    pub fn has_limit(&self) -> bool {
        self.limit_ratio.is_some()
    }

    /// Get limit in log-space (ln(limit_ratio))
    pub fn log_limit(&self) -> Option<f64> {
        self.limit_ratio.map(|r| r.ln())
    }
}

/// Basket order: pay j to receive a weighted basket
/// (Optional for v1; kept for extensibility)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasketOrder {
    pub id: OrderId,
    pub trader: AccountId,
    pub pay: AssetId,
    pub budget: Amount,
    pub basket_weights: BTreeMap<AssetId, f64>, // normalized weights
    pub min_fill_fraction: Option<f64>,
    pub metadata: serde_json::Value,
}

/// General order enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Order {
    Pair(PairOrder),
    Basket(BasketOrder),
}

impl Order {
    pub fn id(&self) -> &str {
        match self {
            Order::Pair(o) => &o.id,
            Order::Basket(o) => &o.id,
        }
    }

    pub fn trader(&self) -> &AccountId {
        match self {
            Order::Pair(o) => &o.trader,
            Order::Basket(o) => &o.trader,
        }
    }
}

/// Fill result for an order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: OrderId,
    pub fill_frac: f64,      // α_k
    pub pay_asset: AssetId,  // j_k
    pub recv_asset: AssetId, // i_k
    pub pay_units: f64,      // α_k * B_k (what trader pays)
    pub recv_units: f64,     // α_k * B_k * exp(y_j - y_i) (what trader receives)
    pub fees_paid: BTreeMap<AssetId, f64>, // fees by asset
}

impl Fill {
    /// Check if order was fully filled
    pub fn is_complete(&self) -> bool {
        self.fill_frac >= 0.9999 // Allow for small numerical tolerance
    }

    /// Check if order was partially filled
    pub fn is_partial(&self) -> bool {
        self.fill_frac > 0.0001 && self.fill_frac < 0.9999
    }

    /// Check if order was not filled
    pub fn is_empty(&self) -> bool {
        self.fill_frac < 0.0001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pair_order() {
        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.15),
            min_fill_fraction: Some(0.1),
            metadata: serde_json::json!({}),
        };

        assert_eq!(order.min_fill(), 0.1);
        assert!(order.has_limit());
        assert!((order.log_limit().unwrap() - 1.15_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_fill_status() {
        let fill = Fill {
            order_id: "order1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        assert!(fill.is_complete());
        assert!(!fill.is_partial());
        assert!(!fill.is_empty());
    }
}


