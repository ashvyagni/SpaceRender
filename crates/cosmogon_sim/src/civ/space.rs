//! A civilization's space programme: satellites, robotic exploration of its home system,
//! crewed landings, colony voyages and (much later) interstellar probes.
//!
//! Missions are real objects with a launch date and an arrival date from a Hohmann-like
//! transfer time, so the renderer can show spacecraft in flight. Everything is stepped
//! with the civilization and is deterministic.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use super::{CivEvent, Civilization, Colony};
use crate::history::Category as C;
use crate::rng::Rng;
use crate::time::SECONDS_PER_YEAR;

/// How thoroughly a world has been visited, in increasing order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MissionKind {
    Flyby,
    Orbiter,
    Lander,
    Crewed,
    Colony,
}

impl MissionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Flyby => "Flyby",
            Self::Orbiter => "Orbiter",
            Self::Lander => "Lander",
            Self::Crewed => "Crewed landing",
            Self::Colony => "Colony ship",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Mission {
    pub kind: MissionKind,
    pub body: u32,
    pub launched: f64,
    pub arrives: f64,
    pub name: String,
}

impl Mission {
    /// Fraction of the journey completed at `t` (0..1).
    pub fn progress(&self, t: f64) -> f64 {
        ((t - self.launched) / (self.arrives - self.launched).max(1.0)).clamp(0.0, 1.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Explored {
    pub body: u32,
    pub level: MissionKind,
    /// When the world was first reached, and when the current level was reached.
    pub first: f64,
    pub since: f64,
}

/// A reachable world in the home system, as seen from the civilization's homeworld.
#[derive(Clone, Debug)]
pub struct Destination {
    pub body: u32,
    pub name: String,
    /// One-way transfer time (s).
    pub transfer: f64,
    pub surface: bool,
    /// Moon of the homeworld (reachable by early crewed programmes).
    pub home_moon: bool,
    /// Desirability as a colony site (0 = unsuitable).
    pub colony: f64,
}

/// Hohmann transfer time (s) between circular orbits of radii `r1`, `r2` around `mu`.
pub fn hohmann_time(r1: f64, r2: f64, mu: f64) -> f64 {
    let a = 0.5 * (r1 + r2);
    std::f64::consts::PI * (a * a * a / mu.max(1.0)).sqrt()
}

const MISSION_NAMES: [&str; 16] = ["Pathfinder", "Voyager", "Horizon", "Mariner", "Pioneer", "Odyssey", "Herald", "Wayfarer", "Seeker", "Lantern", "Compass", "Meridian", "Envoy", "Harbinger", "Tracer", "Venture"];

impl Civilization {
    /// Kardashev rating from total power use (Sagan's interpolation): K = (log₁₀ P − 6) / 10.
    /// Earth in 2025 (~19 TW) ≈ 0.73.
    pub fn kardashev(&self) -> f64 {
        ((self.total_power_w().max(1.0).log10() - 6.0) / 10.0).max(0.0)
    }

    pub fn exploration_level(&self, body: u32) -> Option<MissionKind> {
        self.explored.iter().find(|e| e.body == body).map(|e| e.level)
    }

    fn record_visit(&mut self, body: u32, kind: MissionKind, t: f64) -> bool {
        let level = if kind == MissionKind::Colony { MissionKind::Crewed } else { kind };
        match self.explored.iter_mut().find(|e| e.body == body) {
            Some(e) if e.level >= level => false,
            Some(e) => {
                e.level = level;
                e.since = t;
                true
            }
            None => {
                self.explored.push(Explored { body, level, first: t, since: t });
                true
            }
        }
    }

    /// Next mission a destination calls for, given what has been done and what the
    /// civilization can do.
    fn next_step(&self, d: &Destination) -> Option<MissionKind> {
        let have = self.exploration_level(d.body);
        let in_flight = self.missions.iter().filter(|m| m.body == d.body && m.kind != MissionKind::Colony).map(|m| m.kind).max();
        let reached = have.max(in_flight);
        let crewed_ok = (d.home_moon && self.flags.contains("moon_landing")) || (self.flags.contains("stations") && d.transfer < 3.0 * SECONDS_PER_YEAR) || self.flags.contains("colonies");
        let next = match reached {
            None => MissionKind::Flyby,
            Some(MissionKind::Flyby) => MissionKind::Orbiter,
            Some(MissionKind::Orbiter) if d.surface => MissionKind::Lander,
            Some(MissionKind::Lander) if d.surface && crewed_ok => MissionKind::Crewed,
            _ => return None,
        };
        Some(next)
    }

    /// Advance the space programme by `dt` years.
    pub(super) fn step_space(&mut self, destinations: &[Destination], t: f64, dt: f64, rng: &mut Rng) -> Vec<CivEvent> {
        let mut ev = Vec::new();

        // Arrivals.
        let mut arrived = Vec::new();
        self.missions.retain(|m| {
            if m.arrives <= t {
                arrived.push(m.clone());
                false
            } else {
                true
            }
        });
        for m in arrived {
            let Some(d) = destinations.iter().find(|d| d.body == m.body) else { continue };
            let first_ever = self.exploration_level(m.body).is_none();
            let improved = self.record_visit(m.body, m.kind, t);
            match m.kind {
                MissionKind::Colony => {
                    if !self.colonies.iter().any(|c| c.body == m.body) {
                        self.colonies.push(Colony { body: m.body, founded: t, population: 50.0, terraformed: false });
                        ev.push(CivEvent { importance: 5, category: C::Space, title: format!("Colony founded on {}", d.name), detail: format!("The colony ship {} lands; a permanent settlement begins.", m.name) });
                    }
                }
                MissionKind::Crewed if improved => ev.push(CivEvent { importance: 5, category: C::Space, title: format!("First crewed landing on {}", d.name), detail: format!("{} sets down; explorers walk on another world.", m.name) }),
                MissionKind::Lander if improved => ev.push(CivEvent { importance: 4, category: C::Space, title: format!("Probe lands on {}", d.name), detail: format!("{} returns images from the surface.", m.name) }),
                _ if first_ever => ev.push(CivEvent { importance: 4, category: C::Space, title: format!("First probe reaches {}", d.name), detail: format!("{} ({}) after {:.1} years in flight.", m.name, m.kind.label().to_lowercase(), (m.arrives - m.launched) / SECONDS_PER_YEAR) }),
                _ => {}
            }
        }

        // New robotic and crewed missions, at a rate set by the size of the programme.
        if self.flags.contains("satellites") {
            let capacity = (1.0 + (self.satellites as f64 / 1500.0) + if self.flags.contains("stations") { 3.0 } else { 0.0 }).min(10.0) as usize;
            let rate = 0.15 + 0.1 * capacity as f64;
            if self.missions.len() < capacity && rng.hazard(rate, dt.min(10.0)) {
                // Nearest worlds first, and the least-explored among them.
                let pick = destinations
                    .iter()
                    .filter_map(|d| self.next_step(d).map(|k| (d, k)))
                    .min_by(|a, b| (a.0.transfer * (1.0 + a.1 as u8 as f64 * 0.8)).total_cmp(&(b.0.transfer * (1.0 + b.1 as u8 as f64 * 0.8))));
                // With every reachable step done, programmes keep flying follow-up science
                // missions (new orbiters and landers) to worlds already visited.
                let pick = pick.or_else(|| {
                    let explored: Vec<&Destination> = destinations.iter().filter(|d| self.exploration_level(d.body).is_some()).collect();
                    (!explored.is_empty() && rng.chance(0.4)).then(|| {
                        let d = explored[rng.range_u32(0, explored.len() as u32 - 1) as usize];
                        (d, if d.surface { MissionKind::Lander } else { MissionKind::Orbiter })
                    })
                });
                if let Some((d, kind)) = pick {
                    let name = format!("{} {}", MISSION_NAMES[rng.range_u32(0, MISSION_NAMES.len() as u32 - 1) as usize], self.missions_launched + 1);
                    self.missions_launched += 1;
                    self.missions.push(Mission { kind, body: d.body, launched: t, arrives: t + d.transfer, name });
                }
            }
        }

        // Colony voyages.
        self.colony_timer += dt;
        if self.flags.contains("colonies") && self.colony_timer >= 25.0 {
            self.colony_timer = 0.0;
            let busy = |b: u32| self.colonies.iter().any(|c| c.body == b) || self.missions.iter().any(|m| m.body == b && m.kind == MissionKind::Colony);
            if let Some(d) = destinations.iter().filter(|d| d.colony > 0.0 && !busy(d.body)).max_by(|a, b| a.colony.total_cmp(&b.colony)) {
                if rng.chance(0.5) {
                    let name = format!("Ark {}", self.colonies.len() + 1);
                    self.missions.push(Mission { kind: MissionKind::Colony, body: d.body, launched: t, arrives: t + d.transfer * 1.2, name: name.clone() });
                    ev.push(CivEvent { importance: 4, category: C::Space, title: format!("Colony ship departs for {}", d.name), detail: format!("{name} carries the first settlers.") });
                }
            }
        }
        for c in &mut self.colonies {
            let cap = 2.0e6;
            c.population = cap / (1.0 + (cap / c.population.max(1.0) - 1.0) * (-0.03 * dt).dexp());
        }
        ev
    }
}
