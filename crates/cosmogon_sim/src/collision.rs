//! What a collision does to the bodies: merge, or shatter into a survivor and debris.
//!
//! Outcomes follow the catastrophic-disruption scaling of Leinhardt & Stewart (2012) for
//! gravity-dominated bodies: the specific impact energy Q_R = ½ μ v² / M_tot is compared
//! with the energy needed to disperse half the mass, Q*_RD ≈ c* · (4/5) π ρ₁ G R_C1²
//! (c* ≈ 1.9, ρ₁ = 1000 kg/m³, R_C1 the radius of the combined mass at that density). The
//! largest remnant keeps M_lr = M_tot (1 − Q_R / 2Q*_RD) (with a power-law tail for
//! super-catastrophic impacts); the rest leaves as debris near the remnant's escape speed,
//! sheared along the impact direction — so some escapes, some falls back, and some can
//! settle into orbit (how the Moon is thought to have formed). Very hot remnants glow.
//! Gas giants and stars simply absorb impactors.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::dynamics::State;
use crate::astro::{Body, BodyKind, ObjectClass, Quality, G};
use crate::history::{Category, Event};
use crate::rng::Rng;
use crate::time::SECONDS_PER_YEAR;
use crate::Universe;

/// Molten surface after a violent impact (visual and descriptive).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Melt {
    pub since: f64,
    /// Impact energy relative to the body's gravitational binding energy.
    pub severity: f64,
}

impl Melt {
    /// Cooling time (s): a few years for a scorched surface, millennia for a global magma
    /// ocean (it must radiate away a fraction of the binding energy).
    pub fn timescale(&self) -> f64 {
        (30.0 * SECONDS_PER_YEAR * (self.severity / 0.01).max(1e-4).sqrt()).clamp(SECONDS_PER_YEAR, 5.0e4 * SECONDS_PER_YEAR)
    }
    /// Glow 0..1 at time `t`.
    pub fn glow(&self, t: f64) -> f64 {
        let age = (t - self.since).max(0.0);
        ((self.severity * 30.0).min(1.0)) * (-age / self.timescale()).dexp()
    }
    /// Surface temperature of the melt (K) at `t`, for colour.
    pub fn temperature(&self, t: f64) -> f64 {
        900.0 + 1300.0 * self.glow(t)
    }
}

/// Q*_RD (J/kg) for a combined mass `m_tot` (kg): gravity regime, Leinhardt & Stewart 2012.
pub fn disruption_threshold(m_tot: f64) -> f64 {
    let rho1 = 1000.0;
    let r_c1 = (3.0 * m_tot / (4.0 * std::f64::consts::PI * rho1)).dcbrt();
    1.9 * 0.8 * std::f64::consts::PI * rho1 * G * r_c1 * r_c1
}

/// Fraction of the total mass kept by the largest remnant.
pub fn largest_remnant_fraction(q_r: f64, q_star: f64) -> f64 {
    let x = q_r / q_star.max(1e-30);
    if x < 1.8 {
        (1.0 - 0.5 * x).clamp(0.1, 1.0)
    } else {
        (0.1 * (x / 1.8).dpowf(-1.5)).max(0.002)
    }
}

/// Whether a collision should shatter rather than merge, and how much survives.
pub fn outcome(m_target: f64, m_impactor: f64, speed: f64) -> (bool, f64) {
    let m_tot = m_target + m_impactor;
    let mu = m_target * m_impactor / m_tot;
    let q_r = 0.5 * mu * speed * speed / m_tot;
    let q_star = disruption_threshold(m_tot);
    let f = largest_remnant_fraction(q_r, q_star);
    // Small cratering impacts (a few % of the mass leaves at most) are merges.
    (f < 0.97, f)
}

impl Universe {
    /// After a collision between solid bodies `i` (survivor, already holding the merged mass
    /// and motion) and the impactor: break off debris if the impact was energetic enough.
    pub(crate) fn maybe_fragment(&mut self, s: usize, i: usize, m_t: f64, m_i: f64, c: &cosmogon_physics::nbody::Contact, t_now: f64) {
        let sys = &self.systems[s];
        if sys.dynamics.is_none() || !sys.bodies[i].kind.has_surface() {
            return;
        }
        let speed = c.rel_vel.length();
        let (mut shatter, f) = outcome(m_t, m_i, speed);
        // Only worlds shatter: below ~10²⁰ kg material strength (not modelled) dominates, and
        // debris doesn't re-shatter (no runaway cascades); a system holds at most 150 pieces.
        let debris_now = sys.bodies.iter().filter(|b| b.exists() && b.class == Some(ObjectClass::DebrisField)).count();
        if m_t + m_i < 1.0e20 || sys.bodies[i].class == Some(ObjectClass::DebrisField) || debris_now > 150 {
            shatter = false;
        }
        let m_tot = m_t + m_i;
        let binding = 0.6 * G * m_tot * m_tot / sys.bodies[i].radius.max(1.0);
        let energy = 0.5 * m_t * m_i / m_tot * speed * speed;
        if energy / binding > 1e-4 {
            let b = &mut self.systems[s].bodies[i];
            b.melt = Some(Melt { since: c.time, severity: energy / binding });
            // A global magma ocean boils the seas into a rock-and-steam atmosphere.
            if energy / binding > 0.01 {
                b.hydro.ocean_fraction = 0.0;
                b.hydro.ice_fraction = 0.0;
                b.temperature = b.temperature.max(1500.0);
            }
        }
        if !shatter {
            return;
        }
        let sys = &mut self.systems[s];
        let rho = sys.bodies[i].density().clamp(500.0, 8000.0);
        let m_lr = m_tot * f;
        let ejecta = m_tot - m_lr;
        let survivor = sys.body_state(i, t_now);
        let r_lr = (3.0 * m_lr / (4.0 * std::f64::consts::PI * rho)).dcbrt();
        let v_esc = (2.0 * G * m_lr / r_lr).sqrt();
        // Fragment masses: a power law, at most 24 pieces, none below 0.05 % of the total.
        let n = ((ejecta / (0.004 * m_tot)).ceil() as usize).clamp(3, 24);
        let weights: Vec<f64> = (1..=n).map(|k| (k as f64).dpowf(-1.4)).collect();
        let wsum: f64 = weights.iter().sum();
        let mut rng = Rng::stream(self.settings.seed, crate::rng::domain::CONTACT, &[s as u64, i as u64, c.time.to_bits()]);
        // Shear direction: the impactor's motion across the target's surface.
        let normal = c.rel_pos.normalize();
        let tangent = {
            let tv = c.rel_vel - normal * c.rel_vel.dot(normal);
            if tv.length() > 1e-6 { tv.normalize() } else { normal.cross(crate::Vec3d::new(0.0, 0.0, 1.0)).normalize() }
        };
        let target_name = sys.bodies[i].name.clone();
        let template = sys.bodies[i].clone();
        let mut pieces: Vec<(Body, State)> = Vec::new();
        let mut p_sum = crate::Vec3d::ZERO;
        let mut m_sum = 0.0;
        for (k, w) in weights.iter().enumerate() {
            let m = ejecta * w / wsum;
            let r = (3.0 * m / (4.0 * std::f64::consts::PI * rho)).dcbrt();
            let rand = crate::Vec3d::new(rng.normal(0.0, 1.0), rng.normal(0.0, 1.0), rng.normal(0.0, 1.0)).normalize();
            let dir = (rand * 0.8 + normal * 0.6 + tangent * 0.9).normalize();
            let place = (rand + normal).normalize();
            let pos = survivor.pos + place * (r_lr + r) * 1.6;
            let v = v_esc * rng.range(0.75, 1.6);
            let vel = dir * v + tangent * v * 0.3;
            let mut b = template.clone();
            b.name = format!("{target_name} debris {}", k + 1);
            b.mass = m;
            b.radius = r;
            b.class = Some(ObjectClass::DebrisField);
            b.kind = if template.kind == BodyKind::Icy { BodyKind::Icy } else { BodyKind::Rocky };
            b.parent = None;
            b.atmosphere = Default::default();
            b.hydro = Default::default();
            b.rings = None;
            b.real = false;
            b.elevation_data = None;
            b.deposits.clear();
            b.impacts.clear();
            b.impact_winter = None;
            b.terrain_seed = rng.next_u64();
            b.rotation_period = rng.range(3.0, 30.0) * 3600.0;
            b.tidally_locked = false;
            b.melt = Some(Melt { since: c.time, severity: (energy / binding).max(0.02) });
            b.provenance = Default::default();
            b.provenance.source = format!("Debris from the collision that shattered {target_name}");
            b.mark("mass", Quality::Estimated);
            p_sum += vel * m;
            m_sum += m;
            pieces.push((b, State { pos, vel }));
        }
        // Conserve momentum: debris moves symmetrically about the remnant.
        let drift = p_sum / m_sum.max(1e-30);
        let base = sys.bodies.len();
        for (k, (b, mut st)) in pieces.into_iter().enumerate() {
            st.vel = survivor.vel + (st.vel - drift);
            let mut b = b;
            b.id = (base + k) as u32;
            sys.bodies.push(b);
            sys.dynamics.as_mut().unwrap().bodies.push(Some(st));
        }
        // The remnant loses the ejected mass and shrinks.
        let rem = &mut sys.bodies[i];
        rem.mass = m_lr;
        rem.radius = r_lr.max(rem.radius * f.dcbrt());
        rem.mark("mass", Quality::Estimated);
        self.history.push(Event {
            time: c.time,
            category: Category::Astronomy,
            importance: 5,
            title: format!("{target_name} shattered"),
            detail: format!(
                "The impact at {:.1} km/s exceeds {:.0}% of the disruption energy: {:.0}% of the mass survives, the rest flies off as {} large fragments at up to {:.1} km/s.",
                speed / 1000.0,
                100.0 * (1.0 - f) * 2.0,
                f * 100.0,
                n,
                v_esc * 1.6 / 1000.0
            ),
            system: Some(s as u32),
            body: Some(i as u32),
            civ: None,
        });
    }

    /// Moons that stray inside their planet's Roche limit are torn apart into rings.
    pub(crate) fn roche_disruptions(&mut self, s: usize, t: f64) {
        let sys = &self.systems[s];
        if sys.dynamics.is_none() {
            return;
        }
        let mut victims = Vec::new();
        for (j, b) in sys.bodies.iter().enumerate() {
            let Some(p) = b.parent else { continue };
            let parent = &sys.bodies[p as usize];
            if !b.exists() || !parent.exists() || b.kind.is_stellar() || b.mass * 50.0 > parent.mass {
                continue;
            }
            let roche = crate::sandbox::roche_limit(parent.radius, parent.density(), b.density());
            let d = (sys.body_local_position(j, t) - sys.body_local_position(p as usize, t)).length();
            if d < roche && d > parent.radius {
                victims.push((j, p as usize, roche, d));
            }
        }
        for (j, p, roche, d) in victims {
            let name = self.systems[s].bodies[j].name.clone();
            let pname = self.systems[s].bodies[p].name.clone();
            let sys = &mut self.systems[s];
            let parent_r = sys.bodies[p].radius;
            let opacity = (sys.bodies[j].mass / 1e19).log10().clamp(0.2, 1.0);
            let rings = sys.bodies[p].rings.clone();
            sys.bodies[p].rings = Some(match rings {
                Some(r) => crate::astro::Rings { inner: r.inner.min(parent_r * 1.2), outer: r.outer.max(roche), opacity: r.opacity.max(opacity) },
                None => crate::astro::Rings { inner: (d * 0.6).max(parent_r * 1.15), outer: roche, opacity },
            });
            let ring_mass = sys.bodies[j].mass;
            sys.bodies[p].mass += ring_mass;
            sys.bodies[j].removed = Some(crate::astro::Removal { time: t, cause: crate::astro::RemovalCause::MergedInto(Some(p as u32)) });
            if let Some(dy) = sys.dynamics.as_mut() {
                dy.bodies[j] = None;
            }
            self.on_body_destroyed(s, j, t, &format!("was torn apart by the tides of {pname}"));
            self.history.push(Event {
                time: t,
                category: Category::Astronomy,
                importance: 5,
                title: format!("{name} torn into rings around {pname}"),
                detail: format!("Inside the Roche limit ({:.0} km) tides beat its own gravity; its pieces spread into a ring.", roche / 1000.0),
                system: Some(s as u32),
                body: Some(p as u32),
                civ: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::EARTH_MASS;

    #[test]
    fn gentle_impacts_merge_and_violent_ones_shatter() {
        // Chicxulub on Earth: a scratch.
        assert!(!outcome(EARTH_MASS, 1.0e15, 20_000.0).0);
        // A Mars-sized body at 10 km/s: a giant impact that throws off debris but leaves most.
        let (shatter, f) = outcome(EARTH_MASS, 0.1 * EARTH_MASS, 10_000.0);
        assert!(shatter && f > 0.6, "{f}");
        // Two Earths head-on at 40 km/s: catastrophic.
        let (s2, f2) = outcome(EARTH_MASS, EARTH_MASS, 40_000.0);
        assert!(s2 && f2 < 0.5, "{f2}");
    }

    #[test]
    fn disruption_threshold_matches_published_scale() {
        // Leinhardt & Stewart: ~10⁷ J/kg for Earth-mass combined bodies within a factor ~3.
        let q = disruption_threshold(2.0 * EARTH_MASS);
        assert!((3e6..3e8).contains(&q), "{q}");
    }

    #[test]
    fn melt_cools() {
        let m = Melt { since: 0.0, severity: 0.5 };
        assert!(m.glow(0.0) > 0.9);
        assert!(m.glow(m.timescale() * 5.0) < 0.01);
    }
}
