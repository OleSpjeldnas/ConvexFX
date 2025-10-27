# ConvexFX Ledger

Account balance management and transaction processing.

## Overview

Provides ledger abstractions for tracking user balances across multiple assets with atomic transaction support.

## Key Types

### Ledger Trait

```rust
pub trait Ledger: Send + Sync {
    fn get_balance(&self, account: &AccountId, asset: AssetId) 
        -> Result<Amount, LedgerError>;
    
    fn credit(&mut self, account: &AccountId, asset: AssetId, amount: Amount) 
        -> Result<(), LedgerError>;
    
    fn debit(&mut self, account: &AccountId, asset: AssetId, amount: Amount) 
        -> Result<(), LedgerError>;
    
    fn transfer(&mut self, from: &AccountId, to: &AccountId, 
                asset: AssetId, amount: Amount) 
        -> Result<(), LedgerError>;
}
```

## Implementations

### MemoryLedger

In-memory ledger for testing and development:

```rust
let mut ledger = MemoryLedger::new();

// Credit account
ledger.credit(&"alice".into(), AssetId::USD, 1000)?;

// Check balance
let balance = ledger.get_balance(&"alice".into(), AssetId::USD)?;

// Transfer
ledger.transfer(&"alice".into(), &"bob".into(), AssetId::USD, 100)?;
```

**Features**:
- Fast in-memory operations
- Thread-safe with RwLock
- Automatic account creation
- Balance validation

## Usage

```rust
use convexfx_ledger::{MemoryLedger, Ledger};
use convexfx_types::{AssetId, AccountId};

let mut ledger = MemoryLedger::new();

// Fund account
ledger.credit(&"trader1".into(), AssetId::USD, 10000)?;
ledger.credit(&"trader1".into(), AssetId::EUR, 8000)?;

// Execute trade
ledger.debit(&"trader1".into(), AssetId::USD, 1000)?;
ledger.credit(&"trader1".into(), AssetId::EUR, 860)?;

// Query balances
let usd_balance = ledger.get_balance(&"trader1".into(), AssetId::USD)?;
```

## Error Handling

```rust
pub enum LedgerError {
    InsufficientBalance { 
        account: AccountId, 
        asset: AssetId, 
        required: Amount, 
        available: Amount 
    },
    AccountNotFound(AccountId),
    InvalidAmount,
}
```

## Testing

```bash
cargo test -p convexfx-ledger
```

## Production Extensions

For production use, implement the `Ledger` trait with:
- **Persistent Storage**: RocksDB, PostgreSQL
- **Atomic Transactions**: Full ACID properties
- **Audit Logging**: All balance changes recorded
- **Snapshots**: Point-in-time balance views

