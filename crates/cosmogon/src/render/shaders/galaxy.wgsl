// A disk galaxy, ray-marched through its volume: exponential disk, logarithmic spiral
// arms with young blue stars and pink star-forming knots, a central bar and bulge of old
// yellow stars, and dust lanes that absorb light along the arms' inner edges. Distances in
// light-years in the galaxy's own frame (z = rotation axis). Additive.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import cosmogon::common::{vnoise, quat_rotate}

struct GalaxyUniform {
    // xyz: centre relative to the camera (m), w: metres per light-year
    center: vec4<f32>,
    // World → galaxy rotation (quaternion)
    orient: vec4<f32>,
    // x: disk radius (ly), y: disk scale length (ly), z: arm count, w: arm pitch (rad)
    shape: vec4<f32>,
    // x: bar half-length (ly), y: bulge radius (ly), z: brightness, w: kind (0 spiral, 1 irregular)
    look: vec4<f32>,
    // x: arm phase (rad), y: seed, z: thickness (ly), w: unused
    extra: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gal: GalaxyUniform;

fn wrap_pi(a: f32) -> f32 {
    return a - 6.2831853 * floor((a + 3.1415927) / 6.2831853);
}

// Emission (rgb) and dust (w) density at p (ly, galaxy frame).
fn sample(p: vec3<f32>) -> vec4<f32> {
    let rmax = gal.shape.x;
    let hr = gal.shape.y;
    let r = length(p.xy);
    let z = abs(p.z);
    let th = gal.extra.z;
    let theta = atan2(p.y, p.x);
    let seed = gal.extra.y;
    // Arms.
    let arms = gal.shape.z;
    let pitch = gal.shape.w;
    var arm = 0.0;
    var dust_arm = 0.0;
    if (arms > 0.5) {
        // Arms start at the bar ends (or 2 000 ly without a bar) and wind outward.
        let r0 = max(gal.look.x * 0.5, 2000.0);
        let phi = log(max(r, r0 * 0.5) / r0) / tan(pitch) + gal.extra.x;
        for (var k = 0; k < 4; k++) {
            if (f32(k) >= arms) {
                break;
            }
            // Two major arms (k even) and two weaker ones between them.
            let strength = select(1.0, 0.45, k % 2 == 1);
            let d = wrap_pi(theta - phi - f32(k) * 6.2831853 / arms);
            arm = max(arm, strength * exp(-d * d / 0.16));
            // Dust lanes sit on the inner (trailing) edge of each arm.
            let dd = wrap_pi(d + 0.22);
            dust_arm = max(dust_arm, strength * exp(-dd * dd / 0.03));
        }
    }
    let irregular = gal.look.w > 0.5;
    let clump = vnoise(p * 0.0006 + vec3<f32>(seed)) * 0.5 + 0.5;
    let fine = vnoise(p * 0.004 + vec3<f32>(seed * 1.7)) * 0.5 + 0.5;
    let disk = exp(-r / hr) * exp(-z / th) * (1.0 - smoothstep(rmax * 0.75, rmax, r));
    var young = disk * (0.06 + 2.2 * arm) * (0.6 + 0.8 * fine);
    if (irregular) {
        young = disk * (0.3 + 1.4 * clump * clump);
    }
    // Star-forming knots (H II regions) along the arms.
    let knots = pow(clamp(fine * 1.25 - 0.55, 0.0, 1.0), 2.0) * (arm + select(0.0, clump, irregular)) * disk * 14.0;
    // Bar and bulge of old stars.
    let bar_axis = vec2<f32>(cos(gal.extra.x + 0.47), sin(gal.extra.x + 0.47));
    let along = dot(p.xy, bar_axis);
    let across = dot(p.xy, vec2<f32>(-bar_axis.y, bar_axis.x));
    let bl = gal.look.x;
    let bar = select(0.0, exp(-(along * along) / (bl * bl * 0.5) - (across * across + z * z * 4.0) / (bl * bl * 0.04)), bl > 1.0);
    let rb = gal.look.y;
    let bulge = exp(-(r * r + z * z * 2.5) / (rb * rb)) * 3.0;
    let old = disk * 0.35 * (0.4 + arm) + bar * 1.5 + bulge;
    let emit = vec3<f32>(0.38, 0.58, 1.0) * young + vec3<f32>(1.0, 0.35, 0.55) * knots + vec3<f32>(1.0, 0.82, 0.6) * old;
    let dust = disk * exp(-z / (th * 0.35)) * (0.25 + 2.2 * dust_arm) * (0.5 + fine);
    return vec4<f32>(emit, dust);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let ly = gal.center.w;
    let q = gal.orient;
    // Camera and ray in the galaxy frame (ly).
    let cam = quat_rotate(q, view.world_position - gal.center.xyz) / ly;
    // Scale to light-years before normalising: |x|² of metre-scale vectors (~1e21) overflows f32.
    let dir = normalize(quat_rotate(q, (in.world_position.xyz - view.world_position) / ly));
    let rmax = gal.shape.x;
    let hz = max(gal.extra.z * 6.0, 1500.0);
    // Ray–sphere (radius rmax) interval, then clip to the disk slab |z| < hz.
    let bq = dot(cam, dir);
    let cq = dot(cam, cam) - rmax * rmax;
    let disc = bq * bq - cq;
    if (disc <= 0.0) {
        discard;
    }
    let sq = sqrt(disc);
    var t0 = max(-bq - sq, 0.0);
    var t1 = -bq + sq;
    if (abs(dir.z) > 1e-5) {
        let a = (-hz - cam.z) / dir.z;
        let b = (hz - cam.z) / dir.z;
        t0 = max(t0, min(a, b));
        t1 = min(t1, max(a, b));
    } else if (abs(cam.z) > hz) {
        discard;
    }
    if (t1 <= t0) {
        discard;
    }
    // One pass per pixel: the sphere's front face from outside, its back face from inside.
    let inside = dot(cam, cam) < rmax * rmax;
    if (inside == is_front) {
        discard;
    }
    let steps = 56;
    let dt = (t1 - t0) / f32(steps);
    var col = vec3<f32>(0.0);
    var trans = 1.0;
    for (var i = 0; i < steps; i++) {
        let p = cam + dir * (t0 + (f32(i) + 0.5) * dt);
        let smp = sample(p);
        col += smp.rgb * trans * dt;
        trans *= exp(-smp.w * dt * 0.0022);
    }
    return vec4<f32>(col * gal.look.z * 0.00016, 0.0);
}
