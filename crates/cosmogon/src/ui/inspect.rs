//! Left panel (universe browser, chronicle, civilizations) and right panel (inspector).

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::astro::{BodyKind, ResourceKind, AU, LIGHT_YEAR};
use cosmogon_sim::civ::knowledge::Domain;
use cosmogon_sim::civ::tech::{Status, TechGraph};
use cosmogon_sim::civ::{environment_for, CivStatus, Civilization};
use cosmogon_sim::history::Category;
use cosmogon_sim::life::Stage;
use cosmogon_sim::time::{format_date, format_duration, group_digits, SECONDS_PER_YEAR};
use cosmogon_sim::{BodyRef, Universe};

use super::charts::{bar, line_chart};
use super::{compact, heading, kv, power, BodyTab, LeftTab, UiState, ACCENT, CIV, DANGER, LIFE, MUTED, TEXT};
use crate::camera::CameraRig;
use crate::sim::{Sim, Target};
use super::units::{self, Quantity};
use cosmogon_sim::astro::{Quality, RemovalCause, SOLAR_MASS};
use cosmogon_sim::sandbox::{hill_radius, roche_limit, warnings, BodyProperty, Edit, StarProperty};

fn life_glyph(u: &Universe, r: BodyRef) -> Option<(&'static str, egui::Color32)> {
    if u.civ_on(r).is_some() {
        return Some(("●", CIV));
    }
    let b = u.biosphere(r)?;
    match b.stage {
        Stage::Sterile | Stage::Prebiotic => None,
        Stage::Microbial | Stage::ComplexCells => Some(("•", LIFE)),
        _ => Some(("●", LIFE)),
    }
}

enum Action {
    Focus(Target),
}

pub fn left_panel(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden || !ui_state.show_left {
        return Ok(());
    }
    let mut action = None;
    egui::SidePanel::left("universe").default_width(290.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(10)).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut ui_state.left_tab, LeftTab::Systems, "Systems");
            ui.selectable_value(&mut ui_state.left_tab, LeftTab::Chronicle, "Chronicle");
            ui.selectable_value(&mut ui_state.left_tab, LeftTab::Civilizations, "Civilizations");
        });
        ui.separator();
        let u = &sim.universe;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match ui_state.left_tab {
            LeftTab::Systems => {
                let mut order: Vec<u32> = (0..u.systems.len() as u32).collect();
                order.sort_by(|a, b| u.system(*a).position.length().total_cmp(&u.system(*b).position.length()));
                for sid in order {
                    let sys = u.system(sid);
                    let has_life = u.biospheres.iter().any(|b| b.system == sid && b.stage >= Stage::Microbial);
                    let has_civ = u.civs.iter().any(|c| c.system == sid && c.is_alive());
                    let mut title = format!("{}  ", sys.name);
                    title.push_str(&format!("{}  {:.1} ly", sys.star.spectral_type(u.time), sys.position.length() / LIGHT_YEAR));
                    let color = if has_civ { CIV } else if has_life { LIFE } else { TEXT };
                    let open_default = rig.focus.is_some_and(|f| f.system() == sid);
                    egui::CollapsingHeader::new(egui::RichText::new(title).color(color))
                        .id_salt(("sys", sid))
                        .open(if open_default { Some(true) } else { None })
                        .show(ui, |ui| {
                            let star_sel = sim.selected == Some(Target::Star(sid));
                            let r = ui.selectable_label(star_sel, format!("☀ {}", sys.star.name));
                            if r.clicked() {
                                action = Some(Action::Focus(Target::Star(sid)));
                            }
                            for p in sys.planets() {
                                body_row(ui, u, &sim, BodyRef { system: sid, body: p as u32 }, 0, &mut action);
                                for m in sys.moons_of(p) {
                                    body_row(ui, u, &sim, BodyRef { system: sid, body: m as u32 }, 1, &mut action);
                                }
                            }
                            for belt in &sys.belts {
                                ui.label(egui::RichText::new(format!("   ⋯ {} ({:.1}–{:.1} AU)", belt.name, belt.inner / AU, belt.outer / AU)).size(11.5).color(MUTED));
                            }
                        });
                }
            }
            LeftTab::Chronicle => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Show").color(MUTED));
                    ui.selectable_value(&mut ui_state.chronicle_min_importance, 5, "milestones");
                    ui.selectable_value(&mut ui_state.chronicle_min_importance, 4, "major");
                    ui.selectable_value(&mut ui_state.chronicle_min_importance, 2, "all");
                });
                let min = ui_state.chronicle_min_importance;
                for e in u.history.events.iter().rev().filter(|e| e.importance >= min).take(400) {
                    let color = match e.category {
                        Category::Life => LIFE,
                        Category::Technology | Category::Civilization | Category::Space | Category::Contact => CIV,
                        Category::Disaster | Category::War => DANGER,
                        Category::Astronomy => TEXT,
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format_date(e.time, u.start_time, u.gregorian())).size(11.0).color(MUTED));
                        let r = ui.add(egui::Label::new(egui::RichText::new(&e.title).color(color)).sense(egui::Sense::click()));
                        if r.on_hover_text(&e.detail).clicked() {
                            if let (Some(s), Some(b)) = (e.system, e.body) {
                                action = Some(Action::Focus(Target::Body(BodyRef { system: s, body: b })));
                            }
                        }
                    });
                }
            }
            LeftTab::Civilizations => {
                if u.civs.is_empty() {
                    ui.label(egui::RichText::new("No intelligent species has emerged yet. That may never happen — or it may happen next million years.").color(MUTED));
                }
                let graph = TechGraph::embedded();
                for c in u.civs.iter().rev() {
                    let r = BodyRef { system: c.system, body: c.body };
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let title = egui::RichText::new(&c.name).strong().color(if c.is_alive() { CIV } else { MUTED });
                        if ui.add(egui::Label::new(title).sense(egui::Sense::click())).clicked() {
                            action = Some(Action::Focus(Target::Body(r)));
                        }
                        ui.label(egui::RichText::new(format!("{} · {} · {}", u.body(r).name, c.era(graph), status_label(c))).size(11.5).color(MUTED));
                        ui.label(egui::RichText::new(format!("{} people · {} technologies", compact(c.population), c.discoveries.len())).size(11.5));
                    });
                }
            }
        });
    });
    match action {
        Some(Action::Focus(t)) => {
            sim.selected = Some(t);
            rig.focus_on(t, &sim, None);
        }
        None => {}
    }
    Ok(())
}

fn body_row(ui: &mut egui::Ui, u: &Universe, sim: &Sim, r: BodyRef, indent: usize, action: &mut Option<Action>) {
    let b = u.body(r);
    let glyph = match b.kind {
        BodyKind::GasGiant | BodyKind::IceGiant => "○",
        _ => "○",
    };
    let mut text = format!("{}{} {}", "    ".repeat(indent + 1), glyph, b.name);
    if let Some(c) = u.civ_on(r) {
        text.push_str(&format!("  — {}", c.species.name));
    }
    let mut rt = egui::RichText::new(text);
    if let Some((_, color)) = life_glyph(u, r) {
        rt = rt.color(color);
    }
    let resp = ui.selectable_label(sim.selected == Some(Target::Body(r)), rt);
    if resp.clicked() {
        *action = Some(Action::Focus(Target::Body(r)));
    }
}

fn status_label(c: &Civilization) -> String {
    match &c.status {
        CivStatus::Thriving => "thriving".into(),
        CivStatus::Collapsed { .. } => "in a dark age".into(),
        CivStatus::Extinct { .. } => "extinct".into(),
    }
}

pub fn right_panel(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, settings: Res<crate::persistence::UserSettings>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden || !ui_state.show_right {
        return Ok(());
    }
    let Some(sel) = sim.selected else { return Ok(()) };
    let mut fly = false;
    let mut edits: Vec<Edit> = Vec::new();
    let editable = !sim.is_reference();
    let now = time.elapsed_secs_f64();
    egui::SidePanel::right("inspector").default_width(390.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(12)).show(ctx, |ui| {
        let u = &sim.universe;
        let ctx = Ctx { editable, settings: &settings, edits: &mut edits };
        let mut ctx = ctx;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match sel {
            Target::Star(s) => {
                ui.horizontal(|ui| {
                    ui.heading(&u.system(s).star.name);
                    if rig.focus != Some(sel) && ui.button("Fly to").clicked() {
                        fly = true;
                    }
                });
                star_card(ui, u, s, &mut ctx);
            }
            Target::Body(r) => {
                let Some(b) = u.systems.get(r.system as usize).and_then(|x| x.bodies.get(r.body as usize)) else { return };
                ui.horizontal(|ui| {
                    ui.heading(&b.name);
                    if rig.focus != Some(sel) && b.exists() && ui.button("Fly to").clicked() {
                        fly = true;
                    }
                });
                let sub = format!("{} in the {} system", b.class().label(), u.system(r.system).name);
                ui.label(egui::RichText::new(sub).color(MUTED).size(12.0));
                ui.label(egui::RichText::new(b.source_label()).color(MUTED).size(10.5)).on_hover_text("Where this object's values come from. See the DATA tab.");
                ui.label(egui::RichText::new(format!("Appearance: {}", crate::render::look::Look::of(b).label)).color(MUTED).size(10.5)).on_hover_text("How this world is drawn, chosen from its physical state (or spacecraft imagery for real worlds). Visual only.");
                if let Some(rm) = &b.removed {
                    let how = match rm.cause {
                        RemovalCause::Deleted => "Deleted".to_string(),
                        RemovalCause::MergedInto(Some(i)) => format!("Collided with {}", u.system(r.system).bodies[i as usize].name),
                        RemovalCause::MergedInto(None) => format!("Fell into {}", u.system(r.system).star.name),
                    };
                    ui.colored_label(DANGER, format!("{how} — {}", format_date(rm.time, u.start_time, u.gregorian())));
                    return;
                }
                for w in warnings(u, r) {
                    ui.colored_label(DANGER, format!("⚠ {w}"));
                }
                ui.add_space(4.0);
                let civ = u.civ_on(r).is_some() || u.civs.iter().any(|c| c.system == r.system && c.body == r.body);
                ui.horizontal_wrapped(|ui| {
                    for (tab, label) in [(BodyTab::Overview, "Overview"), (BodyTab::Orbit, "Orbit"), (BodyTab::Physics, "Physics"), (BodyTab::Environment, "Environment"), (BodyTab::Life, "Life")] {
                        ui.selectable_value(&mut ui_state.body_tab, tab, label);
                    }
                    if civ {
                        ui.selectable_value(&mut ui_state.body_tab, BodyTab::Civilization, egui::RichText::new("Civilization").color(CIV));
                    }
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::History, "History");
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::Data, "Data");
                });
                ui.separator();
                let tab = if ui_state.body_tab == BodyTab::Civilization && !civ { BodyTab::Overview } else { ui_state.body_tab };
                match tab {
                    BodyTab::Overview => overview(ui, u, r, &mut ctx),
                    BodyTab::Orbit => orbit(ui, u, r, &mut ctx, &mut ui_state.impulse),
                    BodyTab::Physics => physics_tab(ui, u, r),
                    BodyTab::Environment => environment(ui, u, r, &mut ctx),
                    BodyTab::Life => life(ui, u, r),
                    BodyTab::Civilization => {
                        if let Some(c) = u.civs.iter().rev().find(|c| c.system == r.system && c.body == r.body) {
                            civilization(ui, u, c, &mut ui_state, &mut ctx);
                        }
                    }
                    BodyTab::History => history_tab(ui, u, r),
                    BodyTab::Data => data_tab(ui, u, r),
                }
            }
        });
    });
    for e in edits {
        let _ = sim.edit(e, now);
    }
    if fly {
        rig.focus_on(sel, &sim, None);
    }
    Ok(())
}

/// Editing context for inspector sections.
struct Ctx<'a> {
    editable: bool,
    settings: &'a crate::persistence::UserSettings,
    edits: &'a mut Vec<Edit>,
}

/// A labelled quantity row: editable in sandboxes, with unit choice and provenance.
#[allow(clippy::too_many_arguments)]
fn row(ui: &mut egui::Ui, ctx: &mut Ctx, label: &str, salt: &str, si: f64, q: Quantity, unit: usize, quality: Option<Quality>, editable: bool) -> Option<f64> {
    ui.label(egui::RichText::new(label).color(MUTED));
    let out = units::edit(ui, salt, si, q, unit, editable && ctx.editable, quality);
    ui.end_row();
    out
}

fn star_card(ui: &mut egui::Ui, u: &Universe, s: u32, ctx: &mut Ctx) {
    let sys = u.system(s);
    let st = &sys.star;
    let t = u.time;
    ui.label(egui::RichText::new(format!("{} star · {}", st.spectral_type(t), st.phase(t).label())).color(MUTED));
    let measured = sys.bodies.iter().any(|b| b.real);
    let q = |user: bool| Some(if user { Quality::UserModified } else if measured { Quality::Measured } else { Quality::Procedural });
    let derived = Some(Quality::Derived);
    egui::Grid::new("star").num_columns(2).striped(true).show(ui, |ui| {
        let user = u.edits.iter().any(|e| matches!(&e.edit, Edit::SetStar { system, .. } if *system == s));
        if let Some(v) = row(ui, ctx, "Mass", "star_mass", st.mass * SOLAR_MASS, Quantity::Mass, 3, q(user), true) {
            ctx.edits.push(Edit::SetStar { system: s, property: StarProperty::Mass(v / SOLAR_MASS) });
        }
        if let Some(v) = row(ui, ctx, "Age", "star_age", st.age(t), Quantity::Duration, 3, q(user), true) {
            ctx.edits.push(Edit::SetStar { system: s, property: StarProperty::Age(v) });
        }
        ui.label(egui::RichText::new("Metallicity [Fe/H]").color(MUTED));
        ui.horizontal(|ui| {
            let mut z = st.metallicity;
            if ctx.editable {
                let r = ui.add(egui::DragValue::new(&mut z).speed(0.01).range(-3.0..=1.0).suffix(" dex"));
                if r.drag_stopped() || r.lost_focus() {
                    ctx.edits.push(Edit::SetStar { system: s, property: StarProperty::Metallicity(z) });
                }
            } else {
                ui.label(format!("{z:+.2} dex"));
            }
        });
        ui.end_row();
        row(ui, ctx, "Radius", "star_r", st.current_radius(t), Quantity::Length, 4, derived, false);
        row(ui, ctx, "Luminosity (L☉)", "star_l", st.luminosity(t), Quantity::Plain, 0, derived, false);
        row(ui, ctx, "Temperature", "star_t", st.temperature_at(t), Quantity::Temperature, 0, derived, false);
        kv(ui, "Main-sequence life", format_duration(st.lifetime));
        kv(ui, "Flare activity", format!("{:.2}", st.flare_activity_at(t)));
        let (hi, ho) = st.habitable_zone_au(t);
        kv(ui, "Habitable zone", format!("{hi:.2} – {ho:.2} AU"));
        kv(ui, "Distance from origin", format!("{:.2} ly", sys.position.length() / LIGHT_YEAR));
        if let Some(c) = &sys.companion {
            kv(ui, "Companion", format!("{} ({}, {:.0} AU)", c.star.name, c.star.spectral_type(t), c.orbit.a / AU));
        }
        kv(ui, "Worlds", format!("{} planets, {} moons", sys.planets().count(), sys.existing().count() - sys.planets().count()));
    });
    ui.label(egui::RichText::new("Stellar model: mass sets luminosity, radius, temperature and lifetime (main-sequence relations with brightening). Changing the mass re-derives them; every planet's climate responds.").size(10.5).color(MUTED));
}

fn overview(ui: &mut egui::Ui, u: &Universe, r: BodyRef, ctx: &mut Ctx) {
    let b = u.body(r);
    let hab = u.habitability(r);
    let mu = ctx.settings.mass_unit;
    egui::Grid::new("ov").num_columns(2).striped(true).show(ui, |ui| {
        if let Some(v) = row(ui, ctx, "Mass", "mass", b.mass, Quantity::Mass, mu, Some(b.quality("mass")), true) {
            ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::Mass(v) });
        }
        if let Some(v) = row(ui, ctx, "Radius", "radius", b.radius, Quantity::Length, 1, Some(b.quality("radius")), true) {
            ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::Radius(v) });
        }
        // Surface gravity can be set: radius follows (mass held).
        if let Some(g) = row(ui, ctx, "Surface gravity (m/s²)", "gravity", b.gravity(), Quantity::Plain, 0, Some(Quality::Derived), true) {
            if g > 0.0 {
                ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::Radius((b.mu() / g).sqrt()) });
            }
        }
        row(ui, ctx, "Density (kg/m³)", "density", b.density(), Quantity::Plain, 0, Some(Quality::Derived), false);
        row(ui, ctx, "Escape velocity", "vesc", b.escape_velocity(), Quantity::Speed, ctx.settings.speed_unit, Some(Quality::Derived), false);
        if b.tidally_locked {
            kv(ui, "Rotation", "tidally locked");
        } else if let Some(v) = row(ui, ctx, "Day length", "rot", b.rotation_period.abs(), Quantity::Duration, 1, Some(b.quality("rotation_period")), true) {
            ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::RotationPeriod(v.max(1.0) * b.rotation_period.signum()) });
        }
        if let Some(v) = row(ui, ctx, "Axial tilt", "tilt", b.axial_tilt, Quantity::Angle, 0, Some(b.quality("axial_tilt")), true) {
            ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::AxialTilt(v) });
        }
        row(ui, ctx, "Mean temperature", "temp", b.temperature, Quantity::Temperature, ctx.settings.temperature_unit, Some(Quality::Derived), false);
        kv(ui, "Magnetic field", format!("{:.2}× Earth", b.magnetic_field));
        kv(ui, "Geological activity", format!("{:.2}× Earth", b.geology));
        kv(ui, "Habitability", egui::RichText::new(format!("{} ({:.2})", hab.label(), hab.score)).color(if hab.score > 0.0 { LIFE } else { MUTED }));
        if b.kind.has_surface() {
            kv(ui, "Δv to orbit", format!("{:.1} km/s", b.launch_delta_v_kms()));
        }
    });
    if ctx.editable {
        ui.horizontal(|ui| {
            if ui.button("Delete object").on_hover_text("Undoable (Cmd/Ctrl+Z)").clicked() {
                ctx.edits.push(Edit::RemoveBody { body: r });
            }
        });
    }
    if let Some(c) = u.civ_on(r) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("Home of {} ({} people)", c.name, compact(c.population))).color(CIV));
    }
    for c in u.civs.iter().filter(|c| c.is_alive() && c.colonies.iter().any(|col| col.body == r.body) && c.system == r.system) {
        ui.label(egui::RichText::new(format!("Colony of {}", c.name)).color(CIV));
    }
}

fn orbit(ui: &mut egui::Ui, u: &Universe, r: BodyRef, ctx: &mut Ctx, impulse: &mut [f64; 3]) {
    let sys = u.system(r.system);
    let i = r.body as usize;
    let b = &sys.bodies[i];
    let parent = b.parent.map(|p| sys.bodies[p as usize].name.clone()).unwrap_or_else(|| sys.star.name.clone());
    let el = sys.osculating(i, u.time);
    let mu = sys.parent_mu(i) + b.mu();
    let s = sys.body_state(i, u.time);
    let p = sys.parent_state(i, u.time);
    let dist = (s.pos - p.pos).length();
    let speed = (s.vel - p.vel).length();
    let dynamic = sys.is_dynamic();
    let du = ctx.settings.distance_unit;
    let mut set_orbit: Option<(f64, f64, f64, f64, f64, f64)> = None;
    let base = (el.semi_major_axis, el.eccentricity, el.inclination, el.longitude_ascending, el.argument_perihelion, el.mean_anomaly);
    egui::Grid::new("orb").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Orbits", parent);
        if el.is_bound() {
            let q = Some(b.quality("orbit"));
            if let Some(v) = row(ui, ctx, "Semi-major axis", "a", el.semi_major_axis, Quantity::Length, if b.parent.is_some() { 1 } else { 5 }, q, true) {
                set_orbit = Some((v, base.1, base.2, base.3, base.4, base.5));
            }
            ui.label(egui::RichText::new("Eccentricity").color(MUTED));
            let mut e = el.eccentricity;
            if ctx.editable {
                let resp = ui.add(egui::DragValue::new(&mut e).speed(0.002).range(0.0..=0.999).max_decimals(5));
                if resp.drag_stopped() || resp.lost_focus() {
                    set_orbit = Some((base.0, e, base.2, base.3, base.4, base.5));
                }
            } else {
                ui.label(format!("{e:.5}"));
            }
            ui.end_row();
            if let Some(v) = row(ui, ctx, "Inclination", "inc", el.inclination, Quantity::Angle, 0, q, true) {
                set_orbit = Some((base.0, base.1, v, base.3, base.4, base.5));
            }
            if ctx.settings.advanced {
                if let Some(v) = row(ui, ctx, "Ascending node", "node", el.longitude_ascending, Quantity::Angle, 0, q, true) {
                    set_orbit = Some((base.0, base.1, base.2, v, base.4, base.5));
                }
                if let Some(v) = row(ui, ctx, "Arg. of periapsis", "peri", el.argument_perihelion, Quantity::Angle, 0, q, true) {
                    set_orbit = Some((base.0, base.1, base.2, base.3, v, base.5));
                }
                if let Some(v) = row(ui, ctx, "Mean anomaly", "ma", el.mean_anomaly, Quantity::Angle, 0, q, true) {
                    set_orbit = Some((base.0, base.1, base.2, base.3, base.4, v));
                }
            }
            kv(ui, "Periapsis / apoapsis", format!("{} / {}", super::distance(el.periapsis()), super::distance(el.apoapsis())));
            kv(ui, "Period", format_duration(std::f64::consts::TAU * (el.semi_major_axis.powi(3) / mu).sqrt()));
        } else {
            kv(ui, "Path", egui::RichText::new(format!("unbound (e = {:.3}) — escaping", el.eccentricity)).color(DANGER));
        }
        row(ui, ctx, "Current distance", "dist", dist, Quantity::Length, du, Some(Quality::Derived), false);
        row(ui, ctx, "Speed", "speed", speed, Quantity::Speed, ctx.settings.speed_unit, Some(Quality::Derived), false);
    });
    if let Some((a, e, inc, node, peri, ma)) = set_orbit {
        ctx.edits.push(Edit::SetOrbit { body: r, a, e, i: inc, node, peri, mean_anomaly: ma });
    }
    if ctx.editable {
        heading(ui, "Push (velocity change)");
        ui.horizontal(|ui| {
            for (k, axis) in ["prograde", "radial out", "normal"].iter().enumerate() {
                ui.label(egui::RichText::new(*axis).size(10.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut impulse[k]).speed(10.0).suffix(" m/s"));
            }
        });
        if ui.button("Apply push").on_hover_text("Δv along the direction of motion, away from the parent, and perpendicular to the orbit").clicked() {
            let rel_v = s.vel - p.vel;
            let rel_r = s.pos - p.pos;
            let pro = rel_v.normalize();
            let normal = rel_r.cross(rel_v).normalize();
            let radial = pro.cross(normal);
            let dv = pro * impulse[0] - radial * impulse[1] + normal * impulse[2];
            ctx.edits.push(Edit::Impulse { body: r, dv });
            *impulse = [0.0; 3];
        }
        if ctx.settings.advanced {
            heading(ui, "State vectors (system frame)");
            ui.label(egui::RichText::new(format!("r = ({:.6e}, {:.6e}, {:.6e}) m", s.pos.x, s.pos.y, s.pos.z)).size(11.0).monospace());
            ui.label(egui::RichText::new(format!("v = ({:.4}, {:.4}, {:.4}) m/s", s.vel.x, s.vel.y, s.vel.z)).size(11.0).monospace());
        }
    }
    ui.label(egui::RichText::new(if dynamic { "Osculating elements of the N-body state: the ellipse the body would follow if all other forces vanished now." } else { "Analytic Keplerian propagation: exact at any time scale." }).size(10.5).color(MUTED));
}

fn physics_tab(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let sys = u.system(r.system);
    let i = r.body as usize;
    let b = &sys.bodies[i];
    heading(ui, "Simulation model");
    let (model, tier) = match &sys.dynamics {
        None => ("Analytic Kepler orbit", "Tier 1"),
        Some(d) if d.bodies.get(i).copied().flatten().is_some() => ("Newtonian N-body particle (4th-order symplectic)", "Tier 2 · tier 3 during encounters"),
        Some(_) => ("Kepler 'rails' around its N-body parent (massless test body)", "Tier 1 — becomes a particle if edited or approached"),
    };
    egui::Grid::new("phys").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Motion", model);
        kv(ui, "Fidelity", tier);
        kv(ui, "Shape", "sphere, uniform density (no oblateness)");
        kv(ui, "Tides", "not simulated yet (milestone S3)");
        let star_mass = sys.star.mass * SOLAR_MASS;
        let (pm, pr, pd) = match b.parent {
            Some(p) => (sys.bodies[p as usize].mass, sys.bodies[p as usize].radius, sys.bodies[p as usize].density()),
            None => (star_mass, sys.star.current_radius(u.time), star_mass / (4.0 / 3.0 * std::f64::consts::PI * sys.star.current_radius(u.time).powi(3))),
        };
        let el = sys.osculating(i, u.time);
        if el.is_bound() {
            kv(ui, "Hill sphere", super::distance(hill_radius(el.semi_major_axis, b.mass, pm)));
        }
        kv(ui, "Roche limit of parent (for this body)", super::distance(roche_limit(pr, pd, b.density())));
        let _ = pm;
        if let Some(d) = &sys.dynamics {
            kv(ui, "System step", format_duration(d.dt));
            kv(ui, "Energy error (system)", format!("{:.2e}", d.diagnostics.energy_error));
            kv(ui, "Preset", d.settings.preset.label());
        }
    });
    if !b.impacts.is_empty() {
        heading(ui, &format!("Impacts received ({})", b.impacts.len()));
        for im in b.impacts.iter().rev().take(10) {
            ui.label(egui::RichText::new(format!("{} — {} · {:.3e} Mt · {:.1} km/s · crater {:.1} km · {}", format_date(im.time, u.start_time, u.gregorian()), im.impactor, im.energy_mt(), im.speed / 1000.0, im.crater_m / 1000.0, im.class.label())).size(11.0));
        }
    }
}

fn history_tab(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    heading(ui, "What happened here");
    let mut any = false;
    for e in u.history.events.iter().rev().filter(|e| e.system == Some(r.system) && e.body == Some(r.body)).take(120) {
        any = true;
        let color = match e.category {
            Category::Disaster | Category::War => DANGER,
            Category::Life => LIFE,
            Category::Civilization | Category::Technology => CIV,
            _ => TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {}", format_date(e.time, u.start_time, u.gregorian()), e.title)).size(11.5).color(color)).on_hover_text(&e.detail);
    }
    if !any {
        ui.label(egui::RichText::new("Nothing recorded yet.").color(MUTED));
    }
}

fn data_tab(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let b = u.body(r);
    heading(ui, "Source");
    ui.label(b.source_label());
    heading(ui, "Data quality");
    egui::Grid::new("dq").num_columns(2).striped(true).show(ui, |ui| {
        for (label, f) in [
            ("Mass", "mass"),
            ("Radius", "radius"),
            ("Position & velocity", "position"),
            ("Orbit", "orbit"),
            ("Rotation", "rotation_period"),
            ("Axial tilt", "axial_tilt"),
            ("Albedo", "albedo"),
            ("Atmosphere", "atmosphere"),
            ("Water", "water"),
            ("Temperature", "temperature"),
            ("Habitability", "habitability"),
            ("Surface relief", "terrain"),
            ("Resources", "resources"),
        ] {
            ui.label(egui::RichText::new(label).color(MUTED));
            let q = if f == "terrain" && b.elevation_data.is_some() { Quality::Measured } else { b.quality(f) };
            units::quality_badge(ui, q);
            ui.end_row();
        }
    });
    ui.label(egui::RichText::new("MEASURED: observational dataset · DERIVED: computed by a documented model · ESTIMATED: model estimate without measurement · PROCEDURAL: generated from a seed · USER MODIFIED: changed in this sandbox.").size(10.5).color(MUTED));
    let journal: Vec<_> = u.edits.iter().filter(|e| e.edit.system() == r.system && edit_touches(&e.edit, r)).collect();
    if !journal.is_empty() {
        heading(ui, "Your changes");
        for e in journal.iter().rev().take(30) {
            ui.label(egui::RichText::new(format!("{} — {}", format_date(e.time, u.start_time, u.gregorian()), e.summary)).size(11.0));
        }
    }
}

fn edit_touches(e: &Edit, r: BodyRef) -> bool {
    match e {
        Edit::RemoveBody { body } | Edit::SetState { body, .. } | Edit::Impulse { body, .. } | Edit::SetOrbit { body, .. } | Edit::SetProperty { body, .. } => *body == r,
        Edit::AddBody { .. } => false,
        _ => false,
    }
}

fn environment(ui: &mut egui::Ui, u: &Universe, r: BodyRef, ctx: &mut Ctx) {
    let b = u.body(r);
    heading(ui, "Atmosphere");
    ui.label(b.atmosphere.describe());
    let a = &b.atmosphere;
    if b.kind.has_surface() {
        egui::Grid::new("atm_edit").num_columns(2).show(ui, |ui| {
            if let Some(v) = row(ui, ctx, "Surface pressure", "press", a.pressure_bar, Quantity::Pressure, 0, Some(b.quality("atmosphere")), true) {
                ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::SurfacePressure(v) });
            }
            if let Some(v) = row(ui, ctx, "CO₂", "co2", a.co2, Quantity::Fraction, 0, Some(b.quality("atmosphere")), true) {
                ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::Co2Fraction(v.clamp(0.0, 1.0)) });
            }
            if let Some(v) = row(ui, ctx, "Water (Earth oceans)", "water", b.hydro.water_inventory, Quantity::Plain, 0, Some(b.quality("water")), true) {
                ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::WaterInventory(v.max(0.0)) });
            }
            if let Some(v) = row(ui, ctx, "Bond albedo", "albedo", b.albedo, Quantity::Plain, 0, Some(b.quality("albedo")), true) {
                ctx.edits.push(Edit::SetProperty { body: r, property: BodyProperty::Albedo(v.clamp(0.0, 1.0)) });
            }
        });
    }
    if a.is_present() {
        for (name, v) in [("N₂", a.n2), ("O₂", a.o2), ("CO₂", a.co2), ("H₂O", a.h2o), ("CH₄", a.ch4), ("H₂ / He", a.h2he)] {
            if v > 1e-5 {
                bar(ui, &format!("{name}  {:.3}%", v * 100.0), v.max(0.01), egui::Color32::from_rgb(110, 160, 230), "");
            }
        }
    }
    heading(ui, "Water & climate");
    egui::Grid::new("env").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Ocean cover", format!("{:.0}%", b.hydro.ocean_fraction * 100.0));
        kv(ui, "Ice cover", format!("{:.0}%", b.hydro.ice_fraction * 100.0));
        kv(ui, "Subsurface ocean", if b.hydro.subsurface_ocean { "yes" } else { "no" });
        kv(ui, "Equilibrium temp.", format!("{:.0} K", b.equilibrium_temperature));
        kv(ui, "Surface temp.", format!("{:.1} K ({:.1} °C)", b.temperature, b.temperature - 273.15));
        let sys = u.system(r.system);
        let flux = cosmogon_sim::sandbox::flux_of(sys, r.body as usize, u.time);
        kv(ui, "Insolation (annual mean)", format!("{flux:.3} × Earth's"));
        if let Some(w) = &b.impact_winter {
            kv(ui, "Impact winter", egui::RichText::new(format!("−{:.1} K now", w.cooling_at(u.time))).color(DANGER));
        }
        if b.kind.has_surface() {
            kv(ui, "Climate", if b.climate_stability(u.time) > 0.6 { "stable (interglacial)" } else { "glacial swings" });
        }
    });
    ui.label(egui::RichText::new("Climate model: zero-dimensional grey greenhouse, driven by the current orbit's annual-mean sunlight. Changes to orbit, star or atmosphere propagate to temperature, water, habitability and civilizations.").size(10.5).color(MUTED));
    if b.kind.has_surface() {
        heading(ui, "Resources (Earth = 1)");
        let res = &b.resources;
        for k in ResourceKind::ALL {
            let v = res.get(k);
            bar(ui, &format!("{}  {:.2}", k.label(), v), (v / 2.0).min(1.0), egui::Color32::from_rgb(200, 160, 90), "");
        }
        bar(ui, &format!("Fertile land  {:.2}", res.fertile_land), (res.fertile_land / 2.0).min(1.0), LIFE, "");
        bar(ui, &format!("Fresh water  {:.2}", res.fresh_water), (res.fresh_water / 2.0).min(1.0), egui::Color32::from_rgb(90, 160, 230), "");
        ui.label(egui::RichText::new(format!("{} mapped deposits. Coal and oil are buried biomass: they accumulate only while land and marine life flourish.", b.deposits.len())).size(11.0).color(MUTED));
    }
}

fn life(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let hab = u.habitability(r);
    heading(ui, "Habitability");
    ui.label(egui::RichText::new(format!("{} — {:.2}", hab.label(), hab.score)).color(if hab.score > 0.0 { LIFE } else { MUTED }));
    ui.label(egui::RichText::new(&hab.environment).color(MUTED));
    for f in &hab.factors {
        let c = if f.value > 0.66 { LIFE } else if f.value > 0.33 { ACCENT } else { DANGER };
        bar(ui, &f.name, f.value, c, &f.note);
    }
    if !hab.factors.is_empty() {
        ui.label(egui::RichText::new(format!("Most complex life this environment can support: {}", hab.max_stage.label().to_lowercase())).size(11.0).color(MUTED));
    }
    if let Some(bio) = u.biosphere(r) {
        heading(ui, "Biosphere");
        egui::Grid::new("bio").num_columns(2).striped(true).show(ui, |ui| {
            kv(ui, "Stage", egui::RichText::new(bio.stage.label()).color(if bio.stage > Stage::Prebiotic { LIFE } else { TEXT }));
            kv(ui, "Since", format_date(bio.stage_since, u.start_time, u.gregorian()));
            kv(ui, "Biomass", format!("{:.0}% of a mature biosphere", bio.biomass * 100.0));
            kv(ui, "Biodiversity", format!("{:.0}%", bio.biodiversity * 100.0));
            kv(ui, "Free oxygen", format!("{:.1}%", u.body(r).atmosphere.o2 * 100.0));
            kv(ui, "Photosynthesis", bio.photosynthesis_since.map(|t| format!("for {}", format_duration(u.time - t))).unwrap_or_else(|| "not evolved".into()));
            kv(ui, "Mass extinctions", bio.extinctions.to_string());
        });
        let stages = [Stage::Prebiotic, Stage::Microbial, Stage::ComplexCells, Stage::Multicellular, Stage::ComplexEcosystems, Stage::Intelligent];
        ui.horizontal_wrapped(|ui| {
            for s in stages {
                let reached = bio.stage >= s;
                ui.label(egui::RichText::new(s.label()).size(10.5).color(if reached { LIFE } else { MUTED }));
                if s != Stage::Intelligent {
                    ui.label(egui::RichText::new("›").color(MUTED));
                }
            }
        });
    }
    let events: Vec<_> = u.history.events.iter().filter(|e| e.system == Some(r.system) && e.body == Some(r.body) && e.category == Category::Life).collect();
    if !events.is_empty() {
        heading(ui, "Natural history");
        for e in events.iter().rev().take(30) {
            ui.label(egui::RichText::new(format!("{} — {}", format_date(e.time, u.start_time, u.gregorian()), e.title)).size(11.5));
        }
    }
}

/// Eras in the order the technology graph introduces them.
fn eras(graph: &TechGraph) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for t in &graph.techs {
        if !v.contains(&t.era) {
            v.push(t.era.clone());
        }
    }
    v
}

/// The civilization at a glance: era and progress, energy (Kardashev), people, reach into
/// space, and what is likely to come next.
fn glance(ui: &mut egui::Ui, u: &Universe, c: &Civilization) {
    let graph = TechGraph::embedded();
    let era = c.era(graph);
    let all = eras(graph);
    let idx = all.iter().position(|e| *e == era).unwrap_or(0);
    let in_era = graph.techs.iter().filter(|t| t.era == era).count().max(1);
    let known_in_era = graph.techs.iter().filter(|t| t.era == era && c.knows(&t.id)).count();
    egui::Frame::new().fill(egui::Color32::from_rgb(22, 19, 12)).corner_radius(8).inner_margin(10).stroke(egui::Stroke::new(1.0_f32, CIV.gamma_multiply(0.4))).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(era.to_uppercase()).size(15.0).strong().color(CIV).extra_letter_spacing(1.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("era {} of {}", idx + 1, all.len())).color(MUTED).size(11.0));
            });
        });
        // Progress through the eras: completed segments, then the current one partly filled.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 8.0), egui::Sense::hover());
        let seg = rect.width() / all.len() as f32;
        for k in 0..all.len() {
            let r = egui::Rect::from_min_size(rect.min + egui::vec2(seg * k as f32 + 1.0, 0.0), egui::vec2(seg - 2.0, rect.height()));
            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(45, 40, 30));
            let fill = if k < idx { 1.0 } else if k == idx { known_in_era as f32 / in_era as f32 } else { 0.0 };
            if fill > 0.0 {
                ui.painter().rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * fill, r.height())), 2.0, CIV);
            }
        }
        if let Some(next) = all.get(idx + 1) {
            ui.label(egui::RichText::new(format!("{known_in_era}/{in_era} {era} technologies · next era: {next}")).size(11.0).color(MUTED));
        }
        ui.add_space(4.0);
        egui::Grid::new("glance").num_columns(4).spacing([12.0, 2.0]).show(ui, |ui| {
            let big = |ui: &mut egui::Ui, v: String, l: &str| {
                ui.vertical(|ui| {
                    ui.add(egui::Label::new(egui::RichText::new(v).size(15.0).strong().color(TEXT)).wrap_mode(egui::TextWrapMode::Extend));
                    ui.add(egui::Label::new(egui::RichText::new(l).size(10.0).color(MUTED)).wrap_mode(egui::TextWrapMode::Extend));
                })
                .response
            };
            big(ui, compact(c.population), "people");
            big(ui, format!("{}/{}", c.discoveries.len(), graph.len()), "techs");
            big(ui, format!("{:.2}", c.kardashev()), "K-scale").on_hover_text("Energy use on Sagan's scale: K = (log₁₀ P[W] − 6)/10. Planetary mastery is 1.0; Earth today ≈ 0.73.");
            big(ui, power(c.total_power_w()), "power");
            ui.end_row();
        });
        ui.add_space(4.0);
        // The ladder into space.
        ui.horizontal_wrapped(|ui| {
            for (label, flag, tip) in [
                ("Orbit", "satellites", "Artificial satellites"),
                ("Crew", "crewed_orbit", "Crewed spaceflight"),
                ("Moon", "moon_landing", "Landing on a moon"),
                ("Stations", "stations", "Orbital stations and industry"),
                ("Colonies", "colonies", "Self-sustaining settlements on other worlds"),
                ("Stars", "probes", "Interstellar probes"),
            ] {
                let on = c.flags.contains(flag);
                let text = if on { egui::RichText::new(label).size(11.5).strong().color(CIV) } else { egui::RichText::new(label).size(11.5).color(MUTED.gamma_multiply(0.6)) };
                ui.label(text).on_hover_text(tip);
            }
        });
        // Most likely next breakthroughs, with expected waiting times from the discovery rate.
        let body = u.body(BodyRef { system: c.system, body: c.body });
        let sys = u.system(c.system);
        let moons = sys.moons_of(c.body as usize).count();
        let others = sys.bodies.iter().enumerate().filter(|(i, x)| *i != c.body as usize && x.kind.has_surface() && x.mass > 1e21).count();
        let env = environment_for(body, moons, others, u.time);
        let mut known = vec![false; graph.len()];
        for d in &c.discoveries {
            if let Some(i) = graph.find(&d.tech) {
                known[i] = true;
            }
        }
        let has = |f: &str| c.flags.contains(f);
        let tctx = cosmogon_sim::civ::tech::Context { known: &known, knowledge: &c.knowledge, resources: &body.resources, env: &env, habitat: c.species.habitat, population: c.population, flags: &has };
        let mut next: Vec<(f64, String)> = (0..graph.len())
            .filter_map(|i| {
                let (_, speed, surplus) = graph.available(i, &tctx)?;
                let t = &graph.techs[i];
                let demand = t.demand.as_deref().map(|d| c.pressures.get(d)).unwrap_or(0.0);
                let rate = speed * surplus.powf(1.5) * (1.0 + 2.0 * demand) * u.settings.tech_rate / t.years;
                Some((1.0 / rate.max(1e-12), t.name.clone()))
            })
            .collect();
        next.sort_by(|a, b| a.0.total_cmp(&b.0));
        if next.is_empty() {
            // The nearest goal: prerequisite technologies known, only knowledge short.
            let closest = (0..graph.len())
                .filter(|&i| !known[i])
                .filter_map(|i| {
                    let t = &graph.techs[i];
                    if !t.requires.iter().all(|cond| matches!(cond, cosmogon_sim::civ::tech::Condition::Knowledge(..)) || graph.check(cond, &tctx)) {
                        return None;
                    }
                    let gaps: Vec<(f64, String)> = t
                        .requires
                        .iter()
                        .filter_map(|cond| match cond {
                            cosmogon_sim::civ::tech::Condition::Knowledge(d, n) if c.knowledge[d.index()] < *n => Some((c.knowledge[d.index()] / n, d.name().to_string())),
                            _ => None,
                        })
                        .collect();
                    let worst = gaps.iter().map(|g| g.0).fold(1.0, f64::min);
                    Some((worst, t.name.clone(), gaps.into_iter().map(|g| format!("{} {:.0}%", g.1, g.0 * 100.0)).collect::<Vec<_>>().join(", ")))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            match closest {
                Some((_, name, gaps)) => ui.label(egui::RichText::new(format!("Closest goal: {name} — knowledge {gaps} of the way")).size(11.0).color(MUTED)),
                None => ui.label(egui::RichText::new("Next: nothing within reach yet — knowledge must grow").size(11.0).color(MUTED)),
            };
        } else {
            let items: Vec<String> = next.iter().take(3).map(|(y, n)| format!("{n} (~{} yr)", if *y < 10.0 { format!("{y:.0}") } else { compact(*y) })).collect();
            ui.label(egui::RichText::new(format!("Within reach: {}", items.join(" · "))).size(11.0).color(LIFE));
        }
    });
}

/// Satellites, missions in flight, the exploration record and colonies.
fn space_programme(ui: &mut egui::Ui, u: &Universe, c: &Civilization) {
    if c.satellites == 0 && c.explored.is_empty() && c.missions.is_empty() && c.colonies.is_empty() {
        return;
    }
    let sys = u.system(c.system);
    let name = |b: u32| sys.bodies.get(b as usize).map(|x| x.name.clone()).unwrap_or_default();
    heading(ui, "Space programme");
    egui::Grid::new("space").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Satellites", group_digits(c.satellites as f64));
        kv(ui, "Missions launched", c.missions_launched.to_string());
        for col in &c.colonies {
            kv(ui, &format!("Colony · {}", name(col.body)), format!("{} people, since {}", compact(col.population), format_date(col.founded, u.start_time, u.gregorian())));
        }
    });
    if !c.missions.is_empty() {
        ui.label(egui::RichText::new("In flight").size(11.5).color(MUTED));
        for m in &c.missions {
            let p = m.progress(u.time);
            bar(ui, &format!("{} → {}", m.name, name(m.body)), p, if m.kind == cosmogon_sim::civ::space::MissionKind::Colony { CIV } else { egui::Color32::from_rgb(140, 210, 255) }, &format!("{} · arrives {}", m.kind.label(), format_date(m.arrives, u.start_time, u.gregorian())));
        }
    }
    if !c.explored.is_empty() {
        ui.label(egui::RichText::new("Worlds visited").size(11.5).color(MUTED));
        let mut ex: Vec<_> = c.explored.iter().filter(|e| e.body != c.body).collect();
        ex.sort_by(|a, b| a.first.total_cmp(&b.first));
        for e in ex {
            ui.label(egui::RichText::new(format!("{} — {} · first reached {}", name(e.body), e.level.label(), format_date(e.first, u.start_time, u.gregorian()))).size(11.5));
        }
    }
}

/// Limited sandbox interventions.
fn interfere(ui: &mut egui::Ui, u: &Universe, ci: u32, c: &Civilization, ctx: &mut Ctx, ui_state: &mut UiState) {
    use cosmogon_sim::intervene::{cooldown_left, teachable, Intervention, COOLDOWN_YEARS};
    if !ctx.editable {
        return;
    }
    heading(ui, "Interfere");
    let wait = cooldown_left(c, u.time);
    ui.label(egui::RichText::new(format!("One nudge every {COOLDOWN_YEARS:.0} years. You can teach only what their own technology and world allow, and each act is recorded and undoable.")).size(11.0).color(MUTED));
    if wait > 0.0 {
        ui.label(egui::RichText::new(format!("Available again in {wait:.0} years")).color(ACCENT));
        return;
    }
    let mut act = |a: Intervention| ctx.edits.push(Edit::Intervene { system: c.system, civ: ci, action: a });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Inspire").on_hover_text("Stability +20%, war and famine pressures ease").clicked() {
            act(Intervention::Inspire);
        }
        if ui.button("Send a signal").on_hover_text("They detect an unexplained artificial signal: astronomy and space research surge").clicked() {
            act(Intervention::Signal);
        }
        if ui.button(egui::RichText::new("Hardship").color(DANGER)).on_hover_text("An epidemic kills 2–10%; medicine becomes urgent").clicked() {
            act(Intervention::Hardship);
        }
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("share_domain").selected_text(Domain::ALL[ui_state.share_domain as usize % Domain::ALL.len()].name()).show_ui(ui, |ui| {
            for (k, d) in Domain::ALL.iter().enumerate() {
                ui.selectable_value(&mut ui_state.share_domain, k as u8, d.name());
            }
        });
        if ui.button("Share knowledge").on_hover_text("+50% in that field (at most +50 million insight)").clicked() {
            act(Intervention::ShareKnowledge(ui_state.share_domain));
        }
    });
    let graph = TechGraph::embedded();
    let teach = teachable(u, c);
    if !teach.is_empty() {
        ui.label(egui::RichText::new("Teach a technology (prerequisites already known):").size(11.0).color(MUTED));
        ui.horizontal_wrapped(|ui| {
            for i in teach.into_iter().take(8) {
                let t = &graph.techs[i];
                if ui.small_button(&t.name).on_hover_text(&t.description).clicked() {
                    act(Intervention::Teach(t.id.clone()));
                }
            }
        });
    }
}

fn civilization(ui: &mut egui::Ui, u: &Universe, c: &Civilization, ui_state: &mut UiState, ctx: &mut Ctx) {
    let graph = TechGraph::embedded();
    ui.label(egui::RichText::new(&c.name).size(17.0).strong().color(CIV));
    ui.label(egui::RichText::new(format!("{} · {} · founded {}", c.era(graph), status_label(c), format_date(c.founded, u.start_time, u.gregorian()))).color(MUTED).size(12.0));
    ui.add_space(4.0);
    glance(ui, u, c);
    ui.add_space(6.0);
    let ci = u.civs.iter().position(|x| std::ptr::eq(x, c)).unwrap_or(0) as u32;
    egui::Grid::new("civ").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Population", format!("{} (capacity {})", compact(c.population), compact(c.capacity)));
        kv(ui, "Settlements", format!("{} ({}% urban)", c.sites.iter().filter(|s| s.active()).count(), (c.urbanisation * 100.0).round()));
        if let Some(big) = c.sites.iter().filter(|s| s.active()).max_by(|a, b| a.population.total_cmp(&b.population)) {
            kv(ui, "Largest settlement", format!("{} — {} ({})", big.name, big.tier().label(), compact(big.population)));
        }
        kv(ui, "Energy use", format!("{} ({:.0} W per person)", power(c.total_power_w()), c.energy_per_capita));
        kv(ui, "Fossil share", format!("{:.0}%{}", c.fossil_share * 100.0, if c.energy_shortfall > 0.05 { format!(" · shortfall {:.0}%", c.energy_shortfall * 100.0) } else { String::new() }));
        kv(ui, "Stability", format!("{:.0}%", c.stability * 100.0));
        kv(ui, "Health", format!("{:+.0}%", c.health * 100.0));
        if let Some(r) = c.radio_since {
            kv(ui, "Radio sphere", format!("{:.2} ly", (u.time - r) / SECONDS_PER_YEAR));
        }
        if !c.detected.is_empty() {
            kv(ui, "Has detected", c.detected.iter().map(|d| u.civs[*d as usize].name.clone()).collect::<Vec<_>>().join(", "));
        }
    });

    space_programme(ui, u, c);
    interfere(ui, u, ci, c, ctx, ui_state);

    let mut nations: Vec<_> = c.polities.iter().filter(|p| p.alive()).collect();
    if !nations.is_empty() {
        nations.sort_by(|a, b| b.population.total_cmp(&a.population));
        heading(ui, &format!("Nations ({})", nations.len()));
        for p in nations.iter().take(12) {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, egui::Color32::from_rgb(p.color[0], p.color[1], p.color[2]));
                ui.label(egui::RichText::new(p.title()).strong());
            });
            let capital = &c.sites[p.capital as usize].name;
            let mut line = format!("   {} people · {} settlements · capital {}", compact(p.population), p.sites, capital);
            if !p.at_war.is_empty() {
                let foes: Vec<String> = p.at_war.iter().filter_map(|f| c.polities.get(*f as usize)).map(|f| f.title()).collect();
                line.push_str(&format!(" · at war with {}", foes.join(", ")));
            }
            ui.label(egui::RichText::new(line).size(11.0).color(if p.at_war.is_empty() { MUTED } else { DANGER }));
        }
        if nations.len() > 12 {
            ui.label(egui::RichText::new(format!("…and {} smaller states", nations.len() - 12)).size(11.0).color(MUTED));
        }
        let fallen = c.polities.len() - nations.len();
        if fallen > 0 {
            ui.label(egui::RichText::new(format!("{fallen} states have risen and fallen.")).size(11.0).color(MUTED));
        }
    }

    heading(ui, "Species");
    ui.label(format!("The {} — {:.0} kg, live ~{:.0} years", c.species.name, c.species.mass_kg, c.species.lifespan_years));
    for t in &c.species.traits {
        ui.label(egui::RichText::new(format!("· {t}")).size(11.5).color(MUTED));
    }

    heading(ui, "Trends");
    let pts = &c.samples.points;
    line_chart(ui, "Population", &pts.iter().map(|p| (p.t, p.population)).collect::<Vec<_>>(), true, compact);
    line_chart(ui, "Total knowledge", &pts.iter().map(|p| (p.t, p.knowledge)).collect::<Vec<_>>(), true, compact);
    line_chart(ui, "Energy use", &pts.iter().map(|p| (p.t, p.energy_w.max(1.0))).collect::<Vec<_>>(), true, power);

    heading(ui, "Knowledge");
    let max = c.knowledge.iter().cloned().fold(1.0, f64::max).log10();
    for d in Domain::ALL {
        let k = c.knowledge[d.index()];
        bar(ui, d.name(), (k.max(1.0).log10() / max).clamp(0.0, 1.0), egui::Color32::from_rgb(150, 130, 230), &format!("{} insight", group_digits(k)));
    }

    heading(ui, "Pressures shaping research");
    let p = &c.pressures;
    for (name, v) in [("Food", p.food), ("Disease", p.disease), ("Energy", p.energy), ("War", p.war), ("Climate", p.climate), ("Contact", p.contact)] {
        if v > 0.02 {
            bar(ui, name, v, DANGER, "");
        }
    }

    heading(ui, "Technology");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.tech_filter_blocked, false, format!("Discovered ({})", c.discoveries.len()));
        ui.selectable_value(&mut ui_state.tech_filter_blocked, true, "Next & blocked");
    });
    if !ui_state.tech_filter_blocked {
        for d in c.discoveries.iter().rev() {
            let name = graph.find(&d.tech).map(|i| graph.techs[i].name.clone()).unwrap_or_else(|| d.tech.clone());
            egui::CollapsingHeader::new(format!("{} — {}", format_date(d.time, u.start_time, u.gregorian()), name)).id_salt(("tech", &d.tech)).show(ui, |ui| {
                if let Some(i) = graph.find(&d.tech) {
                    ui.label(egui::RichText::new(&graph.techs[i].description).size(11.5));
                }
                ui.label(egui::RichText::new(format!("Why: {}", d.drivers)).size(11.5).color(ACCENT));
            });
        }
    } else {
        // Explain the frontier: what is possible now, and what is blocked and why.
        let body = u.body(BodyRef { system: c.system, body: c.body });
        let sys = u.system(c.system);
        let moons = sys.moons_of(c.body as usize).count();
        let others = sys.bodies.iter().enumerate().filter(|(i, x)| *i != c.body as usize && x.kind.has_surface() && x.mass > 1e21).count();
        let env = environment_for(body, moons, others, u.time);
        let mut known = vec![false; graph.len()];
        for d in &c.discoveries {
            if let Some(i) = graph.find(&d.tech) {
                known[i] = true;
            }
        }
        let has = |f: &str| c.flags.contains(f);
        let ctx = cosmogon_sim::civ::tech::Context { known: &known, knowledge: &c.knowledge, resources: &body.resources, env: &env, habitat: c.species.habitat, population: c.population, flags: &has };
        let mut shown = 0;
        for i in 0..graph.len() {
            let t = &graph.techs[i];
            match graph.status(i, &ctx) {
                Status::Available { route, .. } => {
                    ui.label(egui::RichText::new(format!("+ {} — within reach{}", t.name, route.map(|r| format!(" ({})", t.routes[r].label)).unwrap_or_default())).color(LIFE).size(12.0));
                }
                Status::Blocked { missing } => {
                    // Only show the near frontier: techs whose prerequisite techs are all known.
                    let near = !missing.iter().any(|m| m.starts_with("knows "));
                    if near && shown < 14 {
                        shown += 1;
                        ui.label(egui::RichText::new(format!("- {}", t.name)).size(12.0));
                        ui.label(egui::RichText::new(format!("   needs {}", missing.join("; "))).size(11.0).color(MUTED));
                    }
                }
                Status::Known => {}
            }
        }
    }

    heading(ui, "History");
    for e in u.history.for_civ(c.id).collect::<Vec<_>>().iter().rev().filter(|e| e.importance >= 3).take(60) {
        let color = match e.category {
            Category::Disaster | Category::War => DANGER,
            Category::Technology => TEXT,
            _ => CIV,
        };
        ui.label(egui::RichText::new(format!("{} — {}", format_date(e.time, u.start_time, u.gregorian()), e.title)).size(11.5).color(color)).on_hover_text(&e.detail);
    }
}
