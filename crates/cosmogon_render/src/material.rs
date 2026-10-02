use wgpu::util::DeviceExt;

/// Surface material properties for a rendered body.
pub struct Material {
    pub albedo: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub texture_bind_group: Option<wgpu::BindGroup>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub albedo: [f32; 4],
    pub metallic_roughness: [f32; 4],
    pub emissive: [f32; 4],
}

impl Material {
    pub fn new(albedo: [f32; 4], metallic: f32, roughness: f32) -> Self {
        Self {
            albedo,
            metallic,
            roughness,
            emissive: [0.0; 3],
            emissive_strength: 0.0,
            texture_bind_group: None,
        }
    }

    pub fn default_planet() -> Self {
        Self::new([0.4, 0.6, 0.8, 1.0], 0.0, 0.7)
    }

    pub fn default_star() -> Self {
        Self {
            albedo: [1.0, 0.95, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.0,
            emissive: [1.0, 0.95, 0.8],
            emissive_strength: 10.0,
            texture_bind_group: None,
        }
    }

    pub fn default_atmosphere() -> Self {
        Self::new([0.3, 0.6, 1.0, 0.3], 0.0, 0.0)
    }

    pub fn with_emissive(mut self, color: [f32; 3], strength: f32) -> Self {
        self.emissive = color;
        self.emissive_strength = strength;
        self
    }

    pub fn to_uniform(&self) -> MaterialUniform {
        MaterialUniform {
            albedo: self.albedo,
            metallic_roughness: [self.metallic, self.roughness, 0.0, 0.0],
            emissive: [
                self.emissive[0],
                self.emissive[1],
                self.emissive[2],
                self.emissive_strength,
            ],
        }
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let uniform = self.to_uniform();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }
}
