use crate::{Dimension, DimensionError, QuantityError, UnitId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable identity for a quantity kind. Equal dimensions do not imply equal kinds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuantityKindId(String);

impl QuantityKindId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn thermodynamic_temperature() -> Self {
        Self::new("si:ThermodynamicTemperature")
    }

    pub fn temperature_difference() -> Self {
        Self::new("si:TemperatureDifference")
    }

    pub fn energy() -> Self {
        Self::new("si:Energy")
    }

    pub fn pressure() -> Self {
        Self::new("si:Pressure")
    }
}

impl fmt::Display for QuantityKindId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A canonical SI magnitude with dimensional and quantity-kind meaning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    value_si: f64,
    dimension: Dimension,
    kind: QuantityKindId,
}

impl Quantity {
    pub fn new(
        value_si: f64,
        dimension: Dimension,
        kind: QuantityKindId,
    ) -> Result<Self, QuantityError> {
        if !value_si.is_finite() {
            return Err(QuantityError::NonFinite);
        }
        Ok(Self {
            value_si,
            dimension,
            kind,
        })
    }

    pub fn value_si(&self) -> f64 {
        self.value_si
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension
    }

    pub fn kind(&self) -> &QuantityKindId {
        &self.kind
    }

    pub fn try_add(&self, rhs: &Self) -> Result<Self, QuantityError> {
        self.require_same_meaning(rhs)?;
        Self::new(
            self.value_si + rhs.value_si,
            self.dimension,
            self.kind.clone(),
        )
    }

    pub fn try_sub(&self, rhs: &Self) -> Result<Self, QuantityError> {
        self.require_same_meaning(rhs)?;
        Self::new(
            self.value_si - rhs.value_si,
            self.dimension,
            self.kind.clone(),
        )
    }

    pub fn product(&self, rhs: &Self, result_kind: QuantityKindId) -> Result<Self, QuantityError> {
        Self::new(
            self.value_si * rhs.value_si,
            self.dimension.checked_product(rhs.dimension)?,
            result_kind,
        )
    }

    pub fn quotient(&self, rhs: &Self, result_kind: QuantityKindId) -> Result<Self, QuantityError> {
        Self::new(
            self.value_si / rhs.value_si,
            self.dimension.checked_quotient(rhs.dimension)?,
            result_kind,
        )
    }

    fn require_same_meaning(&self, rhs: &Self) -> Result<(), QuantityError> {
        if self.dimension != rhs.dimension {
            return Err(DimensionError::Mismatch {
                left: self.dimension,
                right: rhs.dimension,
            }
            .into());
        }
        if self.kind != rhs.kind {
            return Err(QuantityError::KindMismatch {
                unit: UnitId::new("canonical-si"),
                kind: rhs.kind.clone(),
            });
        }
        Ok(())
    }
}

/// Authored magnitude and unit prior to canonicalization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityLiteral {
    pub value: f64,
    pub unit: UnitId,
    pub kind: QuantityKindId,
}

/// Authored unit retained independently from canonical quantity identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayUnit {
    pub unit: UnitId,
    pub symbol: String,
}
