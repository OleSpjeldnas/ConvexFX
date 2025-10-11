// Comprehensive integration tests for ledger

#[cfg(test)]
mod tests {
    use crate::*;
    use convexfx_types::*;

    #[test]
    fn test_multi_account_operations() {
        let mut ledger = MemoryLedger::new();

        let pool = AccountId::new("pool");
        let trader1 = AccountId::new("trader1");
        let trader2 = AccountId::new("trader2");

        // Initialize pool
        ledger
            .deposit(&pool, AssetId::USD, Amount::from_units(10000))
            .unwrap();
        ledger
            .deposit(&pool, AssetId::EUR, Amount::from_units(9000))
            .unwrap();

        // Initialize traders
        ledger
            .deposit(&trader1, AssetId::USD, Amount::from_units(1000))
            .unwrap();
        ledger
            .deposit(&trader2, AssetId::EUR, Amount::from_units(1000))
            .unwrap();

        // Simulate a trade: trader1 sells USD to pool, receives EUR
        ledger
            .transfer(&trader1, &pool, AssetId::USD, Amount::from_units(100))
            .unwrap();
        ledger
            .transfer(&pool, &trader1, AssetId::EUR, Amount::from_units(90))
            .unwrap();

        // Check final balances
        assert_eq!(
            ledger.balance(&trader1, AssetId::USD),
            Amount::from_units(900)
        );
        assert_eq!(
            ledger.balance(&trader1, AssetId::EUR),
            Amount::from_units(90)
        );
        assert_eq!(
            ledger.balance(&pool, AssetId::USD),
            Amount::from_units(10100)
        );
        assert_eq!(
            ledger.balance(&pool, AssetId::EUR),
            Amount::from_units(8910)
        );
    }

    #[test]
    fn test_list_accounts() {
        let mut ledger = MemoryLedger::new();

        let acc1 = AccountId::new("acc1");
        let acc2 = AccountId::new("acc2");

        ledger.create_account(&acc1).unwrap();
        ledger.create_account(&acc2).unwrap();

        let accounts = ledger.list_accounts();
        assert_eq!(accounts.len(), 2);
        assert!(accounts.contains(&acc1));
        assert!(accounts.contains(&acc2));
    }

    #[test]
    fn test_concurrent_transfers() {
        let mut ledger = MemoryLedger::new();
        let acc1 = AccountId::new("acc1");
        let acc2 = AccountId::new("acc2");
        let acc3 = AccountId::new("acc3");

        // Initialize
        ledger.deposit(&acc1, AssetId::USD, Amount::from_units(1000)).unwrap();

        // Chain of transfers
        ledger.transfer(&acc1, &acc2, AssetId::USD, Amount::from_units(300)).unwrap();
        ledger.transfer(&acc1, &acc3, AssetId::USD, Amount::from_units(400)).unwrap();
        ledger.transfer(&acc2, &acc3, AssetId::USD, Amount::from_units(100)).unwrap();

        // Verify final state
        assert_eq!(ledger.balance(&acc1, AssetId::USD), Amount::from_units(300));
        assert_eq!(ledger.balance(&acc2, AssetId::USD), Amount::from_units(200));
        assert_eq!(ledger.balance(&acc3, AssetId::USD), Amount::from_units(500));

        // Total should be conserved
        let total = ledger.balance(&acc1, AssetId::USD).to_f64()
            + ledger.balance(&acc2, AssetId::USD).to_f64()
            + ledger.balance(&acc3, AssetId::USD).to_f64();
        assert!((total - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_insufficient_balance_protection() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("acc");

        ledger.deposit(&acc, AssetId::EUR, Amount::from_units(100)).unwrap();

        // Try to withdraw more than balance
        let result = ledger.withdraw(&acc, AssetId::EUR, Amount::from_units(200));
        assert!(result.is_err());

        // Balance should be unchanged
        assert_eq!(ledger.balance(&acc, AssetId::EUR), Amount::from_units(100));
    }

    #[test]
    fn test_negative_amount_rejection() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("acc");

        // Negative deposit should fail
        let result = ledger.deposit(&acc, AssetId::USD, Amount::from_units(-100));
        assert!(result.is_err());

        // Negative withdrawal should fail
        ledger.deposit(&acc, AssetId::USD, Amount::from_units(100)).unwrap();
        let result = ledger.withdraw(&acc, AssetId::USD, Amount::from_units(-50));
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_transfer_handling() {
        let mut ledger = MemoryLedger::new();
        let acc1 = AccountId::new("acc1");
        let acc2 = AccountId::new("acc2");

        ledger.deposit(&acc1, AssetId::JPY, Amount::from_units(100)).unwrap();

        // Zero transfer should be no-op
        ledger.transfer(&acc1, &acc2, AssetId::JPY, Amount::ZERO).unwrap();

        assert_eq!(ledger.balance(&acc1, AssetId::JPY), Amount::from_units(100));
        assert_eq!(ledger.balance(&acc2, AssetId::JPY), Amount::ZERO);
    }

    #[test]
    fn test_inventory_aggregation() {
        let mut ledger = MemoryLedger::new();

        let lp1 = AccountId::new("lp1");
        let lp2 = AccountId::new("lp2");

        ledger.deposit(&lp1, AssetId::USD, Amount::from_units(1000)).unwrap();
        ledger.deposit(&lp2, AssetId::USD, Amount::from_units(500)).unwrap();
        ledger.deposit(&lp1, AssetId::EUR, Amount::from_units(800)).unwrap();

        let inv = ledger.inventory();
        assert_eq!(inv.get(AssetId::USD), Amount::from_units(1500));
        assert_eq!(inv.get(AssetId::EUR), Amount::from_units(800));
        assert_eq!(inv.get(AssetId::JPY), Amount::ZERO);
    }

    #[test]
    fn test_snapshot_restore() {
        let mut ledger = MemoryLedger::new();
        let account = AccountId::new("test");

        ledger.deposit(&account, AssetId::CHF, Amount::from_units(100)).unwrap();

        let snapshot = ledger.snapshot();

        // Make changes
        ledger.withdraw(&account, AssetId::CHF, Amount::from_units(50)).unwrap();
        ledger.deposit(&account, AssetId::GBP, Amount::from_units(200)).unwrap();

        assert_eq!(ledger.balance(&account, AssetId::CHF), Amount::from_units(50));
        assert_eq!(ledger.balance(&account, AssetId::GBP), Amount::from_units(200));

        // Restore
        ledger.restore(&snapshot).unwrap();

        assert_eq!(ledger.balance(&account, AssetId::CHF), Amount::from_units(100));
        assert_eq!(ledger.balance(&account, AssetId::GBP), Amount::ZERO);
    }

    #[test]
    fn test_has_sufficient_balance() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("acc");

        ledger.deposit(&acc, AssetId::USD, Amount::from_units(100)).unwrap();

        assert!(ledger.has_sufficient(&acc, AssetId::USD, Amount::from_units(50)));
        assert!(ledger.has_sufficient(&acc, AssetId::USD, Amount::from_units(100)));
        assert!(!ledger.has_sufficient(&acc, AssetId::USD, Amount::from_units(101)));
    }

    #[test]
    fn test_multi_asset_account() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("multi_asset");

        // Deposit multiple assets
        for asset in AssetId::all() {
            ledger.deposit(&acc, *asset, Amount::from_units(100)).unwrap();
        }

        let balances = ledger.account_balances(&acc);
        
        // Should have all assets (updated to 6 for AUD)
        assert_eq!(balances.assets().len(), 6);
        
        for asset in AssetId::all() {
            assert_eq!(balances.get(*asset), Amount::from_units(100));
        }
    }

    #[test]
    fn test_transfer_to_self() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("self");

        ledger.deposit(&acc, AssetId::EUR, Amount::from_units(100)).unwrap();

        // Transfer to self should work
        ledger.transfer(&acc, &acc, AssetId::EUR, Amount::from_units(50)).unwrap();

        // Balance should be unchanged
        assert_eq!(ledger.balance(&acc, AssetId::EUR), Amount::from_units(100));
    }

    #[test]
    fn test_empty_account_cleanup() {
        let mut ledger = MemoryLedger::new();
        let acc = AccountId::new("temp");

        ledger.deposit(&acc, AssetId::USD, Amount::from_units(100)).unwrap();
        ledger.withdraw(&acc, AssetId::USD, Amount::from_units(100)).unwrap();

        // Account still exists but with zero balance
        let balances = ledger.account_balances(&acc);
        assert_eq!(balances.get(AssetId::USD), Amount::ZERO);
    }
}
