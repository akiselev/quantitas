use crate::{Dimension, QuantityError, UnitId};
use serde::{Deserialize, Deserializer, Serialize};
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
#[derive(Clone, Debug, PartialEq, Serialize)]
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
        if kind.as_str().trim().is_empty() {
            return Err(QuantityError::EmptyKind);
        }
        let value_si = if value_si == 0.0 { 0.0 } else { value_si };
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
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            value_si: f64,
            dimension: Dimension,
            kind: QuantityKindId,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.value_si, wire.dimension, wire.kind).map_err(serde::de::Error::custom)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialization_rejects_non_finite_canonical_values() {
        let encoded = r#"{
            "value_si": 1e999,
            "dimension": [
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1},
                {"numerator":0,"denominator":1}
            ],
            "kind": "test:Scalar"
        }"#;
        assert!(serde_json::from_str::<Quantity>(encoded).is_err());
        assert_eq!(
            Quantity::new(
                f64::NAN,
                Dimension::DIMENSIONLESS,
                QuantityKindId::new("test:Scalar")
            ),
            Err(QuantityError::NonFinite)
        );
        assert!(
            Quantity::new(
                -0.0,
                Dimension::DIMENSIONLESS,
                QuantityKindId::new("test:Scalar")
            )
            .unwrap()
            .value_si()
            .is_sign_positive()
        );
        assert!(
            serde_json::from_str::<Quantity>(
                r#"{
                "value_si": 1.0,
                "dimension": [
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1},
                    {"numerator":0,"denominator":1}
                ],
                "kind": " "
            }"#
            )
            .is_err()
        );
    }
}
