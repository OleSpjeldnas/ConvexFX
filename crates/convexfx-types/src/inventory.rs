use crate::{Amount, AssetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Pool inventory across all assets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub units: BTreeMap<AssetId, Amount>,
}

impl Inventory {
    /// Create a new empty inventory
    pub fn new() -> Self {
        Inventory {
            units: BTreeMap::new(),
        }
    }

    /// Create an inventory with initial balances
    pub fn from_map(units: BTreeMap<AssetId, Amount>) -> Self {
        Inventory { units }
    }

    /// Get balance for an asset (returns 0 if not present)
    pub fn get(&self, asset: AssetId) -> Amount {
        self.units.get(&asset).copied().unwrap_or(Amount::ZERO)
    }

    /// Set balance for an asset
    pub fn set(&mut self, asset: AssetId, amount: Amount) {
        if amount.is_zero() {
            self.units.remove(&asset);
        } else {
            self.units.insert(asset, amount);
        }
    }

    /// Add to balance for an asset
    pub fn add(&mut self, asset: AssetId, delta: Amount) {
        let current = self.get(asset);
        let new_amount = current + delta;
        self.set(asset, new_amount);
    }

    /// Subtract from balance for an asset
    pub fn sub(&mut self, asset: AssetId, delta: Amount) {
        self.add(asset, -delta);
    }

    /// Convert to f64 map (for solver interface)
    pub fn to_f64_map(&self) -> BTreeMap<AssetId, f64> {
        self.units
            .iter()
            .map(|(asset, amount)| (*asset, amount.to_f64()))
            .collect()
    }

    /// Create from f64 map (for solver interface)
    pub fn from_f64_map(map: &BTreeMap<AssetId, f64>) -> crate::Result<Self> {
        let units = map
            .iter()
            .map(|(asset, value)| Ok((*asset, Amount::from_f64(*value)?)))
            .collect::<crate::Result<BTreeMap<_, _>>>()?;
        Ok(Inventory { units })
    }

    /// Get all assets with non-zero balances
    pub fn assets(&self) -> Vec<AssetId> {
        self.units.keys().copied().collect()
    }

    /// Check if inventory has sufficient balance
    pub fn has_sufficient(&self, asset: AssetId, required: Amount) -> bool {
        self.get(asset) >= required
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_operations() {
        let mut inv = Inventory::new();

        inv.add(AssetId::USD, Amount::from_units(100));
        assert_eq!(inv.get(AssetId::USD), Amount::from_units(100));

        inv.sub(AssetId::USD, Amount::from_units(30));
        assert_eq!(inv.get(AssetId::USD), Amount::from_units(70));

        assert!(inv.has_sufficient(AssetId::USD, Amount::from_units(50)));
        assert!(!inv.has_sufficient(AssetId::USD, Amount::from_units(100)));
    }

    #[test]
    fn test_inventory_f64_conversion() {
        let mut inv = Inventory::new();
        inv.set(AssetId::EUR, Amount::from_f64(123.456).unwrap());

        let f64_map = inv.to_f64_map();
        assert!((f64_map[&AssetId::EUR] - 123.456).abs() < 1e-6);

        let inv2 = Inventory::from_f64_map(&f64_map).unwrap();
        assert_eq!(inv, inv2);
    }
}


