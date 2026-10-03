//! The life and death of stars, and what it does to their planets.
//!
//! A scheduled task watches every star:
//! * **Red giant**: the swelling star engulfs any world whose orbit dips inside it.
//! * **Gentle end** (< 8 M☉): the envelope drifts away as a planetary nebula; the slow
//!   mass loss widens surviving orbits adiabatically (a ∝ 1/M, orbits stay circular) and a
//!   white dwarf remains.
//! * **Core collapse** (≥ 8 M☉): a supernova. Its ~10⁴⁴ J of ejecta vaporise worlds that
//!   intercept more than their gravitational binding energy and sterilise and strip the
//!   rest; the sudden mass loss leaves orbits unbound if more than half the mass goes; a
//!   neutron star or black hole remains. Radiation reaches other star systems at the speed
//!   of light; within ~30 light-years it drives extinctions (ozone loss: Gehrels et al. 2003).
//! Every transition happens on the task's fixed time grid, so outcomes don't depend on
//! frame rate or speed.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::star::{StarKind, StellarPhase};
use crate::astro::{Orbit, Removal, RemovalCause, StarSystem, G, LIGHT_YEAR, SPEED_OF_LIGHT};
use crate::history::{Category, Event};
use crate::life::Stage;
use crate::time::SECONDS_PER_YEAR;
use crate::Universe;

/// Kinetic energy of core-collapse supernova ejecta (J).
pub const SUPERNOVA_ENERGY: f64 = 1.0e44;
/// Radiation fluence (J/m²) at which a living world's biosphere starts to suffer, and at
/// which it is wiped out (ozone destruction and surface radiation; order-of-magnitude).
pub const FLUENCE_HARM: f64 = 1.0e7;
pub const FLUENCE_STERILE: f64 = 1.0e10;
/// Fraction of the explosion energy emitted as ionising radiation (X/γ, cosmic rays).
const RADIATION_FRACTION: f64 = 0.01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NebulaKind {
    /// Gas shed by a dying low-mass star, lit by the hot white dwarf.
    Planetary,
    /// Debris of a supernova sweeping up interstellar gas.
    SupernovaRemnant,
}

/// An expanding shell of gas around a star system (rendered; no gravity).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Nebula {
    pub kind: NebulaKind,
    pub born: f64,
    /// Expansion speed (m/s).
    pub speed: f64,
    /// Ejected mass (M☉).
    pub mass: f64,
    pub seed: u64,
}

impl Nebula {
    /// Radius (m) at `t`: free expansion, then (for remnants) the Sedov–Taylor slowdown
    /// r ∝ t^0.4 after a few centuries.
    pub fn radius(&self, t: f64) -> f64 {
        let age = (t - self.born).max(0.0);
        match self.kind {
            NebulaKind::Planetary => self.speed * age,
            NebulaKind::SupernovaRemnant => {
                let t0 = 300.0 * SECONDS_PER_YEAR;
                if age < t0 {
                    self.speed * age
                } else {
                    self.speed * t0 * (age / t0).dpowf(0.4)
                }
            }
        }
    }
    /// How visible it still is (1 young → 0 dispersed).
    pub fn brightness(&self, t: f64) -> f64 {
        let age = (t - self.born).max(0.0) / SECONDS_PER_YEAR;
        let life = match self.kind {
            NebulaKind::Planetary => 30_000.0,
            NebulaKind::SupernovaRemnant => 100_000.0,
        };
        (1.0 - age / life).clamp(0.0, 1.0)
    }
    pub fn label(&self) -> &'static str {
        match self.kind {
            NebulaKind::Planetary => "Planetary nebula",
            NebulaKind::SupernovaRemnant => "Supernova remnant",
        }
    }
}

/// A supernova's radiation front travelling outwards at light speed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Blast {
    pub system: u32,
    pub time: f64,
    pub energy: f64,
    /// Systems already reached.
    pub reached: Vec<u32>,
}

/// Index of the stellar task in the scheduler.
pub(crate) const TASK_STARS: usize = 3;
const STAR_PERIOD: f64 = SECONDS_PER_YEAR;
/// While a star is a giant, check engulfment this often (years).
const GIANT_CHECK_YEARS: u64 = 1_000;

impl Universe {
    /// Make sure the stellar task exists (older saves and new universes).
    pub fn ensure_star_task(&mut self) {
        while self.scheduler.tasks.len() < TASK_STARS {
            self.scheduler.add("placeholder", self.time, crate::scheduler::NEVER);
        }
        if self.scheduler.tasks.len() == TASK_STARS {
            self.scheduler.add("stars", self.time, STAR_PERIOD);
        }
        // Reschedule from now (an edit may have brought a star's end closer).
        let next = self.next_stellar_index();
        self.scheduler.tasks[TASK_STARS].steps = next;
    }

    /// The stellar task's next useful step: giant onsets, ends of life, blast arrivals.
    fn next_stellar_index(&self) -> u64 {
        let k_now = self.scheduler.index_at_or_after(TASK_STARS, self.time);
        let mut next = f64::INFINITY;
        for sys in &self.systems {
            let s = &sys.star;
            if s.kind != StarKind::Normal {
                continue;
            }
            match s.phase(self.time) {
                StellarPhase::Giant => return k_now + 1,
                _ => next = next.min(s.giant_onset()).min(s.end_of_life()),
            }
        }
        for b in &self.blasts {
            let origin = self.systems[b.system as usize].position;
            for sys in self.systems.iter().filter(|x| !b.reached.contains(&x.id)) {
                next = next.min(b.time + (sys.position - origin).length() / SPEED_OF_LIGHT);
            }
        }
        let now = self.scheduler.index_at_or_after(TASK_STARS, self.time);
        if !next.is_finite() {
            return u64::MAX / 4;
        }
        self.scheduler.index_at_or_after(TASK_STARS, next.max(self.time)).max(now)
    }

    /// Run at the stellar task's step `k` (time `t`). Returns the next step index.
    pub(crate) fn step_stars(&mut self, t: f64, k: u64) -> u64 {
        for s in 0..self.systems.len() {
            let star = &self.systems[s].star;
            if star.kind != StarKind::Normal {
                continue;
            }
            match star.phase(t) {
                StellarPhase::Giant => self.engulf(s, t),
                p if p.is_remnant() => self.end_of_life(s, t),
                _ => {}
            }
        }
        self.propagate_blasts(t);
        let giant = self.systems.iter().any(|x| x.star.kind == StarKind::Normal && x.star.phase(t) == StellarPhase::Giant);
        if giant {
            k + GIANT_CHECK_YEARS
        } else {
            self.next_stellar_index().max(k + 1)
        }
    }

    /// Distance (m) from the star at which `body` comes closest now.
    fn closest_approach(sys: &StarSystem, body: usize, t: f64) -> f64 {
        if sys.dynamics.is_some() {
            (sys.body_local_position(body, t) - sys.star_local_position(t)).length()
        } else {
            let top = sys.top_level(body);
            sys.bodies[top].orbit.a * (1.0 - sys.bodies[top].orbit.e)
        }
    }

    fn engulf(&mut self, s: usize, t: f64) {
        let r_star = self.systems[s].star.current_radius(t);
        let star_name = self.systems[s].star.name.clone();
        let victims: Vec<usize> = (0..self.systems[s].bodies.len())
            .filter(|&j| self.systems[s].bodies[j].exists() && self.systems[s].bodies[j].parent.is_none())
            .filter(|&j| Self::closest_approach(&self.systems[s], j, t) < r_star + self.systems[s].bodies[j].radius)
            .collect();
        for j in victims {
            let name = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::MergedInto(None), &format!("was engulfed by the red giant {star_name}"));
            self.history.push(Event { time: t, category: Category::Astronomy, importance: 5, title: format!("{name} engulfed by {star_name}"), detail: format!("The swelling red giant, now {:.2} AU across in radius, swallows the planet.", r_star / crate::astro::AU), system: Some(s as u32), body: Some(j as u32), civ: None });
        }
    }

    /// Remove a body and its moons (they share its fate).
    fn remove_with_moons(&mut self, s: usize, j: usize, t: f64, cause: RemovalCause, how: &str) {
        let mut doomed = vec![j];
        let mut k = 0;
        while k < doomed.len() {
            let p = doomed[k];
            doomed.extend(self.systems[s].moons_of(p).collect::<Vec<_>>());
            k += 1;
        }
        for &b in &doomed {
            let sys = &mut self.systems[s];
            if sys.bodies[b].removed.is_some() {
                continue;
            }
            sys.bodies[b].removed = Some(Removal { time: t, cause });
            if let Some(d) = sys.dynamics.as_mut() {
                if let Some(slot) = d.bodies.get_mut(b) {
                    *slot = None;
                }
            }
            self.on_body_destroyed(s, b, t, how);
        }
    }

    fn end_of_life(&mut self, s: usize, t: f64) {
        let sys = &mut self.systems[s];
        if sys.dynamics.is_some() {
            sys.sync_to(t);
        }
        // States of every top-level body before the change (moons ride along).
        let tops: Vec<usize> = (0..sys.bodies.len()).filter(|&j| sys.bodies[j].exists() && sys.bodies[j].parent.is_none()).collect();
        let before: Vec<(usize, crate::astro::dynamics::State)> = tops.iter().map(|&j| (j, sys.body_state(j, t))).collect();
        let star_state = crate::astro::dynamics::State { pos: sys.star_local_position(t), vel: sys.dynamics.as_ref().map(|d| d.star.vel).unwrap_or_default() };
        let m0 = sys.star.mass;
        let name = sys.star.name.clone();
        let fate = sys.star.become_remnant(t);
        let m1 = fate.mass;
        let seed = crate::rng::mix(s as u64, t.to_bits());

        if !fate.supernova {
            // Slow wind: orbits expand adiabatically (a ∝ 1/M), keeping their shape.
            let k = m0 / m1;
            for (j, st) in &before {
                let rel_p = (st.pos - star_state.pos) * k;
                let rel_v = (st.vel - star_state.vel) / k;
                Self::set_top_state(sys, *j, star_state.pos + rel_p, star_state.vel + rel_v, t);
            }
            sys.nebulae.push(Nebula { kind: NebulaKind::Planetary, born: t, speed: 20_000.0, mass: m0 - m1, seed });
            self.history.push(Event {
                time: t,
                category: Category::Astronomy,
                importance: 5,
                title: format!("{name} becomes a white dwarf"),
                detail: format!("It sheds {:.2} M☉ as a glowing planetary nebula; a {:.2} M☉ white dwarf the size of Earth remains. Surviving orbits widen {k:.2}×.", m0 - m1, m1),
                system: Some(s as u32),
                body: None,
                civ: None,
            });
            self.refresh_system(s, t);
            return;
        }

        // Core collapse.
        let remnant = if fate.kind == StarKind::BlackHole { "black hole" } else { "neutron star" };
        self.history.push(Event {
            time: t,
            category: Category::Astronomy,
            importance: 5,
            title: format!("Supernova! {name} explodes"),
            detail: format!("The core of the {m0:.1} M☉ star collapses into a {m1:.1} M☉ {remnant}; ~10⁴⁴ J of debris races out at 10 000 km/s."),
            system: Some(s as u32),
            body: None,
            civ: None,
        });
        let sys = &mut self.systems[s];
        sys.nebulae.push(Nebula { kind: NebulaKind::SupernovaRemnant, born: t, speed: 1.0e7, mass: m0 - m1, seed });
        // Worlds: vaporised if they intercept more than their binding energy.
        let mut vaporised = Vec::new();
        let mut scorched = Vec::new();
        for j in 0..sys.bodies.len() {
            let b = &sys.bodies[j];
            if !b.exists() {
                continue;
            }
            let d = (sys.body_local_position(j, t) - star_state.pos).length().max(b.radius);
            let hit = SUPERNOVA_ENERGY * b.radius * b.radius / (4.0 * d * d);
            let binding = 0.6 * G * b.mass * b.mass / b.radius;
            if hit > binding {
                vaporised.push(j);
            } else {
                scorched.push((j, hit / binding));
            }
        }
        for j in vaporised {
            let n = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::Vaporised, "was vaporised by a supernova");
            self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("{n} vaporised"), detail: "The supernova blast delivers more energy than holds the world together.".into(), system: Some(s as u32), body: Some(j as u32), civ: None });
        }
        for (j, frac) in scorched {
            if !self.systems[s].bodies[j].exists() {
                continue;
            }
            let b = &mut self.systems[s].bodies[j];
            // Atmospheres and oceans are blown off; surfaces melt where the blast is strong.
            b.atmosphere.pressure_bar *= (1.0 - frac.sqrt().min(1.0)).max(0.0) * 0.01;
            b.hydro.water_inventory *= 0.01;
            b.hydro.ocean_fraction = 0.0;
            self.sterilise_world(s, j, t, "the supernova's blast and radiation");
            self.refresh_body_environment(s, j, t);
        }
        // Sudden mass loss: bound or not now depends on the new, lighter remnant.
        let sys = &mut self.systems[s];
        let mut escaped = Vec::new();
        if sys.dynamics.is_none() {
            for (j, st) in &before {
                if !sys.bodies[*j].exists() {
                    continue;
                }
                let mu = sys.star.mu() + sys.bodies[*j].mu();
                match Orbit::from_state(st.pos - star_state.pos, st.vel - star_state.vel, mu, t) {
                    Some(o) if o.e < 1.0 => sys.bodies[*j].orbit = o,
                    _ => escaped.push(*j),
                }
            }
        }
        for j in escaped {
            let n = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::Ejected, "was flung out of the system");
            self.history.push(Event { time: t, category: Category::Astronomy, importance: 4, title: format!("{n} escapes"), detail: "With most of the star's mass gone, its orbit is no longer bound: it drifts into interstellar space.".into(), system: Some(s as u32), body: Some(j as u32), civ: None });
        }
        self.blasts.push(Blast { system: s as u32, time: t, energy: SUPERNOVA_ENERGY, reached: vec![s as u32] });
        self.refresh_system(s, t);
    }

    fn set_top_state(sys: &mut StarSystem, j: usize, pos: crate::Vec3d, vel: crate::Vec3d, t: f64) {
        match sys.dynamics.as_mut() {
            Some(d) => {
                if let Some(Some(st)) = d.bodies.get_mut(j) {
                    *st = crate::astro::dynamics::State { pos, vel };
                }
            }
            None => {
                let mu = sys.star.mu() + sys.bodies[j].mu();
                let star = crate::Vec3d::ZERO;
                if let Some(o) = Orbit::from_state(pos - star, vel, mu, t) {
                    sys.bodies[j].orbit = o;
                }
            }
        }
    }

    /// Climate and habitability of every world after the star changed.
    fn refresh_system(&mut self, s: usize, t: f64) {
        for j in 0..self.systems[s].bodies.len() {
            if self.systems[s].bodies[j].exists() {
                self.refresh_body_environment(s, j, t);
            }
        }
    }

    fn sterilise_world(&mut self, s: usize, j: usize, t: f64, why: &str) {
        let name = self.systems[s].bodies[j].name.clone();
        let mut lost = false;
        if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == s as u32 && x.body == j as u32) {
            if bio.stage > Stage::Sterile {
                lost = true;
                bio.stage = Stage::Sterile;
                bio.stage_since = t;
                bio.biomass = 0.0;
                bio.biodiversity = 0.0;
                bio.photosynthesis_since = None;
                bio.civilization_present = false;
            }
        }
        if lost {
            self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("{name} sterilised"), detail: format!("All life is lost: {why}."), system: Some(s as u32), body: Some(j as u32), civ: None });
        }
        self.on_body_destroyed(s, j, t, &format!("was sterilised by {why}"));
    }

    /// Radiation fronts reaching other systems.
    fn propagate_blasts(&mut self, t: f64) {
        for bi in 0..self.blasts.len() {
            let (origin, t0, energy) = {
                let b = &self.blasts[bi];
                (self.systems[b.system as usize].position, b.time, b.energy)
            };
            let source = self.systems[self.blasts[bi].system as usize].star.name.clone();
            for s in 0..self.systems.len() {
                if self.blasts[bi].reached.contains(&(s as u32)) {
                    continue;
                }
                let dist = (self.systems[s].position - origin).length();
                if dist > SPEED_OF_LIGHT * (t - t0) {
                    continue;
                }
                self.blasts[bi].reached.push(s as u32);
                let fluence = energy * RADIATION_FRACTION / (4.0 * std::f64::consts::PI * dist * dist);
                if fluence < FLUENCE_HARM {
                    continue;
                }
                let severity = ((fluence / FLUENCE_HARM).log10() / (FLUENCE_STERILE / FLUENCE_HARM).log10()).clamp(0.0, 1.0);
                for j in 0..self.systems[s].bodies.len() {
                    if !self.systems[s].bodies[j].exists() {
                        continue;
                    }
                    let Some(bi2) = self.biospheres.iter().position(|x| x.system == s as u32 && x.body == j as u32 && x.stage > Stage::Sterile) else { continue };
                    let name = self.systems[s].bodies[j].name.clone();
                    if severity >= 1.0 {
                        self.sterilise_world(s, j, t, &format!("radiation from the supernova of {source}"));
                        continue;
                    }
                    let bio = &mut self.biospheres[bi2];
                    bio.biodiversity *= 1.0 - 0.8 * severity;
                    bio.biomass *= 1.0 - 0.5 * severity;
                    for c in self.civs.iter_mut().filter(|c| c.system == s as u32 && c.body == j as u32 && c.is_alive()) {
                        c.population *= 1.0 - 0.3 * severity;
                        c.pressures.food = c.pressures.food.max(severity);
                        c.pressures.contact = 1.0;
                    }
                    self.history.push(Event {
                        time: t,
                        category: Category::Life,
                        importance: 5,
                        title: format!("Supernova radiation reaches {name}"),
                        detail: format!("The explosion of {source}, {:.1} light-years away, strips the ozone layer: {:.0}% of species are lost.", dist / LIGHT_YEAR, 80.0 * severity),
                        system: Some(s as u32),
                        body: Some(j as u32),
                        civ: None,
                    });
                }
            }
        }
        self.blasts.retain(|b| b.reached.len() < self.systems.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::star::Star;
    use crate::time::SECONDS_PER_GYR;
    use crate::{BodyRef, Scenario, UniverseSettings};

    fn lab_kepler() -> Universe {
        let mut u = Universe::new(UniverseSettings { scenario: Scenario::Sol, seed: 3, ..Default::default() });
        u.civs.clear();
        u
    }

    #[test]
    fn the_sun_swallows_mercury_and_venus_and_leaves_a_white_dwarf() {
        let mut u = lab_kepler();
        let a_mars = u.systems[0].bodies[u.systems[0].find_body("Mars").unwrap()].orbit.a;
        let end = u.systems[0].star.end_of_life();
        u.advance_to(end + 2.0 * SECONDS_PER_YEAR * 1000.0, None, None);
        let sys = &u.systems[0];
        assert_eq!(sys.star.kind, StarKind::WhiteDwarf);
        let gone = |n: &str| !sys.bodies[sys.find_body(n).unwrap()].exists();
        assert!(gone("Mercury") && gone("Venus"), "inner planets engulfed");
        let mars = &sys.bodies[sys.find_body("Mars").unwrap()];
        assert!(mars.exists());
        // Mass loss from 1 to ~0.5 M☉ roughly doubles surviving orbits.
        let k = mars.orbit.a / a_mars;
        assert!((1.8..2.2).contains(&k), "Mars orbit widened {k}×");
        assert!(sys.nebulae.iter().any(|n| n.kind == NebulaKind::Planetary));
        // Earth sits right at the giant's maximum radius; whatever happened, it's lifeless.
        let e = BodyRef { system: 0, body: sys.find_body("Earth").unwrap() as u32 };
        assert!(u.biosphere(e).is_none_or(|b| b.stage == Stage::Sterile));
    }

    #[test]
    fn a_massive_star_explodes_and_leaves_a_black_hole() {
        let mut u = lab_kepler();
        let life = Star::from_mass("x".into(), 30.0, 0.0, 0.0).lifetime;
        let t = u.time;
        // A 30 M☉ star at the very end of its life in place of the Sun.
        u.systems[0].star = Star::from_mass("Doomed".into(), 30.0, 0.0, t - life * 1.1199);
        u.ensure_star_task();
        u.advance_by(2000.0 * SECONDS_PER_YEAR);
        let sys = &u.systems[0];
        assert_eq!(sys.star.kind, StarKind::BlackHole);
        // The red supergiant (~1000 R☉ ≈ 4.7 AU) swallows the inner system before it explodes.
        let earth = &sys.bodies[sys.find_body("Earth").unwrap()];
        assert!(matches!(earth.removed.unwrap().cause, RemovalCause::MergedInto(None)));
        // 30 → ~9.5 M☉ in an instant: more than half the mass gone, so Jupiter is unbound.
        let jupiter = &sys.bodies[sys.find_body("Jupiter").unwrap()];
        assert!(matches!(jupiter.removed.map(|r| r.cause), Some(RemovalCause::Ejected)), "{:?}", jupiter.removed);
        assert!(u.history.events.iter().any(|e| e.title.starts_with("Supernova!")));
        let _ = SECONDS_PER_GYR;
    }

    #[test]
    fn nebulae_expand_and_fade() {
        let n = Nebula { kind: NebulaKind::SupernovaRemnant, born: 0.0, speed: 1e7, mass: 10.0, seed: 1 };
        let yr = SECONDS_PER_YEAR;
        assert!(n.radius(1000.0 * yr) > n.radius(100.0 * yr));
        // Sedov phase grows slower than free expansion.
        assert!(n.radius(10_000.0 * yr) < 1e7 * 10_000.0 * yr * 0.5);
        assert!(n.brightness(200_000.0 * yr) == 0.0);
    }
}
