// A Schwarzschild black hole, ray-traced per pixel inside a bounding sphere.
//
// Each view ray is bent by the hole's gravity by integrating the photon orbit equation
// (in units of the Schwarzschild radius r_s, the acceleration of a light ray is
// −1.5 h² r̂ / r⁴ with h = |r × v|, which reproduces the exact null geodesics). Rays that
// fall below r_s are captured (the shadow, ≈ 2.6 r_s across); rays crossing the accretion
// disk pick up its light (thermal colour T ∝ r^-¾, Doppler beaming from the orbital
// motion, gravitational redshift); escaping rays sample the starfield in their *bent*
// direction, producing the Einstein ring and the photon ring. Only the background sky is
// lensed, not nearby planets.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct HoleUniform {
    // xyz: centre relative to the camera, w: Schwarzschild radius (m)
    center: vec4<f32>,
    // xyz: disk normal (spin axis, world), w: disk brightness (0 = no disk)
    disk: vec4<f32>,
    // x: bounding radius / r_s, y: disk inner radius / r_s, z: outer / r_s, w: time (s)
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> hole: HoleUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var sky_tex: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var sky_samp: sampler;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn disk_color(r: f32, phi: f32, vel_dir: vec3<f32>, ray_dir: vec3<f32>) -> vec4<f32> {
    let rin = hole.params.y;
    let rout = hole.params.z;
    if (r < rin || r > rout) {
        return vec4<f32>(0.0);
    }
    // Temperature profile of a thin disk; inner edge hottest.
    let x = rin / r;
    let temp = pow(x, 0.75) * (1.0 - sqrt(x) * 0.9);
    // Orbital speed β = sqrt(r_s / 2r) (Keplerian, in units of c); Doppler factor.
    let beta = sqrt(0.5 / r);
    let gamma = 1.0 / sqrt(max(1.0 - beta * beta, 1e-3));
    let cosang = dot(normalize(vel_dir), -normalize(ray_dir));
    let doppler = 1.0 / (gamma * (1.0 - beta * cosang));
    let grav = sqrt(max(1.0 - 1.0 / r, 0.0));
    let shift = doppler * grav;
    let intensity = temp * pow(shift, 4.0) * 2.2;
    // Turbulent spiral streaks, slowly rotating.
    let t = hole.params.w;
    let swirl = phi + 3.0 * log(r) - t * 0.4 / pow(r, 1.5);
    let streak = 0.65 + 0.35 * sin(swirl * 7.0 + 3.0 * sin(swirl * 2.0 + r));
    let grain = 0.85 + 0.3 * hash2(vec2<f32>(floor(swirl * 40.0), floor(r * 8.0)));
    // Hot inner disk white-blue, cooler outer disk orange-red; bluer where approaching.
    let warm = vec3<f32>(1.0, 0.30, 0.05);
    let hot = vec3<f32>(1.0, 0.72, 0.42);
    var c = mix(warm, hot, clamp(temp * 3.0 * shift, 0.0, 1.0));
    c = c * vec3<f32>(clamp(1.3 - 0.3 * shift, 0.6, 1.3), 1.0, clamp(0.7 + 0.3 * shift, 0.7, 1.4));
    let edge = smoothstep(rin, rin * 1.15, r) * (1.0 - smoothstep(rout * 0.7, rout, r));
    let a = clamp(intensity * edge * streak, 0.0, 1.0) * 0.92;
    return vec4<f32>(c * intensity * streak * grain * edge * hole.disk.w, a);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let rs = hole.center.w;
    let bound = hole.params.x;
    // Work in units of r_s, origin at the hole.
    let cam = (view.world_position - hole.center.xyz) / rs;
    let dir0 = normalize(in.world_position.xyz - view.world_position);
    let inside = length(cam) < bound;
    if (inside == is_front) {
        discard;
    }
    // Start where the ray enters the bounding sphere (or at the camera if inside).
    var pos = cam;
    if (!inside) {
        let b = dot(cam, dir0);
        let c = dot(cam, cam) - bound * bound;
        let disc = b * b - c;
        if (disc < 0.0) {
            discard;
        }
        pos = cam + dir0 * max(-b - sqrt(disc), 0.0);
    }
    var vel = dir0;
    let n = normalize(hole.disk.xyz);
    let h2 = dot(cross(pos, vel), cross(pos, vel));
    var color = vec3<f32>(0.0);
    var trans = 1.0;
    var captured = false;
    for (var i = 0; i < 220; i++) {
        let r = length(pos);
        if (r < 1.0) {
            captured = true;
            break;
        }
        if (r > bound * 1.02 && dot(pos, vel) > 0.0) {
            break;
        }
        // Adaptive step: small near the hole.
        let dt = clamp(0.08 * r, 0.02, 2.5);
        let acc = -1.5 * h2 * pos / pow(r, 5.0);
        let prev = pos;
        vel = vel + acc * dt;
        pos = pos + vel * dt;
        // Disk plane crossing.
        if (hole.disk.w > 0.0) {
            let s0 = dot(prev, n);
            let s1 = dot(pos, n);
            if (s0 * s1 < 0.0) {
                let f = s0 / (s0 - s1);
                let hit = mix(prev, pos, f);
                let rr = length(hit);
                // Orbital velocity direction (prograde about n).
                let vdir = cross(n, hit);
                let e1 = normalize(cross(n, vec3<f32>(0.0, 0.0, 1.0) + n.yzx * 0.1));
                let e2 = cross(n, e1);
                let phi = atan2(dot(hit, e2), dot(hit, e1));
                let d = disk_color(rr, phi, vdir, vel);
                color += trans * d.rgb;
                trans *= 1.0 - d.a;
                if (trans < 0.02) {
                    break;
                }
            }
        }
    }
    if (!captured) {
        let sky = textureSampleLevel(sky_tex, sky_samp, normalize(vel), 0.0).rgb;
        // Same scale as Bevy's skybox (brightness 300 nits × camera exposure ≈ 0.3).
        color += trans * sky * 0.3;
    }
    // Fade into the real sky towards the bounding sphere, where bending becomes negligible.
    let impact = length(cross(cam, dir0));
    let alpha = select(1.0 - smoothstep(bound * 0.6, bound * 0.97, impact), 1.0, inside);
    return vec4<f32>(color, alpha);
}
