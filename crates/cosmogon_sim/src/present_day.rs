//! Present-day humanity for the Solar System Lab (1 January 2026).
//!
//! What is seeded and where it comes from (all approximate, and labelled ESTIMATED in the
//! app):
//! * population 8.23 billion (UN World Population Prospects 2024, interpolated to 2026),
//!   58 % urban (UN World Urbanization Prospects);
//! * primary energy use ≈ 19.5 TW, ~80 % fossil (Energy Institute Statistical Review 2024);
//! * ~11 000 active satellites (public satellite catalogues, 2025);
//! * major urban agglomerations from `data/cities_2025.toml`;
//! * the robotic and crewed exploration record of the Solar System, and three missions in
//!   flight (BepiColombo, JUICE, Europa Clipper) with their published arrival dates;
//! * technology: everything in the model up to crewed spaceflight and lunar landings;
//!   stations-and-industry ("space infrastructure"), fusion and general AI lie ahead.
//!
//! Carrying capacity is calibrated to the UN's projected peak (~10.3 billion in the 2080s),
//! so the model starts in a plausible place; from then on it evolves by its own rules.

use serde::Deserialize;

use crate::civ::settlements::Site;
use crate::civ::space::{Explored, Mission, MissionKind};
use crate::civ::species::{Habitat, Species};
use crate::civ::tech::{Condition, TechGraph};
use crate::history::{Category, Event};
use crate::life::Stage;
use crate::time::SECONDS_PER_YEAR;
use crate::{BodyRef, Universe};

const CITIES: &str = include_str!("../data/cities_2025.toml");

pub const POPULATION: f64 = 8.23e9;
pub const URBAN_FRACTION: f64 = 0.58;
pub const POWER_W: f64 = 19.5e12;
pub const SATELLITES: u32 = 11_000;
pub const PEAK_POPULATION: f64 = 10.3e9;

/// Technologies not yet achieved in 2026. Anything that depends on one of them (directly
/// or through other technologies) is not achieved either — see [`ahead_of_today`].
const AHEAD: [&str; 5] = ["space_infrastructure", "fusion_power", "artificial_intelligence", "interplanetary_colonies", "interstellar_probes"];

/// Per technology: still in the future in 2026.
pub fn ahead_of_today(graph: &TechGraph) -> Vec<bool> {
    let mut ahead: Vec<bool> = graph.techs.iter().map(|x| AHEAD.contains(&x.id.as_str())).collect();
    let tech_deps = |conds: &[Condition]| conds.iter().filter_map(|c| if let Condition::Tech(i) = c { Some(*i) } else { None }).collect::<Vec<_>>();
    loop {
        let mut changed = false;
        for (i, tech) in graph.techs.iter().enumerate() {
            if ahead[i] {
                continue;
            }
            let needs_future = tech_deps(&tech.requires).iter().any(|&d| ahead[d]);
            // With alternative routes, it is in the future only if every route needs the future.
            let routes_blocked = !tech.routes.is_empty() && tech.routes.iter().all(|r| tech_deps(&r.requires).iter().any(|&d| ahead[d]));
            if needs_future || routes_blocked {
                ahead[i] = true;
                changed = true;
            }
        }
        if !changed {
            return ahead;
        }
    }
}

#[derive(Deserialize)]
struct CityFile {
    city: Vec<City>,
}

#[derive(Deserialize)]
pub struct City {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    /// Millions.
    pub pop: f64,
    pub country: String,
    #[serde(default)]
    pub capital: bool,
}

pub fn cities() -> Vec<City> {
    toml::from_str::<CityFile>(CITIES).expect("embedded data/cities_2025.toml must parse").city
}

/// (body, deepest level reached, first visit year, year that level was reached).
const EXPLORED: [(&str, MissionKind, f64, f64); 15] = [
    ("Moon", MissionKind::Crewed, 1959.0, 1969.55),
    ("Venus", MissionKind::Lander, 1962.95, 1970.95),
    ("Mars", MissionKind::Lander, 1965.5, 1976.55),
    ("Mercury", MissionKind::Orbiter, 1974.25, 2011.2),
    ("Jupiter", MissionKind::Orbiter, 1973.9, 1995.95),
    ("Saturn", MissionKind::Orbiter, 1979.7, 2004.5),
    ("Uranus", MissionKind::Flyby, 1986.07, 1986.07),
    ("Neptune", MissionKind::Flyby, 1989.65, 1989.65),
    ("Io", MissionKind::Flyby, 1979.2, 1979.2),
    ("Europa", MissionKind::Flyby, 1979.2, 1979.2),
    ("Ganymede", MissionKind::Flyby, 1979.2, 1979.2),
    ("Callisto", MissionKind::Flyby, 1979.2, 1979.2),
    ("Titan", MissionKind::Lander, 1980.87, 2005.04),
    ("Enceladus", MissionKind::Flyby, 1980.87, 1980.87),
    ("Earth", MissionKind::Crewed, 1957.8, 1957.8),
];

/// Missions in flight on 1 January 2026: (name, target, kind, launch year, arrival year).
const IN_FLIGHT: [(&str, &str, MissionKind, f64, f64); 3] = [
    ("BepiColombo", "Mercury", MissionKind::Orbiter, 2018.8, 2026.9),
    ("JUICE", "Ganymede", MissionKind::Orbiter, 2023.29, 2031.55),
    ("Europa Clipper", "Europa", MissionKind::Orbiter, 2024.78, 2030.3),
];

impl Universe {
    /// Humanity as of the start epoch (2026). Only for the Solar System Lab.
    pub(crate) fn seed_present_humanity(&mut self) {
        let t = self.start_time;
        let year = |y: f64| t - (2026.0 - y) * SECONDS_PER_YEAR;
        let Some(earth) = self.systems[0].find_body("Earth") else { return };
        let r = BodyRef { system: 0, body: earth as u32 };
        let Some(bi) = self.biospheres.iter().position(|b| b.system == 0 && b.body as usize == earth) else { return };
        self.biospheres[bi].stage = Stage::Intelligent;
        self.biospheres[bi].civilization_present = true;

        let species = Species {
            name: "Humans".into(),
            habitat: Habitat::Land,
            mass_kg: 62.0,
            lifespan_years: 73.0,
            sociality: 0.75,
            traits: vec!["Terrestrial".into(), "Persistence hunters".into(), "Tribal social structure".into()],
        };
        let mut civ = self.make_civ(r, species, t);
        civ.name = "Humanity".into();
        civ.founded = year(-300_000.0 + 2026.0);

        // Technology and the knowledge it implies.
        let graph = TechGraph::embedded();
        let mut need = [0.0f64; crate::civ::knowledge::N_DOMAINS];
        let ahead = ahead_of_today(graph);
        for tech in graph.techs.iter().enumerate().filter(|(i, _)| !ahead[*i]).map(|(_, x)| x) {
            let conds = tech.requires.iter().chain(tech.routes.first().map(|r| r.requires.iter()).into_iter().flatten());
            for c in conds {
                if let Condition::Knowledge(d, v) = c {
                    need[d.index()] = need[d.index()].max(*v);
                }
            }
            civ.grant(graph, &tech.id, t);
        }
        for (k, n) in civ.knowledge.iter_mut().zip(need) {
            *k = n.max(1.0) * 1.15;
        }

        civ.population = POPULATION;
        civ.urbanisation = URBAN_FRACTION;
        civ.urban_target = civ.urban_target.max(0.68);
        civ.energy_per_capita = POWER_W / POPULATION;
        civ.fossil_share = 0.8;
        civ.stability = 0.65;
        civ.satellites = SATELLITES;
        civ.radio_since = Some(year(1895.0));
        civ.missions_launched = 250;
        civ.settlement_timer = 0.0;

        // Settlements: real cities with measured shares, then the model's own sites (on
        // Earth's real relief) as the towns and regions in between.
        let urban = POPULATION * URBAN_FRACTION;
        let mut sites: Vec<Site> = cities()
            .into_iter()
            .enumerate()
            .map(|(k, c)| Site {
                name: c.name,
                lat: c.lat.to_radians(),
                lon: c.lon.to_radians(),
                score: 1.0,
                coastal: false,
                near_deposit: None,
                founded: Some(year(1000.0) + k as f64),
                population: c.pop * 1e6,
                polity: None,
                share: Some(c.pop * 1e6 / urban),
            })
            .collect();
        let named: Vec<([f64; 3], String)> = sites.iter().map(|s| (s.dir(), s.name.clone())).collect();
        for mut s in std::mem::take(&mut civ.sites) {
            let d = s.dir();
            if named.iter().any(|(c, _)| c[0] * d[0] + c[1] * d[1] + c[2] * d[2] > 0.99985) {
                continue; // within ~110 km of a listed city: part of it
            }
            let nearest = named.iter().max_by(|a, b| (a.0[0] * d[0] + a.0[1] * d[1] + a.0[2] * d[2]).total_cmp(&(b.0[0] * d[0] + b.0[1] * d[1] + b.0[2] * d[2]))).map(|n| n.1.clone()).unwrap_or_default();
            s.name = format!("{nearest} region");
            s.founded = Some(year(1500.0));
            sites.push(s);
            if sites.len() >= crate::civ::settlements::MAX_SITES {
                break;
            }
        }
        civ.sites = sites;

        // Countries: each listed city belongs to its country; the model's regional sites
        // join the country of the nearest listed city (approximate borders).
        let list = cities();
        let mut countries: Vec<String> = Vec::new();
        for c in &list {
            if !countries.contains(&c.country) {
                countries.push(c.country.clone());
            }
        }
        civ.polities = countries
            .iter()
            .enumerate()
            .map(|(k, name)| {
                let capital = list.iter().position(|c| &c.country == name && c.capital).or_else(|| list.iter().position(|c| &c.country == name)).unwrap_or(0);
                crate::civ::polity::Polity {
                    id: k as u16,
                    name: name.clone(),
                    government: crate::civ::polity::Government::State,
                    capital: capital as u16,
                    founded: year(1945.0),
                    ended: None,
                    color: crate::civ::polity::color_for(k as u16),
                    population: 0.0,
                    sites: 0,
                    at_war: Vec::new(),
                    relations: Default::default(),
                    wars_fought: 0,
                }
            })
            .collect();
        let city_dirs: Vec<([f64; 3], u16)> = list.iter().map(|c| (crate::planet::terrain::dir_from_lat_lon(c.lat.to_radians(), c.lon.to_radians()), countries.iter().position(|n| n == &c.country).unwrap() as u16)).collect();
        for s in civ.sites.iter_mut() {
            let d = s.dir();
            s.polity = city_dirs.iter().max_by(|a, b| (a.0[0] * d[0] + a.0[1] * d[1] + a.0[2] * d[2]).total_cmp(&(b.0[0] * d[0] + b.0[1] * d[1] + b.0[2] * d[2]))).map(|c| c.1);
        }
        civ.static_polities = true;
        crate::civ::polity::tally(&civ.sites, &mut civ.polities);

        // Exploration record and spacecraft in flight.
        let sys = &self.systems[0];
        for (name, level, first, since) in EXPLORED {
            if let Some(b) = sys.find_body(name) {
                civ.explored.push(Explored { body: b as u32, level, first: year(first), since: year(since) });
            }
        }
        for (name, target, kind, launch, arrive) in IN_FLIGHT {
            if let Some(b) = sys.find_body(target) {
                civ.missions.push(Mission { kind, body: b as u32, launched: year(launch), arrives: year(arrive), name: name.into() });
            }
        }

        // Remaining recoverable fossil fuel, in the model's units (1 unit = 2×10²² J): enough
        // for roughly a century at today's use, in line with reserves-plus-resources
        // estimates (BGR Energy Study). Depletion then pushes research towards alternatives.
        {
            let body = &mut self.systems[0].bodies[earth];
            body.resources.coal = 1.5;
            body.resources.oil = 1.1;
        }
        // Calibrate carrying capacity to the projected peak population.
        let body = &self.systems[0].bodies[earth];
        civ.baseline_temperature = body.temperature;
        civ.baseline_co2 = body.atmosphere.co2;
        let cap = civ.capacity_for(body, body.climate_stability(t));
        if cap > 0.0 {
            civ.capacity_mult *= PEAK_POPULATION / cap;
        }
        civ.capacity = civ.capacity_for(body, body.climate_stability(t));

        let id = civ.id;
        self.history.push(Event {
            time: t,
            category: Category::Civilization,
            importance: 5,
            title: "Humanity, 2026".into(),
            detail: format!("{:.2} billion people, {} active satellites, robotic explorers on or around every planet; crewed spaceflight since 1961.", POPULATION / 1e9, SATELLITES),
            system: Some(0),
            body: Some(earth as u32),
            civ: Some(id),
        });
        self.civs.push(civ);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Scenario, Universe, UniverseSettings};

    fn lab() -> Universe {
        Universe::new(UniverseSettings { scenario: Scenario::SolarSystemLab, ..Default::default() })
    }

    #[test]
    fn present_day_humanity_is_seeded() {
        let u = lab();
        let c = u.civs.iter().find(|c| c.name == "Humanity").expect("humanity");
        assert!((c.population - 8.23e9).abs() < 1e7);
        assert!(c.knows("moon_landing") && c.knows("spaceflight") && !c.knows("fusion_power"));
        // Nothing that builds on future technology leaks into 2026.
        for id in ["terraforming", "dyson_swarm", "generation_ships"] {
            assert!(!c.knows(id), "{id} is not a 2026 technology");
        }
        let k = c.kardashev();
        assert!((0.70..0.76).contains(&k), "Kardashev {k}");
        assert!(c.sites.iter().any(|s| s.name == "Tokyo"));
        assert_eq!(c.missions.len(), 3);
        assert!((c.capacity / 10.3e9 - 1.0).abs() < 0.02, "capacity {}", c.capacity);
    }

    #[test]
    fn humanity_carries_on_for_a_century() {
        let mut u = lab();
        u.advance_by(100.0 * crate::time::SECONDS_PER_YEAR);
        let c = u.civs.iter().find(|c| c.name == "Humanity").unwrap();
        assert!(c.is_alive());
        assert!(c.population > 5e9 && c.population < 1.3e10, "population after a century: {}", c.population);
        // Tokyo keeps a realistic size rather than absorbing the urban rank-size rule.
        let tokyo = c.sites.iter().find(|s| s.name == "Tokyo").unwrap();
        assert!(tokyo.population < 80e6, "Tokyo {}", tokyo.population);
        // The space programme kept going: something new was explored or launched.
        assert!(c.missions_launched > 260, "{}", c.missions_launched);
        // The energy transition happened instead of a permanent shortfall.
        assert!(c.fossil_share < 0.5 && c.energy_shortfall < 0.1, "fossil {} shortfall {}", c.fossil_share, c.energy_shortfall);
    }

    #[test]
    fn civilization_switch_is_respected() {
        let mut s = UniverseSettings { scenario: Scenario::SolarSystemLab, ..Default::default() };
        s.systems.civilization = false;
        let u = Universe::new(s);
        assert!(u.civs.is_empty());
    }
}
