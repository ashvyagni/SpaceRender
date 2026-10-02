use std::time::Instant;
use wgpu::util::DeviceExt;

pub struct LoadingScreen {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    start_time: Instant,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LoadingUniforms {
    time: f32,
    progress: f32,
    _pad: [f32; 2],
}

impl LoadingScreen {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Loading Screen BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniforms = LoadingUniforms {
            time: 0.0,
            progress: 0.0,
            _pad: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Loading Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Loading Screen BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Loading Screen Shader"),
            source: wgpu::ShaderSource::Wgsl(LOADING_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Loading Screen Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Loading Screen Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
            start_time: Instant::now(),
        }
    }

    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let min_duration = 2.5;
        let fade_duration = 3.5;
        if elapsed < min_duration {
            (elapsed / min_duration * 0.8).min(0.8)
        } else if elapsed < fade_duration {
            0.8 + ((elapsed - min_duration) / (fade_duration - min_duration) * 0.2)
        } else {
            1.0
        }
    }

    pub fn is_done(&self) -> bool {
        self.start_time.elapsed().as_secs_f32() >= 3.5
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let time = self.start_time.elapsed().as_secs_f32();
        let progress = self.progress();

        let uniforms = LoadingUniforms {
            time,
            progress,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Loading Screen Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}

const LOADING_SHADER: &str = r#"
struct Uniforms {
    time: f32,
    progress: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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

// Hash function for starfield
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

// Noise function for nebula
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash(i + vec2<f32>(0.0, 0.0));
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 5; i++) {
        value += amplitude * noise(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let aspect = 16.0 / 9.0;
    let centered_uv = (uv - 0.5) * vec2<f32>(aspect, 1.0);
    let t = uniforms.time;
    let progress = uniforms.progress;

    // Deep space background with subtle nebula
    var color = vec3<f32>(0.005, 0.005, 0.02);
    let nebula = fbm(uv * 3.0 + t * 0.02);
    color += vec3<f32>(0.02, 0.01, 0.04) * nebula;

    // Secondary nebula layer
    let nebula2 = fbm(uv * 5.0 - t * 0.015 + 100.0);
    color += vec3<f32>(0.01, 0.02, 0.05) * nebula2 * 0.5;

    // Starfield - multiple layers
    for (var layer = 0; layer < 3; layer++) {
        let scale = 80.0 + f32(layer) * 40.0;
        let star_uv = uv * scale;
        let star_id = floor(star_uv);
        let star_pos = fract(star_uv) - 0.5;

        let h = hash(star_id + f32(layer) * 100.0);
        if h > 0.97 {
            let star_offset = vec2<f32>(
                hash(star_id + 1.0) - 0.5,
                hash(star_id + 2.0) - 0.5
            ) * 0.4;
            let d = length(star_pos - star_offset);

            // Twinkling
            let twinkle = 0.7 + 0.3 * sin(t * (2.0 + h * 4.0) + h * 100.0);
            let brightness = twinkle * smoothstep(0.02, 0.0, d);

            // Star color based on hash
            let temp = h;
            var star_color: vec3<f32>;
            if temp < 0.3 {
                star_color = vec3<f32>(0.6, 0.8, 1.0); // Blue-white
            } else if temp < 0.6 {
                star_color = vec3<f32>(1.0, 1.0, 0.9); // White
            } else if temp < 0.8 {
                star_color = vec3<f32>(1.0, 0.9, 0.7); // Yellow
            } else {
                star_color = vec3<f32>(1.0, 0.7, 0.5); // Orange
            }

            color += star_color * brightness;
        }
    }

    // Central sun glow
    let sun_dist = length(centered_uv);
    let sun_glow = exp(-sun_dist * 3.0) * 0.3;
    let sun_core = exp(-sun_dist * 12.0) * 0.8;
    let sun_pulse = 1.0 + 0.1 * sin(t * 1.5);

    color += vec3<f32>(1.0, 0.8, 0.4) * sun_glow * sun_pulse;
    color += vec3<f32>(1.0, 0.95, 0.8) * sun_core * sun_pulse;

    // Lens flare rays
    let angle = atan2(centered_uv.y, centered_uv.x);
    let rays = pow(abs(sin(angle * 6.0 + t * 0.3)), 8.0) * 0.15;
    let ray_glow = rays * exp(-sun_dist * 2.5) * sun_pulse;
    color += vec3<f32>(1.0, 0.85, 0.6) * ray_glow;

    // Title text area - "COSMOGON" rendered as glowing geometry
    let title_y = 0.12;
    let title_dist = abs(uv.y - title_y);
    let title_width = smoothstep(0.04, 0.02, title_dist);

    // Title glow
    let title_glow = exp(-title_dist * 30.0) * 0.5;
    color += vec3<f32>(0.4, 0.6, 1.0) * title_glow * (0.8 + 0.2 * sin(t * 2.0));

    // Title bar line
    let bar_y = 0.16;
    let bar_dist = abs(uv.y - bar_y);
    let bar_visible = smoothstep(0.002, 0.0, bar_dist) * smoothstep(0.35, 0.34, abs(uv.x - 0.5));
    color += vec3<f32>(0.3, 0.5, 0.9) * bar_visible * (0.5 + 0.5 * sin(t * 3.0));

    // Progress bar
    let bar_center_y = 0.08;
    let bar_height = 0.012;
    let bar_width = 0.3;
    let bar_x = 0.5;

    let bar_area = smoothstep(bar_height, bar_height - 0.001, abs(uv.y - bar_center_y))
                 * smoothstep(bar_width + 0.002, bar_width, abs(uv.x - bar_x));

    // Progress fill
    let fill_x = bar_x - bar_width + progress * bar_width * 2.0;
    let fill_area = smoothstep(bar_height, bar_height - 0.001, abs(uv.y - bar_center_y))
                  * smoothstep(0.002, 0.0, uv.x - (bar_x - bar_width))
                  * smoothstep(0.002, 0.0, fill_x - uv.x);

    // Bar border glow
    let bar_border = smoothstep(0.003, 0.0, abs(abs(uv.y - bar_center_y) - bar_height))
                   * smoothstep(bar_width + 0.003, bar_width + 0.001, abs(uv.x - bar_x));

    color += vec3<f32>(0.15, 0.2, 0.35) * bar_area;
    color += vec3<f32>(0.3, 0.6, 1.0) * fill_area * (0.8 + 0.2 * sin(t * 4.0 + uv.x * 10.0));
    color += vec3<f32>(0.4, 0.7, 1.0) * bar_border * 0.6;

    // Progress shimmer effect on the fill
    let shimmer = sin(uv.x * 50.0 - t * 3.0) * 0.5 + 0.5;
    color += vec3<f32>(0.2, 0.4, 0.8) * fill_area * shimmer * 0.3;

    // Fade to black at edges (vignette)
    let vignette = 1.0 - dot(uv - 0.5, uv - 0.5) * 1.5;
    color *= vignette;

    // Overall fade in
    let fade_in = smoothstep(0.0, 1.0, t);
    color *= fade_in;

    // Final fade to white when loading completes (transition to universe)
    let fade_out = smoothstep(3.0, 3.5, t);
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), fade_out);

    return vec4<f32>(color, 1.0);
}
"#;
