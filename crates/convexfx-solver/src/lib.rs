mod qp_model;
mod backend;
mod simple_backend;
mod osqp_backend;

pub use qp_model::{QpModel, QpVariable, QpConstraint, VarMeta};
pub use backend::{SolverBackend, QpSolution, QpStatus};
pub use simple_backend::SimpleQpSolver;
pub use osqp_backend::OsqpSolver;

#[cfg(test)]
mod tests;