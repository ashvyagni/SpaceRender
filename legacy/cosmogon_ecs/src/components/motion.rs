use bevy_ecs::prelude::*;
use cosmogon_core::math::Vec3d;

/// Linear and angular velocity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Velocity {
    /// Linear velocity in m/s.
    pub linear: Vec3d,
    /// Angular velocity in rad/s (axis-angle representation).
    pub angular: Vec3d,
}

/// Linear acceleration applied to an entity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Acceleration {
    /// Linear acceleration in m/s².
    pub linear: Vec3d,
}

/// Velocity derived from orbital elements, cached each frame.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OrbitalVelocity {
    /// Cached linear velocity in m/s computed from Keplerian elements.
    pub linear: Vec3d,
    /// Cached angular velocity in rad/s.
    pub angular: Vec3d,
}
