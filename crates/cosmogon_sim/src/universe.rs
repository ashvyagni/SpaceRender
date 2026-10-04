//! The universe: owner of all simulation state and the single entry point for advancing it.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::astro::{generate, sol, Body, BodyKind, StarSystem, LIGHT_YEAR, SPEED_OF_LIGHT};
use crate::civ::species::{Habitat, Species};
use crate::civ::tech::TechGraph;
use crate::civ::{environment_for, CivStatus, Civilization, WorldView};
use crate::habitability::{assess, Habitability};
use crate::history::{Category, Event, History};
use crate::life::{Biosphere, LifeEvent, LifeMultipliers, Stage, STEP_SECONDS};
use crate::params::ScienceParams;
use crate::planet::environment::update_climate;
use crate::planet::terrain::{self, SurfaceContext};
use crate::rng::{domain, Rng};
use crate::scheduler::Scheduler;
use crate::time::{format_date, SECONDS_PER_GYR, SECONDS_PER_MYR, SECONDS_PER_YEAR};
use crate::Vec3d;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyRef {
    pub system: u32,
    pub body: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scenario {
    /// A procedurally generated stellar neighbourhood. Anything — or nothing — may happen.
    Neighbourhood,
    /// As above, but the central system has an Earth-like world already rich in complex life.
    GardenWorld,
    /// The real Solar System, 200,000 years ago, with early humans on Earth.
    Sol,
    /// The real Solar System today, started from NASA/JPL Horizons state vectors, with
    /// dynamic (N-body) gravity. Earth has its present biosphere; no civilization model.
    SolarSystemLab,
    /// A single procedurally generated star system.
    StarSystem,
    /// A Sun-like star and nothing else: build your own system.
    EmptySystem,
}

impl Scenario {
    pub const ALL: [Scenario; 6] = [Scenario::SolarSystemLab, Scenario::Sol, Scenario::GardenWorld, Scenario::StarSystem, Scenario::Neighbourhood, Scenario::EmptySystem];
    pub fn label(self) -> &'static str {
        match self {
            Scenario::Neighbourhood => "Stellar neighbourhood",
            Scenario::GardenWorld => "Garden world",
            Scenario::Sol => "Sol — Dawn of Humanity",
            Scenario::SolarSystemLab => "Solar System Lab",
            Scenario::StarSystem => "Procedural star system",
            Scenario::EmptySystem => "Empty system",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Scenario::Neighbourhood => "A procedural neighbourhood of stars with realistic statistics. With realistic settings you may never see intelligent life.",
            Scenario::GardenWorld => "The central star hosts an Earth-like world teeming with complex life. Watch whether intelligence emerges, and what it becomes.",
            Scenario::Sol => "The real Solar System 200,000 years ago. Early humans have fire and stone tools; their history is not written yet.",
            Scenario::SolarSystemLab => "The real Solar System on 1 January 2026, from NASA/JPL Horizons state vectors, with dynamic N-body gravity. Earth has its present-day biosphere.",
            Scenario::StarSystem => "One physically plausible star system generated from the seed.",
            Scenario::EmptySystem => "A Sun-like star alone in space. Add planets, moons and asteroids yourself.",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UniverseSettings {
    pub seed: u64,
    pub scenario: Scenario,
    pub system_count: u32,
    /// Multiplies every life-transition rate.
    pub life_rate: f64,
    /// Additionally multiplies the emergence of intelligence.
    pub intelligence_rate: f64,
    /// Multiplies research output and discovery rates.
    pub tech_rate: f64,
    pub resource_abundance: f64,
    /// Start the home system with dynamic (N-body) gravity using these settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physics: Option<crate::astro::dynamics::PhysicsSettings>,
    /// Which simulation layers run.
    #[serde(default, skip_serializing_if = "EnabledSystems::all")]
    pub systems: EnabledSystems,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnabledSystems {
    /// Biospheres evolve (life can arise, evolve and go extinct).
    pub life: bool,
    /// Intelligent species and civilizations are simulated.
    pub civilization: bool,
}

impl Default for EnabledSystems {
    fn default() -> Self {
        Self { life: true, civilization: true }
    }
}

impl EnabledSystems {
    pub fn all(&self) -> bool {
        self.life && self.civilization
    }
}

impl Default for UniverseSettings {
    fn default() -> Self {
        Self { seed: 1, scenario: Scenario::Neighbourhood, system_count: 24, life_rate: 1.0, intelligence_rate: 1.0, tech_rate: 1.0, resource_abundance: 1.0, physics: None, systems: EnabledSystems::default() }
    }
}

/// Named presets for the life multipliers.
pub const LIFE_PRESETS: &[(&str, f64, f64, &str)] = &[
    ("Realistic", 1.0, 1.0, "Best-guess rates. Intelligent life is rare; most universes stay quiet."),
    ("Hopeful", 4.0, 6.0, "Life arises readily and intelligence is less of a fluke."),
    ("Teeming", 15.0, 40.0, "Life everywhere it can exist. Good for watching many civilizations."),
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Probe {
    pub civ: u32,
    pub from: u32,
    pub to: u32,
    pub launched: f64,
    /// m/s
    pub speed: f64,
    pub arrived: bool,
}

impl Probe {
    pub fn position(&self, systems: &[StarSystem], t: f64) -> Vec3d {
        let a = systems[self.from as usize].position;
        let b = systems[self.to as usize].position;
        let total = (b - a).length();
        let f = if total > 0.0 { ((t - self.launched) * self.speed / total).clamp(0.0, 1.0) } else { 1.0 };
        a.lerp(b, f)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AdvanceReport {
    pub steps: u64,
    /// The CPU budget ran out before reaching the requested time.
    pub lagging: bool,
    /// Index into `history.events` of a milestone that stopped the advance.
    pub milestone: Option<usize>,
    pub cpu_time: Duration,
}

const TASK_BIOSPHERE: usize = 0;
pub(crate) const TASK_CIV: usize = 1;
/// Orbit → climate refresh for dynamic systems (added when the first system becomes dynamic).
pub(crate) const TASK_ENV: usize = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Universe {
    pub settings: UniverseSettings,
    pub params: ScienceParams,
    pub start_time: f64,
    pub time: f64,
    pub systems: Vec<StarSystem>,
    pub biospheres: Vec<Biosphere>,
    pub civs: Vec<Civilization>,
    pub probes: Vec<Probe>,
    pub history: History,
    pub scheduler: Scheduler,
    /// Journal of user modifications (sandboxes): with the initial state this reproduces
    /// the experiment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<crate::sandbox::EditRecord>,
    /// Supernova radiation fronts still travelling between the stars.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blasts: Vec<crate::stellar::Blast>,
    /// Generation ships between the stars.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starships: Vec<crate::expansion::Starship>,
}

fn life_mult(s: &UniverseSettings) -> LifeMultipliers {
    LifeMultipliers { life: s.life_rate, intelligence: s.intelligence_rate * s.life_rate }
}

/// Fertile land and fresh water follow from climate and the biosphere.
pub(crate) fn refresh_derived_resources(body: &mut Body, vegetated: bool) {
    let land = body.land_fraction() / 0.29;
    let temperate = (-((body.temperature - 288.0) / 25.0).powi(2)).dexp();
    let liquid = body.hydro.ocean_fraction > 0.0;
    body.resources.fertile_land = if liquid { land.min(2.5) * temperate * if vegetated { 1.0 } else { 0.05 } } else { 0.0 };
    body.resources.fresh_water = if liquid { body.hydro.water_inventory.sqrt().min(1.5) } else if body.hydro.ice_fraction > 0.0 { 0.1 } else { 0.0 };
}

pub(crate) fn refresh_climate(sys: &mut StarSystem, b: usize, t: f64) {
    let d = sys.stellar_distance(b);
    let star = sys.star.clone();
    let before = sys.bodies[b].hydro.ocean_fraction;
    update_climate(&mut sys.bodies[b], &star, t, d);
    let body = &mut sys.bodies[b];
    if (body.hydro.ocean_fraction - before).abs() > 0.02 {
        body.sea_level = terrain::Terrain::of(body).sea_level_for(body.hydro.ocean_fraction);
    }
    // Hot, wet worlds lose water to space (moist / runaway greenhouse).
    if body.temperature > 340.0 && body.hydro.water_inventory > 0.0 {
        body.hydro.water_inventory *= 0.995;
    }
}

impl Universe {
    /// Create a universe. This runs the deterministic prehistory of every biosphere, so it
    /// can take a few seconds; call it off the main thread in interactive applications.
    pub fn new(settings: UniverseSettings) -> Self {
        let params = ScienceParams::default();
        let mut systems = generate::generate_systems(&settings);
        let mut start_time = if settings.scenario == Scenario::Sol { sol::dawn_of_humanity_start() } else { 0.0 };
        if settings.scenario == Scenario::SolarSystemLab {
            start_time = crate::astro::horizons::apply_to_sol(&mut systems[0], settings.physics.unwrap_or_default());
        }

        let mut biospheres = Vec::new();
        for (si, sys) in systems.iter_mut().enumerate() {
            for b in 0..sys.bodies.len() {
                if !sys.bodies[b].kind.has_surface() {
                    continue;
                }
                refresh_climate(sys, b, start_time);
                biospheres.push(Biosphere::new(si as u32, b as u32, assess(sys, b, start_time), start_time));
            }
        }

        let mut history = History::default();
        let mut u = Self {
            settings: settings.clone(),
            params,
            start_time,
            time: start_time,
            systems,
            biospheres: Vec::new(),
            civs: Vec::new(),
            probes: Vec::new(),
            history: History::default(),
            scheduler: Scheduler::default(),
            edits: Vec::new(),
            blasts: Vec::new(),
            starships: Vec::new(),
        };
        let prehistory_events = u.run_prehistory(&mut biospheres);
        u.biospheres = biospheres;
        for e in prehistory_events {
            history.push(e);
        }
        u.history = history;
        u.history.push(Event {
            time: start_time,
            category: Category::Astronomy,
            importance: 3,
            title: "Observation begins".into(),
            detail: format!("{} — seed {}, {} star systems", settings.scenario.label(), settings.seed, u.systems.len()),
            system: None,
            body: None,
            civ: None,
        });

        match settings.scenario {
            Scenario::GardenWorld => u.seed_garden_world(),
            Scenario::Sol => u.seed_humanity(),
            Scenario::SolarSystemLab => {
                u.seed_present_earth();
                if settings.systems.civilization {
                    u.seed_present_humanity();
                }
            }
            Scenario::Neighbourhood | Scenario::StarSystem | Scenario::EmptySystem => {}
        }

        u.scheduler.add("biosphere", start_time, STEP_SECONDS);
        u.scheduler.add("civilizations", start_time, SECONDS_PER_YEAR);
        if let (Some(p), false) = (settings.physics, u.systems[0].is_dynamic()) {
            u.systems[0].activate_dynamics(start_time, p);
        }
        u.ensure_environment_task();
        u.ensure_star_task();
        if u.systems.iter().any(|s| s.is_dynamic()) {
            u.step_environment(start_time);
        }
        u
    }

    /// Simulate every biosphere from planet formation to the start epoch (in parallel; each
    /// biosphere is independent and uses its own RNG streams, so this is deterministic).
    /// Intelligence is not allowed to arise before observation begins.
    fn run_prehistory(&mut self, biospheres: &mut [Biosphere]) -> Vec<Event> {
        let seed = self.settings.seed;
        let params = &self.params;
        let mult = life_mult(&self.settings);
        let start = self.start_time;
        let skip_earth = self.settings.scenario == Scenario::Sol;
        // The real-data lab starts from observations: no simulated prehistory anywhere (no
        // invented life on Mars); Earth's present biosphere is set from what we know.
        let skip_all = self.settings.scenario == Scenario::SolarSystemLab || !self.settings.systems.life;
        let systems = &self.systems;

        let jobs: Vec<(usize, Biosphere, StarSystem)> = biospheres
            .iter()
            .enumerate()
            .filter(|_| !skip_all)
            .filter(|(_, bio)| !(skip_earth && bio.system == 0 && systems[0].bodies[bio.body as usize].name == "Earth"))
            .map(|(i, bio)| (i, bio.clone(), systems[bio.system as usize].clone()))
            .collect();
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16);
        let chunk = jobs.len().div_ceil(threads).max(1);

        let results: Vec<(usize, Biosphere, Body, Vec<Event>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = jobs
                .chunks(chunk)
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|(i, bio, sys)| {
                                let (bio, body, ev) = prehistory_one(seed, params, mult, bio.clone(), sys.clone(), start);
                                (*i, bio, body, ev)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().expect("prehistory thread panicked")).collect()
        });

        let mut events = Vec::new();
        for (i, bio, body, ev) in results {
            let (s, b) = (bio.system as usize, bio.body as usize);
            self.systems[s].bodies[b] = body;
            biospheres[i] = bio;
            events.extend(ev);
        }
        events.sort_by(|a, b| a.time.total_cmp(&b.time).then(a.system.cmp(&b.system)).then(a.body.cmp(&b.body)));
        events
    }

    fn seed_garden_world(&mut self) {
        let Some(bi) = self.biospheres.iter().position(|b| {
            if b.system != 0 {
                return false;
            }
            let body = &self.systems[0].bodies[b.body as usize];
            body.kind == BodyKind::Rocky && body.hydro.ocean_fraction > 0.3 && b.habitability.score > 0.0
        }) else {
            return;
        };
        let t = self.start_time;
        let bio = &mut self.biospheres[bi];
        let body = &mut self.systems[0].bodies[bio.body as usize];
        if bio.stage < Stage::ComplexEcosystems {
            bio.stage = Stage::ComplexEcosystems;
            bio.stage_since = t - 400.0 * SECONDS_PER_MYR;
            bio.photosynthesis_since = Some(t - 2.5 * SECONDS_PER_GYR);
            bio.biomass = 1.0;
            bio.biodiversity = 0.85;
            bio.land_life_myr = 400.0;
            body.atmosphere.o2 = 0.21;
            body.atmosphere.n2 = (body.atmosphere.n2 - 0.21).max(0.0);
            body.atmosphere.normalise();
            body.resources.coal = body.resources.coal.max(0.8);
            body.resources.oil = body.resources.oil.max(0.8);
        }
        let name = body.name.clone();
        let sys_idx = bio.system as usize;
        let b = bio.body as usize;
        refresh_climate(&mut self.systems[sys_idx], b, t);
        let hab = assess(&self.systems[sys_idx], b, t);
        let bio = &mut self.biospheres[bi];
        bio.habitability = hab;
        refresh_derived_resources(&mut self.systems[sys_idx].bodies[b], bio.vegetated());
        self.history.push(Event { time: t, category: Category::Life, importance: 4, title: format!("{name} is a living world"), detail: "Forests, oceans and complex animal life cover the planet.".into(), system: Some(0), body: Some(b as u32), civ: None });
    }

    /// Earth today: a mature biosphere (the civilization model does not yet start from the
    /// present day; that arrives with the consequence-pipeline milestone).
    fn seed_present_earth(&mut self) {
        let t = self.start_time;
        let Some(earth) = self.systems[0].find_body("Earth") else { return };
        let Some(bi) = self.biospheres.iter().position(|b| b.system == 0 && b.body as usize == earth) else { return };
        let bio = &mut self.biospheres[bi];
        bio.stage = Stage::ComplexEcosystems;
        bio.stage_since = -541.0 * SECONDS_PER_MYR;
        bio.photosynthesis_since = Some(-2.7 * SECONDS_PER_GYR);
        bio.biomass = 1.0;
        bio.biodiversity = 0.9;
        bio.land_life_myr = 470.0;
        refresh_derived_resources(&mut self.systems[0].bodies[earth], true);
        let hab = assess(&self.systems[0], earth, t);
        self.biospheres[bi].habitability = hab;
    }

    fn seed_humanity(&mut self) {
        let t = self.start_time;
        let Some(earth) = self.systems[0].find_body("Earth") else { return };
        let bi = self.biospheres.iter().position(|b| b.system == 0 && b.body as usize == earth).expect("Earth biosphere");
        {
            let bio = &mut self.biospheres[bi];
            bio.stage = Stage::Intelligent;
            bio.stage_since = t;
            bio.photosynthesis_since = Some(-2.7 * SECONDS_PER_GYR);
            bio.biomass = 1.0;
            bio.biodiversity = 0.9;
            bio.land_life_myr = 470.0;
        }
        refresh_derived_resources(&mut self.systems[0].bodies[earth], true);
        let species = Species {
            name: "Humans".into(),
            habitat: Habitat::Land,
            mass_kg: 62.0,
            lifespan_years: 70.0,
            sociality: 0.75,
            traits: vec!["Terrestrial".into(), "Persistence hunters".into(), "Tribal social structure".into()],
        };
        let mut civ = self.make_civ(BodyRef { system: 0, body: earth as u32 }, species, t);
        civ.name = "Humanity".into();
        // Anatomically modern humans begin in East Africa; where they spread from there is
        // up to geography, reach and seafaring.
        let origin = terrain::dir_from_lat_lon((-3.0f64).to_radians(), 36.0f64.to_radians());
        if let Some(i) = (0..civ.sites.len()).min_by(|&a, &b| {
            let d = |i: usize| {
                let v = civ.sites[i].dir();
                -(v[0] * origin[0] + v[1] * origin[1] + v[2] * origin[2])
            };
            d(a).total_cmp(&d(b))
        }) {
            civ.sites.swap(0, i);
        }
        civ.population = 100_000.0;
        let graph = TechGraph::embedded();
        for id in ["stone_tools", "fire", "hunting_weapons", "language"] {
            civ.grant(graph, id, t);
        }
        for k in civ.knowledge.iter_mut() {
            *k = 150.0;
        }
        self.biospheres[bi].civilization_present = true;
        self.history.push(Event { time: t, category: Category::Civilization, importance: 5, title: "Humanity".into(), detail: "Anatomically modern humans live as hunter-gatherers with fire, stone tools and language.".into(), system: Some(0), body: Some(earth as u32), civ: Some(civ.id) });
        self.civs.push(civ);
    }

    fn surface_context(&self, r: BodyRef) -> SurfaceContext {
        let vegetated = self.biosphere(r).is_some_and(|b| b.vegetated());
        SurfaceContext::new(self.body(r), vegetated)
    }

    pub(crate) fn make_civ(&mut self, r: BodyRef, species: Species, t: f64) -> Civilization {
        let id = self.civs.len() as u32;
        let surface = self.surface_context(r);
        let moons = self.systems[r.system as usize].moons_of(r.body as usize).count();
        let mut rng = Rng::stream(self.settings.seed, domain::SETTLEMENT_SITES, &[r.system as u64, r.body as u64, id as u64]);
        Civilization::new(id, r.system, r.body, self.body(r), species, &surface, moons, t, &mut rng)
    }

    // ── Queries ─────────────────────────────────────────────────────────

    pub fn system(&self, id: u32) -> &StarSystem {
        &self.systems[id as usize]
    }
    pub fn body(&self, r: BodyRef) -> &Body {
        &self.systems[r.system as usize].bodies[r.body as usize]
    }
    pub fn body_position(&self, r: BodyRef, t: f64) -> Vec3d {
        self.systems[r.system as usize].body_position(r.body as usize, t)
    }
    pub fn biosphere(&self, r: BodyRef) -> Option<&Biosphere> {
        self.biospheres.iter().find(|b| b.system == r.system && b.body == r.body)
    }
    pub fn civ_on(&self, r: BodyRef) -> Option<&Civilization> {
        self.civs.iter().rev().find(|c| c.system == r.system && c.body == r.body && c.is_alive())
    }
    pub fn habitability(&self, r: BodyRef) -> Habitability {
        match self.biosphere(r) {
            Some(b) => b.habitability.clone(),
            None => assess(self.system(r.system), r.body as usize, self.time),
        }
    }
    pub fn gregorian(&self) -> bool {
        matches!(self.settings.scenario, Scenario::Sol | Scenario::SolarSystemLab)
    }
    pub fn date_label(&self) -> String {
        format_date(self.time, self.start_time, self.gregorian())
    }
    pub fn tech_graph(&self) -> &'static TechGraph {
        TechGraph::embedded()
    }

    // ── Advancing ───────────────────────────────────────────────────────

    /// Advance the clock to `target`, running every scheduled step that falls due.
    ///
    /// * `budget` — stop early (and report `lagging`) when this much CPU time is spent.
    ///   The clock then stays at the last completed step, so the outcome is unaffected.
    /// * `stop_at_importance` — stop right after an event of at least this importance, so a
    ///   viewer can slow down for milestones.
    pub fn advance_to(&mut self, target: f64, budget: Option<Duration>, stop_at_importance: Option<u8>) -> AdvanceReport {
        let started = Instant::now();
        let deadline = budget.map(|b| started + b);
        let mut report = AdvanceReport::default();
        let history_len = self.history.events.len();
        while let Some((task, due)) = self.scheduler.next() {
            if due > target {
                break;
            }
            // Dynamic systems must be at `due` before anything reads their state.
            if !self.advance_physics(due, deadline) {
                report.lagging = true;
                report.cpu_time = started.elapsed();
                return report;
            }
            let k = self.scheduler.tasks[task].steps;
            let mut skip = None;
            let mut star_skip = None;
            match task {
                TASK_BIOSPHERE => {
                    if self.settings.systems.life {
                        self.step_biospheres(due, k)
                    }
                }
                TASK_CIV => skip = Some(if self.settings.systems.civilization { self.step_civs(due, k) } else { k + 1_000_000 }),
                TASK_ENV => self.step_environment(due),
                crate::stellar::TASK_STARS => star_skip = Some(self.step_stars(due, k)),
                _ => {}
            }
            self.scheduler.complete(task);
            if let Some(next_k) = skip {
                self.scheduler.skip_to(TASK_CIV, next_k);
            }
            if let Some(next_k) = star_skip {
                self.scheduler.skip_to(crate::stellar::TASK_STARS, next_k);
            }
            self.time = due;
            report.steps += 1;
            if let Some(min) = stop_at_importance {
                if let Some(i) = (history_len..self.history.events.len()).find(|&i| self.history.events[i].importance >= min) {
                    report.milestone = Some(i);
                    report.cpu_time = started.elapsed();
                    return report;
                }
            }
            if let Some(b) = budget {
                if report.steps % 16 == 0 && started.elapsed() > b {
                    report.lagging = true;
                    report.cpu_time = started.elapsed();
                    return report;
                }
            }
        }
        if !self.advance_physics(target, deadline) {
            report.lagging = true;
            report.cpu_time = started.elapsed();
            return report;
        }
        self.time = self.time.max(target);
        report.cpu_time = started.elapsed();
        report
    }

    /// Integrate every dynamic system up to `to`, applying collision consequences on the
    /// way. Returns false if the deadline stopped it first (the clock then stays at the
    /// earliest state reached, so nothing is skipped).
    fn advance_physics(&mut self, to: f64, deadline: Option<Instant>) -> bool {
        let mut all = true;
        for s in 0..self.systems.len() {
            if !self.systems[s].is_dynamic() {
                continue;
            }
            loop {
                let mut contacts = Vec::new();
                let reached = self.systems[s].advance_dynamics(to, deadline, &mut contacts);
                let had = !contacts.is_empty();
                for c in contacts {
                    self.on_contact(s, c);
                }
                self.apply_pending_contacts(s);
                if reached {
                    break;
                }
                if !had {
                    all = false;
                    break;
                }
            }
        }
        if !all {
            let reached = self.systems.iter().filter_map(|s| s.dynamics.as_ref()).map(|d| d.time()).fold(f64::INFINITY, f64::min);
            if reached.is_finite() {
                self.time = self.time.max(reached.min(to));
            }
        }
        all
    }

    pub fn advance_by(&mut self, dt: f64) -> AdvanceReport {
        self.advance_to(self.time + dt, None, None)
    }

    fn step_biospheres(&mut self, t: f64, k: u64) {
        let seed = self.settings.seed;
        let mult = life_mult(&self.settings);
        let mut new_intelligence = Vec::new();
        for bi in 0..self.biospheres.len() {
            let (s, b) = (self.biospheres[bi].system as usize, self.biospheres[bi].body as usize);
            if !self.systems[s].bodies[b].exists() {
                continue;
            }
            if k % 100 == 0 {
                crate::planet::environment::carbon_cycle(&mut self.systems[s].bodies[b], 1.0);
                refresh_climate(&mut self.systems[s], b, t);
                let hab = assess(&self.systems[s], b, t);
                let bio = &mut self.biospheres[bi];
                bio.habitability = hab;
                let vegetated = bio.vegetated();
                refresh_derived_resources(&mut self.systems[s].bodies[b], vegetated);
            }
            let bio = &mut self.biospheres[bi];
            if bio.habitability.score <= 0.0 && bio.stage == Stage::Sterile {
                continue;
            }
            let flare = self.systems[s].star.flare_activity_at(t);
            let mut rng = Rng::stream(seed, domain::BIOSPHERE, &[s as u64, b as u64, k]);
            let body = &mut self.systems[s].bodies[b];
            let events = bio.step(body, &self.params, mult, flare, t, &mut rng);
            let name = body.name.clone();
            for e in events {
                if e == LifeEvent::StageReached(Stage::Intelligent) {
                    new_intelligence.push(BodyRef { system: s as u32, body: b as u32 });
                }
                if let Some(ev) = life_event(&e, &name, t, s as u32, b as u32) {
                    self.history.push(ev);
                }
            }
        }
        for r in new_intelligence {
            self.spawn_civilization(r, t);
        }
    }

    fn spawn_civilization(&mut self, r: BodyRef, t: f64) {
        let mut rng = Rng::stream(self.settings.seed, domain::SPECIES, &[r.system as u64, r.body as u64, self.civs.len() as u64]);
        let species = Species::generate(&mut rng, self.body(r));
        let mut civ = self.make_civ(r, species, t);
        civ.next_k = self.scheduler.tasks.get(TASK_CIV).map(|task| task.steps).unwrap_or(0);
        if let Some(bio) = self.biospheres.iter_mut().find(|b| b.system == r.system && b.body == r.body) {
            bio.civilization_present = true;
        }
        let body_name = self.body(r).name.clone();
        self.history.push(Event {
            time: t,
            category: Category::Life,
            importance: 5,
            title: format!("Intelligence emerges on {body_name}"),
            detail: format!("The {} — {} — begin to use tools and language.", civ.species.name, civ.species.traits.join(", ").to_lowercase()),
            system: Some(r.system),
            body: Some(r.body),
            civ: Some(civ.id),
        });
        self.civs.push(civ);
    }

    /// Step every civilization that is due. Returns the task index at which the next
    /// civilization is due, so empty or quiet stretches cost nothing.
    fn step_civs(&mut self, t: f64, k: u64) -> u64 {
        let graph = TechGraph::embedded();
        let seed = self.settings.seed;
        for ci in 0..self.civs.len() {
            if !self.civs[ci].is_alive() || self.civs[ci].next_k > k {
                continue;
            }
            let dt = self.civs[ci].stride_years.max(1) as f64;
            let (s, b) = (self.civs[ci].system as usize, self.civs[ci].body as usize);
            let r = BodyRef { system: s as u32, body: b as u32 };
            if k % 10 == 0 || dt >= 10.0 {
                refresh_climate(&mut self.systems[s], b, t);
                let vegetated = self.biosphere(r).is_some_and(|x| x.vegetated());
                refresh_derived_resources(&mut self.systems[s].bodies[b], vegetated);
            }
            let sys = &self.systems[s];
            let moons = sys.moons_of(b).count();
            let destinations = destinations_from(sys, b, t);
            let colonisable = destinations.iter().filter(|d| d.colony > 0.0).count();
            let world = WorldView {
                env: environment_for(&sys.bodies[b], moons, colonisable, t),
                surface: self.surface_context(r),
                destinations,
                params: &self.params,
                tech_rate: self.settings.tech_rate,
            };
            let mut rng = Rng::stream(seed, domain::CIV_STEP, &[ci as u64, k]);
            let body = &mut self.systems[s].bodies[b];
            let events = self.civs[ci].step(body, &world, graph, t, dt, &mut rng);
            self.step_megastructures(ci, t, dt);
            let stride = self.civs[ci].choose_stride(t);
            self.civs[ci].stride_years = stride;
            self.civs[ci].next_k = k + stride as u64;
            for e in events {
                self.history.push(Event { time: t, category: e.category, importance: e.importance, title: e.title, detail: e.detail, system: Some(s as u32), body: Some(b as u32), civ: Some(ci as u32) });
            }
            if let CivStatus::Extinct { .. } = self.civs[ci].status {
                if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == r.system && x.body == r.body) {
                    bio.civilization_present = false;
                    bio.stage = Stage::ComplexEcosystems;
                    bio.stage_since = t;
                }
            }
        }
        self.step_contact(t);
        self.step_probes(t);
        self.step_starships(t);
        // Next due civilization; with none alive, sleep until the next biosphere step (the
        // only place a new one can appear).
        match self.civs.iter().filter(|c| c.is_alive()).map(|c| c.next_k).min() {
            Some(next) => next.max(k + 1),
            None => {
                let next_bio = self.scheduler.tasks[TASK_BIOSPHERE].next_due();
                self.scheduler.index_at_or_after(TASK_CIV, next_bio).max(k + 1)
            }
        }
    }

    /// Radio signals expand at light speed; civilizations with radio astronomy hear them.
    fn step_contact(&mut self, t: f64) {
        let n = self.civs.len();
        for a in 0..n {
            let Some(since) = self.civs[a].radio_since else { continue };
            let radius = SPEED_OF_LIGHT * (t - since);
            for b in 0..n {
                if a == b || !self.civs[b].is_alive() || !self.civs[b].flags.contains("radio_astronomy") || self.civs[b].detected.contains(&(a as u32)) {
                    continue;
                }
                let pa = self.systems[self.civs[a].system as usize].position;
                let pb = self.systems[self.civs[b].system as usize].position;
                let dist = (pa - pb).length();
                if radius >= dist {
                    self.civs[b].detected.push(a as u32);
                    self.civs[b].pressures.contact = 1.0;
                    let (name_a, name_b) = (self.civs[a].name.clone(), self.civs[b].name.clone());
                    self.history.push(Event {
                        time: t,
                        category: Category::Contact,
                        importance: 5,
                        title: format!("{name_b} detect signals from {name_a}"),
                        detail: format!("Radio emissions that left {:.1} light-years away are recognised as artificial.", dist / LIGHT_YEAR),
                        system: Some(self.civs[b].system),
                        body: Some(self.civs[b].body),
                        civ: Some(b as u32),
                    });
                }
            }
        }
    }

    fn step_probes(&mut self, t: f64) {
        for ci in 0..self.civs.len() {
            let c = &self.civs[ci];
            if !c.is_alive() || !c.flags.contains("probes") || (t - c.probe_timer) < 200.0 * SECONDS_PER_YEAR {
                continue;
            }
            self.civs[ci].probe_timer = t;
            let c = &self.civs[ci];
            let home = self.systems[c.system as usize].position;
            let targeted: Vec<u32> = self.probes.iter().filter(|p| p.civ == ci as u32).map(|p| p.to).collect();
            let target = self
                .systems
                .iter()
                .filter(|s| s.id != c.system && !targeted.contains(&s.id))
                .min_by(|a, b| (a.position - home).length().total_cmp(&(b.position - home).length()));
            if let Some(target) = target {
                let speed = self.params.civilization.probe_speed_c * SPEED_OF_LIGHT;
                let (to, to_name) = (target.id, target.name.clone());
                self.probes.push(Probe { civ: ci as u32, from: c.system, to, launched: t, speed, arrived: false });
                self.civs[ci].probes_launched += 1;
                let name = self.civs[ci].name.clone();
                self.history.push(Event { time: t, category: Category::Space, importance: 5, title: format!("Probe launched towards {to_name}"), detail: format!("{name} send a robotic emissary at {:.0}% of light speed.", self.params.civilization.probe_speed_c * 100.0), system: Some(self.civs[ci].system), body: Some(self.civs[ci].body), civ: Some(ci as u32) });
            }
        }
        for p in self.probes.iter_mut().filter(|p| !p.arrived) {
            let dist = (self.systems[p.to as usize].position - self.systems[p.from as usize].position).length();
            if (t - p.launched) * p.speed >= dist {
                p.arrived = true;
                self.history.push(Event { time: t, category: Category::Space, importance: 5, title: format!("Probe arrives at {}", self.systems[p.to as usize].name), detail: format!("After {:.0} years in flight.", (t - p.launched) / SECONDS_PER_YEAR), system: Some(p.to), body: None, civ: Some(p.civ) });
            }
        }
    }
}

/// The other worlds of a system as destinations for a civilization on body `home`.
pub fn destinations_from(sys: &StarSystem, home: usize, t: f64) -> Vec<crate::civ::space::Destination> {
    use crate::civ::space::{hohmann_time, Destination};
    let top_home = sys.top_level(home);
    let a_home = (sys.body_local_position(top_home, t) - sys.star_local_position(t)).length();
    sys.bodies
        .iter()
        .enumerate()
        .filter(|(j, x)| *j != home && x.exists())
        .map(|(j, x)| {
            let top = sys.top_level(j);
            let transfer = if top == top_home {
                // Within the home planet's family: from low orbit out to the moon's orbit.
                let planet = &sys.bodies[top_home];
                let r2 = if j == top_home { planet.radius * 1.05 } else { x.orbit.a };
                hohmann_time(planet.radius * 1.05, r2, planet.mu()).max(86_400.0)
            } else {
                let a = (sys.body_local_position(top, t) - sys.star_local_position(t)).length();
                hohmann_time(a_home, a, sys.star.mu())
            };
            let colony = if x.kind.has_surface() && x.mass > 1e21 { 1.0 / (1.0 + (x.gravity_g() - 0.7).abs()) * (-((x.temperature - 250.0) / 120.0).powi(2)).dexp() } else { 0.0 };
            Destination { body: j as u32, name: x.name.clone(), transfer, surface: x.kind.has_surface(), home_moon: x.parent == Some(home as u32), colony }
        })
        .collect()
}

fn life_event(e: &LifeEvent, body: &str, t: f64, s: u32, b: u32) -> Option<Event> {
    let (importance, category, title, detail) = match e {
        LifeEvent::StageReached(Stage::Intelligent) => return None, // reported by spawn_civilization
        LifeEvent::StageReached(stage) => {
            let imp = match stage {
                Stage::Prebiotic => 1,
                Stage::Microbial | Stage::ComplexEcosystems => 4,
                _ => 3,
            };
            (imp, Category::Life, format!("{} on {body}", stage.label()), format!("Life on {body} reaches a new stage: {}.", stage.label().to_lowercase()))
        }
        LifeEvent::Oxygenation => (3, Category::Life, format!("Great Oxidation on {body}"), "Photosynthesis fills the atmosphere with free oxygen.".into()),
        LifeEvent::MassExtinction { severity, regressed } => (
            if *regressed { 4 } else { 2 },
            Category::Disaster,
            format!("Mass extinction on {body}"),
            format!("{:.0}% of species are lost{}.", severity * 100.0, if *regressed { "; life is set back a full stage" } else { "" }),
        ),
        LifeEvent::Sterilised => (4, Category::Disaster, format!("{body} sterilised"), "A catastrophic event wipes out all life.".into()),
        LifeEvent::Collapse => (4, Category::Life, format!("Biosphere of {body} dies"), "The environment can no longer support life.".into()),
    };
    Some(Event { time: t, category, importance, title, detail, system: Some(s), body: Some(b), civ: None })
}

/// Simulate one biosphere from formation to `start` (prehistory).
fn prehistory_one(seed: u64, params: &ScienceParams, mult: LifeMultipliers, mut bio: Biosphere, mut sys: StarSystem, start: f64) -> (Biosphere, Body, Vec<Event>) {
    let b = bio.body as usize;
    let s = bio.system;
    let formed = sys.star.formed_at + 0.2 * SECONDS_PER_GYR;
    let mut events = Vec::new();
    if formed < start {
        let steps = ((start - formed) / STEP_SECONDS) as u64;
        let mut t = start - steps as f64 * STEP_SECONDS;
        bio.stage_since = t;
        for k in 0..steps {
            t += STEP_SECONDS;
            if k % 100 == 0 {
                crate::planet::environment::carbon_cycle(&mut sys.bodies[b], 1.0);
                refresh_climate(&mut sys, b, t);
                let mut hab = assess(&sys, b, t);
                // Observation starts at `start`: intelligence cannot predate it.
                hab.max_stage = hab.max_stage.min(Stage::ComplexEcosystems);
                bio.habitability = hab;
            }
            if bio.habitability.score <= 0.0 && bio.stage == Stage::Sterile {
                continue;
            }
            let flare = sys.star.flare_activity_at(t);
            let mut rng = Rng::stream(seed, domain::PREHISTORY, &[s as u64, b as u64, k]);
            let name = sys.bodies[b].name.clone();
            for e in bio.step(&mut sys.bodies[b], params, mult, flare, t, &mut rng) {
                let keep = matches!(e, LifeEvent::StageReached(Stage::Microbial | Stage::Multicellular | Stage::ComplexEcosystems) | LifeEvent::Oxygenation | LifeEvent::Sterilised | LifeEvent::Collapse);
                if keep {
                    if let Some(ev) = life_event(&e, &name, t, s, b as u32) {
                        events.push(ev);
                    }
                }
            }
        }
    }
    refresh_climate(&mut sys, b, start);
    let mut body = sys.bodies[b].clone();
    body.sea_level = terrain::Terrain::of(&body).sea_level_for(body.hydro.ocean_fraction);
    sys.bodies[b] = body.clone();
    bio.habitability = assess(&sys, b, start);
    refresh_derived_resources(&mut body, bio.vegetated());
    (bio, body, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SECONDS_PER_KYR;

    fn small(scenario: Scenario, seed: u64) -> UniverseSettings {
        UniverseSettings { seed, scenario, system_count: 6, ..Default::default() }
    }

    #[test]
    fn creation_is_deterministic() {
        let a = Universe::new(small(Scenario::GardenWorld, 7));
        let b = Universe::new(small(Scenario::GardenWorld, 7));
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    }

    #[test]
    fn outcome_is_independent_of_frame_chunking() {
        let mut a = Universe::new(small(Scenario::Sol, 3));
        let mut b = a.clone();
        let total = 3.0 * SECONDS_PER_KYR;
        a.advance_by(total);
        let mut done = 0.0;
        let mut i = 0;
        while done < total {
            let dt = [0.37, 13.0, 101.3, 7.7][i % 4] * SECONDS_PER_YEAR;
            let next = (done + dt).min(total);
            b.advance_to(b.start_time + next, None, None);
            done = next;
            i += 1;
        }
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    }

    #[test]
    fn humanity_grows_and_learns() {
        let mut u = Universe::new(small(Scenario::Sol, 11));
        let pop0 = u.civs[0].population;
        let k0: f64 = u.civs[0].knowledge.iter().sum();
        u.advance_by(20.0 * SECONDS_PER_KYR);
        let c = &u.civs[0];
        assert!(c.is_alive());
        assert!(c.population > pop0, "{} -> {}", pop0, c.population);
        assert!(c.knowledge.iter().sum::<f64>() > k0);
        assert!(c.sites.iter().filter(|s| s.active()).count() > 1);
    }

    #[test]
    fn garden_world_has_complex_life_at_start() {
        let u = Universe::new(small(Scenario::GardenWorld, 21));
        assert!(u.biospheres.iter().any(|b| b.system == 0 && b.stage == Stage::ComplexEcosystems));
        assert!(u.civs.is_empty());
    }
}
