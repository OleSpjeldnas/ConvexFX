use convexfx_types::Result;
use serde::{Deserialize, Serialize};

use crate::qp_model::QpModel;

/// QP solver status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QpStatus {
    Optimal,
    PrimalInfeasible,
    DualInfeasible,
    MaxIterations,
    Unsolved,
}

/// Solution from QP solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpSolution {
    pub x: Vec<f64>,
    pub status: QpStatus,
    pub objective: f64,
    pub iterations: usize,
}

/// Trait for QP solver backends
pub trait SolverBackend: Send + Sync {
    /// Solve a QP problem: minimize 0.5 * x^T P x + q^T x
    /// subject to l <= A x <= u
    fn solve_qp(&self, model: &QpModel) -> Result<QpSolution>;
}


