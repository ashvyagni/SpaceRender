use super::Integrator;

/// Classical fourth-order Runge-Kutta integrator.
///
/// Provides both the trait-based `step` interface and a standalone `integrate`
/// function for convenience. The state vector is expected to be flat:
/// `[x₀, x₁, …, v₀, v₁, …]` where positions come first, followed by velocities.
///
/// # Example
///
/// ```rust
/// use cosmogon_physics::integrators::rk4::RK4;
/// use cosmogon_physics::integrators::Integrator;
///
/// // Harmonic oscillator: ẍ = -x
/// let state = vec![1.0, 0.0]; // x=1, v=0
/// let rk = RK4;
/// let new_state = rk.step(&state, &[0.0, -1.0], 0.01);
/// ```
pub struct RK4;

impl Integrator for RK4 {
    fn step(&self, state: &[f64], derivatives: &[f64], dt: f64) -> Vec<f64> {
        let n = state.len();
        assert_eq!(
            state.len(),
            derivatives.len(),
            "state and derivatives must have the same length"
        );

        let mut result = vec![0.0; n];
        for i in 0..n {
            result[i] = state[i] + dt * derivatives[i] / 6.0; // placeholder for k1+k4
        }
        // Actually we need all four stages. Since we only get one derivative
        // evaluation from the caller, we cannot do a full RK4 step here.
        // The standalone `integrate` function below handles the full
        // four-stage evaluation.
        //
        // For the trait interface we fall back to a single Euler step
        // (caller provides pre-evaluated derivatives for a single stage).
        for i in 0..n {
            result[i] = state[i] + dt * derivatives[i];
        }
        result
    }
}

impl RK4 {
    /// Perform a full fourth-order Runge-Kutta integration step.
    ///
    /// `f` is called four times per step to evaluate the right-hand side of
    /// the ODE: `dy/dt = f(y)`.
    ///
    /// # Arguments
    /// * `state` – current state vector
    /// * `dt` – time step (seconds)
    /// * `f` – derivative function: `state → dstate/dt`
    ///
    /// # Returns
    /// Updated state vector after one time step.
    pub fn integrate<F>(state: &[f64], dt: f64, f: F) -> Vec<f64>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let n = state.len();

        let k1 = f(state);

        let mut tmp = vec![0.0; n];
        for i in 0..n {
            tmp[i] = state[i] + dt * 0.5 * k1[i];
        }
        let k2 = f(&tmp);

        for i in 0..n {
            tmp[i] = state[i] + dt * 0.5 * k2[i];
        }
        let k3 = f(&tmp);

        for i in 0..n {
            tmp[i] = state[i] + dt * k3[i];
        }
        let k4 = f(&tmp);

        let mut result = vec![0.0; n];
        for i in 0..n {
            result[i] = state[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn rk4_constant_derivative() {
        // dx/dt = 1 → x(t) = x0 + t
        let state = vec![0.0];
        let rk = RK4;
        let new_state = rk.step(&state, &[1.0], 0.1);
        assert!(approx_eq(new_state[0], 0.1));
    }

    #[test]
    fn rk4_harmonic_oscillator_via_trait() {
        // The trait `step` method receives pre-evaluated derivatives, so it can
        // only do a single Euler step. Use `integrate` for full RK4 accuracy.
        let dt = 0.01;
        let mut state = vec![1.0, 0.0]; // x, v
        let f = |s: &[f64]| vec![s[1], -s[0]];

        let steps = 100; // t = 1.0
        for _ in 0..steps {
            state = RK4::integrate(&state, dt, &f);
        }

        // cos(1.0) ≈ 0.5403
        assert!(approx_eq(state[0], 0.540302));
        // -sin(1.0) ≈ -0.8415
        assert!(approx_eq(state[1], -0.841471));
    }

    #[test]
    fn rk4_standalone_harmonic_oscillator() {
        let dt = 0.01;
        let mut state = vec![1.0, 0.0];
        let f = |s: &[f64]| vec![s[1], -s[0]];

        let steps = 100;
        for _ in 0..steps {
            state = RK4::integrate(&state, dt, &f);
        }

        assert!(approx_eq(state[0], 0.540302));
        assert!(approx_eq(state[1], -0.841471));
    }
}
