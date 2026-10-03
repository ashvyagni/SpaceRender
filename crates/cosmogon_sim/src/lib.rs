//! # cosmogon_sim
//!
//! The authoritative simulation for Cosmogon. It has **no engine or rendering
//! dependencies**: the desktop app, the CLI and the tests all drive the same
//! [`Universe`] through the same API.
//!
//! Layering (lower layers never depend on higher ones):
//!
//! ```text
//!   rng, time, noise, params            deterministic foundations
//!   astro (stars, orbits, generation)   physical universe
//!   planet (environment, terrain, resources), habitability
//!   life (biosphere abstraction)
//!   civ (species, knowledge, technology graph, settlements)
//!   history, scheduler, universe, save
//! ```
//!
//! See `SIMULATION.md`, `CIVILIZATION_MODEL.md` and `TECHNOLOGY_MODEL.md` at the
//! repository root for the models and their documented simplifications.

pub mod astro;
pub mod civ;
pub mod habitability;
pub mod history;
pub mod impact;
pub mod life;
pub mod names;
pub mod noise;
pub mod params;
pub mod planet;
pub mod rng;
pub mod sandbox;
pub mod save;
pub mod scheduler;
pub mod time;
pub mod universe;

pub use cosmogon_core::math::Vec3d;
pub use universe::{BodyRef, Scenario, Universe, UniverseSettings};
