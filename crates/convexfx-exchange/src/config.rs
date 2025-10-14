use serde::{Deserialize, Serialize};
use convexfx_risk::RiskParams;
use convexfx_types::AssetId;

/// Configuration for the exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    /// How often to run clearing batches (in seconds)
    pub batch_interval_seconds: u64,

    /// Maximum number of orders to process per batch
    pub max_orders_per_batch: usize,

    /// Enable WebSocket server for real-time updates
    pub enable_websocket: bool,

    /// Port for WebSocket server
    pub websocket_port: u16,

    /// Enable HTTP API server
    pub enable_api_server: bool,

    /// Port for HTTP API server
    pub api_port: u16,

    /// Solver backend to use for clearing
    pub solver_backend: SolverBackend,

    /// Risk management parameters
    pub risk_parameters: RiskParams,

    /// Initial assets to set up when exchange starts
    pub initial_assets: Vec<InitialAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialAsset {
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub is_base_currency: bool,
    pub initial_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolverBackend {
    Clarabel,
    OSQP,
    Simple,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            batch_interval_seconds: 60, // Run batches every minute
            max_orders_per_batch: 1000,
            enable_websocket: true,
            websocket_port: 8080,
            enable_api_server: true,
            api_port: 3000,
            solver_backend: SolverBackend::OSQP,
            risk_parameters: {
                let mut risk = RiskParams::default_demo();
                // Use more balanced parameters for stability
                for asset in AssetId::all() {
                    risk.q_target.insert(*asset, 10.0);
                    risk.q_min.insert(*asset, 5.0);
                    risk.q_max.insert(*asset, 15.0);
                }
                risk.gamma_diag = vec![1.0; 6]; // Moderate inventory risk
                risk.w_diag = vec![100.0; 6]; // Moderate oracle tracking
                risk.eta = 1.0;
                // Keep default price_band_bps for compatibility
                risk.rebuild_matrices();
                risk
            },
            initial_assets: vec![
                InitialAsset {
                    symbol: "USD".to_string(),
                    name: "US Dollar".to_string(),
                    decimals: 2,
                    is_base_currency: true,
                    initial_price: 1.0,
                },
                InitialAsset {
                    symbol: "EUR".to_string(),
                    name: "Euro".to_string(),
                    decimals: 2,
                    is_base_currency: false,
                    initial_price: 1.05,
                },
                InitialAsset {
                    symbol: "JPY".to_string(),
                    name: "Japanese Yen".to_string(),
                    decimals: 0,
                    is_base_currency: false,
                    initial_price: 0.009,
                },
            ],
        }
    }
}
