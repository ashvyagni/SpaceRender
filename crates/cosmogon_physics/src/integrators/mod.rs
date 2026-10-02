pub mod rk4;
pub mod verlet;

/// Trait for numerical time integrators.
///
/// An integrator advances a dynamical state by one time step `dt`, given
/// precomputed derivatives. Implementors must handle the composition of
/// the new state from the current state and derivatives.
pub trait Integrator {
    /// Advance the state by one time step.
    ///
    /// # Arguments
    /// * `state` – current flattened state vector (e.g. positions ∥ velocities)
    /// * `derivatives` – derivatives of the state (e.g. velocities ∥ accelerations)
    /// * `dt` – time step (seconds)
    ///
    /// # Returns
    /// The updated state vector.
    fn step(&self, state: &[f64], derivatives: &[f64], dt: f64) -> Vec<f64>;
}
