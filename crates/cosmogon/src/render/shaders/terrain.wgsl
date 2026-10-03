// Close-up terrain patches. Same lighting model as planet.wgsl, but surface colour and water
// mask come from per-vertex data generated from the simulation's terrain, and normals come
// from the displaced geometry, so mountains are shaded by their own star.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct PlanetUniform {
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    atmo: vec4<f32>,
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> planet: PlanetUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var cloud_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var cloud_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var lights_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var lights_samp: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    let surf = in.color;
#else
    let surf = vec4<f32>(0.5, 0.5, 0.5, 0.0);
#endif
    let n = normalize(in.world_normal);
    let l = normalize(planet.sun.xyz);
    let v = normalize(view.world_position - in.world_position.xyz);
    let uv = in.uv;

    let cloud_uv = vec2<f32>(uv.x + planet.params.x, uv.y);
    // Cloud shadows only: up close we are below the cloud deck, so clouds dim, not cover.
    let cloud = textureSample(cloud_tex, cloud_samp, cloud_uv).r * planet.params.y;

    let ndl = dot(n, l);
    let lambert = max(ndl, 0.0);
    let ambient = 0.03 + 0.05 * planet.atmo.a * smoothstep(-0.3, 0.3, ndl);
    var color = surf.rgb * (lambert * (1.0 - 0.55 * cloud) + ambient);

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

    color *= planet.sun_color.rgb * planet.sun.w;

    let lights = textureSample(lights_tex, lights_samp, uv).r;
    let night = 1.0 - smoothstep(-0.18, 0.05, ndl);
    color += vec3<f32>(1.0, 0.68, 0.32) * lights * night * planet.params.z;
    return vec4<f32>(color, 1.0);
}
