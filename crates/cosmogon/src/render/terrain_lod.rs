//! Close-up planetary terrain: a cube-sphere quadtree of displaced, vertex-coloured patches.
//!
//! When the camera focuses on a solid world and gets close, the textured sphere is
//! supplemented by terrain patches that refine with distance (down to tiles ~20 m apart).
//! Patch meshes are generated on worker threads from the *same* terrain function the
//! simulation uses, plus extra procedural octaves for sub-kilometre detail. Patch positions
//! are kept in f64 and expressed relative to the camera like everything else, so there is
//! no jitter at ground level. Skirts hide cracks between levels.

use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::math::{DQuat, DVec3};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use cosmogon_sim::astro::Body;
use cosmogon_sim::noise::{fbm3, ridged3};
use cosmogon_sim::planet::terrain::{SurfaceContext, METRES_PER_UNIT};
use cosmogon_sim::BodyRef;

use super::bake::surface_color;
use super::materials::{PlanetMaterial, TerrainMaterial};
use super::{body_rotation, to_render, BodyVisual, ViewInfo, WorldPos};
use crate::camera::CameraRig;
use crate::persistence::UserSettings;
use crate::sim::{Sim, Target};

/// Vertices per patch edge (without skirt).
const N: usize = 33;
const MAX_LEVEL: u8 = 17;
/// Activate when the focused world is at least this many pixels in radius.
const ACTIVATE_PX: f32 = 280.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey {
    pub face: u8,
    pub level: u8,
    pub x: u32,
    pub y: u32,
}

impl NodeKey {
    fn children(self) -> [NodeKey; 4] {
        let l = self.level + 1;
        let (x, y) = (self.x * 2, self.y * 2);
        [
            NodeKey { face: self.face, level: l, x, y },
            NodeKey { face: self.face, level: l, x: x + 1, y },
            NodeKey { face: self.face, level: l, x, y: y + 1 },
            NodeKey { face: self.face, level: l, x: x + 1, y: y + 1 },
        ]
    }
    /// Face-space bounds (s0, t0, size) in [-1, 1].
    fn bounds(self) -> (f64, f64, f64) {
        let size = 2.0 / (1u64 << self.level) as f64;
        (-1.0 + self.x as f64 * size, -1.0 + self.y as f64 * size, size)
    }
    fn center_dir(self) -> DVec3 {
        let (s0, t0, size) = self.bounds();
        face_dir(self.face, s0 + size * 0.5, t0 + size * 0.5)
    }
    /// Approximate angular half-size (radians).
    fn angular_radius(self) -> f64 {
        std::f64::consts::FRAC_PI_4 * 1.5 / (1u64 << self.level) as f64
    }
}

/// Cube-sphere mapping with tangent warping for near-uniform cells. +Z is the body's pole,
/// matching the UV sphere and the simulation's lat/lon convention.
pub fn face_dir(face: u8, s: f64, t: f64) -> DVec3 {
    let (n, u, v) = match face {
        0 => (DVec3::X, DVec3::Y, DVec3::Z),
        1 => (DVec3::NEG_X, DVec3::NEG_Y, DVec3::Z),
        2 => (DVec3::Y, DVec3::NEG_X, DVec3::Z),
        3 => (DVec3::NEG_Y, DVec3::X, DVec3::Z),
        4 => (DVec3::Z, DVec3::X, DVec3::Y),
        _ => (DVec3::NEG_Z, DVec3::X, DVec3::NEG_Y),
    };
    let q = std::f64::consts::FRAC_PI_4;
    (n + u * (s * q).tan() + v * (t * q).tan()).normalize()
}

/// Everything needed to compute surface height and colour off the main thread.
#[derive(Clone)]
pub struct SurfaceModel {
    pub body: Body,
    pub ctx: SurfaceContext,
    pub metres_per_unit: f64,
    pub has_ocean: bool,
}

impl SurfaceModel {
    pub fn new(body: &Body, vegetated: bool) -> Self {
        let mpu = if body.elevation_data.is_some() { METRES_PER_UNIT } else { METRES_PER_UNIT * (1.0 / body.gravity_g().max(0.05)).sqrt().min(4.0) };
        Self { body: body.clone(), ctx: SurfaceContext::new(body, vegetated), metres_per_unit: mpu, has_ocean: body.hydro.ocean_fraction > 0.0 }
    }

    /// Close-up detail (terrain units): regional hills, ridged ranges in high country,
    /// fine roughness, and impact craters on airless worlds. Visual only — the simulation
    /// reasons with the coarser authoritative terrain.
    fn detail(&self, d: [f64; 3], height: f64) -> f64 {
        let seed = self.body.terrain_seed;
        let p = |k: f64| [d[0] * k, d[1] * k, d[2] * k];
        let rough = 0.3 + height.clamp(0.0, 1.5) * 1.2;
        let hills = fbm3(seed ^ 0xD1, p(300.0), 7, 2.05, 0.5) * 0.10;
        let ranges = ridged3(seed ^ 0xD3, p(900.0), 5) * 0.12 * height.clamp(0.0, 1.0);
        let fine = fbm3(seed ^ 0xD2, p(30_000.0), 5, 2.1, 0.5) * 0.006;
        let mut h = (hills + fine) * rough + ranges;
        if !self.body.atmosphere.is_present() || self.body.atmosphere.pressure_bar < 0.01 {
            for (k, freq, depth) in [(0u64, 40.0, 0.25), (1, 160.0, 0.08), (2, 700.0, 0.025)] {
                h += crater_field(seed ^ (0xC7A7 + k), d, freq) * depth;
            }
        }
        h
    }

    /// Radius (m) of the visible surface along unit direction `d`, plus colour and water flag.
    pub fn sample(&self, d: [f64; 3]) -> (f64, [f32; 4]) {
        let s = self.ctx.sample(d);
        let h = s.height + self.detail(d, s.height);
        let water = self.has_ocean && h < 0.0;
        let r = self.body.radius + if water { 0.0 } else { h * self.metres_per_unit };
        let (rgb, _) = surface_color(self.body.kind, &self.ctx, self.body.color, self.body.terrain_seed, d);
        let lin = rgb.map(|c| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) });
        (r, [lin[0], lin[1], lin[2], if water { 1.0 } else { 0.0 }])
    }

    pub fn ground_radius(&self, d: [f64; 3]) -> f64 {
        let s = self.ctx.sample(d);
        let h = s.height + self.detail(d, s.height);
        self.body.radius + if self.has_ocean && h < 0.0 { 0.0 } else { h * self.metres_per_unit }
    }
}

/// Impact craters from a jittered 3D lattice: each cell may hold one crater (bowl floor,
/// raised rim, fading ejecta). Returns a height offset in units of crater depth.
fn crater_field(seed: u64, d: [f64; 3], freq: f64) -> f64 {
    let p = [d[0] * freq, d[1] * freq, d[2] * freq];
    let c = [p[0].floor() as i64, p[1].floor() as i64, p[2].floor() as i64];
    let mut h = 0.0;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cell = [c[0] + dx, c[1] + dy, c[2] + dz];
                let mut r = cosmogon_sim::rng::Rng::stream(seed, 0, &[cell[0] as u64, cell[1] as u64, cell[2] as u64]);
                if !r.chance(0.55) {
                    continue;
                }
                let center = [cell[0] as f64 + r.f64(), cell[1] as f64 + r.f64(), cell[2] as f64 + r.f64()];
                let radius = 0.15 + 0.4 * r.f64().powf(2.0);
                let dist = ((p[0] - center[0]).powi(2) + (p[1] - center[1]).powi(2) + (p[2] - center[2]).powi(2)).sqrt() / radius;
                if dist < 1.0 {
                    h += -(1.0 - dist * dist) + 0.35 * dist.powi(6);
                } else if dist < 2.0 {
                    let t = (dist - 1.0) / 1.0;
                    h += 0.35 * (1.0 - t).powi(3);
                }
            }
        }
    }
    h
}

pub struct PatchMesh {
    pub center: DVec3,
    pub mesh: Mesh,
}

pub fn build_patch(model: &SurfaceModel, key: NodeKey) -> PatchMesh {
    let (s0, t0, size) = key.bounds();
    let step = size / (N - 1) as f64;
    // Sample with a one-vertex border for smooth normals.
    let m = N + 2;
    let mut pos = vec![DVec3::ZERO; m * m];
    let mut col = vec![[0.0f32; 4]; m * m];
    for j in 0..m {
        for i in 0..m {
            let d = face_dir(key.face, s0 + (i as f64 - 1.0) * step, t0 + (j as f64 - 1.0) * step);
            let (r, c) = model.sample(d.to_array());
            pos[j * m + i] = d * r;
            col[j * m + i] = c;
        }
    }
    let center = key.center_dir() * model.body.radius;
    let mut positions = Vec::with_capacity(N * N + 4 * N);
    let mut normals = Vec::with_capacity(N * N + 4 * N);
    let mut uvs = Vec::with_capacity(N * N + 4 * N);
    let mut colors = Vec::with_capacity(N * N + 4 * N);
    let center_u = {
        let c = key.center_dir();
        c.y.atan2(c.x).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU
    };
    let uv_of = |p: DVec3| {
        let d = p.normalize();
        let mut u = d.y.atan2(d.x).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
        // Keep UVs continuous across the longitude seam (sampler repeats in U).
        if u - center_u > 0.5 {
            u -= 1.0;
        } else if center_u - u > 0.5 {
            u += 1.0;
        }
        let v = (std::f64::consts::FRAC_PI_2 - d.z.clamp(-1.0, 1.0).asin()) / std::f64::consts::PI;
        [u as f32, v as f32]
    };
    for j in 1..=N {
        for i in 1..=N {
            let p = pos[j * m + i];
            let dx = pos[j * m + i + 1] - pos[j * m + i - 1];
            let dy = pos[(j + 1) * m + i] - pos[(j - 1) * m + i];
            let n = dx.cross(dy).normalize_or(p.normalize());
            positions.push((p - center).as_vec3().to_array());
            normals.push(n.as_vec3().to_array());
            uvs.push(uv_of(p));
            colors.push(col[j * m + i]);
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity((N - 1) * (N - 1) * 6 + 4 * (N - 1) * 6);
    let idx = |i: usize, j: usize| (j * N + i) as u32;
    for j in 0..N - 1 {
        for i in 0..N - 1 {
            indices.extend_from_slice(&[idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j), idx(i + 1, j + 1), idx(i, j + 1)]);
        }
    }
    // Skirts along the four edges.
    let skirt = model.body.radius * key.angular_radius() * 0.04 + 50.0;
    let edges: [Vec<(usize, usize)>; 4] = [
        (0..N).map(|i| (i, 0)).collect(),
        (0..N).map(|j| (N - 1, j)).collect(),
        (0..N).rev().map(|i| (i, N - 1)).collect(),
        (0..N).rev().map(|j| (0, j)).collect(),
    ];
    for edge in edges {
        let base = positions.len() as u32;
        for &(i, j) in &edge {
            let k = j * N + i;
            let p = DVec3::from_array(positions[k].map(|x| x as f64)) + center;
            let lowered = p - p.normalize() * skirt;
            positions.push((lowered - center).as_vec3().to_array());
            normals.push(normals[k]);
            uvs.push(uvs[k]);
            colors.push(colors[k]);
        }
        for e in 0..edge.len() - 1 {
            let (a, b) = (idx(edge[e].0, edge[e].1), idx(edge[e + 1].0, edge[e + 1].1));
            let (sa, sb) = (base + e as u32, base + e as u32 + 1);
            indices.extend_from_slice(&[a, b, sb, a, sb, sa]);
        }
    }
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices));
    PatchMesh { center, mesh }
}

#[derive(Component)]
#[allow(dead_code)]
pub struct TerrainPatch {
    pub body: BodyRef,
    pub key: NodeKey,
    pub center_local: DVec3,
}

enum NodeState {
    Building(Task<PatchMesh>),
    Ready(Entity),
}

#[derive(Resource, Default)]
pub struct TerrainLod {
    pub body: Option<BodyRef>,
    nodes: HashMap<NodeKey, NodeState>,
    material: Option<Handle<TerrainMaterial>>,
    model: Option<std::sync::Arc<SurfaceModel>>,
    model_signature: u64,
    pub visible_patches: usize,
    pub building: usize,
}

impl TerrainLod {
    /// Ground radius under a body-local direction, if a surface model is active.
    pub fn ground_radius(&self, body: BodyRef, dir_local: DVec3) -> Option<f64> {
        (self.body == Some(body)).then_some(())?;
        self.model.as_ref().map(|m| m.ground_radius(dir_local.normalize().to_array()))
    }
}

fn select_leaves(cam_local: DVec3, radius: f64, quality: f64, out: &mut Vec<NodeKey>) {
    let cam_dist = cam_local.length();
    let cam_dir = cam_local / cam_dist;
    // Horizon: the surface is only visible within this angle of the sub-camera point.
    let horizon = (radius / cam_dist).clamp(-1.0, 1.0).acos();
    let mut stack: Vec<NodeKey> = (0..6).map(|f| NodeKey { face: f, level: 0, x: 0, y: 0 }).collect();
    while let Some(k) = stack.pop() {
        let dir = k.center_dir();
        let ang = cam_dir.dot(dir).clamp(-1.0, 1.0).acos();
        if ang - k.angular_radius() > horizon + 0.02 {
            continue;
        }
        let node_size = radius * k.angular_radius() * 2.0;
        let dist = (cam_local - dir * radius).length().max(1.0);
        if k.level < MAX_LEVEL && dist < node_size * quality {
            stack.extend(k.children());
        } else {
            out.push(k);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_terrain_lod(
    mut commands: Commands,
    mut lod: ResMut<TerrainLod>,
    sim: Res<Sim>,
    rig: Res<CameraRig>,
    view: Res<ViewInfo>,
    settings: Res<UserSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    planet_mats: Res<Assets<PlanetMaterial>>,
    bodies: Query<(&BodyVisual, &MeshMaterial3d<PlanetMaterial>)>,
) {
    let u = &sim.universe;
    let t = u.time;
    // Which body (if any) deserves close-up terrain?
    let candidate = match rig.focus {
        Some(Target::Body(r)) if u.body(r).kind.has_surface() => {
            let pos = to_render(u.body_position(r, t));
            (view.screen_radius(pos, u.body(r).radius) > ACTIVATE_PX).then_some(r)
        }
        _ => None,
    };
    if candidate != lod.body {
        for (_, state) in lod.nodes.drain() {
            if let NodeState::Ready(e) = state {
                commands.entity(e).despawn();
            }
        }
        lod.body = candidate;
        lod.model = None;
        lod.material = None;
    }
    let Some(r) = lod.body else {
        lod.visible_patches = 0;
        lod.building = 0;
        return;
    };
    let body = u.body(r);

    // Surface model; rebuilt (and patches regenerated) when the world itself changes.
    let veg = u.biosphere(r).is_some_and(|b| b.vegetated());
    let sig = ((body.hydro.ocean_fraction * 40.0) as u64) ^ (((body.hydro.ice_fraction * 40.0) as u64) << 8) ^ ((body.temperature * 0.2) as u64) << 16 ^ (veg as u64) << 40;
    if lod.model.is_none() || lod.model_signature != sig {
        for (_, state) in lod.nodes.drain() {
            if let NodeState::Ready(e) = state {
                commands.entity(e).despawn();
            }
        }
        lod.model = Some(std::sync::Arc::new(SurfaceModel::new(body, veg)));
        lod.model_signature = sig;
    }
    // Terrain material mirrors the planet material's uniforms and textures.
    let Some(planet_mat) = bodies.iter().find(|(b, _)| b.r == r).and_then(|(_, m)| planet_mats.get(&m.0)) else { return };
    match &lod.material {
        Some(h) => {
            if let Some(m) = terrain_mats.get_mut(h) {
                m.u = planet_mat.u;
                m.clouds = planet_mat.clouds.clone();
                m.lights = planet_mat.lights.clone();
            }
        }
        None => {
            lod.material = Some(terrain_mats.add(TerrainMaterial { u: planet_mat.u, clouds: planet_mat.clouds.clone(), lights: planet_mat.lights.clone() }));
        }
    }

    // Choose leaves from the camera position in the body's own frame.
    let center = to_render(u.body_position(r, t));
    let rot = body_rotation(&sim, r, t).as_dquat();
    let cam_local = rot.inverse() * (view.origin - center);
    let quality = match settings.graphics {
        crate::persistence::GraphicsPreset::Low => 2.0,
        crate::persistence::GraphicsPreset::Medium => 3.0,
        crate::persistence::GraphicsPreset::High => 3.2,
        crate::persistence::GraphicsPreset::Ultra => 4.5,
    };
    let mut leaves = Vec::new();
    select_leaves(cam_local, body.radius, quality, &mut leaves);
    let wanted: HashSet<NodeKey> = leaves.iter().copied().collect();

    // Drop what's no longer wanted — but keep coarse patches until their replacements exist.
    let ready: HashSet<NodeKey> = lod.nodes.iter().filter(|(_, s)| matches!(s, NodeState::Ready(_))).map(|(k, _)| *k).collect();
    let covered = |k: &NodeKey| -> bool {
        // A ready patch may be removed once no wanted leaf inside it is still missing.
        !leaves.iter().any(|l| {
            let inside = l.face == k.face && l.level > k.level && (l.x >> (l.level - k.level)) == k.x && (l.y >> (l.level - k.level)) == k.y;
            inside && !ready.contains(l)
        })
    };
    let stale: Vec<NodeKey> = lod.nodes.keys().copied().filter(|k| !wanted.contains(k)).collect();
    for k in stale {
        let remove = match &lod.nodes[&k] {
            NodeState::Building(_) => true,
            NodeState::Ready(_) => covered(&k),
        };
        if remove {
            if let Some(NodeState::Ready(e)) = lod.nodes.remove(&k) {
                commands.entity(e).despawn();
            }
        }
    }

    // Start builds, nearest first.
    let model = lod.model.clone().unwrap();
    let mut missing: Vec<NodeKey> = leaves.iter().copied().filter(|k| !lod.nodes.contains_key(k)).collect();
    missing.sort_by(|a, b| (cam_local - a.center_dir() * body.radius).length().total_cmp(&(cam_local - b.center_dir() * body.radius).length()));
    let mut building = lod.nodes.values().filter(|s| matches!(s, NodeState::Building(_))).count();
    for k in missing {
        if building >= 12 {
            break;
        }
        let m = model.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move { build_patch(&m, k) });
        lod.nodes.insert(k, NodeState::Building(task));
        building += 1;
    }

    // Collect finished meshes.
    let material = lod.material.clone().unwrap();
    let keys: Vec<NodeKey> = lod.nodes.keys().copied().collect();
    for k in keys {
        if let Some(NodeState::Building(task)) = lod.nodes.get_mut(&k) {
            if let Some(patch) = block_on(future::poll_once(task)) {
                let e = commands
                    .spawn((
                        Mesh3d(meshes.add(patch.mesh)),
                        MeshMaterial3d(material.clone()),
                        Transform::default(),
                        WorldPos(center + rot * patch.center),
                        TerrainPatch { body: r, key: k, center_local: patch.center },
                        super::SimVisual,
                    ))
                    .id();
                lod.nodes.insert(k, NodeState::Ready(e));
            }
        }
    }
    lod.visible_patches = lod.nodes.values().filter(|s| matches!(s, NodeState::Ready(_))).count();
    lod.building = lod.nodes.values().filter(|s| matches!(s, NodeState::Building(_))).count();
}

/// Patches follow their body's position and spin in f64.
pub fn position_patches(sim: Res<Sim>, mut q: Query<(&TerrainPatch, &mut WorldPos, &mut Transform)>) {
    let u = &sim.universe;
    let t = u.time;
    let mut cache: Option<(BodyRef, DVec3, DQuat)> = None;
    for (p, mut wp, mut tf) in &mut q {
        let (center, rot) = match cache {
            Some((b, c, r)) if b == p.body => (c, r),
            _ => {
                let c = to_render(u.body_position(p.body, t));
                let r = body_rotation(&sim, p.body, t).as_dquat();
                cache = Some((p.body, c, r));
                (c, r)
            }
        };
        wp.0 = center + rot * p.center_local;
        tf.rotation = rot.as_quat();
    }
}

/// While terrain is active, shrink the textured sphere slightly so it only shows through
/// briefly while finer patches are still being built.
pub fn sink_base_sphere(lod: Res<TerrainLod>, mut q: Query<(&BodyVisual, &mut Transform), Without<TerrainPatch>>) {
    for (b, mut tf) in &mut q {
        let k = if lod.body == Some(b.r) && lod.visible_patches > 0 { 0.996 } else { 1.0 };
        tf.scale = Vec3::splat((b.radius * k) as f32);
    }
}
