use serde::{Deserialize, Serialize};
use std::fmt;

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



