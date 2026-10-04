// The distant universe: thousands of galaxies along a cosmic web and bright quasars, each
// a randomly oriented quad drawn procedurally. Vertex colour: rgb tint, a = kind
// (0..0.33 elliptical, 0.33..0.66 spiral, > 0.66 quasar). Additive.

#import bevy_pbr::forward_io::VertexOutput

struct DeepUniform {
    // x: overall brightness (fades in when zoomed out)
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> deep: DeepUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    let c = in.color;
#else
    let c = vec4<f32>(1.0);
#endif
    let p = in.uv * 2.0 - 1.0;
    let r = length(p);
    if (r > 1.0) {
        discard;
    }
    let kind = c.a;
    var v = 0.0;
    var tint = c.rgb;
    if (kind < 0.33) {
        // Elliptical: smooth de Vaucouleurs-like glow.
        v = exp(-7.0 * pow(r, 0.5)) * 6.0;
    } else if (kind < 0.66) {
        // Spiral: core plus two log-spiral arms.
        let th = atan2(p.y, p.x);
        let phi = log(max(r, 0.02)) * 3.2 + kind * 40.0;
        let arm = pow(0.5 + 0.5 * cos(2.0 * (th - phi)), 4.0);
        v = exp(-r * r * 30.0) * 3.0 + arm * exp(-r * 3.0) * 0.9;
        tint = mix(vec3<f32>(1.0, 0.85, 0.6), c.rgb, smoothstep(0.05, 0.4, r));
    } else {
        // Quasar: an unresolved, blinding blue-white core with a faint jet.
        let core = exp(-r * r * 400.0) * 40.0;
        let jet = exp(-p.x * p.x * 2000.0) * exp(-abs(p.y) * 3.0) * 1.5;
        v = core + jet;
        tint = vec3<f32>(0.75, 0.85, 1.0);
    }
    return vec4<f32>(tint * v * deep.params.x, 0.0);
}
