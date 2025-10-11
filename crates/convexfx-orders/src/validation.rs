use convexfx_types::{ConvexFxError, PairOrder, Result};

/// Validate a pair order for basic consistency
pub fn validate_order(order: &PairOrder) -> Result<()> {
    // Check budget is positive
    if !order.budget.is_positive() {
        return Err(ConvexFxError::InvalidOrder(
            "budget must be positive".to_string(),
        ));
    }

    // Check pay and receive are different
    if order.pay == order.receive {
        return Err(ConvexFxError::InvalidOrder(
            "pay and receive assets must be different".to_string(),
        ));
    }

    // Check limit ratio if present
    if let Some(limit) = order.limit_ratio {
        if !limit.is_finite() || limit <= 0.0 {
            return Err(ConvexFxError::InvalidOrder(
                "limit ratio must be positive and finite".to_string(),
            ));
        }
    }

    // Check min fill fraction if present
    if let Some(min_fill) = order.min_fill_fraction {
        if !(0.0..=1.0).contains(&min_fill) {
            return Err(ConvexFxError::InvalidOrder(
                "min fill fraction must be in [0, 1]".to_string(),
            ));
        }
    }

    // Check order ID is not empty
    if order.id.is_empty() {
        return Err(ConvexFxError::InvalidOrder(
            "order ID cannot be empty".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_types::{AccountId, Amount, AssetId};

    #[test]
    fn test_valid_order() {
        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: Some(1.2),
            min_fill_fraction: Some(0.5),
            metadata: serde_json::json!({}),
        };

        assert!(validate_order(&order).is_ok());
    }

    #[test]
    fn test_invalid_budget() {
        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::ZERO,
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        assert!(validate_order(&order).is_err());
    }

    #[test]
    fn test_same_assets() {
        let order = PairOrder {
            id: "order1".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::EUR,
            receive: AssetId::EUR,
            budget: Amount::from_units(100),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        assert!(validate_order(&order).is_err());
    }
}


