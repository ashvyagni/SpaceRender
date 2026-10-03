#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use cosmogon_core::math::Vec3d;
use cosmogon_core::constants;

/// Classical Keplerian orbital elements derived from Cartesian state vectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalElements {
    /// Semi-major axis a (meters). Negative for hyperbolic orbits, infinite if parabolic.
    pub semi_major_axis: f64,
    /// Eccentricity e (dimensionless).
    pub eccentricity: f64,
    /// Inclination i (radians), 0..π.
    pub inclination: f64,
    /// Longitude of ascending node Ω (radians). 0 for equatorial orbits.
    pub longitude_ascending: f64,
    /// Argument of periapsis ω (radians). For equatorial orbits this is measured from the
    /// reference x-axis (so Ω + ω is the longitude of periapsis); 0 for circular orbits.
    pub argument_perihelion: f64,
    /// True anomaly ν (radians). For circular orbits: argument of latitude (inclined) or
    /// true longitude (equatorial).
    pub true_anomaly: f64,
    /// Mean anomaly M (radians); hyperbolic mean anomaly for e > 1.
    pub mean_anomaly: f64,
}

impl OrbitalElements {
    pub fn is_bound(&self) -> bool {
        self.eccentricity < 1.0 && self.semi_major_axis > 0.0 && self.semi_major_axis.is_finite()
    }
    pub fn periapsis(&self) -> f64 {
        if self.semi_major_axis.is_finite() {
            self.semi_major_axis * (1.0 - self.eccentricity)
        } else {
            f64::NAN
        }
    }
    pub fn apoapsis(&self) -> f64 {
        if self.is_bound() {
            self.semi_major_axis * (1.0 + self.eccentricity)
        } else {
            f64::INFINITY
        }
    }
}

/// Tolerance below which an orbit counts as circular / equatorial.
const SMALL: f64 = 1e-11;

/// Mean anomaly from the true anomaly.
pub fn true_to_mean_anomaly(nu: f64, e: f64) -> f64 {
    if e < 1.0 {
        let ecc = 2.0 * ((1.0 - e).sqrt() * (nu * 0.5).dsin()).datan2((1.0 + e).sqrt() * (nu * 0.5).dcos());
        (ecc - e * ecc.dsin()).rem_euclid(constants::TWO_PI)
    } else if e > 1.0 {
        let x = ((e - 1.0) / (e + 1.0)).sqrt() * (nu * 0.5).dtan();
        // atanh(x) = ½ ln((1+x)/(1−x))
        let h = 2.0 * 0.5 * ((1.0 + x) / (1.0 - x)).dln();
        e * h.sinh() - h
    } else {
        let d = (nu * 0.5).dtan();
        d + d * d * d / 3.0
    }
}

/// Convert position and velocity state vectors to classical orbital elements.
///
/// Robust for circular, equatorial, retrograde and hyperbolic orbits: every angle comes
/// from `atan2` of projections, so no branch loses a quadrant.
///
/// # Panics
/// Panics if `mu` is zero or negative.
pub fn state_vectors_to_elements(r: Vec3d, v: Vec3d, mu: f64) -> OrbitalElements {
    assert!(mu > 0.0, "gravitational parameter must be positive");

    let r_mag = r.length();
    let h = r.cross(v);
    let h_mag = h.length().max(f64::MIN_POSITIVE);
    let h_hat = h / h_mag;

    // Node vector n = k × h.
    let n = Vec3d::new(-h.y, h.x, 0.0);
    let n_mag = n.length();
    let equatorial = n_mag <= SMALL * h_mag;

    // Eccentricity vector e = (v × h)/μ − r̂.
    let e_vec = v.cross(h) / mu - r / r_mag;
    let e = e_vec.length();
    let circular = e <= SMALL;

    let energy = 0.5 * v.length_squared() - mu / r_mag;
    let semi_major_axis = if (e - 1.0).abs() <= SMALL { f64::INFINITY } else { -mu / (2.0 * energy) };

    let inclination = (h.x * h.x + h.y * h.y).sqrt().datan2(h.z);
    let longitude_ascending = if equatorial { 0.0 } else { n.y.datan2(n.x).rem_euclid(constants::TWO_PI) };

    // Angle of `a` measured from reference direction `from` in the orbital plane.
    let angle = |from: Vec3d, a: Vec3d| from.cross(a).dot(h_hat).datan2(from.dot(a));
    // In-plane reference: the ascending node, or the x-axis seen in the orbit's own sense.
    let reference = if equatorial { Vec3d::new(1.0, 0.0, 0.0) } else { n / n_mag };
    let sense = |x: f64| x;

    let argument_perihelion = if circular { 0.0 } else { sense(angle(reference, e_vec)).rem_euclid(constants::TWO_PI) };
    let true_anomaly = if circular { sense(angle(reference, r)) } else { angle(e_vec, r) }.rem_euclid(constants::TWO_PI);
    let mean_anomaly = true_to_mean_anomaly(true_anomaly, e);

    OrbitalElements { semi_major_axis, eccentricity: e, inclination, longitude_ascending, argument_perihelion, true_anomaly, mean_anomaly }
}

/// Convert classical orbital elements back to position and velocity state vectors.
///
/// # Arguments
/// * `elements` – orbital elements
/// * `mu` – gravitational parameter μ (m³/s²)
///
/// # Returns
/// `(position, velocity)` in the inertial frame.
pub fn elements_to_state_vectors(elements: OrbitalElements, mu: f64) -> (Vec3d, Vec3d) {
    let a = elements.semi_major_axis;
    let e = elements.eccentricity;
    let i = elements.inclination;
    let omega = elements.longitude_ascending;
    let w = elements.argument_perihelion;
    let nu = elements.true_anomaly;

    // Orbital radius
    let p = a * (1.0 - e * e); // semi-latus rectum
    let r = p / (1.0 + e * nu.dcos());

    // Position in perifocal frame
    let r_peri = Vec3d::new(r * nu.dcos(), r * nu.dsin(), 0.0);

    // Velocity in perifocal frame
    let h = (mu * p).sqrt();
    let v_peri = Vec3d::new(
        -mu / h * nu.dsin(),
        mu / h * (e + nu.dcos()),
        0.0,
    );

    // Rotation matrix elements
    let cos_w = w.dcos();
    let sin_w = w.dsin();
    let cos_i = i.dcos();
    let sin_i = i.dsin();
    let cos_o = omega.dcos();
    let sin_o = omega.dsin();

    // Perifocal → inertial transformation
    let position = Vec3d::new(
        (cos_o * cos_w - sin_o * sin_w * cos_i) * r_peri.x
            + (-cos_o * sin_w - sin_o * cos_w * cos_i) * r_peri.y,
        (sin_o * cos_w + cos_o * sin_w * cos_i) * r_peri.x
            + (-sin_o * sin_w + cos_o * cos_w * cos_i) * r_peri.y,
        (sin_w * sin_i) * r_peri.x + (cos_w * sin_i) * r_peri.y,
    );

    let velocity = Vec3d::new(
        (cos_o * cos_w - sin_o * sin_w * cos_i) * v_peri.x
            + (-cos_o * sin_w - sin_o * cos_w * cos_i) * v_peri.y,
        (sin_o * cos_w + cos_o * sin_w * cos_i) * v_peri.x
            + (-sin_o * sin_w + cos_o * cos_w * cos_i) * v_peri.y,
        (sin_w * sin_i) * v_peri.x + (cos_w * sin_i) * v_peri.y,
    );

    (position, velocity)
}

/// Compute delta-v requirements for a Hohmann transfer between two circular orbits.
///
/// Returns `(delta_v1, delta_v2)` where:
/// - `delta_v1` is the impulse to enter the transfer ellipse from the inner orbit
/// - `delta_v2` is the impulse to circularise at the outer orbit
///
/// # Arguments
/// * `r1` – radius of the inner orbit (meters)
/// * `r2` – radius of the outer orbit (meters)
/// * `mu` – gravitational parameter μ (m³/s²)
///
/// # Returns
/// `(delta_v1, delta_v2)` in m/s.
///
/// # Panics
/// Panics if `r1` or `r2` is non-positive.
pub fn hohmann_transfer(r1: f64, r2: f64, mu: f64) -> (f64, f64) {
    assert!(r1 > 0.0 && r2 > 0.0, "orbital radii must be positive");

    let a_transfer = (r1 + r2) * 0.5;

    let v1_circular = (mu / r1).sqrt();
    let v2_circular = (mu / r2).sqrt();

    let v_transfer_perihelion = (mu * (2.0 / r1 - 1.0 / a_transfer)).sqrt();
    let v_transfer_apohelion = (mu * (2.0 / r2 - 1.0 / a_transfer)).sqrt();

    let delta_v1 = (v_transfer_perihelion - v1_circular).abs();
    let delta_v2 = (v2_circular - v_transfer_apohelion).abs();

    (delta_v1, delta_v2)
}

/// Escape velocity from a body of given mass at a given radius.
///
/// ```text
/// v_esc = sqrt(2 * G * M / r) = sqrt(2 * μ / r)
/// ```
///
/// # Arguments
/// * `mass` – mass of the central body (kg)
/// * `radius` – distance from centre (meters)
///
/// # Returns
/// Escape velocity (m/s).
pub fn escape_velocity(mass: f64, radius: f64) -> f64 {
    (2.0 * constants::G * mass / radius).sqrt()
}

/// Orbital (specific mechanical) energy at a given radius in a Keplerian orbit.
///
/// ```text
/// ε = v²/2 - μ/r = -μ/(2a)
/// ```
///
/// # Arguments
/// * `mass` – mass of the central body (kg)
/// * `semi_major` – semi-major axis a (meters)
/// * `radius` – current orbital radius r (meters)
///
/// # Returns
/// Specific orbital energy (J/kg).
pub fn orbital_energy(mass: f64, semi_major: f64, _radius: f64) -> f64 {
    let mu = constants::G * mass;
    -mu / (2.0 * semi_major)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn circular_orbit_roundtrip() {
        // Circular orbit in the xy-plane
        let mu: f64 = 1.0;
        let r: f64 = 1.0;
        let v_circ = (mu / r).sqrt();

        let pos = Vec3d::new(r, 0.0, 0.0);
        let vel = Vec3d::new(0.0, v_circ, 0.0);

        let elements = state_vectors_to_elements(pos, vel, mu);

        assert!(approx_eq(elements.eccentricity, 0.0));
        assert!(approx_eq(elements.semi_major_axis, r));

        // Roundtrip
        let (pos2, vel2) = elements_to_state_vectors(elements, mu);
        assert!(approx_eq(pos2.x, pos.x));
        assert!(approx_eq(pos2.y, pos.y));
        assert!(approx_eq(pos2.z, pos.z));
        assert!(approx_eq(vel2.x, vel.x));
        assert!(approx_eq(vel2.y, vel.y));
    }

    fn assert_roundtrip(pos: Vec3d, vel: Vec3d, mu: f64) {
        let el = state_vectors_to_elements(pos, vel, mu);
        let (p2, v2) = elements_to_state_vectors(el, mu);
        let scale_r = pos.length();
        let scale_v = vel.length();
        assert!((p2 - pos).length() / scale_r < 1e-10, "{el:?}: {p2:?} vs {pos:?}");
        assert!((v2 - vel).length() / scale_v < 1e-10, "{el:?}: {v2:?} vs {vel:?}");
    }

    #[test]
    fn roundtrip_inclined_eccentric_retrograde_equatorial() {
        let mu = 1.327e20;
        let r = Vec3d::new(1.1e11, -4.0e10, 2.0e10);
        assert_roundtrip(r, Vec3d::new(9_000.0, 31_000.0, 4_000.0), mu); // inclined, eccentric
        assert_roundtrip(r, Vec3d::new(-9_000.0, -31_000.0, 4_000.0), mu); // retrograde
        assert_roundtrip(Vec3d::new(1.5e11, 2.0e10, 0.0), Vec3d::new(-3_000.0, 33_000.0, 0.0), mu); // prograde equatorial
        assert_roundtrip(Vec3d::new(1.5e11, 2.0e10, 0.0), Vec3d::new(3_000.0, -33_000.0, 0.0), mu); // retrograde equatorial
    }

    #[test]
    fn node_and_mean_anomaly_are_correct() {
        // Build from known elements and recover them.
        let mu = 3.986e14;
        let el = OrbitalElements { semi_major_axis: 2.4e7, eccentricity: 0.3, inclination: 0.5, longitude_ascending: 1.2, argument_perihelion: 2.1, true_anomaly: 0.7, mean_anomaly: 0.0 };
        let (p, v) = elements_to_state_vectors(el, mu);
        let back = state_vectors_to_elements(p, v, mu);
        assert!((back.longitude_ascending - 1.2).abs() < 1e-9, "{back:?}");
        assert!((back.argument_perihelion - 2.1).abs() < 1e-9);
        assert!((back.true_anomaly - 0.7).abs() < 1e-9);
        let m = true_to_mean_anomaly(0.7, 0.3);
        let ecc = crate::kepler::solve_kepler(m, 0.3, 1e-14, 50);
        assert!((crate::kepler::eccentric_to_true_anomaly(ecc, 0.3) - 0.7).abs() < 1e-9);
        assert!((back.mean_anomaly - m).abs() < 1e-9);
    }

    #[test]
    fn hohmann_zero_delta_v_same_orbit() {
        let mu = 1.0;
        let r = 1.0;
        let (dv1, dv2) = hohmann_transfer(r, r, mu);
        assert!(approx_eq(dv1, 0.0));
        assert!(approx_eq(dv2, 0.0));
    }

    #[test]
    fn escape_velocity_earth() {
        let mass = 5.972e24;
        let radius = 6.371e6;
        let v_esc = escape_velocity(mass, radius);
        // Earth escape velocity ≈ 11186 m/s
        assert!((v_esc - 11186.0).abs() < 50.0);
    }

    #[test]
    fn orbital_energy_negative() {
        let mass = 1.0;
        let semi_major = 1.0;
        let radius = 1.0;
        let energy = orbital_energy(mass, semi_major, radius);
        assert!(energy < 0.0);
    }
}
