use convexfx_types::{AccountId, Amount, AssetId, PairOrder};
use crate::scenario::{OrderFlowPattern, ScenarioConfig};

/// Order generator for simulations with various flow patterns
pub struct OrderGenerator {
    seed: u64,
}

impl OrderGenerator {
    pub fn new() -> Self {
        OrderGenerator { seed: 42 }
    }
    
    pub fn with_seed(seed: u64) -> Self {
        OrderGenerator { seed }
    }

    /// Generate a simple buy order
    pub fn generate_buy_order(
        id: &str,
        trader: &str,
        pay: AssetId,
        receive: AssetId,
        budget_units: i64,
    ) -> PairOrder {
        PairOrder {
            id: id.to_string(),
            trader: AccountId::new(trader),
            pay,
            receive,
            budget: Amount::from_units(budget_units),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Generate a batch of random orders based on scenario config
    pub fn generate_batch(&self, count: usize) -> Vec<PairOrder> {
        (0..count)
            .map(|i| {
                Self::generate_buy_order(
                    &format!("order_{}", i),
                    &format!("trader_{}", i % 10),
                    AssetId::USD,
                    AssetId::EUR,
                    100_000, // 0.1M units
                )
            })
            .collect()
    }
    
    /// Generate orders based on scenario configuration
    pub fn generate_orders(&self, config: &ScenarioConfig, epoch_id: u64) -> Vec<PairOrder> {
        // Use seed + epoch for reproducibility
        let mut rng = SimpleRng::new(self.seed + epoch_id);
        
        let mut orders = match &config.flow_pattern {
            OrderFlowPattern::Uniform => {
                self.generate_uniform_orders(config, &mut rng)
            }
            OrderFlowPattern::OneSided { asset, concentration_pct } => {
                self.generate_one_sided_orders(
                    config,
                    asset,
                    *concentration_pct,
                    &mut rng,
                )
            }
            OrderFlowPattern::Biased { bias_pct, target_pairs } => {
                self.generate_biased_orders(
                    config,
                    *bias_pct,
                    target_pairs,
                    &mut rng,
                )
            }
            OrderFlowPattern::Basket { weights } => {
                self.generate_basket_orders(config, weights, &mut rng)
            }
        };

        // Apply limits and min-fill based on config
        orders = self.apply_order_constraints(orders, config, &mut rng);

        orders
    }
    
    /// Generate uniform distribution across all pairs
    fn generate_uniform_orders(
        &self,
        config: &ScenarioConfig,
        rng: &mut SimpleRng,
    ) -> Vec<PairOrder> {
        let assets = AssetId::all();
        let mut all_pairs = Vec::new();
        
        // Create all directed pairs (A→B is different from B→A)
        for pay in assets {
            for receive in assets {
                if pay != receive {
                    all_pairs.push((*pay, *receive));
                }
            }
        }
        
        let mut orders = Vec::new();
        
        for i in 0..config.num_orders {
            // Pick random pair
            let pair_idx = rng.next_usize(all_pairs.len());
            let (pay, receive) = all_pairs[pair_idx];
            
            // Random budget in range
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay,
                receive,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "uniform"}),
            });
        }
        
        orders
    }
    
    /// Generate one-sided flow (concentrated buying or selling)
    fn generate_one_sided_orders(
        &self,
        config: &ScenarioConfig,
        target_asset: &str,
        concentration_pct: f64,
        rng: &mut SimpleRng,
    ) -> Vec<PairOrder> {
        let target = AssetId::from_str(target_asset).unwrap_or(AssetId::EUR);
        let assets = AssetId::all();
        
        let num_concentrated = ((config.num_orders as f64) * concentration_pct / 100.0) as usize;
        
        let mut orders = Vec::new();
        
        // Concentrated orders (buying target asset)
        for i in 0..num_concentrated {
            // Pick a random asset to pay with (not the target)
            let pay_assets: Vec<_> = assets.iter()
                .filter(|&&a| a != target)
                .copied()
                .collect();
            let pay = pay_assets[rng.next_usize(pay_assets.len())];
            
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay,
                receive: target,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "concentrated_buy"}),
            });
        }
        
        // Random orders for the rest
        for i in num_concentrated..config.num_orders {
            let pay = assets[rng.next_usize(assets.len())];
            let mut receive = assets[rng.next_usize(assets.len())];

            // Ensure pay != receive
            while pay == receive {
                receive = assets[rng.next_usize(assets.len())];
            }
            
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay,
                receive,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "random"}),
            });
        }
        
        orders
    }
    
    /// Generate biased flow toward specific pairs
    fn generate_biased_orders(
        &self,
        config: &ScenarioConfig,
        bias_pct: f64,
        target_pairs: &[(String, String)],
        rng: &mut SimpleRng,
    ) -> Vec<PairOrder> {
        let num_biased = ((config.num_orders as f64) * bias_pct / 100.0) as usize;
        
        let mut orders = Vec::new();
        let assets = AssetId::all();
        
        // Biased orders
        for i in 0..num_biased {
            if target_pairs.is_empty() {
                continue;
            }
            
            let pair_idx = rng.next_usize(target_pairs.len());
            let (pay_str, recv_str) = &target_pairs[pair_idx];
            
            let pay = AssetId::from_str(pay_str).unwrap_or(AssetId::USD);
            let receive = AssetId::from_str(recv_str).unwrap_or(AssetId::EUR);
            
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay,
                receive,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "biased"}),
            });
        }
        
        // Random orders
        for i in num_biased..config.num_orders {
            let pay = assets[rng.next_usize(assets.len())];
            let mut receive = assets[rng.next_usize(assets.len())];

            while pay == receive {
                receive = assets[rng.next_usize(assets.len())];
            }
            
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay,
                receive,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "random"}),
            });
        }
        
        orders
    }
    
    /// Generate basket orders (diversified)
    fn generate_basket_orders(
        &self,
        config: &ScenarioConfig,
        weights: &[(String, f64)],
        rng: &mut SimpleRng,
    ) -> Vec<PairOrder> {
        let mut orders = Vec::new();
        
        // For now, implement as separate orders for each basket component
        // A proper basket order would be a single order with multiple receives
        
        for i in 0..config.num_orders {
            // Pick a weight
            if weights.is_empty() {
                continue;
            }
            
            let weight_idx = rng.next_usize(weights.len());
            let (asset_str, _weight) = &weights[weight_idx];
            
            let receive = AssetId::from_str(asset_str).unwrap_or(AssetId::EUR);
            let budget = self.sample_budget(config, rng);
            
            orders.push(PairOrder {
                id: format!("order_{}", i),
                trader: AccountId::new(&format!("trader_{}", i % 50)),
                pay: AssetId::USD, // Basket typically pays in base currency
                receive,
                budget,
                limit_ratio: None,
                min_fill_fraction: None,
                metadata: serde_json::json!({"type": "basket"}),
            });
        }
        
        orders
    }
    
    /// Sample budget from configured range
    fn sample_budget(&self, config: &ScenarioConfig, rng: &mut SimpleRng) -> Amount {
        let (min_m, max_m) = config.budget_range_m;
        let budget_m = min_m + rng.next_f64() * (max_m - min_m);

        // Budgets are expressed in millions, just like the inventory inputs.
        // Using `from_units` would scale them up by 1e6 and make every order
        // several orders of magnitude larger than the available inventory,
        // forcing the solver to clamp fills near zero. We instead keep the
        // natural "millions" scale so slippage and fills stay comparable to the
        // risk configuration.
        Amount::from_f64(budget_m).expect("valid budget amount")
    }
    
    /// Apply limit orders and min-fill constraints
    fn apply_order_constraints(
        &self,
        mut orders: Vec<PairOrder>,
        config: &ScenarioConfig,
        rng: &mut SimpleRng,
    ) -> Vec<PairOrder> {
        // Apply limits to percentage of orders
        if config.limit_orders_pct > 0.0 {
            let num_with_limits = ((orders.len() as f64) * config.limit_orders_pct / 100.0) as usize;
            
            for i in 0..num_with_limits.min(orders.len()) {
                if let Some(tightness_bps) = config.limit_tightness_bps {
                    // Set limit ratio based on tightness
                    // tightness_bps = how many bps inside the mid
                    // For a buy order (pay j, receive i), limit is max price of i in terms of j
                    // We approximate: limit ≈ mid × (1 + tightness_bps/10000)
                    
                    let factor = 1.0 + (tightness_bps / 10_000.0);
                    orders[i].limit_ratio = Some(factor);
                }
            }
        }
        
        // Apply min-fill fractions
        if let Some((min_fill_min, min_fill_max)) = config.min_fill_range {
            for order in orders.iter_mut() {
                if rng.next_f64() < 0.3 {
                    // 30% of orders get min-fill constraint
                    let min_fill = min_fill_min + rng.next_f64() * (min_fill_max - min_fill_min);
                    order.min_fill_fraction = Some(min_fill);
                }
            }
        }
        
        orders
    }
}

impl Default for OrderGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple pseudo-random number generator for reproducibility
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng { state: seed }
    }
    
    fn next(&mut self) -> u64 {
        // Linear congruential generator
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }
    
    fn next_f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    
    fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next() as usize) % max
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::ScenarioConfig;

    #[test]
    fn test_generate_order() {
        let order = OrderGenerator::generate_buy_order(
            "test1",
            "trader1",
            AssetId::USD,
            AssetId::EUR,
            1000,
        );

        assert_eq!(order.id, "test1");
        assert_eq!(order.pay, AssetId::USD);
        assert_eq!(order.receive, AssetId::EUR);
    }

    #[test]
    fn test_generate_batch() {
        let generator = OrderGenerator::new();
        let batch = generator.generate_batch(10);
        assert_eq!(batch.len(), 10);
    }
    
    #[test]
    fn test_uniform_distribution() {
        let generator = OrderGenerator::with_seed(123);
        let mut config = ScenarioConfig::default();
        config.num_orders = 100;
        config.flow_pattern = OrderFlowPattern::Uniform;
        
        let orders = generator.generate_orders(&config, 0);
        
        assert_eq!(orders.len(), 100);
        
        // Check variety of pairs
        let mut pairs = std::collections::HashSet::new();
        for order in &orders {
            pairs.insert((order.pay, order.receive));
        }
        
        // Should have multiple different pairs
        assert!(pairs.len() > 5, "Expected variety in pairs, got {}", pairs.len());
    }
    
    #[test]
    fn test_one_sided_flow() {
        let generator = OrderGenerator::with_seed(456);
        let mut config = ScenarioConfig::default();
        config.num_orders = 100;
        config.flow_pattern = OrderFlowPattern::OneSided {
            asset: "EUR".to_string(),
            concentration_pct: 60.0,
        };
        
        let orders = generator.generate_orders(&config, 0);
        
        assert_eq!(orders.len(), 100);
        
        // Count EUR buys
        let eur_buys = orders.iter()
            .filter(|o| o.receive == AssetId::EUR)
            .count();
        
        // Should be around 60% ±10%
        assert!(eur_buys >= 50 && eur_buys <= 70, 
            "Expected ~60 EUR buys, got {}", eur_buys);
    }
    
    #[test]
    fn test_reproducibility() {
        let gen1 = OrderGenerator::with_seed(789);
        let gen2 = OrderGenerator::with_seed(789);
        
        let config = ScenarioConfig::default();
        
        let orders1 = gen1.generate_orders(&config, 0);
        let orders2 = gen2.generate_orders(&config, 0);
        
        assert_eq!(orders1.len(), orders2.len());
        
        for (o1, o2) in orders1.iter().zip(orders2.iter()) {
            assert_eq!(o1.pay, o2.pay);
            assert_eq!(o1.receive, o2.receive);
            assert_eq!(o1.budget, o2.budget);
        }
    }
}
