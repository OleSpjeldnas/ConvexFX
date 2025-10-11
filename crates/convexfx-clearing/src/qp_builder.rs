use convexfx_solver::{QpModel, QpSolution, VarMeta};
use convexfx_types::{AssetId, Result};
use nalgebra::{DMatrix, DVector};
use std::collections::BTreeMap;

use crate::epoch_instance::EpochInstance;

/// Builder for QP subproblems in SCP loop
pub struct QpBuilder;

impl QpBuilder {
    /// Build linearized QP at current iterate with adaptive trust regions
    pub fn build_qp_with_bands(
        inst: &EpochInstance,
        y_current: &BTreeMap<AssetId, f64>,
        bands: f64,
    ) -> Result<QpModel> {
        let assets = AssetId::all();
        let n_assets = assets.len();
        let n_orders = inst.orders.len();
        let n_vars = n_assets + n_orders; // y (with USD fixed at 0) + alpha

        // Build Hessian P = diag([W, 0]) + diag([Γ, 0]) from inventory linearization
        // Simplified: P = diag([W_diag, zeros])
        let mut p_diag = vec![0.0; n_vars];
        for (i, _asset) in assets.iter().enumerate() {
            p_diag[i] = inst.risk.w_diag[i];
        }

        let p = DMatrix::from_diagonal(&DVector::from_vec(p_diag));

        // Build linear term q
        let mut q_vec = vec![0.0; n_vars];

        // Price tracking term: W * (y - y_ref)
        for (i, asset) in assets.iter().enumerate() {
            let y_ref = inst.ref_prices.get_ref(*asset);
            let y_curr = y_current.get(asset).copied().unwrap_or(0.0);
            q_vec[i] = inst.risk.w_diag[i] * (y_curr - y_ref);
        }

        // Fill incentive: -eta * B_k * beta_k^(t)
        for (k, order) in inst.orders.iter().enumerate() {
            let y_j = y_current.get(&order.pay).copied().unwrap_or(0.0);
            let y_i = y_current.get(&order.receive).copied().unwrap_or(0.0);
            let beta_k = (y_j - y_i).exp();
            let budget = order.budget.to_f64();

            q_vec[n_assets + k] = -inst.risk.eta * budget * beta_k;
        }

        // Build constraint matrix A and bounds l, u
        let n_constraints = n_assets + n_orders + 1 + inst.orders.iter().filter(|o| o.has_limit()).count();
        let mut a_data = vec![vec![0.0; n_vars]; n_constraints];
        let mut l_vec = vec![0.0; n_constraints];
        let mut u_vec = vec![0.0; n_constraints];

        let mut row = 0;

        // USD numeraire
        let usd_idx = AssetId::USD.index();
        a_data[row][usd_idx] = 1.0;
        l_vec[row] = 0.0;
        u_vec[row] = 0.0;
        row += 1;

        // Price bands with adaptive trust regions
        for (i, asset) in assets.iter().enumerate() {
            let y_ref = inst.ref_prices.get_ref(*asset);
            let band_half = bands / 10000.0; // Convert bps to decimal

            a_data[row][i] = 1.0;
            l_vec[row] = y_ref - band_half;
            u_vec[row] = y_ref + band_half;
            row += 1;
        }

        // Fill bounds
        for k in 0..n_orders {
            a_data[row][n_assets + k] = 1.0;
            l_vec[row] = 0.0;
            u_vec[row] = 1.0;
            row += 1;
        }

        // Limit constraints
        for (_k, order) in inst.orders.iter().enumerate() {
            if let Some(log_limit) = order.log_limit() {
                let i_idx = order.receive.index();
                let j_idx = order.pay.index();

                a_data[row][i_idx] = 1.0;
                a_data[row][j_idx] = -1.0;
                l_vec[row] = f64::NEG_INFINITY;
                u_vec[row] = log_limit;
                row += 1;
            }
        }

        let a = DMatrix::from_row_slice(n_constraints, n_vars, &a_data.concat());
        let l = DVector::from_vec(l_vec.clone());
        let u = DVector::from_vec(u_vec.clone());

        // Variable metadata
        let mut var_meta = Vec::new();
        for asset in assets {
            var_meta.push(VarMeta::LogPrice(*asset));
        }
        for order in &inst.orders {
            var_meta.push(VarMeta::FillFraction(order.id.clone()));
        }

        Ok(QpModel::new(p, DVector::from_vec(q_vec), a, DVector::from_vec(l_vec), DVector::from_vec(u_vec), var_meta))
    }

    /// Build linearized QP at current iterate (y^(t), alpha^(t))
    pub fn build_qp(
        inst: &EpochInstance,
        y_current: &BTreeMap<AssetId, f64>,
    ) -> Result<QpModel> {
        let assets = AssetId::all();
        let n_assets = assets.len();
        let n_orders = inst.orders.len();
        let n_vars = n_assets + n_orders; // y (with USD fixed at 0) + alpha

        // Build Hessian P = diag([W, 0]) + diag([Γ, 0]) from inventory linearization
        // Simplified: P = diag([W_diag, zeros])
        let mut p_diag = vec![0.0; n_vars];
        for (i, _asset) in assets.iter().enumerate() {
            p_diag[i] = inst.risk.w_diag[i];
        }

        let p = DMatrix::from_diagonal(&DVector::from_vec(p_diag));

        // Build linear term q
        let mut q_vec = vec![0.0; n_vars];

        // Price tracking term: W * (y - y_ref)
        for (i, asset) in assets.iter().enumerate() {
            let y_ref = inst.ref_prices.get_ref(*asset);
            let y_curr = y_current.get(asset).copied().unwrap_or(0.0);
            q_vec[i] = inst.risk.w_diag[i] * (y_curr - y_ref);
        }

        // Fill incentive: -eta * B_k * beta_k^(t)
        for (k, order) in inst.orders.iter().enumerate() {
            let y_j = y_current.get(&order.pay).copied().unwrap_or(0.0);
            let y_i = y_current.get(&order.receive).copied().unwrap_or(0.0);
            let beta_k = (y_j - y_i).exp();
            let budget = order.budget.to_f64();

            q_vec[n_assets + k] = -inst.risk.eta * budget * beta_k;
        }

        let q = DVector::from_vec(q_vec);

        // Build constraint matrix A and bounds l, u
        // Constraints:
        // 1. Price bounds (trust region): y_low <= y <= y_high
        // 2. Fill bounds: 0 <= alpha <= 1
        // 3. USD numeraire: y_USD = 0
        // 4. Limit constraints: y_i - y_j <= log(limit_ratio)

        let n_constraints = n_assets + n_orders + 1 + inst.orders.iter().filter(|o| o.has_limit()).count();

        let mut a_data = vec![vec![0.0; n_vars]; n_constraints];
        let mut l_vec = vec![0.0; n_constraints];
        let mut u_vec = vec![0.0; n_constraints];

        let mut row = 0;

        // Price bounds
        for (i, asset) in assets.iter().enumerate() {
            a_data[row][i] = 1.0;
            l_vec[row] = inst.ref_prices.get_low(*asset);
            u_vec[row] = inst.ref_prices.get_high(*asset);
            row += 1;
        }

        // Fill bounds
        for k in 0..n_orders {
            a_data[row][n_assets + k] = 1.0;
            l_vec[row] = 0.0;
            u_vec[row] = 1.0;
            row += 1;
        }

        // USD numeraire
        let usd_idx = AssetId::USD.index();
        a_data[row][usd_idx] = 1.0;
        l_vec[row] = 0.0;
        u_vec[row] = 0.0;
        row += 1;

        // Limit constraints
        for (_k, order) in inst.orders.iter().enumerate() {
            if let Some(log_limit) = order.log_limit() {
                let i_idx = order.receive.index();
                let j_idx = order.pay.index();

                a_data[row][i_idx] = 1.0;
                a_data[row][j_idx] = -1.0;
                l_vec[row] = f64::NEG_INFINITY;
                u_vec[row] = log_limit;
                row += 1;
            }
        }

        let a = DMatrix::from_row_slice(n_constraints, n_vars, &a_data.concat());
        let l = DVector::from_vec(l_vec);
        let u = DVector::from_vec(u_vec);

        // Variable metadata
        let mut var_meta = Vec::new();
        for asset in assets {
            var_meta.push(VarMeta::LogPrice(*asset));
        }
        for order in &inst.orders {
            var_meta.push(VarMeta::FillFraction(order.id.clone()));
        }

        Ok(QpModel::new(p, q, a, l, u, var_meta))
    }

    /// Extract y and alpha from QP solution
    pub fn extract_solution(
        solution: &QpSolution,
        _inst: &EpochInstance,
    ) -> Result<(BTreeMap<AssetId, f64>, Vec<f64>)> {
        let assets = AssetId::all();
        let n_assets = assets.len();

        let mut y = BTreeMap::new();
        for (i, asset) in assets.iter().enumerate() {
            y.insert(*asset, solution.x[i]);
        }

        let alpha = solution.x[n_assets..].to_vec();

        Ok((y, alpha))
    }
}


