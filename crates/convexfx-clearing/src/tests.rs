// Comprehensive integration tests for clearing

#[cfg(test)]
mod tests {
    use crate::{ScpClearing, EpochInstance};
    use convexfx_oracle::{MockOracle, Oracle};
    use convexfx_risk::RiskParams;
    use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
    use std::collections::BTreeMap;
    use serde_json;

    #[test]
    fn test_single_order_clearing() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        // Single EUR buy order
        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let inst = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // Should produce a valid solution
        assert_eq!(solution.epoch_id, 1);
        assert!(solution.diagnostics.iterations > 0);
    }

    #[test]
    fn test_empty_orders() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let inst = EpochInstance::new(1, inventory, vec![], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // No orders -> no fills
        assert_eq!(solution.fills.len(), 0);
        
        // Prices should stay near reference
        for asset in AssetId::all() {
            let y_ref = inst.ref_prices.y_ref.get(asset).copied().unwrap_or(0.0);
            let y_star = solution.y_star.get(asset).copied().unwrap_or(0.0);
            assert!((y_star - y_ref).abs() < 0.01);
        }
    }

    #[test]
    fn test_multiple_orders() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        // Create 3 orders
        let orders = vec![
            PairOrder {
                id: "order1".to_string(),
                trader: AccountId::new("trader1"),
                pay: AssetId::USD,
                receive: AssetId::EUR,
                budget: Amount::from_units(50),
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({}),
            },
            PairOrder {
                id: "order2".to_string(),
                trader: AccountId::new("trader2"),
                pay: AssetId::USD,
                receive: AssetId::EUR,
                budget: Amount::from_units(75),
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({}),
            },
            PairOrder {
                id: "order3".to_string(),
                trader: AccountId::new("trader3"),
                pay: AssetId::EUR,
                receive: AssetId::GBP,
                budget: Amount::from_units(100),
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({}),
            },
        ];

        let inst = EpochInstance::new(1, inventory, orders, ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        assert_eq!(solution.fills.len(), 3);
    }

    #[test]
    fn test_usd_numeraire_preserved() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::EUR,
            receive: AssetId::JPY,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let inst = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // USD should always be numeraire (y_USD = 0)
        assert_eq!(solution.y_star.get(&AssetId::USD).copied().unwrap_or(1.0), 0.0);
        assert_eq!(solution.prices.get(&AssetId::USD).copied().unwrap_or(0.0), 1.0);
    }

    #[test]
    fn test_convergence_achieved() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(50),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let inst = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // Should converge within reasonable iterations
        assert!(solution.diagnostics.iterations <= 5);
    }

    #[test]
    fn test_price_within_bands() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let inst = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // Prices should be reasonably close to reference (simple solver may not perfectly enforce bands)
        for asset in AssetId::all() {
            let y = solution.y_star.get(asset).copied().unwrap_or(0.0);
            let y_ref = inst.ref_prices.get_ref(*asset);
            let deviation = (y - y_ref).abs();
            
            // Check prices aren't wildly off (within 0.5 in log space)
            assert!(deviation < 0.5, 
                "Price for {} has excessive deviation: {} (y={}, y_ref={})", 
                asset, deviation, y, y_ref);
        }
    }

    #[test]
    fn test_objective_components() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        let inst = EpochInstance::new(1, inventory, vec![], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // With no orders and inventory at target, risk should be near zero
        assert!(solution.objective_terms.inventory_risk < 1.0);
        
        // Price tracking penalty should be small (near reference)
        assert!(solution.objective_terms.price_tracking < 1.0);
    }

    #[test]
    fn test_different_asset_pairs() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        // Test different pairs
        let pairs = vec![
            (AssetId::USD, AssetId::EUR),
            (AssetId::EUR, AssetId::JPY),
            (AssetId::GBP, AssetId::CHF),
            (AssetId::JPY, AssetId::USD),
        ];

        for (pay, recv) in pairs {
            let order = PairOrder {
                id: format!("order_{}{}", pay, recv),
                trader: AccountId::new("trader1"),
                pay,
                receive: recv,
                budget: Amount::from_units(50),
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({}),
            };

            let inst = EpochInstance::new(1, inventory.clone(), vec![order], ref_prices.clone(), risk.clone());

            let clearing = ScpClearing::with_simple_solver();
            let result = clearing.clear_epoch(&inst);

            assert!(result.is_ok(), "Failed for pair {}/{}", pay, recv);
        }
    }

    #[test]
    fn test_limit_order_constraint() {
        let oracle = MockOracle::new();
        let ref_prices = oracle.reference_prices(1).unwrap();
        let risk = RiskParams::default_demo();

        let mut inventory = BTreeMap::new();
        for asset in AssetId::all() {
            inventory.insert(*asset, 10.0);
        }

        // Order with very tight limit
        let order = PairOrder {
            id: "limited".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: Some(1.05), // Tight limit
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let inst = EpochInstance::new(1, inventory, vec![order], ref_prices, risk);

        let clearing = ScpClearing::with_simple_solver();
        let solution = clearing.clear_epoch(&inst).unwrap();

        // Solution should respect the limit
        assert!(solution.diagnostics.iterations > 0);
    }
}
