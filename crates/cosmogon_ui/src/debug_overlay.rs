//! Debug information overlay — FPS, frame time, entity count, and simulation state.

use cosmogon_ecs::resources::global_time::GlobalTime;
use cosmogon_ecs::resources::render_state::FrameStats;
use egui::Ui;

/// Stateless debug overlay. Draws a semi-transparent floating window via
/// [`egui::Ui::window`].
pub struct DebugOverlay;

impl DebugOverlay {
    /// Draw the debug overlay.
    ///
    /// The overlay is rendered as a collapsible, resizable floating window so
    /// it can be positioned anywhere by the user.
    pub fn show(&self, ui: &mut Ui, stats: &FrameStats, time: &GlobalTime) {
        egui::Window::new("Debug")
            .resizable(true)
            .collapsible(true)
            .default_open(true)
            .show(ui.ctx(), |ui| {
                ui.monospace(format!("FPS: {:.1}", stats.fps));
                ui.monospace(format!("Frame time: {:.2} ms", stats.frame_time_ms));
                ui.monospace(format!("Entities: {}", stats.entities));
                ui.separator();
                ui.monospace(format!("Sim time: {:.1} days", time.elapsed / 86_400.0));
                ui.monospace(format!("Time scale: {}x", time.acceleration));
                ui.monospace(format!("Paused: {}", time.paused));
                ui.monospace(format!("Frame: {}", time.frame_count));
            });
    }
}
