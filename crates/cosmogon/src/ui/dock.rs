//! The tool dock (bottom centre): Select · Throw · Grab · Create · Aim, and the object
//! shelf that opens above it while throwing.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::sandbox::preset;

use super::{icon, UiState, ACCENT, ACCENT_DIM, MUTED, TEXT};
use crate::camera::CameraRig;
use crate::interact::{drag_speed, Drag};
use crate::sim::{Sim, ThrowSpec, Tool};

/// Shelf groups: (title, [(preset id, swatch colour, short name)]).
const SHELF: &[(&str, &[(&str, [u8; 3], &str)])] = &[
    ("Planets", &[("earth_like", [70, 130, 200], "Earth-like"), ("mars_like", [196, 110, 64], "Mars-like"), ("venus_like", [228, 204, 150], "Venus-like"), ("ocean_world", [40, 90, 170], "Ocean"), ("ice_world", [220, 214, 200], "Ice world"), ("gas_giant", [212, 180, 140], "Gas giant"), ("ice_giant", [90, 140, 220], "Ice giant")]),
    ("Small bodies", &[("moon_like", [150, 148, 145], "Moon"), ("dwarf_planet", [210, 190, 170], "Dwarf"), ("asteroid_1km", [130, 120, 110], "Asteroid"), ("asteroid_10km", [115, 105, 95], "Impactor"), ("comet_5km", [170, 200, 220], "Comet")]),
    ("Stars", &[("sun_like_star", [255, 236, 190], "Sun-like"), ("red_dwarf", [255, 140, 90], "Red dwarf"), ("brown_dwarf", [150, 70, 60], "Brown dw.")]),
    ("Exotic", &[("white_dwarf", [220, 230, 255], "White dwarf"), ("neutron_star", [150, 190, 255], "Pulsar"), ("black_hole_1", [20, 20, 24], "BH 1 M☉"), ("black_hole_10", [10, 10, 12], "BH 10 M☉")]),
];

fn tool_button(ui: &mut egui::Ui, glyph: &str, label: &str, tip: &str, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(62.0, 50.0), egui::Sense::click());
    let resp = resp.on_hover_text(tip);
    let p = ui.painter();
    let hover = resp.hovered();
    if on {
        p.rect_filled(rect, 10.0, egui::Color32::from_rgb(58, 45, 22));
        p.rect_stroke(rect, 10.0, egui::Stroke::new(1.0_f32, ACCENT_DIM), egui::StrokeKind::Inside);
    } else if hover {
        p.rect_filled(rect, 10.0, egui::Color32::from_rgb(26, 33, 50));
    }
    let color = if on { ACCENT } else if hover { TEXT } else { MUTED.gamma_multiply(1.3) };
    p.text(rect.center() - egui::vec2(0.0, 7.0), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(21.0), color);
    p.text(rect.center() + egui::vec2(0.0, 15.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(10.5), color);
    resp
}

pub fn tool_dock(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, drag: Res<Drag>, rig: Res<CameraRig>, args: Res<crate::args::Args>, mut applied: Local<bool>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if !*applied {
        *applied = true;
        match args.panel.as_deref() {
            Some("throw") => sim.tool = Tool::Throw(ThrowSpec { preset: "black_hole_1" }),
            Some("grab") => sim.tool = Tool::Grab,
            _ => {}
        }
    }
    if ui_state.hidden || sim.is_reference() {
        return Ok(());
    }
    let throwing = match &sim.tool {
        Tool::Throw(t) => Some(t.preset),
        _ => None,
    };
    let frame = egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(11, 14, 24, 238)).corner_radius(14).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(36, 44, 62))).inner_margin(6).shadow(egui::epaint::Shadow { offset: [0, 6], blur: 22, spread: 0, color: egui::Color32::from_black_alpha(150) });

    egui::Area::new(egui::Id::new("tool_shelf")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -176.0)).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            // The shelf, while throwing.
            if let Some(current) = throwing {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (gi, (title, items)) in SHELF.iter().enumerate() {
                            if gi > 0 {
                                ui.separator();
                            }
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(title.to_uppercase()).size(9.5).color(MUTED).extra_letter_spacing(1.0));
                                ui.horizontal(|ui| {
                                    for (id, rgb, name) in items.iter() {
                                        let on = *id == current;
                                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(50.0, 54.0), egui::Sense::click());
                                        let tip = preset(id).map(|p| format!("{}\n{}", p.label, p.description)).unwrap_or_default();
                                        let resp = resp.on_hover_text(tip);
                                        let p = ui.painter();
                                        if on {
                                            p.rect_filled(rect, 8.0, egui::Color32::from_rgb(58, 45, 22));
                                        } else if resp.hovered() {
                                            p.rect_filled(rect, 8.0, egui::Color32::from_rgb(26, 33, 50));
                                        }
                                        let c = rect.center() - egui::vec2(0.0, 8.0);
                                        let col = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                        let r = 13.0;
                                        if id.starts_with("black_hole") {
                                            p.circle_stroke(c, r + 2.0, egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(255, 150, 60)));
                                            p.circle_filled(c, r, egui::Color32::BLACK);
                                        } else if matches!(*id, "sun_like_star" | "red_dwarf" | "white_dwarf" | "neutron_star") {
                                            p.circle_filled(c, r + 4.0, col.gamma_multiply(0.25));
                                            p.circle_filled(c, r, col);
                                        } else {
                                            p.circle_filled(c, r, col);
                                            // Terminator shading for a planet look.
                                            p.circle_filled(c + egui::vec2(4.0, 3.0), r * 0.8, col.gamma_multiply(0.75));
                                            p.circle_stroke(c, r, egui::Stroke::new(1.0_f32, col.gamma_multiply(1.3)));
                                        }
                                        p.text(rect.center() + egui::vec2(0.0, 18.0), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(9.5), if on { ACCENT } else { TEXT });
                                        if resp.clicked() {
                                            sim.tool = Tool::Throw(ThrowSpec { preset: id });
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
                // What to do, and the speed while dragging.
                let hint = match drag_speed(&sim, &drag, &rig) {
                    Some(v) => format!("{}  {v:.2} km/s relative to the focus — release to throw", icon::SHOOTING_STAR),
                    None => format!("Press in space and drag to throw · click for a circular orbit · right-drag turns the camera · Esc stops"),
                };
                ui.add_space(4.0);
                ui.label(egui::RichText::new(hint).size(11.0).color(if drag.active.is_some() { ACCENT } else { MUTED }));
                ui.add_space(4.0);
            } else if matches!(sim.tool, Tool::Grab) {
                ui.label(egui::RichText::new("Press on an object and drag to move it (it keeps its velocity) · right-drag turns the camera · Esc stops").size(11.0).color(MUTED));
                ui.add_space(4.0);
            }
        });
    });
    egui::Area::new(egui::Id::new("tool_dock")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -108.0)).order(egui::Order::Foreground).show(ctx, |ui| {
        {
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let select = matches!(sim.tool, Tool::Select);
                    if tool_button(ui, icon::CURSOR, "Select", "Select and inspect (click), fly to (double-click)", select).clicked() {
                        sim.tool = Tool::Select;
                    }
                    if tool_button(ui, icon::SHOOTING_STAR, "Throw", "Throw planets, stars, asteroids and black holes: press and drag in space", throwing.is_some()).clicked() {
                        sim.tool = if throwing.is_some() { Tool::Select } else { Tool::Throw(ThrowSpec { preset: "asteroid_10km" }) };
                    }
                    if tool_button(ui, icon::HAND_GRABBING, "Grab", "Drag objects to new places", matches!(sim.tool, Tool::Grab)).clicked() {
                        sim.tool = if matches!(sim.tool, Tool::Grab) { Tool::Select } else { Tool::Grab };
                    }
                    let launching = matches!(sim.tool, Tool::Launch(_));
                    if tool_button(ui, icon::CROSSHAIR, "Aim", "Aim an object precisely at a target and see the impact before it happens", launching).clicked() {
                        sim.tool = if launching { Tool::Select } else { Tool::Launch(Default::default()) };
                    }
                    ui.separator();
                    if tool_button(ui, icon::PLUS_CIRCLE, "Create", "Add an object on a chosen orbit with exact values", ui_state.create_open).clicked() {
                        ui_state.create_open = !ui_state.create_open;
                    }
                    if tool_button(ui, icon::FLASK, "Experiments", "Curated “what if” experiments", ui_state.palette_open).clicked() {
                        ui_state.palette_open = !ui_state.palette_open;
                    }
                });
            });
        }
    });
    Ok(())
}
