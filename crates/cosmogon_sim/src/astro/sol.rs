//! The real Solar System, loaded from `data/sol.toml`.

use std::collections::HashMap;

use serde::Deserialize;

use super::{Atmosphere, Body, BodyKind, Hydrosphere, Orbit, Resources, Rings, Star, StarSystem, AU};
use crate::planet::environment::calibrate_real_body;
use crate::planet::{resources, terrain};
use crate::rng::{domain, Rng};
use crate::time::{SECONDS_PER_GYR, SECONDS_PER_YEAR};
use crate::Vec3d;

const SOL_TOML: &str = include_str!("../../data/sol.toml");
/// Fixed seed so the real Solar System is identical in every universe.
const SOL_SEED: u64 = 0x5011_5011;

#[derive(Deserialize)]
struct SolFile {
    star: SolStar,
    body: Vec<SolBody>,
}

#[derive(Deserialize)]
struct SolStar {
    name: String,
    mass: f64,
    age_gyr: f64,
    metallicity: f64,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SolBody {
    name: String,
    kind: String,
    parent: Option<String>,
    a_au: Option<f64>,
    a_km: Option<f64>,
    e: f64,
    i: f64,
    node: f64,
    peri: f64,
    m0: f64,
    mass: f64,
    radius: f64,
    /// None => tidally locked to parent.
    rotation_h: Option<f64>,
    tilt: f64,
    albedo: f64,
    temperature: f64,
    pressure: f64,
    n2: f64,
    o2: f64,
    co2: f64,
    h2o: f64,
    ch4: f64,
    h2he: f64,
    water: f64,
    ocean: f64,
    ice: f64,
    subsurface_ocean: bool,
    magnetic: f64,
    geology: f64,
    color: [f32; 3],
    resources: Option<HashMap<String, f64>>,
    rings: Option<Rings>,
    glacial_until_years: Option<f64>,
    elevation_data: Option<String>,
}

/// Build the Sol system (at J2000; the star's age is relative to t = 0).
pub fn sol_system() -> StarSystem {
    let file: SolFile = toml::from_str(SOL_TOML).expect("embedded data/sol.toml must parse");
    let star = Star::from_mass(
        file.star.name.clone(),
        file.star.mass,
        file.star.metallicity,
        -file.star.age_gyr * SECONDS_PER_GYR,
    );
    let age_gyr = file.star.age_gyr;

    let mut bodies: Vec<Body> = Vec::new();
    for (idx, sb) in file.body.iter().enumerate() {
        let parent = sb.parent.as_ref().map(|p| {
            file.body.iter().position(|b| &b.name == p).unwrap_or_else(|| panic!("unknown parent {p}")) as u32
        });
        let a = match (sb.a_au, sb.a_km) {
            (Some(au), _) => au * AU,
            (None, Some(km)) => km * 1000.0,
            _ => panic!("{}: needs a_au or a_km", sb.name),
        };
        let kind = match sb.kind.as_str() {
            "Rocky" => BodyKind::Rocky,
            "Icy" => BodyKind::Icy,
            "GasGiant" => BodyKind::GasGiant,
            "IceGiant" => BodyKind::IceGiant,
            k => panic!("unknown body kind {k}"),
        };
        let orbit = Orbit {
            a,
            e: sb.e,
            i: sb.i.to_radians(),
            node: sb.node.to_radians(),
            peri: sb.peri.to_radians(),
            m0: sb.m0.to_radians(),
        };
        let mu_parent = match parent {
            Some(p) => super::G * file.body[p as usize].mass,
            None => star.mu(),
        };
        let (rotation_period, tidally_locked) = match sb.rotation_h {
            Some(h) => (h * 3600.0, false),
            None => (orbit.period(mu_parent), true),
        };
        let atmosphere = Atmosphere {
            pressure_bar: sb.pressure,
            n2: sb.n2,
            o2: sb.o2,
            co2: sb.co2,
            h2o: sb.h2o,
            ch4: sb.ch4,
            h2he: sb.h2he,
        };
        let mut body = Body {
            id: idx as u32,
            name: sb.name.clone(),
            kind,
            parent,
            orbit,
            mass: sb.mass,
            radius: sb.radius,
            rotation_period,
            axial_tilt: sb.tilt.to_radians(),
            tidally_locked,
            albedo: sb.albedo,
            atmosphere,
            hydro: Hydrosphere {
                water_inventory: sb.water,
                ocean_fraction: sb.ocean,
                ice_fraction: sb.ice,
                subsurface_ocean: sb.subsurface_ocean,
            },
            magnetic_field: sb.magnetic,
            geology: sb.geology,
            equilibrium_temperature: sb.temperature,
            temperature: sb.temperature,
            resources: Resources::default(),
            deposits: Vec::new(),
            rings: sb.rings.clone(),
            terrain_seed: crate::rng::hash_keys(SOL_SEED, domain::TERRAIN, &[idx as u64]),
            sea_level: 0.0,
            color: sb.color,
            real: true,
            climate_bias: 0.0,
            glacial_cycle_years: 0.0,
            interglacial_fraction: 1.0,
            glacial_phase_years: 0.0,
            glacial_until_years: sb.glacial_until_years,
            class: None,
            provenance: Default::default(),
            removed: None,
            impacts: Vec::new(),
            impact_winter: None,
            accretion: 0.0,
            melt: None,
            elevation_data: match sb.elevation_data.as_deref() {
                Some("earth") => Some(crate::planet::terrain::ElevationData::Earth),
                Some(other) => panic!("unknown elevation_data {other}"),
                None => None,
            },
        };
        let mut rng = Rng::stream(SOL_SEED, domain::RESOURCES, &[idx as u64]);
        body.resources = resources::generate_resources(&mut rng, &body, file.star.metallicity, age_gyr, 1.0);
        if let Some(r) = &sb.resources {
            for (k, v) in r {
                match k.as_str() {
                    "iron" => body.resources.iron = *v,
                    "copper" => body.resources.copper = *v,
                    "tin" => body.resources.tin = *v,
                    "coal" => body.resources.coal = *v,
                    "oil" => body.resources.oil = *v,
                    "uranium" => body.resources.uranium = *v,
                    "rare_metals" => body.resources.rare_metals = *v,
                    other => panic!("unknown resource {other}"),
                }
            }
        }
        body.sea_level = terrain::Terrain::of(&body).sea_level_for(body.hydro.ocean_fraction);
        bodies.push(body);
    }

    let mut system = StarSystem {
        id: 0,
        name: "Sol".into(),
        position: Vec3d::ZERO,
        star,
        companion: None,
        bodies,
        belts: vec![
            super::Belt { name: "Main Belt".into(), inner: 2.2 * AU, outer: 3.3 * AU, icy: false },
            super::Belt { name: "Kuiper Belt".into(), inner: 30.0 * AU, outer: 50.0 * AU, icy: true },
        ],
        dynamics: None,
        pending_contacts: Vec::new(),
        nebulae: Vec::new(),
    };

    for i in 0..system.bodies.len() {
        let d = system.stellar_distance(i);
        let observed = system.bodies[i].temperature;
        let star = system.star.clone();
        calibrate_real_body(&mut system.bodies[i], &star, 0.0, d, observed);
        let mut rng = Rng::stream(SOL_SEED, domain::RESOURCES, &[i as u64, 1]);
        let deposits = resources::generate_deposits(&mut rng, &system.bodies[i]);
        system.bodies[i].deposits = deposits;
    }
    system
}

/// Years before J2000 at which the "Dawn of Humanity" scenario begins.
pub const DAWN_OF_HUMANITY_YEARS_BP: f64 = 200_000.0;
pub fn dawn_of_humanity_start() -> f64 {
    -DAWN_OF_HUMANITY_YEARS_BP * SECONDS_PER_YEAR
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::AU;

    #[test]
    fn sol_loads_with_real_values() {
        let s = sol_system();
        assert_eq!(s.bodies.len(), 15);
        let earth = &s.bodies[s.find_body("Earth").unwrap()];
        assert!((earth.gravity() - 9.82).abs() < 0.05);
        assert!((earth.temperature - 288.0).abs() < 1e-6);
        let moon = s.find_body("Moon").unwrap();
        assert!(s.bodies[moon].tidally_locked);
        let days = s.bodies[moon].orbit.period(s.parent_mu(moon)) / 86400.0;
        assert!((days - 27.3).abs() < 0.3, "{days}");
        assert!((s.stellar_distance(moon) - AU).abs() / AU < 0.01);
    }

    #[test]
    fn earth_launch_delta_v_is_about_nine_km_s() {
        let s = sol_system();
        let earth = &s.bodies[s.find_body("Earth").unwrap()];
        let dv = earth.launch_delta_v_kms();
        assert!((8.8..10.2).contains(&dv), "{dv}");
    }
}
