use bevy_ecs::prelude::*;
use cosmogon_core::math::Vec3d;

use crate::gravity;
use crate::kepler;
use cosmogon_ecs::components::motion::*;
use cosmogon_ecs::components::orbital::*;
use cosmogon_ecs::components::physics::*;
use cosmogon_ecs::components::transform::*;
use cosmogon_ecs::resources::global_time::*;

/// Configuration for gravitational simulation.
///
/// Insert as a resource to control N-body behaviour.
#[derive(Resource, Debug, Clone)]
pub struct GravityConfig {
    /// Softening length ε (meters) to prevent singularities.
    pub softening: f64,
    /// Enable full N-body gravitational interactions.
    pub use_nbody: bool,
    /// Use parallel computation (rayon) when N-body is enabled.
    pub parallel: bool,
    /// Mass of the central body (kg) used for Keplerian propagation.
    pub central_mass: f64,
}

impl Default for GravityConfig {
    fn default() -> Self {
        Self {
            softening: 1000.0,
            use_nbody: true,
            parallel: true,
            central_mass: cosmogon_core::constants::SOLAR_MASS,
        }
    }
}

/// System: propagate all bodies with Keplerian orbital elements to their
/// current positions and velocities.
///
/// For each entity with `OrbitalElements`, `Mass`, `Position`, and `Velocity`,
/// this system:
/// 1. Computes the mean anomaly at the current simulation time
/// 2. Solves Kepler's equation for eccentric anomaly
/// 3. Converts to position and velocity via classical orbital mechanics
/// 4. Writes the results into `Position` and `Velocity`
pub fn propagate_orbits(
    time: Res<GlobalTime>,
    config: Res<GravityConfig>,
    mut query: Query<(
        &OrbitalElements,
        &Mass,
        &mut Position,
        &mut Velocity,
    )>,
) {
    if time.paused {
        return;
    }

    let mu_central = cosmogon_core::constants::G * config.central_mass;

    for (elements, _mass, mut position, mut velocity) in query.iter_mut() {
        let period = elements.period(mu_central);
        let mean_anomaly = elements.mean_anomaly_at_time(time.elapsed, period);
        let ecc = elements.eccentricity;

        let ecc_anom = kepler::solve_kepler(mean_anomaly, ecc, 1e-12, 50);
        let true_anom = kepler::eccentric_to_true_anomaly(ecc_anom, ecc);
        let r = kepler::orbital_radius(elements.semi_major_axis, ecc, true_anom);

        // Position in orbital plane
        let r_peri = Vec3d::new(r * true_anom.cos(), r * true_anom.sin(), 0.0);

        // Velocity in orbital plane
        let h = (mu_central * elements.semi_major_axis * (1.0 - ecc * ecc)).sqrt();
        let v_peri = if h > 0.0 {
            Vec3d::new(
                -mu_central / h * true_anom.sin(),
                mu_central / h * (ecc + true_anom.cos()),
                0.0,
            )
        } else {
            Vec3d::ZERO
        };

        // Rotation to inertial frame
        let cos_w = elements.argument_perihelion.cos();
        let sin_w = elements.argument_perihelion.sin();
        let cos_i = elements.inclination.cos();
        let sin_i = elements.inclination.sin();
        let cos_o = elements.longitude_ascending.cos();
        let sin_o = elements.longitude_ascending.sin();

        let pos = Vec3d::new(
            (cos_o * cos_w - sin_o * sin_w * cos_i) * r_peri.x
                + (-cos_o * sin_w - sin_o * cos_w * cos_i) * r_peri.y,
            (sin_o * cos_w + cos_o * sin_w * cos_i) * r_peri.x
                + (-sin_o * sin_w + cos_o * cos_w * cos_i) * r_peri.y,
            (sin_w * sin_i) * r_peri.x + (cos_w * sin_i) * r_peri.y,
        );

        let vel = Vec3d::new(
            (cos_o * cos_w - sin_o * sin_w * cos_i) * v_peri.x
                + (-cos_o * sin_w - sin_o * cos_w * cos_i) * v_peri.y,
            (sin_o * cos_w + cos_o * sin_w * cos_i) * v_peri.x
                + (-sin_o * sin_w + cos_o * cos_w * cos_i) * v_peri.y,
            (sin_w * sin_i) * v_peri.x + (cos_w * sin_i) * v_peri.y,
        );

        position.coords = pos;
        velocity.linear = vel;
    }
}

/// System: compute gravitational accelerations for all bodies via N-body
/// interactions.
///
/// Collects all `(Entity, Position, Mass)` into a temporary buffer, computes
/// the full acceleration matrix, and writes results into `Acceleration`.
///
/// This system requires the [`GravityConfig`] resource. If `use_nbody` is
/// `false`, this system is a no-op.
pub fn compute_gravity(
    mut query: Query<(Entity, &Position, &Mass, &mut Acceleration)>,
    config: Res<GravityConfig>,
) {
    if !config.use_nbody {
        return;
    }

    // Collect positions and masses into flat arrays
    let bodies: Vec<(Entity, Vec3d, f64)> = query
        .iter()
        .map(|(entity, pos, mass, _)| (entity, pos.coords, mass.0))
        .collect();

    let n = bodies.len();
    if n == 0 {
        return;
    }

    let positions: Vec<Vec3d> = bodies.iter().map(|(_, p, _)| *p).collect();
    let masses: Vec<f64> = bodies.iter().map(|(_, _, m)| *m).collect();

    let accelerations = if config.parallel && n > 64 {
        gravity::compute_all_accelerations_parallel(&positions, &masses, config.softening)
    } else {
        gravity::compute_all_accelerations(&positions, &masses, config.softening)
    };

    // Write accelerations back — the immutable iterator is consumed, so
    // we can now take a mutable borrow for the update pass.
    for (i, (entity, _, _)) in bodies.iter().enumerate() {
        if let Ok((_, _, _, mut acc)) = query.get_mut(*entity) {
            acc.linear = accelerations[i];
        }
    }
}

/// System: integrate positions from velocities and accelerations using
/// semi-implicit Euler (kick-drift).
///
/// Updates `Position` from `Velocity` and `Acceleration` for one time step.
/// This is a simple first-order integrator suitable for visual smoothing;
/// for accurate orbital mechanics, use the Kepler propagator.
pub fn integrate_motion(
    time: Res<GlobalTime>,
    mut query: Query<(&mut Position, &Velocity, &Acceleration)>,
) {
    if time.paused {
        return;
    }

    let dt = time.delta * time.acceleration;

    for (mut position, velocity, acceleration) in query.iter_mut() {
        // Semi-implicit Euler: x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
        position.coords = position.coords
            + velocity.linear * dt
            + acceleration.linear * (0.5 * dt * dt);
    }
}

/// System: integrate velocities from accelerations (velocity kick).
///
/// Updates `Velocity` from `Acceleration` for one time step. Call this
/// before [`integrate_motion`] for a leapfrog-style integration, or after
/// for a kick-drift-kick scheme.
pub fn integrate_velocity(
    time: Res<GlobalTime>,
    mut query: Query<(&mut Velocity, &Acceleration)>,
) {
    if time.paused {
        return;
    }

    let dt = time.delta * time.acceleration;

    for (mut velocity, acceleration) in query.iter_mut() {
        velocity.linear = velocity.linear + acceleration.linear * dt;
    }
}

/// System: placeholder for future physics-specific time management.
pub fn update_physics_time(_time: Res<GlobalTime>) {
    // Time is currently managed in app.rs RedrawRequested handler.
    // This system will eventually handle sub-stepping, adaptive dt, etc.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_config_default() {
        let config = GravityConfig::default();
        assert!(config.use_nbody);
        assert!(config.parallel);
        assert!(config.softening > 0.0);
    }
}
