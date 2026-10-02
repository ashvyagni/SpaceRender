//! Top-level application states.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    /// A universe is being created (or loaded) on a background thread.
    Generating,
    /// A universe is being observed.
    Observing,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
        configure_frame(app);
    }
}

/// Per-frame ordering. Simulation first, then positions derived from it, then the camera
/// (which defines the floating origin), then everything expressed relative to that origin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame {
    Simulate,
    Positions,
    Camera,
    Apply,
}

pub fn configure_frame(app: &mut App) {
    app.configure_sets(Update, (Frame::Simulate, Frame::Positions, Frame::Camera, Frame::Apply).chain());
}
