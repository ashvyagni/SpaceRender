//! Dynamic gravity for an active sandbox system: the bridge between the analytic Kepler
//! world of `StarSystem` and the N-body integrator in `cosmogon_physics::nbody`.
//!
//! A system without `dynamics` is propagated analytically (Tier 1). With `dynamics`, the
//! star, any companion and the "active" bodies are particles integrated by Newtonian
//! N-body (Tier 2/3). Light, tight moons stay on Kepler "rails" around their (moving)
//! parent until something makes them matter. See docs/PHYSICS_ENGINE.md.
//!
//! Frame: the system frame (sim axes, metres) whose origin is `StarSystem::position`.
//! On activation the particles are shifted to their centre of mass, so the origin is the
//! system barycentre and the system does not drift.

use std::time::Instant;

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;

use cosmogon_physics::nbody::{self, Contact, Particle, Settings};
use cosmogon_physics::orbital_elements::{state_vectors_to_elements, OrbitalElements};
use serde::{Deserialize, Serialize};

use super::{Orbit, StarSystem, G};
use crate::time::SECONDS_PER_DAY;
use crate::Vec3d;

/// Moons lighter than this fraction of their parent ride Kepler rails until touched.
pub const RAILS_MASS_RATIO: f64 = 1.0e-3;
const MIN_DT: f64 = 1.0;
const MAX_DT: f64 = 30.0 * SECONDS_PER_DAY;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum PhysicsPreset {
    Fast,
    #[default]
    Balanced,
    Accurate,
    Research,
    Custom,
}

impl PhysicsPreset {
    pub const ALL: [PhysicsPreset; 5] = [Self::Fast, Self::Balanced, Self::Accurate, Self::Research, Self::Custom];
    pub fn label(self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Balanced => "Balanced",
            Self::Accurate => "Accurate",
            Self::Research => "Research",
            Self::Custom => "Custom",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Fast => "40 steps per tightest orbit. For many bodies and high time acceleration; energy errors ~10⁻⁴ on eccentric orbits.",
            Self::Balanced => "120 steps per tightest orbit. The default: ~10⁻⁵ energy error on eccentric binaries, ~10⁻⁹ on planets.",
            Self::Accurate => "400 steps per tightest orbit and relativistic correction. ~10⁻⁷ energy error; about 3× slower than Balanced.",
            Self::Research => "1500 steps per tightest orbit and relativistic correction. For small systems where precision matters more than speed.",
            Self::Custom => "Choose the steps per orbit and relativity yourself.",
        }
    }
    pub fn settings(self) -> PhysicsSettings {
        let (steps, rel) = match self {
            Self::Fast => (40.0, false),
            Self::Balanced | Self::Custom => (120.0, false),
            Self::Accurate => (400.0, true),
            Self::Research => (1500.0, true),
        };
        PhysicsSettings { preset: self, steps_per_orbit: steps, relativity: rel }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PhysicsSettings {
    pub preset: PhysicsPreset,
    /// Substeps per orbit of the tightest bound pair (η = 2π / this).
    pub steps_per_orbit: f64,
    /// First post-Newtonian correction from the most massive body.
    pub relativity: bool,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        PhysicsPreset::Balanced.settings()
    }
}

impl PhysicsSettings {
    pub fn integrator(&self) -> Settings {
        Settings { eta: std::f64::consts::TAU / self.steps_per_orbit.clamp(8.0, 100_000.0), max_substeps: 4096, relativity: self.relativity }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct State {
    pub pos: Vec3d,
    pub vel: Vec3d,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct Diagnostics {
    /// Energy (×G) when the current step grid began (activation, edit or collision).
    pub energy0: f64,
    /// Relative energy error since then.
    pub energy_error: f64,
    pub last_substeps: u32,
    pub max_substeps: u32,
    /// Macro steps whose encounters needed more than the substep cap.
    pub saturated_steps: u64,
    pub macro_steps_total: u64,
}

/// Which simulated object a particle is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Star,
    Companion,
    Body(u32),
}

/// A collision between two particles, in simulation terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactEvent {
    pub survivor: Slot,
    pub absorbed: Slot,
    pub contact: Contact,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Dynamics {
    pub settings: PhysicsSettings,
    /// Start of the current macro-step grid (s since J2000).
    pub epoch: f64,
    pub dt: f64,
    pub steps: u64,
    pub star: State,
    pub companion: Option<State>,
    /// Per body index: `Some` = integrated particle; `None` = on rails (or removed).
    pub bodies: Vec<Option<State>>,
    pub diagnostics: Diagnostics,
    /// Relative insolation at the last climate refresh and at the last report, per body.
    #[serde(default)]
    pub flux_climate: Vec<f64>,
    #[serde(default)]
    pub flux_reported: Vec<f64>,
    /// Mean temperature at the last report, per body.
    #[serde(default)]
    pub temp_reported: Vec<f64>,
    /// Accelerations at the grid time, for smooth positions between steps (derived).
    #[serde(skip)]
    pub acc: Vec<Vec3d>,
}

impl Dynamics {
    /// Time of the authoritative state.
    pub fn time(&self) -> f64 {
        self.epoch + self.steps as f64 * self.dt
    }
    pub fn active_count(&self) -> usize {
        self.bodies.iter().filter(|b| b.is_some()).count()
    }
}

fn extrapolate(s: &State, acc: Option<Vec3d>, dt: f64) -> Vec3d {
    s.pos + s.vel * dt + acc.unwrap_or(Vec3d::ZERO) * (0.5 * dt * dt)
}

impl StarSystem {
    pub fn is_dynamic(&self) -> bool {
        self.dynamics.is_some()
    }

    /// Whether `body` should be a full particle when the system becomes dynamic.
    fn starts_active(&self, body: usize) -> bool {
        let b = &self.bodies[body];
        if !b.exists() {
            return false;
        }
        match b.parent {
            None => true,
            Some(p) => b.mass >= RAILS_MASS_RATIO * self.bodies[p as usize].mass,
        }
    }

    /// Position of the primary star in the system frame.
    pub fn star_local_position(&self, t: f64) -> Vec3d {
        match &self.dynamics {
            Some(d) => extrapolate(&d.star, d.acc.first().copied(), t - d.time()),
            None => Vec3d::ZERO,
        }
    }

    /// Position and velocity of `body` in the system frame at `t`.
    pub fn body_state(&self, body: usize, t: f64) -> State {
        let b = &self.bodies[body];
        if let Some(d) = &self.dynamics {
            if let Some(Some(s)) = d.bodies.get(body) {
                let k = self.slot_index(Slot::Body(body as u32));
                let acc = k.and_then(|k| d.acc.get(k).copied());
                let dt = t - d.time();
                return State { pos: extrapolate(s, acc, dt), vel: s.vel + acc.unwrap_or(Vec3d::ZERO) * dt };
            }
            // On rails around a moving parent.
            let parent = match b.parent {
                Some(p) => self.body_state(p as usize, t),
                None => State { pos: self.star_local_position(t), vel: d.star.vel },
            };
            let mu = self.parent_mu(body);
            let (p, v) = b.orbit.state(mu, mu + b.mu(), t);
            return State { pos: parent.pos + p, vel: parent.vel + v };
        }
        let mu = self.parent_mu(body);
        let (p, v) = b.orbit.state(mu, mu, t);
        match b.parent {
            Some(par) => {
                let ps = self.body_state(par as usize, t);
                State { pos: ps.pos + p, vel: ps.vel + v }
            }
            None => State { pos: p, vel: v },
        }
    }

    /// State of whatever `body` orbits (its parent body or the star) at `t`.
    pub fn parent_state(&self, body: usize, t: f64) -> State {
        match self.bodies[body].parent {
            Some(p) => self.body_state(p as usize, t),
            None => match &self.dynamics {
                Some(d) => State { pos: self.star_local_position(t), vel: d.star.vel },
                None => State::default(),
            },
        }
    }

    /// Osculating orbital elements of `body` relative to its parent (two-body μ = G(M+m)).
    pub fn osculating(&self, body: usize, t: f64) -> OrbitalElements {
        let s = self.body_state(body, t);
        let p = self.parent_state(body, t);
        let mu = self.parent_mu(body) + self.bodies[body].mu();
        state_vectors_to_elements(s.pos - p.pos, s.vel - p.vel, mu.max(1e-30))
    }

    /// Index of a slot in the particle array built by [`StarSystem::particles`].
    fn slot_index(&self, slot: Slot) -> Option<usize> {
        let d = self.dynamics.as_ref()?;
        let base = 1 + d.companion.is_some() as usize;
        match slot {
            Slot::Star => Some(0),
            Slot::Companion => d.companion.map(|_| 1),
            Slot::Body(i) => {
                if d.bodies.get(i as usize).copied().flatten().is_none() || !self.bodies[i as usize].exists() {
                    return None;
                }
                Some(base + d.bodies[..i as usize].iter().enumerate().filter(|(j, b)| b.is_some() && self.bodies[*j].exists()).count())
            }
        }
    }

    /// The particle array of a dynamic system (star, companion, active bodies — in order).
    pub fn particles(&self, t: f64) -> (Vec<Particle>, Vec<Slot>) {
        let d = self.dynamics.as_ref().expect("system is not dynamic");
        let mut ps = vec![Particle::new(d.star.pos, d.star.vel, self.star.mu(), self.star.current_radius(t))];
        let mut slots = vec![Slot::Star];
        if let (Some(c), Some(cs)) = (&self.companion, d.companion) {
            ps.push(Particle::new(cs.pos, cs.vel, c.star.mu(), c.star.current_radius(t)));
            slots.push(Slot::Companion);
        }
        for (i, s) in d.bodies.iter().enumerate() {
            if let Some(s) = s.filter(|_| self.bodies[i].exists()) {
                let b = &self.bodies[i];
                // Moons on rails are massless test bodies; their mass rides with the parent
                // particle, which then stands for the planet-system barycentre.
                let rails: f64 = self.bodies.iter().enumerate().filter(|(m, x)| x.parent == Some(i as u32) && x.exists() && d.bodies[*m].is_none()).map(|(_, x)| x.mu()).sum();
                ps.push(Particle::new(s.pos, s.vel, b.mu() + rails, b.interaction_radius()));
                slots.push(Slot::Body(i as u32));
            }
        }
        (ps, slots)
    }

    fn write_back(&mut self, ps: &[Particle], slots: &[Slot]) {
        let d = self.dynamics.as_mut().expect("dynamic");
        for (p, slot) in ps.iter().zip(slots) {
            let st = State { pos: p.pos, vel: p.vel };
            match slot {
                Slot::Star => d.star = st,
                Slot::Companion => d.companion = Some(st),
                Slot::Body(i) => d.bodies[*i as usize] = if p.alive { Some(st) } else { None },
            }
        }
    }

    /// Recompute the cached accelerations used for smooth positions between steps.
    pub fn refresh_accelerations(&mut self) {
        let Some(d) = &self.dynamics else { return };
        let t = d.time();
        let rel = d.settings.relativity;
        let (ps, _) = self.particles(t);
        let mut acc = Vec::new();
        nbody::accelerations(&ps, rel, &mut acc);
        self.dynamics.as_mut().unwrap().acc = acc;
    }

    /// Switch from analytic orbits to N-body at time `t`.
    pub fn activate_dynamics(&mut self, t: f64, settings: PhysicsSettings) {
        if self.dynamics.is_some() {
            return;
        }
        // Analytic states in the star-centred frame; only some bodies become particles.
        let bodies: Vec<Option<State>> = (0..self.bodies.len()).map(|i| self.starts_active(i).then(|| self.kepler_state(i, t))).collect();
        let companion = self.companion.as_ref().map(|c| {
            let mu = self.star.mu() + c.star.mu();
            let (p, v) = c.orbit.state(mu, mu, t);
            State { pos: p, vel: v }
        });
        self.dynamics = Some(Dynamics {
            settings,
            epoch: t,
            dt: 0.0,
            steps: 0,
            star: State::default(),
            companion,
            bodies,
            diagnostics: Diagnostics::default(),
            flux_climate: Vec::new(),
            flux_reported: Vec::new(),
            temp_reported: Vec::new(),
            acc: Vec::new(),
        });
        self.recentre_on_barycentre(t);
        self.retune(t);
    }

    /// Start dynamics from measured state vectors (system frame, SI). Bodies without a
    /// state ride rails; rails moons get their osculating orbit from their own measured
    /// state when one is given, so they start exactly where the data puts them.
    pub fn activate_from_states(&mut self, t: f64, settings: PhysicsSettings, star: State, states: &[(usize, State)]) {
        let mut bodies = vec![None; self.bodies.len()];
        let mut measured: Vec<Option<State>> = vec![None; self.bodies.len()];
        for (i, s) in states {
            measured[*i] = Some(*s);
        }
        for i in 0..self.bodies.len() {
            if self.starts_active(i) {
                bodies[i] = measured[i];
            }
        }
        // Rails moons: fit their analytic orbit to the measured state relative to the parent.
        for i in 0..self.bodies.len() {
            if bodies[i].is_some() {
                continue;
            }
            if let (Some(m), Some(p)) = (measured[i], self.bodies[i].parent) {
                if let Some(ps) = measured[p as usize] {
                    let mu = self.parent_mu(i);
                    if let Some(o) = Orbit::from_state(m.pos - ps.pos, m.vel - ps.vel, mu + self.bodies[i].mu(), t) {
                        // Keep the analytic mean motion consistent with how rails are propagated.
                        let n_fit = ((mu + self.bodies[i].mu()) / o.a.powi(3)).sqrt();
                        let n_rails = (mu / o.a.powi(3)).sqrt();
                        let m_now = (o.m0 + n_fit * t).rem_euclid(std::f64::consts::TAU);
                        self.bodies[i].orbit = Orbit { m0: (m_now - n_rails * t).rem_euclid(std::f64::consts::TAU), ..o };
                    }
                }
            }
        }
        self.dynamics = Some(Dynamics {
            settings,
            epoch: t,
            dt: 0.0,
            steps: 0,
            star,
            companion: None,
            bodies,
            diagnostics: Diagnostics::default(),
            flux_climate: Vec::new(),
            flux_reported: Vec::new(),
            temp_reported: Vec::new(),
            acc: Vec::new(),
        });
        self.recentre_on_barycentre(t);
        self.retune(t);
    }

    /// Analytic state of `body` in the star-centred frame (Kepler mode semantics).
    fn kepler_state(&self, body: usize, t: f64) -> State {
        let b = &self.bodies[body];
        let mu = self.parent_mu(body);
        let (p, v) = b.orbit.state(mu, mu + b.mu(), t);
        match b.parent {
            Some(par) => {
                let ps = self.kepler_state(par as usize, t);
                State { pos: ps.pos + p, vel: ps.vel + v }
            }
            None => State { pos: p, vel: v },
        }
    }

    fn recentre_on_barycentre(&mut self, t: f64) {
        let (mut ps, slots) = self.particles(t);
        nbody::to_barycentric(&mut ps);
        self.write_back(&ps, &slots);
    }

    /// Start a new macro-step grid at `t` with a step suited to the current configuration.
    pub fn retune(&mut self, t: f64) {
        let Some(d) = &self.dynamics else { return };
        let settings = d.settings.integrator();
        let (ps, _) = self.particles(t);
        let mut dt = nbody::natural_step(&ps, &settings);
        if !dt.is_finite() {
            dt = MAX_DT;
        }
        let energy = nbody::energy(&ps);
        let d = self.dynamics.as_mut().unwrap();
        d.epoch = t;
        d.steps = 0;
        d.dt = dt.clamp(MIN_DT, MAX_DT);
        d.diagnostics.energy0 = energy;
        d.diagnostics.energy_error = 0.0;
        self.refresh_accelerations();
    }

    /// Integrate exactly to `t` (a partial step if needed) and restart the grid there.
    /// Used before edits so they take effect at the moment the user made them. Collisions
    /// on the way are merged and queued in `pending_contacts` for the caller.
    pub fn sync_to(&mut self, t: f64) {
        if self.dynamics.is_none() {
            return;
        }
        loop {
            let mut c = Vec::new();
            let done = self.advance_dynamics(t, None, &mut c);
            let had = !c.is_empty();
            self.pending_contacts.extend(c);
            if done || !had {
                break;
            }
        }
        let (time, settings) = {
            let d = self.dynamics.as_ref().unwrap();
            (d.time(), d.settings.integrator())
        };
        let rest = t - time;
        if rest > 0.0 {
            let (mut ps, slots) = self.particles(time);
            let r = nbody::step(&mut ps, time, rest, &settings);
            self.write_back(&ps, &slots);
            let cs: Vec<ContactEvent> = r.contacts.iter().map(|c| ContactEvent { survivor: slots[c.a], absorbed: slots[c.b], contact: *c }).collect();
            if !cs.is_empty() {
                self.apply_merges(&cs, &ps, &slots);
                self.pending_contacts.extend(cs);
            }
        }
        self.retune(t);
    }

    /// Book-keeping after collisions: the survivor takes the merged mass and radius, the
    /// absorbed body is marked removed, and its rails moons are handed to the survivor.
    fn apply_merges(&mut self, contacts: &[ContactEvent], ps: &[Particle], slots: &[Slot]) {
        for c in contacts {
            let k = slots.iter().position(|s| *s == c.survivor).expect("survivor slot");
            // Something heavier than the star swallowed it (a black hole passing through):
            // that object becomes the system's centre, carrying the merged mass and motion.
            if let (Slot::Star, Slot::Body(i)) = (c.absorbed, c.survivor) {
                let i = i as usize;
                let b = &self.bodies[i];
                let total = ps[k].gm / G / super::SOLAR_MASS;
                let kind = match b.kind {
                    super::BodyKind::BlackHole => super::star::StarKind::BlackHole,
                    super::BodyKind::NeutronStar => super::star::StarKind::NeutronStar,
                    super::BodyKind::WhiteDwarf => super::star::StarKind::WhiteDwarf,
                    _ => super::star::StarKind::Normal,
                };
                let t = c.contact.time;
                let name = b.name.clone();
                self.star = if kind == super::star::StarKind::Normal {
                    super::Star::from_mass(name, total, self.star.metallicity, t)
                } else {
                    super::Star::compact(name, kind, total, t)
                };
                if let Some(d) = self.dynamics.as_mut() {
                    d.star = State { pos: ps[k].pos, vel: ps[k].vel };
                    d.bodies[i] = None;
                }
                self.bodies[i].removed = Some(super::Removal { time: t, cause: super::RemovalCause::MergedInto(None) });
                continue;
            }
            let survivor_index = match c.survivor {
                Slot::Body(i) => Some(i),
                _ => None,
            };
            // The survivor gains the absorbed body's own mass (rails moons are released, not eaten).
            let gained = match c.absorbed {
                Slot::Body(j) => self.bodies[j as usize].mass,
                _ => c.contact.gm_b / G,
            };
            match c.survivor {
                Slot::Star => self.star.mass += gained / super::SOLAR_MASS,
                Slot::Companion => {
                    if let Some(comp) = self.companion.as_mut() {
                        comp.star.mass += gained / super::SOLAR_MASS;
                    }
                }
                Slot::Body(i) => {
                    let b = &mut self.bodies[i as usize];
                    b.mass += gained;
                    b.radius = ps[k].radius;
                }
            }
            if let Slot::Body(j) = c.absorbed {
                let j = j as usize;
                let t = c.contact.time;
                // Moons riding rails around the absorbed body become free particles.
                let moons: Vec<usize> = (0..self.bodies.len()).filter(|&m| self.bodies[m].parent == Some(j as u32) && self.bodies[m].exists()).collect();
                let kj = slots.iter().position(|s| *s == c.absorbed).expect("absorbed slot");
                for m in moons {
                    let st = match self.dynamics.as_ref().and_then(|d| d.bodies[m]) {
                        Some(st) => st,
                        None => {
                            let mu = self.parent_mu(m);
                            let (p, v) = self.bodies[m].orbit.state(mu, mu + self.bodies[m].mu(), t);
                            State { pos: ps[kj].pos + p, vel: ps[kj].vel + v }
                        }
                    };
                    self.bodies[m].parent = survivor_index;
                    if let Some(d) = self.dynamics.as_mut() {
                        if d.bodies[m].is_none() {
                            d.bodies[m] = Some(st);
                        }
                    }
                }
                self.bodies[j].removed = Some(super::Removal { time: t, cause: super::RemovalCause::MergedInto(survivor_index) });
                if let Some(d) = self.dynamics.as_mut() {
                    d.bodies[j] = None;
                }
            }
        }
    }

    /// Promote a rails moon to a full particle at time `t` (its current analytic state).
    pub fn promote(&mut self, body: usize, t: f64) {
        let Some(d) = &self.dynamics else { return };
        if d.bodies.get(body).copied().flatten().is_some() || !self.bodies[body].exists() {
            return;
        }
        let s = self.body_state(body, t);
        self.dynamics.as_mut().unwrap().bodies[body] = Some(s);
    }

    /// Promote every rails moon of `parent` (e.g. when an intruder enters its Hill sphere).
    fn promote_moons_of(&mut self, parent: usize, t: f64) -> bool {
        let moons: Vec<usize> = (0..self.bodies.len())
            .filter(|&i| self.bodies[i].parent == Some(parent as u32) && self.bodies[i].exists() && self.dynamics.as_ref().is_some_and(|d| d.bodies[i].is_none()))
            .collect();
        for &m in &moons {
            self.promote(m, t);
        }
        !moons.is_empty()
    }

    /// Return to analytic orbits: every body takes its current osculating ellipse.
    /// Fails if any body is unbound (it cannot be represented by an ellipse).
    pub fn deactivate_dynamics(&mut self, t: f64) -> Result<(), String> {
        let Some(_) = &self.dynamics else { return Ok(()) };
        self.sync_to(t);
        let mut orbits = Vec::new();
        for i in 0..self.bodies.len() {
            if !self.bodies[i].exists() {
                continue;
            }
            let s = self.body_state(i, t);
            let p = self.parent_state(i, t);
            let mu = self.parent_mu(i);
            let orbit = Orbit::from_state(s.pos - p.pos, s.vel - p.vel, mu + self.bodies[i].mu(), t)
                .ok_or_else(|| format!("{} is on an unbound path; it cannot follow a fixed orbit", self.bodies[i].name))?;
            // Analytic propagation uses the parent's μ for the mean motion; refer M to that.
            let n_fit = ((mu + self.bodies[i].mu()) / orbit.a.powi(3)).sqrt();
            let n_rails = (mu / orbit.a.powi(3)).sqrt();
            let m_now = (orbit.m0 + n_fit * t).rem_euclid(std::f64::consts::TAU);
            orbits.push((i, Orbit { m0: (m_now - n_rails * t).rem_euclid(std::f64::consts::TAU), ..orbit }));
        }
        // The star returns to the frame origin (analytic mode is star-centred).
        let star = self.star_local_position(t);
        self.position += star;
        for (i, o) in orbits {
            self.bodies[i].orbit = o;
        }
        self.dynamics = None;
        Ok(())
    }

    /// Advance the N-body state through every whole macro step that ends at or before
    /// `target`. Stops early at `deadline`, or after a step with collisions (returned in
    /// `contacts`, already merged in the particle state) so the caller can apply their
    /// consequences. Returns true when `target` was reached.
    pub fn advance_dynamics(&mut self, target: f64, deadline: Option<Instant>, contacts: &mut Vec<ContactEvent>) -> bool {
        let Some(d) = &self.dynamics else { return true };
        if d.time() + d.dt > target {
            return true;
        }
        let settings = d.settings.integrator();
        let (mut ps, mut slots) = self.particles(d.time());
        let mut steps_done = 0u64;
        let mut reached = true;
        loop {
            let d = self.dynamics.as_ref().unwrap();
            let t0 = d.time() + steps_done as f64 * d.dt;
            if t0 + d.dt > target {
                break;
            }
            if let Some(dl) = deadline {
                if steps_done.is_multiple_of(8) && Instant::now() > dl {
                    reached = false;
                    break;
                }
            }
            let dt = d.dt;
            let r = nbody::step(&mut ps, t0, dt, &settings);
            steps_done += 1;
            {
                let diag = &mut self.dynamics.as_mut().unwrap().diagnostics;
                diag.last_substeps = r.substeps;
                diag.max_substeps = diag.max_substeps.max(r.substeps);
                diag.saturated_steps += r.saturated as u64;
                diag.macro_steps_total += 1;
            }
            if !r.contacts.is_empty() {
                let cs: Vec<ContactEvent> = r.contacts.iter().map(|c| ContactEvent { survivor: slots[c.a], absorbed: slots[c.b], contact: *c }).collect();
                self.write_back(&ps, &slots);
                self.apply_merges(&cs, &ps, &slots);
                self.dynamics.as_mut().unwrap().steps += steps_done;
                let now = self.dynamics.as_ref().unwrap().time();
                self.retune(now);
                contacts.extend(cs);
                return false;
            }
            // Intruders inside the Hill sphere of a planet with rails moons wake the moons.
            if self.hill_intrusion(&ps, &slots) {
                self.write_back(&ps, &slots);
                self.dynamics.as_mut().unwrap().steps += steps_done;
                steps_done = 0;
                let now = self.dynamics.as_ref().unwrap().time();
                let parents: Vec<usize> = self.rails_parents();
                let mut any = false;
                for p in parents {
                    if self.intruder_near(p, now) {
                        any |= self.promote_moons_of(p, now);
                    }
                }
                if any {
                    self.retune(now);
                }
                let d = self.dynamics.as_ref().unwrap();
                (ps, slots) = self.particles(d.time());
            }
        }
        self.write_back(&ps, &slots);
        let d = self.dynamics.as_mut().unwrap();
        d.steps += steps_done;
        let e = nbody::energy(&ps);
        if d.diagnostics.energy0 != 0.0 {
            d.diagnostics.energy_error = ((e - d.diagnostics.energy0) / d.diagnostics.energy0).abs();
        }
        self.refresh_accelerations();
        reached
    }

    /// Bodies that still have moons on rails.
    fn rails_parents(&self) -> Vec<usize> {
        let Some(d) = &self.dynamics else { return Vec::new() };
        let mut v: Vec<usize> = (0..self.bodies.len())
            .filter(|&i| d.bodies[i].is_none() && self.bodies[i].exists())
            .filter_map(|i| self.bodies[i].parent.map(|p| p as usize))
            .filter(|&p| d.bodies.get(p).copied().flatten().is_some())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn hill_radius_of(&self, body: usize) -> f64 {
        let b = &self.bodies[body];
        let a = b.orbit.a.max(1.0);
        a * (b.mass / (3.0 * self.star.mass * super::SOLAR_MASS)).dcbrt()
    }

    fn intruder_near(&self, parent: usize, t: f64) -> bool {
        let pp = self.body_state(parent, t).pos;
        let r_h = self.hill_radius_of(parent);
        let d = self.dynamics.as_ref().unwrap();
        d.bodies.iter().enumerate().any(|(i, s)| {
            i != parent && self.bodies[i].parent != Some(parent as u32) && s.is_some_and(|s| (s.pos - pp).length() < r_h)
        })
    }

    /// Cheap check on the live particle array: is any active body inside the Hill sphere
    /// of a planet that has rails moons?
    fn hill_intrusion(&self, ps: &[Particle], slots: &[Slot]) -> bool {
        let parents = self.rails_parents();
        if parents.is_empty() {
            return false;
        }
        for p in parents {
            let Some(k) = slots.iter().position(|s| *s == Slot::Body(p as u32)) else { continue };
            let r_h = self.hill_radius_of(p);
            for (j, s) in slots.iter().enumerate() {
                if let Slot::Body(b) = s {
                    if j != k && ps[j].alive && self.bodies[*b as usize].parent != Some(p as u32) && (ps[j].pos - ps[k].pos).length() < r_h {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Integrate a copy of the system forward and sample the paths (system frame).
    /// `extra` adds a hypothetical particle (the launch tool's projectile) as the last
    /// path. Returns the paths per slot plus the first predicted contact, if any.
    pub fn predict(&self, t: f64, extra: Option<Particle>, horizon: f64, samples: usize, budget_steps: usize) -> Prediction {
        let mut sys = self.clone();
        if sys.dynamics.is_none() {
            sys.activate_dynamics(t, PhysicsSettings::default());
        }
        sys.sync_to(t);
        let d = sys.dynamics.as_ref().unwrap();
        let (mut ps, mut slots) = sys.particles(t);
        if let Some(p) = extra {
            ps.push(p);
            slots.push(Slot::Body(u32::MAX));
        }
        let settings = d.settings.integrator();
        let mut dt = nbody::natural_step(&ps, &settings);
        if !dt.is_finite() {
            dt = MAX_DT;
        }
        dt = dt.clamp(MIN_DT, MAX_DT).max(horizon / budget_steps.max(1) as f64);
        let steps = ((horizon / dt).ceil() as usize).max(1);
        let every = (steps / samples.max(1)).max(1);
        let mut paths: Vec<Vec<Vec3d>> = ps.iter().map(|p| vec![p.pos]).collect();
        let mut contacts = Vec::new();
        for k in 0..steps {
            let r = nbody::step(&mut ps, t + k as f64 * dt, dt, &settings);
            contacts.extend(r.contacts.iter().map(|c| (slots[c.a], slots[c.b], c.time)));
            if k % every == every - 1 || k == steps - 1 {
                for (path, p) in paths.iter_mut().zip(&ps) {
                    if p.alive {
                        path.push(p.pos);
                    }
                }
            }
        }
        // The projectile's own collision matters most to the caller; otherwise the first.
        let contact = contacts.iter().copied().find(|(a, b, _)| *a == Slot::Body(u32::MAX) || *b == Slot::Body(u32::MAX)).or(contacts.first().copied());
        Prediction { slots, paths, contact, contacts, step: dt }
    }

    /// Collisions found while syncing for an edit, waiting to be applied.
    pub fn take_pending_contacts(&mut self) -> Vec<ContactEvent> {
        std::mem::take(&mut self.pending_contacts)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Prediction {
    pub slots: Vec<Slot>,
    pub paths: Vec<Vec<Vec3d>>,
    /// (survivor, absorbed, time) of the projectile's predicted collision, else the first.
    pub contact: Option<(Slot, Slot, f64)>,
    pub contacts: Vec<(Slot, Slot, f64)>,
    pub step: f64,
}

/// Gravitational parameter helper for user-facing code.
pub fn gm(mass_kg: f64) -> f64 {
    G * mass_kg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::sol::sol_system;
    use crate::time::SECONDS_PER_YEAR;

    #[test]
    fn activation_preserves_positions_and_keeps_kepler_orbits() {
        let mut sys = sol_system();
        let t = 1.0e8;
        let earth = sys.find_body("Earth").unwrap();
        let before = sys.body_local_position(earth, t) - sys.star_local_position(t);
        sys.activate_dynamics(t, PhysicsSettings::default());
        let after = sys.body_local_position(earth, t) - sys.star_local_position(t);
        assert!((after - before).length() < 1.0, "{}", (after - before).length());
        // Io rides rails; the Moon is a particle.
        let d = sys.dynamics.as_ref().unwrap();
        assert!(d.bodies[sys.find_body("Io").unwrap()].is_none());
        assert!(d.bodies[sys.find_body("Moon").unwrap()].is_some());
        // A year of N-body keeps Earth's orbit close to the analytic one.
        let mut contacts = Vec::new();
        assert!(sys.advance_dynamics(t + SECONDS_PER_YEAR, None, &mut contacts));
        assert!(contacts.is_empty());
        let el = sys.osculating(earth, sys.dynamics.as_ref().unwrap().time());
        assert!((el.semi_major_axis / crate::astro::AU - 1.0).abs() < 2e-3, "a = {}", el.semi_major_axis / crate::astro::AU);
        assert!(sys.dynamics.as_ref().unwrap().diagnostics.energy_error < 1e-8);
    }

    #[test]
    fn deactivation_round_trips_through_osculating_orbits() {
        let mut sys = sol_system();
        let t = 3.0e8;
        sys.activate_dynamics(t, PhysicsSettings::default());
        let mut c = Vec::new();
        sys.advance_dynamics(t + 0.5 * SECONDS_PER_YEAR, None, &mut c);
        let now = sys.dynamics.as_ref().unwrap().time();
        let mars = sys.find_body("Mars").unwrap();
        let before = sys.body_local_position(mars, now) - sys.star_local_position(now);
        sys.deactivate_dynamics(now).unwrap();
        let after = sys.body_local_position(mars, now);
        assert!((after - before).length() < 1e3, "{}", (after - before).length());
    }
}
