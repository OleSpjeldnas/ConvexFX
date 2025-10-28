//! Custom predicates for ConvexFX Delta executor
//!
//! This module implements validation predicates that ensure ConvexFX clearing
//! results satisfy mathematical optimality conditions and business rules before
//! being proven and submitted to the Delta base layer.

use crate::{DeltaIntegrationError, Result};
use convexfx_clearing::EpochSolution;
use convexfx_oracle::RefPrices;
use convexfx_types::AssetId;
use std::collections::BTreeMap;

/// Context for predicate validation
#[derive(Debug)]
pub struct PredicateContext<'a> {
    /// Reference prices from oracle
    pub oracle_prices: &'a RefPrices,
    /// Initial inventory before clearing
    pub initial_inventory: &'a BTreeMap<AssetId, f64>,
}

/// Parameters for SCP clearing validity predicate
#[derive(Debug, Clone)]
pub struct ScpClearingValidityPredicate {
    /// Tolerance for price convergence (log-space)
    pub tolerance_y: f64,
    /// Tolerance for fill fraction convergence
    pub tolerance_alpha: f64,
    /// Maximum acceptable price deviation from expected
    pub max_price_deviation: f64,
    /// Tolerance for numerical errors in inventory conservation
    pub inventory_tolerance: f64,
}

impl Default for ScpClearingValidityPredicate {
    fn default() -> Self {
        Self {
            tolerance_y: 1e-4,   // Matches SCP convergence tolerance
            tolerance_alpha: 1e-5, // Matches SCP convergence tolerance
            max_price_deviation: 0.01, // 1%
            inventory_tolerance: 1e-4,  // Relaxed for numerical stability
        }
    }
}

impl ScpClearingValidityPredicate {
    /// Validate that a clearing solution satisfies all SCP optimality conditions
    pub fn validate(&self, solution: &EpochSolution, context: &PredicateContext) -> Result<()> {
        // Run all validation checks
        self.validate_convergence(solution)?;
        self.validate_price_consistency(solution)?;
        self.validate_fill_feasibility(solution)?;
        self.validate_inventory_conservation(solution, context)?;
        self.validate_objective_optimality(solution)?;

        Ok(())
    }

    /// Validate that the SCP algorithm converged properly
    fn validate_convergence(&self, solution: &EpochSolution) -> Result<()> {
        // Check if SCP algorithm converged
        if !solution.diagnostics.convergence_achieved {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "SCP algorithm did not converge after {} iterations",
                solution.diagnostics.iterations
            )));
        }

        // Check step norms are within tolerance
        if solution.diagnostics.final_step_norm_y > self.tolerance_y {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Price step norm {} exceeds tolerance {}",
                solution.diagnostics.final_step_norm_y,
                self.tolerance_y
            )));
        }

        if solution.diagnostics.final_step_norm_alpha > self.tolerance_alpha {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Fill step norm {} exceeds tolerance {}",
                solution.diagnostics.final_step_norm_alpha,
                self.tolerance_alpha
            )));
        }

        Ok(())
    }

    /// Validate price consistency: linear prices = exp(log_prices)
    fn validate_price_consistency(&self, solution: &EpochSolution) -> Result<()> {
        // Check that linear prices = exp(log_prices)
        for (asset, log_price) in &solution.y_star {
            let expected_linear_price = log_price.exp();
            let actual_linear_price = solution.prices.get(asset).ok_or_else(|| {
                DeltaIntegrationError::ClearingFailed(format!(
                    "Missing linear price for asset {:?}",
                    asset
                ))
            })?;

            let price_error = (expected_linear_price - actual_linear_price).abs() / actual_linear_price;
            if price_error > self.max_price_deviation {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Price inconsistency for {:?}: expected {:.6}, got {:.6}, error {:.2}%",
                    asset,
                    expected_linear_price,
                    actual_linear_price,
                    price_error * 100.0
                )));
            }
        }

        // Check USD numeraire constraint (y_USD = 0)
        if let Some(usd_log_price) = solution.y_star.get(&AssetId::USD) {
            if usd_log_price.abs() > self.tolerance_y {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "USD numeraire constraint violated: y_USD = {}",
                    usd_log_price
                )));
            }
        }

        // Check all prices are positive and finite
        for (asset, price) in &solution.prices {
            if *price <= 0.0 {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Non-positive price for {:?}: {}",
                    asset, price
                )));
            }
            if !price.is_finite() {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Non-finite price for {:?}: {}",
                    asset, price
                )));
            }
        }

        Ok(())
    }

    /// Validate that all fills are feasible
    fn validate_fill_feasibility(&self, solution: &EpochSolution) -> Result<()> {
        for fill in &solution.fills {
            // Check fill fraction is in valid range [0, 1]
            if fill.fill_frac < 0.0 || fill.fill_frac > 1.0 {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Invalid fill fraction {:.6} for order {}",
                    fill.fill_frac, fill.order_id
                )));
            }

            // Check fill amounts are positive (or zero for unfilled orders)
            // Use a small tolerance to handle numerical precision issues
            const MIN_FILL_AMOUNT: f64 = 1e-8;
            if fill.fill_frac > MIN_FILL_AMOUNT {
                if fill.pay_units <= MIN_FILL_AMOUNT {
                    return Err(DeltaIntegrationError::ClearingFailed(format!(
                        "Non-positive pay amount {:.6} for filled order {} (fill_frac: {})",
                        fill.pay_units, fill.order_id, fill.fill_frac
                    )));
                }
                if fill.recv_units <= MIN_FILL_AMOUNT {
                    return Err(DeltaIntegrationError::ClearingFailed(format!(
                        "Non-positive receive amount {:.6} for filled order {} (fill_frac: {})",
                        fill.recv_units, fill.order_id, fill.fill_frac
                    )));
                }
            }

            // Check amounts are finite
            if !fill.pay_units.is_finite() || !fill.recv_units.is_finite() {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Non-finite amounts for order {}: pay={}, recv={}",
                    fill.order_id, fill.pay_units, fill.recv_units
                )));
            }
        }

        Ok(())
    }

    /// Validate inventory conservation: final = initial + net_flow
    fn validate_inventory_conservation(
        &self,
        solution: &EpochSolution,
        context: &PredicateContext,
    ) -> Result<()> {
        let initial_inventory = context.initial_inventory;
        let final_inventory = &solution.q_post;

        for asset in AssetId::all() {
            let initial_q = initial_inventory.get(asset).copied().unwrap_or(0.0);
            let final_q = final_inventory.get(asset).copied().unwrap_or(0.0);

            // Calculate net flow from fills
            let mut net_flow = 0.0;
            for fill in &solution.fills {
                if fill.pay_asset == *asset {
                    net_flow += fill.pay_units; // Pool receives pay asset
                }
                if fill.recv_asset == *asset {
                    net_flow -= fill.recv_units; // Pool gives receive asset
                }
            }

            let expected_final = initial_q + net_flow;
            let inventory_error = (final_q - expected_final).abs();

            if inventory_error > self.inventory_tolerance {
                return Err(DeltaIntegrationError::ClearingFailed(format!(
                    "Inventory conservation violated for {:?}: initial={:.6}, net_flow={:.6}, expected={:.6}, actual={:.6}, error={:.6}",
                    asset, initial_q, net_flow, expected_final, final_q, inventory_error
                )));
            }
        }

        Ok(())
    }

    /// Validate objective function values are reasonable
    fn validate_objective_optimality(&self, solution: &EpochSolution) -> Result<()> {
        let obj = &solution.objective_terms;

        // Inventory risk should be non-negative
        if obj.inventory_risk < -self.inventory_tolerance {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Negative inventory risk: {:.6}",
                obj.inventory_risk
            )));
        }

        // Price tracking should be non-negative
        if obj.price_tracking < -self.inventory_tolerance {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Negative price tracking: {:.6}",
                obj.price_tracking
            )));
        }

        // Total objective should be finite
        if !obj.total.is_finite() {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Non-finite objective value: {:.6}",
                obj.total
            )));
        }

        // Check that objective components sum correctly
        let computed_total = obj.inventory_risk + obj.price_tracking + obj.fill_incentive;
        let total_error = (obj.total - computed_total).abs();
        if total_error > self.inventory_tolerance {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Objective components don't sum correctly: components={:.6}, total={:.6}, error={:.6}",
                computed_total, obj.total, total_error
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_clearing::{Diagnostics, ObjectiveTerms};
    use convexfx_types::Fill;

    fn create_test_solution(
        convergence: bool,
        step_norm_y: f64,
        step_norm_alpha: f64,
    ) -> EpochSolution {
        let mut y_star = BTreeMap::new();
        let mut prices = BTreeMap::new();
        let mut q_post = BTreeMap::new();

        for asset in AssetId::all() {
            let log_price: f64 = if *asset == AssetId::USD {
                0.0
            } else {
                0.1 // Some non-zero value
            };
            y_star.insert(*asset, log_price);
            prices.insert(*asset, log_price.exp());
            q_post.insert(*asset, 10000.0);
        }

        EpochSolution {
            epoch_id: 1,
            y_star,
            prices,
            q_post,
            fills: Vec::new(),
            objective_terms: ObjectiveTerms {
                inventory_risk: 100.0,
                price_tracking: 50.0,
                fill_incentive: -20.0,
                total: 130.0,
            },
            diagnostics: Diagnostics {
                iterations: 3,
                convergence_achieved: convergence,
                final_step_norm_y: step_norm_y,
                final_step_norm_alpha: step_norm_alpha,
                qp_status: "Optimal".to_string(),
            },
        }
    }

    #[test]
    fn test_convergence_validation_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(true, 1e-6, 1e-7);

        assert!(predicate.validate_convergence(&solution).is_ok());
    }

    #[test]
    fn test_convergence_validation_not_converged() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(false, 1e-6, 1e-7);

        let result = predicate.validate_convergence(&solution);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("did not converge"));
    }

    #[test]
    fn test_convergence_validation_y_tolerance_exceeded() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(true, 1e-4, 1e-7); // y norm too large

        let result = predicate.validate_convergence(&solution);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Price step norm"));
    }

    #[test]
    fn test_price_consistency_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(true, 1e-6, 1e-7);

        assert!(predicate.validate_price_consistency(&solution).is_ok());
    }

    #[test]
    fn test_price_consistency_usd_numeraire() {
        let predicate = ScpClearingValidityPredicate::default();
        let mut solution = create_test_solution(true, 1e-6, 1e-7);
        solution.y_star.insert(AssetId::USD, 0.1); // Violate USD = 0
        solution.prices.insert(AssetId::USD, 0.1_f64.exp()); // Keep price consistent with log_price

        let result = predicate.validate_price_consistency(&solution);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("USD numeraire constraint violated"));
    }

    #[test]
    fn test_fill_feasibility_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let mut solution = create_test_solution(true, 1e-6, 1e-7);

        solution.fills.push(Fill {
            order_id: "test1".to_string(),
            fill_frac: 0.8,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 860.0,
            fees_paid: BTreeMap::new(),
        });

        assert!(predicate.validate_fill_feasibility(&solution).is_ok());
    }

    #[test]
    fn test_fill_feasibility_invalid_fraction() {
        let predicate = ScpClearingValidityPredicate::default();
        let mut solution = create_test_solution(true, 1e-6, 1e-7);

        solution.fills.push(Fill {
            order_id: "test1".to_string(),
            fill_frac: 1.5, // Invalid: > 1.0
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 860.0,
            fees_paid: BTreeMap::new(),
        });

        let result = predicate.validate_fill_feasibility(&solution);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid fill fraction"));
    }

    #[test]
    fn test_inventory_conservation_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let mut solution = create_test_solution(true, 1e-6, 1e-7);

        // Set up initial inventory
        let mut initial_inventory = BTreeMap::new();
        for asset in AssetId::all() {
            initial_inventory.insert(*asset, 10000.0);
        }

        // Add a fill
        solution.fills.push(Fill {
            order_id: "test1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 860.0,
            fees_paid: BTreeMap::new(),
        });

        // Update q_post to reflect the fill
        solution.q_post.insert(AssetId::USD, 11000.0); // +1000
        solution.q_post.insert(AssetId::EUR, 9140.0); // -860

        let context = PredicateContext {
            oracle_prices: &RefPrices::new(
                solution.y_star.clone(),
                20.0,
                0,
                vec!["test".to_string()],
            ),
            initial_inventory: &initial_inventory,
        };

        assert!(predicate
            .validate_inventory_conservation(&solution, &context)
            .is_ok());
    }

    #[test]
    fn test_objective_optimality_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(true, 1e-6, 1e-7);

        assert!(predicate.validate_objective_optimality(&solution).is_ok());
    }

    #[test]
    fn test_complete_validation_success() {
        let predicate = ScpClearingValidityPredicate::default();
        let solution = create_test_solution(true, 1e-6, 1e-7);

        let mut initial_inventory = BTreeMap::new();
        for asset in AssetId::all() {
            initial_inventory.insert(*asset, 10000.0);
        }

        let context = PredicateContext {
            oracle_prices: &RefPrices::new(
                solution.y_star.clone(),
                20.0,
                0,
                vec!["test".to_string()],
            ),
            initial_inventory: &initial_inventory,
        };

        assert!(predicate.validate(&solution, &context).is_ok());
    }
}

