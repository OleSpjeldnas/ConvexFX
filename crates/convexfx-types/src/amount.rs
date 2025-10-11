use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub, Neg};
use crate::error::{ConvexFxError, Result};

/// Fixed-point amount with 9 decimal places
/// Internally stored as i128 to prevent overflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Amount(i128);

const SCALE: i128 = 1_000_000_000; // 10^9

impl Amount {
    /// Zero amount
    pub const ZERO: Amount = Amount(0);

    /// Create from raw i128 (scaled value)
    pub const fn from_raw(raw: i128) -> Self {
        Amount(raw)
    }

    /// Get the raw scaled value
    pub const fn raw(&self) -> i128 {
        self.0
    }

    /// Create from integer units
    pub const fn from_units(units: i64) -> Self {
        Amount((units as i128) * SCALE)
    }

    /// Create from f64 (for solver interface)
    /// Rounds toward zero
    pub fn from_f64(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(ConvexFxError::InvalidAmount(format!(
                "non-finite value: {}",
                value
            )));
        }
        let scaled = value * (SCALE as f64);
        if scaled.abs() > (i128::MAX as f64) {
            return Err(ConvexFxError::InvalidAmount(format!(
                "overflow: {}",
                value
            )));
        }
        Ok(Amount(scaled as i128))
    }

    /// Convert to f64 (for solver interface)
    pub fn to_f64(&self) -> f64 {
        (self.0 as f64) / (SCALE as f64)
    }

    /// Create from string representation (e.g., "123.456789")
    pub fn from_string(s: &str) -> Result<Self> {
        let value: f64 = s.parse().map_err(|_| {
            ConvexFxError::InvalidAmount(format!("cannot parse: {}", s))
        })?;
        Self::from_f64(value)
    }

    /// Check if amount is positive
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }

    /// Check if amount is negative
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Check if amount is zero
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Absolute value
    pub const fn abs(&self) -> Self {
        Amount(self.0.abs())
    }

    /// Checked addition
    pub fn checked_add(&self, other: Self) -> Result<Self> {
        self.0
            .checked_add(other.0)
            .map(Amount)
            .ok_or_else(|| ConvexFxError::InvalidAmount("overflow in addition".to_string()))
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Self) -> Result<Self> {
        self.0
            .checked_sub(other.0)
            .map(Amount)
            .ok_or_else(|| ConvexFxError::InvalidAmount("overflow in subtraction".to_string()))
    }

    /// Checked multiplication by integer
    pub fn checked_mul_int(&self, factor: i64) -> Result<Self> {
        self.0
            .checked_mul(factor as i128)
            .map(Amount)
            .ok_or_else(|| ConvexFxError::InvalidAmount("overflow in multiplication".to_string()))
    }

    /// Multiply by f64 (for solver calculations)
    pub fn mul_f64(&self, factor: f64) -> Result<Self> {
        let result = (self.0 as f64) * factor;
        if !result.is_finite() || result.abs() > (i128::MAX as f64) {
            return Err(ConvexFxError::InvalidAmount(format!(
                "overflow in f64 multiplication: {} * {}",
                self.to_f64(),
                factor
            )));
        }
        Ok(Amount(result as i128))
    }

    /// Round toward pool (conservative rounding)
    /// If negative (outflow from pool), round away from zero (larger outflow)
    /// If positive (inflow to pool), round toward zero (smaller inflow)
    pub fn round_toward_pool(&self) -> Self {
        // Already at our precision, no-op for now
        // In a real implementation with higher internal precision, we'd round here
        *self
    }
}

impl Add for Amount {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Amount(self.0 + other.0)
    }
}

impl Sub for Amount {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Amount(self.0 - other.0)
    }
}

impl Neg for Amount {
    type Output = Self;
    fn neg(self) -> Self {
        Amount(-self.0)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.9}", self.to_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_conversions() {
        let a = Amount::from_units(100);
        assert_eq!(a.to_f64(), 100.0);

        let b = Amount::from_f64(123.456789).unwrap();
        assert!((b.to_f64() - 123.456789).abs() < 1e-8);

        assert_eq!(Amount::ZERO.to_f64(), 0.0);
    }

    #[test]
    fn test_amount_arithmetic() {
        let a = Amount::from_units(10);
        let b = Amount::from_units(5);

        assert_eq!((a + b).to_f64(), 15.0);
        assert_eq!((a - b).to_f64(), 5.0);
        assert_eq!((-a).to_f64(), -10.0);
    }

    #[test]
    fn test_amount_checks() {
        assert!(Amount::from_units(10).is_positive());
        assert!(Amount::from_units(-10).is_negative());
        assert!(Amount::ZERO.is_zero());
    }
}


