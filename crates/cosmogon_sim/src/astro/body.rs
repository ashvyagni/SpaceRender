//! Planets, moons and dwarf bodies.

use serde::{Deserialize, Serialize};

use super::{Orbit, EARTH_MASS, EARTH_RADIUS, G};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// Silicate/iron world with a solid surface.
    Rocky,
    /// Water-ice dominated world (icy moons, ice dwarfs). May hide a subsurface ocean.
    Icy,
    /// Hydrogen/helium giant.
    GasGiant,
    /// Water/ammonia/methane-rich giant.
    IceGiant,
}

impl BodyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rocky => "Rocky world",
            Self::Icy => "Icy world",
            Self::GasGiant => "Gas giant",
            Self::IceGiant => "Ice giant",
        }
    }
    pub fn has_surface(self) -> bool {
        matches!(self, Self::Rocky | Self::Icy)
    }
}

/// Bulk atmosphere. Fractions are by volume and sum to ~1 when `pressure_bar > 0`.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Atmosphere {
    pub pressure_bar: f64,
    pub n2: f64,
    pub o2: f64,
    pub co2: f64,
    pub h2o: f64,
    pub ch4: f64,
    pub h2he: f64,
}

impl Atmosphere {
    pub fn is_present(&self) -> bool {
        self.pressure_bar > 1e-4
    }

    pub fn partial(&self, frac: f64) -> f64 {
        self.pressure_bar * frac
    }

    /// Short human description, e.g. "1.00 bar N₂/O₂".
    pub fn describe(&self) -> String {
        if !self.is_present() {
            return "None (vacuum)".into();
        }
        let mut gases = [
            ("N₂", self.n2),
            ("O₂", self.o2),
            ("CO₂", self.co2),
            ("H₂O", self.h2o),
            ("CH₄", self.ch4),
            ("H₂/He", self.h2he),
        ];
        gases.sort_by(|a, b| b.1.total_cmp(&a.1));
        let main: Vec<&str> = gases.iter().filter(|g| g.1 >= 0.05).map(|g| g.0).collect();
        let p = if self.pressure_bar >= 100.0 { ">100 bar".to_string() } else { format!("{:.3} bar", self.pressure_bar) };
        format!("{p} {}", main.join("/"))
    }

    /// Renormalise fractions after a change to one component.
    pub fn normalise(&mut self) {
        let s = self.n2 + self.o2 + self.co2 + self.h2o + self.ch4 + self.h2he;
        if s > 0.0 {
            self.n2 /= s;
            self.o2 /= s;
            self.co2 /= s;
            self.h2o /= s;
            self.ch4 /= s;
            self.h2he /= s;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Hydrosphere {
    /// Total surface/near-surface water in Earth-ocean equivalents.
    pub water_inventory: f64,
    /// Fraction of the surface covered by liquid water.
    pub ocean_fraction: f64,
    /// Fraction of the surface covered by ice.
    pub ice_fraction: f64,
    /// Liquid water under an ice shell (tidally/radiogenically heated).
    pub subsurface_ocean: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceKind {
    Iron,
    Copper,
    Tin,
    Coal,
    Oil,
    Uranium,
    RareMetals,
}

impl ResourceKind {
    pub const ALL: [ResourceKind; 7] = [
        Self::Iron,
        Self::Copper,
        Self::Tin,
        Self::Coal,
        Self::Oil,
        Self::Uranium,
        Self::RareMetals,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Iron => "iron",
            Self::Copper => "copper",
            Self::Tin => "tin",
            Self::Coal => "coal",
            Self::Oil => "oil",
            Self::Uranium => "uranium",
            Self::RareMetals => "rare_metals",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Iron => "Iron",
            Self::Copper => "Copper",
            Self::Tin => "Tin",
            Self::Coal => "Coal",
            Self::Oil => "Oil & gas",
            Self::Uranium => "Uranium",
            Self::RareMetals => "Rare metals",
        }
    }
}

/// Planet-wide resource endowment, relative to Earth (Earth = 1.0 for each).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Resources {
    pub iron: f64,
    pub copper: f64,
    pub tin: f64,
    /// Fossil fuels come from buried biomass, so they *grow* with the biosphere's history.
    pub coal: f64,
    pub oil: f64,
    pub uranium: f64,
    pub rare_metals: f64,
    /// Arable land potential (needs climate + soil; refined by the biosphere).
    pub fertile_land: f64,
    pub fresh_water: f64,
}

impl Resources {
    pub fn get(&self, kind: ResourceKind) -> f64 {
        match kind {
            ResourceKind::Iron => self.iron,
            ResourceKind::Copper => self.copper,
            ResourceKind::Tin => self.tin,
            ResourceKind::Coal => self.coal,
            ResourceKind::Oil => self.oil,
            ResourceKind::Uranium => self.uranium,
            ResourceKind::RareMetals => self.rare_metals,
        }
    }

    /// Look up by data-file key (used by the technology graph).
    pub fn by_key(&self, key: &str) -> Option<f64> {
        Some(match key {
            "fertile_land" => self.fertile_land,
            "fresh_water" => self.fresh_water,
            k => self.get(ResourceKind::ALL.into_iter().find(|r| r.key() == k)?),
        })
    }
}

/// A geographically located deposit. Positions are latitude/longitude in radians.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Deposit {
    pub kind: ResourceKind,
    pub lat: f64,
    pub lon: f64,
    pub richness: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Rings {
    pub inner: f64,
    pub outer: f64,
    pub opacity: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Body {
    pub id: u32,
    pub name: String,
    pub kind: BodyKind,
    /// Index of the body this one orbits; `None` = the system's primary star.
    pub parent: Option<u32>,
    pub orbit: Orbit,
    /// kg
    pub mass: f64,
    /// m
    pub radius: f64,
    /// Sidereal rotation period (s). Negative = retrograde.
    pub rotation_period: f64,
    /// Obliquity (rad).
    pub axial_tilt: f64,
    pub tidally_locked: bool,
    /// Bond albedo.
    pub albedo: f64,
    pub atmosphere: Atmosphere,
    pub hydro: Hydrosphere,
    /// Dipole strength relative to Earth.
    pub magnetic_field: f64,
    /// Geological activity relative to present-day Earth (0 = dead).
    pub geology: f64,
    /// Radiative equilibrium temperature (K).
    pub equilibrium_temperature: f64,
    /// Mean surface (or 1-bar level) temperature (K).
    pub temperature: f64,
    pub resources: Resources,
    pub deposits: Vec<Deposit>,
    pub rings: Option<Rings>,
    /// Seed for the planet's procedural surface.
    pub terrain_seed: u64,
    /// Measured elevation that replaces the procedural surface (real bodies).
    #[serde(default)]
    pub elevation_data: Option<crate::planet::terrain::ElevationData>,
    /// Terrain elevation threshold that yields the current ocean fraction.
    pub sea_level: f64,
    /// Base colour hint (used for giants and for real bodies' tint).
    pub color: [f32; 3],
    /// True when the values come from real measurements (Sol scenario).
    pub real: bool,
    /// Calibration offset (K) so real bodies match observed temperatures while still
    /// responding to modelled changes (e.g. greenhouse forcing). 0 for procedural bodies.
    #[serde(default)]
    pub climate_bias: f64,
    /// Orbital-forcing (Milankovitch-like) glacial cycle period in years; 0 = none.
    #[serde(default)]
    pub glacial_cycle_years: f64,
    /// Fraction of each cycle spent in a stable interglacial. Large moons stabilise the
    /// axial tilt and lengthen interglacials.
    #[serde(default)]
    pub interglacial_fraction: f64,
    #[serde(default)]
    pub glacial_phase_years: f64,
    /// For real bodies: glacial conditions persist until this time (years rel. J2000).
    #[serde(default)]
    pub glacial_until_years: Option<f64>,
    /// Object class (None = inferred from kind, parent and mass; see `object::infer_class`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<super::ObjectClass>,
    /// Where the values come from and how much to trust each one.
    #[serde(default, skip_serializing_if = "super::Provenance::is_empty")]
    pub provenance: super::Provenance,
    /// Set when the body no longer exists (deleted, merged, destroyed). Bodies are never
    /// removed from storage so that every index-based reference stays valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<Removal>,
    /// Impacts this body has received.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impacts: Vec<crate::impact::ImpactRecord>,
    /// Transient surface cooling after a large impact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_winter: Option<crate::impact::ImpactWinter>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum RemovalCause {
    /// Deleted by the user.
    Deleted,
    /// Collided with and merged into this body (index in the same system; `None` = the star).
    MergedInto(Option<u32>),
    /// Destroyed by a supernova.
    Vaporised,
    /// Left the system on an unbound orbit.
    Ejected,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Removal {
    pub time: f64,
    pub cause: RemovalCause,
}

impl Body {
    pub fn mu(&self) -> f64 {
        G * self.mass
    }
    /// Surface gravity (m/s²).
    pub fn gravity(&self) -> f64 {
        self.mu() / (self.radius * self.radius)
    }
    pub fn gravity_g(&self) -> f64 {
        self.gravity() / 9.80665
    }
    /// Bulk density (kg/m³).
    pub fn density(&self) -> f64 {
        self.mass / (4.0 / 3.0 * std::f64::consts::PI * self.radius.powi(3))
    }
    pub fn escape_velocity(&self) -> f64 {
        (2.0 * self.mu() / self.radius).sqrt()
    }
    pub fn mass_earths(&self) -> f64 {
        self.mass / EARTH_MASS
    }
    pub fn radius_earths(&self) -> f64 {
        self.radius / EARTH_RADIUS
    }
    pub fn surface_area_km2(&self) -> f64 {
        4.0 * std::f64::consts::PI * (self.radius / 1000.0).powi(2)
    }
    pub fn land_fraction(&self) -> f64 {
        if self.kind.has_surface() {
            (1.0 - self.hydro.ocean_fraction - self.hydro.ice_fraction).max(0.0)
        } else {
            0.0
        }
    }

    /// Approximate Δv (km/s) for a chemical rocket to reach low orbit from the surface:
    /// circular orbital speed plus gravity and drag losses that grow with g and air density.
    /// Earth ≈ 9.4 km/s. (Cf. Hippke 2018 on the "super-Earth trap".)
    pub fn launch_delta_v_kms(&self) -> f64 {
        let v_orbit = (self.mu() / (self.radius * 1.01)).sqrt() / 1000.0;
        let gravity_loss = 1.3 * self.gravity_g().sqrt();
        let drag_loss = 0.15 * self.atmosphere.pressure_bar.min(100.0).sqrt();
        v_orbit + gravity_loss + drag_loss
    }

    /// Climate stability at time `t` (s): 1 = stable interglacial, ~0.3 = glacial swings.
    pub fn climate_stability(&self, t: f64) -> f64 {
        let years = t / crate::time::SECONDS_PER_YEAR;
        if let Some(until) = self.glacial_until_years {
            return if years < until { 0.3 } else { 1.0 };
        }
        if self.glacial_cycle_years <= 0.0 {
            return 1.0;
        }
        let phase = ((years - self.glacial_phase_years) / self.glacial_cycle_years).rem_euclid(1.0);
        if phase < self.interglacial_fraction { 1.0 } else { 0.3 }
    }

    pub fn exists(&self) -> bool {
        self.removed.is_none()
    }

    pub fn class(&self) -> super::ObjectClass {
        super::object::infer_class(self)
    }

    pub fn is_moon(&self) -> bool {
        self.parent.is_some()
    }
}
