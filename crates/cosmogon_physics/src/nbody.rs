//! Newtonian N-body integration with close-encounter substepping and collision detection.
//!
//! **Integrator.** Fourth-order symplectic composition (Yoshida 1990; also Forest & Ruth
//! 1990) of the drift–kick–drift leapfrog. Three force evaluations per step; energy error
//! is bounded (no secular drift) for a fixed step, and the local error scales as `h⁵`.
//! See `docs/PHYSICS_ENGINE.md` for why this was chosen over Wisdom–Holman, IAS15 and RK.
//!
//! **Steps.** The caller advances in fixed *macro* steps on an absolute time grid so the
//! result never depends on how the work is split across frames. Each macro step is
//! divided into `n` equal substeps, where `n` follows from the shortest dynamical
//! timescale among all pairs over the coming step (orbital free-fall time and flyby time,
//! evaluated at the predicted closest approach). Quiet systems take one substep;
//! encounters automatically get as many as they need (up to `max_substeps`).
//!
//! **Collisions.** Before each substep every pair is swept along its relative velocity; a
//! predicted overlap of the two spheres within the substep is a contact. Contacts are
//! resolved immediately after that substep as a perfectly inelastic merger (mass and
//! linear momentum conserved). Richer outcomes (fragmentation, cratering, bounce) are
//! decided by the caller from the [`Contact`] record.
//!
//! **Relativity (optional).** A first post-Newtonian correction for test bodies around
//! the most massive body (Schwarzschild, harmonic gauge). It reproduces Mercury's
//! 43″/century perihelion advance. Not a general-relativity solver.
//!
//! Units are SI throughout: metres, seconds, m/s, and `gm = G·m` in m³/s².

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use cosmogon_core::math::Vec3d;

/// Speed of light (m/s), for the post-Newtonian term.
const C: f64 = 2.997_924_58e8;

// Yoshida (1990) fourth-order coefficients, written out so every platform uses the
// identical values. w1 = 1/(2 − 2^(1/3)), w0 = −2^(1/3)/(2 − 2^(1/3)).
const W1: f64 = 1.351_207_191_959_657_6;
const W0: f64 = -1.702_414_383_919_315_3;
const DRIFT: [f64; 4] = [W1 * 0.5, (W0 + W1) * 0.5, (W0 + W1) * 0.5, W1 * 0.5];
const KICK: [f64; 3] = [W1, W0, W1];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub pos: Vec3d,
    pub vel: Vec3d,
    /// Gravitational parameter G·m (m³/s²).
    pub gm: f64,
    /// Collision radius (m). 0 = point mass that never collides.
    pub radius: f64,
    /// False once merged into another particle; dead particles are ignored.
    pub alive: bool,
}

impl Particle {
    pub fn new(pos: Vec3d, vel: Vec3d, gm: f64, radius: f64) -> Self {
        Self { pos, vel, gm, radius, alive: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Accuracy parameter: substep length = `eta` × shortest dynamical timescale.
    /// 2π/`eta` ≈ substeps per orbit for the tightest bound pair.
    pub eta: f64,
    pub max_substeps: u32,
    /// Apply the 1PN correction from the most massive body.
    pub relativity: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { eta: std::f64::consts::TAU / 120.0, max_substeps: 4096, relativity: false }
    }
}

/// A collision found during a step. `a` survives (the more massive), `b` was absorbed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    /// Time of contact (s, same clock as the step).
    pub time: f64,
    /// Position of `b` relative to `a` at contact (length ≈ sum of radii).
    pub rel_pos: Vec3d,
    /// Velocity of `b` relative to `a` just before contact.
    pub rel_vel: Vec3d,
    pub gm_a: f64,
    pub gm_b: f64,
    pub radius_a: f64,
    pub radius_b: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepReport {
    pub substeps: u32,
    /// The encounter needed more than `max_substeps`: accuracy is reduced.
    pub saturated: bool,
    pub contacts: Vec<Contact>,
}

/// Index of the most massive live particle.
pub fn dominant(ps: &[Particle]) -> Option<usize> {
    ps.iter().enumerate().filter(|(_, p)| p.alive).max_by(|a, b| a.1.gm.total_cmp(&b.1.gm)).map(|(i, _)| i)
}

/// Gravitational accelerations on every particle (pairwise, deterministic order).
pub fn accelerations(ps: &[Particle], relativity: bool, out: &mut Vec<Vec3d>) {
    out.clear();
    out.resize(ps.len(), Vec3d::ZERO);
    let n = ps.len();
    for i in 0..n {
        if !ps[i].alive {
            continue;
        }
        for j in (i + 1)..n {
            if !ps[j].alive {
                continue;
            }
            let r = ps[j].pos - ps[i].pos;
            let d2 = r.length_squared();
            if d2 == 0.0 {
                continue;
            }
            let inv3 = 1.0 / (d2 * d2.sqrt());
            out[i] += r * (ps[j].gm * inv3);
            out[j] -= r * (ps[i].gm * inv3);
        }
    }
    if relativity {
        if let Some(s) = dominant(ps) {
            let gm = ps[s].gm;
            for i in 0..n {
                if i == s || !ps[i].alive {
                    continue;
                }
                let r = ps[i].pos - ps[s].pos;
                let v = ps[i].vel - ps[s].vel;
                let d = r.length();
                if d == 0.0 {
                    continue;
                }
                let k = gm / (C * C * d * d * d);
                out[i] += (r * (4.0 * gm / d - v.length_squared()) + v * (4.0 * r.dot(v))) * k;
            }
        }
    }
}

/// Shortest dynamical timescale (s) among all pairs over the next `horizon` seconds:
/// the free-fall time `√(d³/GM)` and the flyby time `d/v` at the predicted (linear)
/// closest approach `d`, never less than the contact distance.
pub fn shortest_timescale(ps: &[Particle], horizon: f64) -> f64 {
    let mut tau = f64::INFINITY;
    let n = ps.len();
    for i in 0..n {
        if !ps[i].alive {
            continue;
        }
        for j in (i + 1)..n {
            if !ps[j].alive {
                continue;
            }
            let r = ps[j].pos - ps[i].pos;
            let v = ps[j].vel - ps[i].vel;
            let v2 = v.length_squared();
            let t_close = if v2 > 0.0 { (-r.dot(v) / v2).clamp(0.0, horizon) } else { 0.0 };
            let d = (r + v * t_close).length().max(ps[i].radius + ps[j].radius).max(1.0);
            let gm = ps[i].gm + ps[j].gm;
            if gm > 0.0 {
                tau = tau.min((d * d * d / gm).sqrt());
            }
            if v2 > 0.0 {
                tau = tau.min(d / v2.sqrt());
            }
        }
    }
    tau
}

/// The substep length that `settings` asks for in the current configuration.
pub fn natural_step(ps: &[Particle], settings: &Settings) -> f64 {
    settings.eta * shortest_timescale(ps, 0.0)
}

/// Swept-sphere test for every pair over `h`: earliest predicted contact, if any.
fn find_contact(ps: &[Particle], h: f64) -> Option<(usize, usize, f64)> {
    let mut best: Option<(usize, usize, f64)> = None;
    let n = ps.len();
    for i in 0..n {
        if !ps[i].alive || ps[i].radius <= 0.0 {
            continue;
        }
        for j in (i + 1)..n {
            if !ps[j].alive || ps[j].radius <= 0.0 {
                continue;
            }
            let r = ps[j].pos - ps[i].pos;
            let v = ps[j].vel - ps[i].vel;
            let rr = ps[i].radius + ps[j].radius;
            // Solve |r + v t| = rr for the first t in [0, h].
            let a = v.length_squared();
            let b = 2.0 * r.dot(v);
            let c = r.length_squared() - rr * rr;
            let t = if c <= 0.0 {
                0.0
            } else if a == 0.0 || b >= 0.0 {
                continue;
            } else {
                let disc = b * b - 4.0 * a * c;
                if disc < 0.0 {
                    continue;
                }
                (-b - disc.sqrt()) / (2.0 * a)
            };
            if t <= h && best.is_none_or(|(_, _, bt)| t < bt) {
                best = Some((i, j, t));
            }
        }
    }
    best
}

/// Merge `b` into `a` (perfectly inelastic): mass, momentum and centre of mass conserved;
/// radius from volume conservation.
fn merge(ps: &mut [Particle], a: usize, b: usize) {
    let (pa, pb) = (ps[a], ps[b]);
    let gm = pa.gm + pb.gm;
    let (wa, wb) = if gm > 0.0 { (pa.gm / gm, pb.gm / gm) } else { (0.5, 0.5) };
    ps[a].pos = pa.pos * wa + pb.pos * wb;
    ps[a].vel = pa.vel * wa + pb.vel * wb;
    ps[a].gm = gm;
    ps[a].radius = (pa.radius * pa.radius * pa.radius + pb.radius * pb.radius * pb.radius).dcbrt();
    ps[b].alive = false;
    ps[b].gm = 0.0;
}

fn substep(ps: &mut [Particle], h: f64, relativity: bool, acc: &mut Vec<Vec3d>) {
    for k in 0..4 {
        let d = DRIFT[k] * h;
        for p in ps.iter_mut().filter(|p| p.alive) {
            p.pos += p.vel * d;
        }
        if k < 3 {
            accelerations(ps, relativity, acc);
            let kick = KICK[k] * h;
            for (p, a) in ps.iter_mut().zip(acc.iter()) {
                if p.alive {
                    p.vel += *a * kick;
                }
            }
        }
    }
}

/// Advance all particles by one macro step `dt` starting at time `t`.
pub fn step(ps: &mut [Particle], t: f64, dt: f64, settings: &Settings) -> StepReport {
    let mut acc = Vec::with_capacity(ps.len());
    let tau = shortest_timescale(ps, dt);
    let wanted = (dt / (settings.eta * tau)).ceil();
    let n = if wanted.is_finite() { (wanted as u32).clamp(1, settings.max_substeps.max(1)) } else { 1 };
    let mut report = StepReport { substeps: n, saturated: wanted > settings.max_substeps as f64, contacts: Vec::new() };
    let h = dt / n as f64;
    for k in 0..n {
        let t0 = t + k as f64 * h;
        let contact = find_contact(ps, h);
        // Record the geometry at contact before the step moves the bodies on.
        let record = contact.map(|(i, j, tc)| {
            let (a, b) = if ps[i].gm >= ps[j].gm { (i, j) } else { (j, i) };
            let rel_vel = ps[b].vel - ps[a].vel;
            Contact {
                a,
                b,
                time: t0 + tc,
                rel_pos: (ps[b].pos - ps[a].pos) + rel_vel * tc,
                rel_vel,
                gm_a: ps[a].gm,
                gm_b: ps[b].gm,
                radius_a: ps[a].radius,
                radius_b: ps[b].radius,
            }
        });
        if let Some(c) = record {
            // Merge at contact, then let the merged body take the whole substep.
            for p in ps.iter_mut().filter(|p| p.alive) {
                p.pos += p.vel * (c.time - t0);
            }
            merge(ps, c.a, c.b);
            for p in ps.iter_mut().filter(|p| p.alive) {
                p.pos -= p.vel * (c.time - t0);
            }
            report.contacts.push(c);
        }
        substep(ps, h, settings.relativity, &mut acc);
    }
    report
}

/// Total energy (kinetic + potential) in units of m⁵/s⁴ / G — i.e. computed with `gm`
/// in place of mass. Proportional to the true energy, which is all conservation tests need.
pub fn energy(ps: &[Particle]) -> f64 {
    let mut e = 0.0;
    for (i, p) in ps.iter().enumerate().filter(|(_, p)| p.alive) {
        e += 0.5 * p.gm * p.vel.length_squared();
        for q in ps[i + 1..].iter().filter(|q| q.alive) {
            let d = (q.pos - p.pos).length();
            if d > 0.0 {
                e -= p.gm * q.gm / d;
            }
        }
    }
    e
}

/// Total linear momentum (×G).
pub fn momentum(ps: &[Particle]) -> Vec3d {
    ps.iter().filter(|p| p.alive).fold(Vec3d::ZERO, |m, p| m + p.vel * p.gm)
}

/// Total angular momentum about the origin (×G).
pub fn angular_momentum(ps: &[Particle]) -> Vec3d {
    ps.iter().filter(|p| p.alive).fold(Vec3d::ZERO, |l, p| l + p.pos.cross(p.vel) * p.gm)
}

/// Shift to the centre-of-mass frame (zero total momentum, centre of mass at the origin).
pub fn to_barycentric(ps: &mut [Particle]) {
    let gm: f64 = ps.iter().filter(|p| p.alive).map(|p| p.gm).sum();
    if gm <= 0.0 {
        return;
    }
    let com = ps.iter().filter(|p| p.alive).fold(Vec3d::ZERO, |c, p| c + p.pos * p.gm) / gm;
    let vcom = momentum(ps) / gm;
    for p in ps.iter_mut() {
        p.pos -= com;
        p.vel -= vcom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbital_elements::state_vectors_to_elements;

    const GM_SUN: f64 = 1.327_124_400_18e20;
    const GM_EARTH: f64 = 3.986_004_418e14;
    const GM_MOON: f64 = 4.904_869_5e12;
    const AU: f64 = 1.495_978_707e11;
    const DAY: f64 = 86_400.0;
    const YEAR: f64 = 365.25 * DAY;

    fn run(ps: &mut [Particle], dt: f64, steps: usize, s: &Settings) -> Vec<Contact> {
        let mut contacts = Vec::new();
        for k in 0..steps {
            contacts.extend(step(ps, k as f64 * dt, dt, s).contacts);
        }
        contacts
    }

    fn rel(a: f64, b: f64) -> f64 {
        ((a - b) / b).abs()
    }

    /// Circular two-body orbit: returns to its start after one period; energy conserved.
    #[test]
    fn two_body_circular_period_and_energy() {
        let v = (GM_SUN / AU).sqrt();
        let mut ps = vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_SUN, 0.0), Particle::new(Vec3d::new(AU, 0.0, 0.0), Vec3d::new(0.0, v, 0.0), 0.0, 0.0)];
        let period = std::f64::consts::TAU * (AU.powi(3) / GM_SUN).sqrt();
        let steps = 2000;
        let e0 = 0.5 * v * v - GM_SUN / AU;
        run(&mut ps, period / steps as f64, steps, &Settings::default());
        let err = (ps[1].pos - Vec3d::new(AU, 0.0, 0.0)).length() / AU;
        assert!(err < 1e-9, "position error after one period: {err:e}");
        let e1 = 0.5 * ps[1].vel.length_squared() - GM_SUN / ps[1].pos.length();
        assert!(rel(e1, e0) < 1e-12, "{e1} vs {e0}");
    }

    /// Sun–Earth over a century: energy and angular momentum stay bounded, momentum exact.
    #[test]
    fn sun_earth_century_conservation() {
        let v = (GM_SUN / AU).sqrt();
        let mut ps = vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_SUN, 6.957e8), Particle::new(Vec3d::new(AU, 0.0, 0.0), Vec3d::new(0.0, v * 1.0167, 0.0), GM_EARTH, 6.371e6)];
        to_barycentric(&mut ps);
        let (e0, l0, p0) = (energy(&ps), angular_momentum(&ps), momentum(&ps));
        let s = Settings::default();
        let dt = natural_step(&ps, &s);
        let steps = (100.0 * YEAR / dt) as usize;
        run(&mut ps, dt, steps, &s);
        assert!(rel(energy(&ps), e0) < 1e-9, "energy drift {:e}", rel(energy(&ps), e0));
        assert!((angular_momentum(&ps) - l0).length() / l0.length() < 1e-10);
        assert!((momentum(&ps) - p0).length() < 1e-6 * l0.length() / AU);
    }

    /// Sun–Earth–Moon: the Moon stays bound to Earth with its real distance range for a decade.
    #[test]
    fn earth_moon_stays_bound() {
        let ve = (GM_SUN / AU).sqrt();
        let a_moon = 384_400e3;
        let vm = ((GM_EARTH + GM_MOON) / a_moon).sqrt();
        let mut ps = vec![
            Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_SUN, 6.957e8),
            Particle::new(Vec3d::new(AU, 0.0, 0.0), Vec3d::new(0.0, ve, 0.0), GM_EARTH, 6.371e6),
            Particle::new(Vec3d::new(AU + a_moon, 0.0, 0.0), Vec3d::new(0.0, ve + vm, 0.0), GM_MOON, 1.737e6),
        ];
        to_barycentric(&mut ps);
        let e0 = energy(&ps);
        let s = Settings::default();
        let dt = natural_step(&ps, &s);
        let (mut lo, mut hi) = (f64::MAX, 0.0f64);
        let steps = (10.0 * YEAR / dt) as usize;
        for k in 0..steps {
            step(&mut ps, k as f64 * dt, dt, &s);
            let d = (ps[2].pos - ps[1].pos).length();
            lo = lo.min(d);
            hi = hi.max(d);
        }
        assert!(lo > 330_000e3 && hi < 440_000e3, "Moon distance range {lo:e}..{hi:e}");
        assert!(rel(energy(&ps), e0) < 1e-8, "energy drift {:e}", rel(energy(&ps), e0));
    }

    /// Eccentric equal-mass binary star: 100 orbits, energy and angular momentum conserved.
    #[test]
    fn binary_star_conservation() {
        let gm = GM_SUN;
        let a = 0.5 * AU;
        let e = 0.5;
        // Each star at apoapsis of its orbit about the barycentre.
        let r_apo = a * (1.0 + e);
        let v_rel = (2.0 * gm * (1.0 - e) / (a * (1.0 + e))).sqrt();
        let mut ps = vec![
            Particle::new(Vec3d::new(-r_apo / 2.0, 0.0, 0.0), Vec3d::new(0.0, -v_rel / 2.0, 0.0), gm, 0.0),
            Particle::new(Vec3d::new(r_apo / 2.0, 0.0, 0.0), Vec3d::new(0.0, v_rel / 2.0, 0.0), gm, 0.0),
        ];
        let start = ps.clone();
        let period = std::f64::consts::TAU * (a.powi(3) / (2.0 * gm)).sqrt();
        // (steps per orbit, energy tolerance): Balanced and Accurate presets.
        for (per_orbit, tol) in [(120.0, 1e-4), (400.0, 1e-6)] {
            let mut ps = start.clone();
            let (e0, l0) = (energy(&ps), angular_momentum(&ps));
            let s = Settings { eta: std::f64::consts::TAU / per_orbit, ..Default::default() };
            let dt = period / 50.0; // substepping handles periapsis
            run(&mut ps, dt, 50 * 100, &s);
            let err = rel(energy(&ps), e0);
            println!("binary e=0.5, {per_orbit} steps/orbit: energy error after 100 orbits {err:.2e}");
            assert!(err < tol, "energy {err:e}");
            assert!((angular_momentum(&ps) - l0).length() / l0.length() < 1e-9);
            // Back at apoapsis after an integer number of periods.
            let sep = (ps[1].pos - ps[0].pos).length();
            assert!(rel(sep, r_apo) < 1e-3, "separation {sep:e} vs {r_apo:e}");
        }
    }

    /// Halley-like comet (e = 0.967): adaptive substeps keep the perihelion passage accurate.
    #[test]
    fn high_eccentricity_comet() {
        let a = 17.8 * AU;
        let e = 0.967;
        let r_apo = a * (1.0 + e);
        let v_apo = (GM_SUN * (1.0 - e) / (a * (1.0 + e))).sqrt();
        let e0 = 0.5 * v_apo * v_apo - GM_SUN / r_apo;
        let period = std::f64::consts::TAU * (a.powi(3) / GM_SUN).sqrt();
        for (per_orbit, tol) in [(120.0, 2e-5), (400.0, 2e-7)] {
            let mut ps = vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_SUN, 0.0), Particle::new(Vec3d::new(r_apo, 0.0, 0.0), Vec3d::new(0.0, v_apo, 0.0), 0.0, 0.0)];
            let s = Settings { eta: std::f64::consts::TAU / per_orbit, ..Default::default() };
            let dt = 5.0 * DAY;
            let steps = (3.0 * period / dt).round() as usize;
            let mut q = f64::MAX;
            for k in 0..steps {
                let r = step(&mut ps, k as f64 * dt, dt, &s);
                assert!(!r.saturated);
                q = q.min(ps[1].pos.length());
            }
            let el = state_vectors_to_elements(ps[1].pos, ps[1].vel, GM_SUN);
            let e1 = 0.5 * ps[1].vel.length_squared() - GM_SUN / ps[1].pos.length();
            println!("comet e=0.967, {per_orbit} steps/orbit: energy error after 3 orbits {:.2e}, e error {:.2e}", rel(e1, e0), rel(el.eccentricity, e));
            assert!(rel(el.eccentricity, e) < tol, "e = {}", el.eccentricity);
            assert!(rel(el.semi_major_axis, a) < tol, "a = {:e}", el.semi_major_axis);
            assert!(q < a * (1.0 - e) * 1.05, "perihelion sampled at {q:e}");
            assert!(rel(e1, e0) < tol);
        }
    }

    /// The figure-eight three-body choreography (Chenciner & Montgomery 2000) is periodic.
    #[test]
    fn three_body_figure_eight_is_periodic() {
        let p1 = Vec3d::new(0.970_004_36, -0.243_087_53, 0.0);
        let v3 = Vec3d::new(-0.932_407_37, -0.864_731_46, 0.0);
        let mut ps = vec![Particle::new(p1, v3 * -0.5, 1.0, 0.0), Particle::new(p1 * -1.0, v3 * -0.5, 1.0, 0.0), Particle::new(Vec3d::ZERO, v3, 1.0, 0.0)];
        let start = ps.clone();
        let e0 = energy(&ps);
        let period = 6.325_913_98;
        let steps = 4000;
        run(&mut ps, period / steps as f64, steps, &Settings::default());
        for (a, b) in ps.iter().zip(&start) {
            assert!((a.pos - b.pos).length() < 2e-5, "{:?} vs {:?}", a.pos, b.pos);
        }
        assert!(rel(energy(&ps), e0) < 1e-10);
    }

    /// Hyperbolic flyby: deflection matches the two-body formula tan(δ/2) = GM / (b v∞²).
    #[test]
    fn close_encounter_deflection() {
        let gm = GM_EARTH;
        let b = 20_000e3;
        let v_inf = 5_000.0;
        let x0 = -5.0e9;
        let mut ps = vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, gm, 6.371e6), Particle::new(Vec3d::new(x0, b, 0.0), Vec3d::new(v_inf, 0.0, 0.0), 0.0, 1.0)];
        // Start far out: account for the finite starting distance through the actual
        // hyperbola (energy-consistent speed at infinity).
        let r0 = ps[1].pos.length();
        let v_inf_true = (v_inf * v_inf - 2.0 * gm / r0).sqrt();
        let h = ps[1].pos.cross(ps[1].vel).length();
        let b_true = h / v_inf_true;
        let s = Settings::default();
        let dt = 3600.0;
        let steps = (2.0 * -x0 / v_inf / dt) as usize;
        let contacts = run(&mut ps, dt, steps, &s);
        assert!(contacts.is_empty());
        // Asymptotic direction from the eccentricity: δ = 2·asin(1/e).
        let el = state_vectors_to_elements(ps[1].pos, ps[1].vel, gm);
        let predicted = 2.0 * (gm / (b_true * v_inf_true * v_inf_true)).atan();
        let from_e = 2.0 * (1.0 / el.eccentricity).asin();
        assert!((from_e - predicted).abs() < 1e-6, "{from_e} vs {predicted}");
        // And the real heading change.
        let heading = ps[1].vel.y.atan2(ps[1].vel.x).abs();
        assert!((heading - predicted).abs() < 0.02 * predicted, "heading {heading} vs {predicted}");
    }

    /// A head-on impact is detected even at 20 km/s with a coarse step, and the merger
    /// conserves momentum.
    #[test]
    fn impact_is_detected_and_conserves_momentum() {
        let mut ps = vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_EARTH, 6.371e6), Particle::new(Vec3d::new(2.0e9, 1.0e6, 0.0), Vec3d::new(-20_000.0, 0.0, 0.0), 1.0e3, 5_000.0)];
        let p0 = momentum(&ps);
        let contacts = run(&mut ps, 6.0 * 3600.0, 40, &Settings::default());
        assert_eq!(contacts.len(), 1, "{contacts:?}");
        let c = contacts[0];
        assert_eq!((c.a, c.b), (0, 1));
        assert!(rel(c.rel_pos.length(), 6.376e6) < 1e-3, "contact distance {}", c.rel_pos.length());
        assert!(c.rel_vel.length() > 20_000.0, "gravity accelerates the impactor");
        assert!(!ps[1].alive);
        assert!((momentum(&ps) - p0).length() / p0.length() < 1e-9);
    }

    /// 1PN correction: Mercury's perihelion advances ≈ 43″ per century.
    #[test]
    fn mercury_relativistic_precession() {
        let a = 0.387_098 * AU;
        let e = 0.205_630;
        let r_peri = a * (1.0 - e);
        let v_peri = (GM_SUN * (1.0 + e) / (a * (1.0 - e))).sqrt();
        let mk = || vec![Particle::new(Vec3d::ZERO, Vec3d::ZERO, GM_SUN, 0.0), Particle::new(Vec3d::new(r_peri, 0.0, 0.0), Vec3d::new(0.0, v_peri, 0.0), 0.0, 0.0)];
        let peri_angle = |ps: &[Particle]| {
            let el = state_vectors_to_elements(ps[1].pos, ps[1].vel, GM_SUN);
            el.argument_perihelion + el.longitude_ascending
        };
        let period = std::f64::consts::TAU * (a.powi(3) / GM_SUN).sqrt();
        let steps = (100.0 * YEAR / period).round() as usize * 400;
        let dt = period / 400.0;
        let mut gr = mk();
        run(&mut gr, dt, steps, &Settings { relativity: true, ..Default::default() });
        let mut newton = mk();
        run(&mut newton, dt, steps, &Settings::default());
        let arcsec = (peri_angle(&gr) - peri_angle(&newton)).to_degrees() * 3600.0;
        let per_century = arcsec * 100.0 * YEAR / (steps as f64 * dt);
        assert!((per_century - 42.98).abs() < 1.5, "precession {per_century}″/century");
    }
}
