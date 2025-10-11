use crate::{EpochKPIs, KpiCalculator, Scenario};
use convexfx_clearing::{EpochInstance, ScpClearing};
use convexfx_oracle::{MockOracle, Oracle};
use convexfx_types::PairOrder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

/// Result of a simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    pub scenario_name: String,
    pub epochs: Vec<EpochResult>,
    pub summary: SimSummary,
}

/// Result of a single epoch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochResult {
    pub epoch_id: u64,
    pub kpis: EpochKPIs,
    pub num_orders: usize,
    pub runtime_ms: f64,
}

/// Summary statistics across all epochs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimSummary {
    pub total_epochs: usize,
    pub avg_fill_rate: f64,
    pub avg_slippage_p90_bps: f64,
    pub max_coherence_error_bps: f64,
    pub avg_iterations: f64,
    pub total_runtime_ms: f64,
    pub passed: bool,
    pub failure_reasons: Vec<String>,
}

/// Simulation runner
pub struct SimRunner {
    clearing: ScpClearing,
}

impl SimRunner {
    pub fn new() -> Self {
        Self {
            clearing: ScpClearing::with_simple_solver(),
        }
    }
    
    /// Run a scenario and collect KPIs
    pub fn run_scenario(&self, scenario: &Scenario) -> SimResult {
        let _start_time = Instant::now();
        let mut epoch_results = Vec::new();
        
        // Setup oracle with testbed prices
        let oracle = self.create_oracle(&scenario);
        
        // Initial inventory
        let mut current_inventory = scenario.testbed.initial_inventory.clone();
        
        // Run epochs
        for epoch_id in 0..scenario.config.num_epochs as u64 {
            let epoch_start = Instant::now();
            
            // Generate orders for this epoch
            let orders = self.generate_orders(&scenario, epoch_id);
            
            // Get reference prices
            let ref_prices = oracle.reference_prices(epoch_id).unwrap();
            
            // Setup risk params
            let mut risk = scenario.testbed.to_risk_params();
            
            // Apply overrides
            if let Some(ref weights) = scenario.config.override_tracking_weights {
                risk.w_diag = weights.clone();
                risk.rebuild_matrices();
            }
            
            // Create epoch instance
            let instance = EpochInstance::new(
                epoch_id,
                current_inventory.clone(),
                orders.clone(),
                ref_prices.clone(),
                risk,
            );
            
            // Clear epoch
            let solution = match self.clearing.clear_epoch(&instance) {
                Ok(sol) => sol,
                Err(e) => {
                    eprintln!("Clearing failed for epoch {}: {:?}", epoch_id, e);
                    continue;
                }
            };
            
            // Calculate KPIs
            let mut kpis = KpiCalculator::calculate_epoch_kpis(
                &orders,
                &solution,
                &ref_prices,
                &current_inventory,
                &scenario.testbed.target_inventory,
            );
            
            // Record runtime
            kpis.qp_solve_time_ms = epoch_start.elapsed().as_millis() as f64;
            
            // Update inventory for next epoch
            current_inventory = solution.q_post.clone();
            
            epoch_results.push(EpochResult {
                epoch_id,
                kpis,
                num_orders: orders.len(),
                runtime_ms: epoch_start.elapsed().as_millis() as f64,
            });
        }
        
        // Calculate summary
        let summary = self.calculate_summary(&scenario, &epoch_results);
        
        SimResult {
            scenario_name: scenario.config.name.clone(),
            epochs: epoch_results,
            summary,
        }
    }
    
    /// Create oracle from testbed
    fn create_oracle(&self, scenario: &Scenario) -> MockOracle {
        let mut prices = BTreeMap::new();
        for (asset, &price) in &scenario.testbed.oracle_mids {
            prices.insert(*asset, price);
        }
        
        MockOracle::with_prices(prices).with_band_bps(scenario.testbed.band_bps)
    }
    
    /// Generate orders for an epoch based on scenario config
    fn generate_orders(&self, scenario: &Scenario, epoch_id: u64) -> Vec<PairOrder> {
        use crate::generator::OrderGenerator;
        
        let config = &scenario.config;
        let seed = config.seed.unwrap_or(0);
        let gen = OrderGenerator::with_seed(seed);
        
        gen.generate_orders(config, epoch_id)
    }
    
    /// Calculate summary statistics
    fn calculate_summary(&self, scenario: &Scenario, epochs: &[EpochResult]) -> SimSummary {
        if epochs.is_empty() {
            return SimSummary {
                total_epochs: 0,
                avg_fill_rate: 0.0,
                avg_slippage_p90_bps: 0.0,
                max_coherence_error_bps: 0.0,
                avg_iterations: 0.0,
                total_runtime_ms: 0.0,
                passed: false,
                failure_reasons: vec!["No epochs executed".to_string()],
            };
        }
        
        let n = epochs.len() as f64;
        
        let avg_fill_rate = epochs.iter()
            .map(|e| e.kpis.fill_rate)
            .sum::<f64>() / n;
        
        let avg_slippage_p90_bps = epochs.iter()
            .map(|e| e.kpis.slippage_bps_p90)
            .sum::<f64>() / n;
        
        let max_coherence_error_bps = epochs.iter()
            .map(|e| e.kpis.coherence_error_max_bps)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        
        let avg_iterations = epochs.iter()
            .map(|e| e.kpis.scp_iterations as f64)
            .sum::<f64>() / n;
        
        let total_runtime_ms = epochs.iter()
            .map(|e| e.runtime_ms)
            .sum::<f64>();
        
        // Check expected outcomes
        let mut failure_reasons = Vec::new();
        
        if let Some(ref expected) = scenario.config.expected_outcomes {
            if let Some(max_iter) = expected.max_iterations {
                let max_actual = epochs.iter()
                    .map(|e| e.kpis.scp_iterations)
                    .max()
                    .unwrap_or(0);
                if max_actual > max_iter {
                    failure_reasons.push(format!(
                        "Max iterations {} > expected {}",
                        max_actual, max_iter
                    ));
                }
            }
            
            if let Some(min_fill) = expected.min_fill_rate {
                if avg_fill_rate < min_fill {
                    failure_reasons.push(format!(
                        "Fill rate {:.2}% < expected {:.2}%",
                        avg_fill_rate * 100.0,
                        min_fill * 100.0
                    ));
                }
            }
            
            if let Some(max_slip) = expected.max_slippage_p90_bps {
                if avg_slippage_p90_bps > max_slip {
                    failure_reasons.push(format!(
                        "Slippage p90 {:.2} bps > expected {:.2} bps",
                        avg_slippage_p90_bps, max_slip
                    ));
                }
            }
            
            if let Some(max_coh) = expected.max_coherence_error_bps {
                if max_coherence_error_bps > max_coh {
                    failure_reasons.push(format!(
                        "Coherence error {:.4} bps > expected {:.4} bps",
                        max_coherence_error_bps, max_coh
                    ));
                }
            }
        }
        
        SimSummary {
            total_epochs: epochs.len(),
            avg_fill_rate,
            avg_slippage_p90_bps,
            max_coherence_error_bps,
            avg_iterations,
            total_runtime_ms,
            passed: failure_reasons.is_empty(),
            failure_reasons,
        }
    }
}

impl Default for SimRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scenario;
    
    #[test]
    fn test_empty_epoch_scenario() {
        let runner = SimRunner::new();
        let scenario = Scenario::empty_epoch();
        
        let result = runner.run_scenario(&scenario);
        
        println!("Empty epoch results:");
        println!("  Fill rate: {:.2}%", result.summary.avg_fill_rate * 100.0);
        println!("  Iterations: {:.1}", result.summary.avg_iterations);
        println!("  Coherence error: {:.6} bps", result.summary.max_coherence_error_bps);
        println!("  Passed: {}", result.summary.passed);
        
        if !result.summary.passed {
            println!("  Failures:");
            for reason in &result.summary.failure_reasons {
                println!("    - {}", reason);
            }
        }
        
        // Empty epoch should pass all checks
        assert!(result.summary.passed, "Empty epoch scenario should pass");
    }
}

