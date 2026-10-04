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
pub mod look;
pub mod materials;
pub mod overlays;
pub mod stellar;
mod sky;
pub mod terrain_lod;


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
use look::{Look, Style};
use materials::{AtmosphereMaterial, AtmosphereUniform, PlanetMaterial, PlanetUniform, RingMaterial, RingUniform, StarMaterial, StarUniform};
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
    /// The star's state when the visual was built (rebuilt when it changes).
    pub kind: cosmogon_sim::astro::star::StarKind,
}

#[derive(Component)]
pub struct BodyVisual {
    pub r: BodyRef,
    pub radius: f64,
    pub level: u8,
    pub signature: u64,
    pub lights_baked_at: f64,
    /// A star or stellar remnant (drawn by `stellar`, never hidden for being small).
    pub stellar: bool,
    /// How the body looks (chosen once per appearance; rebuilt when it changes).
    pub look: std::sync::Arc<Look>,
}

/// A body's ring system (child of the body).
#[derive(Component)]
pub struct RingVisual;

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
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/planet.wgsl");
        embedded_asset!(app, "shaders/atmosphere.wgsl");
        embedded_asset!(app, "shaders/terrain.wgsl");
        embedded_asset!(app, "shaders/common.wgsl");
        embedded_asset!(app, "shaders/ring.wgsl");
        embedded_asset!(app, "shaders/star.wgsl");
        app.add_plugins((
            MaterialPlugin::<PlanetMaterial>::default(),
            MaterialPlugin::<AtmosphereMaterial>::default(),
            MaterialPlugin::<materials::TerrainMaterial>::default(),
            MaterialPlugin::<RingMaterial>::default(),
            MaterialPlugin::<StarMaterial>::default(),
        ))
            .add_systems(Startup, load_shader_library)
            .init_resource::<ViewInfo>()
            .init_resource::<terrain_lod::TerrainLod>()
            .init_resource::<SpawnedBodies>()
            .add_systems(Startup, sky::setup_sky)
            .add_systems(OnEnter(AppState::Observing), spawn_universe)
            .add_systems(OnExit(AppState::Observing), despawn_universe)
            .add_systems(
                Update,
                (
                    (sync_body_visuals, update_world_positions, terrain_lod::position_patches, terrain_lod::sink_base_sphere).chain().in_set(Frame::Positions),
                    (terrain_lod::update_terrain_lod, apply_origin, update_planet_materials, update_star_materials, request_bakes, finish_bakes, update_lights).chain().in_set(Frame::Apply),
                )
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            )
            .add_plugins(overlays::OverlayPlugin)
            .add_plugins(stellar::StellarPlugin)
            .add_systems(
                Update,
                (
                    (stellar::sync_primaries, stellar::update_star_sizes).chain().in_set(Frame::Positions),
                    (stellar::spin_pulsars, stellar::update_compact_objects, stellar::sync_nebulae).chain().in_set(Frame::Apply).after(apply_origin),
                )
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            );
        // Created at build time: the initial state's OnEnter runs before Startup systems.
        let (s, t) = app.world().resource::<UserSettings>().graphics.sphere_segments();
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let shared = SharedMeshes { sphere: meshes.add(Sphere::new(1.0).mesh().uv(s, t)) };
        app.insert_resource(shared);
    }
}

pub fn solid_image(rgba: [u8; 4], format: TextureFormat) -> Image {
    let data = if format == TextureFormat::R8Unorm { vec![rgba[0]; 4] } else { rgba.repeat(4) };
    Image::new(Extent3d { width: 2, height: 2, depth_or_array_layers: 1 }, TextureDimension::D2, data, format, RenderAssetUsages::RENDER_WORLD)
}

/// Upload a texture with a full mip chain (box-filtered on the CPU), so distant planets
/// are smooth rather than shimmering.
pub fn image_from(data: Vec<u8>, w: u32, h: u32, format: TextureFormat) -> Image {
    let channels = if format == TextureFormat::R8Unorm { 1 } else { 4 };
    let mut all = data.clone();
    let (mut cw, mut ch) = (w as usize, h as usize);
    let mut level = data;
    let mut levels = 1;
    while cw > 1 || ch > 1 {
        let (nw, nh) = ((cw / 2).max(1), (ch / 2).max(1));
        let mut next = vec![0u8; nw * nh * channels];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..channels {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(cw - 1);
                        let sy = (y * 2 + dy).min(ch - 1);
                        sum += level[(sy * cw + sx) * channels + c] as u32;
                    }
                    next[(y * nw + x) * channels + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        all.extend_from_slice(&next);
        level = next;
        cw = nw;
        ch = nh;
        levels += 1;
    }
    let mut img = Image::new_uninit(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, format, RenderAssetUsages::RENDER_WORLD);
    img.texture_descriptor.mip_level_count = levels;
    img.data = Some(all);
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    img
}

/// Keeps the shared WGSL library (`cosmogon::common`) loaded so shaders can import it.
#[derive(Resource)]
#[allow(dead_code)]
struct ShaderLibrary(Handle<bevy::shader::Shader>);

fn load_shader_library(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ShaderLibrary(assets.load("embedded://cosmogon/render/shaders/common.wgsl")));
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

/// Which body visuals exist, and the appearance they were built for. When a sandbox edit
/// adds a body or changes how one looks, `sync_body_visuals` builds or rebuilds it.
#[derive(Resource, Default)]
pub struct SpawnedBodies(pub std::collections::HashMap<BodyRef, (Entity, u64)>);

/// Hash of everything that is baked into a body's visual at spawn time.
fn appearance_key(body: &Body) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (body.kind as u8).hash(&mut h);
    body.terrain_seed.hash(&mut h);
    body.color.iter().for_each(|c| c.to_bits().hash(&mut h));
    body.rings.is_some().hash(&mut h);
    body.removed.is_some().hash(&mut h);
    body.accretion.to_bits().hash(&mut h);
    ((body.mass.log10() * 20.0) as i64).hash(&mut h);
    if let Some((c, s)) = atmosphere_look(body) {
        c.iter().for_each(|x| x.to_bits().hash(&mut h));
        ((s * 20.0).round() as i32).hash(&mut h);
    }
    h.finish()
}

/// Scattering properties of a body's atmosphere for the shell shader (distances in body
/// radii). Vertical optical depths are physically motivated (Earth's Rayleigh depth is
/// 0.24 in blue; Mars is dust-dominated; Venus and Titan are opaque); the scale height is
/// exaggerated for visibility and the coefficients rescaled to keep those depths.
pub struct Optics {
    pub top: f32,
    pub h: f32,
    pub beta: [f32; 3],
    pub mie: f32,
    pub g: f32,
}

pub fn atmosphere_optics(body: &Body, look: &Look) -> Option<Optics> {
    let a = &body.atmosphere;
    let column = (a.pressure_bar / body.gravity_g().max(0.05)) as f32;
    let (top, tau, mie, g): (f32, [f32; 3], f32, f32) = match &look.style {
        Style::Giant(_) if body.kind == cosmogon_sim::astro::BodyKind::IceGiant => (1.03, [0.05, 0.14, 0.28], 0.15, 0.7),
        Style::Giant(_) => (1.03, [0.07, 0.10, 0.16], 0.25, 0.7),
        Style::CloudDeck { tint, .. } => (1.025, [tint[0] * 2.5, tint[1] * 2.5, tint[2] * 2.5], 2.0, 0.6),
        Style::Haze { haze } => (1.12, [haze[0] * 1.8, haze[1] * 1.1, haze[2] * 0.5], 1.2, 0.65),
        // Rock-vapour and thin remnant atmospheres over magma.
        Style::Lava { .. } => (1.02, [0.04, 0.05, 0.08], 0.08, 0.7),
        _ if a.pressure_bar < 0.003 => return None,
        _ if a.pressure_bar < 0.05 && body.hydro.ocean_fraction <= 0.0 => {
            // Thin and dusty (Mars): butterscotch by day, blue around the setting sun.
            let dust = (column / 0.006).sqrt().min(3.0);
            (1.02, [0.05 * dust, 0.032 * dust, 0.02 * dust], 0.07 * dust, 0.75)
        }
        _ => {
            let co2 = (1.0 + 1.4 * a.co2) as f32;
            let k = (column * co2).min(30.0);
            (1.02, [0.045 * k, 0.10 * k, 0.24 * k], 0.08 * k.sqrt(), 0.76)
        }
    };
    let h = (top - 1.0) * 0.22;
    let cap = |x: f32| x.min(8.0) / h;
    Some(Optics { top, h, beta: tau.map(cap), mie: cap(mie), g })
}

/// Saturn's main rings from measured radii and approximate optical depths (C, B, Cassini
/// Division, A with the Encke and Keeler gaps, F), Uranus's narrow dark ringlets, or a
/// procedural system. RGBA, `width`×1, alpha = opacity.
fn bake_ring_profile(body: &Body) -> (Vec<u8>, u32) {
    let Some(rings) = &body.rings else { return (vec![0; 4], 1) };
    let width = 2048u32;
    let (inner, outer) = (rings.inner / 1000.0, rings.outer / 1000.0);
    let mut out = Vec::with_capacity(width as usize * 4);
    for i in 0..width {
        let r_km = inner + (outer - inner) * (i as f64 + 0.5) / width as f64;
        let fine = cosmogon_sim::noise::fbm3(body.terrain_seed, [r_km / 300.0, 0.5, 0.5], 4, 2.3, 0.6) * 0.5 + 0.5;
        let micro = cosmogon_sim::noise::fbm3(body.terrain_seed ^ 7, [r_km / 40.0, 0.5, 0.5], 3, 2.0, 0.5) * 0.5 + 0.5;
        let (rgb, alpha): ([f32; 3], f64) = if body.real && body.name == "Saturn" {
            let in_ = |a: f64, b: f64| r_km >= a && r_km < b;
            if in_(74_658.0, 92_000.0) {
                ([0.55, 0.50, 0.44], 0.08 + 0.10 * fine)
            } else if in_(92_000.0, 117_580.0) {
                let x = (r_km - 92_000.0) / 25_580.0;
                ([0.80, 0.72, 0.60], (0.75 + 0.22 * (x * 3.0).min(1.0)) * (0.92 + 0.08 * fine))
            } else if in_(117_580.0, 122_170.0) {
                ([0.50, 0.47, 0.43], 0.06 + 0.06 * fine)
            } else if in_(133_410.0, 133_740.0) || in_(136_485.0, 136_527.0) {
                ([0.5, 0.5, 0.5], 0.01)
            } else if in_(122_170.0, 136_775.0) {
                ([0.72, 0.68, 0.62], 0.42 + 0.12 * fine)
            } else if in_(140_130.0, 140_230.0) {
                ([0.80, 0.78, 0.75], 0.45)
            } else {
                ([0.5, 0.5, 0.5], 0.0)
            }
        } else if body.real && body.name == "Uranus" {
            // Nine narrow ringlets; ε (outermost) is the widest.
            let ringlets = [41_837.0, 42_234.0, 42_570.0, 44_718.0, 45_661.0, 47_175.0, 47_627.0, 48_300.0, 51_149.0];
            let a = ringlets.iter().enumerate().map(|(k, c)| {
                let w = if k == 8 { 40.0 } else { 8.0 };
                (-((r_km - c) / w).powi(2)).exp()
            }).fold(0.0, f64::max);
            ([0.32, 0.31, 0.30], a * 0.7)
        } else {
            let x = (i as f64 + 0.5) / width as f64;
            let bands = fine;
            let gap = if (0.62..0.66).contains(&x) { 0.1 } else { 1.0 };
            let edge = (x * 12.0).min(1.0) * ((1.0 - x) * 20.0).min(1.0);
            let c = bake::mix3(body.color, [0.95, 0.92, 0.85], bands as f32);
            (c, bands * gap * edge * rings.opacity)
        };
        let a = (alpha * (0.85 + 0.3 * micro)).clamp(0.0, 1.0);
        let c = rgb.map(|x| ((x * (0.9 + 0.2 * micro as f32)).clamp(0.0, 1.0) * 255.0) as u8);
        out.extend_from_slice(&c);
        out.push((a * 255.0) as u8);
    }
    (out, width)
}

fn ring_image(data: Vec<u8>, w: u32) -> Image {
    let mut img = image_from(data, w, 1, TextureFormat::Rgba8UnormSrgb);
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    img
}

#[allow(clippy::too_many_arguments)]
fn spawn_body(
    commands: &mut Commands,
    r: BodyRef,
    body: &Body,
    shared: &SharedMeshes,
    meshes: &mut Assets<Mesh>,
    ring_mats: &mut Assets<RingMaterial>,
    planet_mats: &mut Assets<PlanetMaterial>,
    atmo_mats: &mut Assets<AtmosphereMaterial>,
    images: &mut Assets<Image>,
) -> Entity {
    let look = std::sync::Arc::new(Look::of(body));
    let black = images.add(solid_image([0, 0, 0, 0], TextureFormat::R8Unorm));
    let c = body.color;
    let albedo = images.add(solid_image([(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 0], TextureFormat::Rgba8UnormSrgb));
    let (ring_data, ring_w) = bake_ring_profile(body);
    let ring_tex = images.add(ring_image(ring_data, ring_w));
    let mat = planet_mats.add(PlanetMaterial { u: PlanetUniform::default(), albedo, clouds: black.clone(), lights: black.clone(), emission: black, ring: ring_tex.clone() });
    let mut e = commands.spawn((
        Mesh3d(shared.sphere.clone()),
        MeshMaterial3d(mat),
        Transform::from_scale(Vec3::splat(body.radius as f32)),
        // Hidden until positioned (otherwise it is drawn for a frame at the camera).
        Visibility::Hidden,
        WorldPos::default(),
        BodyVisual { r, radius: body.radius, level: 0, signature: 0, lights_baked_at: -10.0, stellar: false, look: look.clone() },
        SimVisual,
    ));
    e.with_children(|p| {
        if let Some(o) = atmosphere_optics(body, &look) {
            let m = atmo_mats.add(AtmosphereMaterial {
                u: AtmosphereUniform {
                    optics: Vec4::new(o.top, o.h, o.mie, o.g),
                    beta: Vec4::new(o.beta[0], o.beta[1], o.beta[2], 0.0),
                    ..default()
                },
            });
            p.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::from_scale(Vec3::splat(o.top)), AtmosphereShell(r)));
        }
        if let Some(rings) = &body.rings {
            let (inner, outer) = ((rings.inner / body.radius) as f32, (rings.outer / body.radius) as f32);
            let ring = meshes.add(ring_mesh(inner, outer));
            let m = ring_mats.add(RingMaterial { u: RingUniform { extent: Vec4::new(inner, outer, 0.0, 0.0), ..default() }, texture: ring_tex });
            p.spawn((Mesh3d(ring), MeshMaterial3d(m), Transform::default(), RingVisual));
        }
    });
    e.id()
}

/// Build visuals for bodies created in the sandbox, rebuild edited ones, drop removed ones.
#[allow(clippy::too_many_arguments)]
fn sync_body_visuals(
    mut commands: Commands,
    sim: Res<Sim>,
    shared: Res<SharedMeshes>,
    mut spawned: ResMut<SpawnedBodies>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ring_mats: ResMut<Assets<RingMaterial>>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut atmo_mats: ResMut<Assets<AtmosphereMaterial>>,
    mut images: ResMut<Assets<Image>>,
    sky: Res<SkyHandle>,
    mut star_mats: ResMut<Assets<StarMaterial>>,
    mut hole_mats: ResMut<Assets<stellar::BlackHoleMaterial>>,
    mut beam_mats: ResMut<Assets<stellar::BeamMaterial>>,
) {
    let u = &sim.universe;
    for sys in &u.systems {
        for (bi, body) in sys.bodies.iter().enumerate() {
            let r = BodyRef { system: sys.id, body: bi as u32 };
            let key = appearance_key(body);
            match spawned.0.get(&r) {
                Some((_, k)) if *k == key => continue,
                Some((e, _)) => {
                    commands.entity(*e).despawn();
                    spawned.0.remove(&r);
                }
                None => {}
            }
            if body.exists() && body.kind.is_stellar() {
                let look = std::sync::Arc::new(Look::of(body));
                let seed = (body.terrain_seed % 997) as f32;
                let e = commands
                    .spawn((
                        Transform::from_scale(Vec3::splat(body.radius as f32)),
                        Visibility::Inherited,
                        WorldPos::default(),
                        BodyVisual { r, radius: body.radius, level: 0, signature: 0, lights_baked_at: -10.0, stellar: true, look },
                        stellar::StellarBody,
                        SimVisual,
                    ))
                    .with_children(|p| {
                        stellar::spawn_stellar_children(p, body.kind, body.mass / cosmogon_sim::astro::SOLAR_MASS, body.radius, body.temperature, body.accretion, seed, &shared, &mut meshes, &mut star_mats, &mut hole_mats, &mut beam_mats, &sky.0);
                    })
                    .id();
                if body.kind == cosmogon_sim::astro::BodyKind::NeutronStar {
                    commands.entity(e).insert(stellar::PulsarSpin { period: body.rotation_period });
                }
                spawned.0.insert(r, (e, key));
            } else if body.exists() {
                let e = spawn_body(&mut commands, r, body, &shared, &mut meshes, &mut ring_mats, &mut planet_mats, &mut atmo_mats, &mut images);
                spawned.0.insert(r, (e, key));
            }
        }
    }
    // Bodies that vanished from the universe entirely (undo of an addition).
    let stale: Vec<BodyRef> = spawned.0.keys().filter(|r| u.systems.get(r.system as usize).is_none_or(|s| s.bodies.len() <= r.body as usize)).copied().collect();
    for r in stale {
        if let Some((e, _)) = spawned.0.remove(&r) {
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_universe(
    mut commands: Commands,
    sim: Res<Sim>,
    shared: Res<SharedMeshes>,
    sky: Res<SkyHandle>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut star_mats: ResMut<Assets<StarMaterial>>,
    mut hole_mats: ResMut<Assets<stellar::BlackHoleMaterial>>,
    mut beam_mats: ResMut<Assets<stellar::BeamMaterial>>,
    mut spawned: ResMut<SpawnedBodies>,
) {
    let u = &sim.universe;
    let t = u.time;
    spawned.0.clear();
    for sys in &u.systems {
        let stars = std::iter::once((&sys.star, false)).chain(sys.companion.as_ref().map(|c| (&c.star, true)));
        for (star, companion) in stars {
            stellar::spawn_primary(&mut commands, sys.id, companion, star, t, &shared, &mut meshes, &mut star_mats, &mut hole_mats, &mut beam_mats, &sky.0);
        }
    }
    // Bodies are built by `sync_body_visuals` (also used when the sandbox changes).
    info!("spawned visuals for {} systems", u.systems.len());
}

/// Photosphere parameters: colour from temperature, limb darkening (stronger for cooler
/// stars, ~0.6 for the Sun in visible light), and spot coverage rising for cool, active stars.
fn star_uniform(star: &cosmogon_sim::astro::Star, t: f64, seed: f32) -> StarUniform {
    let c = star.color(t);
    let lum = (star.luminosity(t).max(1e-4)).powf(0.15) as f32;
    let k = 60.0 * lum;
    let teff = star.temperature_at(t);
    let limb = (0.85 - (teff as f32 - 3500.0) / 12000.0).clamp(0.35, 0.85);
    // Starspots on cool dwarfs; giants show giant convection cells instead.
    let giant = star.phase(t) == cosmogon_sim::astro::star::StellarPhase::Giant;
    let spots = if giant { 0.0 } else { ((6200.0 - teff as f32) / 3000.0).clamp(0.0, 0.9) * 0.6 + 0.1 * star.flare_activity_at(t) as f32 };
    // Convection cells: tens of thousands of small granules on the Sun, a few giant cells
    // on red supergiants (Betelgeuse shows only a handful).
    let r_sun = star.current_radius(t) / cosmogon_sim::astro::SOLAR_RADIUS;
    let cells = (60.0 * r_sun.max(0.01).powf(-0.35)).clamp(4.0, 90.0) as f32;
    StarUniform { color: Vec4::new(c[0] * k, c[1] * k, c[2] * k, 1.0), params: Vec4::new(0.0, spots, limb, seed), center: Vec4::new(0.0, 0.0, 0.0, cells) }
}

fn update_star_materials(sim: Res<Sim>, time: Res<Time>, view: Res<ViewInfo>, q: Query<(&StarVisual, &WorldPos, &MeshMaterial3d<StarMaterial>)>, mut mats: ResMut<Assets<StarMaterial>>) {
    let u = &sim.universe;
    for (s, wp, m) in &q {
        if let Some(mat) = mats.get_mut(&m.0) {
            let sys = u.system(s.system);
            let star = if s.companion { sys.companion.as_ref().map(|c| &c.star).unwrap_or(&sys.star) } else { &sys.star };
            let seed = mat.u.params.w;
            mat.u = star_uniform(star, u.time, seed);
            // A point of light needs to be very bright for the glare; a resolved disc is
            // dimmed towards the display range so limb darkening and granulation show.
            // A resolved disc shows its *surface* brightness (cooler stars are dimmer per
            // area, compressed as T² so red giants read as glowing orange-red).
            let px = view.screen_radius(wp.0, s.radius);
            let resolved = ((px - 4.0) / 120.0).clamp(0.0, 1.0);
            let point = mat.u.color.truncate();
            let teff = star.temperature_at(u.time);
            let c = star.color(u.time);
            // Saturate the blackbody tint (the tonemapper desaturates bright light).
            let sat = Vec3::new(c[0] * c[0], c[1] * c[1], c[2] * c[2]);
            let sat = sat / sat.max_element().max(1e-3);
            let disc = sat * (4.2 * (teff / 5772.0).powi(2)) as f32;
            mat.u.color = point.lerp(disc, resolved).extend(1.0);
            mat.u.params.x = (time.elapsed_secs_f64() % 100_000.0) as f32;
        }
    }
}

fn despawn_universe(mut commands: Commands, q: Query<Entity, With<SimVisual>>, mut lod: ResMut<terrain_lod::TerrainLod>, mut spawned: ResMut<SpawnedBodies>) {
    spawned.0.clear();
    *lod = terrain_lod::TerrainLod::default();
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Where a civilization's colony stands on a world (deterministic; lat, lon in radians):
/// low latitudes, away from the poles.
pub fn colony_site(civ: u32, body: u32) -> (f64, f64) {
    let h = cosmogon_sim::rng::mix(civ as u64 * 0x9E37 + 1, body as u64);
    let f = |s: u32| ((h >> s) & 0xFFFF) as f64 / 65535.0;
    ((f(0) - 0.5) * 0.9, f(16) * std::f64::consts::TAU)
}

/// Orientation of a body: axial tilt and spin (tidally locked bodies face their primary).
/// The spin comes from the simulation so impacts land where they are drawn.
pub fn body_rotation(sim: &Sim, r: BodyRef, t: f64) -> Quat {
    let sys = sim.universe.system(r.system);
    let b = &sys.bodies[r.body as usize];
    let spin = sys.spin_angle(r.body as usize, t);
    sim_to_render_rot() * Quat::from_rotation_x(b.axial_tilt as f32) * Quat::from_rotation_z(spin as f32)
}

fn update_world_positions(sim: Res<Sim>, mut stars: Query<(&StarVisual, &mut WorldPos), Without<BodyVisual>>, mut bodies: Query<(&mut BodyVisual, &mut WorldPos, &mut Transform), Without<StarVisual>>) {
    let u = &sim.universe;
    let t = u.time;
    for (s, mut wp) in &mut stars {
        let sys = u.system(s.system);
        wp.0 = to_render(if s.companion { sys.companion_position(t).unwrap_or(sys.position) } else { sys.star_position(t) });
    }
    for (mut b, mut wp, mut tf) in &mut bodies {
        let Some(body) = u.systems.get(b.r.system as usize).and_then(|s| s.bodies.get(b.r.body as usize)) else { continue };
        wp.0 = to_render(u.body_position(b.r, t));
        tf.rotation = body_rotation(&sim, b.r, t);
        // Radius can change in a sandbox (edits, mergers).
        if b.radius != body.radius {
            b.radius = body.radius;
            tf.scale = Vec3::splat(body.radius as f32);
        }
    }
}

fn apply_origin(view: Res<ViewInfo>, mut q: Query<(&WorldPos, &mut Transform, Option<&StarVisual>, Option<&BodyVisual>, &mut Visibility)>) {
    for (wp, mut tf, star, body, mut vis) in &mut q {
        let rel = wp.0 - view.origin;
        tf.translation = rel.as_vec3();
        if let Some(s) = star {
            use cosmogon_sim::astro::star::StarKind;
            // Keep luminous stars visible as bright points at any distance (bloom does the
            // rest); black holes and neutron stars are drawn at their true size.
            let d = rel.length();
            tf.scale = Vec3::splat(match s.kind {
                StarKind::Normal | StarKind::WhiteDwarf => s.radius.max(d * 0.0018),
                _ => s.radius,
            } as f32);
        }
        if let Some(b) = body.filter(|b| !b.stellar) {
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

/// Up to four bodies that can cast shadows on body `i` (its parent, siblings and moons),
/// as offsets / radius relative to it, preferring the ones looking biggest towards the star.
fn occluders(sys: &cosmogon_sim::astro::StarSystem, i: usize, t: f64, to_star: DVec3) -> [Vec4; 4] {
    let body = &sys.bodies[i];
    let center = to_render(sys.body_position(i, t));
    let l = to_star.normalize();
    let mut cands: Vec<(f64, Vec4)> = sys
        .bodies
        .iter()
        .enumerate()
        .filter(|(j, o)| *j != i && o.exists() && (o.parent == Some(i as u32) || body.parent == Some(*j as u32) || (body.parent.is_some() && o.parent == body.parent)))
        .filter_map(|(j, o)| {
            let off = to_render(sys.body_position(j, t)) - center;
            let along = off.dot(l);
            (along > 0.0).then(|| (o.radius / along.max(1.0), (off / body.radius).as_vec3().extend((o.radius / body.radius) as f32)))
        })
        .collect();
    cands.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = [Vec4::ZERO; 4];
    for (k, c) in cands.into_iter().take(4).enumerate() {
        out[k] = c.1;
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn update_planet_materials(
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    bodies: Query<(&BodyVisual, &WorldPos, &Transform, &MeshMaterial3d<PlanetMaterial>, Option<&Children>)>,
    shells: Query<&MeshMaterial3d<AtmosphereMaterial>>,
    rings: Query<&MeshMaterial3d<RingMaterial>>,
    stars: Query<(&StarVisual, &WorldPos)>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut atmo_mats: ResMut<Assets<AtmosphereMaterial>>,
    mut ring_mats: ResMut<Assets<RingMaterial>>,
) {
    let u = &sim.universe;
    let t = u.time;
    for (b, wp, tf, mat, children) in &bodies {
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
        let sun_color = Vec4::new(c[0], c[1], c[2], (sys.star.current_radius(t) / dist.max(1.0)) as f32);
        let body = u.body(b.r);
        let civ = u.civ_on(b.r);
        let colony = u.civs.iter().any(|c| c.is_alive() && c.system == b.r.system && c.colonies.iter().any(|col| col.body == b.r.body));
        let lights = civ.map(|c| if c.flags.contains("electric_light") { 4.0 } else { 1.2 }).unwrap_or(if colony { 4.0 } else { 0.0 });
        let atmo = atmosphere_look(body);
        let center = (wp.0 - view.origin).as_vec3().extend(b.radius as f32);
        let occ = occluders(sys, b.r.body as usize, t, to_star);
        let look = &b.look;
        let style = match look.style {
            Style::Giant(_) => 1.0,
            Style::CloudDeck { .. } | Style::Haze { .. } => 2.0,
            _ => 0.0,
        };
        let emission_strength = match look.style {
            Style::Lava { .. } => 1.0,
            Style::Volcanic => 1.5,
            Style::Giant(_) => 1.5 * look.night_glow,
            _ => 0.0,
        };
        let ring_normal = (tf.rotation * Vec3::Z).extend(0.0);
        let (ring, ring_extent) = match &body.rings {
            Some(r) => {
                let (i, o) = ((r.inner / body.radius) as f32, (r.outer / body.radius) as f32);
                (Vec4::new(i, o, r.opacity as f32, 1.0), Vec4::new(i, o, 0.0, 0.0))
            }
            None => (Vec4::ZERO, Vec4::ZERO),
        };
        let q = tf.rotation.inverse();
        if let Some(m) = planet_mats.get_mut(&mat.0) {
            m.u.sun = sun;
            m.u.sun_color = sun_color;
            m.u.atmo = atmo.map(|(c, s)| Vec4::new(c[0], c[1], c[2], s)).unwrap_or(Vec4::ZERO);
            let cloud_drift = (t / (86_400.0 * 9.0)).rem_euclid(1.0) as f32;
            m.u.params = Vec4::new(cloud_drift, 0.85, lights, if body.hydro.ocean_fraction > 0.0 { 1.0 } else { 0.0 });
            m.u.emission = Vec4::new(look.emission[0], look.emission[1], look.emission[2], emission_strength);
            let flow = ((t / 86_400.0 * 0.05).rem_euclid(5000.0)) as f32;
            m.u.look = Vec4::new(style, 1.0, flow, (body.terrain_seed % 1000) as f32);
            m.u.center = center;
            m.u.ring = ring;
            m.u.ring_normal = ring_normal;
            m.u.orient = Vec4::new(q.x, q.y, q.z, q.w);
            m.u.occluders = occ;
            m.u.heat = match body.melt {
                Some(melt) => Vec4::new(melt.glow(t) as f32, melt.temperature(t) as f32, 0.0, 0.0),
                None => Vec4::ZERO,
            };
        }
        for child in children.into_iter().flat_map(|c| c.iter()) {
            if let Ok(shell) = shells.get(child) {
                if let Some(m) = atmo_mats.get_mut(&shell.0) {
                    m.u.sun = sun;
                    m.u.sun_color = sun_color;
                    m.u.center = center;
                    m.u.occluders = occ;
                }
            }
            if let Ok(rm) = rings.get(child) {
                if let Some(m) = ring_mats.get_mut(&rm.0) {
                    m.u.sun = sun;
                    m.u.sun_color = sun_color;
                    m.u.center = center;
                    m.u.normal = ring_normal;
                    m.u.extent = ring_extent;
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
    q: Query<(Entity, &BodyVisual, &WorldPos), (Without<PendingBake>, Without<stellar::StellarBody>)>,
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
                m.emission = images.add(image_from(baked.emission, baked.width, baked.height, TextureFormat::R8Unorm));
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
        let u = &sim.universe;
        let civ = u.civ_on(b.r);
        let outposts: Vec<(f64, f64, f64)> = u
            .civs
            .iter()
            .filter(|c| c.is_alive() && c.system == b.r.system)
            .flat_map(|c| c.colonies.iter().filter(|col| col.body == b.r.body).map(move |col| (c.id, col)))
            .map(|(id, col)| {
                let (lat, lon) = colony_site(id, col.body);
                (lat, lon, col.population)
            })
            .collect();
        if civ.is_none() && outposts.is_empty() {
            continue;
        }
        if now - b.lights_baked_at < 1.5 || view.screen_radius(wp.0, b.radius) < 2.0 {
            continue;
        }
        b.lights_baked_at = now;
        let w = 2048;
        let data = bake::bake_lights_with(civ, &outposts, w);
        if let Some(m) = planet_mats.get_mut(&mat.0) {
            m.lights = images.add(image_from(data, w, w / 2, TextureFormat::R8Unorm));
        }
    }
}
