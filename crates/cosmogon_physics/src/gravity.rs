use cosmogon_core::math::Vec3d;

/// Compute the gravitational acceleration on a body at `pos_a` due to a body at `pos_b`.
///
/// Uses softened gravity to avoid singularities at zero separation:
///
/// ```text
/// a = G * m_b * (r_b - r_a) / (|r|^2 + ε^2)^(3/2)
/// ```
///
/// The gravitational constant `G` is absorbed into the caller's convention; this
/// function computes the kinematic part only (acceleration per unit mass of the
/// target). Multiply by `G` externally if working in SI units.
///
/// # Arguments
/// * `pos_a` – position of the body being accelerated
/// * `mass_b` – mass of the source body (kg)
/// * `pos_b` – position of the source body
/// * `softening` – softening length ε (meters) to prevent divergence
///
/// # Returns
/// Acceleration vector (m/s²) exerted on body A by body B.
pub fn gravitational_acceleration(
    pos_a: Vec3d,
    mass_b: f64,
    pos_b: Vec3d,
    softening: f64,
) -> Vec3d {
    let r = pos_b - pos_a;
    let r_sq = r.length_squared();
    let denom = (r_sq + softening * softening).powf(1.5);
    if denom == 0.0 {
        Vec3d::ZERO
    } else {
        r * (mass_b / denom)
    }
}

/// Compute accelerations on every body due to all other bodies (N-body).
///
/// Each body's acceleration is the sum of softened gravitational contributions
/// from every other body. The function is symmetric: body `i` does not
/// accelerate itself.
///
/// # Arguments
/// * `positions` – world-space positions of all bodies
/// * `masses` – masses (kg) corresponding to each position
/// * `softening` – softening length ε (meters)
///
/// # Returns
/// A `Vec<Vec3d>` of acceleration vectors, one per body.
///
/// # Panics
/// Panics if `positions.len() != masses.len()`.
pub fn compute_all_accelerations(
    positions: &[Vec3d],
    masses: &[f64],
    softening: f64,
) -> Vec<Vec3d> {
    assert_eq!(
        positions.len(),
        masses.len(),
        "positions and masses must have the same length"
    );

    let n = positions.len();
    let mut accelerations = vec![Vec3d::ZERO; n];

    for i in 0..n {
        let mut acc = Vec3d::ZERO;
        for j in 0..n {
            if i == j {
                continue;
            }
            acc += gravitational_acceleration(positions[i], masses[j], positions[j], softening);
        }
        accelerations[i] = acc;
    }

    accelerations
}

/// Parallel N-body acceleration computation using rayon.
///
/// Identical to [`compute_all_accelerations`] but distributes work across
/// available CPU cores. Recommended for bodies > ~256.
///
/// # Arguments
/// * `positions` – world-space positions of all bodies
/// * `masses` – masses (kg) corresponding to each position
/// * `softening` – softening length ε (meters)
///
/// # Returns
/// A `Vec<Vec3d>` of acceleration vectors, one per body.
pub fn compute_all_accelerations_parallel(
    positions: &[Vec3d],
    masses: &[f64],
    softening: f64,
) -> Vec<Vec3d> {
    use rayon::prelude::*;

    assert_eq!(
        positions.len(),
        masses.len(),
        "positions and masses must have the same length"
    );

    (0..positions.len())
        .into_par_iter()
        .map(|i| {
            let mut acc = Vec3d::ZERO;
            for j in 0..positions.len() {
                if i == j {
                    continue;
                }
                acc += gravitational_acceleration(
                    positions[i],
                    masses[j],
                    positions[j],
                    softening,
                );
            }
            acc
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn zero_softening_gives_newtonian() {
        let pos_a = Vec3d::ZERO;
        let pos_b = Vec3d::new(1.0, 0.0, 0.0);
        let mass_b = 1.0;
        let softening = 0.0;

        let acc = gravitational_acceleration(pos_a, mass_b, pos_b, softening);
        assert!(approx_eq(acc.x, 1.0));
        assert!(approx_eq(acc.y, 0.0));
        assert!(approx_eq(acc.z, 0.0));
    }

    #[test]
    fn softened_gravity_is_finite_at_overlap() {
        let pos_a = Vec3d::ZERO;
        let pos_b = Vec3d::ZERO; // same position
        let mass_b = 1.0;
        let softening = 1.0;

        let acc = gravitational_acceleration(pos_a, mass_b, pos_b, softening);
        // r = 0, denominator = eps^3 = 1.0, so acc = 0
        assert!(approx_eq(acc.length(), 0.0));
    }

    #[test]
    fn nbody_two_bodies_symmetric() {
        let positions = vec![Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 0.0, 0.0)];
        let masses = vec![1.0, 1.0];
        let softening = 0.0;

        let accels = compute_all_accelerations(&positions, &masses, softening);
        assert_eq!(accels.len(), 2);
        // Accelerations should be equal and opposite
        assert!(approx_eq(accels[0].x, -accels[1].x));
        assert!(approx_eq(accels[0].y, 0.0));
        assert!(approx_eq(accels[0].z, 0.0));
    }

    #[test]
    fn parallel_matches_sequential() {
        let positions = vec![
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(3.0, 0.0, 0.0),
            Vec3d::new(0.0, 4.0, 0.0),
        ];
        let masses = vec![1.0, 2.0, 3.0];
        let softening = 0.5;

        let seq = compute_all_accelerations(&positions, &masses, softening);
        let par = compute_all_accelerations_parallel(&positions, &masses, softening);

        for (a, b) in seq.iter().zip(par.iter()) {
            assert!(approx_eq(a.x, b.x));
            assert!(approx_eq(a.y, b.y));
            assert!(approx_eq(a.z, b.z));
        }
    }
}
