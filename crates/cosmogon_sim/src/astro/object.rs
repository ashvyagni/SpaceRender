//! Universal object taxonomy and the data-truth (provenance) system. See docs/OBJECT_MODEL.md.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Body, BodyKind, EARTH_MASS};

/// What an object *is*, independent of how it is simulated.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectClass {
    MainSequenceStar,
    Protostar,
    BrownDwarf,
    GiantStar,
    WhiteDwarf,
    NeutronStar,
    Pulsar,
    Magnetar,
    StellarBlackHole,
    IntermediateBlackHole,
    SupermassiveBlackHole,
    RockyPlanet,
    OceanWorld,
    IceWorld,
    LavaWorld,
    GasGiant,
    IceGiant,
    DwarfPlanet,
    RoguePlanet,
    Moon,
    BinaryPlanet,
    Asteroid,
    Meteoroid,
    Comet,
    Centaur,
    TransNeptunianObject,
    Rings,
    Dust,
    DebrisField,
    Nebula,
    MolecularCloud,
    StarCluster,
    Galaxy,
    DwarfGalaxy,
    Quasar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectGroup {
    Stars,
    BlackHoles,
    Planets,
    Satellites,
    SmallBodies,
    Diffuse,
    LargeScale,
}

impl ObjectGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Stars => "Stars",
            Self::BlackHoles => "Black holes",
            Self::Planets => "Planets",
            Self::Satellites => "Moons",
            Self::SmallBodies => "Small bodies",
            Self::Diffuse => "Diffuse matter",
            Self::LargeScale => "Large-scale structure",
        }
    }
}

impl ObjectClass {
    pub const ALL: [ObjectClass; 35] = [
        Self::MainSequenceStar,
        Self::Protostar,
        Self::BrownDwarf,
        Self::GiantStar,
        Self::WhiteDwarf,
        Self::NeutronStar,
        Self::Pulsar,
        Self::Magnetar,
        Self::StellarBlackHole,
        Self::IntermediateBlackHole,
        Self::SupermassiveBlackHole,
        Self::RockyPlanet,
        Self::OceanWorld,
        Self::IceWorld,
        Self::LavaWorld,
        Self::GasGiant,
        Self::IceGiant,
        Self::DwarfPlanet,
        Self::RoguePlanet,
        Self::Moon,
        Self::BinaryPlanet,
        Self::Asteroid,
        Self::Meteoroid,
        Self::Comet,
        Self::Centaur,
        Self::TransNeptunianObject,
        Self::Rings,
        Self::Dust,
        Self::DebrisField,
        Self::Nebula,
        Self::MolecularCloud,
        Self::StarCluster,
        Self::Galaxy,
        Self::DwarfGalaxy,
        Self::Quasar,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MainSequenceStar => "Main-sequence star",
            Self::Protostar => "Protostar",
            Self::BrownDwarf => "Brown dwarf",
            Self::GiantStar => "Giant star",
            Self::WhiteDwarf => "White dwarf",
            Self::NeutronStar => "Neutron star",
            Self::Pulsar => "Pulsar",
            Self::Magnetar => "Magnetar",
            Self::StellarBlackHole => "Stellar-mass black hole",
            Self::IntermediateBlackHole => "Intermediate-mass black hole",
            Self::SupermassiveBlackHole => "Supermassive black hole",
            Self::RockyPlanet => "Rocky planet",
            Self::OceanWorld => "Ocean world",
            Self::IceWorld => "Ice world",
            Self::LavaWorld => "Lava world",
            Self::GasGiant => "Gas giant",
            Self::IceGiant => "Ice giant",
            Self::DwarfPlanet => "Dwarf planet",
            Self::RoguePlanet => "Rogue planet",
            Self::Moon => "Moon",
            Self::BinaryPlanet => "Binary planet",
            Self::Asteroid => "Asteroid",
            Self::Meteoroid => "Meteoroid",
            Self::Comet => "Comet",
            Self::Centaur => "Centaur",
            Self::TransNeptunianObject => "Trans-Neptunian object",
            Self::Rings => "Ring system",
            Self::Dust => "Dust",
            Self::DebrisField => "Debris field",
            Self::Nebula => "Nebula",
            Self::MolecularCloud => "Molecular cloud",
            Self::StarCluster => "Star cluster",
            Self::Galaxy => "Galaxy",
            Self::DwarfGalaxy => "Dwarf galaxy",
            Self::Quasar => "Quasar / AGN",
        }
    }

    pub fn group(self) -> ObjectGroup {
        use ObjectClass::*;
        match self {
            MainSequenceStar | Protostar | BrownDwarf | GiantStar | WhiteDwarf | NeutronStar | Pulsar | Magnetar => ObjectGroup::Stars,
            StellarBlackHole | IntermediateBlackHole | SupermassiveBlackHole => ObjectGroup::BlackHoles,
            RockyPlanet | OceanWorld | IceWorld | LavaWorld | GasGiant | IceGiant | DwarfPlanet | RoguePlanet => ObjectGroup::Planets,
            Moon | BinaryPlanet => ObjectGroup::Satellites,
            Asteroid | Meteoroid | Comet | Centaur | TransNeptunianObject => ObjectGroup::SmallBodies,
            Rings | Dust | DebrisField | Nebula | MolecularCloud => ObjectGroup::Diffuse,
            StarCluster | Galaxy | DwarfGalaxy | Quasar => ObjectGroup::LargeScale,
        }
    }

    /// Whether the creator can make this class today (others are planned milestones).
    pub fn creatable(self) -> bool {
        use ObjectClass::*;
        matches!(self, RockyPlanet | OceanWorld | IceWorld | GasGiant | IceGiant | DwarfPlanet | Moon | Asteroid | Comet)
    }

    /// The milestone that brings this class to life, for the creator's "coming later" note.
    pub fn planned_milestone(self) -> &'static str {
        use ObjectClass::*;
        match self.group() {
            ObjectGroup::Stars | ObjectGroup::BlackHoles => "Exotic objects (S6)",
            ObjectGroup::Diffuse | ObjectGroup::LargeScale => "Universe exploration (S5)",
            _ => match self {
                LavaWorld | RoguePlanet | BinaryPlanet => "Solar System expansion (S3)",
                Meteoroid | Centaur | TransNeptunianObject => "Solar System expansion (S3)",
                _ => "",
            },
        }
    }

    /// Physical regime used by the climate, terrain and rendering models.
    pub fn body_kind(self) -> BodyKind {
        use ObjectClass::*;
        match self {
            GasGiant => BodyKind::GasGiant,
            IceGiant => BodyKind::IceGiant,
            IceWorld | Comet | Centaur | TransNeptunianObject => BodyKind::Icy,
            _ => BodyKind::Rocky,
        }
    }
}

/// Infer the class of a body that does not store one (old saves, generated bodies).
pub fn infer_class(body: &Body) -> ObjectClass {
    if let Some(c) = body.class {
        return c;
    }
    if body.parent.is_some() {
        return ObjectClass::Moon;
    }
    match body.kind {
        BodyKind::GasGiant => ObjectClass::GasGiant,
        BodyKind::IceGiant => ObjectClass::IceGiant,
        BodyKind::Icy if body.mass < 0.01 * EARTH_MASS => ObjectClass::DwarfPlanet,
        BodyKind::Icy => ObjectClass::IceWorld,
        BodyKind::Rocky if body.mass < 1e-4 * EARTH_MASS => ObjectClass::Asteroid,
        BodyKind::Rocky if body.hydro.ocean_fraction > 0.9 => ObjectClass::OceanWorld,
        BodyKind::Rocky => ObjectClass::RockyPlanet,
    }
}

/// How much a value can be trusted. Never imply observational certainty that does not exist.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    /// From an observational dataset.
    Measured,
    /// Computed from measured values with a documented model.
    Derived,
    /// Model output with no direct measurement.
    Estimated,
    /// Generated by Cosmogon from a seed.
    Procedural,
    /// Changed by the user in a sandbox.
    UserModified,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Measured => "MEASURED",
            Self::Derived => "DERIVED",
            Self::Estimated => "ESTIMATED",
            Self::Procedural => "PROCEDURAL",
            Self::UserModified => "USER MODIFIED",
        }
    }
}

/// Where a body's values come from, with per-quantity overrides.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Provenance {
    /// Dataset or origin, e.g. "NASA/JPL Horizons (DE441) @1". Empty = infer from `Body::real`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// Quantities whose quality differs from the source default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, Quality>,
}

impl Provenance {
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() && self.fields.is_empty()
    }
}

/// Quantities that are always computed by Cosmogon's models rather than stored.
pub const DERIVED_QUANTITIES: &[&str] = &["gravity", "density", "escape_velocity", "temperature", "equilibrium_temperature", "insolation", "habitability", "position", "velocity", "orbit"];

impl Body {
    pub fn source_label(&self) -> String {
        if !self.provenance.source.is_empty() {
            self.provenance.source.clone()
        } else if self.real {
            "NASA planetary fact sheets (J2000 elements, hand-entered)".into()
        } else {
            format!("Cosmogon procedural generation (terrain seed {:#x})", self.terrain_seed)
        }
    }

    /// Quality of a named quantity (e.g. "mass", "radius", "orbit", "temperature").
    pub fn quality(&self, field: &str) -> Quality {
        if let Some(q) = self.provenance.fields.get(field) {
            return *q;
        }
        if !self.real {
            return if DERIVED_QUANTITIES.contains(&field) && field != "orbit" && field != "position" && field != "velocity" { Quality::Derived } else { Quality::Procedural };
        }
        match field {
            // Model outputs, calibrated against observations for real bodies.
            "temperature" | "equilibrium_temperature" | "insolation" | "habitability" => Quality::Derived,
            "gravity" | "density" | "escape_velocity" => Quality::Derived,
            "resources" | "deposits" | "terrain" => Quality::Procedural,
            _ => Quality::Measured,
        }
    }

    pub fn mark(&mut self, field: &str, q: Quality) {
        self.provenance.fields.insert(field.to_string(), q);
    }
}
