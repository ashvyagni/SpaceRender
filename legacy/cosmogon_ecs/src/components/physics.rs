use bevy_ecs::prelude::*;

/// Mass of a body in kilograms.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Mass(pub f64);

/// Radius of a body in meters.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Radius(pub f64);

/// Density in kg/m³.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Density(pub f64);

/// Surface gravity in m/s².
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SurfaceGravity(pub f64);

/// Escape velocity in m/s.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EscapeVelocity(pub f64);
