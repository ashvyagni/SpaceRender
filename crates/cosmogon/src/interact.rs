//! Direct manipulation in the 3D view: throwing new objects and grabbing existing ones.
//!
//! **Throw**: press anywhere in space to place the chosen object on the plane of the
//! focused system (through the focused object), drag to give it a velocity, release to
//! throw. Velocity is relative to the focused object: dragging as far as the object is from
//! the focus gives it the circular-orbit speed there, so small drags make orbits and long
//! drags make escapes and collisions. A click without dragging places it on a circular
//! orbit. The predicted path updates live while dragging.
//!
//! **Grab**: press on an object, drag it across the same plane, release to move it there
//! (it keeps its velocity — like picking up a marble and putting it down elsewhere).
//! Both are ordinary, undoable sandbox edits.

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use cosmogon_physics::nbody::Particle;
use cosmogon_sim::astro::dynamics::State;
use cosmogon_sim::astro::G;
use cosmogon_sim::sandbox::{body_from_preset, preset, Edit};
use cosmogon_sim::{BodyRef, Vec3d};

use crate::camera::{CameraRig, MainCamera};
use crate::render::{to_render, ViewInfo};
use crate::sim::{Sim, Target, Tool};
use crate::state::AppState;

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Drag>().add_systems(Update, (throw_and_grab, draw_drag).chain().in_set(crate::state::Frame::Apply).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

/// An in-progress drag.
#[derive(Resource, Default)]
pub struct Drag {
    pub active: Option<DragState>,
    /// Throws made this session (for naming).
    pub thrown: u32,
}

#[derive(Clone)]
pub struct DragState {
    pub system: u32,
    /// Absolute render-frame positions (m).
    pub center: DVec3,
    pub normal: DVec3,
    pub start: DVec3,
    pub current: DVec3,
    /// Object being grabbed (Grab tool), or `None` when throwing.
    pub grabbed: Option<BodyRef>,
    pub predicted_at: f64,
}

/// Render frame → simulation frame (inverse of `to_render`).
pub fn to_sim(v: DVec3) -> Vec3d {
    Vec3d::new(v.x, -v.z, v.y)
}

/// Reference object for velocities: (system, centre in render frame, its velocity in the
/// system frame, its gravitational parameter).
fn reference(sim: &Sim, focus: Option<Target>) -> Option<(u32, DVec3, Vec3d, f64)> {
    let u = &sim.universe;
    match focus? {
        Target::Star(s) => {
            let sys = u.system(s);
            let vel = sys.dynamics.as_ref().map(|d| d.star.vel).unwrap_or(Vec3d::ZERO);
            Some((s, to_render(sys.star_position(u.time)), vel, sys.star.mu()))
        }
        Target::Body(r) => {
            let sys = u.system(r.system);
            let st = sys.body_state(r.body as usize, u.time);
            Some((r.system, to_render(sys.position + st.pos), st.vel, sys.bodies[r.body as usize].mu()))
        }
    }
}

/// The ray under the cursor in absolute render coordinates.
fn cursor_ray(window: &Window, camera: &Camera, cam_tf: &GlobalTransform, view: &ViewInfo) -> Option<(DVec3, DVec3)> {
    let cursor = window.cursor_position()?;
    let ray = camera.viewport_to_world(cam_tf, cursor).ok()?;
    Some((view.origin + ray.origin.as_dvec3(), ray.direction.as_vec3().as_dvec3()))
}

fn hit_plane(origin: DVec3, dir: DVec3, point: DVec3, normal: DVec3) -> Option<DVec3> {
    let denom = dir.dot(normal);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (point - origin).dot(normal) / denom;
    (t > 0.0).then(|| origin + dir * t)
}

/// Plane to work in: the system's orbital plane, unless the view is nearly edge-on to it,
/// then the plane facing the camera.
fn work_plane(center: DVec3, view: &ViewInfo, ray_dir: DVec3) -> DVec3 {
    let ecliptic = DVec3::Y;
    if ray_dir.dot(ecliptic).abs() > 0.12 {
        ecliptic
    } else {
        (view.origin - center).normalize()
    }
}

/// Velocity (system frame) for a throw from `start` dragged to `end`, relative to the
/// reference object's velocity.
pub fn throw_velocity(center: DVec3, normal: DVec3, start: DVec3, end: DVec3, ref_vel: Vec3d, mu: f64) -> Vec3d {
    let r = (start - center).length().max(1.0);
    let v_circ = (mu / r).sqrt();
    let drag = end - start;
    let rel = if drag.length() < r * 0.02 {
        // A click: prograde circular orbit around the focus.
        normal.cross(start - center).normalize_or_zero() * v_circ
    } else {
        drag / r * v_circ
    };
    ref_vel + to_sim(rel)
}

#[allow(clippy::too_many_arguments)]
fn throw_and_grab(
    mut sim: ResMut<Sim>,
    mut drag: ResMut<Drag>,
    mut contexts: EguiContexts,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    view: Res<ViewInfo>,
    rig: Res<CameraRig>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let throwing = match &sim.tool {
        Tool::Throw(t) => Some(t.preset),
        _ => None,
    };
    let grabbing = matches!(sim.tool, Tool::Grab);
    if throwing.is_none() && !grabbing {
        drag.active = None;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        drag.active = None;
        return;
    }
    let over_ui = contexts.ctx_mut().map(|c| c.wants_pointer_input() || c.is_pointer_over_area()).unwrap_or(false);
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), cams.single()) else { return };
    let Some((origin, dir)) = cursor_ray(window, camera, cam_tf, &view) else { return };

    if mouse.just_pressed(MouseButton::Left) && !over_ui {
        if let Some(p) = throwing {
            let Some((system, center, _, _)) = reference(&sim, rig.focus) else { return };
            let normal = work_plane(center, &view, dir);
            if let Some(hit) = hit_plane(origin, dir, center, normal) {
                drag.active = Some(DragState { system, center, normal, start: hit, current: hit, grabbed: None, predicted_at: -1.0 });
                let _ = p;
            }
        } else if grabbing {
            // Pick the object nearest the cursor ray (within a few pixels' angle or its size).
            let u = &sim.universe;
            let sid = rig.focus.map(|f| f.system()).unwrap_or(0);
            let sys = u.system(sid);
            let mut best: Option<(f64, usize, DVec3)> = None;
            for (i, b) in sys.bodies.iter().enumerate().filter(|(_, b)| b.exists()) {
                let p = to_render(sys.body_position(i, u.time));
                let to = p - origin;
                let along = to.dot(dir);
                if along <= 0.0 {
                    continue;
                }
                let miss = (to - dir * along).length();
                let tolerance = b.radius.max(along * 0.012);
                if miss < tolerance && best.is_none_or(|(d, _, _)| along < d) {
                    best = Some((along, i, p));
                }
            }
            if let Some((_, i, p)) = best {
                let normal = work_plane(p, &view, dir);
                drag.active = Some(DragState { system: sid, center: p, normal, start: p, current: p, grabbed: Some(BodyRef { system: sid, body: i as u32 }), predicted_at: -1.0 });
                sim.selected = Some(Target::Body(BodyRef { system: sid, body: i as u32 }));
            }
        }
    }

    let Some(mut st) = drag.active.clone() else { return };
    if mouse.pressed(MouseButton::Left) {
        if let Some(hit) = hit_plane(origin, dir, if st.grabbed.is_some() { st.start } else { st.center }, st.normal) {
            st.current = hit;
        }
        // Live trajectory preview for throws.
        if let (Some(p), None) = (throwing, st.grabbed) {
            if now - st.predicted_at > 0.2 && sim.prediction.task.is_none() {
                if let (Some(pr), Some((_, _, ref_vel, mu))) = (preset(p), reference(&sim, rig.focus)) {
                    let body = body_from_preset(pr, "preview", 1);
                    let sys = sim.universe.system(st.system);
                    let pos = to_sim(st.start) - sys.position;
                    let vel = throw_velocity(st.center, st.normal, st.start, st.current, ref_vel, mu);
                    let r = (st.start - st.center).length();
                    let v = vel.length().max(1.0);
                    // Long enough to see an orbit or an approach unfold.
                    let horizon = (std::f64::consts::TAU * r / v).clamp(3600.0, 5.0e9);
                    sim.request_projectile_prediction(st.system, Particle::new(pos, vel, body.mass * G, body.interaction_radius()), horizon, now);
                    st.predicted_at = now;
                }
            }
        }
        drag.active = Some(st);
        return;
    }

    // Released.
    drag.active = None;
    match (throwing, st.grabbed) {
        (Some(p), None) => {
            let Some(pr) = preset(p) else { return };
            let Some((_, _, ref_vel, mu)) = reference(&sim, rig.focus) else { return };
            drag.thrown += 1;
            let name = format!("{} {}", pr.label.split([',', '(']).next().unwrap_or(pr.label).trim(), drag.thrown);
            let body = body_from_preset(pr, &name, now.to_bits() ^ drag.thrown as u64);
            let sys = sim.universe.system(st.system);
            let pos = to_sim(st.start) - sys.position;
            let vel = throw_velocity(st.center, st.normal, st.start, st.current, ref_vel, mu);
            let edit = Edit::AddBody { system: st.system, body: Box::new(body), state: State { pos, vel } };
            match sim.edit(edit, now) {
                Ok(out) => {
                    if let Some(c) = out.created {
                        sim.selected = Some(Target::Body(c));
                    }
                }
                Err(e) => sim.status = Some((e, now)),
            }
            sim.prediction.result = None;
        }
        (_, Some(r)) => {
            if (st.current - st.start).length() < 1.0 {
                return;
            }
            let sys = sim.universe.system(r.system);
            let vel = sys.body_state(r.body as usize, sim.universe.time).vel;
            let pos = to_sim(st.current) - sys.position;
            if let Err(e) = sim.edit(Edit::SetState { body: r, state: State { pos, vel } }, now) {
                sim.status = Some((e, now));
            }
        }
        _ => {}
    }
}

/// Arrow and ghost while dragging.
fn draw_drag(mut gizmos: Gizmos, drag: Res<Drag>, view: Res<ViewInfo>, sim: Res<Sim>, rig: Res<CameraRig>) {
    let Some(st) = &drag.active else { return };
    let rel = |p: DVec3| (p - view.origin).as_vec3();
    let size = ((st.start - view.origin).length() * 0.008) as f32;
    let accent = Color::srgb(1.0, 0.72, 0.3);
    match st.grabbed {
        None => {
            gizmos.sphere(Isometry3d::from_translation(rel(st.start)), size, accent);
            if (st.current - st.start).length() > 1.0 {
                gizmos.arrow(rel(st.start), rel(st.current), accent);
            }
            // Ring at the throw distance around the focus, so the plane reads.
            let r = (st.start - st.center).length() as f32;
            let rot = Quat::from_rotation_arc(Vec3::Z, st.normal.as_vec3());
            gizmos.circle(Isometry3d::new(rel(st.center), rot), r, Color::srgba(1.0, 0.72, 0.3, 0.25)).resolution(96);
        }
        Some(r) => {
            let radius = sim.universe.body(r).radius.max((st.current - view.origin).length() * 0.004) as f32;
            gizmos.sphere(Isometry3d::from_translation(rel(st.current)), radius, accent);
            gizmos.line(rel(st.start), rel(st.current), Color::srgba(1.0, 0.72, 0.3, 0.5));
        }
    }
    let _ = rig;
}

/// Speed readout for the drag (km/s), for the HUD.
pub fn drag_speed(sim: &Sim, drag: &Drag, rig: &CameraRig) -> Option<f64> {
    let st = drag.active.as_ref()?;
    if st.grabbed.is_some() {
        return None;
    }
    let (_, _, ref_vel, mu) = reference(sim, rig.focus)?;
    Some((throw_velocity(st.center, st.normal, st.start, st.current, ref_vel, mu) - ref_vel).length() / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_makes_a_circular_prograde_orbit() {
        let mu = 3.986e14;
        let center = DVec3::ZERO;
        let start = DVec3::new(7.0e6, 0.0, 0.0);
        let v = throw_velocity(center, DVec3::Y, start, start, Vec3d::ZERO, mu);
        let v_circ = (mu / 7.0e6f64).sqrt();
        assert!((v.length() / v_circ - 1.0).abs() < 1e-9);
        // Prograde = counter-clockwise about the system's +Z (render +Y).
        let r_sim = to_sim(start);
        assert!(r_sim.cross(v).z > 0.0);
    }

    #[test]
    fn dragging_as_far_as_the_focus_gives_circular_speed_along_the_drag() {
        let mu = 1.327e20;
        let start = DVec3::new(1.5e11, 0.0, 0.0);
        let end = start + DVec3::new(0.0, 0.0, -1.5e11);
        let v = throw_velocity(DVec3::ZERO, DVec3::Y, start, end, Vec3d::new(10.0, 0.0, 0.0), mu);
        let rel = v - Vec3d::new(10.0, 0.0, 0.0);
        assert!((rel.length() / (mu / 1.5e11f64).sqrt() - 1.0).abs() < 1e-9);
        // Render −Z is simulation +Y.
        assert!(rel.y > 0.0 && rel.x.abs() < 1e-6);
    }

    #[test]
    fn render_and_sim_frames_round_trip() {
        let v = Vec3d::new(1.0, 2.0, 3.0);
        let back = to_sim(to_render(v));
        assert!((back - v).length() < 1e-12);
    }
}
