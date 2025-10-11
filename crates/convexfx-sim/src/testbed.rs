use convexfx_risk::RiskParams;
use convexfx_types::AssetId;
use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Common testbed configuration for all scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testbed {
    /// Assets in the pool (5: USD, EUR, JPY, GBP, CHF)
    pub assets: Vec<AssetId>,
    
    /// Oracle mid prices (USD per unit)
    pub oracle_mids: BTreeMap<AssetId, f64>,
    
    /// Initial inventory (in millions)
    pub initial_inventory: BTreeMap<AssetId, f64>,
    
    /// Target inventory (q*)
    pub target_inventory: BTreeMap<AssetId, f64>,
    
    /// Inventory bounds (min, max) as fraction of target
    pub inventory_bound_factor: (f64, f64), // (0.8, 1.2) for ±20%
    
    /// Price bands in bps
    pub band_bps: f64,
    
    /// Tracking weight diagonal
    pub tracking_weights: Vec<f64>,
    
    /// Daily volatilities (%)
    pub daily_vols_pct: BTreeMap<AssetId, f64>,
    
    /// Correlation matrix (skipped in serialization)
    #[serde(skip)]
    pub correlations: DMatrix<f64>,
    
    /// Risk penalty lambda
    pub risk_lambda: f64,
    
    /// Fees: base taker bps
    pub base_fee_bps: f64,
    
    /// Fee inventory multiplier
    pub fee_multiplier: f64,
    
    /// Fee multiplier clamp range
    pub fee_clamp: (f64, f64),
    
    /// Epoch length in seconds
    pub epoch_length_secs: u64,
    
    /// SCP convergence tolerance (y)
    pub scp_eps_y: f64,
    
    /// SCP convergence tolerance (alpha)
    pub scp_eps_alpha: f64,
    
    /// SCP max iterations
    pub scp_max_iters: usize,
}

impl Default for Testbed {
    fn default() -> Self {
        Self::standard_5_asset()
    }
}

impl Testbed {
    /// Standard 5-asset testbed as specified in the requirements
    /// (Note: system supports 6 assets but test plan specifies 5 for baseline scenarios)
    pub fn standard_5_asset() -> Self {
        let assets = vec![
            AssetId::USD,
            AssetId::EUR,
            AssetId::JPY,
            AssetId::GBP,
            AssetId::CHF,
            AssetId::AUD, // Include 6th asset for consistency
        ];
        
        // Oracle mids (USD per unit)
        let mut oracle_mids = BTreeMap::new();
        oracle_mids.insert(AssetId::USD, 1.0000);
        oracle_mids.insert(AssetId::EUR, 1.1000);
        oracle_mids.insert(AssetId::JPY, 0.006667); // ≈ 1/150
        oracle_mids.insert(AssetId::GBP, 1.2500);
        oracle_mids.insert(AssetId::CHF, 1.0750);
        oracle_mids.insert(AssetId::AUD, 0.7500); // AUDUSD = 0.75
        
        // Initial & target inventory (~$100m notional each at mid)
        let mut inventory = BTreeMap::new();
        inventory.insert(AssetId::USD, 100.0);     // 100.0m USD
        inventory.insert(AssetId::EUR, 90.91);     // = 100/1.10
        inventory.insert(AssetId::JPY, 15_000.0);  // = 100/0.006667
        inventory.insert(AssetId::GBP, 80.00);     // = 100/1.25
        inventory.insert(AssetId::CHF, 93.02);     // = 100/1.075
        inventory.insert(AssetId::AUD, 133.33);    // = 100/0.75
        
        // Daily vols (%)
        let mut daily_vols = BTreeMap::new();
        daily_vols.insert(AssetId::USD, 0.0);  // Numeraire
        daily_vols.insert(AssetId::EUR, 0.60);
        daily_vols.insert(AssetId::JPY, 0.70);
        daily_vols.insert(AssetId::GBP, 0.80);
        daily_vols.insert(AssetId::CHF, 0.50);
        daily_vols.insert(AssetId::AUD, 0.75);
        
        // Correlation matrix (6x6)
        // Order: USD, EUR, JPY, GBP, CHF, AUD
        #[rustfmt::skip]
        let corr_data = vec![
            1.00,  0.00,  0.00,  0.00,  0.00,  0.00,  // USD (numeraire, uncorrelated)
            0.00,  1.00, -0.20,  0.80, -0.70,  0.40,  // EUR
            0.00, -0.20,  1.00, -0.15,  0.10,  0.30,  // JPY
            0.00,  0.80, -0.15,  1.00, -0.60,  0.50,  // GBP
            0.00, -0.70,  0.10, -0.60,  1.00, -0.40,  // CHF
            0.00,  0.40,  0.30,  0.50, -0.40,  1.00,  // AUD
        ];
        let correlations = DMatrix::from_row_slice(6, 6, &corr_data);
        
        Self {
            assets,
            oracle_mids,
            initial_inventory: inventory.clone(),
            target_inventory: inventory,
            inventory_bound_factor: (0.8, 1.2),
            band_bps: 20.0,
            tracking_weights: vec![100.0; 6],
            daily_vols_pct: daily_vols,
            correlations,
            risk_lambda: 1.0,
            base_fee_bps: 3.0,
            fee_multiplier: 0.5,
            fee_clamp: (0.5, 3.0),
            epoch_length_secs: 60,
            scp_eps_y: 1e-5,
            scp_eps_alpha: 1e-6,
            scp_max_iters: 5,
        }
    }
    
    /// Convert to RiskParams
    pub fn to_risk_params(&self) -> RiskParams {
        let mut risk = RiskParams::default_demo();
        
        risk.eta = 1.0; // Fill incentive
        risk.price_band_bps = self.band_bps;
        
        // Set target inventory
        for (asset, &q_target) in &self.target_inventory {
            risk.q_target.insert(*asset, q_target);
            
            let (min_factor, max_factor) = self.inventory_bound_factor;
            risk.q_min.insert(*asset, q_target * min_factor);
            risk.q_max.insert(*asset, q_target * max_factor);
        }
        
        // Set tracking weights
        risk.w_diag = self.tracking_weights.clone();
        
        // Build gamma from volatilities and correlations
        // Γ = λ × D_σ × Corr × D_σ
        let n = self.assets.len();
        let mut d_sigma = vec![0.0; n];
        
        for (i, asset) in self.assets.iter().enumerate() {
            let vol_pct = self.daily_vols_pct.get(asset).copied().unwrap_or(0.0);
            d_sigma[i] = vol_pct / 100.0; // Convert % to decimal
        }
        
        // Γ = λ × D × Corr × D
        let mut gamma = DMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                gamma[(i, j)] = self.risk_lambda 
                    * d_sigma[i] 
                    * self.correlations[(i, j)] 
                    * d_sigma[j];
            }
        }
        
        risk.gamma = gamma;
        risk.gamma_diag = d_sigma.iter().map(|s| self.risk_lambda * s * s).collect();
        
        // Rebuild matrices
        risk.rebuild_matrices();
        
        risk
    }
    
    /// Get log-prices from linear prices
    pub fn get_log_prices(&self) -> BTreeMap<AssetId, f64> {
        self.oracle_mids.iter()
            .map(|(asset, &price)| (*asset, price.ln()))
            .collect()
    }
    
    /// Get price bounds (y_low, y_high) from bands
    pub fn get_price_bounds(&self) -> BTreeMap<AssetId, (f64, f64)> {
        let band_factor = self.band_bps / 10_000.0; // Convert bps to decimal
        
        self.oracle_mids.iter()
            .map(|(asset, &price)| {
                let y_ref = price.ln();
                // For log-prices: band in log space ≈ band_factor for small bands
                let delta = band_factor;
                (*asset, (y_ref - delta, y_ref + delta))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_standard_testbed() {
        let testbed = Testbed::standard_5_asset();
        
        assert_eq!(testbed.assets.len(), 6); // Now includes AUD
        assert_eq!(*testbed.oracle_mids.get(&AssetId::EUR).unwrap(), 1.1);
        assert_eq!(*testbed.target_inventory.get(&AssetId::USD).unwrap(), 100.0);
        assert_eq!(*testbed.target_inventory.get(&AssetId::AUD).unwrap(), 133.33);
    }
    
    #[test]
    fn test_to_risk_params() {
        let testbed = Testbed::standard_5_asset();
        let risk = testbed.to_risk_params();
        
        assert_eq!(risk.q_target.get(&AssetId::USD), Some(&100.0));
        assert_eq!(risk.q_min.get(&AssetId::USD), Some(&80.0));
        assert_eq!(risk.q_max.get(&AssetId::USD), Some(&120.0));
        
        // Check gamma matrix is symmetric
        for i in 0..6 {
            for j in 0..6 {
                assert!((risk.gamma[(i, j)] - risk.gamma[(j, i)]).abs() < 1e-10);
            }
        }
    }
    
    #[test]
    fn test_log_prices() {
        let testbed = Testbed::standard_5_asset();
        let log_prices = testbed.get_log_prices();
        
        // USD should be numeraire (log(1.0) = 0)
        assert!((log_prices[&AssetId::USD]).abs() < 1e-10);
        
        // EUR should be positive (price > 1)
        assert!(log_prices[&AssetId::EUR] > 0.0);
    }
}

