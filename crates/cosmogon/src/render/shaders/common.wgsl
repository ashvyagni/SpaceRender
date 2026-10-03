// Shared helpers for Cosmogon's planet, terrain, ring and star shaders.

#define_import_path cosmogon::common

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

// Value noise in [-1, 1].
fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash3(i);
    let b = hash3(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash3(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash3(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash3(i + vec3<f32>(0.0, 0.0, 1.0));
    let g = hash3(i + vec3<f32>(1.0, 0.0, 1.0));
    let h = hash3(i + vec3<f32>(0.0, 1.0, 1.0));
    let k = hash3(i + vec3<f32>(1.0, 1.0, 1.0));
    let x0 = mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
    let x1 = mix(mix(e, g, u.x), mix(h, k, u.x), u.y);
    return mix(x0, x1, u.z) * 2.0 - 1.0;
}

fn fbm4(p: vec3<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        sum += amp * vnoise(q);
        q = q * 2.07 + vec3<f32>(1.7, 9.2, 3.1);
        amp *= 0.5;
    }
    return sum;
}

// Unit direction in the body frame (+Z pole) from equirectangular UVs.
fn dir_from_uv(uv: vec2<f32>) -> vec3<f32> {
    let lon = uv.x * 6.2831853;
    let colat = uv.y * 3.1415927;
    let s = sin(colat);
    return vec3<f32>(s * cos(lon), s * sin(lon), cos(colat));
}

// Fraction of the star's disc still visible from `p` (planet-radius units, relative to the
// planet centre) past up to four spherical occluders. Soft penumbra from the star's
// angular radius `sun_ang`.
fn eclipse_light(p: vec3<f32>, l: vec3<f32>, occ: array<vec4<f32>, 4>, sun_ang: f32) -> f32 {
    var lit = 1.0;
    for (var i = 0; i < 4; i++) {
        let o = occ[i];
        if (o.w > 0.0) {
            let to = o.xyz - p;
            let t = dot(to, l);
            if (t > 0.0) {
                let sep = length(to - l * t) / t;
                let ro = o.w / t;
                let rs = max(sun_ang, 1e-5);
                // Linear ramp across the penumbra, capped by the area ratio (annular eclipses).
                let cover = clamp((ro + rs - sep) / (2.0 * min(ro, rs)), 0.0, 1.0) * min(1.0, (ro * ro) / (rs * rs));
                lit *= 1.0 - cover;
            }
        }
    }
    return lit;
}

// Bump-map a normal by a scalar height field using screen-space derivatives
// (Mikkelsen 2010, "Bump Mapping Unparametrized Surfaces on the GPU").
fn perturb_normal(n: vec3<f32>, pos: vec3<f32>, height: f32) -> vec3<f32> {
    let dpx = dpdx(pos);
    let dpy = dpdy(pos);
    let dhx = dpdx(height);
    let dhy = dpdy(height);
    let r1 = cross(dpy, n);
    let r2 = cross(n, dpx);
    let det = dot(dpx, r1);
    let grad = sign(det) * (dhx * r1 + dhy * r2);
    let m = abs(det) * n - grad;
    if (dot(m, m) < 1e-20) {
        return n;
    }
    return normalize(m);
}

// Rotate vector `v` by unit quaternion `q` (xyz, w).
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}
