use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Create a deterministic PRNG seeded with the given value.
///
/// Every call with the same `seed` produces an identical sequence, making
/// procedural content reproducible across sessions.
pub fn seeded_rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// Hash a 2-D coordinate pair into a pseudo-random `f64` in `[0.0, 1.0)`.
///
/// Combines `seed`, `x`, and `y` deterministically so that nearby
/// coordinates do **not** produce nearby values (good for noise seeds).
pub fn hash_f64(seed: u64, x: f64, y: f64) -> f64 {
    let bits = (seed ^ (x.to_bits()).wrapping_mul(0x9E3779B97F4A7C15))
        ^ (y.to_bits()).wrapping_mul(0xBF58476D1CE4E5B9);
    // Map u64 -> [0, 1)
    (bits >> 11) as f64 / ((1u64 << 53) as f64)
}

/// Value noise at a single lattice point (deterministic from seed).
fn value_at(seed: u64, ix: i64, iy: i64) -> f64 {
    hash_f64(seed, ix as f64, iy as f64)
}

/// Bilinear interpolation between four lattice values.
fn bilinear(v00: f64, v10: f64, v01: f64, v11: f64, tx: f64, ty: f64) -> f64 {
    let a = v00 + (v10 - v00) * tx;
    let b = v01 + (v11 - v01) * tx;
    a + (b - a) * ty
}

/// Smooth hermite interpolation factor.
fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Evaluate fractal value noise at continuous coordinates `(x, y)`.
///
/// `octaves` controls the number of frequency layers (more = finer detail).
/// Returns a value roughly in `[0.0, 1.0]`.
pub fn terrain_height(seed: u64, lat: f64, lon: f64, octaves: u32) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amp = 0.0;

    for i in 0..octaves {
        let layer_seed = seed.wrapping_add(i as u64 * 0x9E3779B9);
        let x = lon * frequency;
        let y = lat * frequency;

        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let tx = smooth(x - x.floor());
        let ty = smooth(y - y.floor());

        let v00 = value_at(layer_seed, ix, iy);
        let v10 = value_at(layer_seed, ix + 1, iy);
        let v01 = value_at(layer_seed, ix, iy + 1);
        let v11 = value_at(layer_seed, ix + 1, iy + 1);

        value += bilinear(v00, v10, v01, v11, tx, ty) * amplitude;
        max_amp += amplitude;

        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if max_amp > 0.0 {
        value / max_amp
    } else {
        0.5
    }
}

/// Generate a procedural color variation for a planet surface.
///
/// Shifts the `base_color` slightly based on latitude: poles are lighter
/// (icer), equator is slightly darker (warmer terrain).
pub fn procedural_color(
    seed: u64,
    lat: f64,
    lon: f64,
    base_color: [f32; 3],
) -> [f32; 3] {
    let lat_factor = lat.abs() / (std::f64::consts::FRAC_PI_2); // 0 at equator, 1 at poles
    let noise = hash_f64(seed, lat * 10.0, lon * 10.0) * 0.1 - 0.05;

    let brighten = (lat_factor as f32) * 0.25 + noise as f32;

    [
        (base_color[0] + brighten).clamp(0.0, 1.0),
        (base_color[1] + brighten).clamp(0.0, 1.0),
        (base_color[2] + brighten).clamp(0.0, 1.0),
    ]
}

/// Compute a procedural cloud density at a surface point.
///
/// Returns a value in `[0.0, 1.0]` where 0 is clear sky and 1 is full
/// cloud cover. `time` is in seconds and drives a slow drift.
pub fn cloud_density(seed: u64, lat: f64, lon: f64, time: f64) -> f64 {
    let drift = time * 0.00001; // very slow eastward drift
    let x = lon + drift;
    let y = lat;
    let n = terrain_height(seed, y, x, 4);
    // Apply a contrast curve so clouds are more distinct
    let n = (n * 2.0 - 1.0).abs();
    let n = n * n;
    n.clamp(0.0, 1.0)
}
