#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use cosmogon_core::math::Vec3d;
use cosmogon_core::constants;

/// Classical Keplerian orbital elements derived from Cartesian state vectors.
#[derive(Debug, Clone, Copy)]
pub struct OrbitalElements {
    /// Semi-major axis a (meters).
    pub semi_major_axis: f64,
    /// Eccentricity e (dimensionless).
    pub eccentricity: f64,
    /// Inclination i (radians).
    pub inclination: f64,
    /// Longitude of ascending node Ω (radians).
    pub longitude_ascending: f64,
    /// Argument of perihelion ω (radians).
    pub argument_perihelion: f64,
    /// True anomaly ν (radians).
    pub true_anomaly: f64,
}

/// Convert position and velocity state vectors to classical orbital elements.
///
/// Given an inertial-frame position `r` and velocity `v` relative to a central
/// body with gravitational parameter `mu`, this function computes the six
/// classical orbital elements.
///
/// # Arguments
/// * `r` – position vector (meters)
/// * `v` – velocity vector (m/s)
/// * `mu` – gravitational parameter μ = GM (m³/s²)
///
/// # Returns
/// [`OrbitalElements`] struct with all six elements.
///
/// # Panics
/// Panics if `mu` is zero or negative.
pub fn state_vectors_to_elements(r: Vec3d, v: Vec3d, mu: f64) -> OrbitalElements {
    assert!(mu > 0.0, "gravitational parameter must be positive");

    let r_mag = r.length();
    let v_mag = v.length();

    // Specific angular momentum: h = r × v
    let h = r.cross(v);
    let h_mag = h.length();

    // Node vector: n = k × h (k = [0, 0, 1])
    let n = Vec3d::new(-h.y, h.x, 0.0);
    let n_mag = n.length();

    // Eccentricity vector: e = (v × h) / μ - r̂
    let e_vec = {
        let v_cross_h = v.cross(h);
        Vec3d::new(
            v_cross_h.x / mu - r.x / r_mag,
            v_cross_h.y / mu - r.y / r_mag,
            v_cross_h.z / mu - r.z / r_mag,
        )
    };
    let eccentricity = e_vec.length();

    // Specific energy → semi-major axis
    let energy = v_mag * v_mag * 0.5 - mu / r_mag;
    let semi_major_axis = if eccentricity < 1.0 - 1e-12 {
        // Elliptical
        -mu / (2.0 * energy)
    } else if (eccentricity - 1.0).abs() < 1e-12 {
        // Parabolic — semi-major axis is infinite; use periapsis distance
        f64::INFINITY
    } else {
        // Hyperbolic
        -mu / (2.0 * energy) // negative semi-major axis by convention
    };

    // Inclination
    let inclination = (h.z / h_mag).clamp(-1.0, 1.0).dacos();

    // Longitude of ascending node
    let longitude_ascending = if n_mag > 1e-15 {
        let mut raan = n.x.datan2(n.y);
        if raan < 0.0 {
            raan += constants::TWO_PI;
        }
        raan
    } else {
        0.0
    };

    // Argument of perihelion
    let argument_perihelion = if n_mag > 1e-15 && eccentricity > 1e-15 {
        let aop = n.dot(e_vec) / (n_mag * eccentricity);
        let aop = aop.clamp(-1.0, 1.0).dacos();
        if e_vec.z < 0.0 {
            constants::TWO_PI - aop
        } else {
            aop
        }
    } else {
        0.0
    };

    // True anomaly
    let true_anomaly = if eccentricity > 1e-15 {
        let ta = e_vec.dot(r) / (eccentricity * r_mag);
        let ta = ta.clamp(-1.0, 1.0).dacos();
        if r.dot(v) < 0.0 {
            constants::TWO_PI - ta
        } else {
            ta
        }
    } else {
        // Circular orbit — use position angle relative to ascending node
        if n_mag > 1e-15 {
            let cos_ta = n.dot(r) / (n_mag * r_mag);
            let sin_ta = h.dot(n.cross(r)) / (h_mag * n_mag * r_mag);
            let mut ta = cos_ta.clamp(-1.0, 1.0).dacos();
            if sin_ta < 0.0 {
                ta = constants::TWO_PI - ta;
            }
            ta
        } else {
            0.0
        }
    };

    OrbitalElements {
        semi_major_axis,
        eccentricity,
        inclination,
        longitude_ascending,
        argument_perihelion,
        true_anomaly,
    }
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
