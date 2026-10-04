//! Impact flashes: when the simulation records a new impact, a fireball blooms at the
//! impact point (white-hot, cooling through yellow to dull red) and a shock ring races
//! outwards over the surface. Sized from the blast radius and the impactor; animated in
//! real seconds so they are seen even at high time warp. The point rides on the rotating
//! body.

use bevy::math::DVec3;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cosmogon_sim::BodyRef;
use std::collections::HashMap;

use super::{body_rotation, to_render, SharedMeshes, SimVisual, ViewInfo};
use crate::sim::Sim;

/// Real seconds a flash lasts.
const LIFETIME: f32 = 4.5;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct FlashUniform {
    /// xyz: impact point relative to the camera, w: outer radius of the effect (m).
    pub center: Vec4,
    /// xyz: local surface normal, w: age 0..1.
    pub normal: Vec4,
    /// x: fireball radius / outer, y: brightness, z: 1 if the camera is inside the bounds.
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FlashMaterial {
    #[uniform(0)]
    pub u: FlashUniform,
}

impl Material for FlashMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/impact_flash.wgsl".into()
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

#[derive(Component)]
pub struct ImpactFlash {
    pub r: BodyRef,
    /// Body-fixed unit direction of the impact point.
    pub local_dir: DVec3,
    pub outer: f64,
    pub fireball: f32,
    pub brightness: f32,
    pub born: f32,
    pub last_pos: DVec3,
}

/// Impacts already seen per body (so loading a save or a long history doesn't flash).
#[derive(Resource, Default)]
pub struct SeenImpacts {
    counts: HashMap<BodyRef, usize>,
    primed: bool,
}

pub struct ImpactFlashPlugin;

impl Plugin for ImpactFlashPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shaders/impact_flash.wgsl");
        app.add_plugins(MaterialPlugin::<FlashMaterial>::default()).init_resource::<SeenImpacts>();
    }
}

/// Size of the effect for an impact: shock ring out to a few blast radii, never smaller
/// than a few impactor radii, never much larger than the planet.
pub fn flash_extent(blast_radius: f64, impactor_radius: f64, body_radius: f64) -> f64 {
    (blast_radius * 3.0).max(impactor_radius * 12.0).min(body_radius * 1.3).max(2.0e4)
}

#[allow(clippy::too_many_arguments)]
pub fn update_impact_flashes(
    mut commands: Commands,
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    time: Res<Time>,
    shared: Res<SharedMeshes>,
    mut seen: ResMut<SeenImpacts>,
    mut flashes: Query<(Entity, &mut ImpactFlash, &mut Transform, &MeshMaterial3d<FlashMaterial>)>,
    mut mats: ResMut<Assets<FlashMaterial>>,
    mut sounds: MessageWriter<crate::audio::SoundCue>,
) {
    let u = &sim.universe;
    let now = time.elapsed_secs();
    // New impacts.
    let primed = seen.primed;
    for sys in &u.systems {
        for (i, b) in sys.bodies.iter().enumerate() {
            let r = BodyRef { system: sys.id, body: i as u32 };
            let n = b.impacts.len();
            let old = seen.counts.insert(r, n).unwrap_or(n);
            if !primed || n <= old {
                continue;
            }
            for rec in &b.impacts[old..] {
                let local = DVec3::from_array(cosmogon_sim::planet::terrain::dir_from_lat_lon(rec.lat, rec.lon));
                let outer = flash_extent(rec.blast_radius_m, rec.impactor_radius, b.radius);
                // Brightness grows slowly with energy: a 1 Mt airburst is a spark, Chicxulub a sun.
                let brightness = (rec.energy_j.max(1.0).log10() as f32 - 14.0).clamp(0.5, 12.0);
                sounds.write(crate::audio::SoundCue { kind: crate::audio::SynthKind::Rumble, intensity: brightness / 12.0 });
                let m = mats.add(FlashMaterial { u: FlashUniform::default() });
                commands.spawn((
                    Mesh3d(shared.sphere.clone()),
                    MeshMaterial3d(m),
                    Transform::default(),
                    ImpactFlash { r, local_dir: local, outer, fireball: 0.35, brightness, born: now, last_pos: DVec3::ZERO },
                    SimVisual,
                ));
            }
        }
    }
    seen.primed = true;

    for (e, mut f, mut tf, m) in &mut flashes {
        let age = (now - f.born) / LIFETIME;
        if age >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        let alive = u.systems.get(f.r.system as usize).and_then(|s| s.bodies.get(f.r.body as usize)).is_some_and(|b| b.exists());
        let (pos, normal, body_r) = if alive {
            let b = u.body(f.r);
            let rot = body_rotation(&sim, f.r, u.time).as_dquat();
            let n = rot * f.local_dir;
            (to_render(u.body_position(f.r, u.time)) + n * b.radius, n, b.radius)
        } else {
            // The target is gone (shattered or swallowed): no surface for the ring.
            (f.last_pos, DVec3::Y, 0.0)
        };
        f.last_pos = pos;
        let rel = pos - view.origin;
        tf.translation = rel.as_vec3();
        tf.scale = Vec3::splat(f.outer as f32);
        let inside = rel.length() < f.outer;
        if let Some(mat) = mats.get_mut(&m.0) {
            mat.u = FlashUniform {
                center: rel.as_vec3().extend(f.outer as f32),
                normal: normal.as_vec3().extend(age),
                params: Vec4::new(f.fireball, f.brightness, if inside { 1.0 } else { 0.0 }, (body_r / f.outer) as f32),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_scales_with_the_blast_but_stays_planet_sized() {
        let small = flash_extent(1.0e4, 50.0, 6.371e6);
        let chicxulub = flash_extent(1.5e6, 5.0e3, 6.371e6);
        let theia = flash_extent(5.0e7, 3.4e6, 6.371e6);
        assert!(small < chicxulub && chicxulub < theia);
        assert!(theia <= 6.371e6 * 1.3 + 1.0);
    }
}
