use convexfx_types::{AccountId, Amount, AssetId, ConvexFxError, Inventory, Result};
use std::collections::BTreeMap;

use crate::ledger::{Ledger, LedgerSnapshot};

/// In-memory ledger implementation
/// Suitable for testing and demo purposes
#[derive(Debug, Clone)]
pub struct MemoryLedger {
    accounts: BTreeMap<AccountId, Inventory>,
}

impl MemoryLedger {
    /// Create a new empty in-memory ledger
    pub fn new() -> Self {
        MemoryLedger {
            accounts: BTreeMap::new(),
        }
    }

    /// Initialize with pre-funded accounts
    pub fn with_accounts(accounts: BTreeMap<AccountId, Inventory>) -> Self {
        MemoryLedger { accounts }
    }

    /// Get mutable reference to account inventory (creates if not exists)
    fn get_or_create_account_mut(&mut self, account: &AccountId) -> &mut Inventory {
        self.accounts
            .entry(account.clone())
            .or_insert_with(Inventory::new)
    }

    /// Get reference to account inventory
    fn get_account(&self, account: &AccountId) -> Option<&Inventory> {
        self.accounts.get(account)
    }
}

impl Default for MemoryLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger for MemoryLedger {
    fn deposit(&mut self, lp: &AccountId, asset: AssetId, amount: Amount) -> Result<()> {
        if amount.is_negative() {
            return Err(ConvexFxError::InvalidAmount(
                "deposit amount must be non-negative".to_string(),
            ));
        }

        let account = self.get_or_create_account_mut(lp);
        account.add(asset, amount);
        Ok(())
    }

    fn withdraw(&mut self, lp: &AccountId, asset: AssetId, amount: Amount) -> Result<()> {
        if amount.is_negative() {
            return Err(ConvexFxError::InvalidAmount(
                "withdrawal amount must be non-negative".to_string(),
            ));
        }

        let current_balance = self.balance(lp, asset);
        if current_balance < amount {
            return Err(ConvexFxError::InsufficientBalance(
                lp.to_string(),
                asset.to_string(),
            ));
        }

        let account = self.get_or_create_account_mut(lp);
        account.sub(asset, amount);
        Ok(())
    }

    fn transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Amount,
    ) -> Result<()> {
        if amount.is_negative() {
            return Err(ConvexFxError::InvalidAmount(
                "transfer amount must be non-negative".to_string(),
            ));
        }

        if amount.is_zero() {
            return Ok(()); // No-op for zero transfers
        }

        // Check sufficient balance
        let from_balance = self.balance(from, asset);
        if from_balance < amount {
            return Err(ConvexFxError::InsufficientBalance(
                from.to_string(),
                asset.to_string(),
            ));
        }

        // Perform transfer
        {
            let from_account = self.get_or_create_account_mut(from);
            from_account.sub(asset, amount);
        }
        {
            let to_account = self.get_or_create_account_mut(to);
            to_account.add(asset, amount);
        }

        Ok(())
    }

    fn balance(&self, account: &AccountId, asset: AssetId) -> Amount {
        self.get_account(account)
            .map(|inv| inv.get(asset))
            .unwrap_or(Amount::ZERO)
    }

    fn inventory(&self) -> Inventory {
        let mut total = Inventory::new();
        for account_inv in self.accounts.values() {
            for asset in AssetId::all() {
                let amount = account_inv.get(*asset);
                if !amount.is_zero() {
                    total.add(*asset, amount);
                }
            }
        }
        total
    }

    fn account_balances(&self, account: &AccountId) -> Inventory {
        self.get_account(account)
            .cloned()
            .unwrap_or_else(Inventory::new)
    }

    fn create_account(&mut self, account: &AccountId) -> Result<()> {
        self.accounts
            .entry(account.clone())
            .or_insert_with(Inventory::new);
        Ok(())
    }

    fn list_accounts(&self) -> Vec<AccountId> {
        self.accounts.keys().cloned().collect()
    }

    fn snapshot(&self) -> LedgerSnapshot {
        LedgerSnapshot {
            accounts: self.accounts.clone(),
        }
    }

    fn restore(&mut self, snapshot: &LedgerSnapshot) -> Result<()> {
        self.accounts = snapshot.accounts.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_withdraw() {
        let mut ledger = MemoryLedger::new();
        let account = AccountId::new("lp1");

        ledger
            .deposit(&account, AssetId::USD, Amount::from_units(1000))
            .unwrap();
        assert_eq!(
            ledger.balance(&account, AssetId::USD),
            Amount::from_units(1000)
        );

        ledger
            .withdraw(&account, AssetId::USD, Amount::from_units(300))
            .unwrap();
        assert_eq!(
            ledger.balance(&account, AssetId::USD),
            Amount::from_units(700)
        );
    }

    #[test]
    fn test_insufficient_balance() {
        let mut ledger = MemoryLedger::new();
        let account = AccountId::new("lp1");

        ledger
            .deposit(&account, AssetId::EUR, Amount::from_units(100))
            .unwrap();

        let result = ledger.withdraw(&account, AssetId::EUR, Amount::from_units(200));
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer() {
        let mut ledger = MemoryLedger::new();
        let alice = AccountId::new("alice");
        let bob = AccountId::new("bob");

        ledger
            .deposit(&alice, AssetId::GBP, Amount::from_units(500))
            .unwrap();

        ledger
            .transfer(&alice, &bob, AssetId::GBP, Amount::from_units(200))
            .unwrap();

        assert_eq!(
            ledger.balance(&alice, AssetId::GBP),
            Amount::from_units(300)
        );
        assert_eq!(ledger.balance(&bob, AssetId::GBP), Amount::from_units(200));
    }

    #[test]
    fn test_inventory() {
        let mut ledger = MemoryLedger::new();
        let lp1 = AccountId::new("lp1");
        let lp2 = AccountId::new("lp2");

        ledger
            .deposit(&lp1, AssetId::USD, Amount::from_units(1000))
            .unwrap();
        ledger
            .deposit(&lp2, AssetId::USD, Amount::from_units(500))
            .unwrap();
        ledger
            .deposit(&lp1, AssetId::EUR, Amount::from_units(800))
            .unwrap();

        let inv = ledger.inventory();
        assert_eq!(inv.get(AssetId::USD), Amount::from_units(1500));
        assert_eq!(inv.get(AssetId::EUR), Amount::from_units(800));
    }

    #[test]
    fn test_snapshot_restore() {
        let mut ledger = MemoryLedger::new();
        let account = AccountId::new("test");

        ledger
            .deposit(&account, AssetId::CHF, Amount::from_units(100))
            .unwrap();

        let snapshot = ledger.snapshot();

        ledger
            .withdraw(&account, AssetId::CHF, Amount::from_units(50))
            .unwrap();
        assert_eq!(
            ledger.balance(&account, AssetId::CHF),
            Amount::from_units(50)
        );

        ledger.restore(&snapshot).unwrap();
        assert_eq!(
            ledger.balance(&account, AssetId::CHF),
            Amount::from_units(100)
        );
    }
}


