//! Camera settings panel — sliders for distance, speed, and field of view.

use egui::Ui;

/// Camera settings panel. Holds local UI state that is forwarded to the
/// camera component each frame by the caller.
pub struct CameraPanel {
    /// Orbital camera distance from the target (meters, f32 for UI).
    pub distance: f32,
    /// Camera movement speed multiplier.
    pub speed: f32,
    /// Vertical field of view in degrees.
    pub fov: f32,
}

impl Default for CameraPanel {
    fn default() -> Self {
        Self {
            distance: 10.0,
            speed: 1.0,
            fov: 60.0,
        }
    }
}

impl CameraPanel {
    /// Draw the camera settings into `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Camera");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Distance:");
            ui.add(
                egui::Slider::new(&mut self.distance, 0.1..=1_000.0)
                    .logarithmic(true)
                    .text("m"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut self.speed, 0.1..=100.0).text("x"));
        });

        ui.horizontal(|ui| {
            ui.label("FOV:");
            ui.add(egui::Slider::new(&mut self.fov, 10.0..=120.0).text("\u{00B0}"));
        });
    }
}
