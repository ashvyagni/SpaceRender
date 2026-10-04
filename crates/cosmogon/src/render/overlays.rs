//! Line overlays drawn with gizmos: orbits (analytic or osculating), past trails, predicted
//! trajectories (selected body, creation preview, launch path), velocity vectors and the
//! expanding radio spheres of civilizations that broadcast.

use std::collections::{HashMap, VecDeque};

use bevy::math::DVec3;
use bevy::prelude::*;
use cosmogon_sim::astro::dynamics::Slot;
use cosmogon_sim::astro::{Orbit, SPEED_OF_LIGHT};
use cosmogon_sim::{BodyRef, Vec3d};

use super::{to_render, ViewInfo};
use crate::camera::CameraRig;
use crate::persistence::UserSettings;
use crate::sim::{Sim, Target};
use crate::state::{AppState, Frame};

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Trails>().add_systems(
            Update,
            (record_trails, draw_orbits, draw_trails, draw_predictions, draw_velocity, draw_radio_spheres).chain().after(super::apply_origin).in_set(Frame::Apply).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
        );
    }
}

/// Past positions of bodies in the focused dynamic system, relative to their parent
/// (so a moon's trail loops around its planet instead of smearing along the planet's orbit).
#[derive(Resource, Default)]
pub struct Trails {
    generation: u64,
    system: Option<u32>,
    last_t: f64,
    paths: HashMap<BodyRef, VecDeque<Vec3d>>,
}

const TRAIL_POINTS: usize = 400;

fn record_trails(sim: Res<Sim>, rig: Res<CameraRig>, mut trails: ResMut<Trails>) {
    let u = &sim.universe;
    let Some(focus) = rig.focus else { return };
    let sid = focus.system();
    if trails.generation != sim.generation || trails.system != Some(sid) || u.time < trails.last_t {
        trails.paths.clear();
        trails.generation = sim.generation;
        trails.system = Some(sid);
    }
    let sys = u.system(sid);
    if !sys.is_dynamic() {
        return;
    }
    // Sample about 400 points per (shortest visible) orbit: every ~1/400 of a sim-day at low
    // speed, coarser when time runs faster.
    let min_dt = (sim.rate() / 60.0).max(60.0);
    if u.time - trails.last_t < min_dt {
        return;
    }
    trails.last_t = u.time;
    for i in sys.existing() {
        let r = BodyRef { system: sid, body: i as u32 };
        let rel = sys.body_state(i, u.time).pos - sys.parent_state(i, u.time).pos;
        let path = trails.paths.entry(r).or_default();
        path.push_back(rel);
        if path.len() > TRAIL_POINTS {
            path.pop_front();
        }
    }
}

fn draw_trails(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>, trails: Res<Trails>, settings: Res<UserSettings>) {
    if !settings.show_trails {
        return;
    }
    let u = &sim.universe;
    for (r, path) in &trails.paths {
        let Some(sys) = u.systems.get(r.system as usize) else { continue };
        if sys.bodies.get(r.body as usize).is_none_or(|b| !b.exists()) || path.len() < 2 {
            continue;
        }
        let parent = sys.position + sys.parent_state(r.body as usize, u.time).pos;
        let selected = sim.selected == Some(Target::Body(*r));
        let n = path.len() as f32;
        let pts: Vec<(Vec3, Color)> = path
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let a = (k as f32 / n) * if selected { 0.7 } else { 0.35 };
                ((to_render(parent + *p) - view.origin).as_vec3(), Color::srgba(0.95, 0.75, 0.4, a))
            })
            .collect();
        gizmos.linestrip_gradient(pts);
    }
}

fn draw_orbits(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>, rig: Res<CameraRig>, settings: Res<UserSettings>) {
    if !settings.show_orbits {
        return;
    }
    let u = &sim.universe;
    let t = u.time;
    let Some(focus) = rig.focus else { return };
    let sys = u.system(focus.system());
    let star = to_render(sys.star_position(t));
    let cam_dist = (view.origin - star).length();
    for (i, b) in sys.bodies.iter().enumerate() {
        if !b.exists() {
            continue;
        }
        let parent_pos = to_render(sys.position + sys.parent_state(i, t).pos);
        // Analytic orbit, or the osculating ellipse of the N-body state.
        let orbit = if sys.is_dynamic() {
            let s = sys.body_state(i, t);
            let p = sys.parent_state(i, t);
            let mu = sys.parent_mu(i) + b.mu();
            match Orbit::from_state(s.pos - p.pos, s.vel - p.vel, mu, t) {
                Some(o) => o,
                None => continue, // unbound: the prediction shows its path
            }
        } else {
            b.orbit
        };
        // Moons' orbits only when close enough for them to be distinguishable.
        let extent = orbit.apoapsis();
        let parent_dist = (view.origin - parent_pos).length();
        if b.parent.is_some() && parent_dist > extent * 60.0 {
            continue;
        }
        if b.parent.is_none() && cam_dist > extent * 4000.0 {
            continue;
        }
        let selected = sim.selected == Some(Target::Body(BodyRef { system: sys.id, body: i as u32 }));
        let color = if selected {
            Color::srgba(1.0, 0.78, 0.35, 0.9)
        } else if b.parent.is_some() {
            Color::srgba(0.55, 0.65, 0.8, 0.22)
        } else {
            Color::srgba(0.55, 0.7, 0.95, 0.3)
        };
        let points: Vec<Vec3> = orbit.path(256).into_iter().map(|p| (parent_pos + to_render(p) - view.origin).as_vec3()).collect();
        gizmos.linestrip(points, color);
    }
}

fn draw_predictions(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>, settings: Res<UserSettings>) {
    let Some((subject, p)) = &sim.prediction.result else { return };
    let u = &sim.universe;
    // The launch tool and the create preview always show their path.
    if subject.is_some() && !settings.show_predictions {
        return;
    }
    let system = match subject {
        Some(r) => r.system,
        None => match sim.launch_target() {
            Some(t) => t.system,
            None => sim.prediction.system,
        },
    };
    let Some(sys) = u.systems.get(system as usize) else { return };
    let wanted = match subject {
        Some(r) => Slot::Body(r.body),
        None => Slot::Body(u32::MAX),
    };
    let Some(k) = p.slots.iter().position(|s| *s == wanted) else { return };
    let color = if subject.is_some() { Color::srgba(0.45, 0.85, 1.0, 0.75) } else { Color::srgba(1.0, 0.45, 0.25, 0.95) };
    let pts: Vec<Vec3> = p.paths[k].iter().map(|x| (to_render(sys.position + *x) - view.origin).as_vec3()).collect();
    // Dashed: draw every other segment.
    for w in pts.windows(2).step_by(2) {
        gizmos.line(w[0], w[1], color);
    }
    if let Some(end) = pts.last() {
        if p.contact.is_some() {
            gizmos.sphere(Isometry3d::from_translation(*end), (view.origin - (view.origin + end.as_dvec3())).length() as f32 * 0.004, Color::srgb(1.0, 0.3, 0.2));
        }
    }
}

fn draw_velocity(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>, settings: Res<UserSettings>) {
    if !settings.show_velocity {
        return;
    }
    let u = &sim.universe;
    let Some(Target::Body(r)) = sim.selected else { return };
    let Some(sys) = u.systems.get(r.system as usize) else { return };
    if sys.bodies.get(r.body as usize).is_none_or(|b| !b.exists()) {
        return;
    }
    let s = sys.body_state(r.body as usize, u.time);
    let p = sys.parent_state(r.body as usize, u.time);
    let pos: DVec3 = to_render(sys.position + s.pos);
    let rel_v = s.vel - p.vel;
    let d = (pos - view.origin).length();
    // Arrow length: 15% of the view distance, direction of motion relative to the parent.
    let dir = to_render(rel_v.normalize());
    let start = (pos - view.origin).as_vec3();
    let end = (pos + dir * d * 0.15 - view.origin).as_vec3();
    gizmos.arrow(start, end, Color::srgb(0.4, 1.0, 0.6));
}

fn draw_radio_spheres(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>) {
    let u = &sim.universe;
    for c in &u.civs {
        let Some(since) = c.radio_since else { continue };
        let radius = SPEED_OF_LIGHT * (u.time - since);
        let center: DVec3 = to_render(u.system(c.system).position);
        if radius <= 0.0 || (view.origin - center).length() < radius * 0.02 {
            continue;
        }
        let rel = (center - view.origin).as_vec3();
        let iso = Isometry3d::from_translation(rel);
        gizmos.sphere(iso, radius as f32, Color::srgba(0.4, 0.9, 1.0, 0.25)).resolution(48);
    }
}
