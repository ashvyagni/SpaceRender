//! The physical universe: stars, orbits, bodies, systems and their generation.

pub mod body;
pub mod generate;
pub mod orbit;
pub mod sol;
pub mod star;
pub mod system;

pub use body::*;
pub use orbit::Orbit;
pub use star::Star;
pub use system::*;

/// Astronomical unit in metres (IAU 2012).
pub const AU: f64 = 1.495_978_707e11;
pub const LIGHT_YEAR: f64 = 9.460_730_472_580_8e15;
pub const SOLAR_MASS: f64 = 1.988_92e30;
pub const SOLAR_RADIUS: f64 = 6.957e8;
pub const EARTH_MASS: f64 = 5.972_17e24;
pub const EARTH_RADIUS: f64 = 6.371e6;
pub const G: f64 = cosmogon_core::constants::G;
pub const SPEED_OF_LIGHT: f64 = cosmogon_core::constants::C;
