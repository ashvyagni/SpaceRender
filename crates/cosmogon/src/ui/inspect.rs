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
        BodyKind::GasGiant | BodyKind::IceGiant => "●",
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

pub fn right_panel(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>, sim: Res<Sim>, mut rig: ResMut<CameraRig>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.hidden || !ui_state.show_right {
        return Ok(());
    }
    let Some(sel) = sim.selected else { return Ok(()) };
    let mut fly = false;
    egui::SidePanel::right("inspector").default_width(370.0).frame(egui::Frame::new().fill(super::PANEL).inner_margin(12)).show(ctx, |ui| {
        let u = &sim.universe;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match sel {
            Target::Star(s) => {
                ui.horizontal(|ui| {
                    ui.heading(&u.system(s).star.name);
                    if rig.focus != Some(sel) && ui.button("Fly to").clicked() {
                        fly = true;
                    }
                });
                star_card(ui, u, s);
            }
            Target::Body(r) => {
                let b = u.body(r);
                ui.horizontal(|ui| {
                    ui.heading(&b.name);
                    if rig.focus != Some(sel) && ui.button("Fly to").clicked() {
                        fly = true;
                    }
                });
                let sub = format!("{} in the {} system{}", b.kind.label(), u.system(r.system).name, if b.real { " · real data" } else { "" });
                ui.label(egui::RichText::new(sub).color(MUTED).size(12.0));
                ui.add_space(4.0);
                let civ = u.civ_on(r).is_some() || u.civs.iter().any(|c| c.system == r.system && c.body == r.body);
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::Overview, "Overview");
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::Orbit, "Orbit");
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::Environment, "Environment");
                    ui.selectable_value(&mut ui_state.body_tab, BodyTab::Life, "Life");
                    if civ {
                        ui.selectable_value(&mut ui_state.body_tab, BodyTab::Civilization, egui::RichText::new("Civilization").color(CIV));
                    }
                });
                ui.separator();
                let tab = if ui_state.body_tab == BodyTab::Civilization && !civ { BodyTab::Overview } else { ui_state.body_tab };
                match tab {
                    BodyTab::Overview => overview(ui, u, r),
                    BodyTab::Orbit => orbit(ui, u, r),
                    BodyTab::Environment => environment(ui, u, r),
                    BodyTab::Life => life(ui, u, r),
                    BodyTab::Civilization => {
                        if let Some(c) = u.civs.iter().rev().find(|c| c.system == r.system && c.body == r.body) {
                            civilization(ui, u, c, &mut ui_state);
                        }
                    }
                }
            }
        });
    });
    if fly {
        rig.focus_on(sel, &sim, None);
    }
    Ok(())
}

fn star_card(ui: &mut egui::Ui, u: &Universe, s: u32) {
    let sys = u.system(s);
    let st = &sys.star;
    let t = u.time;
    ui.label(egui::RichText::new(format!("{} star · {}", st.spectral_type(t), st.phase(t).label())).color(MUTED));
    egui::Grid::new("star").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Mass", format!("{:.3} M_sun", st.mass));
        kv(ui, "Radius", format!("{:.3} R_sun", st.current_radius(t) / cosmogon_sim::astro::SOLAR_RADIUS));
        kv(ui, "Luminosity", format!("{:.4} L_sun", st.luminosity(t)));
        kv(ui, "Temperature", format!("{:.0} K", st.temperature_at(t)));
        kv(ui, "Age", format_duration(st.age(t)));
        kv(ui, "Main-sequence life", format_duration(st.lifetime));
        kv(ui, "Metallicity [Fe/H]", format!("{:+.2}", st.metallicity));
        kv(ui, "Flare activity", format!("{:.2}", st.flare_activity_at(t)));
        let (hi, ho) = st.habitable_zone_au(t);
        kv(ui, "Habitable zone", format!("{hi:.2} – {ho:.2} AU"));
        kv(ui, "Distance from origin", format!("{:.2} ly", sys.position.length() / LIGHT_YEAR));
        if let Some(c) = &sys.companion {
            kv(ui, "Companion", format!("{} ({}, {:.0} AU)", c.star.name, c.star.spectral_type(t), c.orbit.a / AU));
        }
        kv(ui, "Worlds", format!("{} planets, {} moons", sys.planets().count(), sys.bodies.len() - sys.planets().count()));
    });
}

fn overview(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let b = u.body(r);
    let hab = u.habitability(r);
    egui::Grid::new("ov").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Mass", format!("{:.3} M_E ({:.3e} kg)", b.mass_earths(), b.mass));
        kv(ui, "Radius", format!("{:.3} R_E ({} km)", b.radius_earths(), group_digits(b.radius / 1000.0)));
        kv(ui, "Surface gravity", format!("{:.2} g", b.gravity_g()));
        kv(ui, "Density", format!("{:.0} kg/m³", b.density()));
        kv(ui, "Escape velocity", format!("{:.2} km/s", b.escape_velocity() / 1000.0));
        kv(ui, "Rotation", if b.tidally_locked { "tidally locked".to_string() } else { format!("{:.2} h{}", b.rotation_period.abs() / 3600.0, if b.rotation_period < 0.0 { " (retrograde)" } else { "" }) });
        kv(ui, "Axial tilt", format!("{:.1}°", b.axial_tilt.to_degrees()));
        kv(ui, "Mean temperature", format!("{:.0} K ({:.0} °C)", b.temperature, b.temperature - 273.15));
        kv(ui, "Magnetic field", format!("{:.2}× Earth", b.magnetic_field));
        kv(ui, "Geological activity", format!("{:.2}× Earth", b.geology));
        kv(ui, "Habitability", egui::RichText::new(format!("{} ({:.2})", hab.label(), hab.score)).color(if hab.score > 0.0 { LIFE } else { MUTED }));
        if b.kind.has_surface() {
            kv(ui, "Δv to orbit", format!("{:.1} km/s", b.launch_delta_v_kms()));
        }
    });
    if let Some(c) = u.civ_on(r) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("Home of {} ({} people)", c.name, compact(c.population))).color(CIV));
    }
    for c in u.civs.iter().filter(|c| c.is_alive() && c.colonies.iter().any(|col| col.body == r.body) && c.system == r.system) {
        ui.label(egui::RichText::new(format!("Colony of {}", c.name)).color(CIV));
    }
}

fn orbit(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let sys = u.system(r.system);
    let b = &sys.bodies[r.body as usize];
    let o = &b.orbit;
    let mu = sys.parent_mu(r.body as usize);
    let parent = b.parent.map(|p| sys.bodies[p as usize].name.clone()).unwrap_or_else(|| sys.star.name.clone());
    let pos = sys.body_local_position(r.body as usize, u.time) - b.parent.map(|p| sys.body_local_position(p as usize, u.time)).unwrap_or_default();
    egui::Grid::new("orb").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Orbits", parent);
        kv(ui, "Semi-major axis", super::distance(o.a));
        kv(ui, "Eccentricity", format!("{:.4}", o.e));
        kv(ui, "Inclination", format!("{:.2}°", o.i.to_degrees()));
        kv(ui, "Ascending node", format!("{:.2}°", o.node.to_degrees()));
        kv(ui, "Arg. of periapsis", format!("{:.2}°", o.peri.to_degrees()));
        kv(ui, "Periapsis / apoapsis", format!("{} / {}", super::distance(o.periapsis()), super::distance(o.apoapsis())));
        kv(ui, "Period", format_duration(o.period(mu)));
        kv(ui, "Current distance", super::distance(pos.length()));
        kv(ui, "Mean anomaly", format!("{:.1}°", o.mean_anomaly(mu, u.time).to_degrees()));
    });
    ui.label(egui::RichText::new("Analytic Keplerian propagation: exact at any time scale.").size(11.0).color(MUTED));
}

fn environment(ui: &mut egui::Ui, u: &Universe, r: BodyRef) {
    let b = u.body(r);
    heading(ui, "Atmosphere");
    ui.label(b.atmosphere.describe());
    let a = &b.atmosphere;
    if a.is_present() {
        for (name, v) in [("N₂", a.n2), ("O₂", a.o2), ("CO₂", a.co2), ("H₂O", a.h2o), ("CH₄", a.ch4), ("H₂ / He", a.h2he)] {
            if v > 1e-5 {
                bar(ui, &format!("{name}  {:.3}%", v * 100.0), v.max(0.01), egui::Color32::from_rgb(110, 160, 230), "");
            }
        }
    }
    heading(ui, "Water & climate");
    egui::Grid::new("env").num_columns(2).striped(true).show(ui, |ui| {
        kv(ui, "Water inventory", format!("{:.3} Earth oceans", b.hydro.water_inventory));
        kv(ui, "Ocean cover", format!("{:.0}%", b.hydro.ocean_fraction * 100.0));
        kv(ui, "Ice cover", format!("{:.0}%", b.hydro.ice_fraction * 100.0));
        kv(ui, "Subsurface ocean", if b.hydro.subsurface_ocean { "yes" } else { "no" });
        kv(ui, "Bond albedo", format!("{:.2}", b.albedo));
        kv(ui, "Equilibrium temp.", format!("{:.0} K", b.equilibrium_temperature));
        kv(ui, "Surface temp.", format!("{:.0} K", b.temperature));
        if b.kind.has_surface() {
            kv(ui, "Climate", if b.climate_stability(u.time) > 0.6 { "stable (interglacial)" } else { "glacial swings" });
        }
    });
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

fn civilization(ui: &mut egui::Ui, u: &Universe, c: &Civilization, ui_state: &mut UiState) {
    let graph = TechGraph::embedded();
    ui.label(egui::RichText::new(&c.name).size(17.0).strong().color(CIV));
    ui.label(egui::RichText::new(format!("{} · {} · founded {}", c.era(graph), status_label(c), format_date(c.founded, u.start_time, u.gregorian()))).color(MUTED).size(12.0));
    ui.add_space(4.0);
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
        if c.satellites > 0 {
            kv(ui, "Satellites", c.satellites.to_string());
        }
        if !c.colonies.is_empty() {
            let names: Vec<String> = c.colonies.iter().map(|col| u.system(c.system).bodies[col.body as usize].name.clone()).collect();
            kv(ui, "Colonies", names.join(", "));
        }
        if !c.detected.is_empty() {
            kv(ui, "Has detected", c.detected.iter().map(|d| u.civs[*d as usize].name.clone()).collect::<Vec<_>>().join(", "));
        }
    });

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
