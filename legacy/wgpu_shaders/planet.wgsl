struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_pos: vec4<f32>,
    time: f32,
};

struct LightUniform {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> light: LightUniform;
@group(2) @binding(0) var<uniform> object: ObjectUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = object.model * vec4<f32>(in.position, 1.0);
    out.world_position = world_pos.xyz;
    out.world_normal = normalize((object.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv = in.uv;
    out.clip_position = camera.projection * camera.view * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let light_dir = normalize(light.position.xyz - in.world_position);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);

    // Diffuse (Lambertian)
    let ndotl = max(dot(normal, light_dir), 0.0);
    let diffuse = object.color.rgb * ndotl * light.color.rgb;

    // Specular (Blinn-Phong)
    let half_dir = normalize(light_dir + view_dir);
    let ndoth = max(dot(normal, half_dir), 0.0);
    let specular = pow(ndoth, 32.0) * light.color.rgb * 0.3;

    // Ambient
    let ambient = object.color.rgb * 0.05;

    let color = ambient + diffuse + specular;
    return vec4<f32>(color, object.color.a);
}
