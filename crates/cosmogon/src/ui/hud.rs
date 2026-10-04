//! In-universe HUD: top bar, timeline, toasts, pause menu, debug and help.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::time::{format_duration, SPEEDS};

use super::{icon, UiState, ACCENT, ACCENT_DIM, CIV, LIFE, MUTED, TEXT};
use crate::camera::{find_target, CameraRig};
use crate::persistence::{list_saves, UserSettings};
use crate::render::ViewInfo;
use crate::sim::{Sim, Target};
use crate::state::AppState;

pub fn keyboard(mut contexts: EguiContexts, keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>, mut ui: ResMut<UiState>, mut settings: ResMut<UserSettings>, time: Res<Time>, args: Res<crate::args::Args>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if args.capture.is_some() {
        return Ok(());
    }
    let now = time.elapsed_secs_f64();
    let cmd = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::SuperRight) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // Shortcuts that work even while a text field has focus.
    if cmd && keys.just_pressed(KeyCode::KeyK) {
        ui.palette_open = !ui.palette_open;
    }
    if cmd && keys.just_pressed(KeyCode::KeyS) {
        super::tools::run_command(super::tools::Cmd::Save, &mut ui, &mut sim, &mut settings, now);
    }
    if ctx.wants_keyboard_input() {
        return Ok(());
    }
    if cmd && keys.just_pressed(KeyCode::KeyZ) {
        if shift { sim.redo(now) } else { sim.undo(now) }
        return Ok(());
    }
    if cmd && keys.just_pressed(KeyCode::KeyY) {
        sim.redo(now);
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
    if keys.just_pressed(KeyCode::KeyP) {
        ui.photo_requested = true;
    }
    if keys.just_pressed(KeyCode::F3) {
        ui.debug = !ui.debug;
    }
    if keys.just_pressed(KeyCode::F1) {
        ui.help = !ui.help;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if matches!(sim.tool, crate::sim::Tool::Launch(_)) {
            sim.tool = crate::sim::Tool::Select;
        } else if ui.create_open {
            ui.create_open = false;
        } else {
            ui.pause_menu = !ui.pause_menu;
        }
    }
    Ok(())
}

/// A square icon button with a tooltip; `on` draws it as active.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str, on: bool, enabled: bool) -> egui::Response {
    let text = egui::RichText::new(glyph).size(17.0).color(if !enabled { MUTED.gamma_multiply(0.5) } else if on { ACCENT } else { TEXT });
    let mut b = egui::Button::new(text).min_size(egui::vec2(32.0, 30.0)).corner_radius(7);
    b = if on { b.fill(egui::Color32::from_rgb(58, 45, 22)).stroke(egui::Stroke::new(1.0_f32, ACCENT_DIM)) } else { b.fill(egui::Color32::TRANSPARENT) };
    ui.add_enabled(enabled, b).on_hover_text(tip)
}

pub fn top_bar(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, mut settings: ResMut<UserSettings>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden {
        return Ok(());
    }
    let now = time.elapsed_secs_f64();
    let frame = egui::Frame::new().fill(super::PANEL).inner_margin(egui::Margin::symmetric(10, 6)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(24, 30, 44)));
    egui::TopBottomPanel::top("top").exact_height(46.0).frame(frame).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            if icon_button(ui, icon::LIST, "Menu (Esc)", ui_state.pause_menu, true).clicked() {
                ui_state.pause_menu = !ui_state.pause_menu;
            }
            ui.label(egui::RichText::new("COSMOGON").size(13.0).color(TEXT).extra_letter_spacing(3.5));
            let reference = sim.is_reference();
            let (badge, color) = if reference { ("REFERENCE", egui::Color32::from_rgb(110, 180, 240)) } else { ("SANDBOX", ACCENT) };
            egui::Frame::new().fill(color.gamma_multiply(0.12)).corner_radius(10).inner_margin(egui::Margin::symmetric(8, 2)).show(ui, |ui| {
                ui.label(egui::RichText::new(badge).size(9.5).color(color).extra_letter_spacing(1.2));
            });
            let title = sim.session.title();
            ui.label(egui::RichText::new(format!("{title}{}", if sim.dirty && !reference { "  •" } else { "" })).color(MUTED).size(12.5)).on_hover_text(if sim.dirty { "Unsaved changes (Cmd/Ctrl+S; autosave also runs)" } else { "Saved" });
            if reference && ui.add(egui::Button::new(egui::RichText::new(format!("{}  Clone to sandbox", icon::COPY)).color(ACCENT)).corner_radius(7)).on_hover_text("Copy this real-data state into your own experiment").clicked() {
                ui_state.clone_dialog = Some("My Solar System experiment".into());
            }

            // Centre: search (Enter jumps there; Cmd/Ctrl+K opens every command).
            let avail = ui.available_width();
            ui.add_space((avail * 0.5 - 330.0).max(8.0));
            egui::Frame::new().fill(egui::Color32::from_rgb(16, 21, 33)).corner_radius(9).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(32, 40, 58))).inner_margin(egui::Margin::symmetric(10, 3)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon::MAGNIFYING_GLASS).color(MUTED));
                    let resp = ui.add(egui::TextEdit::singleline(&mut ui_state.search).hint_text("Search worlds, stars, civilizations…").frame(false).desired_width(260.0));
                    // Shortcut hint as a key cap, not squeezed into the placeholder.
                    egui::Frame::new().fill(egui::Color32::from_rgb(28, 35, 52)).corner_radius(5).inner_margin(egui::Margin::symmetric(6, 1)).show(ui, |ui| {
                        ui.label(egui::RichText::new(if cfg!(target_os = "macos") { "⌘K" } else { "Ctrl K" }).size(11.0).color(MUTED));
                    }).response.on_hover_text("Command palette");
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
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, icon::SIDEBAR_SIMPLE, "Inspector panel", ui_state.show_right, true).clicked() {
                    ui_state.show_right = !ui_state.show_right;
                }
                if icon_button(ui, icon::TREE_STRUCTURE, "Universe panel (worlds, chronicle, civilizations)", ui_state.show_left, true).clicked() {
                    ui_state.show_left = !ui_state.show_left;
                }
                if icon_button(ui, icon::QUESTION, "Controls (F1)", ui_state.help, true).clicked() {
                    ui_state.help = !ui_state.help;
                }
                if icon_button(ui, icon::CAMERA, "Photo (P): save a clean screenshot to Pictures/Cosmogon", false, true).clicked() {
                    ui_state.photo_requested = true;
                }
                ui.menu_button(egui::RichText::new(icon::EYE).size(17.0), |ui| {
                    ui.label(egui::RichText::new("SHOW").size(10.0).color(ACCENT));
                    ui.checkbox(&mut settings.show_orbits, "Orbits");
                    ui.checkbox(&mut settings.show_labels, "Labels");
                    ui.checkbox(&mut settings.show_trails, "Trails (past path)");
                    ui.checkbox(&mut settings.show_predictions, "Predicted trajectory");
                    ui.checkbox(&mut settings.show_velocity, "Velocity vector");
                    ui.separator();
                    ui.checkbox(&mut settings.advanced, "Advanced mode (all fields, scientific units)");
                })
                .response
                .on_hover_text("What to show");
                if icon_button(ui, icon::ATOM, "Physics: gravity model, accuracy, diagnostics", ui_state.physics_open, true).clicked() {
                    ui_state.physics_open = !ui_state.physics_open;
                }
                if !reference {
                    ui.separator();
                    if icon_button(ui, icon::FLOPPY_DISK, "Save (Cmd/Ctrl+S)", false, true).clicked() {
                        super::tools::run_command(super::tools::Cmd::Save, &mut ui_state, &mut sim, &mut settings, now);
                    }
                    let redo = sim.redo_label().map(|s| s.to_string());
                    if icon_button(ui, icon::ARROW_CLOCKWISE, &redo.clone().map(|u| format!("Redo: {u} (Shift+Cmd/Ctrl+Z)")).unwrap_or_else(|| "Nothing to redo".into()), false, redo.is_some()).clicked() {
                        sim.redo(now);
                    }
                    let undo = sim.undo_label().map(|s| s.to_string());
                    if icon_button(ui, icon::ARROW_COUNTER_CLOCKWISE, &undo.clone().map(|u| format!("Undo: {u} (Cmd/Ctrl+Z)")).unwrap_or_else(|| "Nothing to undo".into()), false, undo.is_some()).clicked() {
                        sim.undo(now);
                    }
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
    let frame = egui::Frame::new().fill(super::PANEL).inner_margin(egui::Margin::symmetric(14, 8)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(24, 30, 44)));
    egui::TopBottomPanel::bottom("timeline").exact_height(62.0).frame(frame).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            // Play / pause.
            let (glyph, tip) = if sim.paused { (icon::PLAY, "Resume (Space)") } else { (icon::PAUSE, "Pause (Space)") };
            let play = egui::Button::new(egui::RichText::new(glyph).size(20.0).color(egui::Color32::from_rgb(20, 16, 8))).fill(ACCENT).corner_radius(20).min_size(egui::vec2(40.0, 40.0));
            if ui.add(play).on_hover_text(tip).clicked() {
                sim.paused = !sim.paused;
            }
            ui.add_space(6.0);
            // Speed: a stepped slider over every available rate.
            ui.vertical(|ui| {
                ui.set_width(210.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon::GAUGE).color(MUTED));
                    ui.label(egui::RichText::new(SPEEDS[sim.speed].label).color(ACCENT).size(13.0));
                    let requested = sim.rate();
                    if !sim.paused && sim.effective_rate > 0.0 && sim.effective_rate < requested * 0.7 {
                        ui.label(egui::RichText::new(format!("{} CPU-limited · {}/s", icon::WARNING, format_duration(sim.effective_rate))).size(10.0).color(MUTED))
                            .on_hover_text("Every step is computed — gravity, climate, every civilization year — and none is skipped to go faster. It is running as fast as this computer allows.");
                    }
                });
                let mut idx = sim.speed as f32;
                let max = (SPEEDS.len() - 1) as f32;
                ui.spacing_mut().slider_width = 196.0;
                let slider = egui::Slider::new(&mut idx, 0.0..=max).step_by(1.0).show_value(false).trailing_fill(true);
                if ui.add(slider).on_hover_text("Simulation speed (, and . keys)").changed() {
                    sim.speed = idx.round() as usize;
                }
            });
            ui.add_space(10.0);
            ui.separator();
            ui.vertical(|ui| {
                let u = &sim.universe;
                // Human-scale speeds get a calendar clock; geological ones a year count.
                let clock = if u.gregorian() && sim.rate() < 1000.0 * cosmogon_sim::time::SECONDS_PER_YEAR { cosmogon_sim::time::format_datetime(u.time) } else { u.date_label() };
                ui.label(egui::RichText::new(clock).size(17.0).color(TEXT));
                let dynamic = rig.focus.map(|f| u.system(f.system()).is_dynamic()).unwrap_or(false);
                let model = if dynamic { "N-body gravity" } else { "Kepler orbits" };
                ui.label(egui::RichText::new(format!("{} elapsed · {model}", format_duration(u.time - u.start_time))).size(10.5).color(MUTED));
            });
            ui.separator();
            ui.checkbox(&mut settings.auto_slow, egui::RichText::new("auto-slow").size(11.0).color(MUTED)).on_hover_text("Slow down automatically for milestones");
            ui.separator();
            // Recent notable events, newest first, as compact chips.
            let events: Vec<_> = sim.universe.history.events.iter().rev().filter(|e| e.importance >= 4).take(3).cloned().collect();
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for e in events {
                    use cosmogon_sim::history::Category as C;
                    let (glyph, color) = match e.category {
                        C::Life => (icon::LEAF, LIFE),
                        C::Technology => (icon::LIGHTBULB, CIV),
                        C::Civilization => (icon::BUILDINGS, CIV),
                        C::Space => (icon::ROCKET_LAUNCH, CIV),
                        C::Contact => (icon::BROADCAST, CIV),
                        C::Disaster => (icon::WARNING, super::DANGER),
                        C::War => (icon::SWORD, super::DANGER),
                        C::Astronomy => (icon::STAR, TEXT),
                    };
                    let resp = ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(glyph).size(12.0).color(color));
                        ui.label(egui::RichText::new(cosmogon_sim::time::format_date(e.time, sim.universe.start_time, sim.universe.gregorian())).size(10.5).color(MUTED));
                        ui.add(egui::Label::new(egui::RichText::new(&e.title).size(11.5).color(color)).truncate());
                    });
                    let resp = resp.response.interact(egui::Sense::click()).on_hover_text(&e.detail);
                    if resp.clicked() {
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
    egui::Area::new(egui::Id::new("toasts")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 54.0)).interactable(true).show(ctx, |ui| {
        for t in toasts.iter() {
            let age = (now - t.born) as f32;
            let alpha = (1.0 - ((age - 7.5) / 1.5).clamp(0.0, 1.0)) * (age / 0.3).clamp(0.0, 1.0);
            ui.scope(|ui| {
                ui.set_opacity(alpha);
                egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(18, 16, 10, 240)).stroke(egui::Stroke::new(1.0_f32, ACCENT)).corner_radius(8).inner_margin(12).show(ui, |ui| {
                    ui.set_width(520.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(450.0);
                            ui.label(egui::RichText::new(&t.title).strong().color(ACCENT));
                            ui.label(egui::RichText::new(&t.detail).size(11.5).color(TEXT));
                        });
                        if t.target.is_some() && ui.button("Go").clicked() {
                            go = t.target;
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
    });
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
                    let star = crate::render::to_render(sim.universe.system(f.system()).star_position(sim.universe.time));
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
                    ("P", "Photo: save a clean screenshot to Pictures/Cosmogon"),
                    ("Ctrl/⌘ + S", "Save sandbox"),
                    ("Ctrl/⌘ + Z / ⇧Z", "Undo / redo an edit"),
                    ("Ctrl/⌘ + K", "Command palette"),
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
            ui.set_width(320.0);
            let w = egui::vec2(300.0, 30.0);
            if ui.add(egui::Button::new("Resume").min_size(w)).clicked() {
                close = true;
            }
            if sim.is_reference() {
                if ui.add(egui::Button::new("Clone to sandbox…").min_size(w)).clicked() {
                    ui_state.clone_dialog = Some("My Solar System experiment".into());
                    close = true;
                }
            } else {
                if ui.add(egui::Button::new("Save").min_size(w)).clicked() {
                    match sim.save(now) {
                        Ok(_) => close = true,
                        Err(e) => sim.status = Some((format!("Save failed: {e}"), now)),
                    }
                }
                if ui.add(egui::Button::new("Save as new sandbox…").min_size(w)).clicked() {
                    ui_state.save_as_dialog = Some(format!("{} (branch)", sim.session.title()));
                    close = true;
                }
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut ui_state.checkpoint_label).hint_text("Checkpoint name").desired_width(190.0));
                    if ui.button("Create checkpoint").clicked() {
                        let label = if ui_state.checkpoint_label.trim().is_empty() { sim.universe.date_label() } else { ui_state.checkpoint_label.trim().to_string() };
                        let u = sim.universe.clone();
                        if let crate::sim::Session::Sandbox(m) = &mut sim.session {
                            match crate::persistence::create_checkpoint(m, &u, &label) {
                                Ok(()) => sim.status = Some((format!("Checkpoint “{label}” saved"), now)),
                                Err(e) => sim.status = Some((format!("Checkpoint failed: {e}"), now)),
                            }
                        }
                        ui_state.checkpoint_label.clear();
                    }
                });
                let checkpoints = match &sim.session {
                    crate::sim::Session::Sandbox(m) => m.checkpoints.iter().map(|c| (c.label.clone(), c.sim_date.clone(), m.dir().join(&c.file))).collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                if !checkpoints.is_empty() {
                    ui.label(egui::RichText::new("CHECKPOINTS").size(10.5).color(ACCENT));
                    for (label, date, path) in checkpoints.iter().rev().take(8) {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{label} — {date}")).size(11.5));
                            if ui.small_button("Restore").on_hover_text("Undoable").clicked() {
                                match crate::persistence::load_universe_file(path) {
                                    Ok(u) => {
                                        sim.replace_universe(u, &format!("Restored checkpoint “{label}”"), now);
                                        close = true;
                                    }
                                    Err(e) => sim.status = Some((format!("Restore failed: {e}"), now)),
                                }
                            }
                        });
                    }
                }
            }
            if ui.add(egui::Button::new("Open another sandbox…").min_size(w)).clicked() {
                let _ = sim.autosave();
                ui_state.sandboxes = crate::persistence::list_sandboxes();
                ui_state.saves = list_saves();
                ui_state.menu = super::MenuScreen::Load;
                next.set(AppState::MainMenu);
                close = true;
            }
            ui.separator();
            super::home::settings_ui(ui, &mut settings);
            ui.separator();
            if ui.add(egui::Button::new("Home").min_size(w)).clicked() {
                let _ = sim.autosave();
                ui_state.menu = super::MenuScreen::Home;
                next.set(AppState::MainMenu);
                close = true;
            }
            if ui.add(egui::Button::new("Quit to desktop").min_size(w)).clicked() {
                let _ = sim.autosave();
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
