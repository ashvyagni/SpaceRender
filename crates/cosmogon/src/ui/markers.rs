//! Screen-space markers drawn under the panels: stars, planets, moons, settlements,
//! transport networks, satellites and probes. Also handles click-to-select in the 3D view.

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::astro::LIGHT_YEAR;
use cosmogon_sim::civ::settlements::{LinkKind, Tier};
use cosmogon_sim::life::Stage;
use cosmogon_sim::BodyRef;

use super::{UiState, ACCENT, CIV, LIFE, MUTED, TEXT};
use crate::camera::{CameraRig, MainCamera};
use crate::persistence::UserSettings;
use crate::render::{body_rotation, to_render, ViewInfo};
use crate::sim::{Sim, Target};

struct Hit {
    pos: egui::Pos2,
    radius: f32,
    target: Target,
    priority: f32,
}

fn rgb(c: [f32; 3]) -> egui::Color32 {
    let m = c[0].max(c[1]).max(c[2]).max(1e-3);
    egui::Color32::from_rgb((c[0] / m * 255.0) as u8, (c[1] / m * 255.0) as u8, (c[2] / m * 255.0) as u8)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_markers(
    mut contexts: EguiContexts,
    mut sim: ResMut<Sim>,
    mut rig: ResMut<CameraRig>,
    view: Res<ViewInfo>,
    settings: Res<UserSettings>,
    ui_state: Res<UiState>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    args: Res<crate::args::Args>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if ui_state.photo_mode {
        return Ok(());
    }
    let Ok((camera, cam_tf)) = cam.single() else { return Ok(()) };
    let project = |p: DVec3| -> Option<egui::Pos2> {
        let rel = (p - view.origin).as_vec3();
        // Behind the camera?
        if cam_tf.forward().dot(rel) <= 0.0 {
            return None;
        }
        camera.world_to_viewport(cam_tf, rel).ok().map(|v| egui::pos2(v.x, v.y))
    };
    // Large nearby spheres hide markers behind them.
    let occluders: Vec<(DVec3, f64)> = {
        let u = &sim.universe;
        let mut v = Vec::new();
        if let Some(f) = rig.focus {
            let sys = u.system(f.system());
            for (i, b) in sys.bodies.iter().enumerate().filter(|(_, b)| b.exists()) {
                let p = to_render(sys.body_position(i, u.time));
                if view.screen_radius(p, b.radius) > 12.0 {
                    v.push((p, b.radius));
                }
            }
        }
        v
    };
    let occluded = |p: DVec3| -> bool {
        let rel = p - view.origin;
        let dist = rel.length();
        let dir = rel / dist.max(1e-9);
        occluders.iter().any(|(c, r)| {
            let oc = *c - view.origin;
            let t = oc.dot(dir);
            t > 0.0 && t < dist - r * 0.5 && (oc.length_squared() - t * t) < r * r * 0.995 && (p - *c).length() > r * 1.001
        })
    };
    let project = |p: DVec3| -> Option<egui::Pos2> { if occluded(p) { None } else { project(p) } };
    let painter = ctx.layer_painter(egui::LayerId::background());
    let font = egui::FontId::proportional(12.0);
    let small = egui::FontId::proportional(10.5);
    let u = &sim.universe;
    let t = u.time;
    let labels = settings.show_labels && !ui_state.hidden;
    let mut hits: Vec<Hit> = Vec::new();
    let mut label_boxes: Vec<egui::Rect> = Vec::new();
    let mut label = |painter: &egui::Painter, pos: egui::Pos2, text: &str, color: egui::Color32, font: &egui::FontId| {
        let galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
        let rect = egui::Rect::from_min_size(pos + egui::vec2(9.0, -galley.size().y * 0.5), galley.size());
        // Keep a little air between labels so the globe never turns into a wall of text.
        if label_boxes.iter().any(|r| r.expand2(egui::vec2(6.0, 3.0)).intersects(rect)) {
            return;
        }
        label_boxes.push(rect);
        // Soft shadow: legible over bright clouds and ice.
        let shadow = egui::Color32::from_black_alpha((color.a() as f32 * 0.75) as u8);
        let sg = painter.layout_no_wrap(text.to_string(), font.clone(), shadow);
        painter.galley(rect.min + egui::vec2(1.0, 1.0), sg, shadow);
        painter.galley(rect.min, galley, color);
    };
    let focus_sys = rig.focus.map(|f| f.system());

    // Star systems.
    for sys in &u.systems {
        let pos = to_render(sys.star_position(t));
        let d = (pos - view.origin).length();
        let extent = sys.bodies.iter().filter(|b| b.parent.is_none() && b.exists()).map(|b| b.orbit.apoapsis().min(1.5e13)).fold(1e11, f64::max);
        let Some(sp) = project(pos) else { continue };
        let has_civ = u.civs.iter().any(|c| c.system == sys.id && c.is_alive());
        let has_life = u.biospheres.iter().any(|b| b.system == sys.id && b.stage >= Stage::Microbial);
        let selected = sim.selected == Some(Target::Star(sys.id));
        hits.push(Hit { pos: sp, radius: 10.0, target: Target::Star(sys.id), priority: 1.0 });
        if d > extent * 6.0 {
            let c = rgb(sys.star.color(t));
            painter.circle_stroke(sp, if selected { 9.0 } else { 6.0 }, egui::Stroke::new(1.0_f32, if selected { ACCENT } else { c.gamma_multiply(0.6) }));
            if has_civ {
                painter.circle_stroke(sp, 11.0, egui::Stroke::new(1.5_f32, CIV));
            } else if has_life {
                painter.circle_filled(sp + egui::vec2(7.0, -7.0), 2.5, LIFE);
            }
            if labels || selected {
                let text = if d > 0.3 * LIGHT_YEAR { format!("{}  {:.1} ly", sys.name, d / LIGHT_YEAR) } else { sys.name.clone() };
                label(&painter, sp, &text, if has_civ { CIV } else { TEXT }, &font);
            }
        }
    }

    // Bodies of the focused system.
    if let Some(sid) = focus_sys {
        let sys = u.system(sid);
        for (i, b) in sys.bodies.iter().enumerate() {
            if !b.exists() {
                continue;
            }
            let r = BodyRef { system: sid, body: i as u32 };
            let pos = to_render(sys.body_position(i, t));
            let Some(sp) = project(pos) else { continue };
            let px = view.screen_radius(pos, b.radius);
            if let Some(p) = b.parent {
                // Moons only once their parent is well resolved.
                let pp = to_render(sys.body_position(p as usize, t));
                if view.screen_radius(pp, sys.bodies[p as usize].radius) < 6.0 && sim.selected != Some(Target::Body(r)) {
                    continue;
                }
            }
            let selected = sim.selected == Some(Target::Body(r));
            hits.push(Hit { pos: sp, radius: px.max(9.0), target: Target::Body(r), priority: 2.0 + if b.parent.is_none() { 1.0 } else { 0.0 } });
            let civ = u.civ_on(r).is_some();
            let life = u.biosphere(r).is_some_and(|x| x.stage >= Stage::Microbial);
            let color = if civ { CIV } else if life { LIFE } else { MUTED };
            if px < 6.0 {
                painter.circle_stroke(sp, if selected { 8.0 } else { 5.0 }, egui::Stroke::new(1.2_f32, if selected { ACCENT } else { color }));
            } else if selected && !ui_state.hidden {
                painter.circle_stroke(sp, px + 6.0, egui::Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.6)));
            }
            if (labels && px < 200.0) || selected {
                let off = if px > 6.0 { egui::vec2(px * 0.75, -px * 0.75) } else { egui::Vec2::ZERO };
                label(&painter, sp + off, &b.name, if selected { ACCENT } else { color }, if b.parent.is_some() { &small } else { &font });
            }
            if let Some(c) = u.civ_on(r) {
                if labels || selected {
                    let off = if px > 6.0 { egui::vec2(px * 0.75, -px * 0.75 + 14.0) } else { egui::vec2(0.0, 14.0) };
                    let era = c.era(cosmogon_sim::civ::tech::TechGraph::embedded());
                    label(&painter, sp + off, &format!("{} · {} · K {:.2} · {}", c.name, era, c.kardashev(), super::compact(c.population)), CIV, &small);
                }
            }
            for c in u.civs.iter().filter(|c| c.is_alive() && c.system == sid && c.colonies.iter().any(|col| col.body == i as u32)) {
                let pop = c.colonies.iter().find(|col| col.body == i as u32).map(|col| col.population).unwrap_or(0.0);
                label(&painter, sp + egui::vec2(0.0, 14.0), &format!("{} colony · {}", c.species.name, super::compact(pop)), CIV, &small);
            }
        }
    }

    // Settlements, networks and satellites of civilizations in view.
    for c in u.civs.iter().filter(|c| c.is_alive() && !ui_state.hidden) {
        let r = BodyRef { system: c.system, body: c.body };
        let b = u.body(r);
        let center = to_render(u.body_position(r, t));
        let px = view.screen_radius(center, b.radius);
        if px < 40.0 {
            continue;
        }
        let rot = body_rotation(&sim, r, t).as_dquat();
        let to_cam = (view.origin - center).normalize();
        let surface = |dir: [f64; 3], lift: f64| -> (DVec3, bool) {
            let n = rot * DVec3::new(dir[0], dir[1], dir[2]);
            (center + n * b.radius * lift, n.dot(to_cam) > 0.05)
        };
        let alpha = ((px - 40.0) / 80.0).clamp(0.0, 1.0);
        // Transport links.
        for l in &c.links {
            let (pa, va) = surface(c.sites[l.a as usize].dir(), 1.0);
            let (pb, vb) = surface(c.sites[l.b as usize].dir(), 1.0);
            if !(va && vb) {
                continue;
            }
            if let (Some(a), Some(bp)) = (project(pa), project(pb)) {
                let col = match l.kind {
                    LinkKind::Road => egui::Color32::from_rgba_unmultiplied(240, 200, 140, (90.0 * alpha) as u8),
                    LinkKind::Rail => egui::Color32::from_rgba_unmultiplied(255, 220, 160, (150.0 * alpha) as u8),
                    LinkKind::Sea => egui::Color32::from_rgba_unmultiplied(120, 190, 255, (80.0 * alpha) as u8),
                };
                painter.line_segment([a, bp], egui::Stroke::new(if l.kind == LinkKind::Rail { 1.4_f32 } else { 1.0 }, col));
            }
        }
        // Settlements.
        let mut sites: Vec<_> = c.sites.iter().filter(|s| s.active()).collect();
        sites.sort_by(|a, b| b.population.total_cmp(&a.population));
        for (rank, s) in sites.iter().enumerate() {
            let (p, visible) = surface(s.dir(), 1.0);
            if !visible {
                continue;
            }
            let Some(sp) = project(p) else { continue };
            let tier = s.tier();
            let size = match tier {
                Tier::Camp => 1.2,
                Tier::Village => 1.8,
                Tier::Town => 2.4,
                Tier::City => 3.2,
                Tier::Metropolis => 4.2,
                Tier::Megacity => 5.2,
            };
            // Settlements take their nation's colour; capitals get a ring.
            let (r, g, b) = match s.polity.and_then(|p| c.polities.get(p as usize)) {
                Some(p) => (p.color[0], p.color[1], p.color[2]),
                None => (255, 205, 120),
            };
            painter.circle_filled(sp, size, egui::Color32::from_rgba_unmultiplied(r, g, b, (230.0 * alpha) as u8));
            let is_capital = s.polity.and_then(|p| c.polities.get(p as usize)).is_some_and(|p| p.alive() && std::ptr::eq(&c.sites[p.capital as usize], *s));
            if is_capital {
                painter.circle_stroke(sp, size + 3.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, (200.0 * alpha) as u8)));
            }
            // More names as the world fills the screen; capitals first among equals.
            let budget = (px / 45.0).clamp(4.0, 40.0) as usize;
            if labels && (rank < budget || (is_capital && rank < budget * 2)) && tier >= Tier::Town && px > 140.0 {
                label(&painter, sp, &format!("{}{} · {}", if is_capital { "★ " } else { "" }, s.name, super::compact(s.population)), egui::Color32::from_rgba_unmultiplied(255, 225, 170, (255.0 * alpha) as u8), &small);
            }
        }
        // Satellites in their real orbital families: low orbit (most), navigation constellations
        // in medium orbit, and the geostationary belt where the orbital period equals the
        // day (computed from this world's own mass and rotation). Up to 900 drawn; lit ones
        // glint, those in the planet's shadow are faint.
        let n = c.satellites.min(900);
        let mu = b.mu();
        let geo = (mu * (b.rotation_period.abs() / std::f64::consts::TAU).powi(2)).cbrt();
        let geo_ok = geo > b.radius * 1.5 && !b.tidally_locked;
        // Inertial equatorial frame (the spin axis without the daily rotation).
        let tilt = (crate::render::sim_to_render_rot() * Quat::from_rotation_x(b.axial_tilt as f32)).as_dquat();
        let spin = u.system(c.system).spin_angle(c.body as usize, t);
        let sun_dir = (to_render(u.system(c.system).star_position(t)) - center).normalize();
        for k in 0..n {
            let h = cosmogon_sim::rng::mix(c.id as u64 * 7919 + 13, k as u64);
            let fr = |s: u32| ((h >> s) & 0xFFFF) as f64 / 65535.0;
            let family = fr(0);
            let geo_sat = family >= 0.93 && geo_ok;
            let (radius, inc) = if family < 0.86 {
                (b.radius * (1.04 + 0.12 * fr(32)), (fr(16) - 0.5) * 3.0)
            } else if family < 0.93 {
                (b.radius * (3.0 + 1.6 * fr(32)), 0.96)
            } else if geo_sat {
                (geo * (1.0 + 0.002 * (fr(32) - 0.5)), 0.0)
            } else {
                (b.radius * 2.0, 0.3)
            };
            let node = fr(16) * std::f64::consts::TAU;
            let period = std::f64::consts::TAU * (radius.powi(3) / mu.max(1.0)).sqrt();
            // Geostationary satellites hang over fixed longitudes and turn with the planet.
            let ang = fr(48) * std::f64::consts::TAU + if geo_sat { spin } else { (t / period).fract() * std::f64::consts::TAU };
            // Equatorial frame of the body (its spin axis), then inclination and node.
            let p0 = DVec3::new(ang.cos(), ang.sin(), 0.0);
            let p1 = DVec3::new(p0.x, p0.y * inc.cos(), p0.y * inc.sin());
            let p2 = DVec3::new(p1.x * node.cos() - p1.y * node.sin(), p1.x * node.sin() + p1.y * node.cos(), p1.z);
            let wp = center + tilt * (p2 * radius);
            let rel = wp - view.origin;
            let along = rel.normalize();
            let to_center = center - view.origin;
            let tca = to_center.dot(along);
            let behind = tca > 0.0 && (to_center.length_squared() - tca * tca) < b.radius * b.radius && rel.length() > tca;
            if behind {
                continue;
            }
            // In the planet's shadow?
            let off = wp - center;
            let s_along = off.dot(sun_dir);
            let shadowed = s_along < 0.0 && (off.length_squared() - s_along * s_along) < b.radius * b.radius;
            if let Some(sp) = project(wp) {
                let a = if shadowed { 70.0 } else { 235.0 } * alpha;
                let (rgb, size) = if geo_sat { ((255, 210, 120), 1.8) } else if family >= 0.86 { ((160, 255, 190), 1.6) } else { ((140, 225, 255), 1.4) };
                painter.circle_filled(sp, if shadowed { size * 0.7 } else { size }, egui::Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, a as u8));
            }
        }
        // The geostationary (Clarke) belt as a faint ring, so the GEO family reads at a glance.
        if geo_ok && c.satellites > 50 {
            let mut prev: Option<egui::Pos2> = None;
            for k in 0..=96 {
                let a = k as f64 / 96.0 * std::f64::consts::TAU;
                let wp = center + tilt * (DVec3::new(a.cos(), a.sin(), 0.0) * geo);
                let rel = wp - view.origin;
                let to_center = center - view.origin;
                let along = rel.normalize();
                let tca = to_center.dot(along);
                let hidden = tca > 0.0 && (to_center.length_squared() - tca * tca) < b.radius * b.radius && rel.length() > tca;
                let p = if hidden { None } else { project(wp) };
                if let (Some(a0), Some(a1)) = (prev, p) {
                    painter.line_segment([a0, a1], egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 210, 120, (45.0 * alpha) as u8)));
                }
                prev = p;
            }
        }
        // Orbital stations.
        if c.flags.contains("stations") {
            for k in 0..3u64 {
                let radius = b.radius * (1.07 + 0.05 * k as f64);
                let period = std::f64::consts::TAU * (radius.powi(3) / mu.max(1.0)).sqrt();
                let ang = k as f64 * 2.1 + (t / period).fract() * std::f64::consts::TAU;
                let inc = 0.4 + 0.3 * k as f64;
                let p = DVec3::new(ang.cos(), ang.sin() * inc.cos(), ang.sin() * inc.sin());
                let wp = center + tilt * (p * radius);
                if let Some(sp) = project(wp) {
                    painter.rect_filled(egui::Rect::from_center_size(sp, egui::vec2(5.0, 3.0)), 1.0, egui::Color32::from_rgba_unmultiplied(255, 236, 190, (240.0 * alpha) as u8));
                    if labels && px > 200.0 {
                        label(&painter, sp, "station", egui::Color32::from_rgba_unmultiplied(255, 236, 190, (200.0 * alpha) as u8), &small);
                    }
                }
            }
        }
    }

    // Spacecraft in flight between worlds: a gentle arc from the homeworld to the target.
    for c in u.civs.iter().filter(|c| c.is_alive() && !ui_state.hidden) {
        let sys = u.system(c.system);
        if focus_sys != Some(c.system) {
            continue;
        }
        let home = to_render(sys.body_position(c.body as usize, t));
        for m in &c.missions {
            let Some(target) = sys.bodies.get(m.body as usize).filter(|b| b.exists()) else { continue };
            let _ = target;
            let dest = to_render(sys.body_position(m.body as usize, t));
            let span = dest - home;
            let side = span.cross(DVec3::Y).normalize_or_zero() * span.length() * 0.18;
            let at = |s: f64| home + span * s + side * (std::f64::consts::PI * s).sin();
            let colony = m.kind == cosmogon_sim::civ::space::MissionKind::Colony;
            let color = if colony { CIV } else { egui::Color32::from_rgb(140, 215, 255) };
            // Faint dotted path.
            for k in 0..32 {
                if k % 2 == 1 {
                    continue;
                }
                if let (Some(a), Some(bp)) = (project(at(k as f64 / 32.0)), project(at((k + 1) as f64 / 32.0))) {
                    painter.line_segment([a, bp], egui::Stroke::new(1.0_f32, color.gamma_multiply(0.25)));
                }
            }
            if let Some(sp) = project(at(m.progress(t))) {
                painter.circle_filled(sp, if colony { 3.0 } else { 2.2 }, color);
                if labels {
                    label(&painter, sp, &m.name, color, &small);
                }
            }
        }
        // Colony sites on their worlds.
        for col in &c.colonies {
            let r = BodyRef { system: c.system, body: col.body };
            let Some(cb) = sys.bodies.get(col.body as usize) else { continue };
            let center = to_render(sys.body_position(col.body as usize, t));
            let px = view.screen_radius(center, cb.radius);
            if px < 30.0 {
                continue;
            }
            let (lat, lon) = crate::render::colony_site(c.id, col.body);
            let d = cosmogon_sim::planet::terrain::dir_from_lat_lon(lat, lon);
            let rot = body_rotation(&sim, r, t).as_dquat();
            let n = rot * DVec3::new(d[0], d[1], d[2]);
            if n.dot((view.origin - center).normalize()) < 0.05 {
                continue;
            }
            if let Some(sp) = project(center + n * cb.radius) {
                painter.circle_filled(sp, 3.5, CIV);
                painter.circle_stroke(sp, 6.5, egui::Stroke::new(1.0_f32, CIV.gamma_multiply(0.6)));
                if labels {
                    label(&painter, sp, &format!("{} colony", c.species.name), CIV, &small);
                }
            }
        }
    }

    // Star-forming nebulae, named from afar.
    for n in &u.nurseries {
        let c = to_render(n.position);
        let dist = (c - view.origin).length();
        if labels && dist > n.radius_ly * cosmogon_sim::astro::LIGHT_YEAR * 3.0 {
            if let Some(sp) = project(c) {
                let state = if n.active() { format!("stellar nursery · {} new stars", n.formed) } else { "young cluster".into() };
                label(&painter, sp, &format!("{} · {state}", n.name), egui::Color32::from_rgb(255, 150, 175), &small);
            }
        }
    }

    // Generation ships between the stars.
    for sh in u.starships.iter().filter(|s| !s.arrived) {
        let a = to_render(u.systems[sh.from as usize].position);
        let b = to_render(u.systems[sh.to.system as usize].position);
        if let Some(sp) = project(a.lerp(b, sh.progress(u))) {
            painter.circle_filled(sp, 3.0, CIV);
            painter.circle_stroke(sp, 5.5, egui::Stroke::new(1.0_f32, CIV.gamma_multiply(0.5)));
            label(&painter, sp, &sh.name, CIV, &small);
        }
    }

    // Dyson swarms: collectors on inclined orbits well inside the home world's orbit,
    // glinting gold where sunlit. Drawn when the system is near.
    for c in u.civs.iter().filter(|c| c.is_alive() && c.dyson > 0.0 && !ui_state.hidden) {
        let sys = u.system(c.system);
        let star = to_render(sys.star_position(t));
        let home_a = (sys.body_local_position(sys.top_level(c.body as usize), t) - sys.star_local_position(t)).length();
        if (star - view.origin).length() > 400.0 * cosmogon_sim::astro::AU {
            continue;
        }
        let n = (c.dyson * 3000.0) as u64;
        let mu = sys.star.mu();
        for k in 0..n {
            let h = cosmogon_sim::rng::mix(c.id as u64 * 104_729 + 7, k);
            let fr = |s: u32| ((h >> s) & 0xFFFF) as f64 / 65535.0;
            let r = home_a * (0.3 + 0.2 * fr(0));
            let period = std::f64::consts::TAU * (r * r * r / mu.max(1.0)).sqrt();
            let ang = fr(16) * std::f64::consts::TAU + (t / period).fract() * std::f64::consts::TAU;
            let inc = (fr(32) - 0.5) * 2.6;
            let node = fr(48) * std::f64::consts::TAU;
            let p0 = DVec3::new(ang.cos(), ang.sin(), 0.0);
            let p1 = DVec3::new(p0.x, p0.y * inc.cos(), p0.y * inc.sin());
            let p2 = DVec3::new(p1.x * node.cos() - p1.y * node.sin(), p1.x * node.sin() + p1.y * node.cos(), p1.z);
            let wp = star + p2 * r;
            if let Some(sp) = project(wp) {
                // Collectors facing the camera's side of the star glint brighter.
                let lit = 0.45 + 0.55 * (-(wp - star).normalize().dot((view.origin - star).normalize())).mul_add(-0.5, 0.5);
                painter.circle_filled(sp, 1.3, egui::Color32::from_rgba_unmultiplied(255, 200, 110, (lit * 220.0) as u8));
            }
        }
    }

    // Interstellar probes.
    for p in u.probes.iter().filter(|p| !p.arrived) {
        if let Some(sp) = project(to_render(p.position(&u.systems, t))) {
            painter.circle_filled(sp, 2.5, egui::Color32::from_rgb(140, 230, 255));
            label(&painter, sp, "probe", egui::Color32::from_rgb(140, 230, 255), &small);
        }
    }

    // Click handling in the 3D view.
    let (clicked, double, pointer, over_ui) = ctx.input(|i| (i.pointer.primary_clicked(), i.pointer.button_double_clicked(egui::PointerButton::Primary), i.pointer.interact_pos(), false));
    let over_ui = over_ui || ctx.is_pointer_over_area();
    if (clicked || double) && !over_ui && args.capture.is_none() {
        if let Some(pp) = pointer {
            let best = hits
                .iter()
                .filter(|h| h.pos.distance(pp) <= h.radius + 6.0)
                .max_by(|a, b| (a.priority - a.pos.distance(pp) * 0.01).total_cmp(&(b.priority - b.pos.distance(pp) * 0.01)));
            if let Some(h) = best {
                sim.selected = Some(h.target);
                if double {
                    rig.focus_on(h.target, &sim, None);
                }
            }
        }
    }
    Ok(())
}
