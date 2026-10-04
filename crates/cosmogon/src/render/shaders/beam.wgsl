// Pulsar beams (visualised: real pulsar beams are radio and X-rays): a soft additive cone
// along the magnetic axis, fading with distance from the star and from the axis.

#import bevy_pbr::forward_io::VertexOutput

struct BeamUniform {
    // xyz: beam origin relative to the camera, w: beam length (m)
    origin: vec4<f32>,
    // xyz: unit beam axis (world), w: half-angle (rad)
    axis: vec4<f32>,
    // rgb: colour × intensity
    color: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> beam: BeamUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let rel = in.world_position.xyz - beam.origin.xyz;
    let along = dot(rel, beam.axis.xyz);
    let len = beam.origin.w;
    if (along <= 0.0) {
        discard;
    }
    let perp = length(rel - beam.axis.xyz * along);
    let ang = perp / max(along * tan(beam.axis.w), 1.0);
    let core = exp(-ang * ang * 4.0);
    let fade = exp(-along / (len * 0.35));
    return vec4<f32>(beam.color.rgb * core * fade, 0.0);
}
