//! Astronomical camera: orbit / zoom / track / cinematic focus transitions, all in `f64`.
//!
//! The camera's absolute position becomes the floating origin each frame (see `render`).
//! Zoom is exponential and the near plane follows the distance to the nearest surface, so
//! the same controls work from a few hundred metres above a planet to tens of light-years.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::core_pipeline::Skybox;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::math::DVec3;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy_egui::EguiContexts;
use cosmogon_sim::astro::{AU, LIGHT_YEAR};
use cosmogon_sim::BodyRef;

use crate::args::Args;
use crate::persistence::UserSettings;
use crate::render::{to_render, SkyHandle, ViewInfo};
use crate::sim::{Sim, Target};
use crate::state::{AppState, Frame};

#[derive(Component)]
pub struct MainCamera;

struct Transition {
    from_pos: DVec3,
    from_dist: f64,
    elapsed: f32,
    duration: f32,
}

#[derive(Resource)]
pub struct CameraRig {
    pub focus: Option<Target>,
    pub target_pos: DVec3,
    pub distance: f64,
    pub desired_distance: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub focus_radius: f64,
    transition: Option<Transition>,
    pub applied_args: bool,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self { focus: None, target_pos: DVec3::ZERO, distance: 3.0e7, desired_distance: 3.0e7, yaw: 0.6, pitch: 0.35, focus_radius: 6.4e6, transition: None, applied_args: false }
    }
}

impl CameraRig {
    /// Fly to a new target. `distance_radii`: how far to end up, in target radii (None = keep a sensible default).
    pub fn focus_on(&mut self, target: Target, sim: &Sim, distance_radii: Option<f64>) {
        let (_, radius) = target_info(sim, target);
        let default_radii = match target {
            Target::Star(_) => system_view_distance(sim, target.system()) / radius,
            Target::Body(_) => 4.0,
        };
        self.transition = Some(Transition { from_pos: self.target_pos, from_dist: self.distance, elapsed: 0.0, duration: 1.8 });
        self.focus = Some(target);
        self.desired_distance = radius * distance_radii.unwrap_or(default_radii);
    }

    pub fn eye(&self) -> DVec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target_pos + DVec3::new(cp * sy, sp, cp * cy) * self.distance
    }
}

/// A distance from which a whole star system fits on screen.
pub fn system_view_distance(sim: &Sim, system: u32) -> f64 {
    let sys = sim.universe.system(system);
    let extent = sys.bodies.iter().filter(|b| b.parent.is_none()).map(|b| b.orbit.apoapsis()).fold(0.5 * AU, f64::max);
    extent * 2.6
}

/// Position (render frame) and radius of a target.
pub fn target_info(sim: &Sim, target: Target) -> (DVec3, f64) {
    let u = &sim.universe;
    match target {
        Target::Star(s) => {
            let sys = u.system(s);
            (to_render(sys.position), sys.star.current_radius(u.time))
        }
        Target::Body(r) => (to_render(u.body_position(r, u.time)), u.body(r).radius),
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraRig>()
            .add_systems(PostStartup, spawn_camera)
            .add_systems(OnEnter(AppState::Observing), initial_focus)
            .add_systems(Update, (camera_input, update_camera).chain().in_set(Frame::Camera).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)))
            .add_systems(Update, menu_camera.in_set(Frame::Camera).run_if(not(in_state(AppState::Observing))))
            .add_systems(Update, apply_graphics_settings.run_if(resource_changed::<UserSettings>));
    }
}

fn spawn_camera(mut commands: Commands, sky: Res<SkyHandle>, settings: Res<UserSettings>) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::AgX,
        Bloom { intensity: if settings.graphics.bloom() { 0.22 } else { 0.0 }, ..Bloom::NATURAL },
        msaa_for(&settings),
        Projection::Perspective(PerspectiveProjection { fov: 50f32.to_radians(), near: 1.0, far: 1.0e22, ..default() }),
        Skybox { image: sky.0.clone(), brightness: 300.0, ..default() },
        Transform::default(),
        MainCamera,
    ));
}

fn msaa_for(s: &UserSettings) -> Msaa {
    match s.graphics.msaa() {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        _ => Msaa::Sample4,
    }
}

fn apply_graphics_settings(settings: Res<UserSettings>, mut q: Query<(&mut Bloom, &mut Msaa), With<MainCamera>>) {
    for (mut bloom, mut msaa) in &mut q {
        bloom.intensity = if settings.graphics.bloom() { 0.22 } else { 0.0 };
        *msaa = msaa_for(&settings);
    }
}

/// Pick something interesting to look at when a universe opens (or obey `--focus`).
fn initial_focus(mut rig: ResMut<CameraRig>, mut sim: ResMut<Sim>, args: Res<Args>) {
    let u = &sim.universe;
    let mut target = Target::Star(0);
    let mut radii = None;
    if let Some(c) = u.civs.iter().find(|c| c.is_alive()) {
        target = Target::Body(BodyRef { system: c.system, body: c.body });
    } else if let Some(b) = u.biospheres.iter().filter(|b| b.system == 0).max_by(|a, b| a.habitability.score.total_cmp(&b.habitability.score).then(a.stage.cmp(&b.stage))) {
        if b.habitability.score > 0.0 || b.stage > cosmogon_sim::life::Stage::Sterile {
            target = Target::Body(BodyRef { system: b.system, body: b.body });
        }
    }
    if !rig.applied_args {
        rig.applied_args = true;
        if let Some(name) = &args.focus {
            if let Some(t) = find_target(&sim, name) {
                target = t;
            }
        }
        radii = args.distance;
        if let Some(y) = args.yaw {
            rig.yaw = y;
        }
        if let Some(p) = args.pitch {
            rig.pitch = p;
        }
        if args.select_civ {
            sim.selected = Some(target);
        }
    }
    let (pos, radius) = target_info(&sim, target);
    if args.yaw.is_none() {
        // Start on the day side, the star a little off to one side.
        let star = to_render(sim.universe.system(target.system()).position);
        let d = (star - pos).normalize_or(DVec3::Z);
        rig.yaw = d.x.atan2(d.z) + 0.55;
        rig.pitch = d.y.asin() + 0.25;
    }
    rig.target_pos = pos;
    rig.focus = Some(target);
    rig.focus_radius = radius;
    rig.distance = radius * radii.unwrap_or(match target {
        Target::Star(_) => system_view_distance(&sim, 0) / radius,
        Target::Body(_) => 3.2,
    });
    rig.desired_distance = rig.distance;
    rig.transition = None;
    if sim.selected.is_none() {
        sim.selected = Some(target);
    }
}

pub fn find_target(sim: &Sim, name: &str) -> Option<Target> {
    let u = &sim.universe;
    for sys in &u.systems {
        if sys.name.eq_ignore_ascii_case(name) || sys.star.name.eq_ignore_ascii_case(name) {
            return Some(Target::Star(sys.id));
        }
        if let Some(i) = sys.find_body(name) {
            return Some(Target::Body(BodyRef { system: sys.id, body: i as u32 }));
        }
    }
    None
}

fn camera_input(
    mut rig: ResMut<CameraRig>,
    mut contexts: EguiContexts,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    keys: Res<ButtonInput<KeyCode>>,
    sim: Res<Sim>,
    args: Res<Args>,
) {
    // Automated captures must be reproducible: ignore whatever the mouse is doing.
    if args.capture.is_some() {
        return;
    }
    let (wants_pointer, wants_keys) = match contexts.ctx_mut() {
        Ok(ctx) => (ctx.wants_pointer_input() || ctx.is_pointer_over_area(), ctx.wants_keyboard_input()),
        Err(_) => (false, false),
    };
    if !wants_pointer {
        if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
            rig.yaw -= motion.delta.x as f64 * 0.005;
            rig.pitch = (rig.pitch + motion.delta.y as f64 * 0.005).clamp(-1.54, 1.54);
        }
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y as f64,
            MouseScrollUnit::Pixel => scroll.delta.y as f64 / 40.0,
        };
        if lines != 0.0 {
            rig.desired_distance *= (-lines * 0.15).exp();
        }
    }
    if !wants_keys {
        let zoom = (keys.pressed(KeyCode::KeyS) as i32 - keys.pressed(KeyCode::KeyW) as i32) as f64;
        rig.desired_distance *= (zoom * 0.04).exp();
        rig.yaw += (keys.pressed(KeyCode::KeyA) as i32 - keys.pressed(KeyCode::KeyD) as i32) as f64 * 0.02;
        if keys.just_pressed(KeyCode::KeyF) {
            if let Some(t) = sim.selected {
                rig.focus_on(t, &sim, None);
            }
        }
        if keys.just_pressed(KeyCode::Home) {
            if let Some(f) = rig.focus {
                rig.focus_on(Target::Star(f.system()), &sim, None);
            }
        }
    }
}

fn update_camera(
    mut rig: ResMut<CameraRig>,
    sim: Res<Sim>,
    time: Res<Time>,
    mut view: ResMut<ViewInfo>,
    windows: Query<&Window>,
    mut cam: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let dt = time.delta_secs().min(0.1);
    let Some(focus) = rig.focus else { return };
    let (live_pos, radius) = target_info(&sim, focus);
    rig.focus_radius = radius;
    let min_d = match focus {
        Target::Star(_) => radius * 1.5,
        Target::Body(_) => radius * 1.00005 + 200.0,
    };
    rig.desired_distance = rig.desired_distance.clamp(min_d, 60.0 * LIGHT_YEAR);

    if let Some(tr) = rig.transition.as_mut() {
        tr.elapsed += dt;
        let s = (tr.elapsed / tr.duration).clamp(0.0, 1.0) as f64;
        let e = s * s * (3.0 - 2.0 * s);
        let (from_pos, from_dist, done) = (tr.from_pos, tr.from_dist, s >= 1.0);
        rig.target_pos = from_pos.lerp(live_pos, e);
        // Logarithmic interpolation: travel feels uniform across scales; pull back mid-flight.
        let span = (from_pos - live_pos).length();
        let peak = (span * 0.6).max(from_dist).max(rig.desired_distance);
        let l0 = from_dist.ln();
        let l1 = rig.desired_distance.ln();
        let lp = peak.ln();
        let lerp = l0 + (l1 - l0) * e + (lp - l0.max(l1)).max(0.0) * (4.0 * e * (1.0 - e));
        rig.distance = lerp.exp();
        if done {
            rig.transition = None;
        }
    } else {
        rig.target_pos = live_pos;
        let k = 1.0 - (-10.0 * dt as f64).exp();
        rig.distance = (rig.distance.ln() + (rig.desired_distance.ln() - rig.distance.ln()) * k).exp();
    }

    let eye = rig.eye();
    view.origin = eye;
    if let Ok(w) = windows.single() {
        view.viewport_h = w.height();
    }
    let Ok((mut tf, mut proj)) = cam.single_mut() else { return };
    let look = (rig.target_pos - eye).as_vec3();
    *tf = Transform::IDENTITY.looking_to(look.normalize_or(Vec3::NEG_Z), Vec3::Y);
    if let Projection::Perspective(p) = proj.as_mut() {
        let altitude = (rig.distance - radius).max(1.0);
        p.near = (altitude * 0.05).clamp(0.5, 1.0e9) as f32;
        view.fov_y = p.fov;
    }
}

/// Slow drift around the showcase planet on the menu / loading screens.
fn menu_camera(time: Res<Time>, mut view: ResMut<ViewInfo>, mut cam: Query<(&mut Transform, &mut Projection), With<MainCamera>>) {
    let t = time.elapsed_secs_f64() * 0.03;
    let r = 6.371e6 * 2.6;
    let eye = DVec3::new(t.sin() * r, 0.25 * r, t.cos() * r);
    view.origin = eye;
    let Ok((mut tf, mut proj)) = cam.single_mut() else { return };
    // Look slightly past the planet so it sits off-centre, leaving room for the menu.
    let target = DVec3::new(-(t.cos()) * 0.45 * 6.371e6, 0.0, t.sin() * 0.45 * 6.371e6);
    *tf = Transform::IDENTITY.looking_to((target - eye).as_vec3().normalize(), Vec3::Y);
    if let Projection::Perspective(p) = proj.as_mut() {
        p.near = 1000.0;
    }
}
