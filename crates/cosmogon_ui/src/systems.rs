//! ECS systems and resources that drive the UI panels.

use bevy_ecs::prelude::*;
use cosmogon_ecs::resources::global_time::GlobalTime;
use cosmogon_ecs::resources::render_state::FrameStats;

use crate::camera_panel::CameraPanel;
use crate::debug_overlay::DebugOverlay;
use crate::inspector::InspectorPanel;
use crate::time_controls::TimeControlsPanel;

/// Central UI state resource. Add this to the world and each panel system
/// reads from it.
///
/// Panel visibility toggles are provided so a menu bar or hotkeys can show /
/// hide individual panels.
#[derive(Resource)]
pub struct UiState {
    /// Object inspector panel.
    pub inspector: InspectorPanel,
    /// Time controls panel.
    pub time_controls: TimeControlsPanel,
    /// Camera settings panel.
    pub camera_panel: CameraPanel,
    /// Debug overlay.
    pub debug_overlay: DebugOverlay,

    /// Whether the object inspector is visible.
    pub show_inspector: bool,
    /// Whether the time-controls panel is visible.
    pub show_time_controls: bool,
    /// Whether the camera-settings panel is visible.
    pub show_camera_panel: bool,
    /// Whether the debug overlay is visible.
    pub show_debug: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            inspector: InspectorPanel,
            time_controls: TimeControlsPanel,
            camera_panel: CameraPanel::default(),
            debug_overlay: DebugOverlay,
            show_inspector: true,
            show_time_controls: true,
            show_camera_panel: true,
            show_debug: true,
        }
    }
}

/// System: refresh the entity count stored in [`FrameStats`] every frame.
///
/// This is intentionally lightweight — a full entity iteration — so that the
/// debug overlay always shows a current count. More expensive stats (draw
/// calls, etc.) should be updated by the renderer directly.
pub fn update_frame_stats(
    time: Res<GlobalTime>,
    mut stats: ResMut<FrameStats>,
    query: Query<Entity>,
) {
    stats.entities = query.iter().count();
    let _ = time; // time is available here for future per-frame metric logging
}
