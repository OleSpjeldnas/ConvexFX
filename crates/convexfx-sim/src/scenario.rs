use serde::{Deserialize, Serialize};
use crate::testbed::Testbed;

/// Order flow distribution pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderFlowPattern {
    /// Uniform distribution across all pairs
    Uniform,
    /// Bias toward specific pairs with weights
    Biased {
        /// Percentage bias (0-100) toward target pairs
        bias_pct: f64,
        /// Target pairs (pay -> receive)
        target_pairs: Vec<(String, String)>,
    },
    /// One-sided pressure (buy wall or sell pressure)
    OneSided {
        /// Asset being bought/sold
        asset: String,
        /// Percentage of orders in this direction
        concentration_pct: f64,
    },
    /// Basket orders (diversified)
    Basket {
        /// Basket weights per asset
        weights: Vec<(String, f64)>,
    },
}

/// Scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    pub name: String,
    pub description: String,
    
    /// Number of orders per epoch
    pub num_orders: usize,
    
    /// Number of epochs to simulate
    pub num_epochs: usize,
    
    /// Order flow pattern
    pub flow_pattern: OrderFlowPattern,
    
    /// Budget range (min, max) in millions
    pub budget_range_m: (f64, f64),
    
    /// Percentage of orders with limits
    pub limit_orders_pct: f64,
    
    /// Limit tightness (bps inside mid)
    pub limit_tightness_bps: Option<f64>,
    
    /// Min fill fraction range
    pub min_fill_range: Option<(f64, f64)>,
    
    /// Override tracking weights (0 = free market)
    pub override_tracking_weights: Option<Vec<f64>>,
    
    /// Override price bands (bps)
    pub override_band_bps: Option<f64>,
    
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    
    /// Expected outcomes for validation
    pub expected_outcomes: Option<ExpectedOutcomes>,
}

/// Expected outcomes for scenario validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcomes {
    /// Max iterations expected
    pub max_iterations: Option<usize>,
    
    /// Min fill rate
    pub min_fill_rate: Option<f64>,
    
    /// Max slippage p90 (bps)
    pub max_slippage_p90_bps: Option<f64>,
    
    /// Max coherence error (bps)
    pub max_coherence_error_bps: Option<f64>,
    
    /// Max inventory utilization
    pub max_inventory_util: Option<f64>,
    
    /// Max limit violations (%)
    pub max_limit_violations_pct: Option<f64>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        ScenarioConfig {
            name: "default".to_string(),
            description: "Default balanced scenario".to_string(),
            num_orders: 100,
            num_epochs: 1,
            flow_pattern: OrderFlowPattern::Uniform,
            budget_range_m: (0.1, 1.0),
            limit_orders_pct: 0.0,
            limit_tightness_bps: None,
            min_fill_range: None,
            override_tracking_weights: None,
            override_band_bps: None,
            seed: Some(42),
            expected_outcomes: None,
        }
    }
}

/// Scenario for simulation
#[derive(Debug, Clone)]
pub struct Scenario {
    pub config: ScenarioConfig,
    pub testbed: Testbed,
}

impl Scenario {
    pub fn new(config: ScenarioConfig, testbed: Testbed) -> Self {
        Scenario { config, testbed }
    }

    pub fn default_scenario() -> Self {
        Scenario {
            config: ScenarioConfig::default(),
            testbed: Testbed::default(),
        }
    }
    
    /// Scenario A: Empty epoch
    pub fn empty_epoch() -> Self {
        Self::new(
            ScenarioConfig {
                name: "A_empty_epoch".to_string(),
                description: "Empty epoch for sanity check".to_string(),
                num_orders: 0,
                num_epochs: 1,
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(2),
                    min_fill_rate: Some(0.0),
                    max_slippage_p90_bps: Some(0.1),
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.0),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }
    
    /// Scenario B: Balanced two-sided flow
    pub fn balanced_flow() -> Self {
        Self::new(
            ScenarioConfig {
                name: "B_balanced_flow".to_string(),
                description: "Balanced two-sided flow across all pairs".to_string(),
                num_orders: 100, // Increased with Clarabel solver
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::Uniform,
                budget_range_m: (0.1, 1.0),
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(3),
                    min_fill_rate: Some(0.95),
                    max_slippage_p90_bps: Some(50.0), // Realistic for Clarabel solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.8),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }
    
    /// Scenario C: One-sided EUR buy wall
    pub fn eur_buy_wall() -> Self {
        Self::new(
            ScenarioConfig {
                name: "C_eur_buy_wall".to_string(),
                description: "One-sided EUR buy pressure (stress test)".to_string(),
                num_orders: 80, // Increased with Clarabel solver
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::OneSided {
                    asset: "EUR".to_string(),
                    concentration_pct: 60.0,
                },
                budget_range_m: (0.3, 1.0),
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(4),
                    min_fill_rate: Some(0.70), // Tightened with better solver
                    max_slippage_p90_bps: Some(50.0), // Realistic for Clarabel solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(1.0),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }
    
    /// Scenario D: GBP sell pressure with tight limits
    pub fn gbp_sell_limits() -> Self {
        Self::new(
            ScenarioConfig {
                name: "D_gbp_sell_limits".to_string(),
                description: "GBP sell pressure with tight limit orders".to_string(),
                num_orders: 60, // Increased with Clarabel solver
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::OneSided {
                    asset: "GBP".to_string(),
                    concentration_pct: 50.0,
                },
                budget_range_m: (0.3, 1.0),
                limit_orders_pct: 80.0,
                limit_tightness_bps: Some(5.0),
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(4),
                    min_fill_rate: Some(0.60), // Tightened with better solver
                    max_slippage_p90_bps: Some(50.0), // Realistic for Clarabel solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.9),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }
    
    /// Scenario F: Oracle-light mode (price discovery)
    pub fn price_discovery() -> Self {
        let mut testbed = Testbed::standard_5_asset();
        testbed.band_bps = 100.0; // Widen bands

        Self::new(
            ScenarioConfig {
                name: "F_price_discovery".to_string(),
                description: "Oracle-light mode with wide bands".to_string(),
                num_orders: 50, // Increased with Clarabel solver
                num_epochs: 3,
                flow_pattern: OrderFlowPattern::Uniform,
                budget_range_m: (0.5, 2.0),
                override_tracking_weights: Some(vec![0.0; 6]), // W = 0 (updated to 6 assets)
                override_band_bps: Some(100.0),
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(4),
                    min_fill_rate: Some(0.80), // Tightened with better solver
                    max_slippage_p90_bps: Some(50.0), // Tightened with better solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(1.0),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            testbed,
        )
    }

    /// Scenario G: High-frequency trading stress test
    pub fn high_frequency_stress() -> Self {
        Self::new(
            ScenarioConfig {
                name: "G_high_frequency_stress".to_string(),
                description: "High-frequency stress test with many small orders".to_string(),
                num_orders: 200, // Large number of small orders
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::Uniform,
                budget_range_m: (0.01, 0.1), // Small orders
                limit_orders_pct: 30.0,
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(3),
                    min_fill_rate: Some(0.90),
                    max_slippage_p90_bps: Some(50.0), // Realistic for Clarabel solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.5),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }

    /// Scenario H: Multi-asset basket trading
    pub fn basket_trading() -> Self {
        Self::new(
            ScenarioConfig {
                name: "H_basket_trading".to_string(),
                description: "Basket trading with diversified asset allocation".to_string(),
                num_orders: 40,
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::Basket {
                    weights: vec![
                        ("USD".to_string(), 0.3),
                        ("EUR".to_string(), 0.25),
                        ("GBP".to_string(), 0.2),
                        ("JPY".to_string(), 0.15),
                        ("CHF".to_string(), 0.1),
                    ],
                },
                budget_range_m: (0.5, 2.0),
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(3),
                    min_fill_rate: Some(0.85),
                    max_slippage_p90_bps: Some(50.0), // Realistic for Clarabel solver
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.7),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }

    /// Scenario I: Bilateral currency trading matrix
    pub fn bilateral_trading() -> Self {
        Self::new(
            ScenarioConfig {
                name: "I_bilateral_trading".to_string(),
                description: "Comprehensive bilateral trading between all currency pairs".to_string(),
                num_orders: 60, // 6 assets × 10 orders per pair
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::Biased {
                    bias_pct: 100.0, // Focus entirely on bilateral pairs
                    target_pairs: vec![
                        ("USD".to_string(), "EUR".to_string()),
                        ("EUR".to_string(), "GBP".to_string()),
                        ("GBP".to_string(), "JPY".to_string()),
                        ("JPY".to_string(), "CHF".to_string()),
                        ("CHF".to_string(), "USD".to_string()),
                        ("USD".to_string(), "JPY".to_string()),
                        ("EUR".to_string(), "JPY".to_string()),
                        ("GBP".to_string(), "CHF".to_string()),
                    ],
                },
                budget_range_m: (0.3, 1.0),
                limit_orders_pct: 20.0, // Some limit orders to test constraint handling
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(3),
                    min_fill_rate: Some(0.90),
                    max_slippage_p90_bps: Some(50.0), // Higher slippage expected for complex bilateral trading
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.8),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            Testbed::standard_5_asset(),
        )
    }

    /// Scenario J: Moderate slippage optimized trading
    pub fn moderate_slippage_trading() -> Self {
        let mut testbed = Testbed::standard_5_asset();
        testbed.band_bps = 15.0; // Tight bands balance slippage control and fills

        Self::new(
            ScenarioConfig {
                name: "J_moderate_slippage".to_string(),
                description: "Moderate slippage optimization with balanced parameters".to_string(),
                num_orders: 80,
                num_epochs: 1,
                flow_pattern: OrderFlowPattern::Uniform,
                budget_range_m: (0.5, 1.5),
                // Use ultra_low_slippage risk parameters for extreme optimization
                // This includes USD-notional normalization, ghost inventory, and adaptive bands
                expected_outcomes: Some(ExpectedOutcomes {
                    max_iterations: Some(5),
                    min_fill_rate: Some(0.60), // More realistic for moderate optimization
                    max_slippage_p90_bps: Some(30.0), // Conservative expectation
                    max_coherence_error_bps: Some(0.001),
                    max_inventory_util: Some(0.6),
                    max_limit_violations_pct: Some(0.0),
                }),
                ..Default::default()
            },
            testbed,
        )
    }
}


