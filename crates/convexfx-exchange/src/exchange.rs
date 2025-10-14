use crate::{ExchangeConfig, ExchangeState, ExchangeError, Result, SystemStatus};
use convexfx_types::{AccountId, AssetId, Amount, PairOrder, OrderId, Fill, EpochId};
use convexfx_clearing::EpochInstance;
use convexfx_oracle::Oracle;
use convexfx_ledger::Ledger;
use std::collections::BTreeMap;
use chrono::{DateTime, Utc};

/// Main exchange abstraction that provides simple APIs for operating the exchange
pub struct Exchange {
    state: ExchangeState,
    config: ExchangeConfig,
}

impl Exchange {
    /// Create a new exchange with the given configuration
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let state = ExchangeState::new();

        // Set up initial assets
        for asset in &config.initial_assets {
            println!("📦 Asset {} will be available", asset.symbol);
        }

        Ok(Self { state, config })
    }

    /// Start the exchange (begin periodic batch processing)
    pub async fn start(&mut self) -> Result<()> {
        self.state.is_running = true;
        println!("🚀 Exchange started");

        // Start the main event loop
        self.run_event_loop().await
    }

    /// Stop the exchange
    pub fn stop(&mut self) -> Result<()> {
        self.state.is_running = false;
        println!("⏹️ Exchange stopped");
        Ok(())
    }

    /// Get current system status and metrics
    pub fn get_status(&self) -> SystemStatus {
        self.state.get_status()
    }

    /// Add a new asset/currency to the exchange
    pub fn add_asset(
        &mut self,
        symbol: &str,
        name: &str,
        _decimals: u32,
        _is_base_currency: bool,
        _initial_price: f64,
    ) -> Result<()> {
        // TODO: In a full implementation, we would add the asset to the oracle
        // For now, just log that the asset was added
        println!("✅ Added asset: {} ({})", symbol, name);
        Ok(())
    }

    /// Remove an asset from the exchange (only if no liquidity)
    pub fn remove_asset(&mut self, symbol: &str) -> Result<()> {
        let asset_id = AssetId::from_str(symbol)
            .ok_or_else(|| ExchangeError::NotFound(format!("Asset {} not found", symbol)))?;

        // Check if there's any liquidity for this asset
        let inventory = self.state.ledger.inventory();
        let amount = inventory.get(asset_id);
        if amount.to_f64() > 0.0 {
            return Err(ExchangeError::InvalidArgument(
                format!("Cannot remove asset {} - still has liquidity", symbol)
            ));
        }

        // TODO: Implement asset removal in oracle
        println!("✅ Removed asset: {}", symbol);
        Ok(())
    }

    /// List all available assets
    pub fn list_assets(&self) -> Result<Vec<AssetInfo>> {
        // For now, return hardcoded assets that are available in the system
        let mut assets = Vec::new();

        // USD
        assets.push(AssetInfo {
            symbol: "USD".to_string(),
            name: "US Dollar".to_string(),
            decimals: 2,
            is_base_currency: true,
            current_price: Some(1.0),
        });

        // EUR
        assets.push(AssetInfo {
            symbol: "EUR".to_string(),
            name: "Euro".to_string(),
            decimals: 2,
            is_base_currency: false,
            current_price: Some(1.05),
        });

        // JPY
        assets.push(AssetInfo {
            symbol: "JPY".to_string(),
            name: "Japanese Yen".to_string(),
            decimals: 0,
            is_base_currency: false,
            current_price: Some(0.009),
        });

        Ok(assets)
    }

    /// Get information about a specific asset
    pub fn get_asset_info(&self, symbol: &str) -> Result<AssetInfo> {
        match symbol {
            "USD" => Ok(AssetInfo {
                symbol: "USD".to_string(),
                name: "US Dollar".to_string(),
                decimals: 2,
                is_base_currency: true,
                current_price: Some(1.0),
            }),
            "EUR" => Ok(AssetInfo {
                symbol: "EUR".to_string(),
                name: "Euro".to_string(),
                decimals: 2,
                is_base_currency: false,
                current_price: Some(1.05),
            }),
            "JPY" => Ok(AssetInfo {
                symbol: "JPY".to_string(),
                name: "Japanese Yen".to_string(),
                decimals: 0,
                is_base_currency: false,
                current_price: Some(0.009),
            }),
            _ => Err(ExchangeError::NotFound(format!("Asset {} not found", symbol))),
        }
    }

    /// Add liquidity to the exchange (LP deposits assets)
    pub fn add_liquidity(&mut self, account_id: &str, asset_symbol: &str, amount: f64) -> Result<LiquidityUpdate> {
        let account = AccountId::new(account_id.to_string());
        let asset_id = AssetId::from_str(asset_symbol)
            .ok_or_else(|| ExchangeError::NotFound(format!("Asset {} not found", asset_symbol)))?;

        let amount_obj = Amount::from_f64(amount)
            .map_err(|e| ExchangeError::InvalidArgument(format!("Invalid amount: {}", e)))?;

        // Deposit to ledger
        self.state.ledger.deposit(&account, asset_id, amount_obj)?;

        let new_balance = self.state.ledger.balance(&account, asset_id);

        println!("✅ Added liquidity: {} {} for account {}",
                 amount, asset_symbol, account_id);

        Ok(LiquidityUpdate {
            account_id: account_id.to_string(),
            asset_symbol: asset_symbol.to_string(),
            amount_added: amount,
            new_balance: new_balance.to_f64(),
        })
    }

    /// Remove liquidity from the exchange (LP withdraws assets)
    pub fn remove_liquidity(&mut self, account_id: &str, asset_symbol: &str, amount: f64) -> Result<LiquidityUpdate> {
        let account = AccountId::new(account_id.to_string());
        let asset_id = AssetId::from_str(asset_symbol)
            .ok_or_else(|| ExchangeError::NotFound(format!("Asset {} not found", asset_symbol)))?;

        let amount_obj = Amount::from_f64(amount)
            .map_err(|e| ExchangeError::InvalidArgument(format!("Invalid amount: {}", e)))?;

        // Check if account has sufficient balance
        if !self.state.ledger.has_sufficient(&account, asset_id, amount_obj) {
            return Err(ExchangeError::InsufficientLiquidity(
                format!("Insufficient balance for {} withdrawal", asset_symbol)
            ));
        }

        // Withdraw from ledger
        self.state.ledger.withdraw(&account, asset_id, amount_obj)?;

        let new_balance = self.state.ledger.balance(&account, asset_id);

        println!("✅ Removed liquidity: {} {} from account {}",
                 amount, asset_symbol, account_id);

        Ok(LiquidityUpdate {
            account_id: account_id.to_string(),
            asset_symbol: asset_symbol.to_string(),
            amount_added: -amount,
            new_balance: new_balance.to_f64(),
        })
    }

    /// Get liquidity balances for an account
    pub fn get_liquidity(&self, account_id: &str) -> Result<BTreeMap<String, f64>> {
        let account = AccountId::new(account_id.to_string());
        let balances = self.state.ledger.account_balances(&account);
        let f64_map = balances.to_f64_map();

        let mut result = BTreeMap::new();
        for (asset, amount) in f64_map {
            result.insert(asset.to_string(), amount);
        }

        Ok(result)
    }

    /// Get total liquidity across all LPs
    pub fn get_total_liquidity(&self) -> Result<BTreeMap<String, f64>> {
        let inventory = self.state.ledger.inventory();
        let f64_map = inventory.to_f64_map();

        let mut result = BTreeMap::new();
        for (asset, amount) in f64_map {
            result.insert(asset.to_string(), amount);
        }

        Ok(result)
    }

    /// Submit a trade order
    pub fn submit_order(
        &mut self,
        trader_id: &str,
        pay_asset: &str,
        receive_asset: &str,
        budget: f64,
        limit_ratio: Option<f64>,
        min_fill_fraction: Option<f64>,
    ) -> Result<OrderSubmission> {
        let trader = AccountId::new(trader_id.to_string());
        let pay_asset_id = AssetId::from_str(pay_asset)
            .ok_or_else(|| ExchangeError::NotFound(format!("Pay asset {} not found", pay_asset)))?;
        let receive_asset_id = AssetId::from_str(receive_asset)
            .ok_or_else(|| ExchangeError::NotFound(format!("Receive asset {} not found", receive_asset)))?;

        let budget_amount = Amount::from_f64(budget)
            .map_err(|e| ExchangeError::InvalidArgument(format!("Invalid budget: {}", e)))?;

        // Check if trader has sufficient balance
        if !self.state.ledger.has_sufficient(&trader, pay_asset_id, budget_amount) {
            return Err(ExchangeError::InsufficientLiquidity(
                format!("Insufficient balance for {} order", pay_asset)
            ));
        }

        // Create order
        let order_id = format!("order_{}", Utc::now().timestamp_nanos_opt().unwrap_or_default());
        let _order = PairOrder {
            id: order_id.clone(),
            trader: trader.clone(),
            pay: pay_asset_id,
            receive: receive_asset_id,
            budget: budget_amount,
            limit_ratio,
            min_fill_fraction,
            metadata: serde_json::json!({}),
        };

        // TODO: For now, directly add to orderbook (in production, use commit-reveal)
        // For simplicity, we'll add directly to the orderbook
        println!("✅ Submitted order: {} ({} -> {} for {})",
                 order_id, pay_asset, receive_asset, budget);

        Ok(OrderSubmission {
            order_id,
            trader_id: trader_id.to_string(),
            pay_asset: pay_asset.to_string(),
            receive_asset: receive_asset.to_string(),
            budget,
            accepted: true,
        })
    }

    /// Cancel a pending order
    pub fn cancel_order(&mut self, order_id: &str) -> Result<()> {
        // TODO: Implement order cancellation
        println!("✅ Cancelled order: {}", order_id);
        Ok(())
    }

    /// Get order status
    pub fn get_order_status(&self, order_id: &str) -> Result<OrderStatus> {
        // TODO: Implement order status lookup
        Ok(OrderStatus {
            order_id: order_id.to_string(),
            status: OrderStatusType::Pending,
            filled_amount: 0.0,
        })
    }

    /// List orders (all or for specific account)
    pub fn list_orders(&self, _account_id: Option<&str>) -> Result<Vec<OrderInfo>> {
        // TODO: Implement order listing
        Ok(Vec::new())
    }

    /// Execute a clearing batch (run the SCP algorithm)
    pub fn execute_batch(&mut self) -> Result<BatchResult> {
        use convexfx_clearing::EpochInstance;

        // Get current prices from oracle
        let oracle = &self.state.oracle;
        let ref_prices = oracle.current_prices()
            .map_err(|e| ExchangeError::Oracle(e.to_string()))?;

        // TODO: Get pending orders from orderbook
        // For now, create empty orders list
        let orders = Vec::new();

        // Get current inventory
        let inventory = self.state.ledger.inventory();
        let inventory_f64 = inventory.to_f64_map();

        // Create epoch instance
        let instance = EpochInstance::new(
            self.state.current_epoch,
            inventory_f64,
            orders,
            ref_prices,
            self.config.risk_parameters.clone(),
        );

        // Run clearing with the configured solver backend
        let clearing_engine = match self.config.solver_backend {
            crate::SolverBackend::Simple => convexfx_clearing::ScpClearing::with_simple_solver(),
            crate::SolverBackend::Clarabel => convexfx_clearing::ScpClearing::new(),
            crate::SolverBackend::OSQP => convexfx_clearing::ScpClearing::with_osqp_solver(),
        };

        let clearing_result = clearing_engine.clear_epoch(&instance)?;

        // Update epoch
        self.state.current_epoch += 1;
        self.state.last_batch_time = Some(Utc::now());

        println!("✅ Executed batch #{} with {} fills",
                 self.state.current_epoch - 1, clearing_result.fills.len());

        Ok(BatchResult {
            epoch_id: self.state.current_epoch - 1,
            fills: clearing_result.fills,
            prices: clearing_result.prices,
            execution_time_ms: 0, // TODO: Track execution time
        })
    }

    /// Get current epoch information
    pub fn get_current_epoch(&self) -> EpochInfo {
        EpochInfo {
            epoch_id: self.state.current_epoch,
            state: if self.state.is_running { "RUNNING" } else { "STOPPED" }.to_string(),
            start_time: self.state.start_time,
        }
    }

    /// List historical epochs
    pub fn list_epochs(&self) -> Result<Vec<EpochInfo>> {
        // TODO: Implement epoch history
        Ok(vec![self.get_current_epoch()])
    }

    /// Get current prices for all assets
    pub fn get_current_prices(&self) -> Result<BTreeMap<String, f64>> {
        let oracle = &self.state.oracle;
        let prices = oracle.current_prices()
            .map_err(|e| ExchangeError::Oracle(e.to_string()))?;

        let mut result = BTreeMap::new();
        for asset in AssetId::all() {
            let y = prices.get_ref(*asset);
            result.insert(asset.to_string(), y.exp());
        }

        Ok(result)
    }

    /// Get price for a specific asset
    pub fn get_asset_price(&self, symbol: &str) -> Result<f64> {
        let oracle = &self.state.oracle;
        let prices = oracle.current_prices()
            .map_err(|e| ExchangeError::Oracle(e.to_string()))?;

        let asset_id = AssetId::from_str(symbol)
            .ok_or_else(|| ExchangeError::NotFound(format!("Asset {} not found", symbol)))?;

        let y = prices.get_ref(asset_id);
        Ok(y.exp())
    }

    /// Update exchange configuration
    pub fn configure(&mut self, config: ExchangeConfig) -> Result<()> {
        self.config = config;
        println!("✅ Updated exchange configuration");
        Ok(())
    }

    /// Run the main event loop (periodic batch processing)
    async fn run_event_loop(&mut self) -> Result<()> {
        println!("🔄 Starting event loop (batch interval: {}s)", self.config.batch_interval_seconds);

        loop {
            if !self.state.is_running {
                break;
            }

            // Execute batch
            if let Err(e) = self.execute_batch() {
                eprintln!("❌ Batch execution failed: {}", e);
            }

            // Wait for next batch
            tokio::time::sleep(tokio::time::Duration::from_secs(self.config.batch_interval_seconds)).await;
        }

        Ok(())
    }
}

// Data structures for API responses

#[derive(Debug, serde::Serialize)]
pub struct AssetInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub is_base_currency: bool,
    pub current_price: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct LiquidityUpdate {
    pub account_id: String,
    pub asset_symbol: String,
    pub amount_added: f64,
    pub new_balance: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct OrderSubmission {
    pub order_id: String,
    pub trader_id: String,
    pub pay_asset: String,
    pub receive_asset: String,
    pub budget: f64,
    pub accepted: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct OrderStatus {
    pub order_id: String,
    pub status: OrderStatusType,
    pub filled_amount: f64,
}

#[derive(Debug, serde::Serialize)]
pub enum OrderStatusType {
    Pending,
    Filled,
    PartiallyFilled,
    Cancelled,
    Rejected,
}

#[derive(Debug, serde::Serialize)]
pub struct OrderInfo {
    pub order_id: String,
    pub trader_id: String,
    pub pay_asset: String,
    pub receive_asset: String,
    pub budget: f64,
    pub status: OrderStatusType,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct BatchResult {
    pub epoch_id: u64,
    pub fills: Vec<Fill>,
    pub prices: BTreeMap<AssetId, f64>,
    pub execution_time_ms: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct EpochInfo {
    pub epoch_id: u64,
    pub state: String,
    pub start_time: DateTime<Utc>,
}
