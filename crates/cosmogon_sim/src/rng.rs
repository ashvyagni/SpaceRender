//! Deterministic random streams.
//!
//! The simulation never keeps long-lived RNG state. Instead every random decision draws
//! from a short-lived stream keyed by `(universe seed, domain, keys…)`, e.g.
//! `(seed, CIV_STEP, civ_id, year_index)`. Consequences:
//!
//! * results do not depend on the order in which unrelated systems ran,
//! * nothing about RNG state has to be saved — a loaded game continues exactly as an
//!   uninterrupted one would,
//! * adding a new random consumer never perturbs existing ones.
//!
//! The generator is SplitMix64: pure integer arithmetic, identical on every platform.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
/// Stream domains. Never renumber existing values — that would change every universe.
pub mod domain {
    pub const GALAXY: u64 = 1;
    pub const STAR: u64 = 2;
    pub const PLANETS: u64 = 3;
    pub const MOONS: u64 = 4;
    pub const PLANET_PHYSICS: u64 = 5;
    pub const TERRAIN: u64 = 6;
    pub const RESOURCES: u64 = 7;
    pub const BIOSPHERE: u64 = 8;
    pub const SPECIES: u64 = 9;
    pub const CIV_STEP: u64 = 10;
    pub const NAMES: u64 = 11;
    pub const SETTLEMENT_SITES: u64 = 12;
    pub const PREHISTORY: u64 = 13;
    pub const CONTACT: u64 = 14;
}

#[inline]
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Combine two 64-bit values into a well-mixed hash.
#[inline]
pub fn mix(a: u64, b: u64) -> u64 {
    splitmix(a ^ splitmix(b).rotate_left(17))
}

/// Hash an arbitrary key path into a seed.
pub fn hash_keys(seed: u64, domain: u64, keys: &[u64]) -> u64 {
    let mut h = mix(seed, domain);
    for &k in keys {
        h = mix(h, k);
    }
    h
}

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: splitmix(seed) }
    }

    /// A stream for `(seed, domain, keys…)`.
    pub fn stream(seed: u64, domain: u64, keys: &[u64]) -> Self {
        Self::new(hash_keys(seed, domain, keys))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniform integer in `[lo, hi]` (inclusive).
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo + 1) as u64) as u32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// Log-uniform in `[lo, hi]` (both > 0).
    pub fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.dln() + (hi.dln() - lo.dln()) * self.f64()).dexp()
    }

    /// Standard normal via Box–Muller.
    pub fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        mean + sd * (-2.0 * u1.dln()).sqrt() * (std::f64::consts::TAU * u2).dcos()
    }

    /// Rayleigh-distributed value with scale `sigma`.
    pub fn rayleigh(&mut self, sigma: f64) -> f64 {
        sigma * (-2.0 * (1.0 - self.f64()).max(1e-300).dln()).sqrt()
    }

    /// Poisson sample (Knuth; fine for small lambda).
    pub fn poisson(&mut self, lambda: f64) -> u32 {
        let l = (-lambda).dexp();
        let mut k = 0u32;
        let mut p = 1.0;
        loop {
            p *= self.f64();
            if p <= l || k > 1000 {
                return k;
            }
            k += 1;
        }
    }

    /// Probability that an event with `rate` (per unit time) happens within `dt`.
    pub fn hazard(&mut self, rate: f64, dt: f64) -> bool {
        if rate <= 0.0 || dt <= 0.0 {
            return false;
        }
        self.f64() < 1.0 - (-rate * dt).dexp()
    }

    /// Pick an index with probability proportional to `weights`.
    pub fn weighted(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().filter(|w| **w > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut x = self.f64() * total;
        for (i, &w) in weights.iter().enumerate() {
            if w <= 0.0 {
                continue;
            }
            if x < w {
                return Some(i);
            }
            x -= w;
        }
        weights.iter().rposition(|w| *w > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_reproducible_and_independent() {
        let a: Vec<u64> = (0..8).map({ let mut r = Rng::stream(42, domain::STAR, &[3]); move |_| r.next_u64() }).collect();
        let b: Vec<u64> = (0..8).map({ let mut r = Rng::stream(42, domain::STAR, &[3]); move |_| r.next_u64() }).collect();
        let c: Vec<u64> = (0..8).map({ let mut r = Rng::stream(42, domain::STAR, &[4]); move |_| r.next_u64() }).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn f64_in_unit_interval_and_roughly_uniform() {
        let mut r = Rng::new(7);
        let n = 100_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let x = r.f64();
            assert!((0.0..1.0).contains(&x));
            sum += x;
        }
        assert!((sum / n as f64 - 0.5).abs() < 0.01);
    }

    #[test]
    fn weighted_respects_zero_weights() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            assert_eq!(r.weighted(&[0.0, 1.0, 0.0]), Some(1));
        }
        assert_eq!(r.weighted(&[0.0, 0.0]), None);
    }
}
