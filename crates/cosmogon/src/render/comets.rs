//! Comet comae and tails.
//!
//! Ice sublimates when sunlight is strong enough (from ~3–5 AU for the Sun), so activity
//! scales with the flux L / d². Two tails form: the **ion tail**, plasma dragged straight
//! away from the star by the solar wind (blue, from CO⁺ emission), and the **dust tail**,
//! grains pushed by radiation pressure onto slower orbits, so it curves back along the path
//! (yellow-white, reflected sunlight). The coma glows green from C₂ fluorescence.
//! Each comet is a camera-facing billboard in the plane of its ion tail; the shader draws
//! all three analytically.

use bevy::asset::RenderAssetUsages;
use bevy::math::DVec3;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use cosmogon_sim::astro::{BodyKind, ObjectClass, AU};
use cosmogon_sim::BodyRef;

use super::{to_render, SimVisual, ViewInfo};
use crate::sim::Sim;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct CometUniform {
    /// xyz: nucleus relative to the camera, w: coma radius (m).
    pub center: Vec4,
    /// xyz: unit ion-tail axis (anti-sunward), w: tail length (m).
    pub ion: Vec4,
    /// xyz: unit dust-tail axis, w: dust-tail length (m).
    pub dust: Vec4,
    /// xyz: unit direction the dust tail bends towards (behind the motion), w: bend.
    pub lag: Vec4,
    /// x: brightness, y: seed, z: time (s).
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CometMaterial {
    #[uniform(0)]
    pub u: CometUniform,
}

impl Material for CometMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/comet.wgsl".into()
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
pub struct CometVisual {
    pub r: BodyRef,
}

#[derive(Resource)]
pub struct CometQuad(pub Handle<Mesh>);

pub struct CometPlugin;

impl Plugin for CometPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shaders/comet.wgsl");
        app.add_plugins(MaterialPlugin::<CometMaterial>::default());
        let quad = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[-1.0f32, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 0.0, 1.0]; 4])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let h = app.world_mut().resource_mut::<Assets<Mesh>>().add(quad);
        app.insert_resource(CometQuad(h));
    }
}

/// Sublimation activity for a flux of `lum` L☉ at `d` metres: 0 when frozen, ~1 at 1 AU
/// from the Sun, growing (sub-linearly) closer in.
pub fn activity(lum: f64, d: f64) -> f64 {
    let flux = lum / (d / AU).powi(2).max(1e-4);
    // Water ice switches on near 3 AU (flux ≈ 0.11); below ~0.04 a comet is a bare nucleus.
    let on = ((flux - 0.04) / 0.07).clamp(0.0, 1.0);
    on * flux.max(1e-9).powf(0.6)
}

/// Whether a body behaves as a comet (active when warmed).
pub fn is_comet(class: Option<ObjectClass>, kind: BodyKind, radius: f64) -> bool {
    match class {
        Some(ObjectClass::Comet) => true,
        Some(_) => false,
        None => kind == BodyKind::Icy && radius < 60_000.0,
    }
}

/// One billboard per active comet, rebuilt every frame from the simulation.
pub fn sync_comets(
    mut commands: Commands,
    sim: Res<Sim>,
    view: Res<ViewInfo>,
    time: Res<Time>,
    quad: Res<CometQuad>,
    mut existing: Query<(Entity, &CometVisual, &mut Transform, &MeshMaterial3d<CometMaterial>)>,
    mut mats: ResMut<Assets<CometMaterial>>,
) {
    let u = &sim.universe;
    let t = u.time;
    let mut seen = std::collections::HashSet::new();
    let state = |r: BodyRef| -> Option<(CometUniform, Vec3)> {
        let sys = u.systems.get(r.system as usize)?;
        let b = sys.bodies.get(r.body as usize)?;
        if !b.exists() || !is_comet(b.class, b.kind, b.radius) {
            return None;
        }
        let star = to_render(sys.star_position(t));
        let pos = to_render(sys.body_position(r.body as usize, t));
        let to_body = pos - star;
        let d = to_body.length();
        let a = activity(sys.star.luminosity(t), d);
        if a < 0.02 {
            return None;
        }
        // Velocity by a short finite difference (orbits and n-body states alike).
        let h = 3600.0;
        let vel = (pos - to_render(sys.body_position(r.body as usize, t - h))) / h;
        let anti = to_body / d;
        let back = (-vel).normalize_or(anti);
        // The dust tail leans back along the orbit by ~ v_orbit / v_grain.
        let dust_axis = (anti + back * 0.35).normalize();
        let lag = (back - dust_axis * back.dot(dust_axis)).normalize_or(DVec3::ZERO);
        let ion_len = (0.22 * AU * a.min(6.0).sqrt()).max(3.0e9);
        let dust_len = ion_len * 0.7;
        let coma = (6.0e7 * a.min(4.0).sqrt()).max(b.radius * 50.0);
        let rel = pos - view.origin;
        let look = rel.normalize_or(DVec3::NEG_Z).as_vec3();
        Some((CometUniform {
            center: rel.as_vec3().extend(coma as f32),
            ion: anti.as_vec3().extend(ion_len as f32),
            dust: dust_axis.as_vec3().extend(dust_len as f32),
            lag: lag.as_vec3().extend(0.6),
            params: Vec4::new(a.min(4.0) as f32, (r.body as f32 * 7.31 + r.system as f32 * 1.7) % 50.0, time.elapsed_secs(), 0.0),
        }, look))
    };
    let place = |tf: &mut Transform, cu: &CometUniform, look: Vec3| {
        let axis = cu.ion.truncate();
        // Billboard containing the ion tail, facing the camera as well as it can.
        let side = axis.cross(look).normalize_or(axis.any_orthonormal_vector());
        let normal = axis.cross(side);
        tf.translation = cu.center.truncate();
        tf.rotation = Quat::from_mat3(&Mat3::from_cols(axis, side, normal));
        // Tails fade out by ~2.5 lengths; the quad must reach that far or they end in a hard edge.
        let ext = cu.ion.w.max(cu.dust.w) * 2.6;
        tf.scale = Vec3::new(ext, ext, 1.0);
    };
    for (e, cv, mut tf, m) in &mut existing {
        match state(cv.r) {
            Some((cu, look)) => {
                seen.insert(cv.r);
                place(&mut tf, &cu, look);
                if let Some(mat) = mats.get_mut(&m.0) {
                    mat.u = cu;
                }
            }
            None => commands.entity(e).despawn(),
        }
    }
    for sys in &u.systems {
        for (i, b) in sys.bodies.iter().enumerate() {
            let r = BodyRef { system: sys.id, body: i as u32 };
            if seen.contains(&r) || !b.exists() || !is_comet(b.class, b.kind, b.radius) {
                continue;
            }
            if let Some((cu, look)) = state(r) {
                let mut tf = Transform::default();
                place(&mut tf, &cu, look);
                let m = mats.add(CometMaterial { u: cu });
                commands.spawn((Mesh3d(quad.0.clone()), MeshMaterial3d(m), tf, CometVisual { r }, SimVisual));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comets_wake_up_inside_the_snow_line() {
        assert_eq!(activity(1.0, 6.0 * AU), 0.0, "frozen at Jupiter's distance");
        assert!(activity(1.0, 2.5 * AU) > 0.0);
        assert!(activity(1.0, 0.6 * AU) > activity(1.0, 1.0 * AU));
        // A brighter star wakes comets farther out.
        assert!(activity(100.0, 6.0 * AU) > 0.0);
    }
}
