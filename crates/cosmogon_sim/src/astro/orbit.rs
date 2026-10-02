//! Keplerian orbits evaluated analytically.
//!
//! Orbits are never integrated step by step: a position at any time — past or future — is
//! a pure function of the elements and the time. That keeps them exact at any simulation
//! speed and trivially reversible. (Higher-accuracy N-body integration for selected
//! systems is planned; see ROADMAP.md.)
//!
//! Frame: the orbital reference plane is the XY plane with +Z as the reference pole
//! (ecliptic-like). The renderer maps sim (x, y, z) to render (x, z, -y).

use cosmogon_physics::kepler;
use serde::{Deserialize, Serialize};

use crate::Vec3d;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    /// Semi-major axis (m).
    pub a: f64,
    /// Eccentricity (0 ≤ e < 1).
    pub e: f64,
    /// Inclination (rad).
    pub i: f64,
    /// Longitude of the ascending node Ω (rad).
    pub node: f64,
    /// Argument of periapsis ω (rad).
    pub peri: f64,
    /// Mean anomaly at t = 0 (J2000) (rad).
    pub m0: f64,
}

impl Orbit {
    pub fn circular(a: f64, phase: f64) -> Self {
        Self { a, e: 0.0, i: 0.0, node: 0.0, peri: 0.0, m0: phase }
    }

    /// Orbital period (s) around a primary with gravitational parameter `mu` (m³/s²).
    pub fn period(&self, mu: f64) -> f64 {
        kepler::orbital_period(self.a, mu)
    }

    pub fn mean_anomaly(&self, mu: f64, t: f64) -> f64 {
        let n = (mu / (self.a * self.a * self.a)).sqrt();
        (self.m0 + n * t).rem_euclid(std::f64::consts::TAU)
    }

    /// Position relative to the primary at time `t` (s since J2000).
    pub fn position(&self, mu: f64, t: f64) -> Vec3d {
        let m = self.mean_anomaly(mu, t);
        let ecc = kepler::solve_kepler(m, self.e, 1e-12, 30);
        self.position_from_eccentric_anomaly(ecc)
    }

    fn position_from_eccentric_anomaly(&self, ecc_anomaly: f64) -> Vec3d {
        // Perifocal coordinates straight from E (avoids the true-anomaly round trip).
        let (sin_e, cos_e) = ecc_anomaly.sin_cos();
        let px = self.a * (cos_e - self.e);
        let py = self.a * (1.0 - self.e * self.e).sqrt() * sin_e;
        self.rotate(px, py)
    }

    fn rotate(&self, px: f64, py: f64) -> Vec3d {
        let (so, co) = self.node.sin_cos();
        let (sw, cw) = self.peri.sin_cos();
        let (si, ci) = self.i.sin_cos();
        Vec3d::new(
            (co * cw - so * sw * ci) * px + (-co * sw - so * cw * ci) * py,
            (so * cw + co * sw * ci) * px + (-so * sw + co * cw * ci) * py,
            (sw * si) * px + (cw * si) * py,
        )
    }

    /// Points along one full orbit (for drawing), relative to the primary.
    pub fn path(&self, segments: usize) -> Vec<Vec3d> {
        (0..=segments)
            .map(|k| self.position_from_eccentric_anomaly(k as f64 / segments as f64 * std::f64::consts::TAU))
            .collect()
    }

    pub fn periapsis(&self) -> f64 {
        self.a * (1.0 - self.e)
    }

    pub fn apoapsis(&self) -> f64 {
        self.a * (1.0 + self.e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MU_SUN: f64 = 1.327_124_400_18e20;

    #[test]
    fn position_is_periodic() {
        let o = Orbit { a: 1.5e11, e: 0.2, i: 0.3, node: 1.0, peri: 2.0, m0: 0.5 };
        let p = o.period(MU_SUN);
        let a = o.position(MU_SUN, 1234.0);
        let b = o.position(MU_SUN, 1234.0 + 10.0 * p);
        assert!((a - b).length() / o.a < 1e-9, "{}", (a - b).length());
    }

    #[test]
    fn radius_stays_between_apsides() {
        let o = Orbit { a: 2.0e11, e: 0.4, i: 0.1, node: 0.2, peri: 0.3, m0: 0.0 };
        let p = o.period(MU_SUN);
        for k in 0..200 {
            let r = o.position(MU_SUN, p * k as f64 / 200.0).length();
            assert!(r >= o.periapsis() * (1.0 - 1e-9) && r <= o.apoapsis() * (1.0 + 1e-9));
        }
    }

    #[test]
    fn matches_physics_crate_state_vectors() {
        let o = Orbit { a: 1.0e11, e: 0.1, i: 0.2, node: 0.7, peri: 1.1, m0: 0.4 };
        let t = 3.0e6;
        let (expected, _) = kepler::orbital_state_vectors(o.a, o.e, o.i, o.node, o.peri, o.mean_anomaly(MU_SUN, t), MU_SUN);
        assert!((o.position(MU_SUN, t) - expected).length() / o.a < 1e-9);
    }

    #[test]
    fn earth_period_is_one_year() {
        let o = Orbit::circular(super::super::AU, 0.0);
        let years = o.period(MU_SUN) / crate::time::SECONDS_PER_YEAR;
        assert!((years - 1.0).abs() < 1e-3, "{years}");
    }
}
