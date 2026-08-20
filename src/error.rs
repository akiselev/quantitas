use crate::{QuantityKindId, UnitId};
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DimensionError {
    #[error("dimension exponent denominator must be non-zero")]
    ZeroDenominator,
    #[error("dimension exponent is outside the canonical i32 representation")]
    ExponentOverflow,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ScaleError {
    #[error("unit scale denominator must be non-zero")]
    ZeroDenominator,
    #[error("unit scale numerator must be non-zero")]
    ZeroScale,
    #[error("unit scale magnitude cannot be represented canonically")]
    MagnitudeOverflow,
    #[error("unit scale base-10 exponent overflowed")]
    PowerOverflow,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("unit registry is frozen")]
    Frozen,
    #[error("unit id and symbol must be non-empty")]
    EmptyIdentity,
    #[error("unit id `{0}` is already registered")]
    DuplicateUnit(UnitId),
    #[error("unit symbol `{0}` is already registered")]
    DuplicateSymbol(String),
    #[error("unit `{unit}` references missing interval unit `{interval}`")]
    MissingIntervalUnit { unit: UnitId, interval: UnitId },
    #[error("unit `{unit}` and interval unit `{interval}` have different dimensions")]
    IntervalDimensionMismatch { unit: UnitId, interval: UnitId },
    #[error("offset unit `{0}` must name an interval unit")]
    OffsetWithoutInterval(UnitId),
    #[error("linear unit `{0}` must not name an interval form")]
    IntervalFormOnLinearUnit(UnitId),
    #[error("unit `{unit}` and interval unit `{interval}` have different scales")]
    IntervalScaleMismatch { unit: UnitId, interval: UnitId },
    #[error("interval unit `{interval}` referenced by `{unit}` must not have an offset")]
    AffineIntervalUnit { unit: UnitId, interval: UnitId },
    #[error("offset unit `{0}` must explicitly admit at least one point quantity kind")]
    AffineUnitRequiresKinds(UnitId),
    #[error("interval unit `{interval}` referenced by `{unit}` must explicitly admit a kind")]
    IntervalUnitRequiresKinds { unit: UnitId, interval: UnitId },
    #[error("unit `{0}` has incomplete provenance")]
    InvalidProvenance(UnitId),
    #[error("unit `{unit}` contains an empty quantity-kind identity")]
    EmptyKind { unit: UnitId },
    #[error("registry snapshot fields must be non-empty")]
    InvalidSnapshot,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum QuantityError {
    #[error("unknown unit `{0}`")]
    UnknownUnit(UnitId),
    #[error("unit `{unit}` does not admit quantity kind `{kind}`")]
    KindMismatch { unit: UnitId, kind: QuantityKindId },
    #[error("quantity value must be finite")]
    NonFinite,
    #[error("quantity-kind identity must be non-empty")]
    EmptyKind,
    #[error("unit registry must be frozen before canonicalization")]
    UnfrozenRegistry,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseQuantityError {
    #[error("expected a numeric magnitude at byte {offset}")]
    MissingMagnitude { offset: usize },
    #[error("invalid numeric magnitude `{text}` at byte {offset}")]
    InvalidMagnitude { text: String, offset: usize },
    #[error("unknown unit symbol `{symbol}` at byte {offset}")]
    UnknownSymbol { symbol: String, offset: usize },
}
