//! Sandbox tools: Create object, Launch, Physics settings, the command palette and the
//! clone / save-as dialogs. Every change goes through `Sim::edit` (undoable).

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_physics::nbody::Particle;
use cosmogon_sim::astro::dynamics::{PhysicsPreset, PhysicsSettings, Slot};
use cosmogon_sim::astro::{ObjectClass, AU, G};
use cosmogon_sim::sandbox::{aimed_state, body_from_preset, circular_state, Edit, PRESETS};
use cosmogon_sim::time::{format_duration, SECONDS_PER_DAY, SPEEDS};
use cosmogon_sim::{BodyRef, Vec3d};

use super::units::{edit as qty, Quantity};
use super::{UiState, ACCENT, DANGER, MUTED, TEXT};
use crate::camera::CameraRig;
use crate::persistence::UserSettings;
use crate::sim::{LaunchSpec, Session, Sim, Target, Tool};

/// The create-object form.
#[derive(Clone, Debug)]
pub struct CreateForm {
    pub preset: usize,
    pub name: String,
    pub mass: f64,
    pub radius: f64,
    /// Body index of the parent; `None` = the star.
    pub parent: Option<u32>,
    pub distance: f64,
    pub phase_deg: f64,
    pub inclination_deg: f64,
    /// Speed as a multiple of the circular-orbit speed (√2 ≈ escape).
    pub speed_factor: f64,
    pub loaded_preset: Option<usize>,
    pub preview_at: f64,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self { preset: 0, name: "Nova".into(), mass: 0.0, radius: 0.0, parent: None, distance: 1.26 * AU, phase_deg: 30.0, inclination_deg: 0.0, speed_factor: 1.0, loaded_preset: None, preview_at: -10.0 }
    }
}

enum Act {
    None,
    Create,
    Launch,
    Edit(Edit),
    Focus(Target),
    Command(Cmd),
}

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    OpenCreate,
    OpenLaunch,
    OpenPhysics,
    Save,
    Undo,
    Redo,
    TogglePause,
    ToggleOrbits,
    ToggleLabels,
    ToggleTrails,
    TogglePredictions,
    ToggleVelocity,
    ToggleAdvanced,
    Speed(usize),
    CloneToSandbox,
    HideUi,
}

fn parent_label(sim: &Sim, system: u32, p: Option<u32>) -> String {
    let sys = sim.universe.system(system);
    match p {
        Some(i) => sys.bodies[i as usize].name.clone(),
        None => format!("{} (star)", sys.star.name),
    }
}

/// The proposed new body's state for the current form.
fn proposed(sim: &Sim, system: u32, f: &CreateForm) -> Option<(cosmogon_sim::astro::Body, cosmogon_sim::astro::dynamics::State)> {
    let p = PRESETS.get(f.preset)?;
    let mut body = body_from_preset(p, &f.name, cosmogon_sim::rng::mix(f.name.len() as u64, (f.mass.to_bits()) ^ 0xC05A));
    body.mass = f.mass.max(1.0);
    body.radius = f.radius.max(1.0);
    body.parent = f.parent;
    let sys = sim.universe.system(system);
    let t = sim.universe.time;
    let mut st = circular_state(sys, f.parent.map(|x| x as usize), body.mass, f.distance, f.phase_deg.to_radians(), f.inclination_deg.to_radians(), t);
    let center_vel = match f.parent {
        Some(i) => sys.body_state(i as usize, t).vel,
        None => sys.dynamics.as_ref().map(|d| d.star.vel).unwrap_or_default(),
    };
    st.vel = center_vel + (st.vel - center_vel) * f.speed_factor;
    Some((body, st))
}

#[allow(clippy::too_many_arguments)]
pub fn tool_windows(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, mut settings: ResMut<UserSettings>, keys: Res<ButtonInput<KeyCode>>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let now = time.elapsed_secs_f64();
    let system = rig.focus.map(|f| f.system()).unwrap_or(0);
    let mut act = Act::None;
    let s = &mut *ui_state;
    let adv = settings.advanced;

    // ── Create object ──
    if s.create_open {
        let mut open = true;
        egui::Window::new("Create object").open(&mut open).default_pos([330.0, 70.0]).default_width(420.0).resizable(false).show(ctx, |ui| {
            let f = &mut s.create;
            // Category list: creatable presets, then classes that arrive in later milestones.
            egui::ComboBox::from_id_salt("preset").width(380.0).selected_text(PRESETS[f.preset].label).show_ui(ui, |ui| {
                for (i, p) in PRESETS.iter().enumerate() {
                    ui.selectable_value(&mut f.preset, i, format!("{} — {}", p.label, p.class.group().label())).on_hover_text(p.description);
                }
                ui.separator();
                for c in ObjectClass::ALL.iter().filter(|c| !c.creatable()) {
                    ui.add_enabled(false, egui::Button::new(format!("{} · {}", c.label(), c.planned_milestone())).frame(false));
                }
            });
            let p = &PRESETS[f.preset];
            if f.loaded_preset != Some(f.preset) {
                let b = body_from_preset(p, p.label, 1);
                f.mass = b.mass;
                f.radius = b.radius;
                f.name = match p.class {
                    ObjectClass::Asteroid => "Asteroid".into(),
                    ObjectClass::Comet => "Comet".into(),
                    ObjectClass::Moon => "New moon".into(),
                    _ => "Nova".into(),
                };
                if p.class == ObjectClass::Moon && f.parent.is_none() {
                    f.parent = sim.universe.system(system).find_body("Earth").map(|i| i as u32).or_else(|| sim.universe.system(system).planets().next().map(|i| i as u32));
                    f.distance = 7.0e8;
                }
                f.loaded_preset = Some(f.preset);
            }
            ui.label(egui::RichText::new(p.description).size(11.0).color(MUTED));
            ui.add_space(6.0);
            egui::Grid::new("create").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut f.name);
                ui.end_row();
                ui.label("Mass");
                if let Some(v) = qty(ui, "c_mass", f.mass, Quantity::Mass, settings.mass_unit, true, None) {
                    f.mass = v;
                }
                ui.end_row();
                ui.label("Radius");
                if let Some(v) = qty(ui, "c_radius", f.radius, Quantity::Length, 1, true, None) {
                    f.radius = v;
                }
                ui.end_row();
                ui.label("Orbit around");
                let sysr = sim.universe.system(system);
                egui::ComboBox::from_id_salt("parent").selected_text(parent_label(&sim, system, f.parent)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut f.parent, None, parent_label(&sim, system, None));
                    for i in sysr.existing() {
                        if sysr.bodies[i].mass > 1e20 {
                            ui.selectable_value(&mut f.parent, Some(i as u32), sysr.bodies[i].name.clone());
                        }
                    }
                });
                ui.end_row();
                ui.label("Distance");
                if let Some(v) = qty(ui, "c_dist", f.distance, Quantity::Length, if f.parent.is_none() { 5 } else { 1 }, true, None) {
                    f.distance = v.max(1.0);
                }
                ui.end_row();
                ui.label("Position on orbit");
                ui.add(egui::Slider::new(&mut f.phase_deg, 0.0..=360.0).suffix("°"));
                ui.end_row();
                if adv {
                    ui.label("Inclination");
                    ui.add(egui::Slider::new(&mut f.inclination_deg, -90.0..=90.0).suffix("°"));
                    ui.end_row();
                }
                ui.label("Speed");
                ui.vertical(|ui| {
                    ui.add(egui::Slider::new(&mut f.speed_factor, 0.0..=2.0).text("× circular"));
                    let note = if (f.speed_factor - 1.0).abs() < 0.01 {
                        "circular orbit"
                    } else if f.speed_factor >= std::f64::consts::SQRT_2 {
                        "escape: it will leave"
                    } else if f.speed_factor < 1.0 {
                        "falls inward on an ellipse"
                    } else {
                        "eccentric orbit, outward"
                    };
                    ui.label(egui::RichText::new(note).size(11.0).color(MUTED));
                });
                ui.end_row();
            });
            let density = f.mass / (4.0 / 3.0 * std::f64::consts::PI * f.radius.powi(3));
            ui.label(egui::RichText::new(format!("Density {density:.0} kg/m³ · surface gravity {:.2} g", G * f.mass / f.radius.powi(2) / 9.80665)).size(11.5).color(MUTED));
            if density > 25_000.0 {
                ui.colored_label(DANGER, "⚠ Physically unusual: denser than any ordinary planetary matter. Allowed anyway.");
            } else if density < 300.0 {
                ui.colored_label(DANGER, "⚠ Physically unusual: less dense than any known solid body. Allowed anyway.");
            }
            if let Some((None, p)) = &sim.prediction.result {
                if let Some((a, b, tc)) = p.contact {
                    if a == Slot::Body(u32::MAX) || b == Slot::Body(u32::MAX) {
                        ui.colored_label(DANGER, format!("⚠ Predicted collision in {}", format_duration(tc - sim.universe.time)));
                    }
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(egui::RichText::new("Add to sandbox").strong()).fill(egui::Color32::from_rgb(90, 66, 24))).clicked() {
                    act = Act::Create;
                }
                ui.label(egui::RichText::new("The simulation pauses so you can inspect it before pressing play.").size(10.5).color(MUTED));
            });
        });
        if !open {
            s.create_open = false;
            sim.prediction.result = None;
        }
        // Live preview of where it will go.
        if s.create_open && now - s.create.preview_at > 0.6 && sim.prediction.task.is_none() {
            if let Some((b, st)) = proposed(&sim, system, &s.create) {
                s.create.preview_at = now;
                let particle = Particle::new(st.pos, st.vel, G * b.mass, b.radius);
                let horizon = 2.0 * 365.25 * SECONDS_PER_DAY;
                let sys = sim.universe.system(system).clone();
                let t = sim.universe.time;
                sim.prediction.subject = None;
                sim.prediction.started_at = now;
                sim.prediction.task = Some(bevy::tasks::AsyncComputeTaskPool::get().spawn(async move { sys.predict(t, Some(particle), horizon, 600, 20_000) }));
            }
        }
    }

    // ── Launch tool ──
    if let Tool::Launch(mut l) = sim.tool.clone() {
        let mut open = true;
        let sysr = sim.universe.system(system);
        if l.target.is_none() {
            l.target = sysr.find_body("Earth").or_else(|| sysr.planets().next()).map(|i| BodyRef { system, body: i as u32 });
        }
        egui::Window::new("Launch").open(&mut open).default_pos([330.0, 70.0]).default_width(400.0).resizable(false).show(ctx, |ui| {
            ui.label(egui::RichText::new("Throw an object at a target. The path bends under every body's gravity.").size(11.5).color(MUTED));
            egui::Grid::new("launch").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Projectile");
                let cur = PRESETS.iter().find(|p| p.id == l.preset).map(|p| p.label).unwrap_or("?");
                egui::ComboBox::from_id_salt("proj").selected_text(cur).show_ui(ui, |ui| {
                    for p in PRESETS.iter().filter(|p| matches!(p.class, ObjectClass::Asteroid | ObjectClass::Comet | ObjectClass::Moon | ObjectClass::RockyPlanet | ObjectClass::DwarfPlanet)) {
                        ui.selectable_value(&mut l.preset, p.id, p.label);
                    }
                });
                ui.end_row();
                ui.label("Target");
                let tname = l.target.map(|t| sim.universe.body(t).name.clone()).unwrap_or_default();
                egui::ComboBox::from_id_salt("tgt").selected_text(tname).show_ui(ui, |ui| {
                    for i in sysr.existing() {
                        ui.selectable_value(&mut l.target, Some(BodyRef { system, body: i as u32 }), sysr.bodies[i].name.clone());
                    }
                });
                ui.end_row();
                ui.label("Start distance");
                if let Some(v) = qty(ui, "l_dist", l.distance, Quantity::Length, 1, true, None) {
                    l.distance = v.max(1.0);
                }
                ui.end_row();
                ui.label("Approach speed");
                if let Some(v) = qty(ui, "l_speed", l.speed, Quantity::Speed, 1, true, None) {
                    l.speed = v.max(0.0);
                }
                ui.end_row();
                ui.label("Miss distance");
                if let Some(v) = qty(ui, "l_miss", l.miss, Quantity::Length, 1, true, None) {
                    l.miss = v.max(0.0);
                }
                ui.end_row();
                ui.label("Comes from");
                ui.vertical(|ui| {
                    let mut az = l.azimuth.to_degrees();
                    let mut el = l.elevation.to_degrees();
                    ui.add(egui::Slider::new(&mut az, -180.0..=180.0).suffix("° around"));
                    ui.add(egui::Slider::new(&mut el, -89.0..=89.0).suffix("° above plane"));
                    l.azimuth = az.to_radians();
                    l.elevation = el.to_radians();
                });
                ui.end_row();
            });
            match &sim.prediction.result {
                Some((None, p)) => match p.contact {
                    Some((a, b, tc)) if a == Slot::Body(u32::MAX) || b == Slot::Body(u32::MAX) => {
                        let other = if a == Slot::Body(u32::MAX) { b } else { a };
                        let name = match other {
                            Slot::Body(i) => sim.universe.system(system).bodies.get(i as usize).map(|b| b.name.clone()).unwrap_or_default(),
                            _ => sim.universe.system(system).star.name.clone(),
                        };
                        ui.colored_label(DANGER, format!("Predicted impact on {name} in {}", format_duration(tc - sim.universe.time)));
                    }
                    _ => {
                        ui.label(egui::RichText::new("No impact predicted in the next two years.").color(MUTED));
                    }
                },
                _ => {
                    ui.label(egui::RichText::new("Computing trajectory…").color(MUTED));
                }
            }
            ui.add_space(6.0);
            if ui.add(egui::Button::new(egui::RichText::new("Release").strong()).fill(egui::Color32::from_rgb(110, 50, 30))).clicked() {
                act = Act::Launch;
            }
        });
        if !open {
            sim.tool = Tool::Select;
            sim.prediction.result = None;
        } else {
            let changed = sim.tool != Tool::Launch(l.clone());
            sim.tool = Tool::Launch(l.clone());
            if (changed || now - sim.prediction.started_at > 1.0) && sim.prediction.task.is_none() {
                if let Some((body, st)) = launch_state(&sim, &l) {
                    let particle = Particle::new(st.pos, st.vel, G * body.mass, body.radius);
                    sim.request_prediction(None, Some(particle), 2.0 * 365.25 * SECONDS_PER_DAY, now);
                }
            }
        }
    }

    // ── Physics settings ──
    if s.physics_open {
        let mut open = true;
        egui::Window::new("Physics").open(&mut open).default_pos([330.0, 90.0]).default_width(460.0).show(ctx, |ui| {
            let sysr = sim.universe.system(system);
            ui.label(egui::RichText::new(format!("{} system", sysr.name)).strong());
            match &sysr.dynamics {
                None => {
                    ui.label(egui::RichText::new("Fixed orbits (analytic Kepler, fidelity tier 1): exact at any time scale, but bodies do not pull on each other.").size(11.5).color(MUTED));
                    if ui.button("Switch to dynamic gravity (N-body)").clicked() {
                        act = Act::Edit(Edit::SetPhysics { system, nbody: true, settings: sim.universe.settings.physics.unwrap_or_default() });
                    }
                }
                Some(d) => {
                    let mut st: PhysicsSettings = d.settings;
                    ui.label(egui::RichText::new("Dynamic gravity: Newtonian N-body, 4th-order symplectic integrator (tier 2), automatic encounter substepping (tier 3).").size(11.5).color(MUTED));
                    egui::Grid::new("presets").num_columns(2).striped(true).show(ui, |ui| {
                        for p in PhysicsPreset::ALL {
                            if ui.selectable_label(st.preset == p, p.label()).clicked() {
                                st = if p == PhysicsPreset::Custom { PhysicsSettings { preset: p, ..st } } else { p.settings() };
                            }
                            ui.label(egui::RichText::new(p.description()).size(10.5).color(MUTED));
                            ui.end_row();
                        }
                    });
                    if st.preset == PhysicsPreset::Custom {
                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut st.steps_per_orbit, 16.0..=5000.0).logarithmic(true).text("steps per orbit"));
                        });
                        ui.checkbox(&mut st.relativity, "General-relativistic correction (1PN, from the star)");
                    }
                    if st != d.settings {
                        act = Act::Edit(Edit::SetPhysics { system, nbody: true, settings: st });
                    }
                    ui.separator();
                    let g = &d.diagnostics;
                    egui::Grid::new("diag").num_columns(2).show(ui, |ui| {
                        super::kv(ui, "Integrated bodies", format!("{} + star", d.active_count()));
                        super::kv(ui, "Moons on rails (tier 1)", sysr.existing().filter(|&i| d.bodies.get(i).copied().flatten().is_none()).count().to_string());
                        super::kv(ui, "Macro step", format_duration(d.dt));
                        super::kv(ui, "Substeps (last / max)", format!("{} / {}", g.last_substeps, g.max_substeps));
                        super::kv(ui, "Energy error since last change", format!("{:.2e}", g.energy_error));
                        if g.saturated_steps > 0 {
                            super::kv(ui, "Encounters beyond step limit", egui::RichText::new(g.saturated_steps.to_string()).color(DANGER));
                        }
                    });
                    ui.add_space(4.0);
                    if ui.button("Freeze orbits (back to Kepler)").on_hover_text("Each body keeps its current osculating orbit; mutual perturbations stop. Fails if anything is unbound.").clicked() {
                        act = Act::Edit(Edit::SetPhysics { system, nbody: false, settings: d.settings });
                    }
                }
            }
            ui.label(egui::RichText::new("Time acceleration never enlarges the step: if the computer cannot keep up, time runs slower than requested (shown as CPU-limited).").size(10.5).color(MUTED));
        });
        if !open {
            s.physics_open = false;
        }
    }

    // ── Command palette ──
    if s.palette_open {
        let mut close = false;
        egui::Window::new("command_palette").title_bar(false).anchor(egui::Align2::CENTER_TOP, [0.0, 90.0]).fixed_size([520.0, 360.0]).show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut s.palette_query).hint_text("Type a command or an object name…").desired_width(500.0));
            r.request_focus();
            let q = s.palette_query.to_lowercase();
            let mut items: Vec<(String, Act)> = Vec::new();
            let mut add = |label: String, a: Act| {
                if q.is_empty() || label.to_lowercase().contains(&q) {
                    items.push((label, a));
                }
            };
            add("Create object…".into(), Act::Command(Cmd::OpenCreate));
            add("Launch an asteroid / comet…".into(), Act::Command(Cmd::OpenLaunch));
            add("Physics settings".into(), Act::Command(Cmd::OpenPhysics));
            add("Save sandbox".into(), Act::Command(Cmd::Save));
            add("Undo".into(), Act::Command(Cmd::Undo));
            add("Redo".into(), Act::Command(Cmd::Redo));
            add("Pause / resume".into(), Act::Command(Cmd::TogglePause));
            add("Toggle orbits".into(), Act::Command(Cmd::ToggleOrbits));
            add("Toggle labels".into(), Act::Command(Cmd::ToggleLabels));
            add("Toggle trails".into(), Act::Command(Cmd::ToggleTrails));
            add("Toggle predicted trajectories".into(), Act::Command(Cmd::TogglePredictions));
            add("Toggle velocity vectors".into(), Act::Command(Cmd::ToggleVelocity));
            add("Toggle Simple / Advanced mode".into(), Act::Command(Cmd::ToggleAdvanced));
            add("Hide interface (Tab)".into(), Act::Command(Cmd::HideUi));
            if sim.is_reference() {
                add("Clone to sandbox".into(), Act::Command(Cmd::CloneToSandbox));
            }
            for (i, sp) in SPEEDS.iter().enumerate() {
                add(format!("Set time: {}", sp.label), Act::Command(Cmd::Speed(i)));
            }
            for sys in &sim.universe.systems {
                add(format!("Jump to {} (star)", sys.star.name), Act::Focus(Target::Star(sys.id)));
                for i in sys.existing() {
                    add(format!("Jump to {}", sys.bodies[i].name), Act::Focus(Target::Body(BodyRef { system: sys.id, body: i as u32 })));
                }
            }
            let n = items.len();
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                s.palette_index = (s.palette_index + 1).min(n.saturating_sub(1));
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                s.palette_index = s.palette_index.saturating_sub(1);
            }
            s.palette_index = s.palette_index.min(n.saturating_sub(1));
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                let mut chosen = None;
                for (k, (label, _)) in items.iter().enumerate().take(80) {
                    let sel = k == s.palette_index;
                    let r = ui.selectable_label(sel, egui::RichText::new(label).color(if sel { ACCENT } else { TEXT }));
                    if sel {
                        r.scroll_to_me(None);
                    }
                    if r.clicked() || (sel && enter) {
                        chosen = Some(k);
                    }
                }
                if let Some(k) = chosen {
                    act = std::mem::replace(&mut items[k].1, Act::None);
                    close = true;
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
        if close {
            s.palette_open = false;
            s.palette_query.clear();
            s.palette_index = 0;
        }
    }

    // ── Clone / Save-as dialogs ──
    if let Some(mut name) = s.clone_dialog.clone() {
        let mut open = true;
        let mut done = false;
        egui::Window::new("Clone to sandbox").open(&mut open).collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.label(egui::RichText::new("The real data stays untouched; your sandbox is a copy of this moment.").size(11.5).color(MUTED));
            ui.text_edit_singleline(&mut name);
            if ui.add(egui::Button::new(egui::RichText::new("Create sandbox").strong()).fill(egui::Color32::from_rgb(90, 66, 24))).clicked() {
                done = true;
            }
        });
        s.clone_dialog = if open && !done { Some(name.clone()) } else { None };
        if done {
            if let Err(e) = sim.clone_to_sandbox(name.trim(), now) {
                sim.say(format!("Clone failed: {e}"), now);
            }
        }
    }
    if let Some(mut name) = s.save_as_dialog.clone() {
        let mut open = true;
        let mut done = false;
        egui::Window::new("Save as new sandbox").open(&mut open).collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.text_edit_singleline(&mut name);
            if ui.button("Save").clicked() {
                done = true;
            }
        });
        s.save_as_dialog = if open && !done { Some(name.clone()) } else { None };
        if done {
            if let Session::Sandbox(m) = &sim.session {
                let copy = super::home::save_as(m, name.trim());
                sim.session = Session::Sandbox(copy);
                let _ = sim.save(now);
            }
        }
    }

    // ── Apply ──
    match act {
        Act::None => {}
        Act::Create => {
            if let Some((body, state)) = proposed(&sim, system, &s.create) {
                sim.paused = true;
                if let Ok(out) = sim.edit(Edit::AddBody { system, body: Box::new(body), state }, now) {
                    if let Some(r) = out.created {
                        sim.selected = Some(Target::Body(r));
                        s.body_tab = super::BodyTab::Orbit;
                    }
                }
            }
        }
        Act::Launch => {
            if let Tool::Launch(l) = sim.tool.clone() {
                if let Some((body, state)) = launch_state(&sim, &l) {
                    if let Ok(out) = sim.edit(Edit::AddBody { system, body: Box::new(body), state }, now) {
                        sim.selected = out.created.map(Target::Body);
                        sim.tool = Tool::Select;
                        sim.paused = false;
                    }
                }
            }
        }
        Act::Edit(e) => {
            let _ = sim.edit(e, now);
        }
        Act::Focus(t) => {
            sim.selected = Some(t);
            rig.focus_on(t, &sim, None);
        }
        Act::Command(c) => run_command(c, s, &mut sim, &mut settings, now),
    }
    let _ = keys;
    Ok(())
}

pub fn run_command(c: Cmd, s: &mut UiState, sim: &mut Sim, settings: &mut UserSettings, now: f64) {
    match c {
        Cmd::OpenCreate => s.create_open = true,
        Cmd::OpenLaunch => sim.tool = Tool::Launch(LaunchSpec::default()),
        Cmd::OpenPhysics => s.physics_open = true,
        Cmd::Save => {
            if sim.is_reference() {
                s.clone_dialog = Some("My Solar System experiment".into());
            } else if let Err(e) = sim.save(now) {
                sim.say(format!("Save failed: {e}"), now);
            }
        }
        Cmd::Undo => sim.undo(now),
        Cmd::Redo => sim.redo(now),
        Cmd::TogglePause => sim.paused = !sim.paused,
        Cmd::ToggleOrbits => settings.show_orbits = !settings.show_orbits,
        Cmd::ToggleLabels => settings.show_labels = !settings.show_labels,
        Cmd::ToggleTrails => settings.show_trails = !settings.show_trails,
        Cmd::TogglePredictions => settings.show_predictions = !settings.show_predictions,
        Cmd::ToggleVelocity => settings.show_velocity = !settings.show_velocity,
        Cmd::ToggleAdvanced => settings.advanced = !settings.advanced,
        Cmd::Speed(i) => sim.speed = i.min(SPEEDS.len() - 1),
        Cmd::CloneToSandbox => s.clone_dialog = Some("My Solar System experiment".into()),
        Cmd::HideUi => s.hidden = !s.hidden,
    }
}

/// Body and system-frame state of the launch tool's projectile.
pub fn launch_state(sim: &Sim, l: &LaunchSpec) -> Option<(cosmogon_sim::astro::Body, cosmogon_sim::astro::dynamics::State)> {
    let target = l.target?;
    let p = cosmogon_sim::sandbox::preset(l.preset)?;
    let body = body_from_preset(p, &l.name_for(p.label), cosmogon_sim::rng::mix(l.distance.to_bits(), l.speed.to_bits()));
    let sys = sim.universe.system(target.system);
    let (sa, ca) = l.azimuth.sin_cos();
    let (se, ce) = l.elevation.sin_cos();
    let dir = Vec3d::new(ce * ca, ce * sa, se);
    let st = aimed_state(sys, target.body as usize, l.distance, dir, l.speed, l.miss, sim.universe.time);
    Some((body, st))
}

impl LaunchSpec {
    fn name_for(&self, label: &str) -> String {
        if label.contains("Comet") {
            "Comet".into()
        } else if label.contains("Asteroid") {
            "Asteroid".into()
        } else {
            self.name.clone()
        }
    }
}
