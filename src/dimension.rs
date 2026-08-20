use crate::{DimensionError, RationalExponent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SiBasis {
    Mass,
    Length,
    Time,
    ElectricCurrent,
    Temperature,
    Amount,
    LuminousIntensity,
}

impl SiBasis {
    pub const ALL: [Self; 7] = [
        Self::Mass,
        Self::Length,
        Self::Time,
        Self::ElectricCurrent,
        Self::Temperature,
        Self::Amount,
        Self::LuminousIntensity,
    ];

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Mass => "kg",
            Self::Length => "m",
            Self::Time => "s",
            Self::ElectricCurrent => "A",
            Self::Temperature => "K",
            Self::Amount => "mol",
            Self::LuminousIntensity => "cd",
        }
    }
}

/// A dimension over the seven SI base quantities with exact rational exponents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Dimension(pub [RationalExponent; 7]);

const fn dimension(values: [i32; 7]) -> Dimension {
    Dimension([
        RationalExponent::integer(values[0]),
        RationalExponent::integer(values[1]),
        RationalExponent::integer(values[2]),
        RationalExponent::integer(values[3]),
        RationalExponent::integer(values[4]),
        RationalExponent::integer(values[5]),
        RationalExponent::integer(values[6]),
    ])
}

impl Dimension {
    pub const DIMENSIONLESS: Self = dimension([0; 7]);
    pub const MASS: Self = dimension([1, 0, 0, 0, 0, 0, 0]);
    pub const LENGTH: Self = dimension([0, 1, 0, 0, 0, 0, 0]);
    pub const TIME: Self = dimension([0, 0, 1, 0, 0, 0, 0]);
    pub const CURRENT: Self = dimension([0, 0, 0, 1, 0, 0, 0]);
    pub const TEMPERATURE: Self = dimension([0, 0, 0, 0, 1, 0, 0]);
    pub const AMOUNT: Self = dimension([0, 0, 0, 0, 0, 1, 0]);
    pub const LUMINOUS_INTENSITY: Self = dimension([0, 0, 0, 0, 0, 0, 1]);

    pub fn checked_product(self, rhs: Self) -> Result<Self, DimensionError> {
        let mut result = self.0;
        for (left, right) in result.iter_mut().zip(rhs.0) {
            *left = left.checked_add(right)?;
        }
        Ok(Self(result))
    }

    pub fn checked_quotient(self, rhs: Self) -> Result<Self, DimensionError> {
        let mut result = self.0;
        for (left, right) in result.iter_mut().zip(rhs.0) {
            *left = left.checked_sub(right)?;
        }
        Ok(Self(result))
    }

    pub fn checked_powi(self, power: i32) -> Result<Self, DimensionError> {
        let mut result = self.0;
        for exponent in &mut result {
            *exponent = exponent.checked_scale(power)?;
        }
        Ok(Self(result))
    }

    pub fn checked_root(self, degree: i32) -> Result<Self, DimensionError> {
        let mut result = self.0;
        for exponent in &mut result {
            *exponent = exponent.checked_divide(degree)?;
        }
        Ok(Self(result))
    }

    pub fn product(self, rhs: Self) -> Self {
        self.checked_product(rhs)
            .expect("dimension exponent overflow")
    }

    pub fn quotient(self, rhs: Self) -> Self {
        self.checked_quotient(rhs)
            .expect("dimension exponent overflow")
    }

    pub fn powi(self, power: i32) -> Self {
        self.checked_powi(power)
            .expect("dimension exponent overflow")
    }

    pub fn root(self, degree: i32) -> Result<Self, DimensionError> {
        self.checked_root(degree)
    }

    pub fn exponent(self, basis: SiBasis) -> RationalExponent {
        self.0[basis as usize]
    }

    pub fn is_dimensionless(self) -> bool {
        self.0.iter().all(|exponent| exponent.is_zero())
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self::DIMENSIONLESS
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote = false;
        for (basis, exponent) in SiBasis::ALL.into_iter().zip(self.0) {
            if exponent.is_zero() {
                continue;
            }
            if wrote {
                f.write_str(" ")?;
            }
            wrote = true;
            write!(f, "{}", basis.symbol())?;
            if exponent != RationalExponent::ONE {
                write!(f, "^{exponent}")?;
            }
        }
        if wrote { Ok(()) } else { f.write_str("1") }
    }
}

/// A consumer-defined sparse dimension vector, used when the basis is not SI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionVector<B: Ord>(BTreeMap<B, RationalExponent>);

impl<B: Ord> DimensionVector<B> {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn from_terms(terms: impl IntoIterator<Item = (B, RationalExponent)>) -> Self {
        Self(
            terms
                .into_iter()
                .filter(|(_, exponent)| !exponent.is_zero())
                .collect(),
        )
    }

    pub fn terms(&self) -> impl Iterator<Item = (&B, RationalExponent)> {
        self.0.iter().map(|(basis, exponent)| (basis, *exponent))
    }
}

impl<B: Clone + Ord> DimensionVector<B> {
    pub fn checked_product(&self, rhs: &Self) -> Result<Self, DimensionError> {
        let mut result = self.0.clone();
        for (basis, exponent) in &rhs.0 {
            let combined = result
                .get(basis)
                .copied()
                .unwrap_or(RationalExponent::ZERO)
                .checked_add(*exponent)?;
            if combined.is_zero() {
                result.remove(basis);
            } else {
                result.insert(basis.clone(), combined);
            }
        }
        Ok(Self(result))
    }
}

impl<B: Ord> Default for DimensionVector<B> {
    fn default() -> Self {
        Self::new()
    }
}
