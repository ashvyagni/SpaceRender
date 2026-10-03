//! Star systems and positions.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
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
    /// N-body state when the system is dynamic (sandbox); `None` = analytic Kepler orbits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<super::dynamics::Dynamics>,
    /// Collisions found while syncing for an edit, waiting for the universe to apply
    /// their consequences (transient).
    #[serde(skip)]
    pub pending_contacts: Vec<super::dynamics::ContactEvent>,
    /// Expanding shells of gas from dying stars.
    #[serde(default)]
    pub nebulae: Vec<crate::stellar::Nebula>,
}

impl StarSystem {
    /// Gravitational parameter of whatever `body` orbits.
    pub fn parent_mu(&self, body: usize) -> f64 {
        match self.bodies[body].parent {
            Some(p) => self.bodies[p as usize].mu(),
            None => self.star.mu(),
        }
    }

    /// Position of `body` in the system frame at time `t` (analytic mode: relative to the
    /// primary star, which sits at the origin; dynamic mode: relative to the barycentre).
    pub fn body_local_position(&self, body: usize, t: f64) -> Vec3d {
        if self.dynamics.is_some() {
            return self.body_state(body, t).pos;
        }
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

    /// Absolute position of the primary star.
    pub fn star_position(&self, t: f64) -> Vec3d {
        self.position + self.star_local_position(t)
    }

    pub fn companion_position(&self, t: f64) -> Option<Vec3d> {
        if let Some(d) = &self.dynamics {
            let acc = d.acc.get(1).copied().filter(|_| d.companion.is_some()).unwrap_or(Vec3d::ZERO);
            let dt = t - d.time();
            return d.companion.map(|c| self.position + c.pos + c.vel * dt + acc * (0.5 * dt * dt));
        }
        self.companion.as_ref().map(|c| self.position + c.orbit.position(self.star.mu() + c.star.mu(), t))
    }

    /// The planet (top-level body) that `body` belongs to.
    pub fn top_level(&self, body: usize) -> usize {
        let mut idx = body;
        while let Some(p) = self.bodies[idx].parent {
            idx = p as usize;
        }
        idx
    }

    /// Effective distance from the primary star (m) for climate: the distance at which a
    /// circular orbit receives the same *annual-mean* flux, `a·(1−e²)^¼` (for moons, that of
    /// their planet). Dynamic systems use the current osculating orbit; unbound bodies
    /// their instantaneous distance.
    pub fn stellar_distance(&self, body: usize) -> f64 {
        let idx = self.top_level(body);
        let (a, e) = match &self.dynamics {
            Some(d) => {
                let el = self.osculating(idx, d.time());
                if el.is_bound() {
                    (el.semi_major_axis, el.eccentricity)
                } else {
                    let t = d.time();
                    return (self.body_local_position(idx, t) - self.star_local_position(t)).length();
                }
            }
            None => (self.bodies[idx].orbit.a, self.bodies[idx].orbit.e),
        };
        a * (1.0 - e * e).max(0.0).dpowf(0.25)
    }

    pub fn moons_of(&self, body: usize) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(move |(_, b)| b.parent == Some(body as u32) && b.exists()).map(|(i, _)| i)
    }

    pub fn planets(&self) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(|(_, b)| b.parent.is_none() && b.exists()).map(|(i, _)| i)
    }

    /// Indices of bodies that still exist.
    pub fn existing(&self) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(|(_, b)| b.exists()).map(|(i, _)| i)
    }

    pub fn find_body(&self, name: &str) -> Option<usize> {
        self.bodies.iter().position(|b| b.name.eq_ignore_ascii_case(name))
    }
}
