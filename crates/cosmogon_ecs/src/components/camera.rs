use bevy_ecs::prelude::*;

/// Perspective camera configuration.
#[derive(Component, Debug, Clone, Copy)]
pub struct Camera {
    /// Vertical field of view in radians.
    pub fov: f32,
    /// Near clipping plane distance in meters.
    pub near: f32,
    /// Far clipping plane distance in meters.
    pub far: f32,
    /// Viewport aspect ratio (width / height).
    pub aspect: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: std::f32::consts::FRAC_PI_4,
            near: 0.1,
            far: 1e12,
            aspect: 16.0 / 9.0,
        }
    }
}

/// Entity the camera orbits or focuses on.
#[derive(Component, Debug, Clone, Copy)]
pub struct CameraTarget(pub Entity);

/// Distance from the camera to its target in meters.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CameraDistance(pub f32);

/// Input tuning parameters for camera movement.
#[derive(Component, Debug, Clone, Copy)]
pub struct CameraState {
    /// Movement speed in m/s.
    pub speed: f32,
    /// Mouse / drag sensitivity.
    pub sensitivity: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            speed: 1000.0,
            sensitivity: 0.005,
        }
    }
}

/// Marker component: camera moves freely via WASD / QE.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FlyCamera;

/// Marker component: camera orbits a target entity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OrbitCamera;
