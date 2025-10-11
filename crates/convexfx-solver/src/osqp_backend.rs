use clarabel::algebra::CscMatrix;
use clarabel::solver::{DefaultSettings, DefaultSolver, IPSolver};
use crate::{QpModel, QpSolution, QpStatus, SolverBackend};
use convexfx_types::Result;
use nalgebra::DMatrix;

/// Clarabel-based QP solver (production-ready, pure Rust)
pub struct OsqpSolver {
    verbose: bool,
    max_iter: u32,
    tol_gap_abs: f64,
    tol_gap_rel: f64,
}

impl OsqpSolver {
    /// Create a new Clarabel solver with default settings
    pub fn new() -> Self {
        OsqpSolver {
            verbose: false,
            max_iter: 10000, // Increased for better convergence
            tol_gap_abs: 1e-8, // Tighter tolerance
            tol_gap_rel: 1e-8,
        }
    }
    
    /// Create solver with custom settings
    pub fn with_params(max_iter: u32, tolerance: f64) -> Self {
        OsqpSolver {
            verbose: false,
            max_iter,
            tol_gap_abs: tolerance,
            tol_gap_rel: tolerance,
        }
    }
}

impl Default for OsqpSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverBackend for OsqpSolver {
    fn solve_qp(&self, model: &QpModel) -> Result<QpSolution> {
        model.validate()?;
        
        let m = model.num_constraints();
        
        // Convert P matrix to Clarabel CSC format (upper triangle)
        let p_csc = to_clarabel_csc_upper(&model.p);
        
        // Clarabel expects constraints in the form: l <= Ax <= u
        // We need to convert to Clarabel's cone format
        // For box constraints: l <= Ax <= u
        // We create a "box cone" by splitting into inequalities
        
        let mut b = Vec::with_capacity(2 * m);
        let mut cones = Vec::new();
        
        // For each constraint: l_i <= (Ax)_i <= u_i
        // Split into: (Ax)_i >= l_i  and  (Ax)_i <= u_i
        // Rewrite as: -l_i + (Ax)_i >= 0  and  u_i - (Ax)_i >= 0
        
        for i in 0..m {
            let li = model.l[i];
            let ui = model.u[i];
            
            // Clarabel format: Ax + s = b, s ∈ K+ (nonnegative cone)
            // Since s >= 0, we have Ax = b - s, so Ax <= b
            //
            // For LOWER bound (Ax)_i >= l_i:
            //   We want A_i x >= l_i
            //   Rewrite: -A_i x <= -l_i
            //   In Clarabel: -A_i x + s = -l_i, s >= 0
            if li.is_finite() {
                b.push(-li);
                cones.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
            }
            // For UPPER bound (Ax)_i <= u_i:
            //   We want A_i x <= u_i
            //   In Clarabel: A_i x + s = u_i, s >= 0
            if ui.is_finite() {
                b.push(ui);
                cones.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
            }
        }
        
        // Build extended A matrix for split constraints
        let a_extended = build_extended_a(&model.a, model.l.as_slice(), model.u.as_slice());
        let a_ext_csc = to_clarabel_csc(&a_extended);
        
        // Create settings
        let mut settings = DefaultSettings::default();
        settings.verbose = self.verbose;
        settings.max_iter = self.max_iter;
        settings.tol_gap_abs = self.tol_gap_abs;
        settings.tol_gap_rel = self.tol_gap_rel;
        
        // Create and solve problem
        let mut solver = DefaultSolver::new(
            &p_csc,
            model.q.as_slice(),
            &a_ext_csc,
            &b,
            &cones,
            settings,
        );
        
        solver.solve();
        
        // Map Clarabel status to our QpStatus
        let status = match solver.solution.status {
            clarabel::solver::SolverStatus::Solved => QpStatus::Optimal,
            clarabel::solver::SolverStatus::PrimalInfeasible => QpStatus::PrimalInfeasible,
            clarabel::solver::SolverStatus::DualInfeasible => QpStatus::DualInfeasible,
            clarabel::solver::SolverStatus::MaxIterations => QpStatus::MaxIterations,
            _ => QpStatus::Unsolved,
        };
        
        // Clamp solution to box constraints to handle numerical errors
        // For box constraints l_i <= (Ax)_i <= u_i where A is identity,
        // we just clamp each variable directly
        let mut x_clamped = solver.solution.x.clone();
        
        for i in 0..m {
            let li = model.l[i];
            let ui = model.u[i];
            
            // Check if this is a simple box constraint (A row has single 1.0 entry)
            let a_row = model.a.row(i);
            let nonzero_entries: Vec<(usize, f64)> = a_row.iter()
                .enumerate()
                .filter(|(_, &v)| v.abs() > 1e-10)
                .map(|(idx, &v)| (idx, v))
                .collect();
            
            if nonzero_entries.len() == 1 {
                let (var_idx, coeff) = nonzero_entries[0];
                
                // Box constraint: l <= coeff * x[var_idx] <= u
                // => l/coeff <= x[var_idx] <= u/coeff (if coeff > 0)
                if coeff > 0.0 {
                    if li.is_finite() {
                        x_clamped[var_idx] = x_clamped[var_idx].max(li / coeff);
                    }
                    if ui.is_finite() {
                        x_clamped[var_idx] = x_clamped[var_idx].min(ui / coeff);
                    }
                } else {
                    // Negative coefficient: inequality flips
                    if li.is_finite() {
                        x_clamped[var_idx] = x_clamped[var_idx].min(li / coeff);
                    }
                    if ui.is_finite() {
                        x_clamped[var_idx] = x_clamped[var_idx].max(ui / coeff);
                    }
                }
            }
        }
        
        Ok(QpSolution {
            x: x_clamped,
            objective: solver.solution.obj_val,
            status,
            iterations: solver.info.iterations as usize,
        })
    }
}

/// Convert DMatrix to Clarabel CSC format (upper triangle only for P)
fn to_clarabel_csc_upper(mat: &DMatrix<f64>) -> CscMatrix<f64> {
    let mut colptr = vec![0];
    let mut rowval = Vec::new();
    let mut nzval = Vec::new();
    
    let sparsity_threshold = 1e-12;
    
    // Iterate column by column (CSC format)
    for col in 0..mat.ncols() {
        // For upper triangle: row <= col
        for row in 0..=col {
            let val = mat[(row, col)];
            if val.abs() > sparsity_threshold {
                rowval.push(row);
                nzval.push(val);
            }
        }
        colptr.push(nzval.len());
    }
    
    CscMatrix {
        m: mat.nrows(),
        n: mat.ncols(),
        colptr,
        rowval,
        nzval,
    }
}

/// Convert DMatrix to Clarabel CSC format (full matrix)
fn to_clarabel_csc(mat: &DMatrix<f64>) -> CscMatrix<f64> {
    let mut colptr = vec![0];
    let mut rowval = Vec::new();
    let mut nzval = Vec::new();
    
    let sparsity_threshold = 1e-12;
    
    for col in 0..mat.ncols() {
        for row in 0..mat.nrows() {
            let val = mat[(row, col)];
            if val.abs() > sparsity_threshold {
                rowval.push(row);
                nzval.push(val);
            }
        }
        colptr.push(nzval.len());
    }
    
    CscMatrix {
        m: mat.nrows(),
        n: mat.ncols(),
        colptr,
        rowval,
        nzval,
    }
}

/// Build extended A matrix for split constraints
/// 
/// Clarabel format: Ax + s = b, s >= 0, which means Ax <= b
/// 
/// For l_i <= (Ax)_i <= u_i:
///  - Lower bound: (Ax)_i >= l_i → -A_i x <= -l_i → -A_i x + s = -l_i
///  - Upper bound: (Ax)_i <= u_i → A_i x <= u_i → A_i x + s = u_i
fn build_extended_a(a: &DMatrix<f64>, l: &[f64], u: &[f64]) -> DMatrix<f64> {
    let m = a.nrows();
    let n = a.ncols();
    
    // Count how many rows we need
    let mut num_rows = 0;
    for i in 0..m {
        if l[i].is_finite() {
            num_rows += 1;
        }
        if u[i].is_finite() {
            num_rows += 1;
        }
    }
    
    let mut a_ext = DMatrix::zeros(num_rows, n);
    let mut row_idx = 0;
    
    for i in 0..m {
        if l[i].is_finite() {
            // Lower bound: (Ax)_i >= l_i → NEGATE A_i
            for j in 0..n {
                a_ext[(row_idx, j)] = -a[(i, j)];
            }
            row_idx += 1;
        }
        if u[i].is_finite() {
            // Upper bound: (Ax)_i <= u_i → Keep A_i
            for j in 0..n {
                a_ext[(row_idx, j)] = a[(i, j)];
            }
            row_idx += 1;
        }
    }
    
    a_ext
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_qp() {
        use nalgebra::DVector;
        use crate::VarMeta;
        use convexfx_types::AssetId;
        
        // Minimize: 0.5 * x^T [[2,0],[0,2]] x + [1,1]^T x
        // Subject to: x >= 0
        // Solution: x = [0, 0] (gradient [1,1] points away from origin)
        
        let p = DMatrix::from_row_slice(2, 2, &[
            2.0, 0.0,
            0.0, 2.0,
        ]);
        let q = DVector::from_vec(vec![1.0, 1.0]);
        
        // Constraint: x >= 0
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_element(2, 0.0);
        let u = DVector::from_element(2, f64::INFINITY);
        
        let model = QpModel {
            p, q, a, l, u,
            var_meta: vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        };
        let solver = OsqpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        assert_eq!(solution.status, QpStatus::Optimal);
        
        // Optimal solution should be near [0, 0]
        assert!((solution.x[0] - 0.0).abs() < 1e-3, "x[0] = {}", solution.x[0]);
        assert!((solution.x[1] - 0.0).abs() < 1e-3, "x[1] = {}", solution.x[1]);
    }
    
    #[test]
    fn test_constrained_qp() {
        use nalgebra::DVector;
        use crate::VarMeta;
        use convexfx_types::AssetId;
        
        // Minimize: 0.5 * x^T [[1,0],[0,1]] x + [-2,-1]^T x
        // Subject to: x[0] + x[1] <= 1, x >= 0
        // Solution: x ≈ [1, 0]
        
        let p = DMatrix::identity(2, 2);
        let q = DVector::from_vec(vec![-2.0, -1.0]);
        
        // Constraints:
        // x[0] >= 0, x[1] >= 0, x[0] + x[1] <= 1
        let a = DMatrix::from_row_slice(3, 2, &[
            1.0, 0.0,
            0.0, 1.0,
            1.0, 1.0,
        ]);
        let l = DVector::from_vec(vec![0.0, 0.0, -f64::INFINITY]);
        let u = DVector::from_vec(vec![f64::INFINITY, f64::INFINITY, 1.0]);
        
        let model = QpModel {
            p, q, a, l, u,
            var_meta: vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        };
        let solver = OsqpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        assert_eq!(solution.status, QpStatus::Optimal);
        
        // Optimal solution should be near [1, 0]
        assert!((solution.x[0] - 1.0).abs() < 1e-2, "x[0] = {}", solution.x[0]);
        assert!((solution.x[1] - 0.0).abs() < 1e-2, "x[1] = {}", solution.x[1]);
    }
    
    #[test]
    fn test_infeasible_qp() {
        use nalgebra::DVector;
        use crate::VarMeta;
        use convexfx_types::AssetId;
        
        // Create an infeasible problem: x >= 1 and x <= 0
        let p = DMatrix::identity(1, 1);
        let q = DVector::from_element(1, 0.0);
        
        let a = DMatrix::identity(1, 1);
        let l = DVector::from_element(1, 1.0);  // x >= 1
        let u = DVector::from_element(1, 0.0);  // x <= 0 (infeasible!)
        
        let model = QpModel {
            p, q, a, l, u,
            var_meta: vec![VarMeta::LogPrice(AssetId::USD)],
        };
        let solver = OsqpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Should detect infeasibility
        assert_eq!(solution.status, QpStatus::PrimalInfeasible);
    }
}

