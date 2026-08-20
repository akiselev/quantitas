use crate::{Dimension, QuantityKindId, ScaleError};
use serde::{Deserialize, Serialize};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        let sign = if denominator < 0 { -1 } else { 1 };
        Ok(Self {
            numerator: numerator * sign,
            denominator: denominator.abs(),
            power10,
        })
    }

    pub fn as_f64(self) -> f64 {
        (self.numerator as f64 / self.denominator as f64) * 10_f64.powi(self.power10)
    }
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
