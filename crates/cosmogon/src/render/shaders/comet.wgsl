// A comet: green coma, straight streaky blue ion tail, broad curved yellow-white dust tail.
// Drawn on a billboard through the nucleus that contains the ion-tail axis. Additive.

#import bevy_pbr::forward_io::VertexOutput
#import cosmogon::common::vnoise

struct CometUniform {
    // xyz: nucleus relative to the camera, w: coma radius (m)
    center: vec4<f32>,
    // xyz: unit ion-tail axis, w: length (m)
    ion: vec4<f32>,
    // xyz: unit dust-tail axis, w: length (m)
    dust: vec4<f32>,
    // xyz: unit bend direction of the dust tail, w: bend amount
    lag: vec4<f32>,
    // x: brightness, y: seed, z: time (s)
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> comet: CometUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Work in units of the ion-tail length so nothing overflows or underflows in f32.
    let L = comet.ion.w;
    let rel = (in.world_position.xyz - comet.center.xyz) / L;
    let seed = comet.params.y;
    let act = comet.params.x;

    // Coma: bright core with a 1/r halo.
    let rc = comet.center.w / L;
    let r = length(rel);
    let coma = exp(-r * r / (rc * rc)) * 3.0 + rc * 0.25 / (r + rc * 0.15);

    // Ion tail: narrow, straight, with streamers that drift along it.
    let ai = comet.ion.xyz;
    let si = dot(rel, ai);
    var ion = 0.0;
    if (si > 0.0) {
        let perp = rel - ai * si;
        let w = 0.008 + si * 0.05;
        let pl = length(perp);
        let streak = 0.6 + 0.8 * vnoise(vec3<f32>(pl / w * 3.0, si * 1.5 - comet.params.z * 0.02, seed));
        ion = exp(-pl * pl / (w * w)) * exp(-si * 1.4) * streak / (1.0 + 4.0 * w) * (1.0 - smoothstep(1.6, 2.5, si));
    }

    // Dust tail: a widening fan bending back along the orbit.
    let ad = comet.dust.xyz;
    let ld = comet.dust.w / L;
    let sd = dot(rel, ad);
    var dust = 0.0;
    if (sd > 0.0) {
        let bend = comet.lag.xyz * comet.lag.w * sd * sd / ld;
        let off = rel - ad * sd - bend;
        let w = 0.015 + sd * 0.22;
        let d2 = dot(off, off);
        let grain = 0.85 + 0.3 * vnoise(vec3<f32>(sd * 14.0, length(off) / w * 3.0, seed + 3.0));
        dust = exp(-d2 / (w * w)) * exp(-sd / (ld * 0.6)) * grain / (1.0 + 3.0 * w) * (1.0 - smoothstep(1.5, 2.4, sd));
    }

    let col = vec3<f32>(0.55, 1.0, 0.75) * coma * 0.6
            + vec3<f32>(0.32, 0.55, 1.0) * ion * 1.4
            + vec3<f32>(1.0, 0.86, 0.62) * dust * 1.8;
    return vec4<f32>(col * (0.6 + act) * 3.0, 0.0);
}
