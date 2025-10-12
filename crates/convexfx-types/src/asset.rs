use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Asset registry for dynamic asset management
#[derive(Debug, Clone, Default)]
pub struct AssetRegistry {
    assets: HashMap<String, AssetInfo>,
    indices: HashMap<String, usize>,
    next_index: usize,
}

/// Information about an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub is_base_currency: bool,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new asset
    pub fn register_asset(&mut self, symbol: String, name: String, decimals: u32, is_base_currency: bool) -> AssetId {
        if self.assets.contains_key(&symbol) {
            panic!("Asset {} already registered", symbol);
        }

        let asset_info = AssetInfo {
            symbol: symbol.clone(),
            name,
            decimals,
            is_base_currency,
        };

        let index = self.next_index;
        self.next_index += 1;

        self.assets.insert(symbol.clone(), asset_info);
        self.indices.insert(symbol, index);

        AssetId {
            symbol,
            registry: Arc::new(Mutex::new(())),
        }
    }

    /// Get asset info by symbol
    pub fn get_asset_info(&self, symbol: &str) -> Option<&AssetInfo> {
        self.assets.get(symbol)
    }

    /// Get all registered assets
    pub fn get_all_assets(&self) -> Vec<&str> {
        self.assets.keys().map(|s| s.as_str()).collect()
    }

    /// Get asset index
    pub fn get_index(&self, symbol: &str) -> Option<usize> {
        self.indices.get(symbol).copied()
    }

    /// Check if asset is registered
    pub fn contains(&self, symbol: &str) -> bool {
        self.assets.contains_key(symbol)
    }
}

/// Dynamic asset identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId {
    pub symbol: String,
    #[serde(skip)]
    registry: Arc<Mutex<()>>, // Ensures thread safety for asset operations
}

impl AssetId {
    /// Create a new asset ID
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            registry: Arc::new(Mutex::new(())),
        }
    }

    /// Get the asset symbol
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get asset index (for matrix operations)
    pub fn index(&self) -> usize {
        // This would need to be implemented with a global registry
        // For now, return a hash-based index
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.symbol.hash(&mut hasher);
        (hasher.finish() % 1000) as usize // Simple hash for demo
    }

    /// Returns all supported assets (for backward compatibility)
    /// This is a simplified version - in production, this would use the global registry
    pub fn all() -> &'static [AssetId] {
        static ASSETS: [AssetId; 6] = [
            AssetId::new("USD".to_string()),
            AssetId::new("EUR".to_string()),
            AssetId::new("JPY".to_string()),
            AssetId::new("GBP".to_string()),
            AssetId::new("CHF".to_string()),
            AssetId::new("AUD".to_string()),
        ];
        &ASSETS
    }

    /// Parse an asset from a string
    pub fn from_str(s: &str) -> Option<Self> {
        let symbol = s.to_uppercase();
        match symbol.as_str() {
            "USD" | "EUR" | "JPY" | "GBP" | "CHF" | "AUD" => Some(AssetId::new(symbol)),
            _ => None,
        }
    }

    /// Returns the asset as a string slice
    pub fn as_str(&self) -> &str {
        &self.symbol
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

// Backward compatibility constants
impl AssetId {
    pub const USD: AssetId = AssetId { symbol: "USD".to_string(), registry: Arc::new(Mutex::new(())) };
    pub const EUR: AssetId = AssetId { symbol: "EUR".to_string(), registry: Arc::new(Mutex::new(())) };
    pub const JPY: AssetId = AssetId { symbol: "JPY".to_string(), registry: Arc::new(Mutex::new(())) };
    pub const GBP: AssetId = AssetId { symbol: "GBP".to_string(), registry: Arc::new(Mutex::new(())) };
    pub const CHF: AssetId = AssetId { symbol: "CHF".to_string(), registry: Arc::new(Mutex::new(())) };
    pub const AUD: AssetId = AssetId { symbol: "AUD".to_string(), registry: Arc::new(Mutex::new(())) };
}



