use convexfx_solver::{SolverBackend, SimpleQpSolver, OsqpSolver};
use convexfx_types::{AssetId, Fill, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::epoch_instance::EpochInstance;
use crate::epoch_solution::{Diagnostics, EpochSolution, ObjectiveTerms};
use crate::qp_builder::QpBuilder;

/// Parameters for SCP algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScpParams {
    pub max_iterations: usize,
    pub tolerance_y: f64,
    pub tolerance_alpha: f64,
    pub line_search_max_steps: usize,
}

impl Default for ScpParams {
    fn default() -> Self {
        ScpParams {
            max_iterations: 5,
            tolerance_y: 1e-5,
            tolerance_alpha: 1e-6,
            line_search_max_steps: 10,
        }
    }
}

/// Sequential Convex Programming clearing algorithm
pub struct ScpClearing {
    backend: Arc<dyn SolverBackend + Send + Sync>,
    params: ScpParams,
}

impl ScpClearing {
    /// Create a new SCP clearing engine with Clarabel solver (production default)
    ///
    /// Uses Clarabel (pure Rust) for robust, production-ready QP solving.
    /// Simple solver available via with_simple_solver() for debugging.
    pub fn new() -> Self {
        Self::with_clarabel()
    }
    
    /// Create with custom backend and parameters
    pub fn with_backend(backend: Arc<dyn SolverBackend + Send + Sync>, params: ScpParams) -> Self {
        ScpClearing { backend, params }
    }

    /// Create with OSQP solver (production default)
    pub fn with_osqp_solver() -> Self {
        ScpClearing {
            backend: Arc::new(OsqpSolver::new()),
            params: ScpParams::default(),
        }
    }

    /// Create with Clarabel solver (pure Rust alternative)
    pub fn with_clarabel() -> Self {
        ScpClearing {
            backend: Arc::new(OsqpSolver::new()),  // Uses Clarabel backend
            params: ScpParams::default(),
        }
    }

    /// Create with simple gradient solver (for debugging only)
    pub fn with_simple_solver() -> Self {
        ScpClearing {
            backend: Arc::new(SimpleQpSolver::new()),
            params: ScpParams::default(),
        }
    }

    /// Clear an epoch with hot-starting and adaptive trust regions
    pub fn clear_epoch(&self, inst: &EpochInstance) -> Result<EpochSolution> {
        let _assets = AssetId::all();
        let n_orders = inst.orders.len();

        // Hot-start: Initialize from oracle prices (or previous solution if available)
        let mut y_current: BTreeMap<AssetId, f64> = inst
            .ref_prices
            .y_ref
            .iter()
            .map(|(asset, y)| (*asset, *y))
            .collect();

        let mut alpha_current: Vec<f64> = vec![0.0; n_orders];

        let mut iterations = 0;
        let mut converged = false;
        let mut final_step_norm_y = 0.0;
        let mut final_step_norm_alpha = 0.0;
        let mut qp_status = String::new();

        for iter in 0..self.params.max_iterations {
            iterations = iter + 1;

            // Adaptive trust regions: start tight, widen if needed
            let adaptive_bands = if iter == 0 {
                // First iteration: tight bands for stability
                10.0 // Start with 10 bps bands
            } else if final_step_norm_y > self.params.tolerance_y * 10.0 {
                // Large steps in previous iteration: widen bands for flexibility
                30.0 // Widen to 30 bps
            } else {
                // Normal iterations: use moderate bands
                20.0
            };

            // Build linearized QP with adaptive trust regions
            let qp_model = QpBuilder::build_qp_with_bands(inst, &y_current, adaptive_bands)?;

            // Solve QP
            let solution = self.backend.solve_qp(&qp_model)?;
            qp_status = format!("{:?}", solution.status);

            // Extract y~ and alpha~ from solution
            let (y_new, alpha_new) = QpBuilder::extract_solution(&solution, inst)?;

            // Improved line search with backtracking for exact nonlinear feasibility
            let lambda = self.backtracking_line_search(
                inst,
                &y_current,
                &alpha_current,
                &y_new,
                &alpha_new,
                adaptive_bands,
            )?;

            let y_next: BTreeMap<AssetId, f64> = y_current
                .iter()
                .map(|(asset, y_old)| {
                    let y_step = y_new.get(asset).copied().unwrap_or(0.0) - y_old;
                    (*asset, y_old + lambda * y_step)
                })
                .collect();

            let alpha_next: Vec<f64> = alpha_current
                .iter()
                .zip(alpha_new.iter())
                .map(|(a_old, a_step)| {
                    let step = a_step - a_old;
                    a_old + lambda * step
                })
                .collect();

            // Compute step norms
            let step_norm_y = y_next
                .iter()
                .map(|(asset, y)| {
                    let y_old = y_current.get(asset).copied().unwrap_or(0.0);
                    (y - y_old).abs()
                })
                .fold(0.0, f64::max);

            let step_norm_alpha = alpha_next
                .iter()
                .zip(alpha_current.iter())
                .map(|(a_new, a_old)| (a_new - a_old).abs())
                .fold(0.0, f64::max);

            final_step_norm_y = step_norm_y;
            final_step_norm_alpha = step_norm_alpha;

            // Update iterates
            y_current = y_next;
            alpha_current = alpha_next;

            // Check convergence
            if step_norm_y < self.params.tolerance_y && step_norm_alpha < self.params.tolerance_alpha {
                converged = true;
                break;
            }
        }

        // Compute final quantities with exact nonlinear formulas
        let (q_post, fills) = self.compute_fills_and_inventory(inst, &y_current, &alpha_current)?;

        // Compute prices (linear space)
        let prices: BTreeMap<AssetId, f64> = y_current
            .iter()
            .map(|(asset, y)| (*asset, y.exp()))
            .collect();

        // Compute objective terms
        let objective_terms = self.compute_objective_terms(inst, &q_post, &y_current, &fills);

        let diagnostics = Diagnostics {
            iterations,
            convergence_achieved: converged,
            final_step_norm_y,
            final_step_norm_alpha,
            qp_status,
        };

        Ok(EpochSolution {
            epoch_id: inst.epoch_id,
            y_star: y_current,
            prices,
            q_post,
            fills,
            objective_terms,
            diagnostics,
        })
    }

    /// Compute fills and post-trade inventory using exact formulas
    fn compute_fills_and_inventory(
        &self,
        inst: &EpochInstance,
        y: &BTreeMap<AssetId, f64>,
        alpha: &[f64],
    ) -> Result<(BTreeMap<AssetId, f64>, Vec<Fill>)> {
        let mut q_post = inst.inventory_q.clone();
        let mut fills = Vec::new();

        for (k, order) in inst.orders.iter().enumerate() {
            let alpha_k = alpha[k];
            
            // Always create a Fill entry to maintain alignment with orders
            let (pay_units, recv_units) = if alpha_k < 1e-10 {
                (0.0, 0.0)
            } else {
                let y_j = y.get(&order.pay).copied().unwrap_or(0.0);
                let y_i = y.get(&order.receive).copied().unwrap_or(0.0);

                let pay = alpha_k * order.budget.to_f64();
                let recv = pay * (y_j - y_i).exp();

                // Update inventory
                *q_post.entry(order.pay).or_insert(0.0) += pay;
                *q_post.entry(order.receive).or_insert(0.0) -= recv;
                
                (pay, recv)
            };

            fills.push(Fill {
                order_id: order.id.clone(),
                fill_frac: alpha_k,
                pay_asset: order.pay,
                recv_asset: order.receive,
                pay_units,
                recv_units,
                fees_paid: BTreeMap::new(), // Fees computed separately
            });
        }

        Ok((q_post, fills))
    }

    /// Compute objective function terms
    fn compute_objective_terms(
        &self,
        inst: &EpochInstance,
        q_post: &BTreeMap<AssetId, f64>,
        y: &BTreeMap<AssetId, f64>,
        fills: &[Fill],
    ) -> ObjectiveTerms {
        let inventory_risk = inst.risk.inventory_penalty(q_post);
        let price_tracking = inst.risk.tracking_penalty(y, &inst.ref_prices.y_ref);

        let fill_incentive = -inst.risk.eta
            * fills
                .iter()
                .map(|fill| fill.pay_units * (fill.pay_asset as i32) as f64) // Placeholder
                .sum::<f64>();

        let total = inventory_risk + price_tracking + fill_incentive;

        ObjectiveTerms {
            inventory_risk,
            price_tracking,
            fill_incentive,
            total,
        }
    }

    /// Backtracking line search for exact nonlinear feasibility
    fn backtracking_line_search(
        &self,
        inst: &EpochInstance,
        y_current: &BTreeMap<AssetId, f64>,
        alpha_current: &[f64],
        y_new: &BTreeMap<AssetId, f64>,
        alpha_new: &[f64],
        bands: f64,
    ) -> Result<f64> {
        // Start with full step
        let mut lambda = 1.0;
        let alpha = 0.5; // Backtracking factor
        let max_steps = self.params.line_search_max_steps;

        for _step in 0..max_steps {
            // Compute proposed next iterate
            let y_next: BTreeMap<AssetId, f64> = y_current
                .iter()
                .map(|(asset, y_old)| {
                    let y_step = y_new.get(asset).copied().unwrap_or(0.0) - y_old;
                    (*asset, y_old + lambda * y_step)
                })
                .collect();

            let alpha_next: Vec<f64> = alpha_current
                .iter()
                .zip(alpha_new.iter())
                .map(|(a_old, a_step)| {
                    let step = a_step - a_old;
                    a_old + lambda * step
                })
                .collect();

            // Check if proposed iterate satisfies nonlinear constraints
            if self.check_nonlinear_feasibility(inst, &y_next, &alpha_next, bands) {
                return Ok(lambda);
            }

            // Reduce step size
            lambda *= alpha;
        }

        // If line search fails, use minimal step
        Ok(lambda)
    }

    /// Check exact nonlinear feasibility (price bands, inventory bounds, etc.)
    fn check_nonlinear_feasibility(
        &self,
        inst: &EpochInstance,
        y_next: &BTreeMap<AssetId, f64>,
        alpha_next: &[f64],
        bands: f64,
    ) -> bool {
        // Check price bands
        for asset in AssetId::all() {
            let y_ref = inst.ref_prices.get_ref(*asset);
            let y = y_next.get(asset).copied().unwrap_or(0.0);
            let band_half = bands / 10000.0; // Convert bps to decimal

            if (y - y_ref).abs() > band_half {
                return false;
            }
        }

        // Check inventory bounds
        let q_next = self.compute_inventory_next(inst, y_next, alpha_next);
        for asset in AssetId::all() {
            let q = q_next.get(asset).copied().unwrap_or(0.0);
            let min_bound = inst.risk.min_bound(*asset);
            let max_bound = inst.risk.max_bound(*asset);

            if q < min_bound || q > max_bound {
                return false;
            }
        }

        // Check fill bounds
        for alpha in alpha_next {
            if *alpha < 0.0 || *alpha > 1.0 {
                return false;
            }
        }

        true
    }

    /// Compute next inventory state (nonlinear mapping)
    fn compute_inventory_next(
        &self,
        inst: &EpochInstance,
        y_next: &BTreeMap<AssetId, f64>,
        alpha_next: &[f64],
    ) -> BTreeMap<AssetId, f64> {
        let mut q_next = BTreeMap::new();

        for asset in AssetId::all() {
            q_next.insert(*asset, inst.risk.target(*asset));
        }

        // Apply fills to inventory
        for (k, order) in inst.orders.iter().enumerate() {
            let alpha = alpha_next.get(k).copied().unwrap_or(0.0);
            if alpha > 0.0 {
                let pay_asset = order.pay;
                let receive_asset = order.receive;
                let budget = order.budget.to_f64();
                let y_pay = y_next.get(&pay_asset).copied().unwrap_or(0.0);
                let y_receive = y_next.get(&receive_asset).copied().unwrap_or(0.0);

                let pay_amount = alpha * budget;
                let receive_amount = alpha * budget * (y_receive - y_pay).exp();

                // Update inventory
                if let Some(q) = q_next.get_mut(&pay_asset) {
                    *q -= pay_amount;
                }
                if let Some(q) = q_next.get_mut(&receive_asset) {
                    *q += receive_amount;
                }
            }
        }

        q_next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_oracle::{MockOracle, Oracle};
    use convexfx_risk::RiskParams;

    #[test]
    fn test_empty_orders() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.current_prices().unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let inst = EpochInstance::new(1, inventory, vec![], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // No orders -> no fills
        assert_eq!(solution.fills.len(), 0);
        // Prices should be near ref
        for asset in AssetId::all() {
            let y_ref = inst.ref_prices.y_ref.get(asset).copied().unwrap_or(0.0);
            let y_star = solution.y_star.get(asset).copied().unwrap_or(0.0);
            assert!((y_star - y_ref).abs() < 0.01); // Within band
        }
    }
}


