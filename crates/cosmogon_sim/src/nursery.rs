//! Stellar nurseries: giant clouds of gas where new stars are born.
//!
//! A nursery holds a reservoir of gas and turns it into stars at a steady rate; each new
//! star is a full star system (masses from the initial mass function, planets and all)
//! born at age zero somewhere inside the cloud. Only the notable stars are modelled — one
//! system stands for the brighter members of a young cluster. As the gas is used up and
//! blown away by the new stars, the nebula fades and star formation stops. Massive
//! newborns live only a few million years and die as supernovae, as in real clusters.
//!
//! The real Orion Nebula is placed at its catalogued position with the four Trapezium
//! stars (θ¹ Orionis A–D) that light it; stars it forms later are predictions.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::generate::generate_system;
use crate::astro::{Star, StarSystem, LIGHT_YEAR, SOLAR_MASS};
use crate::history::{Category, Event};
use crate::rng::Rng;
use crate::time::{SECONDS_PER_MYR, SECONDS_PER_YEAR};
use crate::universe::Universe;
use crate::Vec3d;

/// Most systems one nursery will add (keeps universes small).
pub const MAX_FORMED: u32 = 24;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Nursery {
    pub name: String,
    /// Centre (m, galactic frame).
    pub position: Vec3d,
    pub radius_ly: f64,
    /// Gas left (M☉) and at the start.
    pub gas: f64,
    pub gas0: f64,
    /// Mean time between notable new stars (s).
    pub interval: f64,
    pub next_star: f64,
    pub formed: u32,
    pub seed: u64,
}

impl Nursery {
    /// Glow of the nebula 0..1: bright while it has gas, fading as it is used up.
    pub fn brightness(&self) -> f64 {
        (self.gas / self.gas0.max(1e-9)).clamp(0.0, 1.0).sqrt()
    }

    pub fn active(&self) -> bool {
        self.formed < MAX_FORMED && self.gas > 0.05 * self.gas0
    }
}

/// J2000 equatorial (RA, Dec in degrees) to a unit vector in the simulation's ecliptic frame.
pub fn equatorial_dir(ra_deg: f64, dec_deg: f64) -> Vec3d {
    let (ra, dec) = (ra_deg.to_radians(), dec_deg.to_radians());
    let eq = Vec3d::new(dec.dcos() * ra.dcos(), dec.dcos() * ra.dsin(), dec.dsin());
    let e = 23.439_291f64.to_radians();
    Vec3d::new(eq.x, eq.y * e.dcos() + eq.z * e.dsin(), -eq.y * e.dsin() + eq.z * e.dcos())
}

/// A bare star (no invented planets) — for real stars.
fn bare_system(id: u32, name: &str, mass: f64, position: Vec3d, t: f64, age_myr: f64) -> StarSystem {
    StarSystem {
        id,
        name: name.into(),
        position,
        star: Star::from_mass(name.into(), mass, 0.0, t - age_myr * SECONDS_PER_MYR),
        companion: None,
        bodies: Vec::new(),
        belts: Vec::new(),
        dynamics: None,
        pending_contacts: Vec::new(),
        nebulae: Vec::new(),
    }
}

impl Universe {
    /// The Orion Nebula (M42): 1 344 ly towards Orion's sword, ~12 ly across, a few
    /// thousand solar masses of gas, lit by the Trapezium.
    pub fn add_orion_nebula(&mut self) {
        let t = self.time;
        let dir = equatorial_dir(83.822, -5.391);
        let centre = self.systems.first().map(|s| s.position).unwrap_or(Vec3d::ZERO) + dir * 1_344.0 * LIGHT_YEAR;
        // θ¹ Orionis A–D: masses after Simón-Díaz et al. (2006) / Kraus et al. (2009).
        let side = dir.cross(Vec3d::new(0.0, 0.0, 1.0)).normalize();
        let up = side.cross(dir).normalize();
        for (k, (name, mass)) in [("Theta1 Orionis C", 33.0), ("Theta1 Orionis A", 14.0), ("Theta1 Orionis D", 16.0), ("Theta1 Orionis B", 7.0)].iter().enumerate() {
            let a = k as f64 * 1.7;
            let pos = centre + (side * a.dcos() + up * a.dsin()) * 0.03 * LIGHT_YEAR;
            let id = self.systems.len() as u32;
            self.systems.push(bare_system(id, name, *mass, pos, t, 0.3));
        }
        self.nurseries.push(Nursery {
            name: "Orion Nebula".into(),
            position: centre,
            radius_ly: 12.0,
            gas: 2_000.0,
            gas0: 2_000.0,
            interval: 0.08 * SECONDS_PER_MYR,
            next_star: t + 0.08 * SECONDS_PER_MYR,
            formed: 0,
            seed: 0x0421,
        });
    }

    /// A star-forming cloud placed by the user or an experiment.
    pub fn add_nursery(&mut self, name: &str, position: Vec3d, radius_ly: f64, gas_msun: f64, interval_years: f64) {
        let t = self.time;
        let seed = 0x5EED ^ self.nurseries.len() as u64;
        self.nurseries.push(Nursery { name: name.into(), position, radius_ly, gas: gas_msun, gas0: gas_msun, interval: interval_years * SECONDS_PER_YEAR, next_star: t + interval_years * SECONDS_PER_YEAR * 0.25, formed: 0, seed });
    }

    /// Earliest time a nursery forms its next star.
    pub(crate) fn next_nursery_time(&self) -> f64 {
        self.nurseries.iter().filter(|n| n.active()).map(|n| n.next_star).fold(f64::INFINITY, f64::min)
    }

    /// Form any stars that are due by `t`.
    pub(crate) fn step_nurseries(&mut self, t: f64) {
        for ni in 0..self.nurseries.len() {
            while self.nurseries[ni].active() && self.nurseries[ni].next_star <= t {
                let n = &self.nurseries[ni];
                let born = n.next_star;
                let mut rng = Rng::stream(self.settings.seed ^ n.seed, crate::rng::domain::STAR, &[n.formed as u64, 0x6E75]);
                // Somewhere in the cloud, concentrated towards the middle.
                let (u, v, w) = (rng.f64(), rng.f64(), rng.f64());
                let r = n.radius_ly * 0.8 * u * LIGHT_YEAR;
                let th = std::f64::consts::TAU * v;
                let ph = (2.0 * w - 1.0).dacos();
                let pos = n.position + Vec3d::new(r * ph.dsin() * th.dcos(), r * ph.dsin() * th.dsin(), r * ph.dcos());
                let id = self.systems.len() as u32;
                let mut sys = generate_system(&self.settings, id, pos, false);
                let (name, mass, z) = (sys.star.name.clone(), sys.star.mass, sys.star.metallicity);
                sys.star = Star::from_mass(name.clone(), mass, z, born);
                if let Some(c) = sys.companion.as_mut() {
                    c.star = Star::from_mass(c.star.name.clone(), c.star.mass, z, born);
                }
                let spectral = sys.star.spectral_type(born);
                self.systems.push(sys);
                // Gas used: the star plus what its birth disperses (star-formation efficiency ~30 %).
                let n = &mut self.nurseries[ni];
                n.gas = (n.gas - mass / 0.3).max(0.0);
                n.formed += 1;
                n.next_star = born + n.interval * (0.3 + 1.4 * rng.f64());
                let nursery = n.name.clone();
                let left = n.gas / n.gas0;
                self.history.push(Event {
                    time: born,
                    category: Category::Astronomy,
                    importance: if mass > 8.0 { 5 } else { 4 },
                    title: format!("A new star ignites in the {nursery}: {name}"),
                    detail: format!("A {mass:.2} M☉ {spectral} star begins fusing hydrogen{}. {:.0}% of the cloud's gas remains.", if self.systems[id as usize].bodies.is_empty() { "" } else { ", with planets forming around it" }, left * 100.0),
                    system: Some(id),
                    body: None,
                    civ: None,
                });
                if !self.nurseries[ni].active() {
                    let nursery = self.nurseries[ni].name.clone();
                    self.history.push(Event { time: born, category: Category::Astronomy, importance: 5, title: format!("Star formation ends in the {nursery}"), detail: "Its gas is used up or blown away by the young stars' light and winds; a young open cluster remains.".into(), system: Some(id), body: None, civ: None });
                }
            }
        }
        let _ = SOLAR_MASS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scenario, UniverseSettings};

    #[test]
    fn orion_is_towards_orion_at_the_right_distance() {
        let u = Universe::new(UniverseSettings { seed: 1, scenario: Scenario::SolarSystemLab, ..Default::default() });
        let n = u.nurseries.iter().find(|n| n.name == "Orion Nebula").expect("Orion Nebula");
        let d = (n.position - u.systems[0].position).length() / LIGHT_YEAR;
        assert!((d - 1344.0).abs() < 1.0, "{d}");
        // Orion's sword is just south of the ecliptic (ecliptic latitude ≈ −28.6°).
        let dir = (n.position - u.systems[0].position).normalize();
        let lat = dir.z.asin().to_degrees();
        assert!((lat + 28.6).abs() < 0.5, "ecliptic latitude {lat}");
        assert!(u.systems.iter().any(|s| s.star.name == "Theta1 Orionis C" && s.bodies.is_empty()));
    }

    #[test]
    fn a_nursery_forms_young_stars_and_then_stops() {
        // Formation alone (advancing the whole N-body Solar System for millions of years
        // would take far too long in a test).
        let mut u = Universe::new(UniverseSettings { seed: 1, scenario: Scenario::SolarSystemLab, ..Default::default() });
        let n0 = u.systems.len();
        u.time += 3.0 * SECONDS_PER_MYR;
        u.step_nurseries(u.time);
        let n = &u.nurseries[0];
        assert!(n.formed >= 8, "formed {}", n.formed);
        assert!(u.systems.len() > n0);
        // The newest star is young.
        let newest = u.systems.last().unwrap();
        assert!(newest.star.age(u.time) < 1.0 * SECONDS_PER_MYR, "age {} Myr", newest.star.age(u.time) / SECONDS_PER_MYR);
        assert!(u.history.events.iter().any(|e| e.title.starts_with("A new star ignites in the Orion Nebula")));
        u.time += 20.0 * SECONDS_PER_MYR;
        u.step_nurseries(u.time);
        assert!(!u.nurseries[0].active());
        assert!(u.nurseries[0].formed <= MAX_FORMED);
    }
}
