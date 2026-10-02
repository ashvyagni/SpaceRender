// Additive atmospheric halo drawn on a shell slightly larger than the planet.
// An approximation of single scattering: strongest at the limb, on the lit side, with a
// warm tint near the terminator. To be replaced by precomputed scattering (see ROADMAP).

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct AtmosphereUniform {
    sun: vec4<f32>,
    // rgb: scattering colour, a: density/strength
    color: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> atmo: AtmosphereUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let l = normalize(atmo.sun.xyz);
    let ndv = max(dot(n, v), 0.0);
    let ndl = dot(n, l);
    let limb = pow(1.0 - ndv, 4.0);
    let lit = smoothstep(-0.3, 0.35, ndl);
    let sunset = smoothstep(-0.25, 0.0, ndl) * (1.0 - smoothstep(0.0, 0.3, ndl));
    let tint = mix(atmo.color.rgb, vec3<f32>(1.0, 0.45, 0.2), sunset * 0.7);
    let glow = tint * limb * lit * atmo.color.a * atmo.sun.w * 1.6;
    return vec4<f32>(glow, 0.0);
}
