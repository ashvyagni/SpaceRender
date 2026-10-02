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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Billboard quad: always faces the camera
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

    let pos_2d = positions[vertex_index];

    // Extract scale from the model matrix (use the maximum component for uniform scaling)
    let scale = max(object.model[0][0], max(object.model[1][1], object.model[2][2]));
    let center = vec3<f32>(object.model[3][0], object.model[3][1], object.model[3][2]);

    // Build billboard orientation: camera-facing quad
    let camera_right = normalize(vec3<f32>(camera.view[0][0], camera.view[1][0], camera.view[2][0]));
    let camera_up = normalize(vec3<f32>(camera.view[0][1], camera.view[1][1], camera.view[2][1]));

    let world_pos = center + (camera_right * pos_2d.x + camera_up * pos_2d.y) * scale;

    out.clip_position = camera.projection * camera.view * vec4<f32>(world_pos, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Radial glow from center
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(in.uv, center);
    let glow = exp(-dist * dist * 4.0);

    // Pulsing brightness
    let pulse = 1.0 + 0.05 * sin(camera.time * 2.0);

    let color = object.color.rgb * glow * object.color.a * pulse;
    return vec4<f32>(color, glow);
}
