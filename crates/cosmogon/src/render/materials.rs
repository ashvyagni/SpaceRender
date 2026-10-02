//! Custom materials for planets and atmospheres.

use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct PlanetUniform {
    pub sun: Vec4,
    pub sun_color: Vec4,
    pub atmo: Vec4,
    pub params: Vec4,
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
}

impl Material for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cosmogon/render/shaders/planet.wgsl".into()
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct AtmosphereUniform {
    pub sun: Vec4,
    pub color: Vec4,
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
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
