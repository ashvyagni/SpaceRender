#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use cosmogon_core::math::Vec3d;
use cosmogon_core::constants;

/// Compute the five Lagrange points for a two-body system.
///
/// The primary body is at the origin and the secondary body is on the positive
/// x-axis at distance `separation`. The system orbits in the xy-plane.
///
/// - **L1**: between the two bodies
/// - **L2**: beyond the secondary (same side as secondary, further out)
/// - **L3**: opposite the primary from the secondary
/// - **L4**: 60° ahead of the secondary in its orbit (leading Trojan point)
/// - **L5**: 60° behind the secondary in its orbit (trailing Trojan point)
///
/// # Arguments
/// * `mass_primary` – mass of the larger body (kg)
/// * `mass_secondary` – mass of the smaller body (kg)
/// * `separation` – distance between the two bodies (meters)
///
/// # Returns
/// Array of five `Vec3d` positions: `[L1, L2, L3, L4, L5]`.
pub fn lagrange_points(
    mass_primary: f64,
    mass_secondary: f64,
    separation: f64,
) -> [Vec3d; 5] {
    let r_l1 = l1_distance(mass_primary, mass_secondary, separation);

    // L1 is between primary and secondary, at distance r_l1 from secondary
    let l1 = Vec3d::new(separation - r_l1, 0.0, 0.0);

    // L2 is beyond secondary at distance r_l2 (approximate same as L1 for small mass ratio)
    // Use iterative refinement for L2
    let r_l2 = l2_distance(mass_primary, mass_secondary, separation);
    let l2 = Vec3d::new(separation + r_l2, 0.0, 0.0);

    // L3 is on the opposite side of the primary
    // Approximate: L3 is at distance ≈ separation from primary, on the far side
    let mu = mass_secondary / (mass_primary + mass_secondary);
    let r_l3 = separation * (1.0 + (5.0 / 12.0) * mu);
    let l3 = Vec3d::new(-r_l3, 0.0, 0.0);

    // L4 and L5 are at 60° from the secondary in its orbit
    let angle_60 = constants::PI / 3.0; // 60 degrees
    let l4 = Vec3d::new(
        separation * angle_60.dcos(),
        separation * angle_60.dsin(),
        0.0,
    );
    let l5 = Vec3d::new(
        separation * angle_60.dcos(),
        -separation * angle_60.dsin(),
        0.0,
    );

    [l1, l2, l3, l4, l5]
}

/// Approximate L1 distance from the secondary body (Hill sphere approximation).
///
/// For a mass ratio μ = m₂/(m₁ + m₂) ≪ 1:
///
/// ```text
/// r_L1 ≈ R * (μ / 3)^(1/3)
/// ```
///
/// where R is the orbital separation.
///
/// # Arguments
/// * `mass_primary` – mass of the larger body (kg)
/// * `mass_secondary` – mass of the smaller body (kg)
/// * `separation` – orbital separation (meters)
///
/// # Returns
/// Distance of L1 from the secondary body (meters).
pub fn l1_distance(mass_primary: f64, mass_secondary: f64, separation: f64) -> f64 {
    let mu = mass_secondary / (mass_primary + mass_secondary);
    separation * (mu / 3.0).dpowf(1.0 / 3.0)
}

/// Approximate L2 distance from the secondary body.
///
/// Uses the same Hill sphere approximation as L1 (accurate to first order
/// in the mass ratio).
///
/// # Arguments
/// * `mass_primary` – mass of the larger body (kg)
/// * `mass_secondary` – mass of the smaller body (kg)
/// * `separation` – orbital separation (meters)
///
/// # Returns
/// Distance of L2 from the secondary body (meters).
pub fn l2_distance(mass_primary: f64, mass_secondary: f64, separation: f64) -> f64 {
    // Same order as L1 for the mass ratio expansion
    l1_distance(mass_primary, mass_secondary, separation)
}

/// Hill sphere radius of a body in its orbit around a primary.
///
/// ```text
/// r_Hill ≈ a * (m / (3 * M))^(1/3)
/// ```
///
/// # Arguments
/// * `mass_primary` – mass of the primary (kg)
/// * `mass_secondary` – mass of the secondary (kg)
/// * `separation` – orbital semi-major axis (meters)
///
/// # Returns
/// Hill sphere radius (meters).
pub fn hill_sphere_radius(mass_primary: f64, mass_secondary: f64, separation: f64) -> f64 {
    let mu = mass_secondary / mass_primary;
    separation * (mu / 3.0).dpowf(1.0 / 3.0)
}

/// Check whether a point is within the Hill sphere of a secondary body.
///
/// # Arguments
/// * `point` – position to test
/// * `secondary_pos` – position of the secondary body
/// * `hill_radius` – Hill sphere radius
///
/// # Returns
/// `true` if the point is inside or on the Hill sphere boundary.
pub fn is_in_hill_sphere(point: Vec3d, secondary_pos: Vec3d, hill_radius: f64) -> bool {
    (point - secondary_pos).length() <= hill_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_distance_sun_jupiter() {
        let mass_sun = 1.989e30;
        let mass_jupiter = 1.898e27;
        let separation = 7.786e11;

        let r_l1 = l1_distance(mass_sun, mass_jupiter, separation);
        // L1 should be on the order of 5e10 m (roughly 0.33 AU)
        assert!(r_l1 > 1e10 && r_l1 < 1e11);
    }

    #[test]
    fn lagrange_points_five() {
        let mass_primary = 1.989e30;
        let mass_secondary = 5.972e24;
        let separation = 1.496e11;

        let points = lagrange_points(mass_primary, mass_secondary, separation);
        assert_eq!(points.len(), 5);

        // L1 should be between primary and secondary
        assert!(points[0].x > 0.0 && points[0].x < separation);

        // L2 should be beyond secondary
        assert!(points[1].x > separation);

        // L3 should be on opposite side
        assert!(points[2].x < 0.0);

        // L4 and L5 should have non-zero y components of opposite sign
        assert!(points[3].y > 0.0);
        assert!(points[4].y < 0.0);
    }

    #[test]
    fn hill_sphere_radius_earth() {
        let mass_sun = 1.989e30;
        let mass_earth = 5.972e24;
        let separation = 1.496e11;

        let r_hill = hill_sphere_radius(mass_sun, mass_earth, separation);
        // Earth's Hill sphere ≈ 1.5e9 m (about 0.01 AU)
        assert!(r_hill > 1e9 && r_hill < 1e10);
    }

    #[test]
    fn is_in_hill_sphere_true() {
        let secondary = Vec3d::new(1.0, 0.0, 0.0);
        let hill_r = 0.1;
        let point = Vec3d::new(1.05, 0.0, 0.0);
        assert!(is_in_hill_sphere(point, secondary, hill_r));
    }

    #[test]
    fn is_in_hill_sphere_false() {
        let secondary = Vec3d::new(1.0, 0.0, 0.0);
        let hill_r = 0.1;
        let point = Vec3d::new(2.0, 0.0, 0.0);
        assert!(!is_in_hill_sphere(point, secondary, hill_r));
    }
}
