//! Procedural planet surfaces, sampled on the unit sphere.
//!
//! **Authoritative for both simulation and rendering**: settlement placement, deposits and
//! the baked planet textures all call these functions, so a city drawn on the coast is on
//! the coast the simulation reasoned about.

use crate::astro::Body;
use crate::noise::{fbm3, ridged3};

/// Unit vector from latitude/longitude (radians). +Z is the rotation pole.
pub fn dir_from_lat_lon(lat: f64, lon: f64) -> [f64; 3] {
    let (sl, cl) = lat.sin_cos();
    let (so, co) = lon.sin_cos();
    [cl * co, cl * so, sl]
}

pub fn lat_lon_from_dir(d: [f64; 3]) -> (f64, f64) {
    (d[2].clamp(-1.0, 1.0).asin(), d[1].atan2(d[0]))
}

/// `n` roughly uniformly spread points on the unit sphere (Fibonacci lattice).
pub fn fibonacci_sphere(n: usize) -> impl Iterator<Item = [f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5.0f64.sqrt());
    (0..n).map(move |i| {
        let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
        let r = (1.0 - z * z).sqrt();
        let th = golden * i as f64;
        [r * th.cos(), r * th.sin(), z]
    })
}

fn scale(d: [f64; 3], s: f64) -> [f64; 3] {
    [d[0] * s, d[1] * s, d[2] * s]
}

/// Raw terrain elevation in roughly `[-1, 1.3]`; sea level is chosen per planet.
pub fn elevation(seed: u64, d: [f64; 3]) -> f64 {
    // Domain warping gives continents irregular, non-blobby outlines.
    let warp = [
        fbm3(seed ^ 0x11, scale(d, 1.1), 3, 2.0, 0.5),
        fbm3(seed ^ 0x22, scale(d, 1.1), 3, 2.0, 0.5),
        fbm3(seed ^ 0x33, scale(d, 1.1), 3, 2.0, 0.5),
    ];
    let p = [d[0] * 1.5 + 0.5 * warp[0], d[1] * 1.5 + 0.5 * warp[1], d[2] * 1.5 + 0.5 * warp[2]];
    let continents = fbm3(seed, p, 7, 2.0, 0.5);
    let mountain_mask = smoothstep(-0.02, 0.3, continents);
    let mountains = ridged3(seed, scale(d, 2.6), 5);
    continents + 0.55 * mountains * mountain_mask
}

pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Elevation threshold such that `ocean_fraction` of the surface lies below it.
pub fn sea_level_for(seed: u64, ocean_fraction: f64) -> f64 {
    if ocean_fraction <= 0.0 {
        return -10.0;
    }
    if ocean_fraction >= 1.0 {
        return 10.0;
    }
    let mut samples: Vec<f64> = fibonacci_sphere(2048).map(|d| elevation(seed, d)).collect();
    samples.sort_by(f64::total_cmp);
    let idx = ((samples.len() as f64 * ocean_fraction) as usize).min(samples.len() - 1);
    samples[idx]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Ocean,
    SeaIce,
    IceSheet,
    Tundra,
    Taiga,
    TemperateForest,
    Grassland,
    Desert,
    Savanna,
    Rainforest,
    Mountain,
    /// Lifeless rock.
    Barren,
}

impl Biome {
    /// Rough agricultural potential 0..1.
    pub fn fertility(self) -> f64 {
        match self {
            Biome::TemperateForest => 0.9,
            Biome::Grassland => 1.0,
            Biome::Savanna => 0.6,
            Biome::Rainforest => 0.55,
            Biome::Taiga => 0.25,
            Biome::Tundra => 0.08,
            Biome::Desert => 0.05,
            Biome::Mountain => 0.1,
            _ => 0.0,
        }
    }
    pub fn is_land(self) -> bool {
        !matches!(self, Biome::Ocean | Biome::SeaIce)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceSample {
    /// Height above sea level in terrain units (~1 ≈ highest mountains).
    pub height: f64,
    pub temperature: f64,
    pub moisture: f64,
    pub biome: Biome,
}

/// Everything needed to evaluate a surface without borrowing the whole universe.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceContext {
    pub seed: u64,
    pub sea_level: f64,
    pub mean_temperature: f64,
    pub has_liquid_water: bool,
    pub tidally_locked: bool,
    /// Whether land vegetation exists (biosphere at complex-ecosystem stage or later).
    pub vegetated: bool,
    pub axial_tilt: f64,
}

impl SurfaceContext {
    pub fn new(body: &Body, vegetated: bool) -> Self {
        Self {
            seed: body.terrain_seed,
            sea_level: body.sea_level,
            mean_temperature: body.temperature,
            has_liquid_water: body.hydro.ocean_fraction > 0.0,
            tidally_locked: body.tidally_locked,
            vegetated,
            axial_tilt: body.axial_tilt,
        }
    }

    pub fn sample(&self, d: [f64; 3]) -> SurfaceSample {
        let e = elevation(self.seed, d);
        let height = e - self.sea_level;

        // Latitudinal (or substellar, if locked) temperature structure.
        let gradient = 32.0 * (1.0 - 0.6 * (self.axial_tilt.sin().abs()));
        let insolation_term = if self.tidally_locked {
            // Substellar point at lon 0: hot day side, frozen night side.
            70.0 * (d[0] - 0.25)
        } else {
            gradient * (0.5 - 1.5 * d[2] * d[2])
        };
        let temperature = self.mean_temperature + insolation_term - 28.0 * height.max(0.0);

        let wet = fbm3(self.seed ^ 0x77, scale(d, 2.2), 4, 2.0, 0.55);
        // Wetter near the equator and mid-latitudes, drier in subtropics and high up.
        let lat = d[2].abs();
        let band = 0.15 * (1.0 - (lat - 0.0).abs() * 3.0).max(0.0) - 0.15 * (1.0 - ((lat - 0.45).abs() * 5.0)).max(0.0);
        let mut moisture = (0.5 + 0.6 * wet + band - 0.5 * height.max(0.0)).clamp(0.0, 1.0);
        if !self.has_liquid_water {
            moisture *= 0.2;
        }

        let biome = if height < 0.0 && self.has_liquid_water {
            if temperature < 271.0 {
                Biome::SeaIce
            } else {
                Biome::Ocean
            }
        } else if temperature < 255.0 {
            Biome::IceSheet
        } else if height > 0.55 {
            Biome::Mountain
        } else if !self.vegetated {
            Biome::Barren
        } else if temperature < 268.0 {
            Biome::Tundra
        } else if temperature < 279.0 {
            if moisture > 0.4 { Biome::Taiga } else { Biome::Tundra }
        } else if temperature < 294.0 {
            if moisture > 0.55 {
                Biome::TemperateForest
            } else if moisture > 0.3 {
                Biome::Grassland
            } else {
                Biome::Desert
            }
        } else if temperature < 320.0 {
            if moisture > 0.6 {
                Biome::Rainforest
            } else if moisture > 0.35 {
                Biome::Savanna
            } else {
                Biome::Desert
            }
        } else {
            Biome::Barren
        };

        SurfaceSample { height, temperature, moisture, biome }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sea_level_hits_requested_ocean_fraction() {
        let seed = 1234;
        for target in [0.3, 0.71, 0.9] {
            let sea = sea_level_for(seed, target);
            let n = 4000;
            let below = fibonacci_sphere(n).filter(|d| elevation(seed, *d) < sea).count();
            let frac = below as f64 / n as f64;
            assert!((frac - target).abs() < 0.03, "target {target} got {frac}");
        }
    }

    #[test]
    fn lat_lon_roundtrip() {
        let (lat, lon) = lat_lon_from_dir(dir_from_lat_lon(0.4, -2.0));
        assert!((lat - 0.4).abs() < 1e-12 && (lon + 2.0).abs() < 1e-12);
    }
}

/// Mountain-belt intensity at `d` (0..1), used to bias ore deposits towards orogenic zones.
pub fn ridged3_at(seed: u64, d: [f64; 3]) -> f64 {
    ridged3(seed, scale(d, 2.6), 5)
}
