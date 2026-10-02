//! Star systems and positions.

use serde::{Deserialize, Serialize};

use super::{Body, Orbit, Star};
use crate::Vec3d;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Companion {
    pub star: Star,
    /// Orbit about the primary (the primary is held fixed: S-type approximation).
    pub orbit: Orbit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Belt {
    pub name: String,
    pub inner: f64,
    pub outer: f64,
    pub icy: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StarSystem {
    pub id: u32,
    pub name: String,
    /// Position of the primary in the local galactic frame (m).
    pub position: Vec3d,
    pub star: Star,
    pub companion: Option<Companion>,
    pub bodies: Vec<Body>,
    pub belts: Vec<Belt>,
}

impl StarSystem {
    /// Gravitational parameter of whatever `body` orbits.
    pub fn parent_mu(&self, body: usize) -> f64 {
        match self.bodies[body].parent {
            Some(p) => self.bodies[p as usize].mu(),
            None => self.star.mu(),
        }
    }

    /// Position of `body` relative to the primary star at time `t`.
    pub fn body_local_position(&self, body: usize, t: f64) -> Vec3d {
        let b = &self.bodies[body];
        let rel = b.orbit.position(self.parent_mu(body), t);
        match b.parent {
            Some(p) => self.body_local_position(p as usize, t) + rel,
            None => rel,
        }
    }

    /// Absolute position (local galactic frame) of `body` at time `t`.
    pub fn body_position(&self, body: usize, t: f64) -> Vec3d {
        self.position + self.body_local_position(body, t)
    }

    pub fn companion_position(&self, t: f64) -> Option<Vec3d> {
        self.companion.as_ref().map(|c| self.position + c.orbit.position(self.star.mu() + c.star.mu(), t))
    }

    /// Distance from the primary star (m) of `body` (for moons, of their planet).
    pub fn stellar_distance(&self, body: usize) -> f64 {
        let mut idx = body;
        while let Some(p) = self.bodies[idx].parent {
            idx = p as usize;
        }
        self.bodies[idx].orbit.a
    }

    pub fn moons_of(&self, body: usize) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(move |(_, b)| b.parent == Some(body as u32)).map(|(i, _)| i)
    }

    pub fn planets(&self) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(|(_, b)| b.parent.is_none()).map(|(i, _)| i)
    }

    pub fn find_body(&self, name: &str) -> Option<usize> {
        self.bodies.iter().position(|b| b.name.eq_ignore_ascii_case(name))
    }
}
