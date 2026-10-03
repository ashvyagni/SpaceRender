// Atmospheric scattering on a shell around the planet.
//
// Single scattering along the view ray through an exponential atmosphere, with optical
// depth towards the star sampled per point, Rayleigh-like wavelength-dependent scattering
// (coefficients per planet: blue for N₂/O₂, butterscotch for Martian dust, orange for
// Titan's organics) plus a forward-peaked Mie term for aerosols. Distances are in planet
// radii; the visual scale height is exaggerated (and the coefficients scaled to keep the
// real vertical optical depth) so the thin shell resolves at every distance. Additive, so
// it brightens what lies behind it (limb glow, blue sky from the ground, sunsets).

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::eclipse_light

struct AtmosphereUniform {
    // xyz: direction to the star, w: intensity
    sun: vec4<f32>,
    // rgb: star colour, w: star angular radius
    sun_color: vec4<f32>,
    // xyz: planet centre relative to the camera, w: planet radius (m)
    center: vec4<f32>,
    // x: shell top radius / R, y: scale height / R, z: mie extinction per R, w: mie g
    optics: vec4<f32>,
    // rgb: scattering coefficient per R at the surface
    beta: vec4<f32>,
    occluders: array<vec4<f32>, 4>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> atmo: AtmosphereUniform;

fn density(p: vec3<f32>) -> f32 {
    return exp(-(length(p) - 1.0) / atmo.optics.y);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let r = atmo.center.w;
    let ro = (view.world_position - atmo.center.xyz) / r;
    let rd = normalize(in.world_position.xyz - view.world_position);
    let top = atmo.optics.x;
    let inside = dot(ro, ro) < top * top;
    // Integrate once per pixel: through the front face from outside, the back face from inside.
    if (inside == is_front) {
        discard;
    }
    let b = dot(ro, rd);
    let c = dot(ro, ro) - top * top;
    let disc = b * b - c;
    if (disc <= 0.0) {
        discard;
    }
    let s = sqrt(disc);
    let t0 = max(-b - s, 0.0);
    var t1 = -b + s;
    let cp = dot(ro, ro) - 1.0;
    let dp = b * b - cp;
    if (dp > 0.0) {
        let tp = -b - sqrt(dp);
        if (tp > 0.0) {
            t1 = min(t1, tp);
        }
    }
    if (t1 <= t0) {
        discard;
    }

    let l = normalize(atmo.sun.xyz);
    let beta = atmo.beta.rgb;
    let mie = atmo.optics.z;
    let ext = beta + vec3<f32>(mie);
    let steps = 16;
    let ds = (t1 - t0) / f32(steps);
    var od_view = 0.0;
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < steps; i++) {
        let p = ro + rd * (t0 + (f32(i) + 0.5) * ds);
        let dens = density(p) * ds;
        od_view += dens * 0.5;
        // Towards the star: blocked by the planet?
        let bl = dot(p, l);
        let cl = dot(p, p) - 1.0;
        let blocked = bl < 0.0 && bl * bl - cl > 0.0;
        if (!blocked) {
            let tl = -bl + sqrt(max(bl * bl - (dot(p, p) - top * top), 0.0));
            let dsl = tl / 4.0;
            var od_light = 0.0;
            for (var j = 0; j < 4; j++) {
                od_light += density(p + l * (f32(j) + 0.5) * dsl) * dsl;
            }
            let shadow = eclipse_light(p, l, atmo.occluders, atmo.sun_color.w);
            sum += dens * exp(-ext * (od_view + od_light)) * shadow;
        }
        od_view += dens * 0.5;
    }
    let mu = dot(rd, l);
    let phase_r = 0.0596831 * (1.0 + mu * mu);
    let g = atmo.optics.w;
    let phase_m = 0.0795775 * (1.0 - g * g) / pow(max(1.0 + g * g - 2.0 * g * mu, 1e-4), 1.5);
    // ×π matches the surface shader's (unnormalised) Lambert convention.
    let inscatter = sum * (beta * phase_r + vec3<f32>(mie * phase_m)) * 3.14159;
    let color = inscatter * atmo.sun_color.rgb * atmo.sun.w;
    return vec4<f32>(color, 0.0);
}
