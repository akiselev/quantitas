use crate::{Dimension, QuantityKindId, UnitId};
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DimensionError {
    #[error("dimension exponent denominator must be non-zero")]
    ZeroDenominator,
    #[error("dimension exponent is outside the canonical i32 representation")]
    ExponentOverflow,
    #[error("dimensions differ: `{left}` and `{right}`")]
    Mismatch { left: Dimension, right: Dimension },
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ScaleError {
    #[error("unit scale denominator must be non-zero")]
    ZeroDenominator,
    #[error("unit scale numerator must be non-zero")]
    ZeroScale,
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum QuantityError {
    #[error("unknown unit `{0}`")]
    UnknownUnit(UnitId),
    #[error("unit `{unit}` does not admit quantity kind `{kind}`")]
    KindMismatch { unit: UnitId, kind: QuantityKindId },
    #[error("offset unit `{0}` has no interval form")]
    MissingIntervalForm(UnitId),
    #[error("quantity value must be finite")]
    NonFinite,
    #[error(transparent)]
    Dimension(#[from] DimensionError),
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseQuantityError {
    #[error("expected a numeric magnitude at byte {offset}")]
    MissingMagnitude { offset: usize },
    #[error("invalid numeric magnitude `{text}` at byte {offset}")]
    InvalidMagnitude { text: String, offset: usize },
    #[error("unknown unit symbol `{symbol}` at byte {offset}")]
    UnknownSymbol { symbol: String, offset: usize },
    #[error("unexpected trailing input at byte {offset}")]
    TrailingInput { offset: usize },
}
