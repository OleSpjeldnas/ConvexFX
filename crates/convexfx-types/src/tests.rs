// Comprehensive integration tests for types crate

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_roundtrip_serialization() {
        let order = PairOrder {
            id: "test_order".to_string(),
            trader: AccountId::new("trader_123"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({"source": "api"}),
        };

        let json = serde_json::to_string(&order).unwrap();
        let deserialized: PairOrder = serde_json::from_str(&json).unwrap();

        assert_eq!(order, deserialized);
    }

    #[test]
    fn test_inventory_consistency() {
        let mut inv = Inventory::new();

        // Add balances
        inv.add(AssetId::USD, Amount::from_units(1000));
        inv.add(AssetId::EUR, Amount::from_units(500));

        // Convert to f64 and back
        let f64_map = inv.to_f64_map();
        let inv2 = Inventory::from_f64_map(&f64_map).unwrap();

        assert_eq!(inv.get(AssetId::USD), inv2.get(AssetId::USD));
        assert_eq!(inv.get(AssetId::EUR), inv2.get(AssetId::EUR));
    }

    #[test]
    fn test_all_assets_unique() {
        let assets = AssetId::all();
        assert_eq!(assets.len(), 6); // Updated for 6 assets

        // Check indices are correct
        for asset in assets {
            assert_eq!(AssetId::from_index(asset.index()), Some(*asset));
        }
    }

    #[test]
    fn test_amount_overflow_protection() {
        let large = Amount::from_units(i64::MAX / 2);
        let result = large.checked_add(large);
        assert!(result.is_ok());

        // Test with values that would overflow i128
        let huge = Amount::from_raw(i128::MAX - 100);
        let very_huge = Amount::from_raw(200);
        let overflow = huge.checked_add(very_huge);
        assert!(overflow.is_err());
    }

    #[test]
    fn test_amount_precision() {
        let a = Amount::from_f64(123.456789012).unwrap();
        let b = Amount::from_f64(123.456789013).unwrap();
        
        // Should be able to distinguish at 9 decimal places
        assert_ne!(a, b);
        
        // But differences smaller than 1e-9 should be equal
        let c = Amount::from_f64(123.4567890120).unwrap();
        let d = Amount::from_f64(123.4567890121).unwrap();
        // At our precision these should be very close
        assert!((c.to_f64() - d.to_f64()).abs() < 1e-8);
    }

    #[test]
    fn test_inventory_zero_handling() {
        let mut inv = Inventory::new();
        
        // Adding zero should not create entry
        inv.add(AssetId::USD, Amount::ZERO);
        assert_eq!(inv.assets().len(), 0);
        
        // Adding positive then subtracting should remove
        inv.add(AssetId::USD, Amount::from_units(100));
        inv.sub(AssetId::USD, Amount::from_units(100));
        assert_eq!(inv.get(AssetId::USD), Amount::ZERO);
    }

    #[test]
    fn test_log_prices_usd_numeraire_immutable() {
        let mut prices = LogPrices::new();
        
        // Try to set USD to non-zero
        prices.set(AssetId::USD, 5.0);
        
        // Should still be zero
        assert_eq!(prices.get(AssetId::USD), 0.0);
    }

    #[test]
    fn test_prices_round_trip() {
        let mut log_prices = LogPrices::new();
        log_prices.set(AssetId::EUR, 0.09531);
        log_prices.set(AssetId::GBP, 0.22314);
        
        let linear_prices = log_prices.to_prices();
        let back_to_log = linear_prices.to_log_prices();
        
        // Should be close to original
        for asset in AssetId::all() {
            let original = log_prices.get(*asset);
            let round_trip = back_to_log.get(*asset);
            assert!((original - round_trip).abs() < 1e-6);
        }
    }

    #[test]
    fn test_fill_status_classification() {
        let full_fill = Fill {
            order_id: "o1".to_string(),
            fill_frac: 1.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 100.0,
            recv_units: 90.0,
            fees_paid: Default::default(),
        };
        assert!(full_fill.is_complete());
        assert!(!full_fill.is_partial());
        assert!(!full_fill.is_empty());

        let partial_fill = Fill {
            order_id: "o2".to_string(),
            fill_frac: 0.5,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 50.0,
            recv_units: 45.0,
            fees_paid: Default::default(),
        };
        assert!(!partial_fill.is_complete());
        assert!(partial_fill.is_partial());
        assert!(!partial_fill.is_empty());

        let no_fill = Fill {
            order_id: "o3".to_string(),
            fill_frac: 0.0,
            pay_asset: AssetId::USD,
            recv_asset: AssetId::EUR,
            pay_units: 0.0,
            recv_units: 0.0,
            fees_paid: Default::default(),
        };
        assert!(!no_fill.is_complete());
        assert!(!no_fill.is_partial());
        assert!(no_fill.is_empty());
    }

    #[test]
    fn test_asset_string_conversion() {
        for asset in AssetId::all() {
            let s = asset.as_str();
            let parsed = AssetId::from_str(s).unwrap();
            assert_eq!(*asset, parsed);
            
            // Case insensitive
            let lower = AssetId::from_str(&s.to_lowercase()).unwrap();
            assert_eq!(*asset, lower);
        }
        
        // Invalid asset
        assert!(AssetId::from_str("INVALID").is_none());
    }

    #[test]
    fn test_account_id_operations() {
        let acc1 = AccountId::new("trader1");
        let acc2 = AccountId::new("trader2");
        let acc3 = AccountId::new("trader1");
        
        assert_eq!(acc1, acc3);
        assert_ne!(acc1, acc2);
        assert_eq!(acc1.as_str(), "trader1");
    }

    #[test]
    fn test_order_validation() {
        use crate::order::*;
        
        let valid_order = PairOrder {
            id: "valid".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        };
        
        assert_eq!(valid_order.min_fill(), 0.5);
        assert!(valid_order.has_limit());
        assert!(valid_order.log_limit().is_some());
    }

    #[test]
    fn test_amount_edge_cases() {
        // Zero
        assert!(Amount::ZERO.is_zero());
        assert!(!Amount::ZERO.is_positive());
        assert!(!Amount::ZERO.is_negative());
        
        // Negative
        let neg = Amount::from_units(-100);
        assert!(neg.is_negative());
        assert!(!neg.is_positive());
        
        // Absolute value
        assert_eq!(neg.abs(), Amount::from_units(100));
        
        // Multiplication
        let a = Amount::from_units(10);
        let doubled = a.checked_mul_int(2).unwrap();
        assert_eq!(doubled, Amount::from_units(20));
    }
}
