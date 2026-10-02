use bevy_ecs::prelude::*;

/// Global rendering configuration.
#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    /// Enable vertical sync.
    pub vsync: bool,
    /// Enable multi-sample anti-aliasing.
    pub msaa: bool,
    /// Enable high-dynamic-range rendering.
    pub hdr: bool,
    /// Enable bloom post-process effect.
    pub bloom_enabled: bool,
    /// Bloom intensity multiplier.
    pub bloom_intensity: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            vsync: true,
            msaa: true,
            hdr: true,
            bloom_enabled: true,
            bloom_intensity: 0.3,
        }
    }
}

/// Per-frame rendering statistics.
#[derive(Resource, Debug, Clone)]
pub struct FrameStats {
    /// Frames per second.
    pub fps: f64,
    /// Time to render the last frame in milliseconds.
    pub frame_time_ms: f64,
    /// Number of tracked entities.
    pub entities: usize,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            entities: 0,
        }
    }
}

/// The currently selected / focused celestial body, if any.
#[derive(Resource, Debug, Clone)]
pub struct SelectedObject(pub Option<Entity>);

impl Default for SelectedObject {
    fn default() -> Self {
        Self(None)
    }
}
