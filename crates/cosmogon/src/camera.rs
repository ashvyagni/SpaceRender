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
    /// Radius of the ground under the camera (terrain-aware when close-up terrain is active).
    pub ground_radius: f64,
    transition: Option<Transition>,
    pub applied_args: bool,
    /// Offset of the orbit pivot from the target: zoomed far out, the camera swings round the
    /// Galactic Centre instead of the focused star.
    pub pivot_shift: DVec3,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self { focus: None, target_pos: DVec3::ZERO, distance: 3.0e7, desired_distance: 3.0e7, yaw: 0.6, pitch: 0.35, focus_radius: 6.4e6, ground_radius: 6.4e6, transition: None, applied_args: false, pivot_shift: DVec3::ZERO }
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
        self.target_pos + self.pivot_shift + DVec3::new(cp * sy, sp, cp * cy) * self.distance
    }
}

/// A distance from which a whole star system fits on screen.
pub fn system_view_distance(sim: &Sim, system: u32) -> f64 {
    let sys = sim.universe.system(system);
    let extent = sys.bodies.iter().filter(|b| b.parent.is_none() && b.exists()).map(|b| b.orbit.apoapsis().min(200.0 * AU)).fold(0.5 * AU, f64::max);
    extent * 2.6
}

/// Position (render frame) and radius of a target.
pub fn target_info(sim: &Sim, target: Target) -> (DVec3, f64) {
    let u = &sim.universe;
    match target {
        Target::Star(s) => {
            let sys = u.system(s);
            (to_render(sys.star_position(u.time)), sys.star.current_radius(u.time))
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
        Projection::Perspective(PerspectiveProjection { fov: 50f32.to_radians(), near: 1.0, far: 1.0e27, ..default() }),
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
    if let (Some((lat, lon)), Target::Body(r)) = (args.latlon, target) {
        // Camera above a geographic point: body-local direction rotated into the world.
        let rot = crate::render::body_rotation(&sim, r, sim.universe.time).as_dquat();
        // "noon" = the longitude currently facing the star.
        let lon = if lon.is_nan() {
            let star = rot.inverse() * (to_render(sim.universe.system(r.system).star_position(sim.universe.time)) - pos);
            star.y.atan2(star.x).to_degrees() - 55.0
        } else {
            lon
        };
        let local = cosmogon_sim::planet::terrain::dir_from_lat_lon(lat.to_radians(), lon.to_radians());
        let d = (rot * DVec3::from_array(local)).normalize();
        rig.yaw = d.x.atan2(d.z);
        rig.pitch = d.y.asin();
    } else if args.yaw.is_none() {
        // Start on the day side, the star a little off to one side.
        let star = to_render(sim.universe.system(target.system()).star_position(sim.universe.time));
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
    // Close to a surface, controls work on altitude and slow down so the ground is navigable.
    let ground = rig.ground_radius;
    let altitude = (rig.desired_distance - ground).max(1.0);
    let orbit_rate = 0.005 * (altitude / (rig.focus_radius * 0.5)).clamp(0.0005, 1.0);
    let zoom_by = |rig: &mut CameraRig, factor: f64| {
        rig.desired_distance = ground + (rig.desired_distance - ground).max(1.0) * factor;
    };
    // With the throw and grab tools the left button belongs to the tool; orbit with the right.
    let tool_owns_left = matches!(sim.tool, crate::sim::Tool::Throw(_) | crate::sim::Tool::Grab);
    if !wants_pointer {
        if (mouse.pressed(MouseButton::Left) && !tool_owns_left) || mouse.pressed(MouseButton::Right) {
            rig.yaw -= motion.delta.x as f64 * orbit_rate;
            rig.pitch = (rig.pitch + motion.delta.y as f64 * orbit_rate).clamp(-1.54, 1.54);
        }
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y as f64,
            MouseScrollUnit::Pixel => scroll.delta.y as f64 / 40.0,
        };
        if lines != 0.0 {
            zoom_by(&mut rig, (-lines * 0.15).exp());
        }
    }
    if !wants_keys {
        let zoom = (keys.pressed(KeyCode::KeyS) as i32 - keys.pressed(KeyCode::KeyW) as i32) as f64;
        if zoom != 0.0 {
            zoom_by(&mut rig, (zoom * 0.04).exp());
        }
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

#[allow(clippy::too_many_arguments)]
fn update_camera(
    mut rig: ResMut<CameraRig>,
    sim: Res<Sim>,
    lod: Res<crate::render::terrain_lod::TerrainLod>,
    time: Res<Time>,
    mut view: ResMut<ViewInfo>,
    windows: Query<&Window>,
    mut cam: Query<(&mut Transform, &mut Projection, &mut Bloom), With<MainCamera>>,
    settings: Res<UserSettings>,
) {
    let dt = time.delta_secs().min(0.1);
    let Some(mut focus) = rig.focus else { return };
    // The focused body may have been destroyed or deleted: fall back to its star.
    if let Target::Body(r) = focus {
        let gone = sim.universe.systems.get(r.system as usize).and_then(|s| s.bodies.get(r.body as usize)).is_none_or(|b| !b.exists());
        if gone {
            focus = Target::Star(r.system);
            rig.focus_on(focus, &sim, None);
        }
    }
    let (live_pos, radius) = target_info(&sim, focus);
    rig.focus_radius = radius;
    // Ground under the camera: real terrain height when close-up terrain is active.
    rig.ground_radius = match focus {
        Target::Body(r) => {
            let rot = crate::render::body_rotation(&sim, r, sim.universe.time).as_dquat();
            let dir_local = rot.inverse() * (rig.eye() - live_pos);
            lod.ground_radius(r, dir_local).unwrap_or(radius).max(radius * 0.99)
        }
        Target::Star(_) => radius,
    };
    let min_d = match focus {
        Target::Star(_) => radius * 1.5,
        Target::Body(_) => rig.ground_radius + 60.0,
    };
    rig.desired_distance = rig.desired_distance.clamp(min_d, 1.5e10 * LIGHT_YEAR);

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

    // Past a few thousand light-years, ease the pivot over to the Galactic Centre so pulling
    // back from any star frames the whole Milky Way.
    let sun = to_render(sim.universe.systems[0].star_position(sim.universe.time));
    let gc = sun + to_render(crate::render::galaxy::galactic_centre_offset() * LIGHT_YEAR);
    let w = ((rig.distance / LIGHT_YEAR / 4_000.0).ln() / (60_000f64 / 4_000.0).ln()).clamp(0.0, 1.0);
    let w = w * w * (3.0 - 2.0 * w);
    rig.pivot_shift = (gc - rig.target_pos) * w;

    let eye = rig.eye();
    view.origin = eye;
    if let Ok(w) = windows.single() {
        view.viewport_h = w.height();
    }
    let Ok((mut tf, mut proj, mut bloom)) = cam.single_mut() else { return };
    // Bloom makes stars glow in space; when a sunlit world fills the view it would only veil
    // the surface, so fade it with proximity.
    if settings.graphics.bloom() {
        let fill = ((rig.distance / radius - 1.0) / 4.0).clamp(0.0, 1.0) as f32;
        bloom.intensity = 0.03 + 0.19 * fill;
    }
    // Normalise in f64: squaring galaxy-scale offsets (~1e21 m) overflows f32.
    let look = (rig.target_pos + rig.pivot_shift - eye).normalize_or(DVec3::NEG_Z).as_vec3();
    *tf = Transform::IDENTITY.looking_to(look, Vec3::Y);
    let altitude = (rig.distance - rig.ground_radius).max(1.0);
    if matches!(focus, Target::Body(_)) {
        // Near the ground, tilt the view from "straight down" towards the horizon.
        let a = altitude / radius;
        let tilt = (1.0 - a / 0.06).clamp(0.0, 1.0).powf(1.5) as f32 * 1.25;
        if tilt > 0.0 {
            let radial = (eye - rig.target_pos).normalize().as_vec3();
            let up_cam = tf.up().as_vec3();
            let tangent = (up_cam - radial * up_cam.dot(radial)).normalize_or(Vec3::X);
            let forward = -radial * tilt.cos() + tangent * tilt.sin();
            *tf = Transform::IDENTITY.looking_to(forward, radial);
        }
    }
    if let Projection::Perspective(p) = proj.as_mut() {
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
