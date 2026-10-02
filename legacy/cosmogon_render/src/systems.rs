use bevy_ecs::prelude::*;
use glam::{Mat4, Vec3};

use cosmogon_ecs::components::camera::*;
use cosmogon_ecs::components::celestial::*;
use cosmogon_ecs::components::physics::*;
use cosmogon_ecs::components::renderable::*;
use cosmogon_ecs::components::transform::*;
use cosmogon_ecs::resources::global_time::*;

use crate::camera::CameraBuffer;
use crate::lighting::{LightBuffer, LightUniform};

/// Scale factor: positions in AU are divided by this to get rendering units.
/// 1 AU = 100 rendering units. So Mercury ~3.87, Earth ~10, Neptune ~30.
pub const AU_RENDER_SCALE: f64 = 100.0;

/// Convert f64 position to f32, applying AU scale.
fn au_to_render(v: cosmogon_core::math::Vec3d) -> Vec3 {
    Vec3::new(
        (v.x * AU_RENDER_SCALE) as f32,
        (v.y * AU_RENDER_SCALE) as f32,
        (v.z * AU_RENDER_SCALE) as f32,
    )
}

/// Queue of render entries produced by the transform build system.
#[derive(Resource, Default)]
pub struct RenderQueue {
    pub entries: Vec<RenderEntry>,
}

#[derive(Clone)]
pub struct RenderEntry {
    pub entity: Entity,
    pub transform: Mat4,
    pub mesh_type: RenderMesh,
    pub color: Color,
    pub emissive: Option<(Color, f32)>,
    pub radius: f32,
}

/// System: build model transform matrices from Position/Rotation/Scale components.
pub fn build_transforms(
    query: Query<(
        Entity,
        &Position,
        &Rotation,
        &Scale,
        &RenderMesh,
        &Color,
        Option<&EmissiveColor>,
        Option<&Radius>,
    )>,
    mut render_queue: ResMut<RenderQueue>,
) {
    render_queue.entries.clear();

    for (entity, pos, rot, _scale, mesh, color, emissive, radius) in query.iter() {
        let scaled = au_to_render(pos.coords);

        // Give every body a visible size based on type
        let r = if emissive.is_some() {
            40.0  // Sun/star — big billboard glow
        } else {
            let raw = radius.map(|rad| rad.0).unwrap_or(1.0);
            if raw > 2.0e7 {
                12.0  // Gas giant
            } else if raw > 1.0e6 {
                6.0   // Rocky planet
            } else {
                2.5   // Moon/small body
            }
        };

        let model = Mat4::from_translation(scaled)
            * Mat4::from_quat(rot.quat)
            * Mat4::from_scale(Vec3::splat(r));

        let emissive_data = emissive.map(|e| (e.color, e.intensity));

        render_queue.entries.push(RenderEntry {
            entity,
            transform: model,
            mesh_type: *mesh,
            color: *color,
            emissive: emissive_data,
            radius: r,
        });
    }
}

/// System: update the camera uniform buffer from camera components.
pub fn update_camera(
    query: Query<(&Position, &Camera, &CameraTarget)>,
    target_positions: Query<&Position>,
    global_time: Res<GlobalTime>,
    mut camera_buffer: ResMut<CameraBuffer>,
) {
    for (cam_pos, camera, target) in query.iter() {
        let eye = au_to_render(cam_pos.coords);

        let target_pos = target_positions
            .get(target.0)
            .map(|p| au_to_render(p.coords))
            .unwrap_or(Vec3::ZERO);

        let uniform = CameraBuffer::build(
            eye,
            target_pos,
            Vec3::Y,
            camera.aspect,
            camera.fov,
            camera.near,
            camera.far,
            global_time.elapsed as f32,
        );

        camera_buffer.uniform = uniform;
        break;
    }
}

/// System: update sun light position from the star's world position.
pub fn update_sun_light(
    star_query: Query<&Position, With<Star>>,
    mut light_buffer: ResMut<LightBuffer>,
) {
    for pos in star_query.iter().take(1) {
        light_buffer.uniform = LightUniform {
            position: [
                pos.coords.x as f32,
                pos.coords.y as f32,
                pos.coords.z as f32,
                1.0,
            ],
            color: [1.0, 0.95, 0.8, 1.0],
        };
        break;
    }
}
