//! Civilizations: the signature system.
//!
//! A civilization is a *statistical* society: population, knowledge per domain, known
//! technologies, energy, stability, pressures and a set of representative settlements.
//! It is stepped once per simulated year. Everything that happens to it is explained by
//! state you can inspect — see CIVILIZATION_MODEL.md.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
pub mod knowledge;
pub mod polity;
pub mod settlements;
pub mod space;
pub mod species;
pub mod tech;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::astro::Body;
use crate::params::ScienceParams;
use crate::planet::terrain::SurfaceContext;
use crate::rng::Rng;
use crate::time::{group_digits, SECONDS_PER_YEAR};
use knowledge::{Domain, Knowledge, N_DOMAINS};
use settlements::{Link, Site, Tier};
use species::{Habitat, Species};
use tech::{Context, EnvKey, Environment, TechGraph};

/// Joules of fossil fuel in one "Earth endowment" (coal + oil ≈ 2 units).
const FOSSIL_JOULES_PER_UNIT: f64 = 2.0e22;
/// CO₂ volume-fraction added per watt-year of fossil power, for an Earth-sized 1-bar atmosphere.
const CO2_PER_WATT_YEAR: f64 = 1.6e-19;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CivStatus {
    Thriving,
    Collapsed { since: f64 },
    Extinct { at: f64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Pressures {
    pub food: f64,
    pub disease: f64,
    pub energy: f64,
    pub war: f64,
    pub climate: f64,
    pub contact: f64,
    pub cold: f64,
}

impl Pressures {
    pub fn get(&self, name: &str) -> f64 {
        match name {
            "food" => self.food,
            "disease" => self.disease,
            "energy" => self.energy,
            "war" => self.war,
            "climate" => self.climate,
            "contact" => self.contact,
            "cold" => self.cold,
            _ => 0.0,
        }
    }
    fn decay(&mut self, dt: f64) {
        let k = 0.985f64.dpowf(dt);
        for p in [&mut self.food, &mut self.disease, &mut self.energy, &mut self.war, &mut self.contact] {
            *p *= k;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Discovery {
    pub tech: String,
    pub time: f64,
    pub route: Option<String>,
    /// Human-readable causes: what made it possible and what pushed for it.
    pub drivers: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Colony {
    pub body: u32,
    pub founded: f64,
    pub population: f64,
    /// Terraforming has made the world habitable (announced once).
    #[serde(default)]
    pub terraformed: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub t: f64,
    pub population: f64,
    pub knowledge: f64,
    pub techs: u32,
    pub energy_w: f64,
    pub stability: f64,
}

/// Snapshots with bounded memory: when full, every other point is dropped and the
/// sampling interval doubles, so a civilization's whole history always fits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Samples {
    pub interval_years: f64,
    pub next_t: f64,
    pub points: Vec<Sample>,
}

impl Samples {
    pub const MAX: usize = 400;
    fn new(t: f64) -> Self {
        Self { interval_years: 10.0, next_t: t, points: Vec::new() }
    }
    fn push(&mut self, s: Sample) {
        if s.t < self.next_t {
            return;
        }
        self.points.push(s);
        if self.points.len() >= Self::MAX {
            let kept: Vec<Sample> = self.points.iter().copied().step_by(2).collect();
            self.points = kept;
            self.interval_years *= 2.0;
        }
        self.next_t = s.t + self.interval_years * SECONDS_PER_YEAR;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Civilization {
    pub id: u32,
    pub name: String,
    pub species: Species,
    pub system: u32,
    pub body: u32,
    pub founded: f64,
    pub status: CivStatus,

    pub population: f64,
    pub capacity: f64,
    pub knowledge: Knowledge,
    pub discoveries: Vec<Discovery>,
    pub flags: BTreeSet<String>,

    // Accumulated technology effects.
    pub capacity_mult: f64,
    pub growth_bonus: f64,
    pub research_mult: f64,
    pub energy_per_capita: f64,
    pub fossil_share: f64,
    pub urban_target: f64,
    pub urbanisation: f64,
    pub base_stability: f64,
    pub stability: f64,
    pub health: f64,
    pub reach: f64,
    pub focus_boost: [f64; N_DOMAINS],
    /// Fixed research inclinations from species and environment.
    pub env_focus: [f64; N_DOMAINS],

    pub pressures: Pressures,
    pub energy_shortfall: f64,
    pub baseline_co2: f64,
    pub baseline_temperature: f64,

    pub sites: Vec<Site>,
    pub links: Vec<Link>,
    /// Rival states sharing this civilization's world.
    #[serde(default)]
    pub polities: Vec<polity::Polity>,
    pub highest_tier: Option<Tier>,

    pub radio_since: Option<f64>,
    pub satellites: u32,
    pub colonies: Vec<Colony>,
    pub probes_launched: u32,
    pub detected: Vec<u32>,
    pub samples: Samples,
    pub collapses: u32,

    // Level of detail: a civilization is stepped every `stride_years`, chosen from its own
    // state (fast-changing societies yearly, stable ones every 10 or 100 years).
    #[serde(default = "one_u32")]
    pub stride_years: u32,
    /// Civilization-task step index at which this civilization is next due.
    #[serde(default)]
    pub next_k: u64,
    #[serde(default)]
    pub last_discovery: f64,
    #[serde(default)]
    pub settlement_timer: f64,
    #[serde(default)]
    pub colony_timer: f64,
    #[serde(default)]
    pub probe_timer: f64,
    #[serde(default)]
    pub starship_timer: f64,
    /// Fraction of the star's output captured by a Dyson swarm (0..0.9).
    #[serde(default)]
    pub dyson: f64,
    /// Power harvested by the swarm (W).
    #[serde(default)]
    pub dyson_power_w: f64,
    /// The civilization whose generation ship founded this one.
    #[serde(default)]
    pub parent: Option<u32>,
    /// Spacecraft in flight within the home system.
    #[serde(default)]
    pub missions: Vec<space::Mission>,
    /// Worlds of the home system visited so far, and how closely.
    #[serde(default)]
    pub explored: Vec<space::Explored>,
    #[serde(default)]
    pub missions_launched: u32,
    /// When the sandbox user last intervened (interventions have a cooldown).
    #[serde(default)]
    pub last_intervention: Option<f64>,
    /// Polities are real countries that the model must not reshape (present-day Earth):
    /// they keep their territory; no wars or conquests between them are simulated.
    #[serde(default)]
    pub static_polities: bool,

    #[serde(skip)]
    known_cache: KnownCache,
    #[serde(skip)]
    adjacency: settlements::Adjacency,
}

/// Derived lookup table (rebuilt from `discoveries`); never part of equality or saves.
#[derive(Clone, Debug, Default)]
struct KnownCache(Vec<bool>);

impl PartialEq for KnownCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

fn one_u32() -> u32 {
    1
}

/// Probability that an event with yearly probability `p` happens at least once in `dt` years.
fn over(p: f64, dt: f64) -> f64 {
    1.0 - (1.0 - p.clamp(0.0, 1.0)).dpowf(dt)
}

/// Fraction remaining of a quantity relaxing at yearly rate `k` after `dt` years.
fn relax(k: f64, dt: f64) -> f64 {
    1.0 - (1.0 - k).dpowf(dt)
}

/// What a civilization's step reports back to the universe.
#[derive(Clone, Debug, PartialEq)]
pub struct CivEvent {
    pub importance: u8,
    pub category: crate::history::Category,
    pub title: String,
    pub detail: String,
}

/// Facts about the civilization's surroundings that only the universe knows.
pub struct WorldView<'a> {
    pub env: Environment,
    pub surface: SurfaceContext,
    /// Other worlds of the home system: transfer times and colony suitability.
    pub destinations: Vec<space::Destination>,
    pub params: &'a ScienceParams,
    pub tech_rate: f64,
}

pub fn environment_for(body: &Body, moons: usize, other_bodies: usize, t: f64) -> Environment {
    let mut env = Environment::default();
    let v = &mut env.values;
    v.insert(EnvKey::Oxygen, body.atmosphere.o2);
    v.insert(EnvKey::Gravity, body.gravity_g());
    v.insert(EnvKey::Ocean, body.hydro.ocean_fraction);
    v.insert(EnvKey::Land, body.land_fraction());
    v.insert(EnvKey::Pressure, body.atmosphere.pressure_bar);
    v.insert(EnvKey::Temperature, body.temperature);
    v.insert(EnvKey::LaunchDv, body.launch_delta_v_kms());
    v.insert(EnvKey::Moons, moons as f64);
    v.insert(EnvKey::OtherBodies, other_bodies as f64);
    v.insert(EnvKey::ClimateStability, body.climate_stability(t));
    env
}

impl Civilization {
    #[allow(clippy::too_many_arguments)]
    pub fn new(id: u32, system: u32, body_idx: u32, body: &Body, species: Species, surface: &SurfaceContext, moons: usize, t: f64, rng: &mut Rng) -> Self {
        let mut env_focus = [1.0; N_DOMAINS];
        let ocean = body.hydro.ocean_fraction;
        env_focus[Domain::Navigation.index()] = 0.5 + 1.5 * ocean;
        env_focus[Domain::Astronomy.index()] = 0.6 + 0.3 * moons.min(3) as f64 - if species.habitat == Habitat::Water { 0.4 } else { 0.0 };
        env_focus[Domain::Computing.index()] = 0.15;
        env_focus[Domain::Agriculture.index()] = 1.2;
        if body.temperature < 275.0 {
            env_focus[Domain::Energy.index()] += 0.3;
        }
        let sites = settlements::candidate_sites(body, surface, species.habitat, rng);
        let name = format!("{} civilization", species.name);
        Self {
            id,
            name,
            species,
            system,
            body: body_idx,
            founded: t,
            status: CivStatus::Thriving,
            population: 20_000.0,
            capacity: 0.0,
            knowledge: [0.0; N_DOMAINS],
            discoveries: Vec::new(),
            flags: BTreeSet::new(),
            capacity_mult: 1.0,
            growth_bonus: 0.0,
            research_mult: 1.0,
            energy_per_capita: 80.0,
            fossil_share: 0.0,
            urban_target: 0.0,
            urbanisation: 0.0,
            base_stability: 0.7,
            stability: 0.7,
            health: 0.0,
            reach: 0.11,
            focus_boost: [1.0; N_DOMAINS],
            env_focus,
            pressures: Pressures::default(),
            energy_shortfall: 0.0,
            baseline_co2: body.atmosphere.co2,
            baseline_temperature: body.temperature,
            sites,
            links: Vec::new(),
            polities: Vec::new(),
            highest_tier: None,
            radio_since: None,
            satellites: 0,
            colonies: Vec::new(),
            probes_launched: 0,
            detected: Vec::new(),
            samples: Samples::new(t),
            collapses: 0,
            stride_years: 1,
            next_k: 0,
            last_discovery: t,
            settlement_timer: 10.0,
            colony_timer: 0.0,
            probe_timer: 0.0,
            starship_timer: 0.0,
            dyson: 0.0,
            dyson_power_w: 0.0,
            parent: None,
            missions: Vec::new(),
            explored: Vec::new(),
            missions_launched: 0,
            last_intervention: None,
            static_polities: false,
            known_cache: KnownCache::default(),
            adjacency: settlements::Adjacency::default(),
        }
    }

    /// Carrying capacity on `body` with the civilization's current technology.
    pub fn capacity_for(&self, body: &Body, climate_stability: f64) -> f64 {
        let area = match self.species.habitat {
            Habitat::Land => body.surface_area_km2() * body.land_fraction(),
            Habitat::Water => body.surface_area_km2() * body.hydro.ocean_fraction * 0.25,
        };
        let fertility = (body.resources.fertile_land * 0.35).max(0.03);
        let climate_stress = (((body.temperature - self.baseline_temperature).abs() - 1.5).max(0.0) / 6.0).min(1.0);
        area * fertility * 0.05 * (0.7 + 0.3 * climate_stability) * self.capacity_mult * (1.0 - 0.6 * climate_stress) * (1.0 - 0.5 * self.energy_shortfall)
    }

    pub fn is_alive(&self) -> bool {
        !matches!(self.status, CivStatus::Extinct { .. })
    }

    pub fn known(&mut self, graph: &TechGraph) -> &[bool] {
        if self.known_cache.0.len() != graph.len() {
            self.known_cache.0 = vec![false; graph.len()];
            for d in &self.discoveries {
                if let Some(i) = graph.find(&d.tech) {
                    self.known_cache.0[i] = true;
                }
            }
        }
        &self.known_cache.0
    }

    pub fn knows(&self, id: &str) -> bool {
        self.discoveries.iter().any(|d| d.tech == id)
    }

    pub fn total_power_w(&self) -> f64 {
        self.population * self.energy_per_capita * (1.0 - self.energy_shortfall) + self.dyson_power_w
    }

    /// Artificial light output at night (arbitrary units: 0 = none, ~1 = modern Earth).
    pub fn night_light(&self) -> f64 {
        let electric = if self.flags.contains("electric_light") { 1.0 } else { 0.03 };
        (self.population * self.urbanisation / 4.0e9 * electric * (1.0 - self.energy_shortfall)).min(3.0)
    }

    pub fn era(&self, graph: &TechGraph) -> String {
        self.discoveries.last().and_then(|_| {
            // The era of the most advanced known tech (later in file = more advanced).
            self.discoveries.iter().filter_map(|d| graph.find(&d.tech)).max().map(|i| graph.techs[i].era.clone())
        }).unwrap_or_else(|| "Paleolithic".into())
    }

    /// Grant a technology without a discovery roll (scenario setup).
    pub fn grant(&mut self, graph: &TechGraph, id: &str, t: f64) {
        if let Some(i) = graph.find(id) {
            if !self.knows(id) {
                self.apply_effects(graph, i);
                self.discoveries.push(Discovery { tech: id.into(), time: t, route: None, drivers: "known at the start".into() });
                self.known_cache.0.clear();
            }
        }
    }

    fn apply_effects(&mut self, graph: &TechGraph, i: usize) {
        let t = &graph.techs[i];
        let e = &t.effects;
        self.capacity_mult *= e.capacity;
        self.growth_bonus += e.growth;
        self.research_mult *= e.research;
        self.energy_per_capita = self.energy_per_capita.max(e.energy);
        self.fossil_share = self.fossil_share.max(e.fossil).min(e.fossil_cap);
        self.urban_target = (self.urban_target + e.urban).clamp(0.0, 0.85);
        self.base_stability = (self.base_stability + e.stability).clamp(0.2, 0.95);
        self.health = (self.health + e.health).clamp(-0.5, 0.95);
        self.reach += e.reach;
        for (d, m) in &t.boost {
            self.focus_boost[d.index()] *= m;
        }
        self.focus_boost[t.domain.index()] *= 1.05;
        for f in &t.unlocks {
            self.flags.insert(f.clone());
        }
    }

    fn focus(&self) -> [f64; N_DOMAINS] {
        let p = &self.pressures;
        let mut w = [0.0; N_DOMAINS];
        for d in Domain::ALL {
            let i = d.index();
            let pressure = match d {
                Domain::Agriculture => 2.0 * p.food,
                Domain::Medicine | Domain::Biology => 3.0 * p.disease,
                Domain::Energy => 2.0 * p.energy + p.cold,
                Domain::Materials | Domain::Engineering => 1.5 * p.war,
                Domain::Astronomy => 2.5 * p.contact,
                Domain::Physics | Domain::Chemistry => p.climate + 0.5 * p.energy,
                _ => 0.0,
            };
            w[i] = self.env_focus[i] * self.focus_boost[i] * (1.0 + pressure);
        }
        let s: f64 = w.iter().sum();
        w.map(|x| x / s)
    }

    /// Choose the next step size from the society's own state. Deterministic: depends only
    /// on simulation state at a step boundary, never on frame rate or speed.
    pub fn choose_stride(&self, t: f64) -> u32 {
        let since = (t - self.last_discovery) / SECONDS_PER_YEAR;
        // Only genuine crises force yearly steps; living at carrying capacity (food
        // pressure 0.5) or a chronic small energy deficit is the normal state of a mature society.
        let turbulent = !matches!(self.status, CivStatus::Thriving)
            || self.pressures.food > 0.6
            || self.pressures.disease > 0.3
            || self.pressures.war > 0.5
            || self.energy_shortfall > 0.3
            || self.population < self.capacity * 0.5;
        if turbulent || since < 300.0 {
            1
        } else if since < 5_000.0 {
            10
        } else if since < 50_000.0 {
            100
        } else {
            1000
        }
    }

    /// Advance `dt` simulated years ending at `t` (dt = 1 for fast-changing societies).
    #[allow(clippy::too_many_arguments)]
    pub fn step(&mut self, body: &mut Body, world: &WorldView, graph: &TechGraph, t: f64, dt: f64, rng: &mut Rng) -> Vec<CivEvent> {
        use crate::history::Category as C;
        let mut ev = Vec::new();
        if !self.is_alive() {
            return ev;
        }
        let cp = &world.params.civilization;
        self.pressures.decay(dt);

        // ── Carrying capacity & population ──────────────────────────────
        let climate_stress = ((body.temperature - self.baseline_temperature).abs() - 1.5).max(0.0) / 6.0;
        self.pressures.climate = climate_stress.min(1.0);
        let stable = world.env.values.get(&EnvKey::ClimateStability).copied().unwrap_or(1.0);
        self.capacity = self.capacity_for(body, stable);
        let r = (cp.base_growth_rate + self.growth_bonus) * (1.0 + self.health).max(0.2);
        let p = self.population;
        // Exact logistic solution over dt (stable for any step size).
        let cap = self.capacity.max(1.0);
        self.population = cap / (1.0 + (cap / p.max(1.0) - 1.0) * (-r * dt).dexp());
        if self.population > self.capacity * 1.02 {
            let lost = (self.population - self.capacity) * 0.3;
            self.population -= lost;
            self.pressures.food = 1.0;
            self.stability -= 0.02;
            if lost > p * 0.05 {
                ev.push(CivEvent { importance: 2, category: C::Disaster, title: "Famine".into(), detail: format!("{} starve as population outstrips food supply", group_digits(lost)) });
            }
        } else if self.population > self.capacity * 0.9 {
            self.pressures.food = self.pressures.food.max(0.5);
        }

        // ── Research ────────────────────────────────────────────────────
        let research = cp.research_scale * self.population.max(1.0).dpowf(cp.research_exponent) * self.research_mult * (0.7 + 0.6 * self.species.sociality) * world.tech_rate * (0.5 + 0.5 * self.stability.clamp(0.0, 1.0));
        let decay = if self.flags.contains("printing") {
            0.00001
        } else if self.flags.contains("writing") {
            0.00005
        } else {
            cp.oral_knowledge_decay
        };
        let focus = self.focus();
        // Exact solution of dK/dt = R·f − λK over dt.
        let keep = (1.0 - decay).dpowf(dt);
        for i in 0..N_DOMAINS {
            let inflow = research * focus[i];
            self.knowledge[i] = self.knowledge[i] * keep + inflow * (1.0 - keep) / decay.max(1e-12);
        }

        // ── Discovery ───────────────────────────────────────────────────
        self.known(graph);
        let flags = &self.flags;
        let has_flag = |f: &str| flags.contains(f);
        let ctx = Context { known: &self.known_cache.0, knowledge: &self.knowledge, resources: &body.resources, env: &world.env, habitat: self.species.habitat, population: self.population, flags: &has_flag };
        let mut found = Vec::new();
        for i in 0..graph.len() {
            if let Some((route, speed, knowledge_surplus)) = graph.available(i, &ctx) {
                let tech = &graph.techs[i];
                let demand = tech.demand.as_deref().map(|d| self.pressures.get(d)).unwrap_or(0.0);
                let rate = speed * knowledge_surplus.dpowf(1.5) * (1.0 + 2.0 * demand) * world.tech_rate / tech.years;
                if rng.hazard(rate, dt) {
                    found.push((i, route, knowledge_surplus, demand));
                }
            }
        }
        for (i, route, surplus, demand) in found {
            let tech = &graph.techs[i];
            let route_label = route.map(|r| tech.routes[r].label.clone());
            let mut drivers = format!("knowledge at {:.1}× the minimum", surplus);
            if let (Some(d), true) = (&tech.demand, demand > 0.2) {
                drivers.push_str(&format!("; spurred by {d} pressure ({:.0}%)", demand * 100.0));
            }
            if let Some(r) = &route_label {
                drivers.push_str(&format!("; route: {r}"));
            }
            self.apply_effects(graph, i);
            self.last_discovery = t;
            self.known_cache.0.clear();
            self.discoveries.push(Discovery { tech: tech.id.clone(), time: t, route: route_label.clone(), drivers: drivers.clone() });
            let importance = match tech.id.as_str() {
                "agriculture" | "writing" | "steam_power" | "electricity" | "radio" | "spaceflight" | "crewed_spaceflight" | "moon_landing" | "interplanetary_colonies" | "interstellar_probes" | "nuclear_weapons" | "fusion_power" => 5,
                _ => match tech.era.as_str() {
                    "Paleolithic" => 2,
                    "Neolithic" | "Bronze Age" | "Iron Age" => 3,
                    _ => 4,
                },
            };
            ev.push(CivEvent { importance, category: C::Technology, title: tech.name.clone(), detail: format!("{} — {}", tech.description, drivers) });
            if tech.unlocks.iter().any(|u| u == "radio") && self.radio_since.is_none() {
                self.radio_since = Some(t);
                ev.push(CivEvent { importance: 5, category: C::Contact, title: "First radio transmissions".into(), detail: "Signals begin spreading into space at the speed of light.".into() });
            }
            if tech.unlocks.iter().any(|u| u == "electric_light") {
                ev.push(CivEvent { importance: 4, category: C::Civilization, title: "Cities light up the night side".into(), detail: "Electric lighting makes the civilization visible from orbit.".into() });
            }
        }

        // ── Energy, fossil fuels and climate forcing ────────────────────
        // Energy transition: with alternatives known, the fossil share falls towards their
        // floor — slowly by default, quickly once depletion or energy pressure bites.
        let alternatives = [("renewables", 0.3), ("nuclear_power", 0.6), ("fusion_power", 0.05)]
            .iter()
            .filter(|(id, _)| self.flags.contains(*id) || self.knows(id))
            .map(|(_, cap)| *cap)
            .fold(f64::INFINITY, f64::min);
        if alternatives.is_finite() && self.fossil_share > alternatives * 0.3 {
            let use_per_year = self.population * self.energy_per_capita * self.fossil_share * SECONDS_PER_YEAR / FOSSIL_JOULES_PER_UNIT;
            let years_left = (body.resources.coal + body.resources.oil) / use_per_year.max(1e-12);
            let urgency = (self.pressures.energy + self.pressures.climate * 0.5 + (80.0 / years_left.max(1.0)).min(1.0)).min(1.5);
            let floor = alternatives * 0.3;
            self.fossil_share -= (self.fossil_share - floor) * relax(0.004 + 0.02 * urgency, dt);
        }
        let demand_w = self.population * self.energy_per_capita;
        let fossil_w = demand_w * self.fossil_share;
        let reserves = body.resources.coal + body.resources.oil;
        let available_fossil_w = if reserves > 0.0 { fossil_w.min(reserves * FOSSIL_JOULES_PER_UNIT / SECONDS_PER_YEAR / 30.0) } else { 0.0 };
        self.energy_shortfall = if demand_w > 0.0 { ((fossil_w - available_fossil_w) / demand_w).clamp(0.0, 0.8) } else { 0.0 };
        if self.energy_shortfall > 0.05 {
            self.pressures.energy = self.pressures.energy.max(self.energy_shortfall * 2.0).min(1.0);
        }
        if available_fossil_w > 0.0 && reserves > 0.0 {
            let used = available_fossil_w * SECONDS_PER_YEAR * dt / FOSSIL_JOULES_PER_UNIT;
            let coal_share = body.resources.coal / reserves;
            body.resources.coal = (body.resources.coal - used * coal_share).max(0.0);
            body.resources.oil = (body.resources.oil - used * (1.0 - coal_share)).max(0.0);
            let atm_mass_scale = body.atmosphere.pressure_bar.max(0.05) * body.radius_earths().powi(2);
            body.atmosphere.co2 += available_fossil_w * dt * CO2_PER_WATT_YEAR / atm_mass_scale;
        }
        body.atmosphere.co2 -= (body.atmosphere.co2 - self.baseline_co2) * relax(0.002, dt);

        // ── Shocks ──────────────────────────────────────────────────────
        let density_risk = (self.urbanisation * 2.0 + 0.05) * if self.population > 1e6 { 1.0 } else { 0.2 };
        let zoonotic = if self.knows("animal_domestication") { 1.5 } else { 1.0 };
        if rng.chance(over(cp.pandemic_rate * density_risk * zoonotic * (1.0 - self.health).max(0.05), dt)) {
            let frac = rng.range(0.03, 0.35) * (1.0 - self.health).max(0.05);
            let dead = self.population * frac;
            self.population -= dead;
            // Pressure scales with how deadly it was: a mild outbreak is not a crisis.
            self.pressures.disease = self.pressures.disease.max((frac * 4.0).min(1.0));
            self.stability -= 0.05;
            ev.push(CivEvent { importance: if frac > 0.1 { 4 } else { 3 }, category: C::Disaster, title: "Pandemic".into(), detail: format!("{:.0}% of the population ({}) dies", frac * 100.0, group_digits(dead)) });
        }
        let organised = self.sites.iter().filter(|s| s.active()).count() > 8;
        let rival_states = self.polities.iter().filter(|p| p.alive()).count() > 1;
        // Between rival states, wars are fought by polities (see below); a lone state still
        // suffers revolts and civil strife.
        if organised && !rival_states && rng.chance(over(cp.war_rate * (1.4 - self.stability).max(0.1) * (1.0 + self.pressures.food), dt)) {
            let nuclear = self.flags.contains("nuclear_weapons") && self.stability < 0.35 && rng.chance(0.05);
            let frac = if nuclear { rng.range(0.3, 0.8) } else { rng.range(0.002, 0.06) };
            self.population -= self.population * frac;
            self.pressures.war = self.pressures.war.max((frac / 0.06).min(1.0));
            self.stability -= if nuclear { 0.5 } else { 0.08 };
            if nuclear {
                ev.push(CivEvent { importance: 5, category: C::War, title: "Nuclear war".into(), detail: format!("{:.0}% of the population is killed", frac * 100.0) });
            } else if frac > 0.02 {
                ev.push(CivEvent { importance: if frac > 0.045 { 3 } else { 2 }, category: C::War, title: "Major war".into(), detail: format!("{:.1}% of the population is killed", frac * 100.0) });
            }
        }
        if rng.chance(over(cp.disaster_rate * body.geology.min(3.0) * 0.2, dt)) {
            let frac = rng.range(0.0005, 0.02);
            self.population -= self.population * frac;
            if frac > 0.01 {
                ev.push(CivEvent { importance: 2, category: C::Disaster, title: "Natural disaster".into(), detail: format!("Earthquake, eruption or flood kills {:.1}%", frac * 100.0) });
            }
        }

        // ── Stability & collapse ────────────────────────────────────────
        let target = self.base_stability - 0.25 * self.pressures.food - 0.2 * self.pressures.climate - 0.2 * self.energy_shortfall - 0.1 * self.pressures.war;
        self.stability += (target - self.stability) * relax(0.04, dt) + rng.normal(0.0, 0.01 * dt.sqrt().min(3.0));
        self.stability = self.stability.clamp(0.0, 1.0);
        if self.stability < cp.collapse_threshold && matches!(self.status, CivStatus::Thriving) && self.population > 1e5 {
            self.collapses += 1;
            self.status = CivStatus::Collapsed { since: t };
            self.population *= 0.5;
            let loss = if self.flags.contains("printing") { 0.9 } else if self.flags.contains("writing") { 0.7 } else { 0.5 };
            for k in self.knowledge.iter_mut() {
                *k *= loss;
            }
            ev.push(CivEvent { importance: 4, category: C::Civilization, title: "Societal collapse".into(), detail: "Institutions fail; population halves and knowledge is lost. A dark age begins.".into() });
        } else if let CivStatus::Collapsed { since } = self.status {
            if self.stability > 0.45 && t - since > 50.0 * SECONDS_PER_YEAR {
                self.status = CivStatus::Thriving;
                ev.push(CivEvent { importance: 3, category: C::Civilization, title: "Recovery".into(), detail: "Order returns after the dark age.".into() });
            }
        }

        // ── Urbanisation & settlements (every 10 years) ─────────────────
        self.urbanisation += (self.urban_target - self.urbanisation) * relax(0.01, dt);
        self.settlement_timer += dt;
        if self.settlement_timer >= 10.0 {
            let elapsed = self.settlement_timer;
            self.settlement_timer = 0.0;
            let p = settlements::SpreadParams {
                reach: self.reach,
                seafaring: self.flags.contains("seafaring") || self.flags.contains("aircraft"),
                short_crossings: self.flags.contains("short_crossings"),
                roads: self.flags.contains("roads"),
                rail: self.flags.contains("rail"),
                urbanisation: self.urbanisation,
                rural_cap: if self.knows("agriculture") { 2_500.0 } else { 150.0 },
            };
            if self.adjacency.is_empty() {
                self.adjacency = settlements::Adjacency::build(&self.sites, &world.surface, self.species.habitat);
            }
            let knows_agriculture = self.knows("agriculture");
            let promotions = settlements::update(&mut self.sites, &mut self.links, &self.adjacency, self.population, &p, t);
            let politics = if self.static_polities {
                polity::tally(&self.sites, &mut self.polities);
                polity::PoliticsOutput { events: Vec::new(), war_deaths: 0.0, active_wars: 0 }
            } else {
                polity::update(
                &mut self.sites,
                &mut self.polities,
                &self.adjacency,
                &polity::PoliticsInput {
                    flags: &self.flags,
                    stability: self.stability,
                    food_pressure: self.pressures.food,
                    reach: self.reach,
                    seafaring: p.seafaring,
                    knows_agriculture,
                },
                elapsed,
                t,
                rng,
            )
            };
            if politics.war_deaths > 0.0 {
                let frac = politics.war_deaths.min(0.5);
                self.population *= 1.0 - frac;
                self.pressures.war = self.pressures.war.max((frac / 0.06).min(1.0));
                self.stability -= 0.02 * politics.active_wars.min(5) as f64;
                let nuclear = self.flags.contains("nuclear_weapons") && self.stability < 0.3 && rng.chance(over(0.004 * politics.active_wars as f64, elapsed));
                if nuclear {
                    let f = rng.range(0.3, 0.8);
                    self.population *= 1.0 - f;
                    self.stability -= 0.5;
                    ev.push(CivEvent { importance: 5, category: C::War, title: "Nuclear war".into(), detail: format!("Rival states exchange nuclear weapons; {:.0}% of the population is killed.", f * 100.0) });
                }
            }
            ev.extend(politics.events);
            for (site, tier) in promotions {
                if self.highest_tier.is_none_or(|h| tier > h) && tier >= Tier::Village {
                    self.highest_tier = Some(tier);
                    let name = &self.sites[site].name;
                    ev.push(CivEvent { importance: if tier >= Tier::City { 4 } else { 3 }, category: C::Civilization, title: format!("First {}: {}", tier.label().to_lowercase(), name), detail: format!("{} grows past {} inhabitants", name, group_digits(self.sites[site].population)) });
                }
            }
        }

        // ── Space activity ──────────────────────────────────────────────
        if self.flags.contains("satellites") {
            let target = ((self.total_power_w() / 1e9).sqrt() * 10.0) as u32;
            if self.satellites == 0 {
                ev.push(CivEvent { importance: 5, category: C::Space, title: "First artificial satellite".into(), detail: "An object built by the civilization now orbits its world.".into() });
            }
            if self.satellites < target {
                self.satellites += 1 + (target - self.satellites) / 20;
            }
        }
        ev.extend(self.step_space(&world.destinations, t, dt, rng));

        if self.population < 500.0 {
            self.status = CivStatus::Extinct { at: t };
            ev.push(CivEvent { importance: 5, category: C::Civilization, title: "Extinction".into(), detail: format!("The {} are gone.", self.species.name) });
        }

        self.samples.push(Sample { t, population: self.population, knowledge: self.knowledge.iter().sum(), techs: self.discoveries.len() as u32, energy_w: self.total_power_w(), stability: self.stability });
        ev
    }
}
