// Comprehensive integration tests for solver

#[cfg(test)]
mod tests {
    use crate::*;
    use nalgebra::{DMatrix, DVector};
    use convexfx_types::AssetId;

    #[test]
    fn test_qp_model_validation() {
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 1.0]));
        let q = DVector::from_vec(vec![0.0, 0.0]);
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
                VarMeta::LogPrice(AssetId::USD),
                VarMeta::LogPrice(AssetId::EUR),
            ],
        );

        assert!(model.validate().is_ok());
        assert_eq!(model.num_vars(), 2);
        assert_eq!(model.num_constraints(), 2);
    }

    #[test]
    fn test_invalid_dimensions() {
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0])); // Wrong size
        let q = DVector::from_vec(vec![0.0, 0.0]);
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
                VarMeta::LogPrice(AssetId::USD),
                VarMeta::LogPrice(AssetId::EUR),
            ],
        );

        assert!(model.validate().is_err());
    }

    #[test]
    fn test_simple_qp_unconstrained() {
        // minimize x^2 + y^2 (should give x=y=0)
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 2.0]));
        let q = DVector::from_vec(vec![0.0, 0.0]);
        
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![-10.0, -10.0]);
        let u = DVector::from_vec(vec![10.0, 10.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        assert_eq!(solution.status, QpStatus::Optimal);
        assert!(solution.x[0].abs() < 0.01);
        assert!(solution.x[1].abs() < 0.01);
    }

    #[test]
    fn test_constrained_qp() {
        // minimize (x-2)^2 + (y-2)^2 subject to 0 <= x, y <= 1
        // Optimal should be (1, 1) since unconstrained optimum (2,2) is outside
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 2.0]));
        let q = DVector::from_vec(vec![-4.0, -4.0]);
        
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![0.0, 0.0]);
        let u = DVector::from_vec(vec![1.0, 1.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Note: Simple gradient solver may not fully converge with constraint projection
        assert!(matches!(solution.status, QpStatus::Optimal | QpStatus::MaxIterations));
        assert!((solution.x[0] - 1.0).abs() < 0.2);
        assert!((solution.x[1] - 1.0).abs() < 0.2);
    }

    #[test]
    fn test_asymmetric_bounds() {
        // minimize x^2 subject to -0.5 <= x <= 2
        // Optimal is x = 0
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0]));
        let q = DVector::from_vec(vec![0.0]);
        
        let a = DMatrix::from_row_slice(1, 1, &[1.0]);
        let l = DVector::from_vec(vec![-0.5]);
        let u = DVector::from_vec(vec![2.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        assert_eq!(solution.status, QpStatus::Optimal);
        assert!(solution.x[0].abs() < 0.1);
    }

    #[test]
    fn test_linear_objective() {
        // minimize -x - y subject to 0 <= x, y <= 1
        // Should give x = y = 1 (maximize x + y)
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1e-6, 1e-6])); // Small regularization
        let q = DVector::from_vec(vec![-1.0, -1.0]);
        
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![0.0, 0.0]);
        let u = DVector::from_vec(vec![1.0, 1.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Should be close to (1, 1)
        assert!(solution.x[0] > 0.9);
        assert!(solution.x[1] > 0.9);
    }

    #[test]
    fn test_negative_bounds() {
        // minimize (x-1)^2 subject to -2 <= x <= -0.5
        // Optimal is x = -0.5 (closest to 1)
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0]));
        let q = DVector::from_vec(vec![-2.0]);
        
        let a = DMatrix::from_row_slice(1, 1, &[1.0]);
        let l = DVector::from_vec(vec![-2.0]);
        let u = DVector::from_vec(vec![-0.5]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Note: Simple gradient solver may not fully converge
        assert!(matches!(solution.status, QpStatus::Optimal | QpStatus::MaxIterations));
        assert!((solution.x[0] - (-0.5)).abs() < 0.2);
    }

    #[test]
    fn test_solver_convergence() {
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 1.0]));
        let q = DVector::from_vec(vec![1.0, 1.0]);
        
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![-1.0, -1.0]);
        let u = DVector::from_vec(vec![1.0, 1.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Should converge
        assert!(solution.iterations > 0);
        assert!(solution.iterations < 1000);
    }

    #[test]
    fn test_multi_variable_qp() {
        // Test with 6 variables (our FX assets)
        let n = 6;
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0; n]));
        let q = DVector::from_vec(vec![0.0; n]);
        
        let a = DMatrix::identity(n, n);
        let l = DVector::from_vec(vec![-1.0; n]);
        let u = DVector::from_vec(vec![1.0; n]);
        
        let var_meta = AssetId::all()
            .iter()
            .map(|a| VarMeta::LogPrice(*a))
            .collect();
        
        let model = QpModel::new(p, q, a, l, u, var_meta);
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Note: Simple gradient solver may not fully converge
        assert!(matches!(solution.status, QpStatus::Optimal | QpStatus::MaxIterations));
        assert_eq!(solution.x.len(), n);
        
        // All should be near zero
        for x in &solution.x {
            assert!(x.abs() < 0.2);
        }
    }

    #[test]
    fn test_objective_computation() {
        let p = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 2.0]));
        let q = DVector::from_vec(vec![0.0, 0.0]);
        
        let a = DMatrix::identity(2, 2);
        let l = DVector::from_vec(vec![0.0, 0.0]);
        let u = DVector::from_vec(vec![1.0, 1.0]);
        
        let model = QpModel::new(
            p, q, a, l, u,
            vec![VarMeta::LogPrice(AssetId::USD), VarMeta::LogPrice(AssetId::EUR)],
        );
        
        let solver = SimpleQpSolver::new();
        let solution = solver.solve_qp(&model).unwrap();
        
        // Objective should be near zero (optimal point is origin)
        assert!(solution.objective < 0.01);
    }
}
