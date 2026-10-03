//! Impacts: first-order physical consequences of a collision between a small body and a
//! planet. Every relation is documented with its source and validity in
//! docs/PHYSICS_ENGINE.md ("Impacts → consequences"). Inputs are SI.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::time::SECONDS_PER_YEAR;

/// One megaton of TNT (J).
pub const MEGATON_TNT_J: f64 = 4.184e15;
/// Below this an impact has no global climate effect (Toon et al. 1997: ~10⁵–10⁶ Mt).
pub const GLOBAL_THRESHOLD_MT: f64 = 1.0e5;
/// Chicxulub (K–Pg), ~10⁸ Mt (≈ 4 × 10²³ J): the anchor for the extinction and winter scales.
pub const CHICXULUB_MT: f64 = 1.0e8;
/// Energy that boils the oceans and sterilises the surface (Sleep et al. 1989, ~10²⁸ J).
pub const STERILISING_J: f64 = 1.0e28;
/// Impactor/target mass ratio at which an impact is a planet-scale "giant impact".
pub const GIANT_IMPACT_RATIO: f64 = 0.01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImpactClass {
    /// Damage limited to the crater and blast zone.
    Local,
    /// Severe regional devastation, no global climate effect.
    Regional,
    /// Global dust/aerosol loading and cooling.
    Global,
    /// Chicxulub-class: global winter and mass extinction.
    MassExtinction,
    /// Oceans boil; the surface is sterilised.
    Sterilising,
    /// Planet-scale collision (comparable bodies): the surface melts.
    GiantImpact,
    /// Into a body without a solid surface (giant planet or star).
    Atmospheric,
}

impl ImpactClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Regional => "regional",
            Self::Global => "global",
            Self::MassExtinction => "mass-extinction",
            Self::Sterilising => "sterilising",
            Self::GiantImpact => "giant impact",
            Self::Atmospheric => "atmospheric",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImpactRecord {
    pub time: f64,
    pub impactor: String,
    pub impactor_mass: f64,
    pub impactor_radius: f64,
    /// Impact speed (m/s), including the target's gravitational acceleration.
    pub speed: f64,
    /// Angle from the horizontal (degrees; 90 = vertical).
    pub angle_deg: f64,
    pub energy_j: f64,
    /// Body-fixed latitude / longitude of the impact point (rad).
    pub lat: f64,
    pub lon: f64,
    pub ocean: bool,
    /// Final crater diameter (m); 0 for atmospheric impacts.
    pub crater_m: f64,
    /// Radius of severe blast damage (≈ 5 psi overpressure), m.
    pub blast_radius_m: f64,
    pub class: ImpactClass,
    /// Fraction of species lost.
    pub extinction: f64,
    /// Peak impact-winter cooling (K).
    pub cooling_k: f64,
    /// People killed directly (blast and crater).
    pub casualties: f64,
}

impl ImpactRecord {
    pub fn energy_mt(&self) -> f64 {
        self.energy_j / MEGATON_TNT_J
    }
}

/// Transient surface cooling from impact dust and aerosols. Ocean thermal inertia keeps
/// sea ice and ocean cover from following a few-year event, so it lowers surface air
/// temperature only (Brugger et al. 2017).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ImpactWinter {
    pub start: f64,
    pub peak_k: f64,
    pub efold_years: f64,
}

impl ImpactWinter {
    pub const EFOLD_YEARS: f64 = 3.0;

    pub fn cooling_at(&self, t: f64) -> f64 {
        if t < self.start {
            return 0.0;
        }
        self.peak_k * (-(t - self.start) / (self.efold_years * SECONDS_PER_YEAR)).dexp()
    }

    pub fn expired(&self, t: f64) -> bool {
        self.cooling_at(t) < 0.05
    }
}

/// Final crater diameter (m) from π-group scaling (Collins, Melosh & Marcus 2005, eqs. 21,
/// 22, 27). `angle` is from the horizontal (rad); `g` the target's surface gravity.
pub fn crater_diameter(impactor_radius: f64, impactor_density: f64, target_density: f64, speed: f64, angle: f64, g: f64) -> f64 {
    if impactor_radius <= 0.0 || speed <= 0.0 || g <= 0.0 {
        return 0.0;
    }
    let l = 2.0 * impactor_radius;
    let transient = 1.161 * (impactor_density / target_density).dcbrt() * l.dpowf(0.78) * speed.dpowf(0.44) * g.dpowf(-0.22) * angle.dsin().max(0.0).dcbrt();
    // Simple-to-complex transition: 3.2 km on Earth, inversely proportional to gravity.
    let d_c_km = 3.2 * 9.80665 / g;
    let simple_km = 1.25 * transient / 1000.0;
    if simple_km < d_c_km {
        simple_km * 1000.0
    } else {
        1.17 * (transient / 1000.0).dpowf(1.13) / d_c_km.dpowf(0.13) * 1000.0
    }
}

/// Radius (m) of ≈5 psi peak overpressure: severe structural damage, ~50 % lethality
/// (Glasstone & Dolan 1977 cube-root scaling, surface burst ≈ 4 km at 1 Mt).
pub fn blast_radius(energy_j: f64) -> f64 {
    4000.0 * (energy_j / MEGATON_TNT_J).max(0.0).dcbrt()
}

/// log-linear interpolation of `y` between (x0, 0) and (x1, y1) in log10(x), capped.
fn log_ramp(x: f64, x0: f64, x1: f64, y1: f64, cap: f64) -> f64 {
    if x <= x0 {
        return 0.0;
    }
    (y1 * (x / x0).dlog10() / (x1 / x0).dlog10()).min(cap)
}

/// Fraction of species lost: 0 at the global threshold, 0.75 at Chicxulub (K–Pg; Schulte
/// et al. 2010), capped at 0.95.
pub fn extinction_severity(energy_mt: f64) -> f64 {
    log_ramp(energy_mt, GLOBAL_THRESHOLD_MT, CHICXULUB_MT, 0.75, 0.95)
}

/// Peak global cooling (K): 0 at the threshold, 26 K at Chicxulub (Brugger et al. 2017),
/// capped at 30 K.
pub fn winter_peak_cooling(energy_mt: f64) -> f64 {
    log_ramp(energy_mt, GLOBAL_THRESHOLD_MT, CHICXULUB_MT, 26.0, 30.0)
}

pub fn classify(energy_j: f64, mass_ratio: f64, has_surface: bool) -> ImpactClass {
    let mt = energy_j / MEGATON_TNT_J;
    if mass_ratio >= GIANT_IMPACT_RATIO {
        ImpactClass::GiantImpact
    } else if !has_surface {
        ImpactClass::Atmospheric
    } else if energy_j >= STERILISING_J {
        ImpactClass::Sterilising
    } else if mt >= 0.3 * CHICXULUB_MT {
        ImpactClass::MassExtinction
    } else if mt >= GLOBAL_THRESHOLD_MT {
        ImpactClass::Global
    } else if mt >= 1.0e3 {
        ImpactClass::Regional
    } else {
        ImpactClass::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chicxulub-class: ~10 km stony impactor at ~20 km/s, 45°, into Earth → ~100+ km crater
    /// and ~10⁸ Mt.
    #[test]
    fn chicxulub_reference() {
        let r: f64 = 5_000.0;
        let rho = 2_600.0;
        let m = rho * 4.0 / 3.0 * std::f64::consts::PI * r.powi(3);
        let v = 20_000.0;
        let e = 0.5 * m * v * v;
        let mt = e / MEGATON_TNT_J;
        assert!((3e7..3e8).contains(&mt), "{mt:e} Mt");
        let d = crater_diameter(r, rho, 2_750.0, v, 45f64.to_radians(), 9.81);
        // Collins et al. give ~100–120 km for these inputs; Chicxulub's ~180 km implies a
        // larger or faster impactor — the inputs, not the scaling, carry that uncertainty.
        assert!((90e3..200e3).contains(&d), "crater {:.0} km", d / 1000.0);
        assert!((extinction_severity(mt) - 0.75).abs() < 0.2);
        assert_eq!(classify(e, m / 5.97e24, true), ImpactClass::MassExtinction);
    }

    /// Meteor Crater (Barringer): ~50 m iron at ~12.8 km/s → ~1.2 km crater, ~10 Mt.
    #[test]
    fn meteor_crater_reference() {
        let r: f64 = 25.0;
        let rho = 7_800.0;
        let d = crater_diameter(r, rho, 2_500.0, 12_800.0, 45f64.to_radians(), 9.81);
        assert!((0.8e3..1.8e3).contains(&d), "crater {d:.0} m");
        let m = rho * 4.0 / 3.0 * std::f64::consts::PI * r.powi(3);
        let mt = 0.5 * m * 12_800f64.powi(2) / MEGATON_TNT_J;
        assert!((2.0..20.0).contains(&mt), "{mt} Mt");
        assert_eq!(extinction_severity(mt), 0.0);
        assert_eq!(classify(mt * MEGATON_TNT_J, 1e-15, true), ImpactClass::Local);
    }

    #[test]
    fn winter_decays() {
        let w = ImpactWinter { start: 0.0, peak_k: 26.0, efold_years: ImpactWinter::EFOLD_YEARS };
        assert!((w.cooling_at(0.0) - 26.0).abs() < 1e-9);
        assert!(w.cooling_at(3.0 * SECONDS_PER_YEAR) < 10.0);
        assert!(w.expired(40.0 * SECONDS_PER_YEAR));
    }
}
