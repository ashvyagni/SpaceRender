/// Simplified bloom post-processing pipeline for v0.1.
///
/// Performs a two-pass blur (downsample + upsample) to produce a soft glow
/// around bright emissive regions.
pub struct BloomPipeline {
    pub downsample_pipeline: wgpu::RenderPipeline,
    pub upsample_pipeline: wgpu::RenderPipeline,
    pub(crate) downsample_textures: Vec<BloomLevel>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

pub(crate) struct BloomLevel {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

impl BloomPipeline {
    pub fn new(
        device: &wgpu::Device,
        _format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bloom BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Bloom Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Inline blur shader for bloom
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_SHADER.into()),
        });

        let downsample_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Bloom Downsample Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &blur_shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &blur_shader,
                    entry_point: Some("fs_downsample"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let upsample_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Bloom Upsample Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &blur_shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &blur_shader,
                    entry_point: Some("fs_upsample"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Create mip chain textures (4 levels, each half size)
        let mut downsample_textures = Vec::new();
        let mut mip_width = width / 2;
        let mut mip_height = height / 2;

        for _ in 0..4 {
            mip_width = mip_width.max(1);
            mip_height = mip_height.max(1);

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Bloom Mip"),
                size: wgpu::Extent3d {
                    width: mip_width,
                    height: mip_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bloom Mip BG"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            downsample_textures.push(BloomLevel {
                _texture: texture,
                view,
                bind_group,
            });

            mip_width /= 2;
            mip_height /= 2;
        }

        Self {
            downsample_pipeline,
            upsample_pipeline,
            downsample_textures,
            bind_group_layout,
            sampler,
        }
    }

    pub fn apply(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        _source: &wgpu::TextureView,
    ) {
        if self.downsample_textures.is_empty() {
            return;
        }

        // Downsample pass chain
        for level in &self.downsample_textures {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Bloom Downsample Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &level.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(&self.downsample_pipeline);
                pass.set_bind_group(0, &level.bind_group, &[]);
                pass.draw(0..6, 0..1);
            }
        }

        // Upsample pass chain (bottom-up, additive blend)
        for i in (0..self.downsample_textures.len()).rev() {
            let target_idx = if i > 0 { i - 1 } else { 0 };
            let target_view = &self.downsample_textures[target_idx].view;

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Upsample Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.upsample_pipeline);
            pass.set_bind_group(0, &self.downsample_textures[i].bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}

const BLOOM_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

@fragment
fn fs_downsample(in: VertexOutput) -> @location(0) vec4<f32> {
    // 13-tap filter (Karis et al.)
    let texel = 1.0 / vec2<f32>(textureDimensions(input_texture));
    var color = vec3<f32>(0.0);

    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>(-1.0, -1.0) * texel).rgb * 0.0625;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 0.0, -1.0) * texel).rgb * 0.125;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 1.0, -1.0) * texel).rgb * 0.0625;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>(-1.0,  0.0) * texel).rgb * 0.125;
    color += textureSample(input_texture, input_sampler, in.uv).rgb * 0.25;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 1.0,  0.0) * texel).rgb * 0.125;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>(-1.0,  1.0) * texel).rgb * 0.0625;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 0.0,  1.0) * texel).rgb * 0.125;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 1.0,  1.0) * texel).rgb * 0.0625;

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_upsample(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(input_texture));
    var color = vec3<f32>(0.0);

    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>(-1.0, -1.0) * texel).rgb;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 1.0, -1.0) * texel).rgb;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>(-1.0,  1.0) * texel).rgb;
    color += textureSample(input_texture, input_sampler, in.uv + vec2<f32>( 1.0,  1.0) * texel).rgb;
    color += textureSample(input_texture, input_sampler, in.uv).rgb * 4.0;

    return vec4<f32>(color * 0.0625, 1.0);
}
"#;
