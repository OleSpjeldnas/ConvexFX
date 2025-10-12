// Comprehensive integration tests for risk

#[cfg(test)]
mod tests {
    use crate::*;
    use convexfx_types::AssetId;
    use std::collections::BTreeMap;

    #[test]
    fn test_params_basic_values() {
        let params = RiskParams::default_demo();

        // Check key values exist (updated to 6 assets)
        assert_eq!(params.target(AssetId::USD), 10.0);
        assert_eq!(params.eta, 1.0);
        assert_eq!(params.gamma_diag.len(), 6);
        assert_eq!(params.w_diag.len(), 6);
    }

    #[test]
    fn test_tracking_penalty() {
        let params = RiskParams::default_demo();

        let mut y = BTreeMap::new();
        let mut y_ref = BTreeMap::new();

        // Same prices -> zero penalty
        for asset in AssetId::all() {
            y.insert(*asset, 0.1);
            y_ref.insert(*asset, 0.1);
        }

        let penalty_same = params.tracking_penalty(&y, &y_ref);
        assert!(penalty_same < 1e-6);

        // Different prices -> positive penalty
        for asset in AssetId::all() {
            y.insert(*asset, 0.2);
        }

        let penalty_diff = params.tracking_penalty(&y, &y_ref);
        assert!(penalty_diff > 0.0);
    }

    #[test]
    fn test_inventory_penalty_at_target() {
        let params = RiskParams::default_demo();

        let mut q = BTreeMap::new();
        for asset in AssetId::all() {
            q.insert(*asset, params.target(*asset));
        }

        let penalty = params.inventory_penalty(&q);
        assert!(penalty < 1e-6); // Should be near zero at target
    }

    #[test]
    fn test_inventory_penalty_deviation() {
        let params = RiskParams::default_demo();

        let mut q_at_target = BTreeMap::new();
        let mut q_deviated = BTreeMap::new();

        for asset in AssetId::all() {
            q_at_target.insert(*asset, params.target(*asset));
            q_deviated.insert(*asset, params.target(*asset) + 2.0); // Deviate by +2
        }

        let penalty_target = params.inventory_penalty(&q_at_target);
        let penalty_deviated = params.inventory_penalty(&q_deviated);

        assert!(penalty_deviated > penalty_target);
    }

    #[test]
    fn test_bounds_checking() {
        let params = RiskParams::default_demo();

        // Need all assets for bounds checking
        let mut q_within = BTreeMap::new();
        for asset in AssetId::all() {
            q_within.insert(*asset, 10.0); // Within bounds
        }
        assert!(params.is_within_bounds(&q_within));

        let mut q_outside = BTreeMap::new();
        for asset in AssetId::all() {
            q_outside.insert(*asset, 10.0);
        }
        q_outside.insert(AssetId::USD, 3.0); // Below min (5.0)
        assert!(!params.is_within_bounds(&q_outside));

        let mut q_over = BTreeMap::new();
        for asset in AssetId::all() {
            q_over.insert(*asset, 10.0);
        }
        q_over.insert(AssetId::USD, 20.0); // Above max (15.0)
        assert!(!params.is_within_bounds(&q_over));
    }

    #[test]
    fn test_custom_risk_params() {
        let mut q_target = BTreeMap::new();
        let mut q_min = BTreeMap::new();
        let mut q_max = BTreeMap::new();

        for asset in AssetId::all() {
            q_target.insert(*asset, 5.0);
            q_min.insert(*asset, 2.0);
            q_max.insert(*asset, 10.0);
        }

        let gamma_diag = vec![2.0; 5]; // Higher penalty
        let w_diag = vec![50.0; 5]; // Lower tracking weight

        let params = RiskParams::new(
            q_target.clone(),
            gamma_diag,
            w_diag,
            2.0, // eta
            q_min,
            q_max,
            30.0, // band_bps
            0.01, // ghost_inventory_weight
        );

        assert_eq!(params.eta, 2.0);
        assert_eq!(params.price_band_bps, 30.0);
        assert_eq!(params.ghost_inventory_weight, 0.01);
        assert_eq!(params.target(AssetId::USD), 5.0);
    }

    #[test]
    fn test_penalty_increases_with_deviation() {
        let params = RiskParams::default_demo();

        let target_val = params.target(AssetId::USD);

        let mut q1 = BTreeMap::new();
        let mut q2 = BTreeMap::new();
        let mut q3 = BTreeMap::new();

        for asset in AssetId::all() {
            q1.insert(*asset, target_val);
            q2.insert(*asset, target_val + 1.0);
            q3.insert(*asset, target_val + 2.0);
        }

        let p1 = params.inventory_penalty(&q1);
        let p2 = params.inventory_penalty(&q2);
        let p3 = params.inventory_penalty(&q3);

        assert!(p1 < p2);
        assert!(p2 < p3);
    }

    #[test]
    fn test_symmetric_penalty() {
        let params = RiskParams::default_demo();

        let target_val = params.target(AssetId::USD);

        let mut q_above = BTreeMap::new();
        let mut q_below = BTreeMap::new();

        for asset in AssetId::all() {
            q_above.insert(*asset, target_val + 1.0);
            q_below.insert(*asset, target_val - 1.0);
        }

        let penalty_above = params.inventory_penalty(&q_above);
        let penalty_below = params.inventory_penalty(&q_below);

        // Should be symmetric (within tolerance)
        assert!((penalty_above - penalty_below).abs() < 1e-6);
    }

    #[test]
    fn test_matrix_dimensions() {
        let params = RiskParams::default_demo();

        // Check matrix dimensions (6 assets now)
        assert_eq!(params.gamma.nrows(), 6);
        assert_eq!(params.gamma.ncols(), 6);
        assert_eq!(params.w_track.nrows(), 6);
        assert_eq!(params.w_track.ncols(), 6);
    }

    #[test]
    fn test_all_assets_configured() {
        let params = RiskParams::default_demo();

        // All assets should have configuration
        for asset in AssetId::all() {
            assert!(params.target(*asset).is_finite());
            assert!(params.min_bound(*asset).is_finite());
            assert!(params.max_bound(*asset).is_finite());
        }
    }

    #[test]
    fn test_tracking_penalty_single_asset() {
        let params = RiskParams::default_demo();

        let mut y = BTreeMap::new();
        let mut y_ref = BTreeMap::new();

        // All at reference except EUR
        for asset in AssetId::all() {
            y_ref.insert(*asset, 0.0);
            if *asset == AssetId::EUR {
                y.insert(*asset, 0.1); // Deviate EUR
            } else {
                y.insert(*asset, 0.0);
            }
        }

        let penalty = params.tracking_penalty(&y, &y_ref);
        assert!(penalty > 0.0);
    }
}
