//! Minimal charts drawn with the egui painter (no extra dependency).

use bevy_egui::egui;

use super::{ACCENT, MUTED};

/// A line chart of `(x, y)` points; `log_y` plots log10(y).
pub fn line_chart(ui: &mut egui::Ui, title: &str, points: &[(f64, f64)], log_y: bool, fmt: impl Fn(f64) -> String) {
    ui.label(egui::RichText::new(title).size(11.5).color(MUTED));
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 74.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5));
    if points.len() < 2 {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "not enough history yet", egui::FontId::proportional(11.0), MUTED);
        return;
    }
    let ty = |y: f64| if log_y { y.max(1e-9).log10() } else { y };
    let (x0, x1) = (points[0].0, points[points.len() - 1].0);
    let (mut y0, mut y1) = (f64::MAX, f64::MIN);
    for p in points {
        y0 = y0.min(ty(p.1));
        y1 = y1.max(ty(p.1));
    }
    if (y1 - y0).abs() < 1e-9 {
        y1 = y0 + 1.0;
    }
    let to_screen = |x: f64, y: f64| {
        let fx = if x1 > x0 { (x - x0) / (x1 - x0) } else { 0.0 };
        let fy = (ty(y) - y0) / (y1 - y0);
        egui::pos2(rect.left() + 4.0 + fx as f32 * (rect.width() - 8.0), rect.bottom() - 6.0 - fy as f32 * (rect.height() - 18.0))
    };
    let pts: Vec<egui::Pos2> = points.iter().map(|p| to_screen(p.0, p.1)).collect();
    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.6_f32, ACCENT)));
    let last = points[points.len() - 1].1;
    painter.text(rect.right_top() + egui::vec2(-6.0, 4.0), egui::Align2::RIGHT_TOP, fmt(last), egui::FontId::proportional(11.0), ACCENT);
}

/// Horizontal bar with a label, value in 0..1.
pub fn bar(ui: &mut egui::Ui, label: &str, value: f64, color: egui::Color32, note: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 16.0], egui::Label::new(egui::RichText::new(label).size(12.0)).truncate());
        let (rect, resp) = ui.allocate_exact_size(egui::vec2((ui.available_width() - 4.0).max(40.0), 10.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 3.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12));
        let mut fill = rect;
        fill.set_width(rect.width() * value.clamp(0.0, 1.0) as f32);
        p.rect_filled(fill, 3.0, color);
        if !note.is_empty() {
            resp.on_hover_text(note);
        }
    });
}
