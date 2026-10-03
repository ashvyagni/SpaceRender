//! Procedural planet surfaces, sampled on the unit sphere.
//!
//! **Authoritative for both simulation and rendering**: settlement placement, deposits and
//! the baked planet textures all call these functions, so a city drawn on the coast is on
//! the coast the simulation reasoned about.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::astro::Body;
use crate::noise::{fbm3, ridged3};

/// Measured elevation datasets that can replace procedural terrain.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElevationData {
    /// NOAA ETOPO5 global relief (land and sea floor), downsampled to 2048×1024.
    Earth,
}

/// Metres of real relief per terrain unit. Chosen so the terrain's temperature lapse
/// (28 K per unit) matches Earth's ~6.5 K/km.
pub const METRES_PER_UNIT: f64 = 4300.0;

const EARTH_W: usize = 2048;
const EARTH_H: usize = 1024;
static EARTH_BYTES: &[u8] = include_bytes!("../../data/earth_elevation.bin");

fn earth_grid() -> &'static [i16] {
    static GRID: OnceLock<Vec<i16>> = OnceLock::new();
    GRID.get_or_init(|| EARTH_BYTES.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect())
}

/// Bilinearly interpolated real elevation (metres) at a unit direction.
pub fn earth_elevation_m(d: [f64; 3]) -> f64 {
    let g = earth_grid();
    let (lat, lon) = lat_lon_from_dir(d);
    let x = lon.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * EARTH_W as f64 - 0.5;
    let y = ((std::f64::consts::FRAC_PI_2 - lat) / std::f64::consts::PI * EARTH_H as f64 - 0.5).clamp(0.0, (EARTH_H - 1) as f64);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let xi = |dx: i64| ((x0 as i64 + dx).rem_euclid(EARTH_W as i64)) as usize;
    let yi = |dy: usize| (y0 as usize + dy).min(EARTH_H - 1);
    let at = |xx: usize, yy: usize| g[yy * EARTH_W + xx] as f64;
    let top = at(xi(0), yi(0)) * (1.0 - fx) + at(xi(1), yi(0)) * fx;
    let bottom = at(xi(0), yi(1)) * (1.0 - fx) + at(xi(1), yi(1)) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// The terrain of one body: procedural from a seed, or measured data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terrain {
    pub seed: u64,
    pub data: Option<ElevationData>,
}

impl Terrain {
    pub fn of(body: &Body) -> Self {
        Self { seed: body.terrain_seed, data: body.elevation_data }
    }

    pub fn procedural(seed: u64) -> Self {
        Self { seed, data: None }
    }

    /// Elevation in terrain units (≈ −1…1.3 for procedural worlds; metres / 4300 for data).
    pub fn elevation(&self, d: [f64; 3]) -> f64 {
        match self.data {
            None => elevation(self.seed, d),
            // Measured relief plus faint sub-grid detail so close views aren't flat.
            Some(ElevationData::Earth) => earth_elevation_m(d) / METRES_PER_UNIT + 0.012 * fbm3(self.seed, scale(d, 60.0), 4, 2.0, 0.5),
        }
    }

    /// Mountain-belt intensity (0..1), used to bias ore deposits.
    pub fn ridge(&self, d: [f64; 3]) -> f64 {
        match self.data {
            None => ridged3_at(self.seed, d),
            Some(_) => ((self.elevation(d) - 0.2) / 0.8).clamp(0.0, 1.0),
        }
    }

    /// Elevation threshold such that `ocean_fraction` of the surface lies below it.
    pub fn sea_level_for(&self, ocean_fraction: f64) -> f64 {
        if ocean_fraction <= 0.0 {
            return -10.0;
        }
        if ocean_fraction >= 1.0 {
            return 10.0;
        }
        let mut samples: Vec<f64> = fibonacci_sphere(4096).map(|d| self.elevation(d)).collect();
        samples.sort_by(f64::total_cmp);
        let idx = ((samples.len() as f64 * ocean_fraction) as usize).min(samples.len() - 1);
        samples[idx]
    }
}

/// Unit vector from latitude/longitude (radians). +Z is the rotation pole.
pub fn dir_from_lat_lon(lat: f64, lon: f64) -> [f64; 3] {
    let (sl, cl) = lat.dsin_cos();
    let (so, co) = lon.dsin_cos();
    [cl * co, cl * so, sl]
}

pub fn lat_lon_from_dir(d: [f64; 3]) -> (f64, f64) {
    (d[2].clamp(-1.0, 1.0).dasin(), d[1].datan2(d[0]))
}

/// `n` roughly uniformly spread points on the unit sphere (Fibonacci lattice).
pub fn fibonacci_sphere(n: usize) -> impl Iterator<Item = [f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5.0f64.sqrt());
    (0..n).map(move |i| {
        let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
        let r = (1.0 - z * z).sqrt();
        let th = golden * i as f64;
        [r * th.dcos(), r * th.dsin(), z]
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
    pub terrain: Terrain,
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
            terrain: Terrain::of(body),
            sea_level: body.sea_level,
            mean_temperature: body.temperature,
            has_liquid_water: body.hydro.ocean_fraction > 0.0,
            tidally_locked: body.tidally_locked,
            vegetated,
            axial_tilt: body.axial_tilt,
        }
    }

    pub fn sample(&self, d: [f64; 3]) -> SurfaceSample {
        let e = self.terrain.elevation(d);
        let height = e - self.sea_level;

        // Latitudinal (or substellar, if locked) temperature structure.
        let gradient = 32.0 * (1.0 - 0.6 * (self.axial_tilt.dsin().abs()));
        let insolation_term = if self.tidally_locked {
            // Substellar point at lon 0: hot day side, frozen night side.
            70.0 * (d[0] - 0.25)
        } else {
            gradient * (0.5 - 1.5 * d[2] * d[2])
        };
        let temperature = self.mean_temperature + insolation_term - 28.0 * height.max(0.0);

        let wet = fbm3(self.terrain.seed ^ 0x77, scale(d, 2.2), 4, 2.0, 0.55);
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
    fn real_earth_data_is_oriented_correctly() {
        let at = |lat: f64, lon: f64| earth_elevation_m(dir_from_lat_lon(lat.to_radians(), lon.to_radians()));
        assert!(at(28.0, 86.9) > 4000.0, "Himalaya {}", at(28.0, 86.9));
        assert!(at(11.35, 142.2) < -6000.0, "Mariana {}", at(11.35, 142.2));
        assert!(at(0.0, -150.0) < -3000.0, "Pacific");
        assert!(at(-80.0, 0.0) > 1500.0, "Antarctic ice");
        assert!(at(48.86, 2.35) > 0.0 && at(48.86, 2.35) < 500.0, "Paris");
        // With Earth's ocean fraction the derived sea level sits near 0 m.
        let t = Terrain { seed: 1, data: Some(ElevationData::Earth) };
        let sea_m = t.sea_level_for(0.71) * METRES_PER_UNIT;
        assert!(sea_m.abs() < 250.0, "sea level {sea_m} m");
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
