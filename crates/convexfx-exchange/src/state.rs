use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use convexfx_types::{AccountId, AssetId, Amount, Inventory, EpochId, OrderId, Fill};

/// Current system status and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub status: ExchangeStatus,
    pub current_epoch: EpochId,
    pub total_accounts: usize,
    pub total_orders_pending: usize,
    pub total_liquidity: BTreeMap<String, f64>,
    pub uptime_seconds: u64,
    pub last_batch_execution: Option<DateTime<Utc>>,
    pub batch_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

/// Internal exchange state management
pub struct ExchangeState {
    pub ledger: convexfx_ledger::MemoryLedger,
    pub orderbook: convexfx_orders::OrderBook,
    pub oracle: convexfx_oracle::MockOracle,
    pub clearing_engine: convexfx_clearing::ScpClearing,
    pub reporter: convexfx_report::MemoryReporter,
    pub current_epoch: EpochId,
    pub start_time: DateTime<Utc>,
    pub last_batch_time: Option<DateTime<Utc>>,
    pub is_running: bool,
}

impl ExchangeState {
    pub fn new() -> Self {
        Self {
            ledger: convexfx_ledger::MemoryLedger::new(),
            orderbook: convexfx_orders::OrderBook::new(1),
            oracle: convexfx_oracle::MockOracle::new(),
            clearing_engine: convexfx_clearing::ScpClearing::new(),
            reporter: convexfx_report::MemoryReporter::new(),
            current_epoch: 1,
            start_time: Utc::now(),
            last_batch_time: None,
            is_running: false,
        }
    }

    pub fn get_uptime_seconds(&self) -> u64 {
        (Utc::now() - self.start_time).num_seconds() as u64
    }

    pub fn get_status(&self) -> SystemStatus {
        use convexfx_ledger::Ledger;

        let inventory = self.ledger.inventory();
        let f64_map = inventory.to_f64_map();
        let mut total_liquidity = BTreeMap::new();

        for (asset, amount) in f64_map {
            total_liquidity.insert(asset.to_string(), amount);
        }

        SystemStatus {
            status: if self.is_running {
                ExchangeStatus::Running
            } else {
                ExchangeStatus::Stopped
            },
            current_epoch: self.current_epoch,
            total_accounts: self.ledger.list_accounts().len(),
            total_orders_pending: self.orderbook.commitment_count(),
            total_liquidity,
            uptime_seconds: self.get_uptime_seconds(),
            last_batch_execution: self.last_batch_time,
            batch_interval_seconds: 60, // TODO: Get from config
        }
    }
}
