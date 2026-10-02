use bevy_ecs::prelude::*;

/// Generic marker for any celestial body.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CelestialBody;

/// Star (luminous body).
#[derive(Component, Debug, Clone, Copy)]
pub struct Star {
    /// Spectral classification character (O, B, A, F, G, K, M).
    pub spectral_class: char,
    /// Surface temperature in Kelvin.
    pub temperature: f64,
}

impl Default for Star {
    fn default() -> Self {
        Self {
            spectral_class: 'G',
            temperature: 5778.0,
        }
    }
}

/// Planet (orbits a star).
#[derive(Component, Debug, Clone, Copy)]
pub struct Planet {
    /// Whether the planet has a ring system.
    pub has_rings: bool,
}

impl Default for Planet {
    fn default() -> Self {
        Self { has_rings: false }
    }
}

/// Moon (orbits a planet).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Moon;

/// Asteroid or minor body.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Asteroid;

/// Man-made spacecraft.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Spacecraft;

/// Human-readable display name.
#[derive(Component, Debug, Clone)]
pub struct BodyName(pub String);

impl BodyName {
    /// Create a new body name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}
