//! The physical universe: stars, orbits, bodies, systems and their generation.

pub mod body;
pub mod dynamics;
pub mod horizons;
pub mod object;
pub mod generate;
pub mod orbit;
pub mod sol;
pub mod star;
pub mod system;

pub use body::*;
pub use object::{ObjectClass, Provenance, Quality};
pub use orbit::Orbit;
pub use star::Star;
pub use system::*;

// One set of physical constants for the whole project (see docs/DATA_SOURCES.md).
pub use cosmogon_core::constants::{AU, EARTH_MASS, EARTH_RADIUS, G, GM_SUN, JUPITER_MASS, JUPITER_RADIUS, LIGHT_YEAR, SOLAR_MASS, SOLAR_RADIUS};
pub const SPEED_OF_LIGHT: f64 = cosmogon_core::constants::C;
