use convexfx_clearing::EpochSolution;
use convexfx_oracle::RefPrices;
use convexfx_types::{AssetId, PairOrder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Key Performance Indicators for simulation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochKPIs {
    /// Effective slippage vs mid in bps (VWAP-weighted)
    pub slippage_bps_vwap: f64,
    pub slippage_bps_p50: f64,
    pub slippage_bps_p90: f64,
    pub slippage_bps_p99: f64,
    
    /// Fill rate (notional filled / notional submitted)
    pub fill_rate: f64,
    pub fill_rate_by_pair: BTreeMap<String, f64>,
    
    /// Cross-rate coherence error (no-arb) in bps
    pub coherence_error_max_bps: f64,
    pub coherence_error_rms_bps: f64,
    
    /// Inventory utilization per asset [0,1]
    pub inventory_utilization: BTreeMap<AssetId, f64>,
    
    /// LP P&L mark-to-market to oracle mids
    pub mtm_pnl: f64,
    
    /// Fee economics
    pub total_fees: f64,
    pub rebate_orders_pct: f64,
    pub fee_per_dollar_notional: f64,
    
    /// MEV fairness proxy
    pub price_dispersion_bps: f64,
    pub pre_post_mid_drift_bps: BTreeMap<AssetId, f64>,
    
    /// Solver health
    pub qp_solve_time_ms: f64,
    pub scp_iterations: usize,
    pub convergence_achieved: bool,
    
    /// Limit compliance
    pub limit_violations_pct: f64,
    
    /// Arb leakage
    pub max_triangular_arb_profit: f64,
}

impl Default for EpochKPIs {
    fn default() -> Self {
        Self {
            slippage_bps_vwap: 0.0,
            slippage_bps_p50: 0.0,
            slippage_bps_p90: 0.0,
            slippage_bps_p99: 0.0,
            fill_rate: 0.0,
            fill_rate_by_pair: BTreeMap::new(),
            coherence_error_max_bps: 0.0,
            coherence_error_rms_bps: 0.0,
            inventory_utilization: BTreeMap::new(),
            mtm_pnl: 0.0,
            total_fees: 0.0,
            rebate_orders_pct: 0.0,
            fee_per_dollar_notional: 0.0,
            price_dispersion_bps: 0.0,
            pre_post_mid_drift_bps: BTreeMap::new(),
            qp_solve_time_ms: 0.0,
            scp_iterations: 0,
            convergence_achieved: false,
            limit_violations_pct: 0.0,
            max_triangular_arb_profit: 0.0,
        }
    }
}

/// KPI calculator
pub struct KpiCalculator;

impl KpiCalculator {
    /// Calculate slippage in bps for an order
    pub fn calculate_slippage_bps(
        order: &PairOrder,
        solution: &EpochSolution,
        ref_prices: &RefPrices,
    ) -> f64 {
        let y_pay_exec = solution.y_star.get(&order.pay).copied().unwrap_or(0.0);
        let y_recv_exec = solution.y_star.get(&order.receive).copied().unwrap_or(0.0);
        let delta_exec = y_pay_exec - y_recv_exec;
        
        let y_pay_mid = ref_prices.get_ref(order.pay);
        let y_recv_mid = ref_prices.get_ref(order.receive);
        let delta_mid = y_pay_mid - y_recv_mid;
        
        (delta_exec - delta_mid) * 10_000.0 // Convert to bps
    }
    
    /// Calculate cross-rate coherence error for a triangle
    pub fn calculate_triangle_error(
        y_star: &BTreeMap<AssetId, f64>,
        a: AssetId,
        b: AssetId,
        c: AssetId,
    ) -> f64 {
        let y_a = y_star.get(&a).copied().unwrap_or(0.0);
        let y_b = y_star.get(&b).copied().unwrap_or(0.0);
        let y_c = y_star.get(&c).copied().unwrap_or(0.0);
        
        ((y_a - y_b) + (y_b - y_c) - (y_a - y_c)).abs()
    }
    
    /// Calculate inventory utilization for an asset
    /// u_i = |q'_i - q*_i| / (q*_i × 0.2) ∈ [0,1]
    pub fn calculate_inventory_utilization(
        q_post: f64,
        q_target: f64,
    ) -> f64 {
        let range = q_target * 0.2;
        if range.abs() < 1e-10 {
            return 0.0;
        }
        ((q_post - q_target).abs() / range).min(1.0)
    }
    
    /// Calculate all KPIs for an epoch
    pub fn calculate_epoch_kpis(
        orders: &[PairOrder],
        solution: &EpochSolution,
        ref_prices: &RefPrices,
        _q_initial: &BTreeMap<AssetId, f64>,
        q_target: &BTreeMap<AssetId, f64>,
    ) -> EpochKPIs {
        let mut kpis = EpochKPIs::default();
        
        // 1. Slippage metrics
        let mut slippages: Vec<(f64, f64)> = Vec::new(); // (slippage, notional)
        let mut total_notional = 0.0;
        
        for (order, fill) in orders.iter().zip(solution.fills.iter()) {
            if fill.fill_frac > 0.0 {
                let slippage = Self::calculate_slippage_bps(order, solution, ref_prices);
                let notional = order.budget.to_f64();
                slippages.push((slippage, notional));
                total_notional += notional;
            }
        }
        
        if !slippages.is_empty() {
            // VWAP slippage
            kpis.slippage_bps_vwap = slippages.iter()
                .map(|(s, n)| s * n)
                .sum::<f64>() / total_notional;
            
            // Percentiles
            let mut slip_values: Vec<f64> = slippages.iter().map(|(s, _)| *s).collect();
            slip_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let len = slip_values.len();
            kpis.slippage_bps_p50 = slip_values[len / 2];
            kpis.slippage_bps_p90 = slip_values[(len * 9) / 10];
            kpis.slippage_bps_p99 = slip_values[(len * 99) / 100];
        }
        
        // 2. Fill rate
        let submitted_notional: f64 = orders.iter()
            .map(|o| o.budget.to_f64())
            .sum();
        let filled_notional: f64 = solution.fills.iter()
            .zip(orders.iter())
            .map(|(f, o)| f.fill_frac * o.budget.to_f64())
            .sum();
        
        kpis.fill_rate = if submitted_notional > 0.0 {
            filled_notional / submitted_notional
        } else {
            0.0
        };
        
        // 3. Coherence error (check all triangles)
        let assets = AssetId::all();
        let mut max_error: f64 = 0.0;
        let mut error_sum_sq = 0.0;
        let mut triangle_count = 0;
        
        for i in 0..assets.len() {
            for j in (i+1)..assets.len() {
                for k in (j+1)..assets.len() {
                    let error = Self::calculate_triangle_error(
                        &solution.y_star,
                        assets[i],
                        assets[j],
                        assets[k],
                    );
                    max_error = max_error.max(error);
                    error_sum_sq += error * error;
                    triangle_count += 1;
                }
            }
        }
        
        kpis.coherence_error_max_bps = max_error * 10_000.0;
        kpis.coherence_error_rms_bps = if triangle_count > 0 {
            (error_sum_sq / triangle_count as f64).sqrt() * 10_000.0
        } else {
            0.0
        };
        
        // 4. Inventory utilization
        for asset in AssetId::all() {
            let q_post = solution.q_post.get(asset).copied().unwrap_or(0.0);
            let q_tgt = q_target.get(asset).copied().unwrap_or(0.0);
            let util = Self::calculate_inventory_utilization(q_post, q_tgt);
            kpis.inventory_utilization.insert(*asset, util);
        }
        
        // 5. Solver health
        kpis.scp_iterations = solution.diagnostics.iterations;
        kpis.convergence_achieved = solution.diagnostics.convergence_achieved;
        
        // 6. Limit compliance
        let mut violations = 0;
        for (order, fill) in orders.iter().zip(solution.fills.iter()) {
            if fill.fill_frac > 0.0 {
                if let Some(limit) = order.limit_ratio {
                    let y_i = solution.y_star.get(&order.receive).copied().unwrap_or(0.0);
                    let y_j = solution.y_star.get(&order.pay).copied().unwrap_or(0.0);
                    let actual_ratio = (y_i - y_j).exp();
                    
                    // Check if limit is violated
                    if actual_ratio > limit * 1.001 { // Small tolerance for numerical errors
                        violations += 1;
                    }
                }
            }
        }
        kpis.limit_violations_pct = if !orders.is_empty() {
            (violations as f64 / orders.len() as f64) * 100.0
        } else {
            0.0
        };
        
        kpis
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_slippage_calculation() {
        // Test basic slippage calculation
        // Add more comprehensive tests
    }
    
    #[test]
    fn test_triangle_error() {
        let mut y_star = BTreeMap::new();
        y_star.insert(AssetId::USD, 0.0);
        y_star.insert(AssetId::EUR, 0.1);
        y_star.insert(AssetId::JPY, -4.6);
        
        let error = KpiCalculator::calculate_triangle_error(
            &y_star,
            AssetId::USD,
            AssetId::EUR,
            AssetId::JPY,
        );
        
        // Should be near zero for consistent prices
        assert!(error.abs() < 1e-10);
    }
}

