// A star-forming (H II) nebula, ray-marched: clumpy gas glowing pink-red in hydrogen-alpha,
// teal-blue near the hot young stars where oxygen is doubly ionised, a cavity blown out by
// their light, and dark dust lanes and pillars in front. Additive.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::vnoise

struct NebulaUniform {
    // xyz: centre relative to the camera, w: radius (m)
    center: vec4<f32>,
    // x: brightness (gas left), y: seed, z: camera inside the bounds, w: unused
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> neb: NebulaUniform;

fn fbm(p: vec3<f32>) -> f32 {
    var a = 0.5;
    var s = 0.0;
    var q = p;
    for (var i = 0; i < 4; i++) {
        s += a * vnoise(q);
        q = q * 2.03 + vec3<f32>(1.7, 9.2, 4.1);
        a *= 0.5;
    }
    return s;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let inside = neb.params.z > 0.5;
    if (inside == is_front) {
        discard;
    }
    // Work in units of the radius.
    let R = neb.center.w;
    let cam = (view.world_position - neb.center.xyz) / R;
    let dir = normalize((in.world_position.xyz - view.world_position) / R);
    let b = dot(cam, dir);
    let c = dot(cam, cam) - 1.0;
    let disc = b * b - c;
    if (disc <= 0.0) {
        discard;
    }
    let sq = sqrt(disc);
    let t0 = max(-b - sq, 0.0);
    let t1 = -b + sq;
    if (t1 <= t0) {
        discard;
    }
    let seed = vec3<f32>(neb.params.y);
    let steps = 40;
    let dt = (t1 - t0) / f32(steps);
    var col = vec3<f32>(0.0);
    var trans = 1.0;
    for (var i = 0; i < steps; i++) {
        let p = cam + dir * (t0 + (f32(i) + 0.5) * dt);
        let r = length(p);
        let n = fbm(p * 3.0 + seed);
        // Ridged noise: thin bright filaments and sheets.
        let ridge = 1.0 - abs(2.0 * fbm(p * 5.5 + seed * 0.7 + vec3<f32>(3.0)) - 1.0);
        // The young stars have blown a cavity; its walls glow brightest (the ionisation front).
        let shell = smoothstep(0.1, 0.32, r) * (1.0 - smoothstep(0.6, 1.0, r));
        let rim = exp(-pow((r - 0.36) / 0.09, 2.0));
        let gas = (max(n - 0.26, 0.0) * 2.4 + pow(ridge, 7.0) * 0.9 * n) * shell * (1.0 + 1.6 * rim);
        // Ionisation: hotter (teal, O III) near the stars, Hα pink-red further out.
        let hot = 1.0 - smoothstep(0.2, 0.55, r);
        let emit = mix(vec3<f32>(1.0, 0.22, 0.38), vec3<f32>(0.3, 0.95, 0.85), hot);
        // Dust: sharper, denser filaments that absorb what is behind them.
        let d = fbm(p * 4.0 + seed * 1.3 + vec3<f32>(5.0));
        let dust = max(d - 0.47, 0.0) * 10.0 * smoothstep(0.25, 0.55, r);
        col += emit * gas * trans * dt;
        trans *= exp(-dust * dt * 4.0);
    }
    // The cluster's glow at the heart.
    let tc = max(-b, 0.0);
    let closest = length(cam + dir * tc);
    col += vec3<f32>(0.75, 0.85, 1.0) * exp(-closest * closest * 900.0) * 0.5;
    return vec4<f32>(col * neb.params.x * 2.6, 0.0);
}
