//! Exact dimensional quantities, quantity kinds, units, and registry provenance.

mod dimension;
mod error;
mod exponent;
mod kind_registry;
mod quantity;
mod registry;
mod unit;
mod unit_expression;

pub use dimension::{Dimension, SiBasis};
pub use error::{
    DimensionError, KindRegistryError, ParseQuantityError, QuantityError, RegistryError,
    ScaleError, UnitExpressionError,
};
pub use exponent::RationalExponent;
pub use kind_registry::{KindDef, QuantityKindRegistry};
pub use quantity::{DisplayUnit, Quantity, QuantityKindId, QuantityLiteral};
pub use registry::{RegistrySnapshot, UnitRegistry};
pub use unit::{ExactScale, UnitDef, UnitId, UnitProvenance};
pub use unit_expression::UnitExpression;
