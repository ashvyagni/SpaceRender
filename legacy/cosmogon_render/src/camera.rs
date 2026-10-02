use bevy_ecs::system::Resource;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_pos: [f32; 4],
    pub time: f32,
    pub _pad: [f32; 3],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view: glam::Mat4::IDENTITY.to_cols_array_2d(),
            projection: glam::Mat4::IDENTITY.to_cols_array_2d(),
            view_pos: [0.0, 0.0, 0.0, 0.0],
            time: 0.0,
            _pad: [0.0; 3],
        }
    }
}

#[derive(Resource)]
pub struct CameraBuffer {
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl CameraBuffer {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let uniform = CameraUniform::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            buffer,
            bind_group,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.uniform));
    }

    pub fn build(
        eye: glam::Vec3,
        target: glam::Vec3,
        up: glam::Vec3,
        aspect: f32,
        fov_y: f32,
        near: f32,
        far: f32,
        time: f32,
    ) -> CameraUniform {
        let view = glam::Mat4::look_at_rh(eye, target, up);
        let projection = glam::Mat4::perspective_rh(fov_y, aspect, near, far);

        CameraUniform {
            view: view.to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
            view_pos: [eye.x, eye.y, eye.z, 1.0],
            time,
            _pad: [0.0; 3],
        }
    }
}
