//! Rendering: turns simulation state into a scene.
//!
//! **Precision.** All positions are kept in `f64` ([`WorldPos`], metres). Every frame the
//! camera's `f64` position becomes the origin and each entity's `Transform` is written as its
//! offset from the camera, converted to `f32`. Objects near the camera are therefore exact
//! to sub-millimetre precision whether the camera is on a planet or between stars, and far
//! objects only lose precision that is invisible anyway. (See RENDERING.md.)
//!
//! **Frames.** Simulation frame: X/Y = orbital reference plane, +Z = pole. Render frame
//! (Bevy): +Y up. Mapping: render = (x, z, -y).

pub mod bake;
pub mod materials;
pub mod overlays;
mod sky;

use std::f64::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::asset::embedded_asset;
use bevy::math::DVec3;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use cosmogon_sim::astro::Body;
use cosmogon_sim::{BodyRef, Vec3d};

use crate::persistence::UserSettings;
use crate::sim::Sim;
use crate::state::{AppState, Frame};
use bake::BakedSurface;
use materials::{AtmosphereMaterial, AtmosphereUniform, PlanetMaterial, PlanetUniform};
pub use sky::SkyHandle;

pub fn to_render(v: Vec3d) -> DVec3 {
    DVec3::new(v.x, v.z, -v.y)
}

/// Rotation from the simulation frame (+Z pole) to the render frame (+Y up).
pub fn sim_to_render_rot() -> Quat {
    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}

/// Where the camera is and how it projects; written by the camera each frame.
#[derive(Resource)]
pub struct ViewInfo {
    pub origin: DVec3,
    pub fov_y: f32,
    pub viewport_h: f32,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self { origin: DVec3::ZERO, fov_y: 0.8, viewport_h: 900.0 }
    }
}

impl ViewInfo {
    /// Apparent radius in pixels of a sphere of `radius` metres at `pos`.
    pub fn screen_radius(&self, pos: DVec3, radius: f64) -> f32 {
        let d = (pos - self.origin).length().max(1e-6);
        let ang = (radius / d).min(1.0) as f32;
        ang / (self.fov_y * 0.5).tan() * self.viewport_h * 0.5
    }
}

/// Everything spawned for the current universe (despawned when leaving it).
#[derive(Component)]
pub struct SimVisual;

/// Absolute position (render frame, metres, f64).
#[derive(Component, Default)]
pub struct WorldPos(pub DVec3);

#[derive(Component)]
pub struct StarVisual {
    pub system: u32,
    pub companion: bool,
    pub radius: f64,
}

#[derive(Component)]
pub struct BodyVisual {
    pub r: BodyRef,
    pub radius: f64,
    pub level: u8,
    pub signature: u64,
    pub lights_baked_at: f64,
}

#[derive(Component)]
pub struct PendingBake {
    task: Task<BakedSurface>,
    level: u8,
    signature: u64,
}

/// Marks an atmosphere shell (child of its planet).
#[derive(Component)]
#[allow(dead_code)]
pub struct AtmosphereShell(pub BodyRef);

#[derive(Resource)]
pub struct SharedMeshes {
    pub sphere: Handle<Mesh>,
    pub sphere_low: Handle<Mesh>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/planet.wgsl");
        embedded_asset!(app, "shaders/atmosphere.wgsl");
        app.add_plugins((MaterialPlugin::<PlanetMaterial>::default(), MaterialPlugin::<AtmosphereMaterial>::default()))
            .init_resource::<ViewInfo>()
            .add_systems(Startup, sky::setup_sky)
            .add_systems(OnEnter(AppState::Observing), spawn_universe)
            .add_systems(OnExit(AppState::Observing), despawn_universe)
            .add_systems(
                Update,
                (
                    update_world_positions.in_set(Frame::Positions),
                    (apply_origin, update_planet_materials, request_bakes, finish_bakes, update_lights).chain().in_set(Frame::Apply),
                )
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            )
            .add_plugins(overlays::OverlayPlugin);
        // Created at build time: the initial state's OnEnter runs before Startup systems.
        let (s, t) = app.world().resource::<UserSettings>().graphics.sphere_segments();
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let shared = SharedMeshes { sphere: meshes.add(Sphere::new(1.0).mesh().uv(s, t)), sphere_low: meshes.add(Sphere::new(1.0).mesh().uv(32, 16)) };
        app.insert_resource(shared);
    }
}

pub fn solid_image(rgba: [u8; 4], format: TextureFormat) -> Image {
    let data = if format == TextureFormat::R8Unorm { vec![rgba[0]; 4] } else { rgba.repeat(4) };
    Image::new(Extent3d { width: 2, height: 2, depth_or_array_layers: 1 }, TextureDimension::D2, data, format, RenderAssetUsages::RENDER_WORLD)
}

fn image_from(data: Vec<u8>, w: u32, h: u32, format: TextureFormat) -> Image {
    let mut img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, format, RenderAssetUsages::RENDER_WORLD);
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    img
}

pub fn atmosphere_look(body: &Body) -> Option<([f32; 3], f32)> {
    let a = &body.atmosphere;
    if !body.kind.has_surface() {
        let c = body.color;
        return Some(([c[0] * 0.8 + 0.2, c[1] * 0.8 + 0.2, c[2] * 0.9 + 0.1], 0.35));
    }
    if a.pressure_bar < 0.003 {
        return None;
    }
    let strength = (a.pressure_bar.powf(0.3) as f32 * 0.8).min(1.1);
    let color = if a.ch4 > 0.02 {
        [0.95, 0.62, 0.25]
    } else if a.co2 > 0.5 && a.pressure_bar > 10.0 {
        [1.0, 0.86, 0.6]
    } else if a.co2 > 0.5 {
        [0.95, 0.62, 0.45]
    } else {
        [0.32, 0.55, 1.0]
    };
    Some((color, strength))
}

fn ring_mesh(inner: f32, outer: f32) -> Mesh {
    let seg = 160;
    let mut pos = Vec::new();
    let mut nor = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=seg {
        let a = i as f32 / seg as f32 * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        for (k, r) in [inner, outer].into_iter().enumerate() {
            pos.push([r * c, r * s, 0.0]);
            nor.push([0.0, 0.0, 1.0]);
            uv.push([k as f32, 0.5]);
        }
        if i < seg {
            let b = i * 2;
            idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(idx))
}

#[allow(clippy::too_many_arguments)]
fn spawn_universe(
    mut commands: Commands,
    sim: Res<Sim>,
    shared: Res<SharedMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut atmo_mats: ResMut<Assets<AtmosphereMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let u = &sim.universe;
    let t = u.time;
    let black = images.add(solid_image([0, 0, 0, 0], TextureFormat::R8Unorm));
    for sys in &u.systems {
        let stars = std::iter::once((&sys.star, false)).chain(sys.companion.as_ref().map(|c| (&c.star, true)));
        for (star, companion) in stars {
            let c = star.color(t);
            let lum = (star.luminosity(t).max(1e-4)).powf(0.15) as f32;
            let k = 60.0 * lum;
            let mat = std_mats.add(StandardMaterial { base_color: Color::LinearRgba(LinearRgba::new(c[0] * k, c[1] * k, c[2] * k, 1.0)), unlit: true, ..default() });
            commands.spawn((
                Mesh3d(shared.sphere_low.clone()),
                MeshMaterial3d(mat),
                Transform::default(),
                WorldPos::default(),
                StarVisual { system: sys.id, companion, radius: star.current_radius(t) },
                SimVisual,
            ));
        }

        for (bi, body) in sys.bodies.iter().enumerate() {
            let r = BodyRef { system: sys.id, body: bi as u32 };
            let c = body.color;
            let albedo = images.add(solid_image([(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 0], TextureFormat::Rgba8UnormSrgb));
            let mat = planet_mats.add(PlanetMaterial { u: PlanetUniform::default(), albedo, clouds: black.clone(), lights: black.clone() });
            let mut e = commands.spawn((
                Mesh3d(shared.sphere.clone()),
                MeshMaterial3d(mat),
                Transform::from_scale(Vec3::splat(body.radius as f32)),
                WorldPos::default(),
                BodyVisual { r, radius: body.radius, level: 0, signature: 0, lights_baked_at: -10.0 },
                SimVisual,
            ));
            e.with_children(|p| {
                if let Some((color, strength)) = atmosphere_look(body) {
                    let thickness = if body.kind.has_surface() { 1.025 } else { 1.04 };
                    let m = atmo_mats.add(AtmosphereMaterial { u: AtmosphereUniform { sun: Vec4::new(1.0, 0.0, 0.0, 1.0), color: Vec4::new(color[0], color[1], color[2], strength) } });
                    p.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::from_scale(Vec3::splat(thickness)), AtmosphereShell(r)));
                }
                if let Some(rings) = &body.rings {
                    let tex = bake::bake_rings(body.terrain_seed, rings.opacity, body.color);
                    let img = images.add(image_from(tex, 256, 1, TextureFormat::Rgba8UnormSrgb));
                    let ring = meshes.add(ring_mesh((rings.inner / body.radius) as f32, (rings.outer / body.radius) as f32));
                    let m = std_mats.add(StandardMaterial {
                        base_color_texture: Some(img),
                        base_color: Color::srgb(0.75, 0.72, 0.66),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        ..default()
                    });
                    p.spawn((Mesh3d(ring), MeshMaterial3d(m), Transform::default()));
                }
            });
        }
    }
    info!("spawned visuals for {} systems", u.systems.len());
}

fn despawn_universe(mut commands: Commands, q: Query<Entity, With<SimVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Orientation of a body: axial tilt and spin (tidally locked bodies face their primary).
pub fn body_rotation(sim: &Sim, r: BodyRef, t: f64) -> Quat {
    let sys = sim.universe.system(r.system);
    let b = &sys.bodies[r.body as usize];
    let spin = if b.tidally_locked {
        let rel = sys.body_local_position(r.body as usize, t) - b.parent.map(|p| sys.body_local_position(p as usize, t)).unwrap_or(Vec3d::ZERO);
        (-rel.y).atan2(-rel.x)
    } else {
        (t / b.rotation_period).rem_euclid(1.0) * TAU
    };
    sim_to_render_rot() * Quat::from_rotation_x(b.axial_tilt as f32) * Quat::from_rotation_z(spin as f32)
}

fn update_world_positions(sim: Res<Sim>, mut stars: Query<(&StarVisual, &mut WorldPos), Without<BodyVisual>>, mut bodies: Query<(&BodyVisual, &mut WorldPos, &mut Transform), Without<StarVisual>>) {
    let u = &sim.universe;
    let t = u.time;
    for (s, mut wp) in &mut stars {
        let sys = u.system(s.system);
        wp.0 = to_render(if s.companion { sys.companion_position(t).unwrap_or(sys.position) } else { sys.position });
    }
    for (b, mut wp, mut tf) in &mut bodies {
        wp.0 = to_render(u.body_position(b.r, t));
        tf.rotation = body_rotation(&sim, b.r, t);
    }
}

fn apply_origin(view: Res<ViewInfo>, mut q: Query<(&WorldPos, &mut Transform, Option<&StarVisual>, Option<&BodyVisual>, &mut Visibility)>) {
    for (wp, mut tf, star, body, mut vis) in &mut q {
        let rel = wp.0 - view.origin;
        tf.translation = rel.as_vec3();
        if let Some(s) = star {
            // Keep stars visible as bright points at any distance (bloom does the rest).
            let d = rel.length();
            tf.scale = Vec3::splat(s.radius.max(d * 0.0018) as f32);
        }
        if let Some(b) = body {
            let px = view.screen_radius(wp.0, b.radius);
            let want = if px > 0.35 { Visibility::Inherited } else { Visibility::Hidden };
            if *vis != want {
                *vis = want;
            }
        }
    }
}

/// Illumination intensity from stellar flux, compressed so outer planets stay visible.
fn sun_intensity(flux_rel_earth: f64) -> f32 {
    (flux_rel_earth.max(1e-6).powf(0.28) as f32 * 1.6).clamp(0.15, 5.0)
}

#[allow(clippy::too_many_arguments)]
fn update_planet_materials(
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    bodies: Query<(&BodyVisual, &WorldPos, &MeshMaterial3d<PlanetMaterial>, &Children)>,
    shells: Query<&MeshMaterial3d<AtmosphereMaterial>>,
    stars: Query<(&StarVisual, &WorldPos)>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut atmo_mats: ResMut<Assets<AtmosphereMaterial>>,
) {
    let u = &sim.universe;
    let t = u.time;
    for (b, wp, mat, children) in &bodies {
        // Only bodies that are actually resolved on screen need fresh lighting.
        if view.screen_radius(wp.0, b.radius) < 0.35 {
            continue;
        }
        let sys = u.system(b.r.system);
        let Some(star_pos) = stars.iter().find(|(s, _)| s.system == b.r.system && !s.companion).map(|(_, p)| p.0) else { continue };
        let to_star = star_pos - wp.0;
        let dist = to_star.length();
        let flux = sys.star.luminosity(t) / (dist / cosmogon_sim::astro::AU).powi(2);
        let sun = to_star.normalize().as_vec3().extend(sun_intensity(flux));
        let c = sys.star.color(t);
        let body = u.body(b.r);
        let civ = u.civ_on(b.r);
        let lights = civ.map(|c| if c.flags.contains("electric_light") { 4.0 } else { 1.2 }).unwrap_or(0.0);
        let atmo = atmosphere_look(body);
        if let Some(m) = planet_mats.get_mut(&mat.0) {
            m.u.sun = sun;
            m.u.sun_color = Vec4::new(c[0], c[1], c[2], 1.0);
            m.u.atmo = atmo.map(|(c, s)| Vec4::new(c[0], c[1], c[2], s)).unwrap_or(Vec4::ZERO);
            let cloud_drift = (t / (86_400.0 * 9.0)).rem_euclid(1.0) as f32;
            m.u.params = Vec4::new(cloud_drift, 0.85, lights, if body.hydro.ocean_fraction > 0.0 { 1.0 } else { 0.0 });
        }
        for child in children.iter() {
            if let Ok(shell) = shells.get(child) {
                if let Some(m) = atmo_mats.get_mut(&shell.0) {
                    m.u.sun = sun;
                }
            }
        }
    }
}

fn surface_signature(sim: &Sim, r: BodyRef) -> u64 {
    let b = sim.universe.body(r);
    let veg = sim.universe.biosphere(r).is_some_and(|x| x.vegetated());
    let q = |x: f64, s: f64| (x * s).round() as i64 as u64;
    q(b.hydro.ocean_fraction, 40.0) ^ (q(b.hydro.ice_fraction, 40.0) << 8) ^ (q(b.temperature, 0.2) << 16) ^ ((veg as u64) << 40)
}

fn request_bakes(
    mut commands: Commands,
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    settings: Res<UserSettings>,
    q: Query<(Entity, &BodyVisual, &WorldPos), Without<PendingBake>>,
    pending: Query<(), With<PendingBake>>,
) {
    let mut in_flight = pending.iter().count();
    let max_w = settings.graphics.max_texture_width();
    // Biggest on screen first.
    let mut wanted: Vec<(f32, Entity, u8, u64, BodyRef)> = Vec::new();
    for (e, b, wp) in &q {
        let px = view.screen_radius(wp.0, b.radius);
        let level = if px > 260.0 { 3 } else if px > 70.0 { 2 } else if px > 1.0 { 1 } else { 0 };
        let sig = surface_signature(&sim, b.r);
        if level > b.level {
            wanted.push((px, e, level, sig, b.r));
        } else if b.level > 0 && sig != b.signature {
            // The simulated environment changed (oceans, ice, vegetation): repaint the world.
            wanted.push((px, e, b.level, sig, b.r));
        }
    }
    wanted.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, e, level, sig, r) in wanted {
        if in_flight >= 3 {
            break;
        }
        let body = sim.universe.body(r).clone();
        let veg = sim.universe.biosphere(r).is_some_and(|x| x.vegetated());
        let width = match level {
            1 => 256,
            2 => (max_w / 2).max(256),
            _ => max_w,
        };
        let task = AsyncComputeTaskPool::get().spawn(async move { bake::bake_surface(&body, veg, width) });
        commands.entity(e).insert(PendingBake { task, level, signature: sig });
        in_flight += 1;
    }
}

fn finish_bakes(
    mut commands: Commands,
    mut q: Query<(Entity, &mut BodyVisual, &mut PendingBake, &MeshMaterial3d<PlanetMaterial>)>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (e, mut b, mut p, mat) in &mut q {
        if let Some(baked) = block_on(future::poll_once(&mut p.task)) {
            if let Some(m) = planet_mats.get_mut(&mat.0) {
                m.albedo = images.add(image_from(baked.albedo, baked.width, baked.height, TextureFormat::Rgba8UnormSrgb));
                m.clouds = images.add(image_from(baked.clouds, baked.width, baked.height, TextureFormat::R8Unorm));
            }
            b.level = p.level;
            b.signature = p.signature;
            commands.entity(e).remove::<PendingBake>();
        }
    }
}

fn update_lights(
    sim: Res<Sim>,
    time: Res<Time>,
    view: Res<ViewInfo>,
    mut q: Query<(&mut BodyVisual, &WorldPos, &MeshMaterial3d<PlanetMaterial>)>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let now = time.elapsed_secs_f64();
    for (mut b, wp, mat) in &mut q {
        let Some(civ) = sim.universe.civ_on(b.r) else { continue };
        if now - b.lights_baked_at < 1.5 || view.screen_radius(wp.0, b.radius) < 2.0 {
            continue;
        }
        b.lights_baked_at = now;
        let w = 2048;
        let data = bake::bake_lights(civ, w);
        if let Some(m) = planet_mats.get_mut(&mat.0) {
            m.lights = images.add(image_from(data, w, w / 2, TextureFormat::R8Unorm));
        }
    }
}
