//! Stars: main-sequence relations and a coarse evolution model.
//!
//! Simplifications (documented in SIMULATION.md): piecewise mass–luminosity and
//! mass–radius power laws, a linear brightening of ~40% over the main-sequence lifetime,
//! and an abrupt giant phase followed by a white-dwarf remnant. Good enough to make
//! habitable zones migrate and planets die; not a stellar-evolution code.

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarPhase {
    MainSequence,
    Giant,
    WhiteDwarf,
}

impl StellarPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::MainSequence => "Main sequence",
            Self::Giant => "Red giant",
            Self::WhiteDwarf => "White dwarf remnant",
        }
    }
}

pub fn luminosity_from_mass(m: f64) -> f64 {
    if m < 0.43 {
        0.23 * m.powf(2.3)
    } else if m < 2.0 {
        m.powi(4)
    } else {
        1.4 * m.powf(3.5)
    }
}

pub fn radius_from_mass(m: f64) -> f64 {
    if m < 1.0 {
        m.powf(0.8)
    } else {
        m.powf(0.57)
    }
}

impl Star {
    pub fn from_mass(name: String, mass: f64, metallicity: f64, formed_at: f64) -> Self {
        let l = luminosity_from_mass(mass);
        let r = radius_from_mass(mass);
        let temperature = 5772.0 * (l / (r * r)).powf(0.25);
        let lifetime = (10.0 * mass.powf(-2.5)).min(1.0e4) * SECONDS_PER_GYR;
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
        }
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
        let f = self.life_fraction(t);
        if f < 1.0 {
            StellarPhase::MainSequence
        } else if f < 1.12 {
            StellarPhase::Giant
        } else {
            StellarPhase::WhiteDwarf
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
            StellarPhase::WhiteDwarf => 1.0e-3,
        }
    }

    pub fn current_radius(&self, t: f64) -> f64 {
        match self.phase(t) {
            StellarPhase::MainSequence => self.radius * (0.9 + 0.2 * self.life_fraction(t).clamp(0.0, 1.0)),
            StellarPhase::Giant => self.radius * (1.0 + 1500.0 * (self.life_fraction(t) - 1.0)).min(250.0),
            StellarPhase::WhiteDwarf => 7.0e6,
        }
    }

    /// Effective temperature (K) at time `t`, from L and R via Stefan–Boltzmann.
    pub fn temperature_at(&self, t: f64) -> f64 {
        match self.phase(t) {
            StellarPhase::WhiteDwarf => 12_000.0,
            _ => {
                let l = self.luminosity(t);
                let r = self.current_radius(t) / SOLAR_RADIUS;
                5772.0 * (l / (r * r)).powf(0.25)
            }
        }
    }

    /// Flare activity at time `t` (decays with age).
    pub fn flare_activity_at(&self, t: f64) -> f64 {
        let age_gyr = self.age(t).max(0.0) / SECONDS_PER_GYR;
        let tau = if self.mass < 0.45 { 4.0 } else { 1.0 };
        self.flare_activity * (-age_gyr / tau).exp()
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
            StellarPhase::WhiteDwarf => return "DA".into(),
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
        let (r, g, b) = cosmogon_core::units::kelvin_to_rgb(self.temperature_at(t));
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
    fn red_dwarfs_are_dim_long_lived_and_flaring() {
        let m = Star::from_mass("x".into(), 0.2, 0.0, 0.0);
        assert!(m.luminosity_ms < 0.01);
        assert!(m.lifetime > 100.0 * SECONDS_PER_GYR);
        assert!(m.spectral_type(1.0).starts_with('M'));
        assert!(m.flare_activity_at(0.0) > 0.5);
    }
}
