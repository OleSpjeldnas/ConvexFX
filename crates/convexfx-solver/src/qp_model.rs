use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use convexfx_types::{AssetId, OrderId};

/// Variable metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VarMeta {
    LogPrice(AssetId),
    FillFraction(OrderId),
}

/// QP model in standard form:
/// minimize 0.5 * x^T P x + q^T x
/// subject to l <= A x <= u
#[derive(Debug, Clone)]
pub struct QpModel {
    /// Hessian matrix P (must be PSD)
    pub p: DMatrix<f64>,
    /// Linear term q
    pub q: DVector<f64>,
    /// Constraint matrix A
    pub a: DMatrix<f64>,
    /// Lower bounds l
    pub l: DVector<f64>,
    /// Upper bounds u
    pub u: DVector<f64>,
    /// Variable metadata
    pub var_meta: Vec<VarMeta>,
}

impl QpModel {
    /// Create a new QP model
    pub fn new(
        p: DMatrix<f64>,
        q: DVector<f64>,
        a: DMatrix<f64>,
        l: DVector<f64>,
        u: DVector<f64>,
        var_meta: Vec<VarMeta>,
    ) -> Self {
        QpModel { p, q, a, l, u, var_meta }
    }

    /// Get number of variables
    pub fn num_vars(&self) -> usize {
        self.q.len()
    }

    /// Get number of constraints
    pub fn num_constraints(&self) -> usize {
        self.l.len()
    }

    /// Validate model dimensions
    pub fn validate(&self) -> convexfx_types::Result<()> {
        let n = self.num_vars();
        let m = self.num_constraints();

        if self.p.nrows() != n || self.p.ncols() != n {
            return Err(convexfx_types::ConvexFxError::SolverError(
                format!("P must be {}x{}, got {}x{}", n, n, self.p.nrows(), self.p.ncols())
            ));
        }

        if self.a.nrows() != m || self.a.ncols() != n {
            return Err(convexfx_types::ConvexFxError::SolverError(
                format!("A must be {}x{}, got {}x{}", m, n, self.a.nrows(), self.a.ncols())
            ));
        }

        if self.var_meta.len() != n {
            return Err(convexfx_types::ConvexFxError::SolverError(
                format!("var_meta length {} != num_vars {}", self.var_meta.len(), n)
            ));
        }

        Ok(())
    }
}

/// Variable in QP (for builder pattern)
#[derive(Debug, Clone)]
pub struct QpVariable {
    pub meta: VarMeta,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

/// Constraint in QP (for builder pattern)
#[derive(Debug, Clone)]
pub struct QpConstraint {
    pub coeffs: BTreeMap<usize, f64>, // var_index -> coefficient
    pub lower: f64,
    pub upper: f64,
}

impl QpConstraint {
    /// Create an equality constraint: sum(coeffs[i] * x[i]) = value
    pub fn eq(coeffs: BTreeMap<usize, f64>, value: f64) -> Self {
        QpConstraint {
            coeffs,
            lower: value,
            upper: value,
        }
    }

    /// Create an inequality constraint: lower <= sum(coeffs[i] * x[i]) <= upper
    pub fn ineq(coeffs: BTreeMap<usize, f64>, lower: f64, upper: f64) -> Self {
        QpConstraint { coeffs, lower, upper }
    }
}


