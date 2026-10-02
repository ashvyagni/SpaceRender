use bevy_ecs::prelude::*;
use cosmogon_core::math::Vec3d;
use glam::Quat;

/// Double-precision position in the world (meters from origin).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Position {
    /// World-space coordinates in meters.
    pub coords: Vec3d,
}

/// Rotation as a quaternion.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Rotation {
    /// Orientation quaternion.
    pub quat: Quat,
}

/// Scale factor (1.0 = normal size).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Scale(pub f32);

/// Combined transform bundle for convenience.
#[derive(Bundle, Default)]
pub struct TransformBundle {
    /// Position component.
    pub position: Position,
    /// Rotation component.
    pub rotation: Rotation,
    /// Scale component.
    pub scale: Scale,
}
