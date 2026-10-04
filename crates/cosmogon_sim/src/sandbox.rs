//! Sandbox editing and the physical consequence pipeline.
//!
//! **Edits.** Every user modification is an [`Edit`] command. `Universe::apply_edit`
//! validates it, makes the system dynamic, brings the N-body state exactly to the current
//! time, applies the change, restarts the step grid, journals it (reproducibility) and
//! propagates its consequences. The UI never mutates the model directly.
//!
//! **Consequences.** Physics produces events (collisions, orbit changes); this module
//! carries them down the causal chain: orbit → insolation → climate → habitability →
//! biosphere → civilization. There is no "damage" number: each layer responds through
//! its own model. See docs/SANDBOX_VISION.md.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::dynamics::{ContactEvent, PhysicsSettings, Slot, State};
use crate::astro::{sol, Body, BodyKind, ObjectClass, Orbit, Quality, Removal, RemovalCause, Star, StarSystem, AU, EARTH_MASS, G, SOLAR_MASS};
use crate::civ::CivStatus;
use crate::habitability::assess;
use crate::history::{Category, Event};
use crate::impact::{self, ImpactClass, ImpactRecord, ImpactWinter, MEGATON_TNT_J};
use crate::life::Stage;
use crate::planet::environment::insolation;
use crate::planet::terrain;
use crate::time::{format_date, SECONDS_PER_DAY, SECONDS_PER_YEAR};
use crate::universe::{refresh_climate, refresh_derived_resources, BodyRef, Universe};
use crate::Vec3d;

/// A scalar property of a body that the user may set.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BodyProperty {
    Name(String),
    /// kg
    Mass(f64),
    /// m
    Radius(f64),
    /// s (negative = retrograde)
    RotationPeriod(f64),
    /// rad
    AxialTilt(f64),
    Albedo(f64),
    /// bar
    SurfacePressure(f64),
    /// volume fraction
    Co2Fraction(f64),
    /// Earth-ocean equivalents
    WaterInventory(f64),
}

impl BodyProperty {
    fn field(&self) -> &'static str {
        match self {
            Self::Name(_) => "name",
            Self::Mass(_) => "mass",
            Self::Radius(_) => "radius",
            Self::RotationPeriod(_) => "rotation_period",
            Self::AxialTilt(_) => "axial_tilt",
            Self::Albedo(_) => "albedo",
            Self::SurfacePressure(_) => "atmosphere",
            Self::Co2Fraction(_) => "atmosphere",
            Self::WaterInventory(_) => "water",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum StarProperty {
    /// Solar masses. Luminosity, radius, temperature and lifetime follow from the stellar
    /// model (physically consistent mode).
    Mass(f64),
    /// [Fe/H] dex
    Metallicity(f64),
    /// s
    Age(f64),
    Name(String),
    /// Replace the star by a remnant of the same mass (a thought experiment: what if the
    /// Sun were a black hole?) or, with `Normal`, a living star again.
    Kind(crate::astro::star::StarKind),
}

/// A user modification of a sandbox.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Edit {
    /// Add a body with a state in the system frame (metres, m/s).
    AddBody { system: u32, body: Box<Body>, state: State },
    RemoveBody { body: BodyRef },
    /// Set position and velocity (system frame).
    SetState { body: BodyRef, state: State },
    /// Add a velocity change (m/s).
    Impulse { body: BodyRef, dv: Vec3d },
    /// Put the body on the given orbit around its parent; angles in rad, mean anomaly *now*.
    SetOrbit { body: BodyRef, a: f64, e: f64, i: f64, node: f64, peri: f64, mean_anomaly: f64 },
    SetProperty { body: BodyRef, property: BodyProperty },
    SetStar { system: u32, property: StarProperty },
    /// Choose analytic (Kepler) or N-body gravity and its settings.
    SetPhysics { system: u32, nbody: bool, settings: PhysicsSettings },
    /// Intervene in a civilization's history (limited; see `intervene`).
    Intervene { system: u32, civ: u32, action: crate::intervene::Intervention },
}

impl Edit {
    pub fn system(&self) -> u32 {
        match self {
            Edit::AddBody { system, .. } | Edit::SetStar { system, .. } | Edit::SetPhysics { system, .. } | Edit::Intervene { system, .. } => *system,
            Edit::RemoveBody { body } | Edit::SetState { body, .. } | Edit::Impulse { body, .. } | Edit::SetOrbit { body, .. } | Edit::SetProperty { body, .. } => body.system,
        }
    }
}

/// An applied edit, kept in the universe's journal (part of the experiment manifest).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EditRecord {
    pub time: f64,
    pub summary: String,
    pub edit: Edit,
}

#[derive(Clone, Debug, Default)]
pub struct EditOutcome {
    /// The body created by `AddBody`.
    pub created: Option<BodyRef>,
    pub summary: String,
}

// ── Presets for the object creator ──────────────────────────────────────────

/// A starting point for a new object, built from a real analogue where one exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub class: ObjectClass,
    /// Name of the real Solar System body whose physical data seed the preset.
    pub analogue: Option<&'static str>,
    pub description: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset { id: "earth_like", label: "Earth-like planet", class: ObjectClass::RockyPlanet, analogue: Some("Earth"), description: "Earth's mass, radius, air and oceans; a new procedural surface." },
    Preset { id: "mars_like", label: "Mars-like planet", class: ObjectClass::RockyPlanet, analogue: Some("Mars"), description: "Small, dry, thin CO₂ air." },
    Preset { id: "venus_like", label: "Venus-like planet", class: ObjectClass::RockyPlanet, analogue: Some("Venus"), description: "Earth-sized with a 92-bar CO₂ greenhouse." },
    Preset { id: "ocean_world", label: "Ocean world", class: ObjectClass::OceanWorld, analogue: Some("Earth"), description: "Earth-like, with ten times the water: no dry land." },
    Preset { id: "ice_world", label: "Ice world", class: ObjectClass::IceWorld, analogue: Some("Europa"), description: "Water-ice crust over a hidden ocean." },
    Preset { id: "moon_like", label: "Moon (Luna-like)", class: ObjectClass::Moon, analogue: Some("Moon"), description: "Airless rocky moon." },
    Preset { id: "gas_giant", label: "Gas giant (Jupiter-like)", class: ObjectClass::GasGiant, analogue: Some("Jupiter"), description: "Hydrogen–helium giant." },
    Preset { id: "ice_giant", label: "Ice giant (Neptune-like)", class: ObjectClass::IceGiant, analogue: Some("Neptune"), description: "Water–ammonia–methane giant." },
    Preset { id: "dwarf_planet", label: "Dwarf planet", class: ObjectClass::DwarfPlanet, analogue: None, description: "Pluto-sized icy world (1.3 × 10²² kg, 1 190 km)." },
    Preset { id: "asteroid_1km", label: "Asteroid, 1 km", class: ObjectClass::Asteroid, analogue: None, description: "Stony asteroid, 2 600 kg/m³. Regional devastation on impact." },
    Preset { id: "asteroid_10km", label: "Asteroid, 10 km (Chicxulub-class)", class: ObjectClass::Asteroid, analogue: None, description: "The size of the impactor that ended the dinosaurs." },
    Preset { id: "comet_5km", label: "Comet nucleus, 5 km", class: ObjectClass::Comet, analogue: None, description: "Ice and dust, 600 kg/m³. Comets strike faster than asteroids." },
    Preset { id: "sun_like_star", label: "Sun-like star", class: ObjectClass::MainSequenceStar, analogue: None, description: "1 M☉, G2V, 5772 K. A second sun: double sunsets, tangled orbits." },
    Preset { id: "red_dwarf", label: "Red dwarf", class: ObjectClass::MainSequenceStar, analogue: None, description: "0.2 M☉ M dwarf, 3200 K — the most common kind of star." },
    Preset { id: "brown_dwarf", label: "Brown dwarf", class: ObjectClass::BrownDwarf, analogue: None, description: "50 Jupiter masses: too light to fuse hydrogen; a dim, glowing failed star." },
    Preset { id: "white_dwarf", label: "White dwarf", class: ObjectClass::WhiteDwarf, analogue: None, description: "0.6 M☉ squeezed into the size of Earth: a teaspoon weighs a tonne." },
    Preset { id: "neutron_star", label: "Neutron star (pulsar)", class: ObjectClass::Pulsar, analogue: None, description: "1.4 M☉ in 12 km, spinning 30 times a second." },
    Preset { id: "black_hole_1", label: "Black hole, 1 M☉", class: ObjectClass::StellarBlackHole, analogue: None, description: "The Sun's mass in a 3 km event horizon. Tears apart anything within ~0.003 AU." },
    Preset { id: "black_hole_10", label: "Black hole, 10 M☉", class: ObjectClass::StellarBlackHole, analogue: None, description: "A typical stellar black hole from a massive star's collapse." },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

fn sphere_mass(radius: f64, density: f64) -> f64 {
    density * 4.0 / 3.0 * std::f64::consts::PI * radius * radius * radius
}

/// Build a new body from a preset. Position and orbit are set by the caller.
pub fn body_from_preset(p: &Preset, name: &str, seed: u64) -> Body {
    let sol = sol::sol_system();
    let template = p.analogue.and_then(|a| sol.find_body(a)).map(|i| sol.bodies[i].clone()).unwrap_or_else(|| {
        let i = sol.find_body("Moon").unwrap();
        sol.bodies[i].clone()
    });
    let mut b = template;
    b.name = name.to_string();
    b.real = false;
    b.parent = None;
    b.elevation_data = None;
    b.terrain_seed = seed;
    b.climate_bias = 0.0;
    b.glacial_until_years = None;
    b.glacial_cycle_years = 0.0;
    b.interglacial_fraction = 1.0;
    b.tidally_locked = false;
    b.class = Some(p.class);
    b.kind = p.class.body_kind();
    b.impacts.clear();
    b.impact_winter = None;
    b.removed = None;
    b.provenance = Default::default();
    b.provenance.source = format!("Created by user from preset “{}”{}", p.label, p.analogue.map(|a| format!(" (physical data of {a})")).unwrap_or_default());
    match p.id {
        "ocean_world" => {
            b.hydro.water_inventory = 10.0;
            b.hydro.ocean_fraction = 1.0;
        }
        "moon_like" => {
            b.rotation_period = 27.3 * SECONDS_PER_DAY;
            b.tidally_locked = true;
        }
        "dwarf_planet" => {
            b.kind = BodyKind::Icy;
            b.mass = 1.303e22;
            b.radius = 1.188e6;
            b.albedo = 0.6;
            b.rotation_period = 6.39 * SECONDS_PER_DAY;
            b.color = [0.82, 0.74, 0.66];
            b.atmosphere = Default::default();
            b.hydro.water_inventory = 0.3;
            b.hydro.ice_fraction = 1.0;
        }
        "sun_like_star" | "red_dwarf" | "brown_dwarf" | "white_dwarf" | "neutron_star" | "black_hole_1" | "black_hole_10" => {
            use crate::astro::{EARTH_RADIUS, SOLAR_RADIUS};
            let (mass_sun, radius, temp, rot) = match p.id {
                "sun_like_star" => (1.0, SOLAR_RADIUS, 5772.0, 25.4 * SECONDS_PER_DAY),
                "red_dwarf" => (0.2, 0.23 * SOLAR_RADIUS, 3200.0, 3.0 * SECONDS_PER_DAY),
                "brown_dwarf" => (0.048, 0.09 * SOLAR_RADIUS, 1300.0, 0.3 * SECONDS_PER_DAY),
                "white_dwarf" => (0.6, 0.0125 * SOLAR_RADIUS, 15_000.0, 3600.0),
                "neutron_star" => (1.4, 12_000.0, 600_000.0, 0.033),
                "black_hole_1" => (1.0, crate::astro::star::schwarzschild_radius(1.0), 0.0, 1.0),
                _ => (10.0, crate::astro::star::schwarzschild_radius(10.0), 0.0, 1.0),
            };
            let _ = EARTH_RADIUS;
            b.kind = p.class.body_kind();
            b.mass = mass_sun * SOLAR_MASS;
            b.radius = radius;
            b.temperature = temp;
            b.equilibrium_temperature = temp;
            b.rotation_period = rot;
            b.axial_tilt = 0.1;
            b.albedo = 0.0;
            b.atmosphere = Default::default();
            b.hydro = Default::default();
            b.magnetic_field = if p.id == "neutron_star" { 2.0e12 } else { 0.0 };
            // Presets show black holes feeding (as if from a companion's gas stream).
            b.accretion = if b.kind == BodyKind::BlackHole { 0.3 } else { 0.0 };
            b.geology = 0.0;
            b.rings = None;
            b.deposits.clear();
            b.resources = Default::default();
            let (r, g, bl) = cosmogon_core::units::kelvin_to_rgb(temp.clamp(1000.0, 40_000.0));
            b.color = if b.kind == BodyKind::BlackHole { [0.0, 0.0, 0.0] } else { [r, g, bl] };
        }
        "asteroid_1km" | "asteroid_10km" | "comet_5km" => {
            let (r, rho) = match p.id {
                "asteroid_1km" => (500.0, 2600.0),
                "asteroid_10km" => (5000.0, 2600.0),
                _ => (2500.0, 600.0),
            };
            b.mass = sphere_mass(r, rho);
            b.radius = r;
            b.rotation_period = 5.0 * 3600.0;
            b.axial_tilt = 0.4;
            b.atmosphere = Default::default();
            b.magnetic_field = 0.0;
            b.geology = 0.0;
            if p.class == ObjectClass::Comet {
                b.kind = BodyKind::Icy;
                b.albedo = 0.04;
                b.color = [0.30, 0.29, 0.28];
                b.hydro = crate::astro::Hydrosphere { water_inventory: 1e-9, ocean_fraction: 0.0, ice_fraction: 1.0, subsurface_ocean: false };
            } else {
                b.albedo = 0.14;
                b.color = [0.45, 0.42, 0.38];
                b.hydro = Default::default();
            }
            b.rings = None;
            b.deposits.clear();
        }
        _ => {}
    }
    b.sea_level = terrain::Terrain::of(&b).sea_level_for(b.hydro.ocean_fraction);
    for f in ["mass", "radius", "atmosphere", "water", "orbit", "position", "velocity"] {
        b.mark(f, Quality::UserModified);
    }
    b
}

/// State for a circular orbit of a body of `mass` around `parent` (or the star) at
/// `distance`, at orbital phase `phase` (rad from the reference direction) and inclination
/// `inclination` (rad), in the system frame at time `t`.
pub fn circular_state(sys: &StarSystem, parent: Option<usize>, mass: f64, distance: f64, phase: f64, inclination: f64, t: f64) -> State {
    let (center, mu) = match parent {
        Some(p) => (sys.body_state(p, t), sys.bodies[p].mu() + G * mass),
        None => (State { pos: sys.star_local_position(t), vel: sys.dynamics.as_ref().map(|d| d.star.vel).unwrap_or_default() }, sys.star.mu() + G * mass),
    };
    let (sp, cp) = phase.dsin_cos();
    let (si, ci) = inclination.dsin_cos();
    let r = Vec3d::new(cp, sp * ci, sp * si) * distance;
    let v = Vec3d::new(-sp, cp * ci, cp * si) * (mu / distance).sqrt();
    State { pos: center.pos + r, vel: center.vel + v }
}

/// A projectile state aimed at `target`: placed `distance` away along `direction` (unit,
/// system frame), approaching at `speed` relative to the target with `miss` metres of
/// impact parameter (0 = dead centre). Gravity will bend it inward.
pub fn aimed_state(sys: &StarSystem, target: usize, distance: f64, direction: Vec3d, speed: f64, miss: f64, t: f64) -> State {
    let tgt = sys.body_state(target, t);
    let dir = direction.normalize();
    // Any vector perpendicular to the approach line, for the impact parameter.
    let side = if dir.z.abs() < 0.9 { dir.cross(Vec3d::new(0.0, 0.0, 1.0)) } else { dir.cross(Vec3d::new(1.0, 0.0, 0.0)) }.normalize();
    // In the target's frame the projectile flies straight in (both share the target's
    // heliocentric velocity, and both feel nearly the same solar pull).
    let pos = tgt.pos + dir * distance + side * miss;
    let vel = tgt.vel - dir * speed;
    State { pos, vel }
}

// ── Warnings ────────────────────────────────────────────────────────────────

/// Physically unusual configurations (warnings, never refusals).
pub fn warnings(u: &Universe, r: BodyRef) -> Vec<String> {
    let sys = u.system(r.system);
    let b = sys.bodies.get(r.body as usize);
    let Some(b) = b.filter(|b| b.exists()) else { return Vec::new() };
    let mut w = Vec::new();
    let rho = b.density();
    if rho > 25_000.0 && !b.kind.is_stellar() {
        w.push(format!("Density {rho:.0} kg/m³ exceeds any ordinary planetary matter (iron ≈ 7 900; Earth's core ≈ 13 000)."));
    } else if rho < 300.0 && b.kind.has_surface() {
        w.push(format!("Density {rho:.0} kg/m³ is lower than any known solid body."));
    }
    if sys.is_dynamic() {
        let el = sys.osculating(r.body as usize, u.time);
        if !el.is_bound() {
            w.push("Orbit is unbound: it will escape its parent.".into());
        }
        let parent_r = sys.parent_state(r.body as usize, u.time);
        let dist = (sys.body_state(r.body as usize, u.time).pos - parent_r.pos).length();
        let roche = match b.parent {
            Some(p) => roche_limit(sys.bodies[p as usize].radius, sys.bodies[p as usize].density(), rho),
            None => roche_limit(sys.star.current_radius(u.time), sys.star.mass * SOLAR_MASS / (4.0 / 3.0 * std::f64::consts::PI * sys.star.current_radius(u.time).powi(3)), rho),
        };
        if dist < roche && b.mass > 1e15 && !b.kind.is_compact() {
            w.push(format!("Inside the Roche limit of its parent ({:.0} km): tides would pull it apart (tidal disruption is not simulated yet).", roche / 1000.0));
        }
        if let Some(d) = &sys.dynamics {
            if d.diagnostics.saturated_steps > 0 {
                w.push("Time step insufficient for a recent close encounter: accuracy reduced. Try the Accurate preset.".into());
            }
        }
    }
    w
}

/// Fluid-body Roche limit, `2.44 R_p (ρ_p/ρ_s)^⅓` (Roche 1849; e.g. Murray & Dermott 1999).
pub fn roche_limit(primary_radius: f64, primary_density: f64, satellite_density: f64) -> f64 {
    2.44 * primary_radius * (primary_density / satellite_density.max(1.0)).dcbrt()
}

/// Hill-sphere radius `a (m / 3M)^⅓`.
pub fn hill_radius(a: f64, m: f64, primary_mass: f64) -> f64 {
    a * (m / (3.0 * primary_mass)).dcbrt()
}

// ── Applying edits ──────────────────────────────────────────────────────────

fn finite_positive(x: f64, what: &str) -> Result<(), String> {
    if x.is_finite() && x > 0.0 {
        Ok(())
    } else {
        Err(format!("{what} must be a positive number"))
    }
}

fn validate(u: &Universe, e: &Edit) -> Result<(), String> {
    let sys = u.systems.get(e.system() as usize).ok_or("no such star system")?;
    let body_ok = |r: &BodyRef| -> Result<(), String> {
        match sys.bodies.get(r.body as usize) {
            Some(b) if b.exists() => Ok(()),
            _ => Err("that object no longer exists".into()),
        }
    };
    let vec_ok = |v: Vec3d| if v.x.is_finite() && v.y.is_finite() && v.z.is_finite() { Ok(()) } else { Err("values must be finite".to_string()) };
    match e {
        Edit::AddBody { body, state, .. } => {
            finite_positive(body.mass, "Mass")?;
            finite_positive(body.radius, "Radius")?;
            vec_ok(state.pos)?;
            vec_ok(state.vel)?;
        }
        Edit::RemoveBody { body } => body_ok(body)?,
        Edit::SetState { body, state } => {
            body_ok(body)?;
            vec_ok(state.pos)?;
            vec_ok(state.vel)?;
        }
        Edit::Impulse { body, dv } => {
            body_ok(body)?;
            vec_ok(*dv)?;
        }
        Edit::SetOrbit { body, a, e, .. } => {
            body_ok(body)?;
            finite_positive(*a, "Semi-major axis")?;
            if !(0.0..1.0).contains(e) {
                return Err("Eccentricity must be in [0, 1) for an orbit; use Set state for unbound paths".into());
            }
        }
        Edit::SetProperty { body, property } => {
            body_ok(body)?;
            match property {
                BodyProperty::Mass(m) => finite_positive(*m, "Mass")?,
                BodyProperty::Radius(r) => finite_positive(*r, "Radius")?,
                BodyProperty::RotationPeriod(p) if !p.is_finite() || *p == 0.0 => return Err("Rotation period must be non-zero".into()),
                BodyProperty::Albedo(a) if !(0.0..=1.0).contains(a) => return Err("Albedo must be between 0 and 1".into()),
                BodyProperty::SurfacePressure(p) | BodyProperty::WaterInventory(p) if !p.is_finite() || *p < 0.0 => return Err("Value must be zero or positive".into()),
                BodyProperty::Co2Fraction(f) if !(0.0..=1.0).contains(f) => return Err("CO₂ fraction must be between 0 and 1".into()),
                BodyProperty::Name(n) if n.trim().is_empty() => return Err("Name cannot be empty".into()),
                _ => {}
            }
        }
        Edit::SetStar { property, .. } => match property {
            StarProperty::Mass(m) => {
                finite_positive(*m, "Mass")?;
                let compact = u.system(e.system()).star.kind != crate::astro::star::StarKind::Normal;
                if !compact && !(0.08..=150.0).contains(m) {
                    return Err("Living stars span 0.08–150 solar masses (lighter objects are brown dwarfs or planets; make it a black hole for anything heavier)".into());
                }
            }
            StarProperty::Age(a) => finite_positive(*a, "Age")?,
            StarProperty::Metallicity(z) if !z.is_finite() => return Err("Metallicity must be finite".into()),
            StarProperty::Name(n) if n.trim().is_empty() => return Err("Name cannot be empty".into()),
            _ => {}
        },
        Edit::SetPhysics { settings, .. } => finite_positive(settings.steps_per_orbit, "Steps per orbit")?,
        Edit::Intervene { civ, action, .. } => crate::intervene::validate(u, *civ, action)?,
    }
    Ok(())
}

fn describe(u: &Universe, e: &Edit) -> String {
    let name = |r: &BodyRef| u.body(*r).name.clone();
    match e {
        Edit::AddBody { body, .. } => format!("Added {} ({})", body.name, body.class().label().to_lowercase()),
        Edit::RemoveBody { body } => format!("Deleted {}", name(body)),
        Edit::SetState { body, .. } => format!("Moved {}", name(body)),
        Edit::Impulse { body, dv } => format!("Gave {} a push of {:.2} km/s", name(body), dv.length() / 1000.0),
        Edit::SetOrbit { body, a, e, .. } => format!("Put {} on a new orbit (a = {:.4} AU, e = {:.3})", name(body), a / AU, e),
        Edit::SetProperty { body, property } => match property {
            BodyProperty::Name(n) => format!("Renamed {} to {n}", name(body)),
            BodyProperty::Mass(m) => format!("Set mass of {} to {:.4} Earth masses", name(body), m / EARTH_MASS),
            BodyProperty::Radius(r) => format!("Set radius of {} to {:.0} km", name(body), r / 1000.0),
            BodyProperty::RotationPeriod(p) => format!("Set day length of {} to {:.2} h", name(body), p / 3600.0),
            BodyProperty::AxialTilt(a) => format!("Set axial tilt of {} to {:.1}°", name(body), a.to_degrees()),
            BodyProperty::Albedo(a) => format!("Set albedo of {} to {a:.2}", name(body)),
            BodyProperty::SurfacePressure(p) => format!("Set surface pressure of {} to {p:.3} bar", name(body)),
            BodyProperty::Co2Fraction(f) => format!("Set CO₂ of {} to {:.4}%", name(body), f * 100.0),
            BodyProperty::WaterInventory(w) => format!("Set water of {} to {w:.3} Earth oceans", name(body)),
        },
        Edit::SetStar { system, property } => {
            let s = &u.system(*system).star.name;
            match property {
                StarProperty::Mass(m) => format!("Set mass of {s} to {m:.3} solar masses"),
                StarProperty::Metallicity(z) => format!("Set metallicity of {s} to {z:+.2} dex"),
                StarProperty::Age(a) => format!("Set age of {s} to {:.2} Gyr", a / (1e9 * SECONDS_PER_YEAR)),
                StarProperty::Name(n) => format!("Renamed {s} to {n}"),
                StarProperty::Kind(k) => format!("Turned {s} into a {}", k.label().to_lowercase()),
            }
        }
        Edit::SetPhysics { nbody, settings, .. } => {
            if *nbody {
                format!("Dynamic gravity (N-body, {})", settings.preset.label())
            } else {
                "Fixed orbits (analytic Kepler)".into()
            }
        }
        Edit::Intervene { civ, action, .. } => format!("{} — {}", action.label(), u.civs.get(*civ as usize).map(|c| c.name.as_str()).unwrap_or("?")),
    }
}

impl Universe {
    /// Apply a user edit. See the module documentation.
    pub fn apply_edit(&mut self, edit: Edit) -> Result<EditOutcome, String> {
        validate(self, &edit)?;
        let t = self.time;
        // Interventions touch only the civilization: no gravity changes, no physics sync.
        if let Edit::Intervene { civ, action, .. } = &edit {
            let summary = self.apply_intervention(*civ, action);
            self.edits.push(EditRecord { time: t, summary: summary.clone(), edit });
            return Ok(EditOutcome { created: None, summary });
        }
        let s = edit.system() as usize;
        let summary = describe(self, &edit);
        let wants_kepler = matches!(edit, Edit::SetPhysics { nbody: false, .. });
        if !wants_kepler && !self.systems[s].is_dynamic() {
            let settings = match &edit {
                Edit::SetPhysics { settings, .. } => *settings,
                _ => self.settings.physics.unwrap_or_default(),
            };
            self.systems[s].activate_dynamics(t, settings);
        }
        self.systems[s].sync_to(t);
        self.apply_pending_contacts(s);
        let mut outcome = EditOutcome { created: None, summary: summary.clone() };
        let mut affected: Vec<usize> = Vec::new();

        match edit.clone() {
            Edit::AddBody { body, state, .. } => {
                let sys = &mut self.systems[s];
                let idx = sys.bodies.len();
                let mut body = *body;
                body.id = idx as u32;
                // Name the parent by proximity if the body is placed inside a planet's Hill sphere.
                let star_mass = sys.star.mass * SOLAR_MASS;
                if body.parent.is_none() {
                    for p in sys.planets().collect::<Vec<_>>() {
                        let ps = sys.body_state(p, t);
                        let a = (ps.pos - sys.star_local_position(t)).length();
                        if (state.pos - ps.pos).length() < 0.5 * hill_radius(a, sys.bodies[p].mass, star_mass) {
                            body.parent = Some(p as u32);
                            break;
                        }
                    }
                }
                sys.bodies.push(body);
                let d = sys.dynamics.as_mut().unwrap();
                d.bodies.push(Some(state));
                // Keep an analytic orbit on record (used if the system is frozen to Kepler).
                let p = sys.parent_state(idx, t);
                let mu = sys.parent_mu(idx) + sys.bodies[idx].mu();
                if let Some(o) = Orbit::from_state(state.pos - p.pos, state.vel - p.vel, mu, t) {
                    sys.bodies[idx].orbit = o;
                }
                if sys.bodies[idx].tidally_locked {
                    sys.bodies[idx].rotation_period = sys.bodies[idx].orbit.period(sys.parent_mu(idx).max(1.0));
                }
                if sys.bodies[idx].kind.has_surface() {
                    let hab = assess(sys, idx, t);
                    self.biospheres.push(crate::life::Biosphere::new(s as u32, idx as u32, hab, t));
                }
                affected.push(idx);
                outcome.created = Some(BodyRef { system: s as u32, body: idx as u32 });
            }
            Edit::RemoveBody { body } => {
                let j = body.body as usize;
                let sys = &mut self.systems[s];
                // Moons are released onto their own paths around the removed body's parent.
                let moons: Vec<usize> = sys.moons_of(j).collect();
                let grand = sys.bodies[j].parent;
                for m in moons {
                    sys.promote(m, t);
                    sys.bodies[m].parent = grand;
                }
                sys.bodies[j].removed = Some(Removal { time: t, cause: RemovalCause::Deleted });
                if let Some(d) = sys.dynamics.as_mut() {
                    d.bodies[j] = None;
                }
                self.on_body_destroyed(s, j, t, "was deleted from the sandbox");
            }
            Edit::SetState { body, state } => {
                let j = body.body as usize;
                self.systems[s].promote(j, t);
                self.systems[s].dynamics.as_mut().unwrap().bodies[j] = Some(state);
                self.systems[s].bodies[j].mark("position", Quality::UserModified);
                self.systems[s].bodies[j].mark("velocity", Quality::UserModified);
                affected.push(j);
            }
            Edit::Impulse { body, dv } => {
                let j = body.body as usize;
                self.systems[s].promote(j, t);
                if let Some(Some(st)) = self.systems[s].dynamics.as_mut().map(|d| &mut d.bodies[j]) {
                    st.vel += dv;
                }
                self.systems[s].bodies[j].mark("velocity", Quality::UserModified);
                affected.push(j);
            }
            Edit::SetOrbit { body, a, e, i, node, peri, mean_anomaly } => {
                let j = body.body as usize;
                let sys = &mut self.systems[s];
                sys.promote(j, t);
                let p = sys.parent_state(j, t);
                let mu = sys.parent_mu(j) + sys.bodies[j].mu();
                let (rp, rv) = cosmogon_physics::kepler::orbital_state_vectors(a, e, i, node, peri, mean_anomaly, mu);
                sys.dynamics.as_mut().unwrap().bodies[j] = Some(State { pos: p.pos + rp, vel: p.vel + rv });
                if let Some(o) = Orbit::from_state(rp, rv, mu, t) {
                    sys.bodies[j].orbit = o;
                }
                sys.bodies[j].mark("orbit", Quality::UserModified);
                affected.push(j);
            }
            Edit::SetProperty { body, property } => {
                let j = body.body as usize;
                // Changing a rails moon's mass or radius makes it a full particle.
                if matches!(property, BodyProperty::Mass(_) | BodyProperty::Radius(_)) {
                    self.systems[s].promote(j, t);
                }
                let b = &mut self.systems[s].bodies[j];
                b.mark(property.field(), Quality::UserModified);
                match property {
                    BodyProperty::Name(n) => b.name = n,
                    BodyProperty::Mass(m) => b.mass = m,
                    BodyProperty::Radius(r) => b.radius = r,
                    BodyProperty::RotationPeriod(p) => {
                        b.rotation_period = p;
                        b.tidally_locked = false;
                    }
                    BodyProperty::AxialTilt(a) => b.axial_tilt = a,
                    BodyProperty::Albedo(a) => {
                        b.albedo = a;
                        // Measured-body calibration no longer applies to an altered world.
                        b.real = false;
                    }
                    BodyProperty::SurfacePressure(p) => {
                        b.atmosphere.pressure_bar = p;
                        if p > 0.0 && b.atmosphere.n2 + b.atmosphere.o2 + b.atmosphere.co2 + b.atmosphere.h2o + b.atmosphere.ch4 + b.atmosphere.h2he == 0.0 {
                            b.atmosphere.n2 = 1.0;
                        }
                    }
                    BodyProperty::Co2Fraction(f) => {
                        let rest = 1.0 - b.atmosphere.co2;
                        let scale = if rest > 0.0 { (1.0 - f) / rest } else { 0.0 };
                        let a = &mut b.atmosphere;
                        a.n2 *= scale;
                        a.o2 *= scale;
                        a.h2o *= scale;
                        a.ch4 *= scale;
                        a.h2he *= scale;
                        a.co2 = f;
                        if scale == 0.0 && f < 1.0 {
                            a.n2 = 1.0 - f;
                        }
                    }
                    BodyProperty::WaterInventory(w) => {
                        b.hydro.water_inventory = w;
                        b.sea_level = terrain::Terrain::of(b).sea_level_for(b.hydro.ocean_fraction);
                    }
                }
                affected.push(j);
            }
            Edit::SetStar { system, property } => {
                let sys = &mut self.systems[system as usize];
                let st = &sys.star;
                let (mut mass, mut z, mut formed, mut name, mut kind) = (st.mass, st.metallicity, st.formed_at, st.name.clone(), st.kind);
                match property {
                    StarProperty::Mass(m) => mass = m,
                    StarProperty::Metallicity(v) => z = v,
                    StarProperty::Age(a) => formed = t - a,
                    StarProperty::Name(n) => name = n,
                    StarProperty::Kind(k) => {
                        kind = k;
                        formed = t;
                        if k == crate::astro::star::StarKind::Normal {
                            mass = mass.clamp(0.08, 150.0);
                            formed = t - 4.6e9 * SECONDS_PER_YEAR * mass.powf(-2.5).min(1.0) * 0.46;
                        }
                    }
                }
                // Physically consistent mode: radius, luminosity, temperature and lifetime
                // follow from the stellar model.
                sys.star = if kind == crate::astro::star::StarKind::Normal { Star::from_mass(name, mass, z, formed) } else { Star::compact(name, kind, mass, formed) };
                affected.extend(sys.existing());
            }
            Edit::SetPhysics { system, nbody, settings } => {
                let sys = &mut self.systems[system as usize];
                if nbody {
                    if let Some(d) = sys.dynamics.as_mut() {
                        d.settings = settings;
                    }
                } else {
                    sys.deactivate_dynamics(t)?;
                }
            }
            Edit::Intervene { .. } => unreachable!("handled above"),
        }

        let sys = &mut self.systems[s];
        if sys.is_dynamic() {
            sys.retune(t);
        }
        self.ensure_environment_task();
        self.ensure_star_task();
        for j in affected {
            self.refresh_body_environment(s, j, t);
        }
        self.edits.push(EditRecord { time: t, summary: summary.clone(), edit });
        self.history.push(Event { time: t, category: Category::Astronomy, importance: 4, title: summary, detail: "Sandbox edit.".into(), system: Some(s as u32), body: outcome.created.map(|c| c.body), civ: None });
        Ok(outcome)
    }

    /// Recompute climate, resources and habitability of one body now.
    pub(crate) fn refresh_system_climates(&mut self, s: usize, t: f64) {
        for j in 0..self.systems[s].bodies.len() {
            if self.systems[s].bodies[j].exists() {
                self.refresh_body_environment(s, j, t);
            }
        }
    }

    pub(crate) fn refresh_body_environment(&mut self, s: usize, j: usize, t: f64) {
        if !self.systems[s].bodies[j].exists() {
            return;
        }
        refresh_climate(&mut self.systems[s], j, t);
        let r = BodyRef { system: s as u32, body: j as u32 };
        let vegetated = self.biosphere(r).is_some_and(|b| b.vegetated());
        refresh_derived_resources(&mut self.systems[s].bodies[j], vegetated);
        let hab = assess(&self.systems[s], j, t);
        if let Some(bio) = self.biospheres.iter_mut().find(|b| b.system == s as u32 && b.body == j as u32) {
            bio.habitability = hab;
        }
        if self.systems[s].is_dynamic() {
            let f = flux_of(&self.systems[s], j, t);
            let n = self.systems[s].bodies.len();
            let d = self.systems[s].dynamics.as_mut().unwrap();
            d.flux_climate.resize(n, 0.0);
            d.flux_climate[j] = f;
        }
    }

    /// Apply the consequences of collisions found while syncing a system for an edit.
    pub(crate) fn apply_pending_contacts(&mut self, s: usize) {
        let pending = self.systems[s].take_pending_contacts();
        for c in pending {
            self.on_contact(s, c);
        }
    }

    // ── Consequences ────────────────────────────────────────────────────────

    /// A collision happened (already merged in the physics state).
    pub(crate) fn on_contact(&mut self, s: usize, ev: ContactEvent) {
        let t = ev.contact.time;
        let Slot::Body(j) = ev.absorbed else {
            // The star itself was swallowed by something heavier (see Dynamics::apply_merges).
            if let Slot::Body(i) = ev.survivor {
                let who = self.systems[s].bodies[i as usize].name.clone();
                self.history.push(Event { time: t, category: Category::Astronomy, importance: 5, title: format!("{who} swallows the star"), detail: format!("Now {:.2} M☉, it becomes the centre of the system; the planets orbit it instead.", self.systems[s].star.mass), system: Some(s as u32), body: None, civ: None });
                self.refresh_system_climates(s, t);
            }
            return;
        };
        let j = j as usize;
        let imp_name = self.systems[s].bodies[j].name.clone();
        let c = ev.contact;
        match ev.survivor {
            Slot::Star | Slot::Companion => {
                let star = if ev.survivor == Slot::Star { self.systems[s].star.name.clone() } else { self.systems[s].companion.as_ref().map(|c| c.star.name.clone()).unwrap_or_default() };
                self.history.push(Event {
                    time: t,
                    category: Category::Astronomy,
                    importance: 5,
                    title: format!("{imp_name} falls into {star}"),
                    detail: format!("It strikes the star at {:.0} km/s and is vaporised.", c.rel_vel.length() / 1000.0),
                    system: Some(s as u32),
                    body: Some(j as u32),
                    civ: None,
                });
                self.on_body_destroyed(s, j, t, &format!("fell into {star}"));
            }
            Slot::Body(i) if self.systems[s].bodies[i as usize].kind.is_stellar() => {
                let i = i as usize;
                let who = self.systems[s].bodies[i].name.clone();
                let kind = self.systems[s].bodies[i].kind;
                let detail = match kind {
                    BodyKind::BlackHole => "Stretched by tides far stronger than its own gravity, it is torn into a stream of gas (\"spaghettified\"); part spirals in, flaring in X-rays.",
                    BodyKind::Star => "It plunges into the star and is vaporised.",
                    _ => "Tidal forces shred it; the debris falls onto the dense remnant.",
                };
                self.history.push(Event { time: t, category: Category::Astronomy, importance: 5, title: format!("{imp_name} torn apart by {who}"), detail: detail.into(), system: Some(s as u32), body: Some(j as u32), civ: None });
                self.on_body_destroyed(s, j, t, &format!("was torn apart by {who}"));
            }
            Slot::Body(i) => {
                let i = i as usize;
                let record = self.impact_record(s, i, j, &c);
                self.apply_impact(s, i, record);
                self.on_body_destroyed(s, j, t, &format!("collided with {}", self.systems[s].bodies[i].name));
            }
        }
    }

    fn impact_record(&self, s: usize, i: usize, j: usize, c: &cosmogon_physics::nbody::Contact) -> ImpactRecord {
        let sys = &self.systems[s];
        let (target, imp) = (&sys.bodies[i], &sys.bodies[j]);
        let m_t = c.gm_a / G;
        let m_i = c.gm_b / G;
        let rho = |m: f64, r: f64| m / (4.0 / 3.0 * std::f64::consts::PI * r * r * r);
        let speed = c.rel_vel.length();
        // Reduced mass: correct for comparable bodies, ≈ impactor mass for small ones.
        let mu = m_t * m_i / (m_t + m_i);
        let energy = 0.5 * mu * speed * speed;
        let sin_angle = (c.rel_pos.dot(c.rel_vel).abs() / (c.rel_pos.length() * speed).max(1e-30)).clamp(0.0, 1.0);
        let angle = sin_angle.dasin();
        let g = c.gm_a / (c.radius_a * c.radius_a);
        let surface = target.kind.has_surface();
        let class = impact::classify(energy, m_i / m_t, surface);
        let crater = if surface && class != ImpactClass::GiantImpact { impact::crater_diameter(c.radius_b, rho(m_i, c.radius_b), rho(m_t, c.radius_a), speed, angle, g) } else { 0.0 };
        let dir = c.rel_pos.normalize();
        let local = sys.inertial_to_body_fixed(i, dir, c.time);
        let (lat, lon) = terrain::lat_lon_from_dir(local);
        let ocean = surface && target.hydro.ocean_fraction > 0.0 && terrain::Terrain::of(target).elevation(local) < target.sea_level;
        let mt = energy / MEGATON_TNT_J;
        let (extinction, cooling) = match class {
            ImpactClass::GiantImpact | ImpactClass::Sterilising => (1.0, 0.0),
            ImpactClass::Atmospheric => (0.0, 0.0),
            _ => (impact::extinction_severity(mt), impact::winter_peak_cooling(mt)),
        };
        ImpactRecord {
            time: c.time,
            impactor: imp.name.clone(),
            impactor_mass: m_i,
            impactor_radius: c.radius_b,
            speed,
            angle_deg: angle.to_degrees(),
            energy_j: energy,
            lat,
            lon,
            ocean,
            crater_m: crater,
            blast_radius_m: if surface { impact::blast_radius(energy) } else { 0.0 },
            class,
            extinction,
            cooling_k: cooling,
            casualties: 0.0,
        }
    }

    /// Carry an impact down the causal chain: environment, biosphere, civilization.
    fn apply_impact(&mut self, s: usize, i: usize, mut rec: ImpactRecord) {
        let t = rec.time;
        let target = self.systems[s].bodies[i].name.clone();
        let r = BodyRef { system: s as u32, body: i as u32 };

        // Civilizations: settlements inside the crater are lost, half the people inside the
        // severe-blast radius die. Impact winter acts later through climate → capacity.
        let radius = self.systems[s].bodies[i].radius;
        let impact_dir = terrain::dir_from_lat_lon(rec.lat, rec.lon);
        let mut casualties = 0.0;
        for c in self.civs.iter_mut().filter(|c| c.system == s as u32 && c.body == i as u32 && c.is_alive()) {
            let total: f64 = c.sites.iter().map(|x| x.population).sum::<f64>().max(1.0);
            let mut lost_share = 0.0;
            for site in c.sites.iter_mut() {
                let d = site.dir();
                let cosang = (d[0] * impact_dir[0] + d[1] * impact_dir[1] + d[2] * impact_dir[2]).clamp(-1.0, 1.0);
                let dist = cosang.dacos() * radius;
                let f = if dist < rec.crater_m * 0.5 { 1.0 } else if dist < rec.blast_radius_m { 0.5 } else { 0.0 };
                if f > 0.0 {
                    lost_share += site.population * f / total;
                    site.population *= 1.0 - f;
                }
            }
            let dead = c.population * lost_share.min(1.0);
            c.population -= dead;
            casualties += dead;
        }
        rec.casualties = casualties;

        let mut extinction_event = None;
        match rec.class {
            ImpactClass::GiantImpact | ImpactClass::Sterilising => {
                self.sterilise(s, i, t, if rec.class == ImpactClass::GiantImpact { "a giant impact melted the surface" } else { "the impact boiled the oceans" });
            }
            ImpactClass::Atmospheric | ImpactClass::Local | ImpactClass::Regional => {}
            ImpactClass::Global | ImpactClass::MassExtinction => {
                let b = &mut self.systems[s].bodies[i];
                let winter = ImpactWinter { start: t, peak_k: rec.cooling_k, efold_years: ImpactWinter::EFOLD_YEARS };
                b.impact_winter = Some(match b.impact_winter {
                    Some(w) if w.cooling_at(t) > winter.peak_k => w,
                    _ => winter,
                });
                if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == s as u32 && x.body == i as u32) {
                    if bio.stage >= Stage::Microbial && rec.extinction > 0.0 {
                        bio.biodiversity *= 1.0 - rec.extinction;
                        bio.biomass *= 1.0 - rec.extinction * 0.5;
                        bio.extinctions += 1;
                        extinction_event = Some(rec.extinction);
                    }
                }
            }
        }

        let mt = rec.energy_mt();
        let energy = if mt >= 1.0 { format!("{} Mt TNT", crate::time::group_digits(mt)) } else { format!("{:.1} kt TNT", mt * 1000.0) };
        let place = if rec.class == ImpactClass::Atmospheric {
            "the atmosphere".to_string()
        } else {
            format!("{} at {:.1}°{}, {:.1}°{}", if rec.ocean { "ocean" } else { "land" }, rec.lat.to_degrees().abs(), if rec.lat >= 0.0 { "N" } else { "S" }, rec.lon.to_degrees().abs(), if rec.lon >= 0.0 { "E" } else { "W" })
        };
        let mut detail = format!(
            "{} ({:.1} km across) hits {place} at {:.1} km/s, {:.0}° from horizontal: {energy} ({:.2e} J), a {} impact.",
            rec.impactor,
            2.0 * rec.impactor_radius / 1000.0,
            rec.speed / 1000.0,
            rec.angle_deg,
            rec.energy_j,
            rec.class.label()
        );
        if rec.crater_m > 0.0 {
            detail.push_str(&format!(" Crater ≈ {:.1} km; severe blast damage to {:.0} km.", rec.crater_m / 1000.0, rec.blast_radius_m / 1000.0));
        }
        if rec.cooling_k > 0.05 {
            detail.push_str(&format!(" Dust and aerosols cool the surface by up to {:.1} K for several years.", rec.cooling_k));
        }
        if casualties >= 1.0 {
            detail.push_str(&format!(" {} people killed.", compact(casualties)));
        }
        self.history.push(Event {
            time: t,
            category: Category::Disaster,
            importance: if matches!(rec.class, ImpactClass::Local) && casualties < 1.0 { 4 } else { 5 },
            title: format!("Impact on {target}"),
            detail,
            system: Some(s as u32),
            body: Some(i as u32),
            civ: None,
        });
        if let Some(sev) = extinction_event {
            self.history.push(Event { time: t, category: Category::Life, importance: 5, title: format!("Mass extinction on {target}"), detail: format!("The impact winter wipes out {:.0}% of species.", sev * 100.0), system: Some(s as u32), body: Some(i as u32), civ: None });
        }
        self.systems[s].bodies[i].impacts.push(rec);
        let _ = r;
        self.refresh_body_environment(s, i, t);
    }

    fn sterilise(&mut self, s: usize, i: usize, t: f64, why: &str) {
        let name = self.systems[s].bodies[i].name.clone();
        if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == s as u32 && x.body == i as u32) {
            if bio.stage > Stage::Sterile {
                bio.stage = Stage::Sterile;
                bio.stage_since = t;
                bio.biomass = 0.0;
                bio.biodiversity = 0.0;
                bio.photosynthesis_since = None;
                bio.civilization_present = false;
                self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("{name} sterilised"), detail: format!("All life is lost: {why}."), system: Some(s as u32), body: Some(i as u32), civ: None });
            }
        }
        self.end_civilizations(s, i, t, why);
    }

    fn end_civilizations(&mut self, s: usize, i: usize, t: f64, why: &str) {
        for ci in 0..self.civs.len() {
            let c = &mut self.civs[ci];
            if c.system == s as u32 && c.body == i as u32 && c.is_alive() {
                c.status = CivStatus::Extinct { at: t };
                c.population = 0.0;
                let name = c.name.clone();
                self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("The end of {name}"), detail: format!("Their world is gone: {why}."), system: Some(s as u32), body: Some(i as u32), civ: Some(ci as u32) });
            }
        }
    }

    /// A body ceased to exist (deleted, merged, fell into a star).
    pub(crate) fn on_body_destroyed(&mut self, s: usize, j: usize, t: f64, how: &str) {
        let name = self.systems[s].bodies[j].name.clone();
        if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == s as u32 && x.body == j as u32) {
            bio.stage = Stage::Sterile;
            bio.biomass = 0.0;
            bio.biodiversity = 0.0;
            bio.habitability.score = 0.0;
            bio.civilization_present = false;
        }
        self.end_civilizations(s, j, t, &format!("{name} {how}"));
    }

    // ── Orbit → insolation → climate ────────────────────────────────────────

    /// Environment task for dynamic systems: when a world's annual-mean insolation changes,
    /// its climate, resources and habitability are recomputed; large changes are reported.
    pub(crate) fn step_environment(&mut self, t: f64) {
        for s in 0..self.systems.len() {
            if !self.systems[s].is_dynamic() {
                continue;
            }
            let n = self.systems[s].bodies.len();
            {
                let d = self.systems[s].dynamics.as_mut().unwrap();
                d.flux_climate.resize(n, 0.0);
                d.flux_reported.resize(n, 0.0);
            }
            for j in 0..n {
                let b = &self.systems[s].bodies[j];
                if !b.exists() {
                    continue;
                }
                // Expire impact winters.
                if b.impact_winter.is_some_and(|w| w.expired(t)) {
                    self.systems[s].bodies[j].impact_winter = None;
                    self.refresh_body_environment(s, j, t);
                }
                let f = flux_of(&self.systems[s], j, t);
                let d = self.systems[s].dynamics.as_ref().unwrap();
                let (fc, fr) = (d.flux_climate[j], d.flux_reported[j]);
                let winter = self.systems[s].bodies[j].impact_winter.is_some();
                if fc <= 0.0 || (f / fc - 1.0).abs() > 0.002 || winter {
                    self.refresh_body_environment(s, j, t);
                    self.systems[s].dynamics.as_mut().unwrap().flux_climate[j] = f;
                }
                let surface = self.systems[s].bodies[j].kind.has_surface();
                let temp = self.systems[s].bodies[j].temperature;
                let d = self.systems[s].dynamics.as_mut().unwrap();
                d.temp_reported.resize(n, 0.0);
                if fr <= 0.0 {
                    d.flux_reported[j] = f;
                    d.temp_reported[j] = temp;
                } else if (f / fr - 1.0).abs() > 0.02 && surface {
                    let before = d.temp_reported[j];
                    d.flux_reported[j] = f;
                    d.temp_reported[j] = temp;
                    let sys = &self.systems[s];
                    let el = sys.osculating(sys.top_level(j), t);
                    let name = sys.bodies[j].name.clone();
                    let orbit = if el.is_bound() { format!("orbit now a = {:.4} AU, e = {:.4}", el.semi_major_axis / AU, el.eccentricity) } else { "on an unbound path".to_string() };
                    let has_civ = self.civ_on(BodyRef { system: s as u32, body: j as u32 }).is_some();
                    self.history.push(Event {
                        time: t,
                        category: Category::Astronomy,
                        importance: if has_civ || (f / fr - 1.0).abs() > 0.1 { 5 } else { 4 },
                        title: format!("{name}: sunlight {:+.1}%", (f / fr - 1.0) * 100.0),
                        detail: format!(
                            "Annual-mean insolation {:.3} S⊕ ({orbit}). Mean surface temperature {:.1} K → {:.1} K ({:+.1} K) as of {}.",
                            f,
                            before,
                            temp,
                            temp - before,
                            format_date(t, self.start_time, self.gregorian())
                        ),
                        system: Some(s as u32),
                        body: Some(j as u32),
                        civ: None,
                    });
                }
            }
        }
    }

    /// Make sure the environment task exists once any system is dynamic.
    pub(crate) fn ensure_environment_task(&mut self) {
        if !self.systems.iter().any(|s| s.is_dynamic()) {
            return;
        }
        let env = crate::universe::TASK_ENV;
        while self.scheduler.tasks.len() < env {
            self.scheduler.add("placeholder", self.time, crate::scheduler::NEVER);
        }
        if self.scheduler.tasks.len() == env {
            self.scheduler.add("environment", self.time, ENVIRONMENT_PERIOD);
        } else if self.scheduler.tasks[env].period >= crate::scheduler::NEVER {
            // A placeholder held the slot (another task registered after it).
            self.scheduler.tasks[env] = crate::scheduler::Task { name: "environment".into(), origin: self.time, period: ENVIRONMENT_PERIOD, steps: 0 };
        }
    }
}

/// Period of the orbit → climate task in dynamic systems.
pub const ENVIRONMENT_PERIOD: f64 = 10.0 * SECONDS_PER_DAY;

/// Annual-mean insolation of `body` relative to Earth's (S⊕).
pub fn flux_of(sys: &StarSystem, body: usize, t: f64) -> f64 {
    insolation(&sys.star, t, sys.stellar_distance(body))
}

impl StarSystem {
    /// Rotation angle of a body about its axis at `t` (tidally locked bodies face their
    /// parent). The renderer uses the same function so impacts land where they are drawn.
    pub fn spin_angle(&self, body: usize, t: f64) -> f64 {
        let b = &self.bodies[body];
        if b.tidally_locked {
            let rel = self.body_local_position(body, t) - b.parent.map(|p| self.body_local_position(p as usize, t)).unwrap_or_else(|| self.star_local_position(t));
            (-rel.y).datan2(-rel.x)
        } else {
            (t / b.rotation_period).rem_euclid(1.0) * std::f64::consts::TAU
        }
    }

    /// Convert a direction in the system frame to body-fixed coordinates (the frame of
    /// latitude/longitude and terrain): undo the spin about +Z, then the axial tilt about +X.
    pub fn inertial_to_body_fixed(&self, body: usize, dir: Vec3d, t: f64) -> [f64; 3] {
        let tilt = self.bodies[body].axial_tilt;
        let spin = self.spin_angle(body, t);
        // Rx(−tilt)
        let (st, ct) = tilt.dsin_cos();
        let v = Vec3d::new(dir.x, dir.y * ct + dir.z * st, -dir.y * st + dir.z * ct);
        // Rz(−spin)
        let (ss, cs) = spin.dsin_cos();
        [v.x * cs + v.y * ss, -v.x * ss + v.y * cs, v.z]
    }
}

/// 1.2 million, 3.4 billion — for event text.
fn compact(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.2} billion", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.2} million", n / 1e6)
    } else if n >= 1e4 {
        format!("{:.0} thousand", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

// ── Curated experiments ─────────────────────────────────────────────────────

/// A curated "what if" built from ordinary edits on the Solar System Lab — no special physics.
#[derive(Clone, Copy, Debug)]
pub struct WhatIf {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

pub const WHAT_IFS: &[WhatIf] = &[
    WhatIf { id: "no_moon", title: "What if the Moon disappeared?", description: "The Moon is removed from today's Solar System. Watch Earth's orbit and climate without it." },
    WhatIf { id: "jupiter_x2", title: "What if Jupiter were twice as massive?", description: "Jupiter's mass doubles; every planet feels the stronger pull." },
    WhatIf { id: "sun_plus10", title: "What if the Sun were 10% more massive?", description: "A brighter, heavier Sun: faster orbits and a hotter Earth." },
    WhatIf { id: "chicxulub_today", title: "What if Chicxulub happened today?", description: "A 10 km asteroid is a day away from Earth, on a collision course." },
    WhatIf { id: "rogue_planet", title: "What if a rogue planet passed through?", description: "An Earth-mass rogue world falls in from 40 AU on a path that crosses the inner Solar System." },
    WhatIf { id: "two_moons", title: "What if Earth had two moons?", description: "A second Moon-like body orbits Earth at twice the Moon's distance." },
    WhatIf { id: "mars_earth_air", title: "What if Mars had Earth's air?", description: "Mars gets a 1-bar nitrogen–oxygen atmosphere and some of its water back." },
    WhatIf { id: "sun_black_hole", title: "What if the Sun became a black hole?", description: "Same mass, 3 km across: every orbit stays exactly the same — but the light goes out." },
    WhatIf { id: "black_hole_flyby", title: "What if a black hole passed through?", description: "A 10 M☉ black hole falls in from 60 AU on a path through the inner Solar System." },
    WhatIf { id: "sun_red_giant", title: "What if it were 7.6 billion years from now?", description: "The Sun near the tip of its red-giant phase, swelling towards 1 AU. Mercury and Venus are next." },
    WhatIf { id: "sun_supernova", title: "What if the Sun were a dying supergiant?", description: "A 25 M☉ star at the very end of its life takes the Sun's place. Its gravity is 25× stronger, so the planets plunge inwards; within decades it explodes and leaves a black hole." },
    WhatIf { id: "feeding_black_hole", title: "What if a black hole orbited the Sun?", description: "A 10 M☉ black hole feeding on gas circles at 30 AU, its disk blazing — and Neptune's orbit is in its way." },
    WhatIf { id: "second_sun", title: "What if a second sun arrived?", description: "A Sun-like star approaches from 300 AU. Will the planets stay with their star — or follow the newcomer?" },
];

fn find(u: &Universe, name: &str) -> Result<BodyRef, String> {
    u.system(0).find_body(name).map(|i| BodyRef { system: 0, body: i as u32 }).ok_or_else(|| format!("{name} not found"))
}

/// Set up a curated experiment on a freshly created Solar System Lab.
pub fn apply_what_if(u: &mut Universe, id: &str) -> Result<(), String> {
    let t = u.time;
    match id {
        "no_moon" => {
            let moon = find(u, "Moon")?;
            u.apply_edit(Edit::RemoveBody { body: moon })?;
        }
        "jupiter_x2" => {
            let j = find(u, "Jupiter")?;
            let m = u.body(j).mass * 2.0;
            u.apply_edit(Edit::SetProperty { body: j, property: BodyProperty::Mass(m) })?;
        }
        "sun_plus10" => {
            let m = u.system(0).star.mass * 1.1;
            u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Mass(m) })?;
        }
        "chicxulub_today" => {
            let e = find(u, "Earth")?;
            let body = body_from_preset(preset("asteroid_10km").unwrap(), "Impactor", 0xC41C);
            let state = aimed_state(u.system(0), e.body as usize, 1.5e9, Vec3d::new(0.2, 1.0, 0.15), 17_000.0, 2.0e6, t);
            u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state })?;
        }
        "rogue_planet" => {
            let body = body_from_preset(preset("earth_like").unwrap(), "Rogue", 0x2066);
            let sys = u.system(0);
            let gm = sys.star.mu();
            let (r0, v0, q) = (40.0 * AU, 8_000.0, 1.1 * AU);
            // Hyperbolic path with perihelion q: energy and angular momentum from (r0, v0, q).
            let energy = 0.5 * v0 * v0 - gm / r0;
            let vq = (2.0 * (energy + gm / q)).sqrt();
            let vt = q * vq / r0;
            let vr = -(v0 * v0 - vt * vt).max(0.0).sqrt();
            let dir = Vec3d::new(0.8, 0.6, 0.05).normalize();
            let side = Vec3d::new(0.0, 0.0, 1.0).cross(dir).normalize();
            let star = sys.star_local_position(t);
            let state = State { pos: star + dir * r0, vel: dir * vr + side * vt };
            let mut body = body;
            body.class = Some(ObjectClass::RoguePlanet);
            u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state })?;
        }
        "two_moons" => {
            let e = find(u, "Earth")?;
            let mut body = body_from_preset(preset("moon_like").unwrap(), "Selene", 0x5E1E);
            body.parent = Some(e.body);
            let state = circular_state(u.system(0), Some(e.body as usize), body.mass, 2.0 * 384_400e3, 2.5, 0.09, t);
            u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state })?;
        }
        "mars_earth_air" => {
            let m = find(u, "Mars")?;
            u.apply_edit(Edit::SetProperty { body: m, property: BodyProperty::SurfacePressure(1.0) })?;
            {
                let a = &mut u.systems[0].bodies[m.body as usize].atmosphere;
                a.n2 = 0.7804;
                a.o2 = 0.2095;
                a.co2 = 0.0004;
                a.h2o = 0.0;
                a.ch4 = 0.0;
                a.h2he = 0.0;
            }
            u.apply_edit(Edit::SetProperty { body: m, property: BodyProperty::WaterInventory(0.3) })?;
        }
        "sun_black_hole" => {
            u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Kind(crate::astro::star::StarKind::BlackHole) })?;
        }
        "black_hole_flyby" | "second_sun" => {
            let (pid, name, r0, v0, q) = if id == "black_hole_flyby" { ("black_hole_10", "Wanderer", 60.0 * AU, 15_000.0, 2.5 * AU) } else { ("sun_like_star", "Nemesis", 300.0 * AU, 6_000.0, 8.0 * AU) };
            let mut body = body_from_preset(preset(pid).unwrap(), name, 0xB1AC);
            // A lone wandering black hole has nothing to feed on: dark, seen only by lensing.
            body.accretion = 0.0;
            let sys = u.system(0);
            let gm = sys.star.mu() + G * body.mass;
            let energy = 0.5 * v0 * v0 - gm / r0;
            let vq = (2.0 * (energy + gm / q)).sqrt();
            let vt = q * vq / r0;
            let vr = -(v0 * v0 - vt * vt).max(0.0).sqrt();
            let dir = Vec3d::new(-0.6, 0.75, 0.12).normalize();
            let side = Vec3d::new(0.0, 0.0, 1.0).cross(dir).normalize();
            let star = sys.star_local_position(t);
            let state = State { pos: star + dir * r0, vel: dir * vr + side * vt };
            u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state })?;
        }
        "feeding_black_hole" => {
            let body = body_from_preset(preset("black_hole_10").unwrap(), "Charon's Gate", 0xFEED);
            let state = circular_state(u.system(0), None, body.mass, 30.0 * AU, 1.2, 0.05, t);
            u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state })?;
        }
        "sun_red_giant" => {
            let s = &u.system(0).star;
            let age = s.lifetime * 1.1185;
            u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Age(age) })?;
        }
        "sun_supernova" => {
            u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Mass(25.0) })?;
            let life = u.system(0).star.lifetime;
            // Decades before core collapse.
            u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Age(life * crate::astro::star::END_OF_LIFE - 30.0 * SECONDS_PER_YEAR) })?;
        }
        other => return Err(format!("unknown experiment {other}")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::dynamics::PhysicsPreset;
    use crate::universe::{Scenario, UniverseSettings};

    fn lab() -> Universe {
        Universe::new(UniverseSettings { seed: 7, scenario: Scenario::SolarSystemLab, ..Default::default() })
    }

    fn earth(u: &Universe) -> BodyRef {
        BodyRef { system: 0, body: u.system(0).find_body("Earth").unwrap() as u32 }
    }

    #[test]
    fn lab_starts_from_real_data_today() {
        let u = lab();
        assert!(u.system(0).is_dynamic());
        assert_eq!(u.systems.len(), 1, "no procedural neighbours in a real-data template");
        assert!((u.time / SECONDS_PER_YEAR - 26.0).abs() < 0.01);
        let e = earth(&u);
        assert!(u.biosphere(e).unwrap().vegetated());
        assert_eq!(u.civs.len(), 1, "present-day humanity only");
        // Mars has no invented life.
        let mars = u.system(0).find_body("Mars").unwrap() as u32;
        assert_eq!(u.biosphere(BodyRef { system: 0, body: mars }).unwrap().stage, Stage::Sterile);
        assert_eq!(u.body(e).quality("position"), Quality::Measured);
        assert!((u.body(e).temperature - 288.0).abs() < 2.0, "{}", u.body(e).temperature);
    }

    /// Demonstration 1: add an Earth-mass planet between Earth and Mars; the system
    /// responds; saving and reloading continues identically.
    #[test]
    fn add_planet_perturbs_and_save_resumes() {
        let mut u = lab();
        let mut control = u.clone();
        let p = preset("earth_like").unwrap();
        let body = body_from_preset(p, "Nova", 99);
        let state = circular_state(u.system(0), None, body.mass, 1.26 * AU, 0.3, 0.0, u.time);
        let out = u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state }).unwrap();
        let nova = out.created.unwrap();
        assert_eq!(u.body(nova).quality("mass"), Quality::UserModified);
        u.advance_by(20.0 * SECONDS_PER_YEAR);
        control.advance_by(20.0 * SECONDS_PER_YEAR);
        let mars = u.system(0).find_body("Mars").unwrap();
        let em = u.system(0).osculating(mars, u.time).eccentricity;
        let ec = control.system(0).osculating(mars, control.time).eccentricity;
        assert!((em - ec).abs() > 1e-5, "Mars must feel the new planet: {em} vs {ec}");
        assert!(u.body(nova).exists());
        // Save → load → continue == uninterrupted.
        let json = crate::save::to_json(&u, "nova").unwrap();
        let mut loaded = crate::save::from_json(&json).unwrap();
        let mut straight = u.clone();
        straight.advance_by(2.0 * SECONDS_PER_YEAR);
        loaded.advance_by(2.0 * SECONDS_PER_YEAR);
        assert_eq!(serde_json::to_string(&straight).unwrap(), serde_json::to_string(&loaded).unwrap());
        assert_eq!(u.edits.len(), 1);
    }

    /// Demonstration 2: an asteroid launched at Earth hits it; the impact reaches the
    /// environment and the biosphere.
    #[test]
    fn asteroid_impact_on_earth() {
        let mut u = lab();
        let e = earth(&u);
        let p = preset("asteroid_10km").unwrap();
        let body = body_from_preset(p, "Impactor", 5);
        let state = aimed_state(u.system(0), e.body as usize, 4.0e8, Vec3d::new(0.3, 1.0, 0.1), 18_000.0, 0.0, u.time);
        let out = u.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state }).unwrap();
        let imp = out.created.unwrap();
        let t0 = u.time;
        let temp0 = u.body(e).temperature;
        let div0 = u.biosphere(e).unwrap().biodiversity;
        // Coarse chunks: the step grid, not the frame size, decides the outcome.
        for _ in 0..30 {
            u.advance_by(SECONDS_PER_DAY);
        }
        assert!(!u.body(imp).exists(), "the asteroid should have hit Earth");
        let rec = u.body(e).impacts.last().expect("impact recorded");
        assert!(rec.time > t0 && rec.time < t0 + 3.0 * SECONDS_PER_DAY);
        assert!(rec.speed > 18_000.0, "gravity accelerates the impactor: {}", rec.speed);
        assert!(rec.energy_mt() > 3e7 && rec.energy_mt() < 3e8, "{} Mt", rec.energy_mt());
        assert!(rec.crater_m > 80e3, "{} m", rec.crater_m);
        assert!(matches!(rec.class, ImpactClass::MassExtinction));
        assert!(u.biosphere(e).unwrap().biodiversity < div0 * 0.5);
        assert!(u.body(e).temperature < temp0 - 10.0, "impact winter: {} -> {}", temp0, u.body(e).temperature);
        assert!(u.history.events.iter().any(|ev| ev.title == "Impact on Earth"));
        // Momentum went into Earth; Earth's mass grew by the impactor's.
        assert!(u.body(e).mass > 5.97e24);
    }

    /// Moving Earth inward raises insolation and temperature through the environment task.
    #[test]
    fn moving_earth_changes_its_climate() {
        let mut u = lab();
        let e = earth(&u);
        let t_before = u.body(e).temperature;
        let el = u.system(0).osculating(e.body as usize, u.time);
        u.apply_edit(Edit::SetOrbit { body: e, a: 0.9 * AU, e: el.eccentricity, i: el.inclination, node: el.longitude_ascending, peri: el.argument_perihelion, mean_anomaly: el.mean_anomaly }).unwrap();
        u.advance_by(30.0 * SECONDS_PER_DAY);
        let t_after = u.body(e).temperature;
        assert!(t_after > t_before + 10.0, "{t_before} -> {t_after}");
        assert!(u.history.events.iter().any(|ev| ev.title.starts_with("Earth: sunlight +")));
    }

    #[test]
    fn dynamic_outcome_is_independent_of_frame_chunking() {
        let mut a = lab();
        let p = preset("mars_like").unwrap();
        let body = body_from_preset(p, "Extra", 3);
        let state = circular_state(a.system(0), None, body.mass, 1.9 * AU, 2.0, 0.05, a.time);
        a.apply_edit(Edit::AddBody { system: 0, body: Box::new(body), state }).unwrap();
        let mut b = a.clone();
        let total = 3.0 * SECONDS_PER_YEAR;
        a.advance_by(total);
        let start = b.time;
        let mut done = 0.0;
        let mut i = 0;
        while done < total {
            let dt = [0.37, 13.0, 101.3, 7.7][i % 4] * SECONDS_PER_DAY;
            done = (done + dt).min(total);
            b.advance_to(start + done, None, None);
            i += 1;
        }
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    }

    #[test]
    fn star_mass_edit_changes_luminosity_and_climate() {
        let mut u = lab();
        let e = earth(&u);
        let t0 = u.body(e).temperature;
        u.apply_edit(Edit::SetStar { system: 0, property: StarProperty::Mass(1.1) }).unwrap();
        assert!(u.system(0).star.luminosity(u.time) > 1.3);
        assert!(u.body(e).temperature > t0 + 5.0);
    }

    #[test]
    fn delete_and_freeze_orbits() {
        let mut u = lab();
        let moon = BodyRef { system: 0, body: u.system(0).find_body("Moon").unwrap() as u32 };
        u.apply_edit(Edit::RemoveBody { body: moon }).unwrap();
        assert!(!u.body(moon).exists());
        u.advance_by(SECONDS_PER_YEAR);
        u.apply_edit(Edit::SetPhysics { system: 0, nbody: false, settings: PhysicsPreset::Balanced.settings() }).unwrap();
        assert!(!u.system(0).is_dynamic());
        u.advance_by(SECONDS_PER_YEAR);
        assert!(u.apply_edit(Edit::SetProperty { body: moon, property: BodyProperty::Mass(1.0) }).is_err());
    }

    #[test]
    fn invalid_edits_are_refused_absurd_ones_allowed() {
        let mut u = lab();
        let e = earth(&u);
        assert!(u.apply_edit(Edit::SetProperty { body: e, property: BodyProperty::Mass(-1.0) }).is_err());
        assert!(u.apply_edit(Edit::SetProperty { body: e, property: BodyProperty::Mass(f64::NAN) }).is_err());
        // Earth-sized with Jupiter's mass: allowed, with a warning.
        u.apply_edit(Edit::SetProperty { body: e, property: BodyProperty::Mass(1.898e27) }).unwrap();
        assert!(warnings(&u, e).iter().any(|w| w.contains("exceeds any ordinary")));
    }

    #[test]
    fn every_what_if_sets_up_and_runs() {
        for w in WHAT_IFS {
            let mut u = lab();
            apply_what_if(&mut u, w.id).unwrap_or_else(|e| panic!("{}: {e}", w.id));
            u.advance_by(0.2 * SECONDS_PER_YEAR);
            assert!(u.system(0).dynamics.as_ref().unwrap().diagnostics.energy_error.is_finite(), "{}", w.id);
        }
        // Chicxulub today really hits.
        let mut u = lab();
        apply_what_if(&mut u, "chicxulub_today").unwrap();
        u.advance_by(5.0 * SECONDS_PER_DAY);
        assert!(!u.body(earth(&u)).impacts.is_empty());
    }

    #[test]
    fn a_black_hole_sun_keeps_the_orbits_and_turns_out_the_light() {
        // Gravity outside the horizon is the Sun's: Earth follows the same path as in an
        // untouched Solar System.
        let mut control = lab();
        control.apply_edit(Edit::SetPhysics { system: 0, nbody: true, settings: control.system(0).dynamics.as_ref().unwrap().settings }).unwrap();
        control.advance_by(1.0 * SECONDS_PER_YEAR);
        let mut u = lab();
        let e = earth(&u);
        apply_what_if(&mut u, "sun_black_hole").unwrap();
        u.advance_by(1.0 * SECONDS_PER_YEAR);
        let p0 = control.body_position(e, control.time);
        let p1 = u.body_position(e, u.time);
        assert!((p1 - p0).length() < 1.0e6, "Earth's position differs by {} km", (p1 - p0).length() / 1000.0);
        assert!(u.body(e).temperature < 100.0, "Earth without sunlight: {} K", u.body(e).temperature);
        assert!(u.history.events.iter().any(|ev| ev.title.contains("black hole")));
    }

    #[test]
    fn a_wandering_black_hole_wrecks_the_inner_system() {
        let mut u = lab();
        apply_what_if(&mut u, "black_hole_flyby").unwrap();
        let earth_a0 = u.system(0).osculating(earth(&u).body as usize, u.time).semi_major_axis;
        u.advance_by(40.0 * SECONDS_PER_YEAR);
        let e = &u.system(0).bodies[earth(&u).body as usize];
        let changed = !e.exists() || {
            let el = u.system(0).osculating(earth(&u).body as usize, u.time);
            !el.is_bound() || (el.semi_major_axis / earth_a0 - 1.0).abs() > 0.05
        };
        assert!(changed, "a 10 M☉ black hole passing at 2.5 AU must disturb Earth");
    }

    #[test]
    fn prediction_sees_an_aimed_impact() {
        let u = lab();
        let e = earth(&u);
        let body = body_from_preset(preset("asteroid_1km").unwrap(), "A", 1);
        let (sa, ca) = 0.6f64.sin_cos();
        let (se, ce) = 0.1f64.sin_cos();
        let st = aimed_state(u.system(0), e.body as usize, 2.0e9, Vec3d::new(ce * ca, ce * sa, se), 20_000.0, 0.0, u.time);
        let p = u.system(0).predict(u.time, Some(cosmogon_physics::nbody::Particle::new(st.pos, st.vel, G * body.mass, body.radius)), 2.0 * SECONDS_PER_YEAR, 600, 30_000);
        let (a, b, tc) = p.contact.expect("impact predicted");
        assert_eq!(a, Slot::Body(e.body));
        assert_eq!(b, Slot::Body(u32::MAX));
        assert!(tc - u.time < 2.0 * SECONDS_PER_DAY, "{}", (tc - u.time) / SECONDS_PER_DAY);
    }

    #[test]
    fn dawn_of_humanity_with_dynamic_gravity_keeps_civilization() {
        let mut u = Universe::new(UniverseSettings { seed: 3, scenario: Scenario::Sol, system_count: 3, physics: Some(PhysicsPreset::Fast.settings()), ..Default::default() });
        assert!(u.system(0).is_dynamic());
        let pop0 = u.civs[0].population;
        u.advance_by(200.0 * SECONDS_PER_YEAR);
        assert!(u.civs[0].is_alive() && u.civs[0].population > pop0 * 0.9);
    }
}
