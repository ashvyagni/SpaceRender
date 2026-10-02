//! Simulation time control panel — pause, resume, and adjust time scale.

use cosmogon_ecs::resources::global_time::GlobalTime;
use egui::Ui;

/// Stateless time-controls panel. Call [`TimeControlsPanel::show`] each frame.
pub struct TimeControlsPanel;

impl TimeControlsPanel {
    /// Draw the time controls into `ui`, modifying `time` in-place.
    pub fn show(&self, ui: &mut Ui, time: &mut GlobalTime) {
        ui.heading("Time Controls");
        ui.separator();

        // Pause / Resume
        let pause_label = if time.paused { "\u{25B6} Resume" } else { "\u{23F8} Pause" };
        if ui.button(pause_label).clicked() {
            time.paused = !time.paused;
        }

        ui.add_space(4.0);

        // Preset speed buttons
        ui.horizontal(|ui| {
            ui.label("Speed:");
            for &speed in &[1.0, 10.0, 100.0, 1_000.0, 10_000.0] {
                let label = format_speed(speed);
                if ui
                    .selectable_label(time.acceleration == speed, &label)
                    .clicked()
                {
                    time.acceleration = speed;
                }
            }
        });

        // Custom slider
        ui.horizontal(|ui| {
            ui.label("Custom:");
            ui.add(
                egui::Slider::new(&mut time.acceleration, 0.0..=100_000.0)
                    .logarithmic(true)
                    .text("x"),
            );
        });

        ui.separator();

        // Display elapsed time
        let days = time.elapsed / 86_400.0;
        let years = days / 365.25;
        ui.label(format!("Elapsed: {days:.1} days ({years:.2} years)"));
        ui.label(format!("Frame: {}", time.frame_count));
        ui.label(format!("Delta: {:.4} s", time.delta));
    }
}

/// Format a time-acceleration value as a compact label.
fn format_speed(speed: f64) -> String {
    if speed >= 10_000.0 {
        format!("{}x", speed as i64)
    } else if speed >= 1.0 {
        format!("{}x", speed as i64)
    } else {
        format!("{speed:.1}x")
    }
}
