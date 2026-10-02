//! Deterministic 3D gradient noise and fractal sums.
//!
//! Planet surfaces are sampled on the unit sphere in 3D so there are no seams or polar
//! pinching. The same functions are used by the simulation (to place settlements and
//! deposits) and by the renderer (to bake textures), which guarantees that what you see is
//! what the simulation used.

use crate::rng::mix;

#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lattice_hash(seed: u64, x: i64, y: i64, z: i64) -> u64 {
    mix(mix(mix(seed, x as u64), y as u64), z as u64)
}

/// Pseudo-random gradient from the 12 cube-edge directions.
#[inline]
fn grad(h: u64, x: f64, y: f64, z: f64) -> f64 {
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

/// Perlin-style gradient noise in roughly `[-1, 1]`.
pub fn gradient3(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let (xi, yi, zi) = (xi as i64, yi as i64, zi as i64);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));

    let mut acc = [0.0f64; 8];
    for (i, a) in acc.iter_mut().enumerate() {
        let dx = (i & 1) as i64;
        let dy = ((i >> 1) & 1) as i64;
        let dz = ((i >> 2) & 1) as i64;
        let h = lattice_hash(seed, xi + dx, yi + dy, zi + dz);
        *a = grad(h, xf - dx as f64, yf - dy as f64, zf - dz as f64);
    }
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let x00 = lerp(acc[0], acc[1], u);
    let x10 = lerp(acc[2], acc[3], u);
    let x01 = lerp(acc[4], acc[5], u);
    let x11 = lerp(acc[6], acc[7], u);
    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);
    lerp(y0, y1, w)
}

/// Fractal Brownian motion: `octaves` layers of gradient noise. Output roughly `[-1, 1]`.
pub fn fbm3(seed: u64, p: [f64; 3], octaves: u32, lacunarity: f64, gain: f64) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        let s = mix(seed, o as u64);
        sum += amp * gradient3(s, p[0] * freq, p[1] * freq, p[2] * freq);
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    sum / norm
}

/// Ridged multifractal — good for mountain chains. Output roughly `[0, 1]`.
pub fn ridged3(seed: u64, p: [f64; 3], octaves: u32) -> f64 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        let s = mix(seed ^ 0xA5A5, o as u64);
        let n = 1.0 - gradient3(s, p[0] * freq, p[1] * freq, p[2] * freq).abs();
        sum += amp * n * n;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        for i in 0..1000 {
            let p = [i as f64 * 0.37, i as f64 * 0.11, -(i as f64) * 0.23];
            let a = fbm3(9, p, 6, 2.0, 0.5);
            assert_eq!(a, fbm3(9, p, 6, 2.0, 0.5));
            assert!(a.abs() <= 1.5, "{a}");
        }
    }

    #[test]
    fn noise_is_zero_at_lattice_points() {
        assert_eq!(gradient3(1, 3.0, -2.0, 5.0), 0.0);
    }
}
