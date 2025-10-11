use convexfx_oracle::RefPrices;
use convexfx_risk::RiskParams;
use convexfx_types::{AssetId, EpochId, PairOrder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Input instance for epoch clearing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochInstance {
    pub epoch_id: EpochId,
    /// Initial inventory in solver float units (e.g., millions)
    pub inventory_q: BTreeMap<AssetId, f64>,
    /// Orders to be cleared
    pub orders: Vec<PairOrder>,
    /// Reference prices from oracle
    pub ref_prices: RefPrices,
    /// Risk parameters
    pub risk: RiskParams,
}

impl EpochInstance {
    pub fn new(
        epoch_id: EpochId,
        inventory_q: BTreeMap<AssetId, f64>,
        orders: Vec<PairOrder>,
        ref_prices: RefPrices,
        risk: RiskParams,
    ) -> Self {
        EpochInstance {
            epoch_id,
            inventory_q,
            orders,
            ref_prices,
            risk,
        }
    }

    /// Get number of orders
    pub fn num_orders(&self) -> usize {
        self.orders.len()
    }

    /// Get number of assets
    pub fn num_assets(&self) -> usize {
        AssetId::all().len()
    }
}


