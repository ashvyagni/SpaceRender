use cosmogon_core::math::Vec3d;
use cosmogon_core::constants;

/// Solve Kepler's equation `M = E - e * sin(E)` for the eccentric anomaly `E`.
///
/// Uses Newton-Raphson iteration with an initial guess derived from the mean
/// anomaly and eccentricity. Converges in 3–5 iterations for typical orbits.
///
/// # Arguments
/// * `mean_anomaly` – mean anomaly M (radians)
/// * `eccentricity` – orbital eccentricity e (0 ≤ e < 1 for elliptical orbits)
/// * `tolerance` – convergence threshold on ΔE (radians)
/// * `max_iter` – maximum number of iterations
///
/// # Returns
/// Eccentric anomaly E (radians) in `[0, 2π)`.
pub fn solve_kepler(mean_anomaly: f64, eccentricity: f64, tolerance: f64, max_iter: u32) -> f64 {
    let mut e = mean_anomaly;

    for _ in 0..max_iter {
        let delta = e - eccentricity * e.sin() - mean_anomaly;
        let derivative = 1.0 - eccentricity * e.cos();
        if derivative.abs() < f64::EPSILON {
            break;
        }
        let step = delta / derivative;
        e -= step;
        if step.abs() < tolerance {
            break;
        }
    }

    // Normalize to [0, 2π)
    e = e % constants::TWO_PI;
    if e < 0.0 {
        e += constants::TWO_PI;
    }
    e
}

/// Convert eccentric anomaly to true anomaly.
///
/// ```text
/// ν = 2 * atan( sqrt((1+e)/(1-e)) * tan(E/2) )
/// ```
///
/// # Arguments
/// * `eccentric_anomaly` – eccentric anomaly E (radians)
/// * `eccentricity` – orbital eccentricity e
///
/// # Returns
/// True anomaly ν (radians) in `[0, 2π)`.
pub fn eccentric_to_true_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let e = eccentricity;
    let half_e = eccentric_anomaly * 0.5;
    let factor = ((1.0 + e) / (1.0 - e)).sqrt();
    let mut nu = 2.0 * (factor * half_e.tan()).atan();

    // Ensure positive
    if nu < 0.0 {
        nu += constants::TWO_PI;
    }
    nu
}

/// Compute the orbital radius from semi-major axis, eccentricity, and true anomaly.
///
/// ```text
/// r = a(1 - e²) / (1 + e * cos(ν))
/// ```
///
/// # Arguments
/// * `semi_major_axis` – semi-major axis a (meters)
/// * `eccentricity` – orbital eccentricity e
/// * `true_anomaly` – true anomaly ν (radians)
///
/// # Returns
/// Orbital radius r (meters).
pub fn orbital_radius(semi_major_axis: f64, eccentricity: f64, true_anomaly: f64) -> f64 {
    let e = eccentricity;
    let p = semi_major_axis * (1.0 - e * e); // semi-latus rectum
    p / (1.0 + e * true_anomaly.cos())
}

/// Convert classical orbital elements to Cartesian state vectors (position and velocity).
///
/// This function solves the two-body problem analytically: given six Keplerian
/// elements it returns the 3D position and velocity in the inertial frame
/// centred on the primary body.
///
/// # Arguments
/// * `semi_major_axis` – semi-major axis a (meters)
/// * `eccentricity` – eccentricity e (0 ≤ e < 1)
/// * `inclination` – inclination i (radians)
/// * `longitude_ascending` – longitude of ascending node Ω (radians)
/// * `argument_perihelion` – argument of perihelion ω (radians)
/// * `mean_anomaly` – mean anomaly at the current epoch M (radians)
/// * `gravitational_param` – standard gravitational parameter μ = GM (m³/s²)
///
/// # Returns
/// `(position, velocity)` as `Vec3d` tuples in the inertial frame.
pub fn orbital_state_vectors(
    semi_major_axis: f64,
    eccentricity: f64,
    inclination: f64,
    longitude_ascending: f64,
    argument_perihelion: f64,
    mean_anomaly: f64,
    gravitational_param: f64,
) -> (Vec3d, Vec3d) {
    // 1. Solve Kepler's equation
    let e_anom = solve_kepler(mean_anomaly, eccentricity, 1e-12, 50);

    // 2. True anomaly
    let nu = eccentric_to_true_anomaly(e_anom, eccentricity);

    // 3. Orbital radius
    let r = orbital_radius(semi_major_axis, eccentricity, nu);

    // 4. Position in orbital plane (perifocal frame)
    let r_peri = Vec3d::new(r * nu.cos(), r * nu.sin(), 0.0);

    // 5. Velocity in orbital plane
    let h = (gravitational_param * semi_major_axis * (1.0 - eccentricity * eccentricity)).sqrt();
    let v_peri = Vec3d::new(
        -gravitational_param / h * nu.sin(),
        gravitational_param / h * (eccentricity + nu.cos()),
        0.0,
    );

    // 6. Rotation by argument of perihelion (ω), inclination (i), and Ω
    let cos_omega = argument_perihelion.cos();
    let sin_omega = argument_perihelion.sin();
    let cos_i = inclination.cos();
    let sin_i = inclination.sin();
    let cos_node = longitude_ascending.cos();
    let sin_node = longitude_ascending.sin();

    // Combined rotation matrix elements (perifocal → inertial)
    let p_x = cos_node * cos_omega - sin_node * sin_omega * cos_i;
    let p_y = sin_node * cos_omega + cos_node * sin_omega * cos_i;
    let p_z = sin_omega * sin_i;

    let q_x = -cos_node * sin_omega - sin_node * cos_omega * cos_i;
    let q_y = -sin_node * sin_omega + cos_node * cos_omega * cos_i;
    let q_z = cos_omega * sin_i;

    let position = Vec3d::new(
        p_x * r_peri.x + q_x * r_peri.y,
        p_y * r_peri.x + q_y * r_peri.y,
        p_z * r_peri.x + q_z * r_peri.y,
    );

    let velocity = Vec3d::new(
        p_x * v_peri.x + q_x * v_peri.y,
        p_y * v_peri.x + q_y * v_peri.y,
        p_z * v_peri.x + q_z * v_peri.y,
    );

    (position, velocity)
}

/// Compute the orbital period from semi-major axis and gravitational parameter.
///
/// ```text
/// T = 2π * sqrt(a³ / μ)
/// ```
///
/// # Arguments
/// * `semi_major_axis` – semi-major axis a (meters)
/// * `gravitational_param` – standard gravitational parameter μ (m³/s²)
///
/// # Returns
/// Orbital period T (seconds).
pub fn orbital_period(semi_major_axis: f64, gravitational_param: f64) -> f64 {
    constants::TWO_PI * (semi_major_axis.powi(3) / gravitational_param).sqrt()
}

/// Vis-viva equation: compute orbital speed at a given radius.
///
/// ```text
/// v = sqrt( μ * (2/r - 1/a) )
/// ```
///
/// # Arguments
/// * `gravitational_param` – standard gravitational parameter μ (m³/s²)
/// * `semi_major_axis` – semi-major axis a (meters)
/// * `radius` – current orbital radius r (meters)
///
/// # Returns
/// Orbital speed v (m/s).
pub fn vis_viva(gravitational_param: f64, semi_major_axis: f64, radius: f64) -> f64 {
    (gravitational_param * (2.0 / radius - 1.0 / semi_major_axis)).sqrt()
}

/// Specific orbital energy (energy per unit mass).
///
/// ```text
/// ε = -μ / (2a)
/// ```
///
/// # Arguments
/// * `gravitational_param` – standard gravitational parameter μ (m³/s²)
/// * `semi_major_axis` – semi-major axis a (meters)
///
/// # Returns
/// Specific orbital energy ε (J/kg).
pub fn specific_energy(gravitational_param: f64, semi_major_axis: f64) -> f64 {
    -gravitational_param / (2.0 * semi_major_axis)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn kepler_zero_eccentricity() {
        // Circular orbit: M = E when e = 0
        let m = 1.5;
        let e = solve_kepler(m, 0.0, 1e-12, 50);
        assert!(approx_eq(e, m));
    }

    #[test]
    fn kepler_small_eccentricity() {
        let m = 0.7;
        let ecc = 0.1;
        let e = solve_kepler(m, ecc, 1e-12, 50);
        let residual = e - ecc * e.sin() - m;
        assert!(residual.abs() < 1e-10);
    }

    #[test]
    fn eccentric_to_true_circular() {
        // e = 0 → ν = E
        let e_anom = 1.2;
        let nu = eccentric_to_true_anomaly(e_anom, 0.0);
        assert!(approx_eq(nu, e_anom));
    }

    #[test]
    fn orbital_radius_at_perihelion() {
        let a = 1.0;
        let e = 0.5;
        // At ν = 0: r = a(1-e)
        let r = orbital_radius(a, e, 0.0);
        assert!(approx_eq(r, a * (1.0 - e)));
    }

    #[test]
    fn orbital_radius_at_apohelion() {
        let a = 1.0;
        let e = 0.5;
        // At ν = π: r = a(1+e)
        let r = orbital_radius(a, e, std::f64::consts::PI);
        assert!(approx_eq(r, a * (1.0 + e)));
    }

    #[test]
    fn orbital_period_earth() {
        // Earth: a ≈ 1.496e11 m, μ ≈ 1.327e20 m³/s²
        let a = 1.496e11;
        let mu = 1.32712440018e20;
        let period = orbital_period(a, mu);
        // Should be ~365.25 days ≈ 3.156e7 s
        assert!((period - 3.156e7).abs() / 3.156e7 < 0.01);
    }

    #[test]
    fn vis_viva_circular() {
        let mu = 1.0;
        let a = 1.0;
        let r = 1.0;
        let v = vis_viva(mu, a, r);
        assert!(approx_eq(v, 1.0));
    }

    #[test]
    fn specific_energy_negative_for_elliptical() {
        let mu = 1.0;
        let a = 1.0;
        let energy = specific_energy(mu, a);
        assert!(energy < 0.0);
        assert!(approx_eq(energy, -0.5));
    }
}
