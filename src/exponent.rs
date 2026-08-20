use crate::DimensionError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A reduced rational exponent used in dimensional algebra.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RationalExponent {
    numerator: i32,
    denominator: u32,
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
        reduce(i64::from(numerator), i64::from(denominator))
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
        let left = i64::from(self.numerator)
            .checked_mul(i64::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let right = i64::from(rhs.numerator)
            .checked_mul(i64::from(self.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        let denominator = i64::from(self.denominator)
            .checked_mul(i64::from(rhs.denominator))
            .ok_or(DimensionError::ExponentOverflow)?;
        reduce(
            left.checked_add(right)
                .ok_or(DimensionError::ExponentOverflow)?,
            denominator,
        )
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, DimensionError> {
        self.checked_add(rhs.checked_scale(-1)?)
    }

    pub fn checked_scale(self, factor: i32) -> Result<Self, DimensionError> {
        reduce(
            i64::from(self.numerator)
                .checked_mul(i64::from(factor))
                .ok_or(DimensionError::ExponentOverflow)?,
            i64::from(self.denominator),
        )
    }

    pub fn checked_divide(self, divisor: i32) -> Result<Self, DimensionError> {
        if divisor == 0 {
            return Err(DimensionError::ZeroDenominator);
        }
        reduce(
            i64::from(self.numerator),
            i64::from(self.denominator)
                .checked_mul(i64::from(divisor))
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

fn reduce(mut numerator: i64, mut denominator: i64) -> Result<RationalExponent, DimensionError> {
    if denominator == 0 {
        return Err(DimensionError::ZeroDenominator);
    }
    if denominator < 0 {
        numerator = numerator
            .checked_neg()
            .ok_or(DimensionError::ExponentOverflow)?;
        denominator = denominator
            .checked_neg()
            .ok_or(DimensionError::ExponentOverflow)?;
    }
    if numerator == 0 {
        return Ok(RationalExponent::ZERO);
    }
    let divisor = gcd(numerator.unsigned_abs(), denominator as u64);
    numerator /= divisor as i64;
    denominator /= divisor as i64;
    Ok(RationalExponent {
        numerator: i32::try_from(numerator).map_err(|_| DimensionError::ExponentOverflow)?,
        denominator: u32::try_from(denominator).map_err(|_| DimensionError::ExponentOverflow)?,
    })
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
