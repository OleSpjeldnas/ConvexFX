use convexfx_risk::RiskParams;
use convexfx_types::{AssetId, Fill};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Fee configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    pub base_fee_bps: f64,      // Base taker fee in bps
    pub lambda_inventory: f64,   // Inventory sensitivity
    pub multiplier_min: f64,     // Min fee multiplier
    pub multiplier_max: f64,     // Max fee multiplier
}

impl Default for FeeConfig {
    fn default() -> Self {
        FeeConfig {
            base_fee_bps: 3.0,
            lambda_inventory: 0.5,
            multiplier_min: 0.5,
            multiplier_max: 3.0,
        }
    }
}

/// Fee line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeLine {
    pub order_id: String,
    pub asset: AssetId,
    pub amount: f64,
    pub multiplier: f64,
}

/// Fee policy trait
pub trait FeePolicy {
    fn compute_fees(
        &self,
        fills: &[Fill],
        q_post: &BTreeMap<AssetId, f64>,
        risk: &RiskParams,
    ) -> Vec<FeeLine>;
}

/// Inventory-aware fee policy
pub struct InventoryAwareFees {
    config: FeeConfig,
}

impl InventoryAwareFees {
    pub fn new(config: FeeConfig) -> Self {
        InventoryAwareFees { config }
    }

    pub fn with_defaults() -> Self {
        InventoryAwareFees {
            config: FeeConfig::default(),
        }
    }

    /// Compute inventory gradient: Γ (q' - q*)
    fn inventory_gradient(&self, q_post: &BTreeMap<AssetId, f64>, risk: &RiskParams) -> BTreeMap<AssetId, f64> {
        let assets = AssetId::all();
        let mut gradient = BTreeMap::new();

        for (i, asset) in assets.iter().enumerate() {
            let q = q_post.get(asset).copied().unwrap_or(0.0);
            let q_target = risk.target(*asset);
            let gamma_ii = risk.gamma_diag[i];

            gradient.insert(*asset, gamma_ii * (q - q_target));
        }

        gradient
    }

    /// Compute fee multiplier for an asset
    fn multiplier(&self, gradient: f64) -> f64 {
        let m = 1.0 + self.config.lambda_inventory * gradient;
        m.clamp(self.config.multiplier_min, self.config.multiplier_max)
    }
}

impl FeePolicy for InventoryAwareFees {
    fn compute_fees(
        &self,
        fills: &[Fill],
        q_post: &BTreeMap<AssetId, f64>,
        risk: &RiskParams,
    ) -> Vec<FeeLine> {
        let gradient = self.inventory_gradient(q_post, risk);
        let mut fee_lines = Vec::new();

        for fill in fills {
            // Fee on pay asset
            let g_pay = gradient.get(&fill.pay_asset).copied().unwrap_or(0.0);
            let m_pay = self.multiplier(g_pay);

            let notional = fill.pay_units;
            let fee_amount = notional * (self.config.base_fee_bps / 10000.0) * m_pay;

            fee_lines.push(FeeLine {
                order_id: fill.order_id.clone(),
                asset: fill.pay_asset,
                amount: fee_amount,
                multiplier: m_pay,
            });
        }

        fee_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiplier() {
        let fees = InventoryAwareFees::with_defaults();

        // Positive gradient -> higher multiplier
        assert!(fees.multiplier(1.0) > 1.0);

        // Negative gradient -> lower multiplier
        assert!(fees.multiplier(-1.0) < 1.0);

        // Check bounds
        assert!(fees.multiplier(100.0) <= 3.0); // Max
        assert!(fees.multiplier(-100.0) >= 0.5); // Min
    }
}

