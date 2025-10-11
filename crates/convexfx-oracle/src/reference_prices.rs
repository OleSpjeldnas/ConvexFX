use convexfx_types::AssetId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Reference prices with bands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefPrices {
    /// Reference log-prices (y_ref), with USD = 0
    pub y_ref: BTreeMap<AssetId, f64>,
    /// Lower band for each asset (log-space)
    pub band_low: BTreeMap<AssetId, f64>,
    /// Upper band for each asset (log-space)
    pub band_high: BTreeMap<AssetId, f64>,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Oracle data providers
    pub providers: Vec<String>,
}

impl RefPrices {
    /// Create new reference prices
    pub fn new(
        y_ref: BTreeMap<AssetId, f64>,
        band_bps: f64,
        timestamp_ms: u64,
        providers: Vec<String>,
    ) -> Self {
        let band_low = y_ref
            .iter()
            .map(|(asset, y)| (*asset, y - band_bps / 10000.0))
            .collect();

        let band_high = y_ref
            .iter()
            .map(|(asset, y)| (*asset, y + band_bps / 10000.0))
            .collect();

        RefPrices {
            y_ref,
            band_low,
            band_high,
            timestamp_ms,
            providers,
        }
    }

    /// Get reference log-price for an asset
    pub fn get_ref(&self, asset: AssetId) -> f64 {
        self.y_ref.get(&asset).copied().unwrap_or(0.0)
    }

    /// Get lower band for an asset
    pub fn get_low(&self, asset: AssetId) -> f64 {
        self.band_low.get(&asset).copied().unwrap_or(0.0)
    }

    /// Get upper band for an asset
    pub fn get_high(&self, asset: AssetId) -> f64 {
        self.band_high.get(&asset).copied().unwrap_or(0.0)
    }

    /// Check if data is stale (age in milliseconds)
    pub fn is_stale(&self, current_time_ms: u64, max_age_ms: u64) -> bool {
        current_time_ms.saturating_sub(self.timestamp_ms) > max_age_ms
    }
}

/// Price band specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PriceBand {
    pub lower_bps: f64,
    pub upper_bps: f64,
}

impl Default for PriceBand {
    fn default() -> Self {
        PriceBand {
            lower_bps: 20.0,
            upper_bps: 20.0,
        }
    }
}


