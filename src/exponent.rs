use crate::DimensionError;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// A reduced rational exponent used in dimensional algebra.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct RationalExponent {
    numerator: i32,
    denominator: u32,
}

impl<'de> Deserialize<'de> for RationalExponent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            numerator: i32,
            denominator: u32,
        }

        let wire = Wire::deserialize(deserializer)?;
        reduce(i128::from(wire.numerator), i128::from(wire.denominator))
            .map_err(serde::de::Error::custom)
    }
}

impl RationalExponent {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    pub const fn integer(value: i32) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub fn new(numerator: i32, denominator: i32) -> Result<Self, DimensionError> {
        reduce(i128::from(numerator), i128::from(denominator))
    }

    pub const fn numerator(self) -> i32 {
        self.numerator
    }

    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, DimensionError> {
        let left = i128::from(self.numerator)
            .checked_mul(i128::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let right = i128::from(rhs.numerator)
            .checked_mul(i128::from(self.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let denominator = i128::from(self.denominator)
            .checked_mul(i128::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        reduce(
            left.checked_add(right)
                .ok_or(DimensionError::ExponentOverflow)?,
            denominator,
        )
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, DimensionError> {
        let left = i128::from(self.numerator)
            .checked_mul(i128::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let right = i128::from(rhs.numerator)
            .checked_mul(i128::from(self.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let denominator = i128::from(self.denominator)
            .checked_mul(i128::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        reduce(
            left.checked_sub(right)
                .ok_or(DimensionError::ExponentOverflow)?,
            denominator,
        )
    }

    pub fn checked_scale(self, factor: i32) -> Result<Self, DimensionError> {
        reduce(
            i128::from(self.numerator)
                .checked_mul(i128::from(factor))
                .ok_or(DimensionError::ExponentOverflow)?,
            i128::from(self.denominator),
        )
    }

    pub fn checked_divide(self, divisor: i32) -> Result<Self, DimensionError> {
        if divisor == 0 {
            return Err(DimensionError::ZeroDenominator);
        }
        reduce(
            i128::from(self.numerator),
            i128::from(self.denominator)
                .checked_mul(i128::from(divisor))
                .ok_or(DimensionError::ExponentOverflow)?,
        )
    }
}

impl fmt::Debug for RationalExponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for RationalExponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

fn reduce(numerator: i128, denominator: i128) -> Result<RationalExponent, DimensionError> {
    if denominator == 0 {
        return Err(DimensionError::ZeroDenominator);
    }
    if numerator == 0 {
        return Ok(RationalExponent::ZERO);
    }
    let negative = (numerator < 0) != (denominator < 0);
    let mut numerator = numerator.unsigned_abs();
    let mut denominator = denominator.unsigned_abs();
    let divisor = gcd(numerator, denominator);
    numerator /= divisor;
    denominator /= divisor;
    Ok(RationalExponent {
        numerator: signed_i32(numerator, negative)?,
        denominator: u32::try_from(denominator).map_err(|_| DimensionError::ExponentOverflow)?,
    })
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn signed_i32(magnitude: u128, negative: bool) -> Result<i32, DimensionError> {
    if !negative {
        return i32::try_from(magnitude).map_err(|_| DimensionError::ExponentOverflow);
    }
    if magnitude == i32::MAX as u128 + 1 {
        return Ok(i32::MIN);
    }
    i32::try_from(magnitude)
        .map(|value| -value)
        .map_err(|_| DimensionError::ExponentOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialization_reduces_and_rejects_invalid_exponents() {
        let reduced: RationalExponent =
            serde_json::from_str(r#"{"numerator":2,"denominator":4}"#).unwrap();
        assert_eq!(reduced, RationalExponent::new(1, 2).unwrap());
        assert!(
            serde_json::from_str::<RationalExponent>(r#"{"numerator":1,"denominator":0}"#).is_err()
        );
        let zero: RationalExponent =
            serde_json::from_str(r#"{"numerator":0,"denominator":4294967295}"#).unwrap();
        assert_eq!(zero, RationalExponent::ZERO);
    }

    #[test]
    fn checked_subtraction_does_not_overflow_an_unrepresentable_intermediate() {
        assert_eq!(
            RationalExponent::integer(-1)
                .checked_sub(RationalExponent::integer(i32::MIN))
                .unwrap(),
            RationalExponent::integer(i32::MAX)
        );
        assert_eq!(
            RationalExponent::new(i32::MIN, i32::MIN).unwrap(),
            RationalExponent::ONE
        );
    }
}
