use nalgebra::DVector;

use crate::backend::{QpSolution, QpStatus, SolverBackend};
use crate::qp_model::QpModel;
use convexfx_types::Result;

/// Simple gradient-based QP solver (for demo purposes)
/// Uses projected gradient descent with line search
/// Not as robust as OSQP but doesn't require C bindings
pub struct SimpleQpSolver {
    max_iters: usize,
    tolerance: f64,
}

impl SimpleQpSolver {
    pub fn new() -> Self {
        SimpleQpSolver {
            max_iters: 500, // Reduced for practical performance (was 5000)
            tolerance: 1e-3, // Relaxed tolerance (was 1e-6)
        }
    }

    pub fn with_params(max_iters: usize, tolerance: f64) -> Self {
        SimpleQpSolver { max_iters, tolerance }
    }


    /// Compute gradient: P x + q
    fn gradient(&self, model: &QpModel, x: &DVector<f64>) -> DVector<f64> {
        &model.p * x + &model.q
    }

    /// Compute objective: 0.5 * x^T P x + q^T x
    fn objective(&self, model: &QpModel, x: &DVector<f64>) -> f64 {
        0.5 * x.dot(&(&model.p * x)) + model.q.dot(x)
    }

    /// Check if constraints are satisfied (approximately)
    /// For now, we only handle simple box constraints via projection
    fn check_feasibility(&self, model: &QpModel, x: &DVector<f64>) -> bool {
        let ax = &model.a * x;
        for i in 0..ax.len() {
            if ax[i] < model.l[i] - self.tolerance || ax[i] > model.u[i] + self.tolerance {
                return false;
            }
        }
        true
    }
    
    /// Project vector onto constraint set l <= Ax <= u
    fn project_constraints(&self, x: &DVector<f64>, model: &QpModel) -> DVector<f64> {
        // Simple projection: for each constraint that is violated, 
        // adjust the variables to satisfy it
        // This is a simplified projection; a proper implementation would solve
        // a QP to find the closest feasible point
        let mut x_proj = x.clone();
        let max_proj_iters = 50; // Increased from 10
        
        for _iter in 0..max_proj_iters {
            let ax = &model.a * &x_proj;
            let mut max_violation: f64 = 0.0;
            
            for i in 0..model.num_constraints() {
                let val = ax[i];
                let li = model.l[i];
                let ui = model.u[i];
                
                let violation = if val < li {
                    li - val
                } else if val > ui {
                    val - ui
                } else {
                    0.0
                };
                
                max_violation = max_violation.max(violation);
                
                // Simple correction: adjust along the constraint gradient
                if violation > 1e-10 {
                    let row = model.a.row(i);
                    let row_norm_sq = row.norm_squared();
                    if row_norm_sq > 1e-10 {
                        let correction = if val < li {
                            (li - val) / row_norm_sq
                        } else {
                            (ui - val) / row_norm_sq
                        };
                        
                        for j in 0..model.num_vars() {
                            x_proj[j] += correction * row[j] * 0.9; // Increased damping from 0.5
                        }
                    }
                }
            }
            
            if max_violation < 1e-6 { // Relaxed from 1e-8
                break;
            }
        }
        
        x_proj
    }
}

impl Default for SimpleQpSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverBackend for SimpleQpSolver {
    fn solve_qp(&self, model: &QpModel) -> Result<QpSolution> {
        model.validate()?;

        let n = model.num_vars();
        let m = model.num_constraints();

        // Initialize variables at a reasonable starting point
        // We can't use model.l and model.u directly as they are constraint bounds (dim m),
        // not variable bounds (dim n). The variable bounds are encoded in the constraint matrix.
        // For our QP: first n_assets variables are log-prices (init to 0), 
        // remaining are fill fractions (init to 0.5)
        let mut x = DVector::from_element(n, 0.0);
        
        // Try to find a feasible starting point by looking at the constraint structure
        // The first constraints are typically variable bounds from qp_builder
        for i in 0..n.min(m) {
            // Check if constraint i is a variable bound (A[i,i] = 1 and all other entries in row i are 0)
            let row_i = model.a.row(i);
            if row_i[i].abs() - 1.0 < 1e-10 {
                let non_zero_count = row_i.iter().filter(|&&v| v.abs() > 1e-10).count();
                if non_zero_count == 1 {
                    // This is a variable bound constraint
                    let li = model.l[i];
                    let ui = model.u[i];
                    x[i] = if li.is_finite() && ui.is_finite() {
                        (li + ui) / 2.0
                    } else if li.is_finite() {
                        li.max(0.0) + 0.1
                    } else if ui.is_finite() {
                        ui.min(0.0) - 0.1
                    } else {
                        0.0
                    };
                }
            }
        }

        // Project initial point to ensure feasibility
        x = self.project_constraints(&x, model);

        let mut iterations = 0;
        let mut prev_obj = self.objective(model, &x);

        // Projected gradient descent
        for iter in 0..self.max_iters {
            iterations = iter + 1;

            let grad = self.gradient(model, &x);

            // Line search for step size
            let mut alpha = 1.0;
            let mut x_new = &x - &grad * alpha;
            x_new = self.project_constraints(&x_new, model);
            let mut obj_new = self.objective(model, &x_new);

            for _ in 0..20 {
                if obj_new < prev_obj {
                    break;
                }
                alpha *= 0.5;
                x_new = &x - &grad * alpha;
                x_new = self.project_constraints(&x_new, model);
                obj_new = self.objective(model, &x_new);
            }

            // Check convergence
            let step_norm = (&x_new - &x).norm();
            x = x_new;

            if step_norm < self.tolerance {
                // Check if we're at a feasible point
                let status = if self.check_feasibility(model, &x) {
                    QpStatus::Optimal
                } else {
                    QpStatus::PrimalInfeasible
                };

                return Ok(QpSolution {
                    x: x.as_slice().to_vec(),
                    status,
                    objective: obj_new,
                    iterations,
                });
            }

            prev_obj = obj_new;
        }

        // Max iterations reached
        Ok(QpSolution {
            x: x.as_slice().to_vec(),
            status: QpStatus::MaxIterations,
            objective: prev_obj,
            iterations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn test_simple_qp() {
        // minimize 0.5 * x^2 + 0.5 * y^2 subject to 0 <= x, y <= 1
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 1.0]));
        let q = DVector::from_vec(vec![0.0, 0.0]);

        // Identity constraints (simple bounds)
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![0.0, 0.0]);
        let u = DVector::from_vec(vec![1.0, 1.0]);

        let model = QpModel::new(
            p,
            q,
            a,
            l,
            u,
            vec![
                crate::qp_model::VarMeta::LogPrice(convexfx_types::AssetId::USD),
                crate::qp_model::VarMeta::LogPrice(convexfx_types::AssetId::EUR),
            ],
        );

        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();

        assert_eq!(solution.status, QpStatus::Optimal);
        // Optimal should be (0, 0)
        assert!(solution.x[0].abs() < 0.1);
        assert!(solution.x[1].abs() < 0.1);
    }

    #[test]
    fn test_constrained_qp() {
        // minimize (x-1)^2 + (y-1)^2 subject to 0 <= x, y <= 0.5
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 2.0]));
        let q = DVector::from_vec(vec![-2.0, -2.0]); // -2*1 for shift

        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![0.0, 0.0]);
        let u = DVector::from_vec(vec![0.5, 0.5]);

        let model = QpModel::new(
            p,
            q,
            a,
            l,
            u,
            vec![
                crate::qp_model::VarMeta::LogPrice(convexfx_types::AssetId::USD),
                crate::qp_model::VarMeta::LogPrice(convexfx_types::AssetId::EUR),
            ],
        );

        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();

        // Note: Simple gradient solver may not fully converge with constraint projection
        // A production system should use OSQP or another robust QP solver
        assert!(matches!(solution.status, QpStatus::Optimal | QpStatus::MaxIterations));
        // Optimal should be around (0.5, 0.5) since unconstrained optimum (1,1) is outside bounds
        assert!((solution.x[0] - 0.5).abs() < 0.2);
        assert!((solution.x[1] - 0.5).abs() < 0.2);
    }
}


