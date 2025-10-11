use convexfx_types::AssetId;
use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Risk parameters for the clearing optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParams {
    /// Target inventory (q*) in float units (millions for demo)
    pub q_target: BTreeMap<AssetId, f64>,

    /// Covariance matrix Γ (PSD) for inventory risk
    /// Penalizes deviation from q_target
    #[serde(skip)]
    pub gamma: DMatrix<f64>,

    /// Diagonal elements of Γ (serializable)
    pub gamma_diag: Vec<f64>,

    /// Price tracking matrix W (PSD) for oracle tracking
    /// Penalizes deviation from y_ref
    #[serde(skip)]
    pub w_track: DMatrix<f64>,

    /// Diagonal elements of W (serializable)
    pub w_diag: Vec<f64>,

    /// Fill incentive weight η
    pub eta: f64,

    /// Minimum inventory bounds
    pub q_min: BTreeMap<AssetId, f64>,

    /// Maximum inventory bounds
    pub q_max: BTreeMap<AssetId, f64>,

    /// Price band in basis points (trust region)
    pub price_band_bps: f64,
}

impl RiskParams {
    /// Create ultra-low slippage risk parameters (retail-optimized)
    /// Optimized for minimal price impact while maintaining coherence
    pub fn ultra_low_slippage() -> Self {
        let assets = AssetId::all();
        let n = assets.len();

        // Target inventory: 10M units each
        let mut q_target = BTreeMap::new();
        let mut q_min = BTreeMap::new();
        let mut q_max = BTreeMap::new();

        for asset in assets {
            q_target.insert(*asset, 10.0);
            q_min.insert(*asset, 5.0);
            q_max.insert(*asset, 15.0);
        }

        // Moderate optimization: balanced inventory risk and oracle tracking
        let gamma_diag = vec![0.5; n]; // 2x reduction from default (was 1.0)
        let w_diag = vec![200.0; n]; // 2x stronger tracking (was 100.0)

        let gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(gamma_diag.clone()));
        let w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(w_diag.clone()));

        RiskParams {
            q_target,
            gamma,
            gamma_diag,
            w_track,
            w_diag,
            eta: 1.0, // Standard fill incentive
            q_min,
            q_max,
            price_band_bps: 25.0, // Moderate bands for stability
        }
    }

    /// Create low slippage risk parameters (balanced trading)
    /// Optimized for reduced price impact with good fill rates
    pub fn low_slippage() -> Self {
        let assets = AssetId::all();
        let n = assets.len();

        // Target inventory: 10M units each
        let mut q_target = BTreeMap::new();
        let mut q_min = BTreeMap::new();
        let mut q_max = BTreeMap::new();

        for asset in assets {
            q_target.insert(*asset, 10.0);
            q_min.insert(*asset, 5.0);
            q_max.insert(*asset, 15.0);
        }

        // Optimized for low slippage: moderate inventory risk, strong oracle tracking
        let gamma_diag = vec![0.1; n]; // Moderate inventory penalty (was 1.0)
        let w_diag = vec![1000.0; n]; // Strong oracle tracking (was 100.0)

        let gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(gamma_diag.clone()));
        let w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(w_diag.clone()));

        RiskParams {
            q_target,
            gamma,
            gamma_diag,
            w_track,
            w_diag,
            eta: 0.5, // Moderate fill incentive (was 1.0)
            q_min,
            q_max,
            price_band_bps: 30.0, // Moderate bands for flexibility (was 20.0)
        }
    }

    /// Create fill-friendly risk parameters (institutional-optimized)
    /// Optimized for maximum fill rates in stressed market conditions
    pub fn fill_friendly() -> Self {
        let assets = AssetId::all();
        let n = assets.len();

        // Target inventory: 10M units each
        let mut q_target = BTreeMap::new();
        let mut q_min = BTreeMap::new();
        let mut q_max = BTreeMap::new();

        for asset in assets {
            q_target.insert(*asset, 10.0);
            q_min.insert(*asset, 5.0);
            q_max.insert(*asset, 15.0);
        }

        // Fill-friendly: strong inventory tolerance, moderate oracle tracking
        let gamma_diag = vec![2.0; n]; // Higher inventory tolerance for fills
        let w_diag = vec![200.0; n]; // Moderate oracle tracking

        let gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(gamma_diag.clone()));
        let w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(w_diag.clone()));

        RiskParams {
            q_target,
            gamma,
            gamma_diag,
            w_track,
            w_diag,
            eta: 2.0, // Strong fill incentive
            q_min,
            q_max,
            price_band_bps: 50.0, // Wider bands for flexibility in stress
        }
    }

    /// Create default risk parameters for demo
    pub fn default_demo() -> Self {
        let assets = AssetId::all();
        let n = assets.len();

        // Target inventory: 10M units each
        let mut q_target = BTreeMap::new();
        let mut q_min = BTreeMap::new();
        let mut q_max = BTreeMap::new();

        for asset in assets {
            q_target.insert(*asset, 10.0);
            q_min.insert(*asset, 5.0);
            q_max.insert(*asset, 15.0);
        }

        // Diagonal Γ and W (optimized for low slippage)
        let gamma_diag = vec![0.1; n]; // Reduced inventory risk (was 1.0)
        let w_diag = vec![1000.0; n]; // Stronger oracle tracking (was 100.0)

        let gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(gamma_diag.clone()));
        let w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(w_diag.clone()));

        RiskParams {
            q_target,
            gamma,
            gamma_diag,
            w_track,
            w_diag,
            eta: 1.0,
            q_min,
            q_max,
            price_band_bps: 50.0, // Increased for better flexibility
        }
    }

    /// Create with custom parameters
    pub fn new(
        q_target: BTreeMap<AssetId, f64>,
        gamma_diag: Vec<f64>,
        w_diag: Vec<f64>,
        eta: f64,
        q_min: BTreeMap<AssetId, f64>,
        q_max: BTreeMap<AssetId, f64>,
        price_band_bps: f64,
    ) -> Self {
        let gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(gamma_diag.clone()));
        let w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(w_diag.clone()));

        RiskParams {
            q_target,
            gamma,
            gamma_diag,
            w_track,
            w_diag,
            eta,
            q_min,
            q_max,
            price_band_bps,
        }
    }

    /// Rebuild matrices from serialized diagonal elements
    pub fn rebuild_matrices(&mut self) {
        self.gamma = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(self.gamma_diag.clone()));
        self.w_track = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(self.w_diag.clone()));
    }

    /// Get target inventory for an asset
    pub fn target(&self, asset: AssetId) -> f64 {
        self.q_target.get(&asset).copied().unwrap_or(0.0)
    }

    /// Get min bound for an asset
    pub fn min_bound(&self, asset: AssetId) -> f64 {
        self.q_min.get(&asset).copied().unwrap_or(f64::NEG_INFINITY)
    }

    /// Get max bound for an asset
    pub fn max_bound(&self, asset: AssetId) -> f64 {
        self.q_max.get(&asset).copied().unwrap_or(f64::INFINITY)
    }

    /// Check if inventory is within bounds
    pub fn is_within_bounds(&self, q: &BTreeMap<AssetId, f64>) -> bool {
        for asset in AssetId::all() {
            let value = q.get(asset).copied().unwrap_or(0.0);
            if value < self.min_bound(*asset) || value > self.max_bound(*asset) {
                return false;
            }
        }
        true
    }

    /// Compute inventory risk penalty: 0.5 * (q - q*)^T Γ (q - q*)
    pub fn inventory_penalty(&self, q: &BTreeMap<AssetId, f64>) -> f64 {
        let assets = AssetId::all();
        let n = assets.len();

        let q_vec = nalgebra::DVector::from_iterator(
            n,
            assets.iter().map(|a| q.get(a).copied().unwrap_or(0.0)),
        );

        let q_target_vec = nalgebra::DVector::from_iterator(
            n,
            assets.iter().map(|a| self.target(*a)),
        );

        let delta = q_vec - q_target_vec;
        let gamma_delta = self.gamma.clone() * &delta;
        0.5 * delta.dot(&gamma_delta)
    }

    /// Compute price tracking penalty: 0.5 * (y - y_ref)^T W (y - y_ref)
    pub fn tracking_penalty(&self, y: &BTreeMap<AssetId, f64>, y_ref: &BTreeMap<AssetId, f64>) -> f64 {
        let assets = AssetId::all();
        let n = assets.len();

        let y_vec = nalgebra::DVector::from_iterator(
            n,
            assets.iter().map(|a| y.get(a).copied().unwrap_or(0.0)),
        );

        let y_ref_vec = nalgebra::DVector::from_iterator(
            n,
            assets.iter().map(|a| y_ref.get(a).copied().unwrap_or(0.0)),
        );

        let delta = y_vec - y_ref_vec;
        let w_delta = self.w_track.clone() * &delta;
        0.5 * delta.dot(&w_delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_risk_params() {
        let params = RiskParams::default_demo();

        assert_eq!(params.target(AssetId::USD), 10.0);
        assert_eq!(params.min_bound(AssetId::USD), 5.0);
        assert_eq!(params.max_bound(AssetId::USD), 15.0);
        assert_eq!(params.eta, 1.0);
        assert_eq!(params.price_band_bps, 20.0);
    }

    #[test]
    fn test_bounds_checking() {
        let params = RiskParams::default_demo();

        // Need all assets for bounds checking
        let mut q_good = BTreeMap::new();
        for asset in AssetId::all() {
            q_good.insert(*asset, 10.0);
        }
        assert!(params.is_within_bounds(&q_good));

        let mut q_bad = BTreeMap::new();
        for asset in AssetId::all() {
            q_bad.insert(*asset, 10.0);
        }
        q_bad.insert(AssetId::USD, 3.0); // Below min
        assert!(!params.is_within_bounds(&q_bad));
    }

    #[test]
    fn test_inventory_penalty() {
        let params = RiskParams::default_demo();

        let mut q_at_target = BTreeMap::new();
        for asset in AssetId::all() {
            q_at_target.insert(*asset, 10.0);
        }

        let penalty_at_target = params.inventory_penalty(&q_at_target);
        assert!(penalty_at_target < 1e-6); // Should be near zero

        let mut q_deviated = BTreeMap::new();
        for asset in AssetId::all() {
            q_deviated.insert(*asset, 12.0); // Deviate by +2
        }

        let penalty_deviated = params.inventory_penalty(&q_deviated);
        assert!(penalty_deviated > 0.0);
    }
}

