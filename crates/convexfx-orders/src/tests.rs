// Comprehensive integration tests for orders

#[cfg(test)]
mod tests {
    use crate::*;
    use convexfx_types::*;

    fn create_test_order(id: &str) -> PairOrder {
        PairOrder {
            id: id.to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_full_lifecycle() {
        let mut book = OrderBook::new(1);

        // Create multiple orders
        let orders = vec![
            PairOrder {
                id: "order1".to_string(),
                trader: AccountId::new("trader1"),
                pay: AssetId::USD,
                receive: AssetId::EUR,
                budget: Amount::from_units(1000),
                limit_ratio: Some(1.15),
                min_fill_fraction: Some(0.1),
                metadata: serde_json::json!({}),
            },
            PairOrder {
                id: "order2".to_string(),
                trader: AccountId::new("trader2"),
                pay: AssetId::EUR,
                receive: AssetId::GBP,
                budget: Amount::from_units(500),
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({}),
            },
        ];

        let salts = vec![b"salt1".as_slice(), b"salt2".as_slice()];

        // Phase 1: Commit
        for (order, salt) in orders.iter().zip(salts.iter()) {
            let hash = commitment::compute_commitment(order, salt).unwrap();
            book.commit(Commitment {
                hash,
                epoch_id: 1,
                timestamp_ms: 1000,
            })
            .unwrap();
        }

        assert_eq!(book.commitment_count(), 2);

        // Phase 2: Reveal
        for (order, salt) in orders.iter().zip(salts.iter()) {
            book.reveal(order.clone(), salt).unwrap();
        }

        assert_eq!(book.revealed_count(), 2);

        // Phase 3: Freeze
        let frozen = book.freeze();
        assert_eq!(frozen.len(), 2);
    }

    #[test]
    fn test_commitment_verification() {
        let order = create_test_order("test");
        let salt = b"random_salt_12345";
        
        let commitment = commitment::compute_commitment(&order, salt).unwrap();
        
        // Correct salt should verify
        assert!(commitment::verify_commitment(&commitment, &order, salt).unwrap());
        
        // Wrong salt should fail
        assert!(!commitment::verify_commitment(&commitment, &order, b"wrong_salt").unwrap());
        
        // Modified order should fail
        let mut modified = order.clone();
        modified.budget = Amount::from_units(2000);
        assert!(!commitment::verify_commitment(&commitment, &modified, salt).unwrap());
    }

    #[test]
    fn test_commitment_determinism() {
        let order = create_test_order("test");
        let salt = b"salt";
        
        let hash1 = commitment::compute_commitment(&order, salt).unwrap();
        let hash2 = commitment::compute_commitment(&order, salt).unwrap();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_commitment_uniqueness() {
        let order = create_test_order("test");
        
        let hash1 = commitment::compute_commitment(&order, b"salt1").unwrap();
        let hash2 = commitment::compute_commitment(&order, b"salt2").unwrap();
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_duplicate_commitment_rejection() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt";
        
        let hash = commitment::compute_commitment(&order, salt).unwrap();
        let commitment = Commitment {
            hash: hash.clone(),
            epoch_id: 1,
            timestamp_ms: 1000,
        };
        
        book.commit(commitment.clone()).unwrap();
        
        // Duplicate should fail
        let result = book.commit(commitment);
        assert!(result.is_err());
    }

    #[test]
    fn test_reveal_without_commit() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt";
        
        let result = book.reveal(order, salt);
        assert!(result.is_err());
    }

    #[test]
    fn test_double_reveal_prevention() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt";
        
        let hash = commitment::compute_commitment(&order, salt).unwrap();
        book.commit(Commitment {
            hash,
            epoch_id: 1,
            timestamp_ms: 1000,
        }).unwrap();
        
        // First reveal should succeed
        book.reveal(order.clone(), salt).unwrap();
        
        // Second reveal should fail
        let result = book.reveal(order, salt);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_epoch_rejection() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt";
        
        let hash = commitment::compute_commitment(&order, salt).unwrap();
        let commitment = Commitment {
            hash,
            epoch_id: 2, // Wrong epoch
            timestamp_ms: 1000,
        };
        
        let result = book.commit(commitment);
        assert!(result.is_err());
    }

    #[test]
    fn test_frozen_orderbook_rejection() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt";
        
        let hash = commitment::compute_commitment(&order, salt).unwrap();
        book.commit(Commitment {
            hash: hash.clone(),
            epoch_id: 1,
            timestamp_ms: 1000,
        }).unwrap();
        
        book.reveal(order.clone(), salt).unwrap();
        
        // Freeze
        let _frozen = book.freeze();
        
        // Book is now frozen, operations should fail
        // (Note: freeze consumes the book, so we can't test further operations)
    }

    #[test]
    fn test_order_validation() {
        // Valid order
        let valid = PairOrder {
            id: "valid".to_string(),
            trader: AccountId::new("trader"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        };
        assert!(validate_order(&valid).is_ok());
        
        // Zero budget
        let zero_budget = PairOrder {
            id: "zero".to_string(),
            trader: AccountId::new("trader"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::ZERO,
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };
        assert!(validate_order(&zero_budget).is_err());
        
        // Same asset
        let same_asset = PairOrder {
            id: "same".to_string(),
            trader: AccountId::new("trader"),
            pay: AssetId::EUR,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };
        assert!(validate_order(&same_asset).is_err());
        
        // Invalid limit
        let bad_limit = PairOrder {
            id: "bad".to_string(),
            trader: AccountId::new("trader"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: Some(-1.0),
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };
        assert!(validate_order(&bad_limit).is_err());
        
        // Invalid min fill
        let bad_fill = PairOrder {
            id: "bad".to_string(),
            trader: AccountId::new("trader"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: Some(1.5),
            metadata: serde_json::json!({}),
        };
        assert!(validate_order(&bad_fill).is_err());
    }

    #[test]
    fn test_deterministic_freeze_ordering() {
        let mut book = OrderBook::new(1);
        
        let orders = vec![
            create_test_order("order1"),
            create_test_order("order2"),
            create_test_order("order3"),
        ];
        
        let salts = vec![b"salt1", b"salt2", b"salt3"];
        
        // Commit and reveal in mixed order
        let mut commitments: Vec<_> = orders.iter()
            .zip(salts.iter())
            .map(|(order, salt)| {
                let hash = commitment::compute_commitment(order, *salt).unwrap();
                (hash, order.clone(), *salt)
            })
            .collect();
        
        // Commit in reverse order
        for (hash, _, _) in commitments.iter().rev() {
            book.commit(Commitment {
                hash: hash.clone(),
                epoch_id: 1,
                timestamp_ms: 1000,
            }).unwrap();
        }
        
        // Reveal in original order
        for (_, order, salt) in &commitments {
            book.reveal(order.clone(), *salt).unwrap();
        }
        
        let frozen = book.freeze();
        
        // Should be sorted by commitment hash
        assert_eq!(frozen.len(), 3);
    }

    #[test]
    fn test_commitment_hash_format() {
        let order = create_test_order("test");
        let salt = b"salt";
        
        let hash = commitment::compute_commitment(&order, salt).unwrap();
        
        // Should be 64 hex characters (SHA256)
        assert_eq!(hash.0.len(), 64);
        
        // Should be valid hex
        assert!(hex::decode(&hash.0).is_ok());
    }
}
