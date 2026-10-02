use bevy_ecs::prelude::*;

/// Per-entity simulation time in seconds since epoch.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SimulationTime(pub f64);

/// Per-entity time acceleration multiplier (1.0 = real-time).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TimeAcceleration(pub f64);

/// Per-entity pause flag.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TimePaused(pub bool);
