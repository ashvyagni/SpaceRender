use bevy_ecs::prelude::*;

/// Global simulation time state, shared across all systems.
#[derive(Resource, Debug, Clone)]
pub struct GlobalTime {
    /// Total elapsed simulation seconds.
    pub elapsed: f64,
    /// Time step (delta) for the current frame in seconds.
    pub delta: f64,
    /// Time multiplier (1.0 = real-time).
    pub acceleration: f64,
    /// Whether simulation is paused.
    pub paused: bool,
    /// Number of frames elapsed.
    pub frame_count: u64,
}

impl Default for GlobalTime {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            delta: 0.0,
            acceleration: 1.0,
            paused: false,
            frame_count: 0,
        }
    }
}
