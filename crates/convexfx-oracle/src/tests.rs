// Comprehensive integration tests for oracle

#[cfg(test)]
mod tests {
    use crate::*;
    use convexfx_types::AssetId;

    #[test]
    fn test_oracle_staleness() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        // Should not be stale immediately
        let current_time = prices.timestamp_ms;
        assert!(!prices.is_stale(current_time, 60_000));

        // Should be stale after max age
        assert!(prices.is_stale(current_time + 120_000, 60_000));
    }

    #[test]
    fn test_cross_rate_consistency() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        // Get log-prices
        let y_eur = prices.get_ref(AssetId::EUR);
        let y_jpy = prices.get_ref(AssetId::JPY);
        let y_usd = prices.get_ref(AssetId::USD);

        // Check USD is numeraire
        assert_eq!(y_usd, 0.0);

        // Compute cross rates in linear space
        let eurusd = (y_eur - y_usd).exp();
        let jpyusd = (y_jpy - y_usd).exp();
        let eurjpy_direct = (y_eur - y_jpy).exp();  // EUR/JPY: how many JPY for 1 EUR
        let eurjpy_cross = eurusd / jpyusd;

        // Cross rate consistency: EURJPY should equal EURUSD / JPYUSD
        assert!((eurjpy_direct - eurjpy_cross).abs() < 1e-6,
            "eurjpy_direct={}, eurjpy_cross={}", eurjpy_direct, eurjpy_cross);
    }

    #[test]
    fn test_custom_prices() {
        let mut custom_prices = std::collections::BTreeMap::new();
        custom_prices.insert(AssetId::USD, 1.0);
        custom_prices.insert(AssetId::EUR, 1.2);
        custom_prices.insert(AssetId::GBP, 1.25);

        let oracle = MockOracle::with_prices(custom_prices);
        let prices = oracle.reference_prices(1).unwrap();

        let eur_log = prices.get_ref(AssetId::EUR);
        assert!((eur_log - 1.2_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_band_calculation() {
        let oracle = MockOracle::new().with_band_bps(50.0);
        let prices = oracle.reference_prices(1).unwrap();

        let eur_ref = prices.get_ref(AssetId::EUR);
        let eur_low = prices.get_low(AssetId::EUR);
        let eur_high = prices.get_high(AssetId::EUR);

        // Bands should be ±50 bps
        assert!((eur_ref - eur_low - 0.0050).abs() < 1e-6);
        assert!((eur_high - eur_ref - 0.0050).abs() < 1e-6);
    }

    #[test]
    fn test_price_mutability() {
        let mut oracle = MockOracle::new();
        
        let initial = oracle.reference_prices(1).unwrap();
        let eur_initial = initial.get_ref(AssetId::EUR);

        // Update price
        oracle.set_price(AssetId::EUR, 1.15);

        let updated = oracle.reference_prices(1).unwrap();
        let eur_updated = updated.get_ref(AssetId::EUR);

        assert_ne!(eur_initial, eur_updated);
        assert!((eur_updated - 1.15_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_all_assets_have_prices() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        // All assets should have prices
        for asset in AssetId::all() {
            let y = prices.get_ref(*asset);
            assert!(y.is_finite());
        }
    }

    #[test]
    fn test_provider_metadata() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        assert!(!prices.providers.is_empty());
        assert_eq!(prices.providers[0], "mock");
    }

    #[test]
    fn test_timestamp_freshness() {
        let oracle = MockOracle::new();
        let prices1 = oracle.reference_prices(1).unwrap();
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let prices2 = oracle.reference_prices(2).unwrap();

        // Timestamps should be different (or at least not far apart)
        assert!(prices2.timestamp_ms >= prices1.timestamp_ms);
    }

    #[test]
    fn test_band_symmetry() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        for asset in AssetId::all() {
            let ref_price = prices.get_ref(*asset);
            let low = prices.get_low(*asset);
            let high = prices.get_high(*asset);

            // Bands should be symmetric
            let lower_diff = ref_price - low;
            let upper_diff = high - ref_price;
            assert!((lower_diff - upper_diff).abs() < 1e-10);
        }
    }

    #[test]
    fn test_realistic_fx_prices() {
        let oracle = MockOracle::new();
        let prices = oracle.reference_prices(1).unwrap();

        // Check that prices are in realistic ranges
        let eur_price = prices.get_ref(AssetId::EUR).exp();
        assert!(eur_price > 1.0 && eur_price < 1.3); // EURUSD typically 1.0-1.3

        let jpy_price = prices.get_ref(AssetId::JPY).exp();
        assert!(jpy_price > 0.005 && jpy_price < 0.02); // JPYUSD typically 0.007-0.015

        let gbp_price = prices.get_ref(AssetId::GBP).exp();
        assert!(gbp_price > 1.1 && gbp_price < 1.4); // GBPUSD typically 1.2-1.35
    }
}
