// Expanding gas shells: planetary nebulae and supernova remnants, ray-marched through a
// spherical shell with filamentary noise. Emission-line colours: ionised oxygen (teal)
// inside, hydrogen-alpha and nitrogen (red) towards the rim for planetary nebulae;
// red-and-blue filaments for supernova remnants. Additive; brightness fades with age.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::vnoise

struct NebulaUniform {
    // xyz: centre relative to the camera, w: outer radius (m)
    center: vec4<f32>,
    // x: kind (0 planetary, 1 supernova remnant), y: brightness, z: seed, w: shell thickness (fraction)
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> neb: NebulaUniform;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let r = neb.center.w;
    let ro = (view.world_position - neb.center.xyz) / r;
    let rd = normalize(in.world_position.xyz - view.world_position);
    let inside = dot(ro, ro) < 1.0;
    if (inside == is_front) {
        discard;
    }
    let b = dot(ro, rd);
    let c = dot(ro, ro) - 1.0;
    let disc = b * b - c;
    if (disc <= 0.0) {
        discard;
    }
    let s = sqrt(disc);
    let t0 = max(-b - s, 0.0);
    let t1 = -b + s;
    let steps = 16;
    let dt = (t1 - t0) / f32(steps);
    let kind = neb.params.x;
    let seed = vec3<f32>(neb.params.z);
    let inner = 1.0 - neb.params.w;
    var col = vec3<f32>(0.0);
    for (var i = 0; i < steps; i++) {
        let p = ro + rd * (t0 + (f32(i) + 0.5) * dt);
        let d = length(p);
        // Cheap reject outside the shell before any noise.
        if (d < inner - 0.25 || d > 1.0) {
            continue;
        }
        // Shell profile with a ragged inner edge.
        let warp = vnoise(p * 3.0 + seed) * 0.6 + vnoise(p * 6.1 + seed) * 0.3;
        let shell = smoothstep(inner - 0.15 + 0.1 * warp, inner + 0.05, d) * (1.0 - smoothstep(0.92, 1.0, d));
        if (shell <= 0.0) {
            continue;
        }
        let fil = vnoise(p * 7.0 + seed * 1.7 + warp) * 0.65 + vnoise(p * 14.0 + seed) * 0.35;
        let fine = vnoise(p * 19.0 - seed);
        var e: vec3<f32>;
        if (kind < 0.5) {
            let oiii = vec3<f32>(0.25, 0.85, 0.8);
            let ha = vec3<f32>(1.0, 0.25, 0.3);
            e = mix(oiii, ha, smoothstep(inner, 1.0, d)) * (0.6 + 0.6 * fil);
            // Bipolar lobes are common: brighter along an axis.
            e *= 0.7 + 0.6 * abs(p.y / max(d, 1e-3));
        } else {
            let fil2 = pow(clamp(1.0 - abs(fil) * 2.5, 0.0, 1.0), 4.0);
            e = mix(vec3<f32>(1.0, 0.3, 0.25), vec3<f32>(0.35, 0.55, 1.0), smoothstep(-0.2, 0.3, fine)) * (0.2 + 2.0 * fil2);
        }
        col += e * shell * dt;
    }
    return vec4<f32>(col * neb.params.y * 0.9, 0.0);
}
