use std::sync::Arc;

use crate::camera::CameraBuffer;
use crate::device::GpuContext;
use crate::lighting::{LightBuffer, LightUniform};
use crate::material::Material;
use crate::mesh::Mesh;
use crate::pipeline::{ObjectUniform, RenderPipelines};
use crate::systems::RenderEntry;
use wgpu::util::DeviceExt;

pub struct SceneMeshEntry {
    pub mesh: Mesh,
    pub material: Material,
    pub transform: glam::Mat4,
    pub object_buffer: wgpu::Buffer,
    pub object_bind_group: wgpu::BindGroup,
    pub is_emissive: bool,
}

pub struct Renderer {
    pub gpu: GpuContext,
    pub pipelines: RenderPipelines,
    pub camera_buffer: CameraBuffer,
    pub light_buffer: LightBuffer,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub scene_meshes: Vec<SceneMeshEntry>,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let gpu = pollster::block_on(GpuContext::new(window));
        let pipelines = RenderPipelines::new(&gpu.device, gpu.surface_format());

        let camera_buffer =
            CameraBuffer::new(&gpu.device, &pipelines.bind_group_layouts.camera);
        let light_buffer = LightBuffer::new(&gpu.device, &pipelines.bind_group_layouts.light);

        let (depth_texture, depth_view) =
            Self::create_depth_resources(&gpu.device, &gpu.surface_config);

        Self {
            gpu,
            pipelines,
            camera_buffer,
            light_buffer,
            depth_texture,
            depth_view,
            scene_meshes: Vec::new(),
        }
    }

    fn create_depth_resources(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        let (depth_texture, depth_view) =
            Self::create_depth_resources(&self.gpu.device, &self.gpu.surface_config);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    pub fn add_mesh(
        &mut self,
        mesh: Mesh,
        material: Material,
        transform: glam::Mat4,
        is_emissive: bool,
    ) -> usize {
        let uniform = ObjectUniform {
            model: transform.to_cols_array_2d(),
            color: material.albedo,
        };

        let object_buffer =
            self.gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Object Uniform Buffer"),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let object_bind_group =
            self.gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Object Bind Group"),
                    layout: &self.pipelines.bind_group_layouts.object,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: object_buffer.as_entire_binding(),
                    }],
                });

        let entry = SceneMeshEntry {
            mesh,
            material,
            transform,
            object_buffer,
            object_bind_group,
            is_emissive,
        };

        self.scene_meshes.push(entry);
        self.scene_meshes.len() - 1
    }

    pub fn update_mesh_transform(&mut self, index: usize, transform: glam::Mat4) {
        if let Some(entry) = self.scene_meshes.get_mut(index) {
            entry.transform = transform;
            let uniform = ObjectUniform {
                model: transform.to_cols_array_2d(),
                color: entry.material.albedo,
            };
            self.gpu
                .queue
                .write_buffer(&entry.object_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }

    /// Synchronize GPU meshes from ECS render queue entries.
    /// On first call, creates all meshes. After that, updates transforms.
    pub fn sync_scene(&mut self, entries: &[RenderEntry]) {
        if self.scene_meshes.is_empty() && !entries.is_empty() {
            // First frame: create all GPU meshes
            for entry in entries {
                let mut mesh = match entry.mesh_type {
                    cosmogon_ecs::components::renderable::RenderMesh::Sphere { subdivisions } => {
                        Mesh::uv_sphere(subdivisions)
                    }
                    _ => Mesh::uv_sphere(32),
                };
                mesh.upload(&self.gpu.device);

                let material = if entry.emissive.is_some() {
                    Material {
                        albedo: [entry.color.r, entry.color.g, entry.color.b, entry.color.a],
                        ..Material::default_star()
                    }
                } else {
                    Material {
                        albedo: [entry.color.r, entry.color.g, entry.color.b, entry.color.a],
                        ..Material::default_planet()
                    }
                };

                self.add_mesh(mesh, material, entry.transform, entry.emissive.is_some());
            }
        } else {
            // Subsequent frames: update transforms only
            for (i, entry) in entries.iter().enumerate() {
                if i < self.scene_meshes.len() {
                    self.update_mesh_transform(i, entry.transform);
                }
            }
        }
    }

    /// Set the camera view/projection directly (bypasses ECS for simplicity).
    pub fn set_camera(
        &mut self,
        eye: glam::Vec3,
        target: glam::Vec3,
        up: glam::Vec3,
        aspect: f32,
        fov_y: f32,
        near: f32,
        far: f32,
        time: f32,
    ) {
        self.camera_buffer.uniform = CameraBuffer::build(eye, target, up, aspect, fov_y, near, far, time);
    }

    /// Set the sun light position and color.
    pub fn set_light(&mut self, position: glam::Vec3, color: glam::Vec3) {
        self.light_buffer.uniform = LightUniform {
            position: [position.x, position.y, position.z, 1.0],
            color: [color.x, color.y, color.z, 1.0],
        };
    }

    pub fn render(&mut self, _time: f32) {
        let frame = match self.gpu.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost) => {
                let (w, h) = (
                    self.gpu.surface_config.width,
                    self.gpu.surface_config.height,
                );
                self.gpu.resize(w, h);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of GPU memory");
                return;
            }
            Err(e) => {
                log::warn!("Surface error: {e:?}");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Upload camera and light uniforms to GPU
        self.camera_buffer.update(&self.gpu.queue);
        self.light_buffer.update(&self.gpu.queue);

        let mut encoder =
            self.gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Render Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.001,
                            g: 0.001,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw opaque meshes (planet pipeline for regular bodies)
            pass.set_bind_group(0, &self.camera_buffer.bind_group, &[]);
            pass.set_bind_group(1, &self.light_buffer.bind_group, &[]);

            let mut current_pipeline = "";
            for entry in &self.scene_meshes {
                let pipeline_name = if entry.is_emissive { "star" } else { "planet" };
                if pipeline_name != current_pipeline {
                    if entry.is_emissive {
                        pass.set_pipeline(&self.pipelines.star);
                    } else {
                        pass.set_pipeline(&self.pipelines.planet);
                    }
                    current_pipeline = pipeline_name;
                    pass.set_bind_group(0, &self.camera_buffer.bind_group, &[]);
                    pass.set_bind_group(1, &self.light_buffer.bind_group, &[]);
                }
                pass.set_bind_group(2, &entry.object_bind_group, &[]);
                if let (Some(vb), Some(ib)) = (&entry.mesh.vertex_buffer, &entry.mesh.index_buffer) {
                    pass.set_vertex_buffer(0, vb.slice(..));
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    if entry.is_emissive {
                        // Star billboard: draw 6 verts directly, shader uses vertex_index
                        pass.draw(0..6, 0..1);
                    } else {
                        pass.draw_indexed(0..entry.mesh.index_count(), 0, 0..1);
                    }
                }
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
