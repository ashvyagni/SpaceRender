// Impact flash: a fireball at the impact point (white-hot → yellow → dull red as it cools)
// and a shock ring racing outwards in the local tangent plane. Additive; drawn on a sphere
// bounding the effect, one face per pixel.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct FlashUniform {
    // xyz: impact point relative to the camera, w: outer radius (m)
    center: vec4<f32>,
    // xyz: surface normal, w: age 0..1
    normal: vec4<f32>,
    // x: fireball radius / outer, y: brightness, z: camera inside the bounds, w: planet radius / outer
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> flash: FlashUniform;

fn fire_colour(t: f32) -> vec3<f32> {
    // t: 1 = hottest.
    let white = vec3<f32>(1.0, 0.97, 0.9);
    let yellow = vec3<f32>(1.0, 0.72, 0.3);
    let red = vec3<f32>(0.9, 0.25, 0.06);
    return select(mix(red, yellow, t * 2.0), mix(yellow, white, t * 2.0 - 1.0), t > 0.5);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let inside = flash.params.z > 0.5;
    if (inside == is_front) {
        discard;
    }
    // Units of the outer radius (keeps f32 comfortable at any scale).
    let R = flash.center.w;
    let cam = (view.world_position - flash.center.xyz) / R;
    let dir = normalize((in.world_position.xyz - view.world_position) / R);
    let age = flash.normal.w;
    let n = normalize(flash.normal.xyz);

    // Fireball: grows fast, then cools and fades.
    let grow = 1.0 - pow(1.0 - min(age * 4.0, 1.0), 3.0);
    let rf = max(flash.params.x * (0.25 + 0.75 * grow), 1e-3);
    let tc = max(-dot(cam, dir), 0.0);
    let closest = cam + dir * tc;
    let d = length(closest) / rf;
    let heat = clamp(1.0 - age * 1.4, 0.0, 1.0);
    // Feather to zero before the bounding sphere so its edge never shows.
    let edge = 1.0 - smoothstep(0.55, 0.95, length(closest));
    let fire = (exp(-d * d * 2.5) * 2.5 + exp(-d * 1.6) * 0.5) * heat * heat * edge;
    var col = fire_colour(heat) * fire;

    // Shock ring hugging the planet: find where the view ray meets the surface and measure
    // the great-circle distance from the impact point.
    let rp = flash.params.w;
    let pc = -n * rp;
    let oc = cam - pc;
    let b = dot(oc, dir);
    let disc = b * b - (dot(oc, oc) - rp * rp);
    if (disc > 0.0) {
        let th = -b - sqrt(disc);
        if (th > 0.0) {
            let p = cam + dir * th;
            let ang = acos(clamp(dot(normalize(p - pc), n), -1.0, 1.0));
            let s = ang * rp;
            let rr = 1.0 - pow(1.0 - age, 2.4);
            let w = 0.02 + 0.06 * age;
            let ring = exp(-pow((s - rr) / w, 2.0)) * (1.0 - age) * (1.0 - age);
            // A hot glow inside the ring (the ejecta curtain and burning ground).
            let curtain = select(0.0, 0.25 * (1.0 - s / max(rr, 1e-3)) * (1.0 - age), s < rr);
            col += vec3<f32>(1.0, 0.8, 0.55) * ring * 1.5 + vec3<f32>(1.0, 0.45, 0.15) * curtain;
        }
    }
    return vec4<f32>(col * flash.params.y, 0.0);
}
