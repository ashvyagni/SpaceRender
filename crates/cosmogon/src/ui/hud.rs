//! In-universe HUD: top bar, timeline, toasts, pause menu, debug and help.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::time::{format_duration, SPEEDS};

use super::{UiState, ACCENT, CIV, LIFE, MUTED, TEXT};
use crate::camera::{find_target, CameraRig};
use crate::persistence::{list_saves, UserSettings};
use crate::render::ViewInfo;
use crate::sim::{Sim, Target};
use crate::state::AppState;

pub fn keyboard(mut contexts: EguiContexts, keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>, mut ui: ResMut<UiState>, time: Res<Time>, args: Res<crate::args::Args>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if args.capture.is_some() {
        return Ok(());
    }
    if ctx.wants_keyboard_input() {
        return Ok(());
    }
    if keys.just_pressed(KeyCode::Space) {
        sim.paused = !sim.paused;
    }
    if keys.just_pressed(KeyCode::Period) || keys.just_pressed(KeyCode::Equal) {
        sim.speed = (sim.speed + 1).min(SPEEDS.len() - 1);
    }
    if keys.just_pressed(KeyCode::Comma) || keys.just_pressed(KeyCode::Minus) {
        sim.speed = sim.speed.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::Tab) {
        ui.hidden = !ui.hidden;
    }
    if keys.just_pressed(KeyCode::F3) {
        ui.debug = !ui.debug;
    }
    if keys.just_pressed(KeyCode::F1) {
        ui.help = !ui.help;
    }
    if keys.just_pressed(KeyCode::Escape) {
        ui.pause_menu = !ui.pause_menu;
    }
    let cmd = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::SuperRight) || keys.pressed(KeyCode::ControlRight);
    if cmd && keys.just_pressed(KeyCode::KeyS) {
        let stem = format!("quicksave-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
        if let Err(e) = sim.save(&stem, time.elapsed_secs_f64()) {
            sim.status = Some((format!("Save failed: {e}"), time.elapsed_secs_f64()));
        }
    }
    Ok(())
}

pub fn top_bar(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, mut settings: ResMut<UserSettings>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden {
        return Ok(());
    }
    let now = time.elapsed_secs_f64();
    egui::TopBottomPanel::top("top").exact_height(40.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(egui::Margin::symmetric(12, 6))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            if ui.button("☰").on_hover_text("Menu (Esc)").clicked() {
                ui_state.pause_menu = !ui_state.pause_menu;
            }
            ui.label(egui::RichText::new("COSMOGON").strong().extra_letter_spacing(3.0));
            ui.label(egui::RichText::new(format!("{} · seed {}", sim.universe.settings.scenario.label(), sim.universe.settings.seed)).color(MUTED).size(12.0));
            ui.separator();
            // Search across systems, bodies and civilizations.
            let resp = ui.add(egui::TextEdit::singleline(&mut ui_state.search).hint_text("Search worlds, stars, civilizations…").desired_width(260.0));
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let q = ui_state.search.trim().to_string();
                let found = find_target(&sim, &q).or_else(|| sim.universe.civs.iter().find(|c| c.name.to_lowercase().contains(&q.to_lowercase())).map(|c| Target::Body(cosmogon_sim::BodyRef { system: c.system, body: c.body })));
                match found {
                    Some(t) => {
                        sim.selected = Some(t);
                        rig.focus_on(t, &sim, None);
                        ui_state.search.clear();
                    }
                    None => sim.status = Some((format!("Nothing called “{q}”"), now)),
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut ui_state.show_right, "Inspector");
                ui.toggle_value(&mut ui_state.show_left, "Universe");
                ui.toggle_value(&mut settings.show_labels, "Labels");
                ui.toggle_value(&mut settings.show_orbits, "Orbits");
                if ui.button("?").on_hover_text("Controls (F1)").clicked() {
                    ui_state.help = !ui_state.help;
                }
                if let Some((m, at)) = sim.status.clone() {
                    if now - at < 5.0 {
                        ui.label(egui::RichText::new(m).color(ACCENT).size(12.0));
                    }
                }
            });
        });
    });
    Ok(())
}

pub fn bottom_bar(mut contexts: EguiContexts, ui_state: Res<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, mut settings: ResMut<UserSettings>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden {
        return Ok(());
    }
    egui::TopBottomPanel::bottom("timeline").exact_height(64.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(egui::Margin::symmetric(14, 8))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let play = if sim.paused { "▶" } else { "⏸" };
            if ui.add_sized([36.0, 32.0], egui::Button::new(egui::RichText::new(play).size(18.0))).on_hover_text("Pause / resume (Space)").clicked() {
                sim.paused = !sim.paused;
            }
            if ui.add_sized([28.0, 32.0], egui::Button::new("−")).on_hover_text("Slower (,)").clicked() {
                sim.speed = sim.speed.saturating_sub(1);
            }
            ui.vertical(|ui| {
                ui.set_width(118.0);
                ui.label(egui::RichText::new(SPEEDS[sim.speed].label).strong().color(ACCENT));
                let requested = sim.rate();
                if !sim.paused && sim.effective_rate > 0.0 && sim.effective_rate < requested * 0.7 {
                    ui.label(egui::RichText::new(format!("CPU-limited: {}/s", format_duration(sim.effective_rate))).size(10.5).color(MUTED))
                        .on_hover_text("The simulation steps every civilization year by year and never skips steps to go faster. It is running as fast as this computer allows.");
                } else {
                    ui.label(egui::RichText::new(if sim.paused { "paused" } else { "running" }).size(10.5).color(MUTED));
                }
            });
            if ui.add_sized([28.0, 32.0], egui::Button::new("+")).on_hover_text("Faster (.)").clicked() {
                sim.speed = (sim.speed + 1).min(SPEEDS.len() - 1);
            }
            ui.separator();
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(sim.universe.date_label()).size(17.0).strong());
                ui.label(egui::RichText::new(format!("{} since observation began", format_duration(sim.universe.time - sim.universe.start_time))).size(10.5).color(MUTED));
            });
            ui.separator();
            ui.checkbox(&mut settings.auto_slow, "").on_hover_text("Slow down automatically for milestones");
            ui.label(egui::RichText::new("auto-slow").size(11.0).color(MUTED));
            ui.separator();
            // Recent notable events.
            let events: Vec<_> = sim.universe.history.events.iter().rev().filter(|e| e.importance >= 4).take(3).cloned().collect();
            ui.vertical(|ui| {
                for e in events {
                    let color = match e.category {
                        cosmogon_sim::history::Category::Life => LIFE,
                        cosmogon_sim::history::Category::Technology | cosmogon_sim::history::Category::Civilization => CIV,
                        _ => TEXT,
                    };
                    let text = egui::RichText::new(format!("{} — {}", cosmogon_sim::time::format_date(e.time, sim.universe.start_time, sim.universe.gregorian()), e.title)).size(11.5).color(color);
                    if ui.add(egui::Label::new(text).sense(egui::Sense::click()).truncate()).on_hover_text(&e.detail).clicked() {
                        if let (Some(s), Some(b)) = (e.system, e.body) {
                            let t = Target::Body(cosmogon_sim::BodyRef { system: s, body: b });
                            sim.selected = Some(t);
                            rig.focus_on(t, &sim, None);
                        }
                    }
                }
            });
        });
    });
    Ok(())
}

pub fn toasts(mut contexts: EguiContexts, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, time: Res<Time>, ui_state: Res<UiState>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden {
        return Ok(());
    }
    let now = time.elapsed_secs_f64();
    let toasts = sim.toasts.clone();
    let mut go = None;
    for (i, t) in toasts.iter().enumerate() {
        let age = (now - t.born) as f32;
        let alpha = (1.0 - ((age - 7.5) / 1.5).clamp(0.0, 1.0)) * (age / 0.3).clamp(0.0, 1.0);
        egui::Area::new(egui::Id::new(("toast", i)))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 54.0 + i as f32 * 74.0))
            .show(ctx, |ui| {
                ui.set_opacity(alpha);
                egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(18, 16, 10, 240)).stroke(egui::Stroke::new(1.0_f32, ACCENT)).corner_radius(8).inner_margin(12).show(ui, |ui| {
                    ui.set_width(440.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&t.title).strong().color(ACCENT));
                            ui.label(egui::RichText::new(&t.detail).size(11.5).color(TEXT));
                        });
                        if t.target.is_some() && ui.button("Go").clicked() {
                            go = t.target;
                        }
                    });
                });
            });
    }
    if let Some(t) = go {
        sim.selected = Some(t);
        rig.focus_on(t, &sim, None);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn overlays_windows(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut sim: ResMut<Sim>,
    mut settings: ResMut<UserSettings>,
    diagnostics: Res<DiagnosticsStore>,
    view: Res<ViewInfo>,
    rig: Res<CameraRig>,
    lod: Res<crate::render::terrain_lod::TerrainLod>,
    entities: Query<Entity>,
    pending: Query<(), With<crate::render::PendingBake>>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
    time: Res<Time>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let now = time.elapsed_secs_f64();
    if ui_state.debug {
        let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()).unwrap_or(0.0);
        let ms = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|d| d.smoothed()).unwrap_or(0.0);
        let mut open = true;
        egui::Window::new("Developer").open(&mut open).default_pos([20.0, 60.0]).show(ctx, |ui| {
            egui::Grid::new("dbg").num_columns(2).show(ui, |ui| {
                let u = &sim.universe;
                super::kv(ui, "FPS", format!("{fps:.0}  ({ms:.1} ms)"));
                super::kv(ui, "Sim CPU / frame", format!("{:.2} ms  ({} steps{})", sim.sim_ms, sim.last.steps, if sim.last.lagging { ", lagging" } else { "" }));
                super::kv(ui, "Speed", format!("{} req · {}/s achieved", SPEEDS[sim.speed].label, format_duration(sim.effective_rate)));
                super::kv(ui, "Entities", entities.iter().count().to_string());
                super::kv(ui, "Texture bakes in flight", pending.iter().count().to_string());
                super::kv(ui, "Systems / bodies", format!("{} / {}", u.systems.len(), u.systems.iter().map(|s| s.bodies.len()).sum::<usize>()));
                super::kv(ui, "Biospheres / civs", format!("{} / {}", u.biospheres.len(), u.civs.len()));
                super::kv(ui, "History events", u.history.events.len().to_string());
                super::kv(ui, "Seed", u.settings.seed.to_string());
                super::kv(ui, "Sim time (s)", format!("{:.6e}", u.time));
                super::kv(ui, "Camera distance", super::distance(rig.distance));
                super::kv(ui, "Altitude above ground", super::distance(rig.distance - rig.ground_radius));
                if let Some(f) = rig.focus {
                    let center = crate::camera::target_info(&sim, f).0;
                    let star = crate::render::to_render(sim.universe.system(f.system()).position);
                    let up = (view.origin - center).normalize();
                    let sun = (star - center).normalize();
                    super::kv(ui, "Sun elevation at camera", format!("{:.1}°", up.dot(sun).clamp(-1.0, 1.0).asin().to_degrees()));
                }
                super::kv(ui, "Terrain patches", format!("{} visible, {} building", lod.visible_patches, lod.building));
                super::kv(ui, "Floating origin", format!("({:.3e}, {:.3e}, {:.3e})", view.origin.x, view.origin.y, view.origin.z));
            });
        });
        ui_state.debug = open;
    }
    if ui_state.help {
        let mut open = true;
        egui::Window::new("Controls").open(&mut open).collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            egui::Grid::new("help").num_columns(2).striped(true).show(ui, |ui| {
                for (k, v) in [
                    ("Drag (left or right)", "Orbit the camera"),
                    ("Scroll · W / S", "Zoom (works from metres to light-years)"),
                    ("Click marker / list item", "Select"),
                    ("Double-click · F", "Fly to the selection"),
                    ("Home", "View the whole star system"),
                    ("Space", "Pause / resume"),
                    (", and .", "Slower / faster"),
                    ("Tab", "Hide the interface"),
                    ("Ctrl/⌘ + S", "Quick save"),
                    ("F3", "Developer overlay"),
                    ("Esc", "Menu"),
                ] {
                    ui.label(egui::RichText::new(k).color(ACCENT));
                    ui.label(v);
                    ui.end_row();
                }
            });
        });
        ui_state.help = open;
    }
    if ui_state.pause_menu {
        let mut close = false;
        egui::Window::new("Cosmogon").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_width(300.0);
            let w = egui::vec2(280.0, 30.0);
            if ui.add(egui::Button::new("Resume").min_size(w)).clicked() {
                close = true;
            }
            if ui.add(egui::Button::new("Save").min_size(w)).clicked() {
                let stem = sim.save_name.replace(['/', '\\', ':', '#', ' ', '—'], "_");
                match sim.save(&stem, now) {
                    Ok(_) => close = true,
                    Err(e) => sim.status = Some((format!("Save failed: {e}"), now)),
                }
            }
            if ui.add(egui::Button::new("Load…").min_size(w)).clicked() {
                ui_state.saves = list_saves();
                ui_state.menu = super::MenuScreen::Load;
                next.set(AppState::MainMenu);
                close = true;
            }
            if ui.add(egui::Button::new("New universe…").min_size(w)).clicked() {
                ui_state.menu = super::MenuScreen::NewUniverse;
                next.set(AppState::MainMenu);
                close = true;
            }
            ui.separator();
            super::menu::settings_ui(ui, &mut settings);
            ui.separator();
            if ui.add(egui::Button::new("Main menu").min_size(w)).clicked() {
                ui_state.menu = super::MenuScreen::Home;
                next.set(AppState::MainMenu);
                close = true;
            }
            if ui.add(egui::Button::new("Quit to desktop").min_size(w)).clicked() {
                let _ = sim.save("autosave", now);
                exit.write(AppExit::Success);
            }
        });
        if close {
            settings.save();
            ui_state.pause_menu = false;
        }
    }
    Ok(())
}
