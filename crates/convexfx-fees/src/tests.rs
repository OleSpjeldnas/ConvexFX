// Comprehensive integration tests for fees

#[cfg(test)]
mod tests {
    use crate::*;
    use convexfx_risk::RiskParams;
    use convexfx_types::{AssetId, Fill};
    use std::collections::BTreeMap;

    #[test]
    fn test_inventory_aware_fees() {
        let fees = InventoryAwareFees::with_defaults();
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 12.0); // Above target (10.0)
        }

        let fill = Fill {
            order_id: "order1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_post, &risk);

        assert_eq!(fee_lines.len(), 1);
        assert!(fee_lines[0].amount > 0.0);
        // Since inventory is above target, multiplier should be > 1
        assert!(fee_lines[0].multiplier > 1.0);
    }

    #[test]
    fn test_fee_multiplier_bounds() {
        let fees = InventoryAwareFees::with_defaults();
        let risk = RiskParams::default_demo();

        // Test extreme deviations
        let mut q_way_above = BTreeMap::new();
        for asset in AssetId::all() {
            q_way_above.insert(*asset, 50.0); // Way above target
        }

        let fill = Fill {
            order_id: "order1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_way_above, &risk);

        // Multiplier should be capped at max (3.0)
        assert!(fee_lines[0].multiplier <= 3.0);
    }

    #[test]
    fn test_fee_rebate_below_target() {
        let fees = InventoryAwareFees::with_defaults();
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 8.0); // Below target (10.0)
        }

        let fill = Fill {
            order_id: "order1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_post, &risk);

        // Multiplier should be < 1 (rebate)
        assert!(fee_lines[0].multiplier < 1.0);
    }

    #[test]
    fn test_base_fee_calculation() {
        let config = FeeConfig {
            base_fee_bps: 5.0,
            lambda_inventory: 0.0, // No inventory effect
            multiplier_min: 1.0,
            multiplier_max: 1.0,
        };

        let fees = InventoryAwareFees::new(config);
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 10.0); // At target
        }

        let fill = Fill {
            order_id: "order1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 10000.0,
            recv_units: 9000.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_post, &risk);

        // Fee should be 5 bps of 10000 = 5.0
        assert!((fee_lines[0].amount - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_multiple_fills() {
        let fees = InventoryAwareFees::with_defaults();
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 10.0);
        }

        let fills = vec![
            Fill {
                order_id: "order1".to_string(),
                fill_frac: 1.0,
                pay_asset: AssetId::USD,
                recv_asset: AssetId::EUR,
                pay_units: 1000.0,
                recv_units: 900.0,
                fees_paid: BTreeMap::new(),
            },
            Fill {
                order_id: "order2".to_string(),
                fill_frac: 0.5,
                pay_asset: AssetId::EUR,
                recv_asset: AssetId::GBP,
                pay_units: 500.0,
                recv_units: 400.0,
                fees_paid: BTreeMap::new(),
            },
        ];

        let fee_lines = fees.compute_fees(&fills, &q_post, &risk);

        assert_eq!(fee_lines.len(), 2);
        assert_eq!(fee_lines[0].order_id, "order1");
        assert_eq!(fee_lines[1].order_id, "order2");
    }

    #[test]
    fn test_different_assets_different_gradients() {
        let fees = InventoryAwareFees::with_defaults();
        let mut risk = RiskParams::default_demo();

        // Set different targets so assets have different gradients
        risk.q_target.insert(AssetId::USD, 10.0);
        risk.q_target.insert(AssetId::EUR, 8.0);

        let mut q_post = BTreeMap::new();
        q_post.insert(AssetId::USD, 12.0); // 2 above
        q_post.insert(AssetId::EUR, 12.0); // 4 above

        let fills = vec![
            Fill {
                order_id: "usd_order".to_string(),
                fill_frac: 1.0,
                pay_asset: AssetId::USD,
                recv_asset: AssetId::JPY,
                pay_units: 1000.0,
                recv_units: 100000.0,
                fees_paid: BTreeMap::new(),
            },
            Fill {
                order_id: "eur_order".to_string(),
                fill_frac: 1.0,
                pay_asset: AssetId::EUR,
                recv_asset: AssetId::JPY,
                pay_units: 1000.0,
                recv_units: 110000.0,
                fees_paid: BTreeMap::new(),
            },
        ];

        let fee_lines = fees.compute_fees(&fills, &q_post, &risk);

        // EUR has larger deviation, so should have higher multiplier
        let usd_multiplier = fee_lines.iter()
            .find(|f| f.asset == AssetId::USD)
            .map(|f| f.multiplier)
            .unwrap();
        
        let eur_multiplier = fee_lines.iter()
            .find(|f| f.asset == AssetId::EUR)
            .map(|f| f.multiplier)
            .unwrap();

        assert!(eur_multiplier > usd_multiplier);
    }

    #[test]
    fn test_zero_fill() {
        let fees = InventoryAwareFees::with_defaults();
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 10.0);
        }

        let fill = Fill {
            order_id: "zero".to_string(),
            fill_frac: 0.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 0.0,
            recv_units: 0.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_post, &risk);

        // Zero fill should have zero fee
        assert_eq!(fee_lines.len(), 1);
        assert_eq!(fee_lines[0].amount, 0.0);
    }

    #[test]
    fn test_custom_fee_config() {
        let config = FeeConfig {
            base_fee_bps: 10.0,
            lambda_inventory: 1.0,
            multiplier_min: 0.1,
            multiplier_max: 5.0,
        };

        let fees = InventoryAwareFees::new(config);
        let risk = RiskParams::default_demo();

        let mut q_post = BTreeMap::new();
        for asset in AssetId::all() {
            q_post.insert(*asset, 10.0);
        }

        let fill = Fill {
            order_id: "custom".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 1000.0,
            recv_units: 900.0,
            fees_paid: BTreeMap::new(),
        };

        let fee_lines = fees.compute_fees(&[fill], &q_post, &risk);

        // Should use custom base fee (10 bps)
        assert!(fee_lines[0].amount > 0.0);
    }
}

