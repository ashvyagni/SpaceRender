use crate::celestial_body::CelestialBodyDef;
use crate::solar_system;

/// Load the default solar system scene.
///
/// Returns a [`Vec`] of [`CelestialBodyDef`] containing the Sun, eight
/// planets, Earth's Moon, and selected major moons of the outer planets.
pub fn load_default_scene() -> Vec<CelestialBodyDef> {
    solar_system::solar_system_bodies()
}

// Future extensions:
// - Load from .ron scene files
// - Import SPICE kernel data
// - Procedurally generate fictional star systems
