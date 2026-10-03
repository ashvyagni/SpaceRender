//! Resource endowments and geographically placed deposits.
//!
//! Causal links (see SIMULATION.md):
//! * metals scale with stellar metallicity; ore *concentration* needs geological activity;
//! * U-235 decays (mean life ~1 Gyr), so older worlds have poorer uranium for fission;
//! * coal and oil are buried biomass and are produced by the biosphere over time — a world
//!   whose land life is young has little fossil fuel no matter how rich its rocks are.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use crate::astro::{Body, BodyKind, Deposit, ResourceKind, Resources};
use crate::planet::terrain::{fibonacci_sphere, lat_lon_from_dir, Terrain};
use crate::rng::Rng;

pub fn generate_resources(rng: &mut Rng, body: &Body, metallicity: f64, age_gyr: f64, abundance: f64) -> Resources {
    if !body.kind.has_surface() {
        return Resources::default();
    }
    let metal = 10f64.dpowf(metallicity) * abundance;
    let tect = body.geology.clamp(0.0, 3.0);
    let rocky = if body.kind == BodyKind::Rocky { 1.0 } else { 0.15 };
    let mut ln = |sd: f64| rng.normal(0.0, sd).dexp();
    let u235 = (-(age_gyr - 4.5) / 1.015).dexp().clamp(0.05, 8.0);
    Resources {
        iron: rocky * metal * ln(0.3),
        copper: rocky * metal * tect.sqrt() * ln(0.5),
        tin: rocky * metal * tect.dpowf(0.7) * ln(0.8),
        coal: 0.0,
        oil: 0.0,
        uranium: rocky * metal * tect.sqrt() * u235.min(3.0) * ln(0.5),
        rare_metals: rocky * metal * ln(0.6),
        fertile_land: 0.0,
        fresh_water: 0.0,
    }
}

/// Place deposits on land according to geology. Coal/oil sites are sedimentary basins whose
/// richness is multiplied by the planet's current fossil-fuel endowment when used.
pub fn generate_deposits(rng: &mut Rng, body: &Body) -> Vec<Deposit> {
    if !body.kind.has_surface() {
        return Vec::new();
    }
    let terrain = Terrain::of(body);
    let candidates: Vec<([f64; 3], f64, f64)> = fibonacci_sphere(900)
        .map(|d| {
            let h = terrain.elevation(d) - body.sea_level;
            let ridge = terrain.ridge(d);
            (d, h, ridge)
        })
        .collect();

    let mut out = Vec::new();
    for kind in ResourceKind::ALL {
        let weights: Vec<f64> = candidates
            .iter()
            .map(|&(_, h, ridge)| {
                let land = h > 0.0;
                match kind {
                    ResourceKind::Iron => if land { 1.0 } else { 0.0 },
                    ResourceKind::Copper | ResourceKind::Tin => if land { 0.2 + ridge * ridge * 3.0 } else { 0.0 },
                    ResourceKind::Coal => if land && h < 0.25 { 1.0 } else { 0.0 },
                    ResourceKind::Oil => if (-0.12..0.15).contains(&h) { 1.0 } else { 0.0 },
                    ResourceKind::Uranium => if land && h > 0.05 { 0.5 + ridge } else { 0.0 },
                    ResourceKind::RareMetals => if land { 0.1 + ridge * 2.0 } else { 0.0 },
                }
            })
            .collect();
        let count = match kind {
            ResourceKind::Iron => 10,
            ResourceKind::Tin => 4,
            ResourceKind::Uranium => 4,
            _ => 7,
        };
        for _ in 0..count {
            if let Some(i) = rng.weighted(&weights) {
                let (lat, lon) = lat_lon_from_dir(candidates[i].0);
                out.push(Deposit { kind, lat, lon, richness: rng.log_uniform(0.2, 3.0) });
            }
        }
    }
    out
}
