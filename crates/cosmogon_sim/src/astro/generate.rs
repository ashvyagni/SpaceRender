//! Deterministic procedural generation of a stellar neighbourhood.
//!
//! The goal is *statistical plausibility*, not completeness: a Kroupa-like initial mass
//! function (most stars are M dwarfs), planet architectures organised around the frost line,
//! giant-planet occurrence rising with metallicity, atmospheres retained or lost according to
//! escape velocity, temperature and stellar activity. Every step is a small function so that
//! better astrophysical models can replace it independently.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use super::*;
use crate::names;
use crate::planet::environment::update_climate;
use crate::planet::{resources, terrain};
use crate::rng::{domain, hash_keys, Rng};
use crate::time::SECONDS_PER_GYR;
use crate::universe::{Scenario, UniverseSettings};
use crate::Vec3d;

/// Local stellar density (stars per cubic light-year) near the Sun.
const STELLAR_DENSITY_PER_LY3: f64 = 0.004;

pub fn generate_systems(settings: &UniverseSettings) -> Vec<StarSystem> {
    let seed = settings.seed;
    // Real-data and build-your-own templates contain only their own system: procedural
    // neighbours would masquerade as real stars.
    let n = match settings.scenario {
        Scenario::SolarSystemLab | Scenario::StarSystem | Scenario::EmptySystem => 1,
        _ => settings.system_count.max(1) as usize,
    };
    let radius_ly = (n as f64 / (4.0 / 3.0 * std::f64::consts::PI * STELLAR_DENSITY_PER_LY3)).dcbrt();

    let mut systems = Vec::with_capacity(n);
    for i in 0..n {
        let mut rng = Rng::stream(seed, domain::GALAXY, &[i as u64]);
        let home = i == 0 && settings.scenario != Scenario::Neighbourhood;
        let position = if home {
            Vec3d::ZERO
        } else {
            // Uniform in a sphere, flattened like the galactic thin disk is locally (barely).
            let (u, v, w) = (rng.f64(), rng.f64(), rng.f64());
            let r = radius_ly * u.dcbrt() * LIGHT_YEAR;
            let th = std::f64::consts::TAU * v;
            let ph = (2.0 * w - 1.0).dacos();
            Vec3d::new(r * ph.dsin() * th.dcos(), r * ph.dsin() * th.dsin(), 0.6 * r * ph.dcos())
        };
        if home && matches!(settings.scenario, Scenario::Sol | Scenario::SolarSystemLab | Scenario::EmptySystem) {
            let mut sol = sol::sol_system();
            sol.id = 0;
            if settings.scenario == Scenario::EmptySystem {
                sol.name = "New system".into();
                sol.bodies.clear();
                sol.belts.clear();
            }
            systems.push(sol);
            continue;
        }
        // Only the Garden World template promises a living world; a procedural system is
        // whatever the statistics give (most have no temperate ocean planet at all).
        let garden = home && settings.scenario == Scenario::GardenWorld;
        systems.push(generate_system(settings, i as u32, position, garden));
    }
    systems
}

/// Sample a stellar mass (M☉) from a broken power-law IMF between 0.08 and 8 M☉.
fn sample_imf(rng: &mut Rng) -> f64 {
    let pl = |rng: &mut Rng, lo: f64, hi: f64, alpha: f64| {
        let k = 1.0 - alpha;
        (lo.dpowf(k) + rng.f64() * (hi.dpowf(k) - lo.dpowf(k))).dpowf(1.0 / k)
    };
    let u = rng.f64();
    if u < 0.73 {
        pl(rng, 0.08, 0.5, 1.3)
    } else if u < 0.97 {
        pl(rng, 0.5, 1.5, 2.3)
    } else {
        pl(rng, 1.5, 8.0, 2.3)
    }
}

pub fn generate_system(settings: &UniverseSettings, id: u32, position: Vec3d, garden: bool) -> StarSystem {
    let seed = settings.seed;
    let mut rng = Rng::stream(seed, domain::STAR, &[id as u64]);
    let mut name_rng = Rng::stream(seed, domain::NAMES, &[id as u64]);
    let name = names::word(&mut name_rng);

    let mass = if garden { rng.range(0.85, 1.1) } else { sample_imf(&mut rng) };
    let metallicity = rng.normal(-0.05, 0.2).clamp(-1.0, 0.5);
    let lifetime_gyr = (10.0 * mass.dpowf(-2.5)).min(1.0e4);
    let age_gyr = if garden { rng.range(3.5, 5.5) } else { rng.range(0.3, lifetime_gyr.min(12.0) * 0.98) };
    let star = Star::from_mass(name.clone(), mass, metallicity, -age_gyr * SECONDS_PER_GYR);

    let companion = if !garden && rng.chance(0.22) {
        let cm = rng.range(0.08, mass.min(1.5));
        let a = rng.log_uniform(150.0, 3000.0) * AU;
        Some(Companion {
            star: Star::from_mass(format!("{name} B"), cm, metallicity, star.formed_at),
            orbit: Orbit {
                a,
                e: rng.range(0.0, 0.5),
                i: rng.range(0.0, 1.2),
                node: rng.range(0.0, std::f64::consts::TAU),
                peri: rng.range(0.0, std::f64::consts::TAU),
                m0: rng.range(0.0, std::f64::consts::TAU),
            },
        })
    } else {
        None
    };
    let outer_limit_au = companion.as_ref().map(|c| c.orbit.periapsis() / AU / 3.5).unwrap_or(80.0) * mass.sqrt().max(0.5);

    let mut system = StarSystem { id, name, position, star, companion, bodies: Vec::new(), belts: Vec::new(), dynamics: None, pending_contacts: Vec::new(), nebulae: Vec::new() };
    generate_planets(settings, &mut system, outer_limit_au, garden, age_gyr);
    system
}

fn rocky_radius(mass_e: f64) -> f64 {
    // Chen & Kipping (2017) terran branch, then a flatter volatile-rich branch.
    if mass_e < 2.0 {
        mass_e.dpowf(0.279)
    } else {
        1.21 * (mass_e / 2.0).dpowf(0.45)
    }
}

fn giant_radius(mass_e: f64, ice: bool) -> f64 {
    if ice {
        0.8 * mass_e.dpowf(0.589)
    } else {
        // Degenerate interiors: radius nearly flat with mass.
        11.2 * (mass_e / 318.0).dpowf(-0.04)
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_planets(settings: &UniverseSettings, sys: &mut StarSystem, outer_limit_au: f64, garden: bool, age_gyr: f64) {
    let seed = settings.seed;
    let sid = sys.id as u64;
    let mut rng = Rng::stream(seed, domain::PLANETS, &[sid]);
    let m = sys.star.mass;
    let frost = sys.star.frost_line_au();
    let feh = sys.star.metallicity;
    let p_giant = if m < 0.5 { 0.04 } else { 0.12 } * 10f64.dpowf(2.0 * feh) * m.min(2.0);

    let expected = 1.5 + 4.5 * m.min(1.3);
    let count = rng.poisson(expected).clamp(if garden { 3 } else { 0 }, 11) as usize;
    let mut a_au = rng.log_uniform(0.025, 0.25) * m.sqrt();

    let mut planets: Vec<(f64, BodyKind, f64)> = Vec::new(); // (a_au, kind, mass_e)
    let mut giants = 0;
    for _ in 0..count {
        if a_au > outer_limit_au {
            break;
        }
        let (kind, mass_e) = if a_au < frost {
            let mass_e = if m < 0.6 { rng.log_uniform(0.1, 6.0) } else { rng.log_uniform(0.05, 9.0) };
            (BodyKind::Rocky, mass_e)
        } else if rng.chance(p_giant * if giants == 0 { 3.0 } else { 1.2 }) {
            giants += 1;
            (BodyKind::GasGiant, rng.log_uniform(30.0, 2500.0))
        } else if a_au > frost * 1.5 && rng.chance(0.5) {
            (BodyKind::IceGiant, rng.log_uniform(8.0, 40.0))
        } else {
            (BodyKind::Icy, rng.log_uniform(0.005, 0.6))
        };
        planets.push((a_au, kind, mass_e));
        let ratio = if kind == BodyKind::GasGiant { rng.range(1.7, 2.6) } else { rng.range(1.35, 2.1) };
        a_au *= ratio;
    }

    // Hot Jupiters: ~1% of Sun-like stars host a giant that migrated to a few-day orbit,
    // more often at high metallicity (Wright et al. 2012; Fischer & Valenti 2005). Such
    // systems rarely keep close-in neighbours.
    let mut hrng = Rng::stream(seed, domain::PLANETS, &[sid, 0x407]);
    if !garden && m > 0.5 && hrng.chance((0.012 * 10f64.dpowf(2.0 * feh)).min(0.08)) {
        let a_hj = hrng.log_uniform(0.025, 0.09) * m.dcbrt();
        planets.retain(|p| p.0 > a_hj * 4.0);
        planets.insert(0, (a_hj, BodyKind::GasGiant, hrng.log_uniform(100.0, 1500.0)));
    }

    if garden {
        // Guarantee one Earth-analogue at Earth-equivalent insolation.
        let hz_mid = sys.star.luminosity(0.0).sqrt();
        let idx = planets
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 .0 / hz_mid).dln().abs().total_cmp(&(b.1 .0 / hz_mid).dln().abs()))
            .map(|(i, _)| i)
            .unwrap_or(0);
        if planets.is_empty() {
            planets.push((hz_mid, BodyKind::Rocky, 1.0));
        } else {
            planets[idx] = (hz_mid * rng.range(0.97, 1.03), BodyKind::Rocky, rng.range(0.8, 1.4));
        }
    }

    let star_age_gyr = age_gyr;
    for (n, &(a, kind, mass_e)) in planets.iter().enumerate() {
        let is_garden_world = garden && kind == BodyKind::Rocky && (a / sys.star.luminosity(0.0).sqrt()).dln().abs() < 0.05;
        let name = format!("{} {}", sys.name, names::planet_letter(n));
        let body = make_body(settings, sys, sys.bodies.len() as u32, name, None, a * AU, kind, mass_e, &mut rng, star_age_gyr, is_garden_world);
        sys.bodies.push(body);
    }

    // Moons.
    let planet_count = sys.bodies.len();
    for p in 0..planet_count {
        let mut mrng = Rng::stream(seed, domain::MOONS, &[sid, p as u64]);
        let parent = sys.bodies[p].clone();
        let hill = parent.orbit.a * (parent.mass / (3.0 * sys.star.mass * SOLAR_MASS)).dcbrt();
        let n_moons = match parent.kind {
            BodyKind::GasGiant => mrng.poisson(3.5).min(7),
            BodyKind::IceGiant => mrng.poisson(2.0).min(5),
            BodyKind::Rocky if parent.mass_earths() > 0.2 => u32::from(mrng.chance(0.35)),
            _ => 0,
        } as usize;
        let mut a = parent.radius * mrng.range(4.0, 12.0);
        for k in 0..n_moons {
            if a > hill * 0.4 {
                break;
            }
            let giant_host = !matches!(parent.kind, BodyKind::Rocky | BodyKind::Icy);
            let (kind, mass_e) = if giant_host {
                if mrng.chance(0.6) { (BodyKind::Icy, mrng.log_uniform(0.0005, 0.03)) } else { (BodyKind::Rocky, mrng.log_uniform(0.0005, 0.02)) }
            } else {
                // A large, Moon-like satellite from a giant impact.
                a = parent.radius * mrng.range(25.0, 70.0);
                (BodyKind::Rocky, mrng.log_uniform(0.004, 0.02))
            };
            let name = format!("{} {}", parent.name, names::roman(k));
            let mut moon = make_body(settings, sys, sys.bodies.len() as u32, name, Some(p as u32), a, kind, mass_e, &mut mrng, star_age_gyr, false);
            if giant_host && kind == BodyKind::Icy && a < parent.radius * 20.0 && mass_e > 0.004 {
                moon.hydro.subsurface_ocean = true;
                moon.geology += 0.8;
            }
            if giant_host && kind == BodyKind::Rocky && a < parent.radius * 8.0 {
                moon.geology += 5.0; // Io-like tidal heating
            }
            sys.bodies.push(moon);
            a *= mrng.range(1.5, 2.4);
        }
    }

    // Glacial cycles from orbital forcing. Without a large moon, obliquity wanders
    // chaotically and stable interglacials are short.
    for p in 0..planet_count {
        if sys.bodies[p].kind != BodyKind::Rocky || sys.bodies[p].tidally_locked {
            continue;
        }
        let big_moon = sys.moons_of(p).any(|m| sys.bodies[m].mass_earths() > 0.003);
        let mut grng = Rng::stream(seed, domain::PLANET_PHYSICS, &[sid, p as u64]);
        let b = &mut sys.bodies[p];
        b.glacial_cycle_years = grng.range(20_000.0, 120_000.0) * (1.0 + 4.0 * b.orbit.e);
        b.interglacial_fraction = if big_moon { grng.range(0.12, 0.35) } else { grng.range(0.04, 0.15) };
        b.glacial_phase_years = grng.range(0.0, b.glacial_cycle_years);
    }

    // Belts: an asteroid belt inside the first giant, a Kuiper belt beyond everything.
    if let Some(first_giant) = sys.bodies.iter().filter(|b| b.parent.is_none() && b.kind == BodyKind::GasGiant).map(|b| b.orbit.a).reduce(f64::min) {
        sys.belts.push(Belt { name: "Inner belt".into(), inner: first_giant * 0.42, outer: first_giant * 0.64, icy: false });
    }
    if sys.star.mass > 0.4 {
        let outer = sys.bodies.iter().filter(|b| b.parent.is_none()).map(|b| b.orbit.a).fold(frost * AU, f64::max);
        if outer * 1.4 < outer_limit_au * AU {
            sys.belts.push(Belt { name: "Outer belt".into(), inner: outer * 1.4, outer: outer * 2.0, icy: true });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn make_body(
    settings: &UniverseSettings,
    sys: &StarSystem,
    id: u32,
    name: String,
    parent: Option<u32>,
    a: f64,
    kind: BodyKind,
    mass_e: f64,
    rng: &mut Rng,
    age_gyr: f64,
    garden: bool,
) -> Body {
    let star = &sys.star;
    let stellar_a_au = match parent {
        Some(p) => sys.bodies[p as usize].orbit.a / AU,
        None => a / AU,
    };
    let radius_e = match kind {
        BodyKind::Rocky => rocky_radius(mass_e),
        BodyKind::Icy => 1.2 * rocky_radius(mass_e),
        BodyKind::GasGiant => giant_radius(mass_e, false),
        BodyKind::IceGiant => giant_radius(mass_e, true),
        _ => unreachable!("planet generator only makes planets"),
    };
    let mass = mass_e * EARTH_MASS;
    let radius = radius_e * EARTH_RADIUS;
    let tau = std::f64::consts::TAU;

    let ecc = if parent.is_some() { rng.rayleigh(0.01) } else { rng.rayleigh(0.05) }.min(0.45);
    let orbit = Orbit {
        a,
        e: if garden { ecc.min(0.03) } else { ecc },
        i: rng.rayleigh(0.025).min(0.4),
        node: rng.range(0.0, tau),
        peri: rng.range(0.0, tau),
        m0: rng.range(0.0, tau),
    };
    let mu_parent = match parent {
        Some(p) => sys.bodies[p as usize].mu(),
        None => star.mu(),
    };
    // Tidal locking: close-in planets (scale ~0.5 AU·M^⅓ at Gyr ages) and nearly all moons.
    let lock_radius_au = 0.45 * star.mass.dcbrt() * (age_gyr / 4.5).max(0.05).dpowf(1.0 / 6.0);
    let tidally_locked = parent.is_some() || (stellar_a_au < lock_radius_au && kind.has_surface());
    let rotation_period = if tidally_locked {
        orbit.period(mu_parent)
    } else if kind.has_surface() {
        let h = rng.log_uniform(8.0, 90.0);
        (if rng.chance(0.1) { -h } else { h }) * 3600.0
    } else {
        rng.range(8.0, 20.0) * 3600.0
    };
    let axial_tilt = if tidally_locked {
        rng.range(0.0, 0.05)
    } else if rng.chance(0.06) {
        rng.range(0.0, std::f64::consts::PI)
    } else {
        rng.rayleigh(0.3).min(1.2)
    };

    let lum = star.luminosity(0.0);
    let t_eq_guess = crate::planet::environment::equilibrium_temperature(lum, stellar_a_au, 0.3);
    let frost = star.frost_line_au();

    // Geology: radiogenic heat ~ mass, decaying with age; Earth (1 M⊕, 4.5 Gyr) ≈ 1.
    let geology = if kind.has_surface() {
        (1.76 * mass_e.sqrt() * (-age_gyr / (8.0 * mass_e.sqrt().max(0.05))).dexp()).min(3.0)
    } else {
        0.0
    };
    let magnetic_field = match kind {
        BodyKind::GasGiant => rng.log_uniform(200.0, 30_000.0),
        BodyKind::IceGiant => rng.log_uniform(10.0, 100.0),
        _ => {
            let rot_h: f64 = (rotation_period / 3600.0f64).abs();
            let spin = (40.0 / rot_h.max(10.0)).min(1.5);
            if mass_e > 0.25 && geology > 0.2 { (mass_e.sqrt() * spin * geology.min(1.5)).min(5.0) } else { 0.0 }
        }
    };

    // Water inventory (Earth oceans).
    let water_inventory = if garden {
        rng.range(0.7, 1.4)
    } else {
        match kind {
            BodyKind::Rocky if stellar_a_au < frost * 0.8 => rng.log_uniform(1e-4, 3.0) * if rng.chance(0.15) { 10.0 } else { 1.0 },
            BodyKind::Rocky => rng.log_uniform(0.5, 50.0),
            BodyKind::Icy => rng.log_uniform(1.0, 40.0) * mass_e.max(0.01) * 30.0,
            _ => 0.0,
        }
    };

    // Atmosphere retention: escape velocity against thermal speed, eroded by stellar flares
    // and helped by a magnetic field.
    let atmosphere = match kind {
        BodyKind::GasGiant | BodyKind::IceGiant => Atmosphere { pressure_bar: 1000.0, h2he: if kind == BodyKind::GasGiant { 0.99 } else { 0.97 }, ch4: if kind == BodyKind::GasGiant { 0.01 } else { 0.03 }, ..Default::default() },
        _ => {
            let v_esc = (2.0 * super::G * mass / radius).sqrt() / 1000.0;
            let retention = (v_esc / 11.2) / (t_eq_guess.max(30.0) / 255.0).sqrt();
            let flare = star.flare_activity_at(0.0);
            let shield = if magnetic_field > 0.1 { 1.0 } else { 0.6 };
            let keep = retention * shield * (1.0 - 0.5 * flare);
            if garden {
                Atmosphere { pressure_bar: rng.range(0.8, 1.4), n2: 0.975, co2: rng.range(0.003, 0.02), h2o: 0.01, ..Default::default() }
            } else if keep < 0.33 {
                Atmosphere { pressure_bar: rng.log_uniform(1e-9, 1e-4), co2: 0.5, n2: 0.5, ..Default::default() }
            } else if kind == BodyKind::Icy && t_eq_guess < 120.0 && mass_e > 0.01 && rng.chance(0.2) {
                Atmosphere { pressure_bar: rng.range(0.5, 2.0), n2: 0.95, ch4: 0.05, ..Default::default() }
            } else if mass_e > 4.0 && rng.chance(0.35) {
                // Super-Earth that kept part of its primordial hydrogen envelope.
                Atmosphere { pressure_bar: rng.log_uniform(20.0, 500.0), h2he: 0.9, h2o: 0.05, ch4: 0.05, ..Default::default() }
            } else {
                let p = (rng.normal(0.0, 1.1).dexp() * mass_e.dpowf(1.3) * keep.min(2.0)).clamp(0.003, 150.0);
                let hot = t_eq_guess > 300.0 && water_inventory > 0.01;
                if hot {
                    Atmosphere { pressure_bar: p.max(30.0), co2: 0.96, n2: 0.035, h2o: 0.005, ..Default::default() }
                } else {
                    let co2 = rng.range(0.02, 0.6);
                    Atmosphere { pressure_bar: p, co2, n2: 1.0 - co2 - 0.01, h2o: 0.01, ..Default::default() }
                }
            }
        }
    };

    let color = match kind {
        BodyKind::GasGiant => [rng.range(0.7, 0.95) as f32, rng.range(0.6, 0.82) as f32, rng.range(0.45, 0.7) as f32],
        BodyKind::IceGiant => [rng.range(0.35, 0.6) as f32, rng.range(0.6, 0.85) as f32, rng.range(0.8, 0.95) as f32],
        BodyKind::Icy => [0.82, 0.82, 0.8],
        BodyKind::Rocky => {
            let rust = (0.3 + 0.4 * rng.f64()) as f32;
            [0.45 + 0.25 * rust, 0.4 + 0.08 * rust, 0.35]
        }
        _ => unreachable!("planet generator only makes planets"),
    };
    let rings = if matches!(kind, BodyKind::GasGiant | BodyKind::IceGiant) && rng.chance(0.3) {
        Some(Rings { inner: radius * rng.range(1.25, 1.6), outer: radius * rng.range(1.9, 2.6), opacity: rng.range(0.1, 0.9) })
    } else {
        None
    };

    let mut body = Body {
        id,
        name,
        kind,
        parent,
        orbit,
        mass,
        radius,
        rotation_period,
        axial_tilt,
        tidally_locked,
        albedo: 0.3,
        atmosphere,
        hydro: Hydrosphere { water_inventory, ocean_fraction: 0.0, ice_fraction: 0.0, subsurface_ocean: false },
        magnetic_field,
        geology,
        equilibrium_temperature: t_eq_guess,
        temperature: t_eq_guess,
        resources: Resources::default(),
        deposits: Vec::new(),
        rings,
        terrain_seed: hash_keys(settings.seed, domain::TERRAIN, &[sys.id as u64, id as u64]),
        sea_level: 0.0,
        color,
        real: false,
        climate_bias: 0.0,
        glacial_cycle_years: 0.0,
        interglacial_fraction: 1.0,
        glacial_phase_years: 0.0,
        glacial_until_years: None,
        class: None,
        provenance: Default::default(),
        removed: None,
        impacts: Vec::new(),
        impact_winter: None,
        elevation_data: None,
        accretion: 0.0,
        melt: None,
    };
    update_climate(&mut body, star, 0.0, stellar_a_au * AU);
    if garden {
        // A carbonate–silicate thermostat: weathering draws CO₂ down on warm worlds and
        // volcanic outgassing builds it up on cold ones, settling into the temperate range.
        for _ in 0..40 {
            if body.temperature < 284.0 {
                body.atmosphere.co2 = (body.atmosphere.co2 * 1.5).min(0.3);
            } else if body.temperature > 296.0 {
                body.atmosphere.co2 /= 1.5;
            } else {
                break;
            }
            body.atmosphere.normalise();
            update_climate(&mut body, star, 0.0, stellar_a_au * AU);
        }
    }
    // Climate at the universe's start epoch is refined later; this is the formation state.
    body.sea_level = terrain::Terrain::of(&body).sea_level_for(body.hydro.ocean_fraction);
    let mut rrng = Rng::stream(settings.seed, domain::RESOURCES, &[sys.id as u64, id as u64]);
    body.resources = resources::generate_resources(&mut rrng, &body, star.metallicity, age_gyr, settings.resource_abundance);
    body.deposits = resources::generate_deposits(&mut rrng, &body);
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(seed: u64, scenario: Scenario) -> UniverseSettings {
        UniverseSettings { seed, scenario, system_count: 12, ..UniverseSettings::default() }
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate_systems(&settings(99, Scenario::Neighbourhood));
        let b = generate_systems(&settings(99, Scenario::Neighbourhood));
        assert_eq!(a, b);
        let c = generate_systems(&settings(100, Scenario::Neighbourhood));
        assert_ne!(a, c);
    }

    #[test]
    fn population_is_statistically_plausible() {
        let mut m_dwarfs = 0;
        let mut total = 0;
        let mut planets = 0;
        for seed in 0..40 {
            for s in generate_systems(&settings(seed, Scenario::Neighbourhood)) {
                total += 1;
                if s.star.mass < 0.5 {
                    m_dwarfs += 1;
                }
                planets += s.planets().count();
                for b in &s.bodies {
                    assert!(b.mass > 0.0 && b.radius > 0.0 && b.temperature.is_finite());
                    assert!(b.orbit.e < 1.0);
                }
            }
        }
        let frac = m_dwarfs as f64 / total as f64;
        assert!((0.55..0.9).contains(&frac), "M dwarf fraction {frac}");
        assert!(planets > total, "too few planets: {planets} for {total} stars");
    }

    #[test]
    fn garden_scenario_has_a_temperate_ocean_world() {
        for seed in 0..10 {
            let systems = generate_systems(&settings(seed, Scenario::GardenWorld));
            let home = &systems[0];
            assert!(home.bodies.iter().any(|b| b.kind == BodyKind::Rocky && b.hydro.ocean_fraction > 0.3 && (255.0..320.0).contains(&b.temperature)),
                "seed {seed}: {:?}", home.bodies.iter().map(|b| (b.name.clone(), b.temperature, b.hydro.ocean_fraction)).collect::<Vec<_>>());
        }
    }
}
