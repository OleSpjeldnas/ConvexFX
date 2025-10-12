use convexfx_types::{AssetId, EpochId, Result, AssetRegistry};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::oracle::Oracle;
use crate::reference_prices::RefPrices;

/// Mock oracle with configurable prices
/// Useful for testing and demos
#[derive(Debug, Clone)]
pub struct MockOracle {
    prices: BTreeMap<String, f64>, // Store by symbol for flexibility
    band_bps: f64,
    registry: Arc<Mutex<AssetRegistry>>,
}

impl MockOracle {
    /// Create a new mock oracle with default FX prices
    pub fn new() -> Self {
        let mut registry = AssetRegistry::new();

        // Register default assets
        registry.register_asset("USD".to_string(), "US Dollar".to_string(), 2, true);
        registry.register_asset("EUR".to_string(), "Euro".to_string(), 2, false);
        registry.register_asset("JPY".to_string(), "Japanese Yen".to_string(), 0, false);
        registry.register_asset("GBP".to_string(), "British Pound".to_string(), 2, false);
        registry.register_asset("CHF".to_string(), "Swiss Franc".to_string(), 2, false);
        registry.register_asset("AUD".to_string(), "Australian Dollar".to_string(), 2, false);

        let mut prices = BTreeMap::new();
        prices.insert("USD".to_string(), 1.0);
        prices.insert("EUR".to_string(), 1.1000); // EURUSD = 1.1
        prices.insert("JPY".to_string(), 0.0100); // USDJPY = 100
        prices.insert("GBP".to_string(), 1.2500); // GBPUSD = 1.25
        prices.insert("CHF".to_string(), 1.0800); // CHFUSD = 1.08
        prices.insert("AUD".to_string(), 0.7500); // AUDUSD = 0.75

        MockOracle {
            prices,
            band_bps: 20.0, // ±20 bps default
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    /// Create with custom prices (linear prices, not log)
    pub fn with_prices(prices: BTreeMap<String, f64>) -> Self {
        let mut registry = AssetRegistry::new();

        // Register assets from the prices map
        for (symbol, _) in &prices {
            registry.register_asset(symbol.clone(), format!("{} Currency", symbol), 2, symbol == "USD");
        }

        MockOracle {
            prices,
            band_bps: 20.0,
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    /// Set price band in basis points
    pub fn with_band_bps(mut self, band_bps: f64) -> Self {
        self.band_bps = band_bps;
        self
    }

    /// Update a price
    pub fn set_price(&mut self, symbol: &str, price: f64) {
        self.prices.insert(symbol.to_string(), price);
    }

    /// Add a new asset to the oracle
    pub fn add_asset(&mut self, symbol: String, name: String, price: f64, decimals: u32, is_base: bool) {
        let mut registry = self.registry.lock().unwrap();
        registry.register_asset(symbol.clone(), name, decimals, is_base);
        self.prices.insert(symbol, price);
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Convert linear prices to log-prices
    fn to_log_prices(&self) -> BTreeMap<AssetId, f64> {
        let registry = self.registry.lock().unwrap();
        self.prices
            .iter()
            .map(|(symbol, price)| {
                let asset_id = AssetId::new(symbol.clone());
                let log_price = if *symbol == "USD" {
                    0.0 // USD is numeraire
                } else {
                    price.ln()
                };
                (asset_id, log_price)
            })
            .collect()
    }
}

impl Default for MockOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl Oracle for MockOracle {
    fn reference_prices(&self, _at: EpochId) -> Result<RefPrices> {
        let y_ref = self.to_log_prices();
        let timestamp_ms = Self::current_timestamp_ms();

        Ok(RefPrices::new(
            y_ref,
            self.band_bps,
            timestamp_ms,
            vec!["mock".to_string()],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_oracle_default() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        // Check USD is numeraire
        assert_eq!(prices.get_ref(AssetId::USD), 0.0);

        // Check EUR is positive (since EURUSD > 1)
        let eur_log = prices.get_ref(AssetId::EUR);
        assert!(eur_log > 0.0);
        assert!((eur_log - 1.1_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_mock_oracle_bands() {
        let oracle = MockOracle::new().with_band_bps(50.0);
        let prices = oracle.reference_prices(1).unwrap();

        let eur_ref = prices.get_ref(AssetId::EUR);
        let eur_low = prices.get_low(AssetId::EUR);
        let eur_high = prices.get_high(AssetId::EUR);

        // Check bands are approximately ±50 bps
        assert!((eur_ref - eur_low - 0.0050).abs() < 1e-6);
        assert!((eur_high - eur_ref - 0.0050).abs() < 1e-6);
    }

    #[test]
    fn test_custom_prices() {
        let mut custom_prices = BTreeMap::new();
        custom_prices.insert(AssetId::USD, 1.0);
        custom_prices.insert(AssetId::EUR, 1.2);

        let oracle = MockOracle::with_prices(custom_prices);
        let prices = oracle.reference_prices(1).unwrap();

        let eur_log = prices.get_ref(AssetId::EUR);
        assert!((eur_log - 1.2_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_set_price() {
        let mut oracle = MockOracle::new();
        oracle.set_price(AssetId::EUR, 1.15);

        let prices = oracle.reference_prices(1).unwrap();
        let eur_log = prices.get_ref(AssetId::EUR);
        assert!((eur_log - 1.15_f64.ln()).abs() < 1e-10);
    }
}


