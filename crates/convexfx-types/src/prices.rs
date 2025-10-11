use crate::AssetId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Log-prices (y) in solver space
/// USD is numeraire with y_USD = 0
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogPrices {
    pub y: BTreeMap<AssetId, f64>,
}

impl LogPrices {
    /// Create new log-prices with USD numeraire
    pub fn new() -> Self {
        let mut y = BTreeMap::new();
        y.insert(AssetId::USD, 0.0);
        LogPrices { y }
    }

    /// Create from map (ensures USD = 0)
    pub fn from_map(mut y: BTreeMap<AssetId, f64>) -> Self {
        y.insert(AssetId::USD, 0.0);
        LogPrices { y }
    }

    /// Get log-price for an asset
    pub fn get(&self, asset: AssetId) -> f64 {
        self.y.get(&asset).copied().unwrap_or(0.0)
    }

    /// Set log-price for an asset (ignored if asset is USD)
    pub fn set(&mut self, asset: AssetId, value: f64) {
        if asset != AssetId::USD {
            self.y.insert(asset, value);
        }
    }

    /// Convert to linear prices (p_i = exp(y_i))
    pub fn to_prices(&self) -> Prices {
        let p = self
            .y
            .iter()
            .map(|(asset, y)| (*asset, y.exp()))
            .collect();
        Prices { p }
    }

    /// Get cross-rate (asset1/asset2)
    pub fn cross_rate(&self, asset1: AssetId, asset2: AssetId) -> f64 {
        let y1 = self.get(asset1);
        let y2 = self.get(asset2);
        (y1 - y2).exp()
    }
}

impl Default for LogPrices {
    fn default() -> Self {
        Self::new()
    }
}

/// Linear prices (p_i = exp(y_i))
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prices {
    pub p: BTreeMap<AssetId, f64>,
}

impl Prices {
    /// Create new prices with USD = 1.0
    pub fn new() -> Self {
        let mut p = BTreeMap::new();
        p.insert(AssetId::USD, 1.0);
        Prices { p }
    }

    /// Get price for an asset
    pub fn get(&self, asset: AssetId) -> f64 {
        self.p.get(&asset).copied().unwrap_or(1.0)
    }

    /// Convert to log-prices
    pub fn to_log_prices(&self) -> LogPrices {
        let y = self
            .p
            .iter()
            .map(|(asset, p)| (*asset, p.ln()))
            .collect();
        LogPrices::from_map(y)
    }

    /// Get cross-rate (asset1/asset2)
    pub fn cross_rate(&self, asset1: AssetId, asset2: AssetId) -> f64 {
        self.get(asset1) / self.get(asset2)
    }
}

impl Default for Prices {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_prices() {
        let mut log_prices = LogPrices::new();
        log_prices.set(AssetId::EUR, 0.09531); // ln(1.1)

        let prices = log_prices.to_prices();
        assert!((prices.get(AssetId::EUR) - 1.1).abs() < 1e-6);
        assert_eq!(prices.get(AssetId::USD), 1.0);
    }

    #[test]
    fn test_cross_rates() {
        let mut log_prices = LogPrices::new();
        log_prices.set(AssetId::EUR, 0.09531); // ln(1.1) - 1 EUR = 1.1 USD
        log_prices.set(AssetId::JPY, -4.60517); // ln(0.01) - 1 JPY = 0.01 USD

        let eurusd = log_prices.cross_rate(AssetId::EUR, AssetId::USD);
        let jpyusd = log_prices.cross_rate(AssetId::JPY, AssetId::USD);

        assert!((eurusd - 1.1).abs() < 1e-4);
        assert!((jpyusd - 0.01).abs() < 1e-4);

        // Check consistency via log prices: y_EUR - y_JPY should give EUR/JPY
        let y_eur = log_prices.get(AssetId::EUR);
        let y_jpy = log_prices.get(AssetId::JPY);
        let eurjpy_from_logs = (y_eur - y_jpy).exp();
        let eurjpy_direct = log_prices.cross_rate(AssetId::EUR, AssetId::JPY);

        assert!((eurjpy_from_logs - eurjpy_direct).abs() < 1e-6); // Consistency check
    }

    #[test]
    fn test_usd_numeraire() {
        let mut log_prices = LogPrices::new();
        log_prices.set(AssetId::USD, 5.0); // Should be ignored

        assert_eq!(log_prices.get(AssetId::USD), 0.0);
    }
}


