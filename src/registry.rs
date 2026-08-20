use crate::{
    Dimension, DisplayUnit, ExactScale, ParseQuantityError, Quantity, QuantityError,
    QuantityKindId, QuantityLiteral, UnitDef, UnitId, UnitProvenance,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub source: String,
    pub version: String,
    pub retrieval_date: String,
    pub content_digest: String,
    pub generator_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitRegistry {
    units: BTreeMap<UnitId, UnitDef>,
    symbols: BTreeMap<String, UnitId>,
    pub snapshot: RegistrySnapshot,
}

impl UnitRegistry {
    pub fn empty(snapshot: RegistrySnapshot) -> Self {
        Self {
            units: BTreeMap::new(),
            symbols: BTreeMap::new(),
            snapshot,
        }
    }

    pub fn insert(&mut self, definition: UnitDef) -> Option<UnitDef> {
        self.symbols
            .insert(definition.symbol.clone(), definition.id.clone());
        self.units.insert(definition.id.clone(), definition)
    }

    pub fn get(&self, id: &UnitId) -> Option<&UnitDef> {
        self.units.get(id)
    }

    pub fn by_symbol(&self, symbol: &str) -> Option<&UnitDef> {
        self.symbols.get(symbol).and_then(|id| self.units.get(id))
    }

    pub fn canonicalize(&self, literal: &QuantityLiteral) -> Result<Quantity, QuantityError> {
        if !literal.value.is_finite() {
            return Err(QuantityError::NonFinite);
        }
        let authored = self
            .get(&literal.unit)
            .ok_or_else(|| QuantityError::UnknownUnit(literal.unit.clone()))?;
        let is_interval = literal.kind == QuantityKindId::temperature_difference();
        let unit = if is_interval && authored.offset_to_si.is_some() {
            let interval_id = authored
                .interval_form
                .as_ref()
                .ok_or_else(|| QuantityError::MissingIntervalForm(authored.id.clone()))?;
            self.get(interval_id)
                .ok_or_else(|| QuantityError::UnknownUnit(interval_id.clone()))?
        } else {
            authored
        };
        if !unit.admitted_kinds.is_empty() && !unit.admitted_kinds.contains(&literal.kind) {
            return Err(QuantityError::KindMismatch {
                unit: unit.id.clone(),
                kind: literal.kind.clone(),
            });
        }
        let mut value_si = literal.value * unit.scale_to_si.as_f64();
        if !is_interval && let Some(offset) = unit.offset_to_si {
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
        let unit = self
            .by_symbol(symbol)
            .ok_or_else(|| ParseQuantityError::UnknownSymbol {
                symbol: symbol.to_owned(),
                offset: symbol_offset,
            })?;
        Ok((
            QuantityLiteral {
                value,
                unit: unit.id.clone(),
                kind,
            },
            DisplayUnit {
                unit: unit.id.clone(),
                symbol: unit.symbol.clone(),
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
        let mut registry = Self::empty(RegistrySnapshot {
            source: "bootstrap://quantitas/si".into(),
            version: "0.1.0".into(),
            retrieval_date: "2026-08-20".into(),
            content_digest: "bootstrap-not-an-admitted-registry".into(),
            generator_version: "quantitas/bootstrap/1".into(),
        });

        let definitions = [
            UnitDef {
                id: UnitId::new("si:metre"),
                symbol: "m".into(),
                dimension: Dimension::LENGTH,
                scale_to_si: ExactScale::ONE,
                offset_to_si: None,
                admitted_kinds: vec![],
                interval_form: None,
                provenance: provenance.clone(),
            },
            UnitDef {
                id: UnitId::new("si:second"),
                symbol: "s".into(),
                dimension: Dimension::TIME,
                scale_to_si: ExactScale::ONE,
                offset_to_si: None,
                admitted_kinds: vec![],
                interval_form: None,
                provenance: provenance.clone(),
            },
            UnitDef {
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
            },
            UnitDef {
                id: UnitId::new("si:degree-celsius"),
                symbol: "degC".into(),
                dimension: Dimension::TEMPERATURE,
                scale_to_si: ExactScale::ONE,
                offset_to_si: Some(ExactScale::new(27_315, 100, 0).expect("valid scale")),
                admitted_kinds: vec![QuantityKindId::thermodynamic_temperature()],
                interval_form: Some(UnitId::new("si:degree-celsius-interval")),
                provenance: provenance.clone(),
            },
            UnitDef {
                id: UnitId::new("si:degree-celsius-interval"),
                symbol: "delta_degC".into(),
                dimension: Dimension::TEMPERATURE,
                scale_to_si: ExactScale::ONE,
                offset_to_si: None,
                admitted_kinds: vec![QuantityKindId::temperature_difference()],
                interval_form: None,
                provenance,
            },
        ];
        for definition in definitions {
            registry.insert(definition);
        }
        registry
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
}
