//! Exact dimensional quantities, quantity kinds, units, and registry provenance.

mod dimension;
mod error;
mod exponent;
mod quantity;
mod registry;
mod unit;

pub use dimension::{Dimension, DimensionVector, SiBasis};
pub use error::{DimensionError, ParseQuantityError, QuantityError, ScaleError};
pub use exponent::RationalExponent;
pub use quantity::{DisplayUnit, Quantity, QuantityKindId, QuantityLiteral};
pub use registry::{RegistrySnapshot, UnitRegistry};
pub use unit::{ExactScale, UnitDef, UnitId, UnitProvenance};
