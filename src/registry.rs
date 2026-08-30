use crate::{
    Dimension, DisplayUnit, ExactScale, ParseQuantityError, Quantity, QuantityError,
    QuantityKindId, QuantityLiteral, RegistryError, UnitDef, UnitId, UnitProvenance,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Builds a coherent derived dimension from `(base, exponent)` factors, e.g.
/// `[(MASS, 1), (LENGTH, 1), (TIME, -2)]` for force. Bootstrap-only: the
/// factors are fixed, small integers that cannot overflow the checked
/// dimensional algebra.
fn derived_dimension(factors: &[(Dimension, i32)]) -> Dimension {
    factors
        .iter()
        .try_fold(Dimension::DIMENSIONLESS, |acc, &(base, exponent)| {
            acc.checked_product(base.checked_powi(exponent)?)
        })
        .expect("valid derived dimension")
}

/// One SI prefix: an id-naming fragment, a symbol prefix, and its exact
/// power-of-ten scale relative to the unprefixed unit.
struct PrefixSpec {
    id_fragment: &'static str,
    symbol: &'static str,
    power10: i32,
}

const SI_PREFIXES: &[PrefixSpec] = &[
    PrefixSpec {
        id_fragment: "giga",
        symbol: "G",
        power10: 9,
    },
    PrefixSpec {
        id_fragment: "mega",
        symbol: "M",
        power10: 6,
    },
    PrefixSpec {
        id_fragment: "kilo",
        symbol: "k",
        power10: 3,
    },
    PrefixSpec {
        id_fragment: "centi",
        symbol: "c",
        power10: -2,
    },
    PrefixSpec {
        id_fragment: "milli",
        symbol: "m",
        power10: -3,
    },
    PrefixSpec {
        id_fragment: "micro",
        symbol: "u",
        power10: -6,
    },
    PrefixSpec {
        id_fragment: "micro-mu",
        symbol: "\u{b5}",
        power10: -6,
    },
    PrefixSpec {
        id_fragment: "nano",
        symbol: "n",
        power10: -9,
    },
];

/// Builds every prefixed variant of `base` (skipping any prefix whose
/// generated symbol collides with `skip_symbol`, e.g. `kilo` + `gram`
/// duplicating the base SI unit `kilogram`).
fn prefixed_variants(base: &UnitDef, id_tail: &str, skip_symbol: Option<&str>) -> Vec<UnitDef> {
    SI_PREFIXES
        .iter()
        .filter_map(|prefix| {
            let symbol = format!("{}{}", prefix.symbol, base.symbol);
            if Some(symbol.as_str()) == skip_symbol {
                return None;
            }
            let scale = base
                .scale_to_si
                .checked_mul(ExactScale::new(1, 1, prefix.power10).expect("valid prefix scale"))
                .expect("valid prefixed scale");
            Some(UnitDef {
                id: UnitId::new(format!("si:{}{}", prefix.id_fragment, id_tail)),
                symbol,
                dimension: base.dimension,
                scale_to_si: scale,
                offset_to_si: None,
                admitted_kinds: vec![],
                interval_form: None,
                provenance: base.provenance.clone(),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub source: String,
    pub version: String,
    pub retrieval_date: String,
    pub content_digest: String,
    pub generator_version: String,
}

#[derive(Clone, Debug)]
pub struct UnitRegistry {
    units: BTreeMap<UnitId, UnitDef>,
    symbols: BTreeMap<String, UnitId>,
    snapshot: Option<RegistrySnapshot>,
}

#[derive(Serialize)]
struct RegistryRef<'a> {
    units: Vec<&'a UnitDef>,
    snapshot: &'a RegistrySnapshot,
}

#[derive(Deserialize)]
struct RegistryWire {
    units: Vec<UnitDef>,
    snapshot: RegistrySnapshot,
}

impl Serialize for UnitRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let snapshot = self.snapshot.as_ref().ok_or_else(|| {
            serde::ser::Error::custom("cannot serialize an unfrozen unit registry")
        })?;
        RegistryRef {
            units: self.units.values().collect(),
            snapshot,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnitRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RegistryWire::deserialize(deserializer)?;
        let mut registry = Self::empty();
        for definition in wire.units {
            registry
                .insert(definition)
                .map_err(serde::de::Error::custom)?;
        }
        registry
            .freeze(wire.snapshot)
            .map_err(serde::de::Error::custom)
    }
}

impl UnitRegistry {
    pub fn empty() -> Self {
        Self {
            units: BTreeMap::new(),
            symbols: BTreeMap::new(),
            snapshot: None,
        }
    }

    pub fn insert(&mut self, mut definition: UnitDef) -> Result<(), RegistryError> {
        if self.snapshot.is_some() {
            return Err(RegistryError::Frozen);
        }
        if definition.id.as_str().trim().is_empty() || definition.symbol.trim().is_empty() {
            return Err(RegistryError::EmptyIdentity);
        }
        if definition.provenance.authority.trim().is_empty()
            || definition.provenance.version.trim().is_empty()
            || definition
                .provenance
                .persistent_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
        {
            return Err(RegistryError::InvalidProvenance(definition.id));
        }
        if definition
            .admitted_kinds
            .iter()
            .any(|kind| kind.as_str().trim().is_empty())
        {
            return Err(RegistryError::EmptyKind {
                unit: definition.id,
            });
        }
        definition.admitted_kinds.sort();
        definition.admitted_kinds.dedup();
        if self.units.contains_key(&definition.id) {
            return Err(RegistryError::DuplicateUnit(definition.id));
        }
        if self.symbols.contains_key(&definition.symbol) {
            return Err(RegistryError::DuplicateSymbol(definition.symbol));
        }
        self.symbols
            .insert(definition.symbol.clone(), definition.id.clone());
        self.units.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn freeze(mut self, snapshot: RegistrySnapshot) -> Result<Self, RegistryError> {
        if self.snapshot.is_some() {
            return Err(RegistryError::Frozen);
        }
        if snapshot.source.trim().is_empty()
            || snapshot.version.trim().is_empty()
            || snapshot.retrieval_date.trim().is_empty()
            || snapshot.content_digest.trim().is_empty()
            || snapshot.generator_version.trim().is_empty()
        {
            return Err(RegistryError::InvalidSnapshot);
        }
        for definition in self.units.values() {
            if definition.offset_to_si.is_some() && definition.admitted_kinds.is_empty() {
                return Err(RegistryError::AffineUnitRequiresKinds(
                    definition.id.clone(),
                ));
            }
            if definition.offset_to_si.is_some() && definition.interval_form.is_none() {
                return Err(RegistryError::OffsetWithoutInterval(definition.id.clone()));
            }
            if definition.offset_to_si.is_none() && definition.interval_form.is_some() {
                return Err(RegistryError::IntervalFormOnLinearUnit(
                    definition.id.clone(),
                ));
            }
            if let Some(interval_id) = &definition.interval_form {
                let interval = self.units.get(interval_id).ok_or_else(|| {
                    RegistryError::MissingIntervalUnit {
                        unit: definition.id.clone(),
                        interval: interval_id.clone(),
                    }
                })?;
                if interval.dimension != definition.dimension {
                    return Err(RegistryError::IntervalDimensionMismatch {
                        unit: definition.id.clone(),
                        interval: interval.id.clone(),
                    });
                }
                if interval.scale_to_si != definition.scale_to_si {
                    return Err(RegistryError::IntervalScaleMismatch {
                        unit: definition.id.clone(),
                        interval: interval.id.clone(),
                    });
                }
                if interval.offset_to_si.is_some() {
                    return Err(RegistryError::AffineIntervalUnit {
                        unit: definition.id.clone(),
                        interval: interval.id.clone(),
                    });
                }
                if interval.admitted_kinds.is_empty() {
                    return Err(RegistryError::IntervalUnitRequiresKinds {
                        unit: definition.id.clone(),
                        interval: interval.id.clone(),
                    });
                }
            }
        }
        self.snapshot = Some(snapshot);
        Ok(self)
    }

    pub fn snapshot(&self) -> Option<&RegistrySnapshot> {
        self.snapshot.as_ref()
    }

    pub fn is_frozen(&self) -> bool {
        self.snapshot.is_some()
    }

    pub fn get(&self, id: &UnitId) -> Option<&UnitDef> {
        self.units.get(id)
    }

    pub fn by_symbol(&self, symbol: &str) -> Option<&UnitDef> {
        self.symbols.get(symbol).and_then(|id| self.units.get(id))
    }

    pub fn canonicalize(&self, literal: &QuantityLiteral) -> Result<Quantity, QuantityError> {
        if self.snapshot.is_none() {
            return Err(QuantityError::UnfrozenRegistry);
        }
        if !literal.value.is_finite() {
            return Err(QuantityError::NonFinite);
        }
        let authored = match self.get(&literal.unit) {
            Some(definition) => definition,
            None => {
                // GX-F2: a literal's unit id may instead be a compound unit
                // expression (e.g. `W/(m*K)`) with no single registered
                // `UnitDef`. Compound expressions carry no kind admission or
                // affine offset, so they canonicalize directly.
                let expression = self
                    .parse_unit_expression(literal.unit.as_str())
                    .map_err(|_| QuantityError::UnknownUnit(literal.unit.clone()))?;
                let value_si = literal.value * expression.scale.as_f64();
                return Quantity::new(value_si, expression.dimension, literal.kind.clone());
            }
        };
        let admits_authored =
            authored.admitted_kinds.is_empty() || authored.admitted_kinds.contains(&literal.kind);
        let unit = if admits_authored {
            authored
        } else if let Some(interval_id) = &authored.interval_form {
            let interval = self
                .get(interval_id)
                .ok_or_else(|| QuantityError::UnknownUnit(interval_id.clone()))?;
            if !interval.admitted_kinds.is_empty()
                && !interval.admitted_kinds.contains(&literal.kind)
            {
                return Err(QuantityError::KindMismatch {
                    unit: authored.id.clone(),
                    kind: literal.kind.clone(),
                });
            }
            interval
        } else {
            return Err(QuantityError::KindMismatch {
                unit: authored.id.clone(),
                kind: literal.kind.clone(),
            });
        };
        if !unit.admitted_kinds.is_empty() && !unit.admitted_kinds.contains(&literal.kind) {
            return Err(QuantityError::KindMismatch {
                unit: unit.id.clone(),
                kind: literal.kind.clone(),
            });
        }
        let mut value_si = literal.value * unit.scale_to_si.as_f64();
        if let Some(offset) = unit.offset_to_si {
            value_si += offset.as_f64();
        }
        Quantity::new(value_si, unit.dimension, literal.kind.clone())
    }

    pub fn parse(
        &self,
        input: &str,
        kind: QuantityKindId,
    ) -> Result<(QuantityLiteral, DisplayUnit), ParseQuantityError> {
        let trimmed = input.trim_start();
        let start = input.len() - trimmed.len();
        let end = numeric_prefix_len(trimmed);
        if end == 0 {
            return Err(ParseQuantityError::MissingMagnitude { offset: start });
        }
        let numeric = &trimmed[..end];
        let value = numeric
            .parse::<f64>()
            .map_err(|_| ParseQuantityError::InvalidMagnitude {
                text: numeric.to_owned(),
                offset: start,
            })?;
        let after_number = &trimmed[end..];
        let whitespace = after_number.len() - after_number.trim_start().len();
        let symbol_offset = start + end + whitespace;
        let symbol = after_number.trim();
        if let Some(unit) = self.by_symbol(symbol) {
            return Ok((
                QuantityLiteral {
                    value,
                    unit: unit.id.clone(),
                    kind,
                },
                DisplayUnit {
                    unit: unit.id.clone(),
                    symbol: unit.symbol.clone(),
                },
            ));
        }
        // GX-F2: fall back to a compound unit expression (e.g. `W/(m*K)`),
        // keyed on its canonical rendering so a later `canonicalize` call
        // re-parses the same expression deterministically.
        let expression =
            self.parse_unit_expression(symbol)
                .map_err(|_| ParseQuantityError::UnknownSymbol {
                    symbol: symbol.to_owned(),
                    offset: symbol_offset,
                })?;
        let unit_id = UnitId::new(expression.rendering);
        Ok((
            QuantityLiteral {
                value,
                unit: unit_id.clone(),
                kind,
            },
            DisplayUnit {
                unit: unit_id,
                symbol: symbol.to_owned(),
            },
        ))
    }

    /// Small offline bootstrap. Production registries should replace its snapshot metadata
    /// with an admitted standards artifact without changing the library representation.
    pub fn si_bootstrap() -> Self {
        let provenance = UnitProvenance {
            authority: "BIPM SI Brochure".into(),
            version: "9th edition".into(),
            persistent_id: Some("https://www.bipm.org/en/publications/si-brochure".into()),
        };
        let snapshot = RegistrySnapshot {
            source: "bootstrap://quantitas/si".into(),
            version: "0.1.0".into(),
            retrieval_date: "2026-08-20".into(),
            content_digest: "bootstrap-not-an-admitted-registry".into(),
            generator_version: "quantitas/bootstrap/1".into(),
        };
        let mut registry = Self::empty();

        let metre = UnitDef {
            id: UnitId::new("si:metre"),
            symbol: "m".into(),
            dimension: Dimension::LENGTH,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let second = UnitDef {
            id: UnitId::new("si:second"),
            symbol: "s".into(),
            dimension: Dimension::TIME,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let kelvin = UnitDef {
            id: UnitId::new("si:kelvin"),
            symbol: "K".into(),
            dimension: Dimension::TEMPERATURE,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![
                QuantityKindId::thermodynamic_temperature(),
                QuantityKindId::temperature_difference(),
            ],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let degree_celsius = UnitDef {
            id: UnitId::new("si:degree-celsius"),
            symbol: "degC".into(),
            dimension: Dimension::TEMPERATURE,
            scale_to_si: ExactScale::ONE,
            offset_to_si: Some(ExactScale::new(27_315, 100, 0).expect("valid scale")),
            admitted_kinds: vec![QuantityKindId::thermodynamic_temperature()],
            interval_form: Some(UnitId::new("si:degree-celsius-interval")),
            provenance: provenance.clone(),
        };
        let degree_celsius_interval = UnitDef {
            id: UnitId::new("si:degree-celsius-interval"),
            symbol: "delta_degC".into(),
            dimension: Dimension::TEMPERATURE,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![QuantityKindId::temperature_difference()],
            interval_form: None,
            provenance: provenance.clone(),
        };

        // GX-F2: remaining SI base units.
        let kilogram = UnitDef {
            id: UnitId::new("si:kilogram"),
            symbol: "kg".into(),
            dimension: Dimension::MASS,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        // The gram, not the kilogram, is the unit SI prefixes attach to.
        let gram = UnitDef {
            id: UnitId::new("si:gram"),
            symbol: "g".into(),
            dimension: Dimension::MASS,
            scale_to_si: ExactScale::new(1, 1, -3).expect("valid scale"),
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let ampere = UnitDef {
            id: UnitId::new("si:ampere"),
            symbol: "A".into(),
            dimension: Dimension::CURRENT,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let mole = UnitDef {
            id: UnitId::new("si:mole"),
            symbol: "mol".into(),
            dimension: Dimension::AMOUNT,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let candela = UnitDef {
            id: UnitId::new("si:candela"),
            symbol: "cd".into(),
            dimension: Dimension::LUMINOUS_INTENSITY,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };

        // GX-F2: coherent SI derived units with named symbols.
        let hertz = UnitDef {
            id: UnitId::new("si:hertz"),
            symbol: "Hz".into(),
            dimension: derived_dimension(&[(Dimension::TIME, -1)]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let newton = UnitDef {
            id: UnitId::new("si:newton"),
            symbol: "N".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 1),
                (Dimension::TIME, -2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let pascal = UnitDef {
            id: UnitId::new("si:pascal"),
            symbol: "Pa".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, -1),
                (Dimension::TIME, -2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let joule = UnitDef {
            id: UnitId::new("si:joule"),
            symbol: "J".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 2),
                (Dimension::TIME, -2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let watt = UnitDef {
            id: UnitId::new("si:watt"),
            symbol: "W".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 2),
                (Dimension::TIME, -3),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let coulomb = UnitDef {
            id: UnitId::new("si:coulomb"),
            symbol: "C".into(),
            dimension: derived_dimension(&[(Dimension::CURRENT, 1), (Dimension::TIME, 1)]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let volt = UnitDef {
            id: UnitId::new("si:volt"),
            symbol: "V".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 2),
                (Dimension::TIME, -3),
                (Dimension::CURRENT, -1),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let farad = UnitDef {
            id: UnitId::new("si:farad"),
            symbol: "F".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, -1),
                (Dimension::LENGTH, -2),
                (Dimension::TIME, 4),
                (Dimension::CURRENT, 2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let ohm = UnitDef {
            id: UnitId::new("si:ohm"),
            symbol: "ohm".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 2),
                (Dimension::TIME, -3),
                (Dimension::CURRENT, -2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let siemens = UnitDef {
            id: UnitId::new("si:siemens"),
            symbol: "S".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, -1),
                (Dimension::LENGTH, -2),
                (Dimension::TIME, 3),
                (Dimension::CURRENT, 2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let tesla = UnitDef {
            id: UnitId::new("si:tesla"),
            symbol: "T".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::TIME, -2),
                (Dimension::CURRENT, -1),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let henry = UnitDef {
            id: UnitId::new("si:henry"),
            symbol: "H".into(),
            dimension: derived_dimension(&[
                (Dimension::MASS, 1),
                (Dimension::LENGTH, 2),
                (Dimension::TIME, -2),
                (Dimension::CURRENT, -2),
            ]),
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };
        let radian = UnitDef {
            id: UnitId::new("si:radian"),
            symbol: "rad".into(),
            dimension: Dimension::DIMENSIONLESS,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: provenance.clone(),
        };

        let mut definitions: Vec<UnitDef> = vec![
            metre.clone(),
            second.clone(),
            kelvin,
            degree_celsius,
            degree_celsius_interval,
            kilogram,
            gram.clone(),
            ampere.clone(),
            mole.clone(),
            candela,
            hertz.clone(),
            newton.clone(),
            pascal.clone(),
            joule.clone(),
            watt.clone(),
            coulomb.clone(),
            volt.clone(),
            farad.clone(),
            siemens.clone(),
            ohm.clone(),
            tesla.clone(),
            henry.clone(),
            radian,
        ];

        // GX-F2: SI prefixes (at least k, M, G, m, u/µ, n, c) on base/derived units.
        definitions.extend(prefixed_variants(&metre, "metre", None));
        definitions.extend(prefixed_variants(&gram, "gram", Some("kg")));
        definitions.extend(prefixed_variants(&second, "second", None));
        definitions.extend(prefixed_variants(&pascal, "pascal", None));
        definitions.extend(prefixed_variants(&joule, "joule", None));
        definitions.extend(prefixed_variants(&watt, "watt", None));
        definitions.extend(prefixed_variants(&hertz, "hertz", None));
        definitions.extend(prefixed_variants(&volt, "volt", None));
        definitions.extend(prefixed_variants(&ampere, "ampere", None));
        definitions.extend(prefixed_variants(&ohm, "ohm", None));
        definitions.extend(prefixed_variants(&farad, "farad", None));
        definitions.extend(prefixed_variants(&henry, "henry", None));
        definitions.extend(prefixed_variants(&coulomb, "coulomb", None));
        definitions.extend(prefixed_variants(&newton, "newton", None));
        definitions.extend(prefixed_variants(&tesla, "tesla", None));
        definitions.extend(prefixed_variants(&mole, "mole", None));
        definitions.extend(prefixed_variants(&siemens, "siemens", None));

        for definition in definitions {
            registry.insert(definition).expect("valid bootstrap unit");
        }
        registry.freeze(snapshot).expect("valid bootstrap registry")
    }
}

impl Default for UnitRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

fn numeric_prefix_len(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exponent = false;
    while end < bytes.len() {
        let character = bytes[end] as char;
        let accepted = match character {
            '0'..='9' => {
                seen_digit = true;
                true
            }
            '+' | '-' => end == 0 || matches!(bytes[end - 1] as char, 'e' | 'E'),
            '.' if !seen_dot && !seen_exponent => {
                seen_dot = true;
                true
            }
            'e' | 'E' if seen_digit && !seen_exponent => {
                seen_exponent = true;
                true
            }
            _ => false,
        };
        if !accepted {
            break;
        }
        end += 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RegistrySnapshot {
        RegistrySnapshot {
            source: "test://registry".into(),
            version: "1".into(),
            retrieval_date: "2026-08-20".into(),
            content_digest: "sha256:test".into(),
            generator_version: "test/1".into(),
        }
    }

    fn linear_unit(id: &str, symbol: &str) -> UnitDef {
        UnitDef {
            id: UnitId::new(id),
            symbol: symbol.into(),
            dimension: Dimension::LENGTH,
            scale_to_si: ExactScale::ONE,
            offset_to_si: None,
            admitted_kinds: vec![],
            interval_form: None,
            provenance: UnitProvenance {
                authority: "test".into(),
                version: "1".into(),
                persistent_id: None,
            },
        }
    }

    #[test]
    fn absolute_and_interval_celsius_are_distinct() {
        let registry = UnitRegistry::si_bootstrap();
        let absolute = registry
            .canonicalize(&QuantityLiteral {
                value: 25.0,
                unit: UnitId::new("si:degree-celsius"),
                kind: QuantityKindId::thermodynamic_temperature(),
            })
            .unwrap();
        let interval = registry
            .canonicalize(&QuantityLiteral {
                value: 10.0,
                unit: UnitId::new("si:degree-celsius"),
                kind: QuantityKindId::temperature_difference(),
            })
            .unwrap();
        assert!((absolute.value_si() - 298.15).abs() < 1e-12);
        assert!((interval.value_si() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn parse_preserves_display_unit() {
        let registry = UnitRegistry::si_bootstrap();
        let (literal, display) = registry
            .parse("25 degC", QuantityKindId::thermodynamic_temperature())
            .unwrap();
        assert_eq!(literal.value, 25.0);
        assert_eq!(display.symbol, "degC");
    }

    #[test]
    fn mutable_registry_cannot_canonicalize_or_overwrite_identity() {
        let mut registry = UnitRegistry::empty();
        let unit = linear_unit("test:metre", "m");
        registry.insert(unit.clone()).unwrap();
        assert!(matches!(
            registry.canonicalize(&QuantityLiteral {
                value: 1.0,
                unit: unit.id.clone(),
                kind: QuantityKindId::new("test:Length"),
            }),
            Err(QuantityError::UnfrozenRegistry)
        ));
        assert!(matches!(
            registry.insert(unit),
            Err(RegistryError::DuplicateUnit(_))
        ));
        assert!(serde_json::to_string(&registry).is_err());
    }

    #[test]
    fn frozen_registry_round_trips_as_a_validated_snapshot() {
        let registry = UnitRegistry::si_bootstrap();
        let encoded = serde_json::to_string(&registry).unwrap();
        let mut decoded: UnitRegistry = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.is_frozen());
        assert_eq!(decoded.snapshot(), registry.snapshot());
        assert_eq!(
            decoded.by_symbol("degC").unwrap().id.as_str(),
            "si:degree-celsius"
        );
        assert_eq!(
            decoded.insert(linear_unit("test:late", "late")),
            Err(RegistryError::Frozen)
        );
    }

    #[test]
    fn freeze_rejects_an_incomplete_affine_contract() {
        let mut registry = UnitRegistry::empty();
        let mut point = linear_unit("test:point", "point");
        point.offset_to_si = Some(ExactScale::ONE);
        point.admitted_kinds = vec![QuantityKindId::new("test:Point")];
        registry.insert(point).unwrap();
        assert!(matches!(
            registry.freeze(snapshot()),
            Err(RegistryError::OffsetWithoutInterval(unit)) if unit == UnitId::new("test:point")
        ));
    }
}
