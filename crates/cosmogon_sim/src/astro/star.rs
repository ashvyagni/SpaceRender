//! Stars: main-sequence relations and a coarse evolution model.
//!
//! Simplifications (documented in PHYSICS_ENGINE.md, "Stellar evolution"): piecewise
//! mass–luminosity and mass–radius power laws, a linear brightening of ~40% over the
//! main-sequence lifetime, a giant phase lasting 12% of it, then a remnant set by the
//! initial mass: white dwarf (< 8 M☉, mass from the initial–final mass relation of
//! Kalirai et al. 2008), neutron star (8–20 M☉, 1.4 M☉) or black hole (> 20 M☉). Good
//! enough to make habitable zones migrate and planets die; not a stellar-evolution code.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use super::{G, SOLAR_MASS, SOLAR_RADIUS};
use crate::time::SECONDS_PER_GYR;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Star {
    pub name: String,
    /// Mass (solar masses).
    pub mass: f64,
    /// Main-sequence radius (m).
    pub radius: f64,
    /// Characteristic main-sequence luminosity (L☉), i.e. at mid-life.
    pub luminosity_ms: f64,
    /// Effective temperature on the main sequence (K).
    pub temperature: f64,
    /// Time of formation (s relative to J2000; negative = in the past).
    pub formed_at: f64,
    /// Main-sequence lifetime (s).
    pub lifetime: f64,
    /// [Fe/H] (dex). 0 = solar.
    pub metallicity: f64,
    /// Flare activity when young, 0..1 (mostly relevant for M dwarfs).
    pub flare_activity: f64,
    /// What the star is now. A remnant's `mass` is the remnant's mass.
    #[serde(default)]
    pub kind: StarKind,
    /// When the remnant formed (s), for cooling and nebula ages.
    #[serde(default)]
    pub remnant_since: Option<f64>,
    /// Mass before it became a remnant (M☉).
    #[serde(default)]
    pub initial_mass: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum StarKind {
    #[default]
    Normal,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
}

impl StarKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Star",
            Self::WhiteDwarf => "White dwarf",
            Self::NeutronStar => "Neutron star",
            Self::BlackHole => "Black hole",
        }
    }
}

/// How a star of a given initial mass ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fate {
    pub kind: StarKind,
    /// Remnant mass (M☉).
    pub mass: f64,
    /// Whether it explodes as a core-collapse supernova.
    pub supernova: bool,
}

/// Initial mass (M☉) → remnant. White dwarfs from the initial–final mass relation
/// (Kalirai et al. 2008: M_f = 0.109 M_i + 0.394); neutron stars at 1.4 M☉; black holes
/// keep roughly a quarter of the star plus a core (5–40 M☉), after Fryer et al. 2012.
pub fn fate(initial_mass: f64) -> Fate {
    if initial_mass < 8.0 {
        Fate { kind: StarKind::WhiteDwarf, mass: (0.109 * initial_mass + 0.394).min(1.35), supernova: false }
    } else if initial_mass < 20.0 {
        Fate { kind: StarKind::NeutronStar, mass: 1.4, supernova: true }
    } else {
        Fate { kind: StarKind::BlackHole, mass: (0.25 * initial_mass + 2.0).clamp(5.0, 40.0), supernova: true }
    }
}

/// Schwarzschild radius (m) of a mass in M☉.
pub fn schwarzschild_radius(mass_sun: f64) -> f64 {
    2.0 * G * mass_sun * SOLAR_MASS / (super::SPEED_OF_LIGHT * super::SPEED_OF_LIGHT)
}

/// Fraction of main-sequence life at which the giant phase ends and the remnant forms.
pub const END_OF_LIFE: f64 = 1.12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarPhase {
    MainSequence,
    Giant,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
}

impl StellarPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::MainSequence => "Main sequence",
            Self::Giant => "Red giant",
            Self::WhiteDwarf => "White dwarf remnant",
            Self::NeutronStar => "Neutron star",
            Self::BlackHole => "Black hole",
        }
    }
    pub fn is_remnant(self) -> bool {
        matches!(self, Self::WhiteDwarf | Self::NeutronStar | Self::BlackHole)
    }
}

pub fn luminosity_from_mass(m: f64) -> f64 {
    if m < 0.43 {
        0.23 * m.dpowf(2.3)
    } else if m < 2.0 {
        m.powi(4)
    } else {
        1.4 * m.dpowf(3.5)
    }
}

pub fn radius_from_mass(m: f64) -> f64 {
    if m < 1.0 {
        m.dpowf(0.8)
    } else {
        m.dpowf(0.57)
    }
}

impl Star {
    pub fn from_mass(name: String, mass: f64, metallicity: f64, formed_at: f64) -> Self {
        let l = luminosity_from_mass(mass);
        let r = radius_from_mass(mass);
        let temperature = 5772.0 * (l / (r * r)).dpowf(0.25);
        let lifetime = (10.0 * mass.dpowf(-2.5)).min(1.0e4) * SECONDS_PER_GYR;
        let flare_activity = if mass < 0.45 { 1.0 } else if mass < 0.8 { 0.3 } else { 0.08 };
        Self {
            name,
            mass,
            radius: r * SOLAR_RADIUS,
            luminosity_ms: l,
            temperature,
            formed_at,
            lifetime,
            metallicity,
            flare_activity,
            kind: StarKind::Normal,
            remnant_since: None,
            initial_mass: None,
        }
    }

    /// A compact object from the start (sandbox creations): white dwarf, neutron star or
    /// black hole of `mass` M☉ that formed at `formed_at`.
    pub fn compact(name: String, kind: StarKind, mass: f64, formed_at: f64) -> Self {
        let mut s = Self::from_mass(name, mass.max(0.08), 0.0, formed_at);
        s.mass = mass;
        s.kind = kind;
        s.remnant_since = Some(formed_at);
        s.flare_activity = 0.0;
        s
    }

    /// Turn into the remnant this star's mass dictates (at time `t`).
    pub fn become_remnant(&mut self, t: f64) -> Fate {
        let f = fate(self.mass);
        self.initial_mass = Some(self.mass);
        self.mass = f.mass;
        self.kind = f.kind;
        self.remnant_since = Some(t);
        self.flare_activity = 0.0;
        f
    }

    /// When the remnant will form, for a star still burning (s).
    pub fn end_of_life(&self) -> f64 {
        self.formed_at + self.lifetime * END_OF_LIFE
    }

    /// When the giant phase begins (s).
    pub fn giant_onset(&self) -> f64 {
        self.formed_at + self.lifetime
    }

    fn remnant_age(&self, t: f64) -> f64 {
        (t - self.remnant_since.unwrap_or(self.formed_at)).max(0.0)
    }

    pub fn mu(&self) -> f64 {
        G * self.mass * SOLAR_MASS
    }

    pub fn age(&self, t: f64) -> f64 {
        t - self.formed_at
    }

    /// Fraction of main-sequence life used at time `t`.
    pub fn life_fraction(&self, t: f64) -> f64 {
        self.age(t) / self.lifetime
    }

    pub fn phase(&self, t: f64) -> StellarPhase {
        match self.kind {
            StarKind::WhiteDwarf => return StellarPhase::WhiteDwarf,
            StarKind::NeutronStar => return StellarPhase::NeutronStar,
            StarKind::BlackHole => return StellarPhase::BlackHole,
            StarKind::Normal => {}
        }
        let f = self.life_fraction(t);
        if f < 1.0 {
            StellarPhase::MainSequence
        } else if f < END_OF_LIFE {
            StellarPhase::Giant
        } else {
            // Past its end but not yet converted by the simulation (between steps): treat as
            // the remnant it is about to become.
            match fate(self.mass).kind {
                StarKind::NeutronStar => StellarPhase::NeutronStar,
                StarKind::BlackHole => StellarPhase::BlackHole,
                _ => StellarPhase::WhiteDwarf,
            }
        }
    }

    /// Luminosity (L☉) at time `t`.
    pub fn luminosity(&self, t: f64) -> f64 {
        let f = self.life_fraction(t).max(0.0);
        match self.phase(t) {
            // Brightens ~70% -> ~135% of the characteristic value across the main sequence;
            // calibrated so the present-day Sun (f ≈ 0.46) has L ≈ 1.
            StellarPhase::MainSequence => self.luminosity_ms * (0.72 + 0.62 * f),
            StellarPhase::Giant => self.luminosity_ms * (1.34 + 2000.0 * (f - 1.0)),
            // Cooling white dwarf (Mestel: L ∝ t^-7/5), ~0.01 L☉ at 100 Myr.
            StellarPhase::WhiteDwarf => (0.01 * (self.remnant_age(t) / (0.1 * SECONDS_PER_GYR)).max(1e-3).dpowf(-1.4)).clamp(1e-5, 1.0),
            // Thermal emission of a cooling neutron star is negligible in visible light.
            StellarPhase::NeutronStar => 1.0e-6,
            StellarPhase::BlackHole => 0.0,
        }
    }

    pub fn current_radius(&self, t: f64) -> f64 {
        match self.phase(t) {
            StellarPhase::MainSequence => self.radius * (0.9 + 0.2 * self.life_fraction(t).clamp(0.0, 1.0)),
            // Red giants reach ~200 R☉ (≈1 AU for the Sun); supergiants ~1000 R☉.
            StellarPhase::Giant => {
                let cap = if self.mass >= 8.0 { 1000.0 } else { 220.0 } * SOLAR_RADIUS / self.radius;
                self.radius * (1.0 + (cap - 1.0) * ((self.life_fraction(t) - 1.0) / (END_OF_LIFE - 1.0)).clamp(0.0, 1.0).dpowf(1.5))
            }
            // Mass–radius relation of white dwarfs (smaller when heavier): ~0.012 R☉ at 0.6 M☉.
            StellarPhase::WhiteDwarf => 0.0125 * SOLAR_RADIUS * (self.remnant_mass() / 0.6).dpowf(-1.0 / 3.0),
            StellarPhase::NeutronStar => 12_000.0,
            StellarPhase::BlackHole => schwarzschild_radius(self.remnant_mass()),
        }
    }

    /// Mass now (M☉): the remnant's mass once the star has ended.
    pub fn remnant_mass(&self) -> f64 {
        if self.kind == StarKind::Normal {
            fate(self.mass).mass
        } else {
            self.mass
        }
    }

    /// Effective temperature (K) at time `t`, from L and R via Stefan–Boltzmann.
    pub fn temperature_at(&self, t: f64) -> f64 {
        match self.phase(t) {
            StellarPhase::WhiteDwarf => {
                let l = self.luminosity(t);
                let r = self.current_radius(t) / SOLAR_RADIUS;
                (5772.0 * (l / (r * r)).dpowf(0.25)).max(3000.0)
            }
            StellarPhase::NeutronStar => 600_000.0,
            StellarPhase::BlackHole => 0.0,
            _ => {
                let l = self.luminosity(t);
                let r = self.current_radius(t) / SOLAR_RADIUS;
                5772.0 * (l / (r * r)).dpowf(0.25)
            }
        }
    }

    /// Flare activity at time `t` (decays with age).
    pub fn flare_activity_at(&self, t: f64) -> f64 {
        let age_gyr = self.age(t).max(0.0) / SECONDS_PER_GYR;
        let tau = if self.mass < 0.45 { 4.0 } else { 1.0 };
        self.flare_activity * (-age_gyr / tau).dexp()
    }

    /// Conservative habitable zone (AU), Kasting-style flux limits.
    pub fn habitable_zone_au(&self, t: f64) -> (f64, f64) {
        let l = self.luminosity(t);
        ((l / 1.1).sqrt(), (l / 0.53).sqrt())
    }

    /// Water/ice condensation line in the protoplanetary disk (AU).
    pub fn frost_line_au(&self) -> f64 {
        2.7 * (self.luminosity_ms * 0.72).sqrt()
    }

    pub fn spectral_type(&self, t: f64) -> String {
        let temp = self.temperature_at(t);
        let classes: [(char, f64, f64); 7] = [
            ('O', 30_000.0, 50_000.0),
            ('B', 10_000.0, 30_000.0),
            ('A', 7_500.0, 10_000.0),
            ('F', 6_000.0, 7_500.0),
            ('G', 5_200.0, 6_000.0),
            ('K', 3_700.0, 5_200.0),
            ('M', 2_000.0, 3_700.0),
        ];
        let lum_class = match self.phase(t) {
            StellarPhase::MainSequence => "V",
            StellarPhase::Giant => "III",
            StellarPhase::WhiteDwarf => return "DA (white dwarf)".into(),
            StellarPhase::NeutronStar => return "Neutron star".into(),
            StellarPhase::BlackHole => return "Black hole".into(),
        };
        for (c, lo, hi) in classes {
            if temp >= lo {
                let sub = (((hi - temp.min(hi)) / (hi - lo)) * 10.0).floor().clamp(0.0, 9.0);
                return format!("{c}{sub:.0}{lum_class}");
            }
        }
        format!("M9{lum_class}")
    }

    /// Linear RGB tint for rendering.
    pub fn color(&self, t: f64) -> [f32; 3] {
        if self.phase(t) == StellarPhase::BlackHole {
            return [0.0, 0.0, 0.0];
        }
        let (r, g, b) = cosmogon_core::units::kelvin_to_rgb(self.temperature_at(t).min(40_000.0));
        [r, g, b]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sun() -> Star {
        Star::from_mass("Sun".into(), 1.0, 0.0, -4.57 * SECONDS_PER_GYR)
    }

    #[test]
    fn present_day_sun_is_calibrated() {
        let s = sun();
        assert!((s.luminosity(0.0) - 1.0).abs() < 0.03, "{}", s.luminosity(0.0));
        assert!((s.temperature - 5772.0).abs() < 1.0);
        assert!(s.spectral_type(0.0).starts_with('G'), "{}", s.spectral_type(0.0));
        let (inner, outer) = s.habitable_zone_au(0.0);
        assert!(inner < 1.0 && outer > 1.0);
    }

    #[test]
    fn young_sun_was_fainter_and_old_sun_dies() {
        let s = sun();
        assert!(s.luminosity(-4.0 * SECONDS_PER_GYR) < 0.8);
        assert_eq!(s.phase(6.0 * SECONDS_PER_GYR), StellarPhase::Giant);
        assert_eq!(s.phase(9.0 * SECONDS_PER_GYR), StellarPhase::WhiteDwarf);
    }

    #[test]
    fn fates_follow_initial_mass() {
        assert_eq!(fate(1.0).kind, StarKind::WhiteDwarf);
        assert!((fate(1.0).mass - 0.503).abs() < 0.01);
        assert_eq!(fate(12.0).kind, StarKind::NeutronStar);
        assert!(fate(12.0).supernova && !fate(3.0).supernova);
        let bh = fate(30.0);
        assert_eq!(bh.kind, StarKind::BlackHole);
        assert!((5.0..40.0).contains(&bh.mass));
        // A 1 M☉ black hole is ~3 km across in radius.
        assert!((schwarzschild_radius(1.0) - 2953.0).abs() < 5.0);
    }

    #[test]
    fn sun_becomes_a_red_giant_about_one_au_across() {
        let s = sun();
        let tip = s.formed_at + s.lifetime * (END_OF_LIFE - 1e-6);
        let r_au = s.current_radius(tip) / super::super::AU;
        assert!((0.9..1.1).contains(&r_au), "{r_au}");
        let mut wd = s.clone();
        wd.become_remnant(s.end_of_life());
        assert_eq!(wd.phase(s.end_of_life() + 1.0), StellarPhase::WhiteDwarf);
        assert!(wd.current_radius(s.end_of_life()) < 1.0e7);
        assert!(wd.luminosity(s.end_of_life() + SECONDS_PER_GYR) < 1e-3);
    }

    #[test]
    fn red_dwarfs_are_dim_long_lived_and_flaring() {
        let m = Star::from_mass("x".into(), 0.2, 0.0, 0.0);
        assert!(m.luminosity_ms < 0.01);
        assert!(m.lifetime > 100.0 * SECONDS_PER_GYR);
        assert!(m.spectral_type(1.0).starts_with('M'));
        assert!(m.flare_activity_at(0.0) > 0.5);
    }
}
