use convexfx_ledger::MemoryLedger;
use convexfx_orders::OrderBook;
use convexfx_oracle::MockOracle;
use convexfx_clearing::ScpClearing;
use convexfx_report::MemoryReporter;
// SolverBackend is defined in convexfx-solver but accessed through clearing
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<Mutex<MemoryLedger>>,
    pub orderbook: Arc<Mutex<OrderBook>>,
    pub oracle: Arc<Mutex<MockOracle>>,
    pub clearing_engine: Arc<ScpClearing>,
    pub reporter: Arc<Mutex<MemoryReporter>>,
    pub current_epoch: Arc<Mutex<u64>>,
    pub epoch_states: Arc<Mutex<BTreeMap<u64, String>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            ledger: Arc::new(Mutex::new(MemoryLedger::new())),
            orderbook: Arc::new(Mutex::new(OrderBook::new(1))),
            oracle: Arc::new(Mutex::new(MockOracle::new())),
            clearing_engine: Arc::new(ScpClearing::new()),
            reporter: Arc::new(Mutex::new(MemoryReporter::new())),
            current_epoch: Arc::new(Mutex::new(1)),
            epoch_states: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}


