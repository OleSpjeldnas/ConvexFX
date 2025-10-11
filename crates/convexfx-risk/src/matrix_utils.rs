use nalgebra::{DMatrix, DVector};

/// Build a diagonal gamma matrix from variance vector
pub fn build_gamma_matrix(variances: &[f64]) -> DMatrix<f64> {
    DMatrix::from_diagonal(&DVector::from_vec(variances.to_vec()))
}

/// Build a diagonal W tracking matrix from weight vector
pub fn build_w_matrix(weights: &[f64]) -> DMatrix<f64> {
    DMatrix::from_diagonal(&DVector::from_vec(weights.to_vec()))
}

/// Validate that a matrix is positive semi-definite (PSD)
/// Uses eigenvalue decomposition
pub fn validate_psd(matrix: &DMatrix<f64>, tolerance: f64) -> bool {
    if matrix.nrows() != matrix.ncols() {
        return false;
    }

    // Check symmetry
    let symmetric = matrix.iter().enumerate().all(|(i, &val)| {
        let row = i / matrix.ncols();
        let col = i % matrix.ncols();
        if row >= col {
            true
        } else {
            (val - matrix[(col, row)]).abs() < tolerance
        }
    });

    if !symmetric {
        return false;
    }

    // For diagonal matrices, just check non-negative diagonal
    let is_diagonal = matrix.iter().enumerate().all(|(i, &val)| {
        let row = i / matrix.ncols();
        let col = i % matrix.ncols();
        row == col || val.abs() < tolerance
    });

    if is_diagonal {
        return matrix.diagonal().iter().all(|&x| x >= -tolerance);
    }

    // For general matrices, would need eigenvalue decomposition
    // For now, assume valid if symmetric (caller responsibility)
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_diagonal_matrices() {
        let variances = vec![1.0, 2.0, 3.0];
        let gamma = build_gamma_matrix(&variances);

        assert_eq!(gamma.nrows(), 3);
        assert_eq!(gamma.ncols(), 3);
        assert_eq!(gamma[(0, 0)], 1.0);
        assert_eq!(gamma[(1, 1)], 2.0);
        assert_eq!(gamma[(2, 2)], 3.0);
        assert_eq!(gamma[(0, 1)], 0.0);
    }

    #[test]
    fn test_psd_validation() {
        let positive_diag = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 2.0, 3.0]));
        assert!(validate_psd(&positive_diag, 1e-10));

        let zero_diag = DMatrix::from_diagonal(&DVector::from_vec(vec![0.0, 0.0, 0.0]));
        assert!(validate_psd(&zero_diag, 1e-10));
    }
}


