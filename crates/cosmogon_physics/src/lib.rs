//! Engine-independent orbital mechanics.
//!
//! Everything here is pure `f64` math with no ECS or rendering dependencies, so it can be
//! used by the headless simulation, the CLI and tests alike.
pub mod gravity;
pub mod integrators;
pub mod kepler;
pub mod lagrange;
pub mod nbody;
pub mod orbital_elements;
