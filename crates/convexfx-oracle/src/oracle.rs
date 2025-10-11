use convexfx_types::{EpochId, Result};

use crate::reference_prices::RefPrices;

/// Oracle trait for fetching reference prices
pub trait Oracle {
    /// Get reference prices for an epoch
    fn reference_prices(&self, at: EpochId) -> Result<RefPrices>;

    /// Get current reference prices
    fn current_prices(&self) -> Result<RefPrices> {
        // Default implementation uses epoch 0 for "now"
        self.reference_prices(0)
    }
}


