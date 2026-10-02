#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    /// Generate a UV sphere with the given number of latitude/longitude subdivisions.
    pub fn uv_sphere(subdivisions: u32) -> Self {
        let lat_segments = subdivisions;
        let lon_segments = subdivisions * 2;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for lat in 0..=lat_segments {
            let theta = std::f32::consts::PI * lat as f32 / lat_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=lon_segments {
                let phi = std::f32::consts::TAU * lon as f32 / lon_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = sin_theta * cos_phi;
                let y = cos_theta;
                let z = sin_theta * sin_phi;

                let u = lon as f32 / lon_segments as f32;
                let v = lat as f32 / lat_segments as f32;

                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [x, y, z],
                    uv: [u, v],
                });
            }
        }

        for lat in 0..lat_segments {
            for lon in 0..lon_segments {
                let first = lat * (lon_segments + 1) + lon;
                let second = first + lon_segments + 1;

                indices.push(first);
                indices.push(second);
                indices.push(first + 1);

                indices.push(second);
                indices.push(second + 1);
                indices.push(first + 1);
            }
        }

        Self::new(vertices, indices)
    }

    /// Generate an icosphere by subdividing an icosahedron.
    pub fn icosphere(subdivisions: u32) -> Self {
        let (mut vertices, mut triangles) = Self::icosahedron();

        for _ in 0..subdivisions {
            let mut midpoint_cache = std::collections::HashMap::new();
            let mut new_triangles = Vec::new();

            for tri in &triangles {
                let a = Self::get_midpoint(tri[0], tri[1], &mut vertices, &mut midpoint_cache);
                let b = Self::get_midpoint(tri[1], tri[2], &mut vertices, &mut midpoint_cache);
                let c = Self::get_midpoint(tri[2], tri[0], &mut vertices, &mut midpoint_cache);

                new_triangles.push([tri[0], a, c]);
                new_triangles.push([tri[1], b, a]);
                new_triangles.push([tri[2], c, b]);
                new_triangles.push([a, b, c]);
            }

            triangles = new_triangles;
        }

        // Normalize all vertices to unit sphere and build output
        let mut out_vertices = Vec::with_capacity(vertices.len());
        for v in &vertices {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            let nx = v[0] / len;
            let ny = v[1] / len;
            let nz = v[2] / len;

            // Spherical UV
            let u = 0.5 + ny.atan2(nx) / std::f32::consts::TAU;
            let v_coord = 0.5 - (nz).asin() / std::f32::consts::PI;

            out_vertices.push(Vertex {
                position: [nx, ny, nz],
                normal: [nx, ny, nz],
                uv: [u, v_coord],
            });
        }

        let mut out_indices = Vec::with_capacity(triangles.len() * 3);
        for tri in &triangles {
            out_indices.push(tri[0]);
            out_indices.push(tri[1]);
            out_indices.push(tri[2]);
        }

        Self::new(out_vertices, out_indices)
    }

    fn icosahedron() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let t = (1.0 + 5.0_f32.sqrt()) / 2.0;

        let mut vertices = vec![
            [-1.0, t, 0.0],
            [1.0, t, 0.0],
            [-1.0, -t, 0.0],
            [1.0, -t, 0.0],
            [0.0, -1.0, t],
            [0.0, 1.0, t],
            [0.0, -1.0, -t],
            [0.0, 1.0, -t],
            [t, 0.0, -1.0],
            [t, 0.0, 1.0],
            [-t, 0.0, -1.0],
            [-t, 0.0, 1.0],
        ];

        // Normalize
        for v in &mut vertices {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            v[0] /= len;
            v[1] /= len;
            v[2] /= len;
        }

        let triangles = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        (vertices, triangles)
    }

    fn get_midpoint(
        a: u32,
        b: u32,
        vertices: &mut Vec<[f32; 3]>,
        cache: &mut std::collections::HashMap<(u32, u32), u32>,
    ) -> u32 {
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(&idx) = cache.get(&key) {
            return idx;
        }

        let va = vertices[a as usize];
        let vb = vertices[b as usize];
        let mid = [
            (va[0] + vb[0]) * 0.5,
            (va[1] + vb[1]) * 0.5,
            (va[2] + vb[2]) * 0.5,
        ];

        let idx = vertices.len() as u32;
        vertices.push(mid);
        cache.insert(key, idx);
        idx
    }

    /// Upload mesh data to GPU buffers.
    pub fn upload(&mut self, device: &wgpu::Device) {
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }

    /// A simple fullscreen quad for post-processing passes.
    pub fn fullscreen_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: [-1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        Self::new(vertices, indices)
    }
}

use wgpu::util::DeviceExt;
