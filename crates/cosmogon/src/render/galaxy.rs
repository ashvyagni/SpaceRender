//! The universe beyond the stars: the Milky Way seen from outside, the Local Group, and a
//! deep field of galaxies and quasars along a cosmic web.
//!
//! Real geometry: the Sun lies 26 700 ly from the Galactic Centre (GRAVITY Collaboration
//! 2019), 65 ly above the plane; the centre and north Galactic pole are placed at their
//! J2000 equatorial coordinates and converted to the simulation's ecliptic frame. The
//! Milky Way is a barred spiral (bar ≈ 16 000 ly, four arms, pitch ≈ 12–17°). Andromeda,
//! Triangulum and the Magellanic Clouds sit at their catalogued positions and distances.
//! Distant galaxies and quasars are procedural (no catalogue is used beyond the Local Group).
//! These layers fade in as the camera pulls back past a few thousand light-years, while the
//! painted sky (which shows the Milky Way from inside) fades out.

use bevy::asset::RenderAssetUsages;
use bevy::math::DVec3;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cosmogon_sim::astro::LIGHT_YEAR;
use cosmogon_sim::Vec3d;

use super::{to_render, SharedMeshes, SimVisual, ViewInfo};
use crate::sim::Sim;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct GalaxyUniform {
    pub center: Vec4,
    pub orient: Vec4,
    pub shape: Vec4,
    pub look: Vec4,
    pub extra: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GalaxyMaterial {
    #[uniform(0)]
    pub u: GalaxyUniform,
}

impl Material for GalaxyMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/galaxy.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(_: &MaterialPipeline, d: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(ds) = d.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct DeepUniform {
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct DeepFieldMaterial {
    #[uniform(0)]
    pub u: DeepUniform,
}

impl Material for DeepFieldMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/deep_field.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(_: &MaterialPipeline, d: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(ds) = d.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}

/// A galaxy placed relative to the Sun (simulation frame, light-years).
#[derive(Component)]
pub struct GalaxyVisual {
    pub offset_ly: Vec3d,
    pub radius_ly: f64,
    /// The Milky Way (hidden from inside).
    pub home: bool,
    pub name: &'static str,
}

#[derive(Component)]
pub struct DeepField;

pub struct GalaxyPlugin;

impl Plugin for GalaxyPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shaders/galaxy.wgsl");
        bevy::asset::embedded_asset!(app, "shaders/deep_field.wgsl");
        app.add_plugins((MaterialPlugin::<GalaxyMaterial>::default(), MaterialPlugin::<DeepFieldMaterial>::default()));
    }
}

// ── Sky coordinates ──────────────────────────────────────────────────────

const OBLIQUITY_DEG: f64 = 23.439_291;

/// Unit vector in the simulation frame (ecliptic J2000) from equatorial RA/Dec (degrees).
pub fn eq_to_sim(ra_deg: f64, dec_deg: f64) -> Vec3d {
    let (ra, dec) = (ra_deg.to_radians(), dec_deg.to_radians());
    let (x, y, z) = (dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin());
    let e = OBLIQUITY_DEG.to_radians();
    Vec3d::new(x, y * e.cos() + z * e.sin(), -y * e.sin() + z * e.cos())
}

/// Galactic frame in simulation coordinates: (towards the centre, in-plane, north pole).
pub fn galactic_frame() -> (Vec3d, Vec3d, Vec3d) {
    let gc = eq_to_sim(266.405, -28.936);
    let ngp = eq_to_sim(192.859, 27.128);
    let north = (ngp - gc * gc.dot(ngp)).normalize();
    let side = north.cross(gc).normalize();
    (gc, side, north)
}

/// Distance from the Sun to the Galactic Centre (ly).
pub const SUN_TO_CENTRE_LY: f64 = 26_700.0;

/// The Galactic Centre relative to the Sun (simulation frame, light-years).
pub fn galactic_centre_offset() -> Vec3d {
    let (gc, _, north) = galactic_frame();
    gc * SUN_TO_CENTRE_LY - north * 65.0
}

fn quat_world_to_frame(x: DVec3, y: DVec3, z: DVec3) -> Quat {
    // Rows of the world→frame matrix are the frame axes.
    let m = Mat3::from_cols(x.as_vec3(), y.as_vec3(), z.as_vec3()).transpose();
    Quat::from_mat3(&m)
}

/// The galaxy's own frame in render coordinates: x from the centre towards the Sun's side,
/// z the rotation (north) axis. Returns (world→galaxy rotation).
fn milky_way_orientation() -> Quat {
    let (gc, side, north) = galactic_frame();
    let x = to_render(-gc).normalize();
    let z = to_render(north).normalize();
    let y = z.cross(x).normalize();
    let _ = side;
    quat_world_to_frame(x, y, z)
}

/// Orientation for an external disk galaxy from its inclination and position angle.
fn disk_orientation(dir: Vec3d, incl_deg: f64, pa_deg: f64) -> Quat {
    let los = dir.normalize();
    let pole = eq_to_sim(0.0, 90.0);
    let north = (pole - los * los.dot(pole)).normalize();
    let east = north.cross(los).normalize();
    let (pa, i) = (pa_deg.to_radians(), incl_deg.to_radians());
    let major = north * pa.cos() + east * pa.sin();
    let minor = major.cross(los).normalize();
    let normal = (los * -i.cos() + minor * i.sin()).normalize();
    let z = to_render(normal).normalize();
    let x = to_render(major).normalize();
    let y = z.cross(x).normalize();
    quat_world_to_frame(x, y, z)
}

struct Known {
    name: &'static str,
    ra: f64,
    dec: f64,
    dist_ly: f64,
    radius_ly: f64,
    incl: f64,
    pa: f64,
    arms: f32,
    irregular: bool,
    brightness: f32,
}

/// Local Group members (NED / McConnachie 2012 distances).
const LOCAL_GROUP: &[Known] = &[
    Known { name: "Andromeda (M31)", ra: 10.6847, dec: 41.2690, dist_ly: 2.537e6, radius_ly: 110_000.0, incl: 77.0, pa: 38.0, arms: 2.0, irregular: false, brightness: 1.3 },
    Known { name: "Triangulum (M33)", ra: 23.4621, dec: 30.6599, dist_ly: 2.73e6, radius_ly: 30_000.0, incl: 54.0, pa: 22.0, arms: 2.0, irregular: false, brightness: 0.8 },
    Known { name: "Large Magellanic Cloud", ra: 80.894, dec: -69.756, dist_ly: 163_000.0, radius_ly: 16_000.0, incl: 35.0, pa: 170.0, arms: 1.0, irregular: true, brightness: 0.9 },
    Known { name: "Small Magellanic Cloud", ra: 13.187, dec: -72.829, dist_ly: 200_000.0, radius_ly: 9_500.0, incl: 64.0, pa: 45.0, arms: 0.0, irregular: true, brightness: 0.7 },
];

/// Spawn the galaxy layers (once per universe).
pub fn spawn_galaxies(commands: &mut Commands, shared: &SharedMeshes, meshes: &mut Assets<Mesh>, gal_mats: &mut Assets<GalaxyMaterial>, deep_mats: &mut Assets<DeepFieldMaterial>) {
    // The Milky Way.
    // (Placement comes from `galactic_centre_offset`.)
    let mw_offset = galactic_centre_offset();
    let m = gal_mats.add(GalaxyMaterial {
        u: GalaxyUniform {
            orient: milky_way_orientation().into(),
            shape: Vec4::new(60_000.0, 11_000.0, 4.0, 17f32.to_radians()),
            look: Vec4::new(16_000.0, 4_500.0, 1.0, 0.0),
            extra: Vec4::new(0.5, 3.7, 900.0, 0.0),
            ..default()
        },
    });
    commands.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden, GalaxyVisual { offset_ly: mw_offset, radius_ly: 60_000.0, home: true, name: "Milky Way" }, SimVisual));
    for (k, g) in LOCAL_GROUP.iter().enumerate() {
        let dir = eq_to_sim(g.ra, g.dec);
        let m = gal_mats.add(GalaxyMaterial {
            u: GalaxyUniform {
                orient: disk_orientation(dir, g.incl, g.pa).into(),
                shape: Vec4::new(g.radius_ly as f32, (g.radius_ly * 0.22) as f32, g.arms, 14f32.to_radians()),
                look: Vec4::new(if g.irregular { 0.0 } else { (g.radius_ly * 0.15) as f32 }, (g.radius_ly * 0.08) as f32, g.brightness, if g.irregular { 1.0 } else { 0.0 }),
                extra: Vec4::new(k as f32 * 1.3, 11.0 + k as f32 * 5.0, (g.radius_ly * 0.02).max(300.0) as f32, 0.0),
                ..default()
            },
        });
        commands.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden, GalaxyVisual { offset_ly: dir * g.dist_ly, radius_ly: g.radius_ly, home: false, name: g.name }, SimVisual));
    }
    // The deep field.
    let mesh = meshes.add(deep_field_mesh(0xC05_3EB));
    let dm = deep_mats.add(DeepFieldMaterial { u: DeepUniform { params: Vec4::ZERO } });
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(dm), Transform::default(), Visibility::Hidden, DeepField, SimVisual));
}

/// Thousands of galaxies (and ~200 quasars) out to ~3 Gly, accepted where a 3D noise
/// field is high so they trace filaments and walls; positions in metres from the Sun.
fn deep_field_mesh(seed: u64) -> Mesh {
    let mut rng = cosmogon_sim::rng::Rng::new(seed);
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    let mut col = Vec::new();
    let mut idx = Vec::new();
    let mut add = |c: DVec3, size: f64, kind: f32, tint: [f32; 3], rng: &mut cosmogon_sim::rng::Rng| {
        let a = DVec3::new(rng.normal(0.0, 1.0), rng.normal(0.0, 1.0), rng.normal(0.0, 1.0)).normalize();
        let b = a.any_orthonormal_vector();
        let b2 = a.cross(b);
        let base = pos.len() as u32;
        for (u, v) in [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            let p = c + (b * (u as f64 - 0.5) + b2 * (v as f64 - 0.5)) * size * 2.0;
            pos.push(p.as_vec3().to_array());
            uv.push([u, v]);
            col.push([tint[0], tint[1], tint[2], kind]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    let mut placed = 0;
    let mut tries = 0;
    while placed < 5000 && tries < 200_000 {
        tries += 1;
        let d = 6.0e6 * (500.0f64).powf(rng.f64()); // 6 Mly – 3 Gly, log-uniform
        let dir = DVec3::new(rng.normal(0.0, 1.0), rng.normal(0.0, 1.0), rng.normal(0.0, 1.0)).normalize();
        let p = dir * d;
        let web = cosmogon_sim::noise::fbm3(seed, [p.x / 6.0e7, p.y / 6.0e7, p.z / 6.0e7], 4, 2.0, 0.5);
        if web < 0.05 && rng.f64() > 0.15 {
            continue;
        }
        let quasar = d > 3.0e8 && rng.chance(0.04);
        let (size_ly, kind, tint) = if quasar {
            (8_000.0, 0.9, [0.7, 0.85, 1.0])
        } else if rng.chance(0.4) {
            (rng.range(30_000.0, 150_000.0), rng.range(0.0, 0.3) as f32, [1.0, 0.82, 0.6])
        } else {
            (rng.range(25_000.0, 110_000.0), rng.range(0.34, 0.65) as f32, [0.62, 0.72, 1.0])
        };
        add(p * LIGHT_YEAR, size_ly * LIGHT_YEAR, kind, tint, &mut rng);
        placed += 1;
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx))
}

/// Place and fade the layers for the current camera; dim the painted sky when outside.
#[allow(clippy::type_complexity)]
pub fn update_galaxies(
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    mut gals: Query<(&GalaxyVisual, &MeshMaterial3d<GalaxyMaterial>, &mut Transform, &mut Visibility), Without<DeepField>>,
    mut deep: Query<(&MeshMaterial3d<DeepFieldMaterial>, &mut Transform, &mut Visibility), With<DeepField>>,
    mut gal_mats: ResMut<Assets<GalaxyMaterial>>,
    mut deep_mats: ResMut<Assets<DeepFieldMaterial>>,
    mut skybox: Query<&mut bevy::core_pipeline::Skybox>,
) {
    let u = &sim.universe;
    let sun = to_render(u.systems[0].position);
    let cam_ly = (view.origin - sun).length() / LIGHT_YEAR;
    // From inside the disk the painted sky *is* the Milky Way; outside it fades away.
    let outside = ((cam_ly - 4_000.0) / 30_000.0).clamp(0.0, 1.0);
    for mut sb in &mut skybox {
        sb.brightness = 300.0 * (1.0 - outside as f32 * 0.97);
    }
    for (g, m, mut tf, mut vis) in &mut gals {
        let center = sun + to_render(g.offset_ly * LIGHT_YEAR);
        let rel = center - view.origin;
        let fade = if g.home { outside } else { ((cam_ly - 2_000.0) / 20_000.0).clamp(0.0, 1.0) };
        let want = if fade > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        if fade <= 0.0 {
            continue;
        }
        tf.translation = rel.as_vec3();
        let r = (g.radius_ly * LIGHT_YEAR) as f32;
        tf.scale = Vec3::splat(r * 1.05);
        if let Some(mat) = gal_mats.get_mut(&m.0) {
            mat.u.center = rel.as_vec3().extend(LIGHT_YEAR as f32);
            mat.u.look.z = base_brightness(g.name) * fade as f32;
        }
    }
    let deep_fade = ((cam_ly - 3.0e5) / 3.0e6).clamp(0.0, 1.0) as f32;
    for (m, mut tf, mut vis) in &mut deep {
        let want = if deep_fade > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        tf.translation = (sun - view.origin).as_vec3();
        if let Some(mat) = deep_mats.get_mut(&m.0) {
            mat.u.params = Vec4::new(deep_fade * 2.0, 0.0, 0.0, 0.0);
        }
    }
}

fn base_brightness(name: &str) -> f32 {
    LOCAL_GROUP.iter().find(|g| g.name == name).map(|g| g.brightness).unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galactic_centre_is_towards_sagittarius() {
        let (gc, _, north) = galactic_frame();
        // Sagittarius is just south of the ecliptic, near ecliptic longitude 266.8°.
        let lon = gc.y.atan2(gc.x).to_degrees().rem_euclid(360.0);
        let lat = gc.z.asin().to_degrees();
        assert!((lon - 266.8).abs() < 0.5 && (lat + 5.6).abs() < 0.5, "lon {lon} lat {lat}");
        assert!(gc.dot(north).abs() < 1e-9);
    }
}

#[cfg(test)]
mod intersect_tests {
    use super::*;

    #[test]
    fn a_ray_at_the_centre_crosses_the_disk() {
        let q = milky_way_orientation();
        let (gc, _, north) = galactic_frame();
        let centre = to_render(gc * SUN_TO_CENTRE_LY);
        // Camera 100 000 ly above the Sun along the galactic pole.
        let cam_world = to_render(north * 100_000.0);
        let cam = q * (cam_world - centre).as_vec3();
        let dir = (q * (centre - cam_world).as_vec3()).normalize();
        println!("cam {cam:?} dir {dir:?} |q| {}", q.length());
        assert!((cam.x.hypot(cam.y)) < 30_000.0, "camera should be above the disk: {cam:?}");
        assert!(dir.z < -0.9);
    }
}
