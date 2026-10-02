use wgpu::util::DeviceExt;

use crate::pipeline::BindGroupLayouts;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtmosphereUniform {
    pub planet_radius: f32,
    pub atmosphere_radius: f32,
    pub rayleigh_scattering: [f32; 3],
    pub rayleigh_scale_height: f32,
    pub mie_scattering: [f32; 3],
    pub mie_scale_height: f32,
    pub sun_direction: [f32; 4],
    pub camera_position: [f32; 4],
    pub planet_center: [f32; 4],
}

pub struct AtmospherePipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl AtmospherePipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        bind_group_layouts: &BindGroupLayouts,
    ) -> Self {
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Atmosphere Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layouts.camera,
                    &bind_group_layouts.atmosphere,
                ],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Atmosphere Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/atmosphere.wgsl").into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Atmosphere Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[crate::mesh::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let uniform = AtmosphereUniform::earth();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Atmosphere Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atmosphere Bind Group"),
            layout: &bind_group_layouts.atmosphere,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, uniform: &AtmosphereUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniform));
    }
}

impl Default for AtmosphereUniform {
    fn default() -> Self {
        Self::earth()
    }
}

impl AtmosphereUniform {
    pub fn earth() -> Self {
        Self {
            planet_radius: 6_371_000.0,
            atmosphere_radius: 6_471_000.0,
            rayleigh_scattering: [5.5e-6, 13.0e-6, 22.4e-6],
            rayleigh_scale_height: 8500.0,
            mie_scattering: [21e-6, 21e-6, 21e-6],
            mie_scale_height: 1200.0,
            sun_direction: [0.0, 1.0, 0.0, 1.0],
            camera_position: [0.0, 0.0, 0.0, 0.0],
            planet_center: [0.0, 0.0, 0.0, 0.0],
        }
    }
}
