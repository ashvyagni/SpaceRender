// Cosmogon planet globe shader.
//
// Lighting is computed per planet from its *own* star direction (passed as a uniform),
// not from engine lights, so every planet in every system is lit correctly regardless of
// distance. Adds: procedural detail finer than the baked texture when zoomed in, zonal
// flow on giants, self-emission (lava, hot spots, thermal glow), moon eclipse shadows and
// ring shadows. Output is linear HDR; Bevy's tonemapping and bloom run afterwards.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::{fbm4, vnoise, dir_from_uv, eclipse_light, perturb_normal}

struct PlanetUniform {
    // xyz: direction to the star (world space), w: illumination intensity
    sun: vec4<f32>,
    // rgb: star colour, w: star angular radius seen from the planet (radians)
    sun_color: vec4<f32>,
    // rgb: atmosphere scattering colour, a: strength
    atmo: vec4<f32>,
    // x: cloud longitude offset, y: cloud opacity, z: city-light intensity, w: ocean specular
    params: vec4<f32>,
    // rgb: emission colour (linear), w: emission strength
    emission: vec4<f32>,
    // x: style (0 solid, 1 giant, 2 cloud deck), y: detail strength, z: flow phase, w: seed
    look: vec4<f32>,
    // xyz: centre relative to the camera, w: radius (m)
    center: vec4<f32>,
    // x: inner radius / R, y: outer radius / R, z: opacity, w: 1 when ringed
    ring: vec4<f32>,
    // xyz: ring-plane normal (world)
    ring_normal: vec4<f32>,
    // Quaternion rotating world-frame offsets into the body's own (spinning) frame
    orient: vec4<f32>,
    // Shadow-casting moons: xyz offset / R, w: radius / R (0 = unused)
    occluders: array<vec4<f32>, 4>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> planet: PlanetUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var albedo_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var albedo_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var cloud_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var cloud_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var lights_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var lights_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var emission_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var emission_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var ring_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var ring_samp: sampler;

// Light reaching a point after moons and rings have had their say.
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
    var n = normalize(in.world_normal);
    let l = normalize(planet.sun.xyz);
    let v = normalize(view.world_position - in.world_position.xyz);
    let style = planet.look.x;
    let seed = planet.look.w;
    let d = dir_from_uv(in.uv);
    // Detail is defined against the full-resolution texture (4096 texels around), not the
    // one currently loaded, so a world still baking looks soft rather than noisy.
    let tex_w = 4096.0;

    // How far we are past the texture's resolution: 0 when a texel covers a pixel or less.
    let texel_px = fwidth(in.uv.x) * tex_w;
    let zoom = clamp(1.0 - texel_px * 1.5, 0.0, 1.0) * planet.look.y;

    var uv = in.uv;
    if (style > 0.5 && style < 1.5) {
        // Giants: bands drift at different speeds (zonal jets), and fine turbulence
        // displaces the lookup so structure continues below the texture resolution.
        let lat = asin(clamp(d.z, -1.0, 1.0));
        let jet = 0.006 * cos(lat * 14.0 + seed) + 0.003 * cos(lat * 5.0);
        uv.x += jet * planet.look.z;
        let q = vec3<f32>(d.x * 900.0, d.y * 900.0, d.z * 2400.0) + seed;
        let warp = vec2<f32>(fbm4(q), fbm4(q + vec3<f32>(5.2, 1.3, 7.7))) * 0.35 / tex_w;
        uv += warp * zoom;
    }

    let surf = textureSample(albedo_tex, albedo_samp, uv);
    var base = surf.rgb;
    let cloud_uv = vec2<f32>(fract(uv.x + planet.params.x), uv.y);
    var cloud = textureSample(cloud_tex, cloud_samp, cloud_uv).r * planet.params.y;

    // Procedural detail beyond the texture: albedo variation, and bump-mapped relief on
    // solid worlds so craters and ridges keep catching the light up close.
    let k = tex_w / 6.2831853;
    let fine = fbm4(d * k * 3.0 + seed);
    let finer = vnoise(d * k * 24.0 + seed);
    if (style < 0.5) {
        base *= 1.0 + zoom * (0.10 * fine + 0.05 * finer);
        let h = (fine * 0.8 + finer * 0.1) * planet.center.w / tex_w * 0.3 * zoom;
        n = perturb_normal(n, in.world_position.xyz, h);
        cloud *= 1.0 + zoom * 0.5 * fine;
    } else if (style < 1.5) {
        base *= 1.0 + zoom * 0.06 * fine;
    } else {
        base *= 1.0 + zoom * (0.05 * fine + 0.02 * finer);
    }

    let p = (in.world_position.xyz - planet.center.xyz) / planet.center.w;
    let shadow = shadowing(p, l);
    let ndl = dot(n, l);
    let lambert = max(ndl, 0.0) * shadow;
    // A soft terminator stands in for atmospheric twilight.
    let twilight = smoothstep(-0.12, 0.08, ndl) * 0.06 * planet.atmo.a * shadow;

    // A whisper of starlight/airglow keeps the night side from being pure black.
    var color = base * (lambert + twilight + 0.012);

    // Sun glint on oceans (water mask in the alpha channel).
    let h = normalize(l + v);
    let glint = pow(max(dot(n, h), 0.0), 2000.0) * surf.a * planet.params.w * lambert;
    color += vec3<f32>(1.0, 0.95, 0.85) * glint * 0.25 * (1.0 - cloud);

    // Clouds over the surface.
    color = mix(color, vec3<f32>(0.95) * (lambert + twilight), clamp(cloud, 0.0, 1.0));

    color *= planet.sun_color.rgb * planet.sun.w;

    // Self-emission: lava and hot spots, and the thermal glow of hot giants' night sides.
    let night = 1.0 - smoothstep(-0.18, 0.05, dot(normalize(in.world_normal), l));
    let emit = textureSample(emission_tex, emission_samp, uv).r;
    color += planet.emission.rgb * planet.emission.w * emit * mix(0.25, 1.0, night) * (1.0 + 0.3 * zoom * fine);

    // Artificial lights on the night side — the signature of an industrial civilization.
    let lights = textureSample(lights_tex, lights_samp, in.uv).r;
    color += vec3<f32>(1.0, 0.68, 0.32) * lights * night * (1.0 - 0.75 * cloud) * planet.params.z;

    return vec4<f32>(color, 1.0);
}
