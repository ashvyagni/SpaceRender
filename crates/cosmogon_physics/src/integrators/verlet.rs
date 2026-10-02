use super::Integrator;

/// Velocity Verlet integrator (symplectic, second-order).
///
/// Velocity Verlet is energy-preserving over long time spans, making it
/// well-suited for orbital mechanics and molecular dynamics. Unlike RK4, it
/// is time-reversible and symplectic.
///
/// The algorithm per step:
///
/// 1. `v(t + Δt/2) = v(t) + a(t) * Δt/2`
/// 2. `x(t + Δt) = x(t) + v(t + Δt/2) * Δt`
/// 3. `a(t + Δt) = f(x(t + Δt))`
/// 4. `v(t + Δt) = v(t + Δt/2) + a(t + Δt) * Δt/2`
pub struct VelocityVerlet;

impl Integrator for VelocityVerlet {
    fn step(&self, state: &[f64], derivatives: &[f64], dt: f64) -> Vec<f64> {
        // For the trait interface we assume the state is flat [pos..., vel...]
        // and derivatives is [vel..., acc...]. We perform a simple Verlet
        // kick-drift-kick using the provided derivatives.
        let n = state.len();
        let half = n / 2;

        let mut result = vec![0.0; n];

        // Half-step velocity: v += a * dt/2
        for i in 0..half {
            result[half + i] = state[half + i] + derivatives[half + i] * dt * 0.5;
        }

        // Full-step position: x += v * dt
        for i in 0..half {
            result[i] = state[i] + result[half + i] * dt;
        }

        // The caller must re-evaluate accelerations and call `step` again
        // with the updated derivatives to complete the second half-kick.
        // For a single-call interface we approximate by using the same accelerations.
        for i in 0..half {
            result[half + i] += derivatives[half + i] * dt * 0.5;
        }

        result
    }
}

impl VelocityVerlet {
    /// Perform a full Velocity Verlet step with an acceleration function.
    ///
    /// This is the recommended interface: it handles the half-kick, drift,
    /// re-evaluation of accelerations, and second half-kick in one call.
    ///
    /// # Arguments
    /// * `positions` – current positions
    /// * `velocities` – current velocities
    /// * `accelerations` – current accelerations (from previous step or initial conditions)
    /// * `dt` – time step (seconds)
    /// * `f` – acceleration function: `positions → accelerations`
    ///
    /// # Returns
    /// `(new_positions, new_velocities, new_accelerations)`
    pub fn integrate<F>(
        positions: &[f64],
        velocities: &[f64],
        accelerations: &[f64],
        dt: f64,
        f: F,
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>)
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let n = positions.len();
        assert_eq!(n, velocities.len(), "positions and velocities must match");
        assert_eq!(
            n,
            accelerations.len(),
            "positions and accelerations must match"
        );

        // 1. Half-step velocity: v(t + dt/2) = v(t) + a(t) * dt/2
        let mut vel_half = vec![0.0; n];
        for i in 0..n {
            vel_half[i] = velocities[i] + accelerations[i] * dt * 0.5;
        }

        // 2. Full-step position: x(t + dt) = x(t) + v(t + dt/2) * dt
        let mut new_positions = vec![0.0; n];
        for i in 0..n {
            new_positions[i] = positions[i] + vel_half[i] * dt;
        }

        // 3. New accelerations: a(t + dt) = f(x(t + dt))
        let new_accelerations = f(&new_positions);

        // 4. Full-step velocity: v(t + dt) = v(t + dt/2) + a(t + dt) * dt/2
        let mut new_velocities = vec![0.0; n];
        for i in 0..n {
            new_velocities[i] = vel_half[i] + new_accelerations[i] * dt * 0.5;
        }

        (new_positions, new_velocities, new_accelerations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-4;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn verlet_constant_acceleration() {
        // x(t) = x0 + v0*t + 0.5*a*t²
        let positions = vec![0.0];
        let velocities = vec![0.0];
        let accelerations = vec![1.0];
        let dt = 0.1;
        let f = |_pos: &[f64]| vec![1.0];

        let (new_pos, new_vel, new_acc) =
            VelocityVerlet::integrate(&positions, &velocities, &accelerations, dt, f);

        // v(0.1) = 0 + 1*0.1 = 0.1
        assert!(approx_eq(new_vel[0], 0.1));
        // x(0.1) = 0 + 0.05*0.1 = 0.005 (half-step velocity was 0.05)
        assert!(approx_eq(new_pos[0], 0.005));
        assert!(approx_eq(new_acc[0], 1.0));
    }

    #[test]
    fn verlet_harmonic_oscillator() {
        // ẍ = -x, x(0) = 1, v(0) = 0
        let dt = 0.01;
        let mut positions = vec![1.0];
        let mut velocities = vec![0.0];
        let f = |pos: &[f64]| vec![-pos[0]];

        let accelerations = f(&positions);

        let _verlet = VelocityVerlet;
        let steps = 100; // t = 1.0
        let mut acc = accelerations;

        for _ in 0..steps {
            let (p, v, a) = VelocityVerlet::integrate(&positions, &velocities, &acc, dt, &f);
            positions = p;
            velocities = v;
            acc = a;
        }

        // cos(1.0) ≈ 0.5403
        assert!(approx_eq(positions[0], 0.5403));
    }

    #[test]
    fn verlet_trait_interface() {
        let state = vec![0.0, 0.0]; // x, v
        let derivatives = vec![0.0, 1.0]; // v, a
        let dt = 0.1;
        let verlet = VelocityVerlet;

        let new_state = verlet.step(&state, &derivatives, dt);
        // v_half = 0 + 1*0.05 = 0.05
        // x_new = 0 + 0.05*0.1 = 0.005
        // v_new = 0.05 + 1*0.05 = 0.1
        assert!(approx_eq(new_state[0], 0.005));
        assert!(approx_eq(new_state[1], 0.1));
    }
}
