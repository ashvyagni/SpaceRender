//! Custom materials for planets, terrain, atmospheres, rings and stars.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

/// Per-planet shading state, shared by the globe and its close-up terrain.
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct PlanetUniform {
    /// xyz: direction to the star, w: illumination intensity.
    pub sun: Vec4,
    /// rgb: star colour, w: star angular radius seen from the planet (radians).
    pub sun_color: Vec4,
    /// rgb: atmosphere colour, a: strength.
    pub atmo: Vec4,
    /// x: cloud drift, y: cloud opacity, z: city lights, w: ocean specular.
    pub params: Vec4,
    /// rgb: emission colour (linear), w: emission strength.
    pub emission: Vec4,
    /// x: style (0 solid, 1 giant, 2 cloud deck), y: detail strength, z: flow phase, w: seed.
    pub look: Vec4,
    /// xyz: centre relative to the camera, w: radius (m).
    pub center: Vec4,
    /// x: inner / R, y: outer / R, z: opacity, w: 1 when ringed.
    pub ring: Vec4,
    /// xyz: ring-plane normal (world).
    pub ring_normal: Vec4,
    /// World → body-frame rotation (quaternion xyzw).
    pub orient: Vec4,
    /// Shadow-casting moons: xyz offset / R, w: radius / R (0 = unused).
    pub occluders: [Vec4; 4],
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlanetMaterial {
    #[uniform(0)]
    pub u: PlanetUniform,
    #[texture(1)]
    #[sampler(2)]
    pub albedo: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub clouds: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub lights: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub emission: Handle<Image>,
    #[texture(9)]
    #[sampler(10)]
    pub ring: Handle<Image>,
}

impl Material for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/planet.wgsl".into()
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct AtmosphereUniform {
    pub sun: Vec4,
    pub sun_color: Vec4,
    /// xyz: planet centre relative to the camera, w: planet radius (m).
    pub center: Vec4,
    /// x: shell top / R, y: scale height / R, z: Mie extinction per R, w: Mie asymmetry g.
    pub optics: Vec4,
    /// rgb: scattering coefficients per R at the surface.
    pub beta: Vec4,
    pub occluders: [Vec4; 4],
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AtmosphereMaterial {
    #[uniform(0)]
    pub u: AtmosphereUniform,
}

impl Material for AtmosphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/atmosphere.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        // The shader picks the face to integrate (front from outside, back from inside).
        descriptor.primitive.cull_mode = None;
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}

/// Material for close-up terrain patches (vertex colours, shared planet uniforms/textures).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub u: PlanetUniform,
    #[texture(3)]
    #[sampler(4)]
    pub clouds: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub lights: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub emission: Handle<Image>,
    #[texture(9)]
    #[sampler(10)]
    pub ring: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/terrain.wgsl".into()
    }
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        // Skirts hang below patch edges to hide cracks between detail levels; draw both sides.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct RingUniform {
    pub sun: Vec4,
    pub sun_color: Vec4,
    pub center: Vec4,
    pub normal: Vec4,
    /// x: inner / R, y: outer / R.
    pub extent: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct RingMaterial {
    #[uniform(0)]
    pub u: RingUniform,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl Material for RingMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/ring.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct StarUniform {
    /// rgb: surface colour × intensity (linear HDR).
    pub color: Vec4,
    /// x: time (s), y: spot coverage, z: limb darkening, w: seed.
    pub params: Vec4,
    pub center: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StarMaterial {
    #[uniform(0)]
    pub u: StarUniform,
}

impl Material for StarMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/star.wgsl".into()
    }
}
