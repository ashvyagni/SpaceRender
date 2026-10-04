// Close-up terrain patches. Same lighting model as planet.wgsl, but surface colour and water
// mask come from per-vertex data generated from the simulation's terrain, and normals come
// from the displaced geometry, so mountains are shaded by their own star. Below the vertex
// spacing, three bands of procedural detail (km, hundred-metre and metre scale) fade in
// with distance as albedo variation and bump mapping, so the ground never turns to plastic.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::{fbm4, eclipse_light, perturb_normal, quat_rotate}

struct PlanetUniform {
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    atmo: vec4<f32>,
    params: vec4<f32>,
    emission: vec4<f32>,
    look: vec4<f32>,
    center: vec4<f32>,
    ring: vec4<f32>,
    ring_normal: vec4<f32>,
    orient: vec4<f32>,
    // x: molten glow 0..1, y: melt temperature (K)
    heat: vec4<f32>,
    occluders: array<vec4<f32>, 4>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> planet: PlanetUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var cloud_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var cloud_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var lights_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var lights_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var emission_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var emission_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var ring_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var ring_samp: sampler;

fn shadowing(p: vec3<f32>, l: vec3<f32>) -> f32 {
    var lit = eclipse_light(p, l, planet.occluders, planet.sun_color.w);
    if (planet.ring.w > 0.5) {
        let n = planet.ring_normal.xyz;
        let dn = dot(l, n);
        if (abs(dn) > 1e-4) {
            let t = -dot(p, n) / dn;
            if (t > 0.0) {
                let r = length(p + l * t);
                if (r > planet.ring.x && r < planet.ring.y) {
                    let u = (r - planet.ring.x) / (planet.ring.y - planet.ring.x);
                    let a = textureSampleLevel(ring_tex, ring_samp, vec2<f32>(u, 0.5), 0.0).a;
                    lit *= 1.0 - a * 0.92;
                }
            }
        }
    }
    return lit;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    let surf = in.color;
#else
    let surf = vec4<f32>(0.5, 0.5, 0.5, 0.0);
#endif
    var n = normalize(in.world_normal);
    let l = normalize(planet.sun.xyz);
    let v = normalize(view.world_position - in.world_position.xyz);
    let uv = in.uv;
    let seed = planet.look.w;

    // Detail bands, each fading in once a pixel is small compared with its wavelength.
    let rel = in.world_position.xyz - planet.center.xyz;
    // Detail is anchored to the ground, so it turns with the planet.
    let local = quat_rotate(planet.orient, rel);
    let footprint = max(fwidth(length(rel)) + length(fwidth(in.world_position.xyz)) * 0.5, 1e-3);
    var albedo_k = 0.0;
    var height = 0.0;
    var lambda = 4000.0;
    for (var i = 0; i < 3; i++) {
        let w = clamp(lambda / (footprint * 10.0) - 0.5, 0.0, 1.0);
        let nz = fbm4(local / lambda + vec3<f32>(seed + f32(i) * 17.0));
        albedo_k += w * nz * 0.10;
        height += w * nz * lambda * 0.025;
        lambda *= 0.0625;
    }
    // Water stays flat; land gets the relief.
    let land = 1.0 - surf.a;
    n = perturb_normal(n, in.world_position.xyz, height * land);
    let base = surf.rgb * (1.0 + albedo_k * land);

    let cloud_uv = vec2<f32>(uv.x + planet.params.x, uv.y);
    // Cloud shadows only: up close we are below the cloud deck, so clouds dim, not cover.
    let cloud = textureSample(cloud_tex, cloud_samp, cloud_uv).r * planet.params.y;
    // From high up the cloud deck covers the ground; below ~10 km it only casts shadows.
    let altitude = length(view.world_position - planet.center.xyz) - planet.center.w;
    let cover = smoothstep(8000.0, 40000.0, altitude);

    let shadow = shadowing(rel / planet.center.w, l);
    let ndl = dot(n, l);
    let lambert = max(ndl, 0.0) * shadow;
    let ambient = 0.03 + 0.05 * planet.atmo.a * smoothstep(-0.3, 0.3, ndl) * shadow;
    var color = base * (lambert * (1.0 - 0.55 * cloud) + ambient);

    let h = normalize(l + v);
    let glint = pow(max(dot(n, h), 0.0), 1500.0) * surf.a * planet.params.w * lambert;
    color += vec3<f32>(1.0, 0.95, 0.85) * glint * 0.35;

    // Aerial perspective through the atmosphere actually traversed: ~60 km looking
    // straight down, much more towards the horizon (and capped by the camera distance).
    let dist = length(view.world_position - in.world_position.xyz);
    let cos_view = max(dot(n, v), 0.04);
    let path = min(dist, 60000.0 / cos_view);
    let haze = (1.0 - exp(-path / 220000.0)) * min(planet.atmo.a, 1.0);
    color = mix(color, planet.atmo.rgb * (0.05 + 0.5 * lambert), clamp(haze, 0.0, 0.6));

    color = mix(color, vec3<f32>(0.95) * (max(dot(normalize(in.world_normal), l), 0.0) * shadow + ambient), clamp(cloud * cover, 0.0, 1.0));

    color *= planet.sun_color.rgb * planet.sun.w;

    let night = 1.0 - smoothstep(-0.18, 0.05, dot(normalize(in.world_normal), l));
    let emit = textureSample(emission_tex, emission_samp, uv).r;
    color += planet.emission.rgb * planet.emission.w * emit * mix(0.25, 1.0, night) * (1.0 + 2.0 * albedo_k);

    if (planet.heat.x > 0.001) {
        let crack = 1.0 - abs(fbm4(local / 40000.0 + vec3<f32>(seed)));
        let veins = pow(clamp(crack, 0.0, 1.0), 6.0);
        let tk = planet.heat.y / 1000.0;
        let glow_col = vec3<f32>(1.0, clamp(0.12 * tk * tk, 0.08, 0.55), clamp(0.012 * tk * tk * tk, 0.0, 0.18));
        color = color * (1.0 - 0.92 * planet.heat.x) + glow_col * planet.heat.x * (0.45 + 3.0 * veins) * 1.8;
    }
    let lights = textureSample(lights_tex, lights_samp, uv).r;
    color += vec3<f32>(1.0, 0.68, 0.32) * lights * night * planet.params.z;
    return vec4<f32>(color, 1.0);
}
