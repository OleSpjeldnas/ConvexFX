use convexfx_types::{AssetId, EpochId, Fill};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Solution from epoch clearing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochSolution {
    pub epoch_id: EpochId,
    /// Final log-prices y*
    pub y_star: BTreeMap<AssetId, f64>,
    /// Final linear prices (exp(y*))
    pub prices: BTreeMap<AssetId, f64>,
    /// Post-trade inventory
    pub q_post: BTreeMap<AssetId, f64>,
    /// Order fills
    pub fills: Vec<Fill>,
    /// Objective function breakdown
    pub objective_terms: ObjectiveTerms,
    /// Diagnostic information
    pub diagnostics: Diagnostics,
}

/// Objective function breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveTerms {
    pub inventory_risk: f64,
    pub price_tracking: f64,
    pub fill_incentive: f64,
    pub total: f64,
}

/// Diagnostic information from clearing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostics {
    pub iterations: usize,
    pub convergence_achieved: bool,
    pub final_step_norm_y: f64,
    pub final_step_norm_alpha: f64,
    pub qp_status: String,
}


