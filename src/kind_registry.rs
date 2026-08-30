use crate::{
    Dimension, KindRegistryError, QuantityKindId, RationalExponent, RegistrySnapshot, UnitId,
    UnitProvenance,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// One quantity-kind identity: its dimension, an optional canonical SI unit,
/// and any additional lookup names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindDef {
    pub id: QuantityKindId,
    pub dimension: Dimension,
    pub canonical_unit: Option<UnitId>,
    pub aliases: Vec<String>,
    pub provenance: UnitProvenance,
}

/// A conflict-checked, freezable registry of quantity-kind identities.
///
/// Mirrors [`crate::UnitRegistry`]'s design: mutable insertion followed by
/// an immutable, provenance-bearing frozen state; validating (de)serialization
/// rebuilds and revalidates every identity and lookup index.
#[derive(Clone, Debug)]
pub struct QuantityKindRegistry {
    kinds: BTreeMap<QuantityKindId, KindDef>,
    names: BTreeMap<String, QuantityKindId>,
    snapshot: Option<RegistrySnapshot>,
}

#[derive(Serialize)]
struct KindRegistryRef<'a> {
    kinds: Vec<&'a KindDef>,
    snapshot: &'a RegistrySnapshot,
}

#[derive(Deserialize)]
struct KindRegistryWire {
    kinds: Vec<KindDef>,
    snapshot: RegistrySnapshot,
}

impl Serialize for QuantityKindRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let snapshot = self.snapshot.as_ref().ok_or_else(|| {
            serde::ser::Error::custom("cannot serialize an unfrozen quantity-kind registry")
        })?;
        KindRegistryRef {
            kinds: self.kinds.values().collect(),
            snapshot,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuantityKindRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = KindRegistryWire::deserialize(deserializer)?;
        let mut registry = Self::empty();
        for definition in wire.kinds {
            registry
                .insert(definition)
                .map_err(serde::de::Error::custom)?;
        }
        registry
            .freeze(wire.snapshot)
            .map_err(serde::de::Error::custom)
    }
}

impl QuantityKindRegistry {
    pub fn empty() -> Self {
        Self {
            kinds: BTreeMap::new(),
            names: BTreeMap::new(),
            snapshot: None,
        }
    }

    pub fn insert(&mut self, mut definition: KindDef) -> Result<(), KindRegistryError> {
        if self.snapshot.is_some() {
            return Err(KindRegistryError::Frozen);
        }
        if definition.id.as_str().trim().is_empty() {
            return Err(KindRegistryError::EmptyIdentity);
        }
        if definition.provenance.authority.trim().is_empty()
            || definition.provenance.version.trim().is_empty()
            || definition
                .provenance
                .persistent_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
        {
            return Err(KindRegistryError::InvalidProvenance(definition.id));
        }
        if definition
            .aliases
            .iter()
            .any(|alias| alias.trim().is_empty())
        {
            return Err(KindRegistryError::EmptyAlias {
                kind: definition.id,
            });
        }
        definition.aliases.sort();
        definition.aliases.dedup();

        if self.kinds.contains_key(&definition.id) {
            return Err(KindRegistryError::DuplicateKind(definition.id));
        }

        let mut candidate_names: Vec<String> = vec![definition.id.as_str().to_owned()];
        let tail = tail_name(definition.id.as_str());
        if tail != definition.id.as_str() {
            candidate_names.push(tail.to_owned());
        }
        candidate_names.extend(definition.aliases.iter().cloned());
        candidate_names.sort();
        candidate_names.dedup();

        for name in &candidate_names {
            if self.names.contains_key(name) {
                return Err(KindRegistryError::DuplicateName(name.clone()));
            }
        }

        for name in candidate_names {
            self.names.insert(name, definition.id.clone());
        }
        self.kinds.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn freeze(mut self, snapshot: RegistrySnapshot) -> Result<Self, KindRegistryError> {
        if self.snapshot.is_some() {
            return Err(KindRegistryError::Frozen);
        }
        if snapshot.source.trim().is_empty()
            || snapshot.version.trim().is_empty()
            || snapshot.retrieval_date.trim().is_empty()
            || snapshot.content_digest.trim().is_empty()
            || snapshot.generator_version.trim().is_empty()
        {
            return Err(KindRegistryError::InvalidSnapshot);
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

    pub fn get(&self, id: &QuantityKindId) -> Option<&KindDef> {
        self.kinds.get(id)
    }

    /// Resolves a namespaced id (`si:ThermodynamicTemperature`), a bare tail
    /// name (`ThermodynamicTemperature`), or a registered alias.
    pub fn by_name(&self, name: &str) -> Option<&KindDef> {
        self.names.get(name).and_then(|id| self.kinds.get(id))
    }

    /// The SI quantity-kind bootstrap covering the dimensions and kinds used
    /// by the corpus and provider-signature examples in the GX contracts.
    /// Production registries should replace its snapshot metadata with an
    /// admitted standards artifact without changing the library
    /// representation, matching [`crate::UnitRegistry::si_bootstrap`].
    pub fn si_bootstrap() -> Self {
        let provenance = UnitProvenance {
            authority: "BIPM SI Brochure".into(),
            version: "9th edition".into(),
            persistent_id: Some("https://www.bipm.org/en/publications/si-brochure".into()),
        };
        let snapshot = RegistrySnapshot {
            source: "bootstrap://quantitas/si-kinds".into(),
            version: "0.1.0".into(),
            retrieval_date: "2026-08-30".into(),
            content_digest: "bootstrap-not-an-admitted-registry".into(),
            generator_version: "quantitas/bootstrap/1".into(),
        };
        let mut registry = Self::empty();
        for spec in SI_KIND_SPECS {
            let definition = KindDef {
                id: QuantityKindId::new(format!("si:{}", spec.name)),
                dimension: dim(spec.dimension),
                canonical_unit: spec.canonical_unit.map(UnitId::new),
                aliases: spec
                    .aliases
                    .iter()
                    .map(|alias| (*alias).to_owned())
                    .collect(),
                provenance: provenance.clone(),
            };
            registry.insert(definition).expect("valid bootstrap kind");
        }
        registry.freeze(snapshot).expect("valid bootstrap registry")
    }
}

impl Default for QuantityKindRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

fn tail_name(id: &str) -> &str {
    id.rsplit(':').next().unwrap_or(id)
}

/// Builds a [`Dimension`] from `[Mass, Length, Time, ElectricCurrent,
/// Temperature, Amount, LuminousIntensity]` integer exponents, matching
/// [`crate::SiBasis::ALL`]'s order.
fn dim(values: [i32; 7]) -> Dimension {
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

struct KindSpec {
    name: &'static str,
    /// `[Mass, Length, Time, ElectricCurrent, Temperature, Amount, LuminousIntensity]`
    dimension: [i32; 7],
    canonical_unit: Option<&'static str>,
    aliases: &'static [&'static str],
}

/// SI quantity kinds covering the corpus and provider-signature examples in
/// `GX-CONTRACTS.md` §C1. Dimensions follow the SI Brochure's coherent
/// derived units; kinds with no single named SI unit (most transport
/// coefficients and volumetric source terms) carry `canonical_unit: None`.
///
/// A few dimension choices are documented here because they are not named
/// SI derived units:
/// - `VolumetricSource` is the rate of a dimensionless quantity per volume
///   per time, i.e. `1/(m^3*s)`; it has no associated unit symbol.
/// - `AcousticSource` is a pressure rate, `Pa/s`; `AcousticBodySource` is a
///   body-force-like source, `N/m^3`. Both are chosen for dimensional
///   consistency with the corpus's acoustic transport terms, not because
///   either has a standard SI name.
/// - `MechanicalTorque` shares `Energy`'s dimension (`N*m` = `J`) but is a
///   distinct kind and is never assigned `si:joule` as its canonical unit,
///   since torque is conventionally reported in newton-metres.
const SI_KIND_SPECS: &[KindSpec] = &[
    KindSpec {
        name: "Dimensionless",
        dimension: [0, 0, 0, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "ThermodynamicTemperature",
        dimension: [0, 0, 0, 0, 1, 0, 0],
        canonical_unit: Some("si:kelvin"),
        aliases: &[],
    },
    KindSpec {
        name: "TemperatureDifference",
        dimension: [0, 0, 0, 0, 1, 0, 0],
        canonical_unit: Some("si:kelvin"),
        aliases: &[],
    },
    KindSpec {
        name: "Length",
        dimension: [0, 1, 0, 0, 0, 0, 0],
        canonical_unit: Some("si:metre"),
        aliases: &[],
    },
    KindSpec {
        name: "Time",
        dimension: [0, 0, 1, 0, 0, 0, 0],
        canonical_unit: Some("si:second"),
        aliases: &[],
    },
    KindSpec {
        name: "Mass",
        dimension: [1, 0, 0, 0, 0, 0, 0],
        canonical_unit: Some("si:kilogram"),
        aliases: &[],
    },
    KindSpec {
        name: "Amount",
        dimension: [0, 0, 0, 0, 0, 1, 0],
        canonical_unit: Some("si:mole"),
        aliases: &[],
    },
    KindSpec {
        name: "ElectricCurrent",
        dimension: [0, 0, 0, 1, 0, 0, 0],
        canonical_unit: Some("si:ampere"),
        aliases: &[],
    },
    KindSpec {
        name: "Velocity",
        dimension: [0, 1, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Acceleration",
        dimension: [0, 1, -2, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Density",
        dimension: [1, -3, 0, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Pressure",
        dimension: [1, -1, -2, 0, 0, 0, 0],
        canonical_unit: Some("si:pascal"),
        aliases: &["Stress"],
    },
    KindSpec {
        name: "Energy",
        dimension: [1, 2, -2, 0, 0, 0, 0],
        canonical_unit: Some("si:joule"),
        aliases: &[],
    },
    KindSpec {
        name: "Power",
        dimension: [1, 2, -3, 0, 0, 0, 0],
        canonical_unit: Some("si:watt"),
        aliases: &[],
    },
    KindSpec {
        name: "Force",
        dimension: [1, 1, -2, 0, 0, 0, 0],
        canonical_unit: Some("si:newton"),
        aliases: &[],
    },
    KindSpec {
        name: "MechanicalBodyForce",
        dimension: [1, -2, -2, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &["BodyForce"],
    },
    KindSpec {
        name: "MechanicalTorque",
        dimension: [1, 2, -2, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "ThermalConductivity",
        dimension: [1, 1, -3, 0, -1, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "SpecificHeat",
        dimension: [0, 2, -2, 0, -1, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "VolumetricHeatSource",
        dimension: [1, -1, -3, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Diffusivity",
        dimension: [0, 2, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Concentration",
        dimension: [0, -3, 0, 0, 0, 1, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "SpeciesFlux",
        dimension: [0, -2, -1, 0, 0, 1, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "SpeciesSource",
        dimension: [0, -3, -1, 0, 0, 1, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "MassSource",
        dimension: [1, -3, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "VolumetricSource",
        dimension: [0, -3, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Permittivity",
        dimension: [-1, -3, 4, 2, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "ElectricalConductivity",
        dimension: [-1, -3, 3, 2, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "ChargeDensity",
        dimension: [0, -3, 1, 1, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "CurrentDensity",
        dimension: [0, -2, 0, 1, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "ElectricPotential",
        dimension: [1, 2, -3, -1, 0, 0, 0],
        canonical_unit: Some("si:volt"),
        aliases: &[],
    },
    KindSpec {
        name: "VoltageSource",
        dimension: [1, 2, -3, -1, 0, 0, 0],
        canonical_unit: Some("si:volt"),
        aliases: &[],
    },
    KindSpec {
        name: "CurrentSource",
        dimension: [0, 0, 0, 1, 0, 0, 0],
        canonical_unit: Some("si:ampere"),
        aliases: &[],
    },
    KindSpec {
        name: "Magnetization",
        dimension: [0, -1, 0, 1, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "MagneticFluxDensity",
        dimension: [1, 0, -2, -1, 0, 0, 0],
        canonical_unit: Some("si:tesla"),
        aliases: &[],
    },
    KindSpec {
        name: "Permeability",
        dimension: [0, 2, 0, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "DynamicViscosity",
        dimension: [1, -1, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "KinematicViscosity",
        dimension: [0, 2, -1, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "LameFirstParameter",
        dimension: [1, -1, -2, 0, 0, 0, 0],
        canonical_unit: Some("si:pascal"),
        aliases: &[],
    },
    KindSpec {
        name: "ShearModulus",
        dimension: [1, -1, -2, 0, 0, 0, 0],
        canonical_unit: Some("si:pascal"),
        aliases: &[],
    },
    KindSpec {
        name: "AcousticSource",
        dimension: [1, -1, -3, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "AcousticBodySource",
        dimension: [1, -2, -2, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Frequency",
        dimension: [0, 0, -1, 0, 0, 0, 0],
        canonical_unit: Some("si:hertz"),
        aliases: &[],
    },
    KindSpec {
        name: "Area",
        dimension: [0, 2, 0, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Volume",
        dimension: [0, 3, 0, 0, 0, 0, 0],
        canonical_unit: None,
        aliases: &[],
    },
    KindSpec {
        name: "Angle",
        dimension: [0, 0, 0, 0, 0, 0, 0],
        canonical_unit: Some("si:radian"),
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitRegistry;

    fn snapshot() -> RegistrySnapshot {
        RegistrySnapshot {
            source: "test://kind-registry".into(),
            version: "1".into(),
            retrieval_date: "2026-08-30".into(),
            content_digest: "sha256:test".into(),
            generator_version: "test/1".into(),
        }
    }

    fn provenance() -> UnitProvenance {
        UnitProvenance {
            authority: "test".into(),
            version: "1".into(),
            persistent_id: None,
        }
    }

    fn kind(id: &str) -> KindDef {
        KindDef {
            id: QuantityKindId::new(id),
            dimension: Dimension::LENGTH,
            canonical_unit: None,
            aliases: vec![],
            provenance: provenance(),
        }
    }

    #[test]
    fn bootstrap_kinds_have_consistent_dimensions_against_their_canonical_units() {
        let kinds = QuantityKindRegistry::si_bootstrap();
        let units = UnitRegistry::si_bootstrap();
        for spec in SI_KIND_SPECS {
            let def = kinds
                .get(&QuantityKindId::new(format!("si:{}", spec.name)))
                .expect("bootstrap kind present");
            if let Some(unit_id) = &def.canonical_unit {
                let unit = units.get(unit_id).unwrap_or_else(|| {
                    panic!(
                        "canonical unit `{unit_id}` for kind `{}` not registered",
                        spec.name
                    )
                });
                assert_eq!(
                    unit.dimension, def.dimension,
                    "kind `{}` dimension does not match canonical unit `{unit_id}`",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn by_name_accepts_bare_tail_and_namespaced_ids() {
        let kinds = QuantityKindRegistry::si_bootstrap();
        let by_tail = kinds.by_name("ThermodynamicTemperature").unwrap();
        let by_namespaced = kinds.by_name("si:ThermodynamicTemperature").unwrap();
        assert_eq!(by_tail.id, by_namespaced.id);
        assert_eq!(
            by_tail.id,
            QuantityKindId::new("si:ThermodynamicTemperature")
        );
    }

    #[test]
    fn by_name_accepts_registered_alias() {
        let kinds = QuantityKindRegistry::si_bootstrap();
        let via_alias = kinds.by_name("Stress").unwrap();
        assert_eq!(via_alias.id, QuantityKindId::new("si:Pressure"));
        let via_alias = kinds.by_name("BodyForce").unwrap();
        assert_eq!(via_alias.id, QuantityKindId::new("si:MechanicalBodyForce"));
    }

    #[test]
    fn insert_refuses_duplicate_kind_id_and_conflicting_name() {
        let mut registry = QuantityKindRegistry::empty();
        registry.insert(kind("test:Length")).unwrap();
        assert_eq!(
            registry.insert(kind("test:Length")),
            Err(KindRegistryError::DuplicateKind(QuantityKindId::new(
                "test:Length"
            )))
        );

        let mut other = kind("test:OtherLength");
        other.aliases = vec!["Length".into()];
        assert_eq!(
            registry.insert(other),
            Err(KindRegistryError::DuplicateName("Length".into()))
        );
    }

    #[test]
    fn frozen_registry_round_trips_as_a_validated_snapshot() {
        let registry = QuantityKindRegistry::si_bootstrap();
        let encoded = serde_json::to_string(&registry).unwrap();
        let mut decoded: QuantityKindRegistry = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.is_frozen());
        assert_eq!(decoded.snapshot(), registry.snapshot());
        assert_eq!(
            decoded.by_name("Pressure").unwrap().id,
            QuantityKindId::new("si:Pressure")
        );
        assert_eq!(
            decoded.insert(kind("test:late")),
            Err(KindRegistryError::Frozen)
        );
    }

    #[test]
    fn mutable_registry_cannot_serialize_or_overwrite_identity() {
        let mut registry = QuantityKindRegistry::empty();
        registry.insert(kind("test:Length")).unwrap();
        assert!(serde_json::to_string(&registry).is_err());
        assert_eq!(
            registry.insert(kind("test:Length")),
            Err(KindRegistryError::DuplicateKind(QuantityKindId::new(
                "test:Length"
            )))
        );
    }

    #[test]
    fn freeze_rejects_incomplete_snapshot() {
        let mut registry = QuantityKindRegistry::empty();
        registry.insert(kind("test:Length")).unwrap();
        let mut bad_snapshot = snapshot();
        bad_snapshot.source = String::new();
        assert!(matches!(
            registry.freeze(bad_snapshot),
            Err(KindRegistryError::InvalidSnapshot)
        ));
    }
}
