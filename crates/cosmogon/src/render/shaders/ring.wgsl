// Planetary rings: radial structure from a baked profile, lit by the star with the
// planet's shadow cast across them. The lit face reflects (Lommel–Seeliger law for a
// layer of particles); the unlit face glows only by light diffusing through, strongest
// where the ring is translucent and when looking towards the star (forward scattering).

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct RingUniform {
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    // xyz: planet centre relative to the camera, w: planet radius (m)
    center: vec4<f32>,
    // xyz: ring-plane normal, w unused
    normal: vec4<f32>,
    // x inner / R, y outer / R
    extent: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> ring: RingUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var ring_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var ring_samp: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = (in.world_position.xyz - ring.center.xyz) / ring.center.w;
    let r = length(p);
    let u = (r - ring.extent.x) / (ring.extent.y - ring.extent.x);
    if (u < 0.0 || u > 1.0) {
        discard;
    }
    let tex = textureSample(ring_tex, ring_samp, vec2<f32>(u, 0.5));
    let alpha = tex.a;
    let l = normalize(ring.sun.xyz);
    let v = normalize(view.world_position - in.world_position.xyz);
    let n = normalize(ring.normal.xyz);

    // The planet's shadow (soft edge a few hundredths of a radius wide).
    let bl = dot(p, l);
    let closest = sqrt(max(dot(p, p) - bl * bl, 0.0));
    let shadow = select(1.0, smoothstep(0.985, 1.015, closest), bl < 0.0);

    let mu0 = abs(dot(n, l)) + 0.02;
    let mu = abs(dot(n, v)) + 0.02;
    let same_side = dot(n, l) * dot(n, v) > 0.0;
    let reflect = 2.0 * mu0 / (mu0 + mu);
    let fwd = pow(max(dot(-v, l), 0.0), 6.0);
    let transmit = (1.0 - alpha) * (0.5 + 2.5 * fwd) * reflect + 0.15 * alpha;
    let light = select(transmit, reflect, same_side) * shadow;
    let color = tex.rgb * light * ring.sun_color.rgb * ring.sun.w * 0.9;
    return vec4<f32>(color, alpha);
}
