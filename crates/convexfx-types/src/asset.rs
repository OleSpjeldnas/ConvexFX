use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Information about an asset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub decimals: u32,
    pub is_base_currency: bool,
}

/// Registry of asset information
#[derive(Debug, Clone)]
pub struct AssetRegistry {
    assets: BTreeMap<String, AssetInfo>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        let mut assets = BTreeMap::new();

        // Add default assets
        assets.insert("USD".to_string(), AssetInfo {
            name: "US Dollar".to_string(),
            decimals: 2,
            is_base_currency: true,
        });
        assets.insert("EUR".to_string(), AssetInfo {
            name: "Euro".to_string(),
            decimals: 2,
            is_base_currency: false,
        });
        assets.insert("JPY".to_string(), AssetInfo {
            name: "Japanese Yen".to_string(),
            decimals: 0,
            is_base_currency: false,
        });
        assets.insert("GBP".to_string(), AssetInfo {
            name: "British Pound".to_string(),
            decimals: 2,
            is_base_currency: false,
        });
        assets.insert("CHF".to_string(), AssetInfo {
            name: "Swiss Franc".to_string(),
            decimals: 2,
            is_base_currency: false,
        });
        assets.insert("AUD".to_string(), AssetInfo {
            name: "Australian Dollar".to_string(),
            decimals: 2,
            is_base_currency: false,
        });

        AssetRegistry { assets }
    }

    pub fn get_all_assets(&self) -> Vec<String> {
        self.assets.keys().cloned().collect()
    }

    pub fn get_asset_info(&self, symbol: &str) -> Option<&AssetInfo> {
        self.assets.get(symbol)
    }

    pub fn add_asset(&mut self, symbol: String, name: String, decimals: u32, is_base_currency: bool) -> Result<(), String> {
        if self.assets.contains_key(&symbol) {
            return Err(format!("Asset {} already exists", symbol));
        }

        self.assets.insert(symbol.clone(), AssetInfo {
            name,
            decimals,
            is_base_currency,
        });

        Ok(())
    }
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported asset identifiers in the FX pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AssetId {
    USD,
    EUR,
    JPY,
    GBP,
    CHF,
    AUD,
}

impl AssetId {
    /// Returns all supported assets
    pub fn all() -> &'static [AssetId] {
        &[
            AssetId::USD,
            AssetId::EUR,
            AssetId::JPY,
            AssetId::GBP,
            AssetId::CHF,
            AssetId::AUD,
        ]
    }

    /// Returns the asset as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetId::USD => "USD",
            AssetId::EUR => "EUR",
            AssetId::JPY => "JPY",
            AssetId::GBP => "GBP",
            AssetId::CHF => "CHF",
            AssetId::AUD => "AUD",
        }
    }

    /// Parse an asset from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USD" => Some(AssetId::USD),
            "EUR" => Some(AssetId::EUR),
            "JPY" => Some(AssetId::JPY),
            "GBP" => Some(AssetId::GBP),
            "CHF" => Some(AssetId::CHF),
            "AUD" => Some(AssetId::AUD),
            _ => None,
        }
    }

    /// Returns the index of this asset in the canonical ordering
    pub fn index(&self) -> usize {
        match self {
            AssetId::USD => 0,
            AssetId::EUR => 1,
            AssetId::JPY => 2,
            AssetId::GBP => 3,
            AssetId::CHF => 4,
            AssetId::AUD => 5,
        }
    }

    /// Returns the asset at the given index
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(AssetId::USD),
            1 => Some(AssetId::EUR),
            2 => Some(AssetId::JPY),
            3 => Some(AssetId::GBP),
            4 => Some(AssetId::CHF),
            5 => Some(AssetId::AUD),
            _ => None,
        }
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}



