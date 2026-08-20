use crate::{Dimension, QuantityKindId, ScaleError};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnitId(String);

impl UnitId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Exact scale represented as `(numerator / denominator) * 10^power10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ExactScale {
    numerator: i128,
    denominator: i128,
    power10: i32,
}

impl ExactScale {
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
        power10: 0,
    };

    pub fn new(numerator: i128, denominator: i128, power10: i32) -> Result<Self, ScaleError> {
        if denominator == 0 {
            return Err(ScaleError::ZeroDenominator);
        }
        if numerator == 0 {
            return Err(ScaleError::ZeroScale);
        }
        let negative = (numerator < 0) != (denominator < 0);
        let mut numerator = numerator.unsigned_abs();
        let mut denominator = denominator.unsigned_abs();
        let divisor = gcd(numerator, denominator);
        numerator /= divisor;
        denominator /= divisor;
        let mut power10 = power10;

        // Move all factors shared with the decimal radix out of the denominator. Handling a
        // factor of ten first avoids a needless intermediate multiplication and makes values
        // such as i128::MIN / i128::MIN canonicalizable without signed overflow.
        while denominator % 10 == 0 {
            denominator /= 10;
            power10 = power10.checked_sub(1).ok_or(ScaleError::PowerOverflow)?;
        }
        while denominator % 2 == 0 {
            denominator /= 2;
            numerator = numerator
                .checked_mul(5)
                .ok_or(ScaleError::MagnitudeOverflow)?;
            power10 = power10.checked_sub(1).ok_or(ScaleError::PowerOverflow)?;
        }
        while denominator % 5 == 0 {
            denominator /= 5;
            numerator = numerator
                .checked_mul(2)
                .ok_or(ScaleError::MagnitudeOverflow)?;
            power10 = power10.checked_sub(1).ok_or(ScaleError::PowerOverflow)?;
        }
        while numerator % 10 == 0 {
            numerator /= 10;
            power10 = power10.checked_add(1).ok_or(ScaleError::PowerOverflow)?;
        }
        let denominator = i128::try_from(denominator).map_err(|_| ScaleError::MagnitudeOverflow)?;
        let numerator = signed_magnitude(numerator, negative)?;
        Ok(Self {
            numerator,
            denominator,
            power10,
        })
    }

    pub fn as_f64(self) -> f64 {
        (self.numerator as f64 / self.denominator as f64) * 10_f64.powi(self.power10)
    }

    pub const fn numerator(self) -> i128 {
        self.numerator
    }

    pub const fn denominator(self) -> i128 {
        self.denominator
    }

    pub const fn power10(self) -> i32 {
        self.power10
    }
}

impl<'de> Deserialize<'de> for ExactScale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            numerator: i128,
            denominator: i128,
            power10: i32,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.numerator, wire.denominator, wire.power10).map_err(serde::de::Error::custom)
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn signed_magnitude(magnitude: u128, negative: bool) -> Result<i128, ScaleError> {
    if !negative {
        return i128::try_from(magnitude).map_err(|_| ScaleError::MagnitudeOverflow);
    }
    if magnitude == i128::MAX as u128 + 1 {
        return Ok(i128::MIN);
    }
    i128::try_from(magnitude)
        .map(|value| -value)
        .map_err(|_| ScaleError::MagnitudeOverflow)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitProvenance {
    pub authority: String,
    pub version: String,
    pub persistent_id: Option<String>,
}

/// One named unit. Offset units distinguish absolute points from intervals explicitly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitDef {
    pub id: UnitId,
    pub symbol: String,
    pub dimension: Dimension,
    pub scale_to_si: ExactScale,
    pub offset_to_si: Option<ExactScale>,
    pub admitted_kinds: Vec<QuantityKindId>,
    pub interval_form: Option<UnitId>,
    pub provenance: UnitProvenance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_scale_has_one_canonical_representation() {
        assert_eq!(
            ExactScale::new(2, 4, 0).unwrap(),
            ExactScale::new(5, 1, -1).unwrap()
        );
        assert_eq!(
            ExactScale::new(-2, -4, 0).unwrap(),
            ExactScale::new(5, 1, -1).unwrap()
        );
        assert_eq!(
            ExactScale::new(i128::MIN, i128::MIN, 0).unwrap(),
            ExactScale::ONE
        );
        assert!(ExactScale::new(1, i128::MIN, 0).is_err());
    }

    #[test]
    fn deserialization_canonicalizes_and_rejects_unrepresentable_scales() {
        let scale: ExactScale =
            serde_json::from_str(r#"{"numerator":20,"denominator":40,"power10":1}"#).unwrap();
        assert_eq!(scale, ExactScale::new(5, 1, 0).unwrap());
        assert!(
            serde_json::from_str::<ExactScale>(r#"{"numerator":1,"denominator":0,"power10":0}"#)
                .is_err()
        );
        assert_eq!(
            ExactScale::new(1, 2, i32::MIN),
            Err(ScaleError::PowerOverflow)
        );
        assert_eq!(
            ExactScale::new(10, 1, i32::MAX),
            Err(ScaleError::PowerOverflow)
        );
    }
}
