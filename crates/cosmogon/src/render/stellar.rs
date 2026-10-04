//! Stars in every state, compact objects and nebulae.
//!
//! * The primary star of each system is drawn according to what it is *now*: a living
//!   star (photosphere shader, sized by its current radius, so red giants swell), a white
//!   dwarf, a neutron star with beams, or a black hole (ray-traced lensing).
//! * Stars and remnants created in a sandbox are bodies with the same visuals.
//! * Planetary nebulae and supernova remnants are expanding, fading gas shells.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cosmogon_sim::astro::star::{schwarzschild_radius, StarKind};
use cosmogon_sim::astro::{BodyKind, SOLAR_MASS};
use cosmogon_sim::stellar::NebulaKind;

use super::materials::{StarMaterial, StarUniform};
use super::{to_render, SharedMeshes, SimVisual, StarVisual, ViewInfo, WorldPos};
use crate::sim::Sim;

/// Bounding sphere of the lensing shader, in Schwarzschild radii.
pub const HOLE_BOUND: f32 = 60.0;

// ── Materials ───────────────────────────────────────────────────────────

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct HoleUniform {
    pub center: Vec4,
    pub disk: Vec4,
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BlackHoleMaterial {
    #[uniform(0)]
    pub u: HoleUniform,
    #[texture(1, dimension = "cube")]
    #[sampler(2)]
    pub sky: Handle<Image>,
}

impl Material for BlackHoleMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/black_hole.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(_: &MaterialPipeline, d: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct NebulaUniform {
    pub center: Vec4,
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct NebulaMaterial {
    #[uniform(0)]
    pub u: NebulaUniform,
}

impl Material for NebulaMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/nebula.wgsl".into()
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
pub struct BeamUniform {
    pub origin: Vec4,
    pub axis: Vec4,
    pub color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BeamMaterial {
    #[uniform(0)]
    pub u: BeamUniform,
}

impl Material for BeamMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/beam.wgsl".into()
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

// ── Components ──────────────────────────────────────────────────────────

/// A body that is a star or stellar remnant (no texture bakes, no terrain).
#[derive(Component)]
pub struct StellarBody;

/// A black hole's lensing sphere; the centre comes from `WorldPos` on itself or its parent.
#[derive(Component)]
pub struct HoleVisual {
    pub rs: f64,
    pub accretion: f32,
}

/// A pulsar beam (one of two, `sign` ±1 along the magnetic axis).
#[derive(Component)]
pub struct BeamVisual {
    pub sign: f32,
    pub length: f32,
}

/// The rotating frame of a neutron star (spin shown slowed down when it is too fast to see).
#[derive(Component)]
pub struct PulsarSpin {
    pub period: f64,
}

#[derive(Component)]
pub struct NebulaVisual {
    pub system: u32,
    pub index: usize,
}

pub struct StellarPlugin;

impl Plugin for StellarPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shaders/black_hole.wgsl");
        bevy::asset::embedded_asset!(app, "shaders/nebula.wgsl");
        bevy::asset::embedded_asset!(app, "shaders/beam.wgsl");
        app.add_plugins((MaterialPlugin::<BlackHoleMaterial>::default(), MaterialPlugin::<NebulaMaterial>::default(), MaterialPlugin::<BeamMaterial>::default()));
    }
}

// ── Building visuals ────────────────────────────────────────────────────

/// Photosphere uniform for a star of temperature `teff` and luminosity `lum` (L☉).
pub fn photosphere(teff: f64, lum: f64, spots: f32, seed: f32, cells: f32) -> StarUniform {
    let (r, g, b) = cosmogon_sim::kelvin_to_rgb(teff.clamp(1000.0, 40_000.0));
    let k = 60.0 * (lum.max(1e-4)).powf(0.15) as f32;
    let limb = (0.85 - (teff as f32 - 3500.0) / 12000.0).clamp(0.35, 0.85);
    StarUniform { color: Vec4::new(r * k, g * k, b * k, 1.0), params: Vec4::new(0.0, spots, limb, seed), center: Vec4::new(0.0, 0.0, 0.0, cells) }
}

/// Children that make a stellar body (or a remnant primary) look like what it is. The
/// parent's scale is the object's physical radius.
#[allow(clippy::too_many_arguments)]
pub fn spawn_stellar_children(
    p: &mut ChildSpawnerCommands,
    kind: BodyKind,
    mass_sun: f64,
    radius: f64,
    teff: f64,
    accretion: f64,
    seed: f32,
    shared: &SharedMeshes,
    meshes: &mut Assets<Mesh>,
    star_mats: &mut Assets<StarMaterial>,
    hole_mats: &mut Assets<BlackHoleMaterial>,
    beam_mats: &mut Assets<BeamMaterial>,
    sky: &Handle<Image>,
) {
    match kind {
        BodyKind::BlackHole => {
            let rs = schwarzschild_radius(mass_sun);
            let m = hole_mats.add(BlackHoleMaterial { u: HoleUniform::default(), sky: sky.clone() });
            p.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::from_scale(Vec3::splat(HOLE_BOUND * (rs / radius) as f32)), HoleVisual { rs, accretion: accretion as f32 }));
        }
        _ => {
            let lum = match kind {
                BodyKind::Star => cosmogon_sim::astro::star::luminosity_from_mass(mass_sun),
                BodyKind::WhiteDwarf => 0.01,
                _ => 0.002,
            };
            let m = star_mats.add(StarMaterial { u: photosphere(teff, lum, if teff < 4500.0 { 0.4 } else { 0.1 }, seed, 60.0) });
            p.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default()));
            if kind == BodyKind::NeutronStar {
                // Two beams along a magnetic axis tilted 30° from the spin axis.
                let length = 3.0e11f32;
                let cone = meshes.add(Cone { radius: 1.0, height: 1.0 }.mesh().resolution(24));
                for sign in [1.0f32, -1.0] {
                    let bm = beam_mats.add(BeamMaterial { u: BeamUniform { color: Vec4::new(0.55, 0.75, 1.0, 1.0) * 0.6, ..default() } });
                    let half = 0.06f32;
                    let s = length / radius as f32;
                    // Cone apex at the star, opening outwards along ±axis.
                    let axis = Quat::from_rotation_x(0.52) * Vec3::Z * sign;
                    let rot = Quat::from_rotation_arc(Vec3::NEG_Y, axis);
                    p.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(bm),
                        Transform::from_translation(axis * s * 0.5).with_rotation(rot).with_scale(Vec3::new(s * half.tan(), s, s * half.tan())),
                        BeamVisual { sign, length },
                    ));
                }
            }
        }
    }
}

/// Spawn the visual of a system's primary star in its current state.
#[allow(clippy::too_many_arguments)]
pub fn spawn_primary(
    commands: &mut Commands,
    system: u32,
    companion: bool,
    star: &cosmogon_sim::astro::Star,
    t: f64,
    shared: &SharedMeshes,
    meshes: &mut Assets<Mesh>,
    star_mats: &mut Assets<StarMaterial>,
    hole_mats: &mut Assets<BlackHoleMaterial>,
    beam_mats: &mut Assets<BeamMaterial>,
    sky: &Handle<Image>,
) {
    let seed = system as f32 * 7.3 + companion as u8 as f32;
    let radius = star.current_radius(t);
    let visual = StarVisual { system, companion, radius, kind: star.kind };
    match star.kind {
        StarKind::Normal => {
            let m = star_mats.add(StarMaterial { u: super::star_uniform(star, t, seed) });
            commands.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default(), WorldPos::default(), visual, SimVisual));
        }
        kind => {
            let body_kind = match kind {
                StarKind::WhiteDwarf => BodyKind::WhiteDwarf,
                StarKind::NeutronStar => BodyKind::NeutronStar,
                _ => BodyKind::BlackHole,
            };
            let mut e = commands.spawn((Transform::from_scale(Vec3::splat(radius as f32)), Visibility::Inherited, WorldPos::default(), visual, SimVisual));
            if kind == StarKind::NeutronStar {
                e.insert(PulsarSpin { period: 0.033 });
            }
            e.with_children(|p| {
                spawn_stellar_children(p, body_kind, star.mass, radius, star.temperature_at(t), 0.0, seed, shared, meshes, star_mats, hole_mats, beam_mats, sky);
            });
        }
    }
}

// ── Per-frame updates ───────────────────────────────────────────────────

/// Rebuild a primary's visual when its star changes state (giant → remnant, swallowed…).
#[allow(clippy::too_many_arguments)]
pub fn sync_primaries(
    mut commands: Commands,
    sim: Res<Sim>,
    shared: Res<SharedMeshes>,
    sky: Res<super::SkyHandle>,
    q: Query<(Entity, &StarVisual)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut star_mats: ResMut<Assets<StarMaterial>>,
    mut hole_mats: ResMut<Assets<BlackHoleMaterial>>,
    mut beam_mats: ResMut<Assets<BeamMaterial>>,
) {
    let u = &sim.universe;
    for (e, v) in &q {
        let sys = u.system(v.system);
        let star = if v.companion { sys.companion.as_ref().map(|c| &c.star) } else { Some(&sys.star) };
        let Some(star) = star else { continue };
        if star.kind != v.kind {
            commands.entity(e).despawn();
            spawn_primary(&mut commands, v.system, v.companion, star, u.time, &shared, &mut meshes, &mut star_mats, &mut hole_mats, &mut beam_mats, &sky.0);
        }
    }
}

/// Stars change size as they age (red giants swell to ~1 AU).
pub fn update_star_sizes(sim: Res<Sim>, mut q: Query<&mut StarVisual>) {
    let u = &sim.universe;
    for mut v in &mut q {
        let sys = u.system(v.system);
        let star = if v.companion { sys.companion.as_ref().map(|c| &c.star) } else { Some(&sys.star) };
        if let Some(star) = star {
            let r = star.current_radius(u.time);
            if (r - v.radius).abs() > v.radius * 1e-4 {
                v.radius = r;
            }
        }
    }
}

/// Pulsars spin far too fast to see (a 33 ms period is 30 turns a second); shown at about
/// one turn every 2 s so the lighthouse sweep reads.
pub fn spin_pulsars(time: Res<Time>, mut spins: Query<(&PulsarSpin, &mut Transform)>) {
    let now = time.elapsed_secs_f64();
    for (p, mut tf) in &mut spins {
        let shown = p.period.max(2.0);
        tf.rotation = super::sim_to_render_rot() * Quat::from_rotation_z((now / shown * std::f64::consts::TAU) as f32);
    }
}

/// Lensing and beam uniforms, after positions are final for the frame.
pub fn update_compact_objects(
    time: Res<Time>,
    holes: Query<(&HoleVisual, &MeshMaterial3d<BlackHoleMaterial>, &ChildOf)>,
    beams: Query<(&BeamVisual, &MeshMaterial3d<BeamMaterial>, &ChildOf)>,
    parents: Query<&Transform>,
    mut hole_mats: ResMut<Assets<BlackHoleMaterial>>,
    mut beam_mats: ResMut<Assets<BeamMaterial>>,
) {
    let now = time.elapsed_secs_f64();
    for (h, m, parent) in &holes {
        let Ok(tf) = parents.get(parent.parent()) else { continue };
        if let Some(mat) = hole_mats.get_mut(&m.0) {
            let n = tf.rotation * Vec3::Z;
            mat.u.center = tf.translation.extend(h.rs as f32);
            mat.u.disk = n.extend(h.accretion.min(1.0) * 2.5);
            mat.u.params = Vec4::new(HOLE_BOUND, 3.0, 14.0, (now % 10_000.0) as f32);
        }
    }
    for (b, m, parent) in &beams {
        let Ok(ptf) = parents.get(parent.parent()) else { continue };
        if let Some(mat) = beam_mats.get_mut(&m.0) {
            let axis = ptf.rotation * (Quat::from_rotation_x(0.52) * Vec3::Z * b.sign);
            mat.u.origin = ptf.translation.extend(b.length);
            mat.u.axis = axis.normalize().extend(0.06);
        }
    }
}

/// Keep one shell per visible nebula; size, centre and brightness from the simulation.
pub fn sync_nebulae(
    mut commands: Commands,
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    shared: Res<SharedMeshes>,
    mut existing: Query<(Entity, &NebulaVisual, &mut Transform, &MeshMaterial3d<NebulaMaterial>)>,
    mut mats: ResMut<Assets<NebulaMaterial>>,
) {
    let u = &sim.universe;
    let t = u.time;
    let mut seen = std::collections::HashSet::new();
    for (e, nv, mut tf, m) in &mut existing {
        let Some(n) = u.systems.get(nv.system as usize).and_then(|s| s.nebulae.get(nv.index)) else {
            commands.entity(e).despawn();
            continue;
        };
        let b = n.brightness(t);
        if b <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        seen.insert((nv.system, nv.index));
        let sys = u.system(nv.system);
        let center = to_render(sys.position + sys.star_local_position(t)) - view.origin;
        let r = n.radius(t).max(1.0);
        tf.translation = center.as_vec3();
        tf.scale = Vec3::splat(r as f32);
        if let Some(mat) = mats.get_mut(&m.0) {
            mat.u.center = center.as_vec3().extend(r as f32);
            mat.u.params.y = b as f32 * if n.kind == NebulaKind::SupernovaRemnant { 0.12 } else { 0.3 };
        }
    }
    for sys in &u.systems {
        for (i, n) in sys.nebulae.iter().enumerate() {
            if seen.contains(&(sys.id, i)) || n.brightness(t) <= 0.0 {
                continue;
            }
            let kind = if n.kind == NebulaKind::Planetary { 0.0 } else { 1.0 };
            let thickness = if n.kind == NebulaKind::Planetary { 0.45 } else { 0.25 };
            let m = mats.add(NebulaMaterial { u: NebulaUniform { center: Vec4::ZERO, params: Vec4::new(kind, 1.0, (n.seed % 997) as f32, thickness) } });
            commands.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default(), NebulaVisual { system: sys.id, index: i }, SimVisual));
        }
    }
    let _ = SOLAR_MASS;
}
