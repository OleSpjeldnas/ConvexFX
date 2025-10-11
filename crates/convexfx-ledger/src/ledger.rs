use convexfx_types::{AccountId, Amount, AssetId, Inventory, Result};

/// Ledger trait for managing account balances and transfers
/// Provides an abstraction that can be implemented for in-memory, database, or on-chain storage
pub trait Ledger {
    /// Deposit assets from external source into an LP account
    fn deposit(&mut self, lp: &AccountId, asset: AssetId, amount: Amount) -> Result<()>;

    /// Withdraw assets from an LP account to external destination
    fn withdraw(&mut self, lp: &AccountId, asset: AssetId, amount: Amount) -> Result<()>;

    /// Transfer assets between accounts
    fn transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Amount,
    ) -> Result<()>;

    /// Get balance for a specific account and asset
    fn balance(&self, account: &AccountId, asset: AssetId) -> Amount;

    /// Get total pool inventory (sum across all LP accounts)
    fn inventory(&self) -> Inventory;

    /// Get all balances for an account
    fn account_balances(&self, account: &AccountId) -> Inventory;

    /// Check if account has sufficient balance
    fn has_sufficient(&self, account: &AccountId, asset: AssetId, required: Amount) -> bool {
        self.balance(account, asset) >= required
    }

    /// Create a new account (if it doesn't exist)
    fn create_account(&mut self, account: &AccountId) -> Result<()>;

    /// List all accounts
    fn list_accounts(&self) -> Vec<AccountId>;

    /// Get a snapshot of all account balances (for checkpoint/restore)
    fn snapshot(&self) -> LedgerSnapshot;

    /// Restore from a snapshot
    fn restore(&mut self, snapshot: &LedgerSnapshot) -> Result<()>;
}

/// Snapshot of ledger state for checkpoint/restore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LedgerSnapshot {
    pub accounts: std::collections::BTreeMap<AccountId, Inventory>,
}


