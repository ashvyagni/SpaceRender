// Cosmogon planet surface shader.
//
// Lighting is computed per planet from its *own* star direction (passed as a uniform),
// not from engine lights, so every planet in every system is lit correctly regardless of
// distance. Output is linear HDR; Bevy's tonemapping and bloom run afterwards.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct PlanetUniform {
    // xyz: direction to the star (world space), w: illumination intensity
    sun: vec4<f32>,
    // rgb: star colour
    sun_color: vec4<f32>,
    // rgb: atmosphere scattering colour, a: strength
    atmo: vec4<f32>,
    // x: cloud longitude offset, y: cloud opacity, z: city-light intensity, w: ocean specular
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> planet: PlanetUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var albedo_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var albedo_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var cloud_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var cloud_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var lights_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var lights_samp: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(planet.sun.xyz);
    let v = normalize(view.world_position - in.world_position.xyz);
    let uv = in.uv;

    let surf = textureSample(albedo_tex, albedo_samp, uv);
    let cloud_uv = vec2<f32>(fract(uv.x + planet.params.x), uv.y);
    let cloud = textureSample(cloud_tex, cloud_samp, cloud_uv).r * planet.params.y;

    let ndl = dot(n, l);
    let lambert = max(ndl, 0.0);
    // A soft terminator stands in for atmospheric twilight.
    let twilight = smoothstep(-0.12, 0.08, ndl) * 0.06 * planet.atmo.a;

    // A whisper of starlight/airglow keeps the night side from being pure black.
    var color = surf.rgb * (lambert + twilight + 0.012);

    // Sun glint on oceans (water mask in the alpha channel).
    let h = normalize(l + v);
    let glint = pow(max(dot(n, h), 0.0), 600.0) * surf.a * planet.params.w * lambert;
    color += vec3<f32>(1.0, 0.95, 0.85) * glint * 0.5 * (1.0 - cloud);

    // Clouds over the surface.
    color = mix(color, vec3<f32>(0.95) * (lambert + twilight), cloud);

    // Rayleigh-like limb brightening on the day side.
    let rim = pow(1.0 - max(dot(n, v), 0.0), 2.5);
    color += planet.atmo.rgb * planet.atmo.a * rim * smoothstep(-0.25, 0.4, ndl) * 0.6;

    color *= planet.sun_color.rgb * planet.sun.w;

    // Artificial lights on the night side — the signature of an industrial civilization.
    let lights = textureSample(lights_tex, lights_samp, uv).r;
    let night = 1.0 - smoothstep(-0.18, 0.05, ndl);
    color += vec3<f32>(1.0, 0.68, 0.32) * lights * night * (1.0 - 0.75 * cloud) * planet.params.z;

    return vec4<f32>(color, 1.0);
}
