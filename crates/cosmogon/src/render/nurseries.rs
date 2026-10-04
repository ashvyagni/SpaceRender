//! Star-forming nebulae (see `cosmogon_sim::nursery`): one ray-marched volume per cloud,
//! glowing while it has gas.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cosmogon_sim::astro::LIGHT_YEAR;

use super::{to_render, SharedMeshes, SimVisual, ViewInfo};
use crate::sim::Sim;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct EmissionUniform {
    pub center: Vec4,
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct EmissionNebulaMaterial {
    #[uniform(0)]
    pub u: EmissionUniform,
}

impl Material for EmissionNebulaMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/emission_nebula.wgsl".into()
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
pub struct NurseryVisual(pub usize);

pub struct NurseryPlugin;

impl Plugin for NurseryPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shaders/emission_nebula.wgsl");
        app.add_plugins(MaterialPlugin::<EmissionNebulaMaterial>::default());
    }
}

pub fn sync_nurseries(
    mut commands: Commands,
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    shared: Res<SharedMeshes>,
    mut existing: Query<(Entity, &NurseryVisual, &mut Transform, &MeshMaterial3d<EmissionNebulaMaterial>)>,
    mut mats: ResMut<Assets<EmissionNebulaMaterial>>,
) {
    let u = &sim.universe;
    let mut seen = vec![false; u.nurseries.len()];
    for (e, nv, mut tf, m) in &mut existing {
        let Some(n) = u.nurseries.get(nv.0) else {
            commands.entity(e).despawn();
            continue;
        };
        seen[nv.0] = true;
        let r = n.radius_ly * LIGHT_YEAR;
        let rel = to_render(n.position) - view.origin;
        tf.translation = rel.as_vec3();
        tf.scale = Vec3::splat((r * 1.02) as f32);
        if let Some(mat) = mats.get_mut(&m.0) {
            mat.u.center = rel.as_vec3().extend(r as f32);
            mat.u.params.x = n.brightness() as f32;
            mat.u.params.z = if rel.length() < r * 1.02 { 1.0 } else { 0.0 };
        }
    }
    for (i, n) in u.nurseries.iter().enumerate() {
        if seen[i] {
            continue;
        }
        let m = mats.add(EmissionNebulaMaterial { u: EmissionUniform { center: Vec4::ZERO, params: Vec4::new(n.brightness() as f32, (n.seed % 97) as f32 * 0.37, 0.0, 0.0) } });
        commands.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::default(), NurseryVisual(i), SimVisual));
    }
}
