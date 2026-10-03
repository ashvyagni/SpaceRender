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

pub fn top_bar(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, mut settings: ResMut<UserSettings>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden {
        return Ok(());
    }
    let now = time.elapsed_secs_f64();
    egui::TopBottomPanel::top("top").exact_height(44.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(egui::Margin::symmetric(12, 6))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            if ui.button("☰").on_hover_text("Menu (Esc)").clicked() {
                ui_state.pause_menu = !ui_state.pause_menu;
            }
            ui.label(egui::RichText::new("COSMOGON").strong().extra_letter_spacing(3.0));
            let reference = sim.is_reference();
            let (badge, color) = if reference { ("REFERENCE · READ-ONLY", egui::Color32::from_rgb(110, 180, 240)) } else { ("SANDBOX", ACCENT) };
            egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, color)).corner_radius(4).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                ui.label(egui::RichText::new(badge).size(10.0).color(color).strong());
            });
            let title = sim.session.title();
            ui.label(egui::RichText::new(format!("{title}{}", if sim.dirty && !reference { " •" } else { "" })).color(TEXT).size(13.0)).on_hover_text(if sim.dirty { "Unsaved changes (Cmd/Ctrl+S to save; autosave also runs)" } else { "Saved" });
            ui.separator();
            if reference {
                if ui.add(egui::Button::new(egui::RichText::new("Clone to sandbox").strong()).fill(egui::Color32::from_rgb(70, 54, 24))).on_hover_text("Copy this real-data state into your own experiment").clicked() {
                    ui_state.clone_dialog = Some("My Solar System experiment".into());
                }
            } else {
                let undo = sim.undo_label().map(|s| s.to_string());
                let redo = sim.redo_label().map(|s| s.to_string());
                if ui.add_enabled(undo.is_some(), egui::Button::new("Undo")).on_hover_text(undo.map(|u| format!("Undo: {u} (Cmd/Ctrl+Z)")).unwrap_or_default()).clicked() {
                    sim.undo(now);
                }
                if ui.add_enabled(redo.is_some(), egui::Button::new("Redo")).on_hover_text(redo.map(|u| format!("Redo: {u} (Shift+Cmd/Ctrl+Z)")).unwrap_or_default()).clicked() {
                    sim.redo(now);
                }
                ui.separator();
                if ui.selectable_label(ui_state.create_open, "+ Create").on_hover_text("Add a planet, moon, asteroid or comet").clicked() {
                    ui_state.create_open = !ui_state.create_open;
                }
                let launching = matches!(sim.tool, crate::sim::Tool::Launch(_));
                if ui.selectable_label(launching, "Launch").on_hover_text("Throw an object at a target").clicked() {
                    sim.tool = if launching { crate::sim::Tool::Select } else { crate::sim::Tool::Launch(Default::default()) };
                }
            }
            if ui.selectable_label(ui_state.physics_open, "Physics").clicked() {
                ui_state.physics_open = !ui_state.physics_open;
            }
            ui.separator();
            // Search across systems, bodies and civilizations (⌘K for everything).
            let resp = ui.add(egui::TextEdit::singleline(&mut ui_state.search).hint_text("Search… (Cmd/Ctrl+K: commands)").desired_width(190.0));
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
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut settings.show_orbits, "Orbits");
                    ui.checkbox(&mut settings.show_labels, "Labels");
                    ui.checkbox(&mut settings.show_trails, "Trails (past path)");
                    ui.checkbox(&mut settings.show_predictions, "Predicted trajectory");
                    ui.checkbox(&mut settings.show_velocity, "Velocity vector");
                    ui.separator();
                    ui.checkbox(&mut settings.advanced, "Advanced mode");
                });
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
                        .on_hover_text("Every step is computed — gravity, climate, every civilization year — and none is skipped to go faster. It is running as fast as this computer allows.");
                } else {
                    ui.label(egui::RichText::new(if sim.paused { "paused" } else { "running" }).size(10.5).color(MUTED));
                }
            });
            if ui.add_sized([28.0, 32.0], egui::Button::new("+")).on_hover_text("Faster (.)").clicked() {
                sim.speed = (sim.speed + 1).min(SPEEDS.len() - 1);
            }
            ui.separator();
            ui.vertical(|ui| {
                let u = &sim.universe;
                // Human-scale speeds get a calendar clock; geological ones a year count.
                let clock = if u.gregorian() && sim.rate() < 1000.0 * cosmogon_sim::time::SECONDS_PER_YEAR { cosmogon_sim::time::format_datetime(u.time) } else { u.date_label() };
                ui.label(egui::RichText::new(clock).size(17.0).strong());
                let dynamic = rig.focus.map(|f| u.system(f.system()).is_dynamic()).unwrap_or(false);
                let model = if dynamic { "N-body gravity" } else { "Kepler orbits" };
                ui.label(egui::RichText::new(format!("{} elapsed · {model}", format_duration(u.time - u.start_time))).size(10.5).color(MUTED));
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
