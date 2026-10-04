// Stellar photosphere: limb darkening, animated granulation and (for cool, active
// stars) starspots, in HDR so bloom turns the disc into a glare at a distance.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::{fbm4, vnoise}

struct StarUniform {
    // rgb: surface colour × intensity (linear HDR)
    color: vec4<f32>,
    // x: time (s), y: spot coverage 0..1, z: limb-darkening coefficient, w: seed
    params: vec4<f32>,
    // xyz: centre relative to the camera
    center: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> star: StarUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let mu = clamp(dot(n, v), 0.0, 1.0);
    let u = star.params.z;
    let limb = 1.0 - u * (1.0 - mu) - 0.15 * (1.0 - mu) * (1.0 - mu);
    // Granulation: convection cells that slowly boil.
    let t = star.params.x * 0.002;
    let cells_n = select(60.0, star.center.w, star.center.w > 0.0);
    let d = n * cells_n + vec3<f32>(star.params.w);
    let cells = vnoise(d + vec3<f32>(t, -t, t)) * 0.6 + vnoise(d * 2.3 - vec3<f32>(t)) * 0.4;
    // Starspots: dark umbrae in active latitude bands.
    let lat = abs(n.y);
    let band = smoothstep(0.05, 0.25, lat) * (1.0 - smoothstep(0.45, 0.6, lat));
    let spots_n = fbm4(n * 7.0 + vec3<f32>(star.params.w * 3.0));
    let spot = smoothstep(0.45 - star.params.y * 0.4, 0.6 - star.params.y * 0.4, spots_n) * band * step(0.001, star.params.y);
    // Faculae: bright patches near the limb.
    let faculae = smoothstep(0.2, 0.5, fbm4(n * 12.0)) * (1.0 - mu) * 0.3;
    // Fewer, larger cells (giants) have much higher contrast.
    let contrast = 0.07 + 0.35 * clamp(1.0 - cells_n / 60.0, 0.0, 1.0);
    let k = max(limb, 0.05) * (1.0 + contrast * cells + faculae) * (1.0 - 0.75 * spot);
    return vec4<f32>(star.color.rgb * k, 1.0);
}
