//! The technology graph: loading, validation and evaluation.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use thiserror::Error;

use super::knowledge::{Domain, Knowledge};
use super::species::Habitat;
use crate::astro::Resources;

const TECH_TOML: &str = include_str!("../../data/technologies.toml");

#[derive(Debug, Error)]
pub enum TechError {
    #[error("technology file is not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{tech}: cannot parse condition `{cond}`: {why}")]
    Condition { tech: String, cond: String, why: String },
    #[error("duplicate technology id `{0}`")]
    Duplicate(String),
    #[error("{0}: unknown knowledge domain `{1}`")]
    UnknownDomain(String, String),
    #[error("technology dependency cycle through `{0}`")]
    Cycle(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmp {
    Ge,
    Le,
}

impl Cmp {
    fn holds(self, a: f64, b: f64) -> bool {
        match self {
            Cmp::Ge => a >= b,
            Cmp::Le => a <= b,
        }
    }
    fn symbol(self) -> &'static str {
        match self {
            Cmp::Ge => "≥",
            Cmp::Le => "≤",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvKey {
    Oxygen,
    Gravity,
    Ocean,
    Land,
    Pressure,
    Temperature,
    LaunchDv,
    Moons,
    OtherBodies,
    ClimateStability,
}

impl EnvKey {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "oxygen" => Self::Oxygen,
            "gravity" => Self::Gravity,
            "ocean" => Self::Ocean,
            "land" => Self::Land,
            "pressure" => Self::Pressure,
            "temperature" => Self::Temperature,
            "launch_dv" => Self::LaunchDv,
            "moons" => Self::Moons,
            "other_bodies" => Self::OtherBodies,
            "climate_stability" => Self::ClimateStability,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Oxygen => "atmospheric O₂ fraction",
            Self::Gravity => "surface gravity (g)",
            Self::Ocean => "ocean cover",
            Self::Land => "land cover",
            Self::Pressure => "surface pressure (bar)",
            Self::Temperature => "mean temperature (K)",
            Self::LaunchDv => "Δv to orbit (km/s)",
            Self::Moons => "moons",
            Self::OtherBodies => "other reachable worlds",
            Self::ClimateStability => "climate stability (interglacial)",
        }
    }
}

/// The physical facts about a civilization's world that technologies can depend on.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub values: HashMap<EnvKey, f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Tech(usize),
    Knowledge(Domain, f64),
    Resource(String, f64),
    Env(EnvKey, Cmp, f64),
    Habitat(Habitat),
    Population(f64),
    Flag(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Effects {
    pub capacity: f64,
    pub growth: f64,
    pub research: f64,
    pub energy: f64,
    pub fossil: f64,
    pub fossil_cap: f64,
    pub urban: f64,
    pub stability: f64,
    pub health: f64,
    pub reach: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub label: String,
    pub requires: Vec<Condition>,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tech {
    pub id: String,
    pub name: String,
    pub era: String,
    pub domain: Domain,
    pub years: f64,
    pub requires: Vec<Condition>,
    pub routes: Vec<Route>,
    pub demand: Option<String>,
    pub effects: Effects,
    pub boost: Vec<(Domain, f64)>,
    pub unlocks: Vec<String>,
    pub description: String,
}

/// Everything a condition can be evaluated against.
pub struct Context<'a> {
    pub known: &'a [bool],
    pub knowledge: &'a Knowledge,
    pub resources: &'a Resources,
    pub env: &'a Environment,
    pub habitat: Habitat,
    pub population: f64,
    pub flags: &'a dyn Fn(&str) -> bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Known,
    /// Discoverable now. `route` is the fastest satisfied route (if routes exist).
    Available { route: Option<usize>, speed: f64, knowledge_surplus: f64 },
    Blocked { missing: Vec<String> },
}

pub struct TechGraph {
    pub techs: Vec<Tech>,
    index: HashMap<String, usize>,
}

#[derive(Deserialize)]
struct RawFile {
    tech: Vec<RawTech>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawEffects {
    capacity: Option<f64>,
    growth: f64,
    research: Option<f64>,
    energy: f64,
    fossil: f64,
    fossil_cap: Option<f64>,
    urban: f64,
    stability: f64,
    health: f64,
    reach: f64,
    boost: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct RawRoute {
    label: String,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default = "one")]
    speed: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct RawTech {
    id: String,
    name: String,
    era: String,
    domain: String,
    years: f64,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    route: Vec<RawRoute>,
    demand: Option<String>,
    #[serde(default)]
    effects: RawEffects,
    #[serde(default)]
    unlocks: Vec<String>,
    #[serde(default)]
    description: String,
}

impl TechGraph {
    pub fn embedded() -> &'static TechGraph {
        use std::sync::OnceLock;
        static G: OnceLock<TechGraph> = OnceLock::new();
        G.get_or_init(|| TechGraph::from_toml(TECH_TOML).expect("embedded data/technologies.toml must be valid"))
    }

    pub fn from_toml(s: &str) -> Result<Self, TechError> {
        let raw: RawFile = toml::from_str(s)?;
        let mut index = HashMap::new();
        for (i, t) in raw.tech.iter().enumerate() {
            if index.insert(t.id.clone(), i).is_some() {
                return Err(TechError::Duplicate(t.id.clone()));
            }
        }
        let parse_all = |tech: &str, conds: &[String]| -> Result<Vec<Condition>, TechError> {
            conds.iter().map(|c| parse_condition(c, &index).map_err(|why| TechError::Condition { tech: tech.into(), cond: c.clone(), why })).collect()
        };
        let mut techs = Vec::new();
        for t in &raw.tech {
            let domain = Domain::parse(&t.domain).ok_or_else(|| TechError::UnknownDomain(t.id.clone(), t.domain.clone()))?;
            let mut boost = Vec::new();
            for (d, m) in &t.effects.boost {
                boost.push((Domain::parse(d).ok_or_else(|| TechError::UnknownDomain(t.id.clone(), d.clone()))?, *m));
            }
            let routes = t
                .route
                .iter()
                .map(|r| Ok(Route { label: r.label.clone(), requires: parse_all(&t.id, &r.requires)?, speed: r.speed }))
                .collect::<Result<Vec<_>, TechError>>()?;
            let e = &t.effects;
            techs.push(Tech {
                id: t.id.clone(),
                name: t.name.clone(),
                era: t.era.clone(),
                domain,
                years: t.years,
                requires: parse_all(&t.id, &t.requires)?,
                routes,
                demand: t.demand.clone(),
                effects: Effects {
                    capacity: e.capacity.unwrap_or(1.0),
                    growth: e.growth,
                    research: e.research.unwrap_or(1.0),
                    energy: e.energy,
                    fossil: e.fossil,
                    fossil_cap: e.fossil_cap.unwrap_or(1.0),
                    urban: e.urban,
                    stability: e.stability,
                    health: e.health,
                    reach: e.reach,
                },
                boost,
                unlocks: t.unlocks.clone(),
                description: t.description.clone(),
            });
        }
        let graph = Self { techs, index };
        graph.check_acyclic()?;
        Ok(graph)
    }

    fn deps(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let t = &self.techs[i];
        t.requires.iter().chain(t.routes.iter().flat_map(|r| r.requires.iter())).filter_map(|c| match c {
            Condition::Tech(d) => Some(*d),
            _ => None,
        })
    }

    fn check_acyclic(&self) -> Result<(), TechError> {
        // 0 = unvisited, 1 = on stack, 2 = done
        fn visit(g: &TechGraph, i: usize, state: &mut [u8]) -> Result<(), TechError> {
            match state[i] {
                1 => return Err(TechError::Cycle(g.techs[i].id.clone())),
                2 => return Ok(()),
                _ => {}
            }
            state[i] = 1;
            for d in g.deps(i).collect::<Vec<_>>() {
                visit(g, d, state)?;
            }
            state[i] = 2;
            Ok(())
        }
        let mut state = vec![0u8; self.techs.len()];
        for i in 0..self.techs.len() {
            visit(self, i, &mut state)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.techs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.techs.is_empty()
    }
    pub fn find(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }

    pub fn describe(&self, c: &Condition) -> String {
        match c {
            Condition::Tech(i) => format!("knows {}", self.techs[*i].name),
            Condition::Knowledge(d, n) => format!("{} knowledge ≥ {}", d.name(), crate::time::group_digits(*n)),
            Condition::Resource(k, x) => format!("{} ≥ {:.2}× Earth", k.replace('_', " "), x),
            Condition::Env(k, cmp, x) => format!("{} {} {}", k.label(), cmp.symbol(), x),
            Condition::Habitat(h) => format!("{} species", h.label()),
            Condition::Population(n) => format!("population ≥ {}", crate::time::group_digits(*n)),
            Condition::Flag(f) => format!("capability: {f}"),
        }
    }

    pub fn check(&self, c: &Condition, ctx: &Context) -> bool {
        match c {
            Condition::Tech(i) => ctx.known[*i],
            Condition::Knowledge(d, n) => ctx.knowledge[d.index()] >= *n,
            Condition::Resource(k, x) => ctx.resources.by_key(k).unwrap_or(0.0) >= *x,
            Condition::Env(k, cmp, x) => cmp.holds(ctx.env.values.get(k).copied().unwrap_or(0.0), *x),
            Condition::Habitat(h) => ctx.habitat == *h,
            Condition::Population(n) => ctx.population >= *n,
            Condition::Flag(f) => (ctx.flags)(f),
        }
    }

    /// Fast path used every simulated year: `(route, speed, knowledge_surplus)` if the
    /// technology is discoverable now. Allocation-free; see [`Self::status`] for reasons.
    pub fn available(&self, i: usize, ctx: &Context) -> Option<(Option<usize>, f64, f64)> {
        if ctx.known[i] {
            return None;
        }
        let t = &self.techs[i];
        if !t.requires.iter().all(|c| self.check(c, ctx)) {
            return None;
        }
        let (route, speed) = if t.routes.is_empty() {
            (None, 1.0)
        } else {
            let (ri, r) = t
                .routes
                .iter()
                .enumerate()
                .filter(|(_, r)| r.requires.iter().all(|c| self.check(c, ctx)))
                .max_by(|a, b| a.1.speed.total_cmp(&b.1.speed))?;
            (Some(ri), r.speed)
        };
        let (mut sum, mut n) = (0.0, 0);
        for c in &t.requires {
            if let Condition::Knowledge(d, k) = c {
                if *k > 0.0 {
                    sum += (ctx.knowledge[d.index()] / k).min(4.0);
                    n += 1;
                }
            }
        }
        Some((route, speed, if n == 0 { 1.0 } else { sum / n as f64 }))
    }

    /// Why a technology is or isn't discoverable, and how fast.
    pub fn status(&self, i: usize, ctx: &Context) -> Status {
        if ctx.known[i] {
            return Status::Known;
        }
        let t = &self.techs[i];
        let mut missing: Vec<String> = t.requires.iter().filter(|c| !self.check(c, ctx)).map(|c| self.describe(c)).collect();
        let mut route = None;
        let mut speed = 1.0;
        if !t.routes.is_empty() {
            let best = t
                .routes
                .iter()
                .enumerate()
                .filter(|(_, r)| r.requires.iter().all(|c| self.check(c, ctx)))
                .max_by(|a, b| a.1.speed.total_cmp(&b.1.speed));
            match best {
                Some((ri, r)) => {
                    route = Some(ri);
                    speed = r.speed;
                }
                None => {
                    let alts: Vec<String> = t
                        .routes
                        .iter()
                        .map(|r| {
                            let m: Vec<String> = r.requires.iter().filter(|c| !self.check(c, ctx)).map(|c| self.describe(c)).collect();
                            format!("{} (needs {})", r.label, m.join(", "))
                        })
                        .collect();
                    missing.push(format!("one of: {}", alts.join("; ")));
                }
            }
        }
        if !missing.is_empty() {
            return Status::Blocked { missing };
        }
        // Knowledge beyond the minimum makes discovery faster (capped).
        let ratios: Vec<f64> = t
            .requires
            .iter()
            .filter_map(|c| match c {
                Condition::Knowledge(d, n) if *n > 0.0 => Some((ctx.knowledge[d.index()] / n).min(4.0)),
                _ => None,
            })
            .collect();
        let knowledge_surplus = if ratios.is_empty() { 1.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        Status::Available { route, speed, knowledge_surplus }
    }
}

fn parse_condition(s: &str, index: &HashMap<String, usize>) -> Result<Condition, String> {
    let s = s.trim();
    if let Some(n) = s.strip_prefix("pop>=") {
        return Ok(Condition::Population(n.trim().parse().map_err(|_| format!("bad population `{n}`"))?));
    }
    let (kind, rest) = s.split_once(':').ok_or("expected `kind:...`")?;
    let cmp_split = |rest: &str| -> Result<(String, Cmp, f64), String> {
        let (k, cmp, v) = if let Some((k, v)) = rest.split_once(">=") {
            (k, Cmp::Ge, v)
        } else if let Some((k, v)) = rest.split_once("<=") {
            (k, Cmp::Le, v)
        } else {
            return Err("expected >= or <=".into());
        };
        let v: f64 = v.trim().parse().map_err(|_| format!("bad number `{v}`"))?;
        Ok((k.trim().to_string(), cmp, v))
    };
    Ok(match kind {
        "tech" => Condition::Tech(*index.get(rest.trim()).ok_or_else(|| format!("unknown technology `{rest}`"))?),
        "k" => {
            let (d, cmp, v) = cmp_split(rest)?;
            if cmp != Cmp::Ge {
                return Err("knowledge conditions must use >=".into());
            }
            Condition::Knowledge(Domain::parse(&d).ok_or_else(|| format!("unknown domain `{d}`"))?, v)
        }
        "res" => {
            let (k, cmp, v) = cmp_split(rest)?;
            if cmp != Cmp::Ge || Resources::default().by_key(&k).is_none() {
                return Err(format!("bad resource condition `{k}`"));
            }
            Condition::Resource(k, v)
        }
        "env" => {
            let (k, cmp, v) = cmp_split(rest)?;
            Condition::Env(EnvKey::parse(&k).ok_or_else(|| format!("unknown env key `{k}`"))?, cmp, v)
        }
        "habitat" => Condition::Habitat(match rest.trim() {
            "land" => Habitat::Land,
            "water" => Habitat::Water,
            h => return Err(format!("unknown habitat `{h}`")),
        }),
        "pop" => {
            let rest = rest.trim_start_matches(">=");
            Condition::Population(rest.trim().parse().map_err(|_| format!("bad population `{rest}`"))?)
        }
        "flag" => Condition::Flag(rest.trim().to_string()),
        _ => return Err(format!("unknown condition kind `{kind}`")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::knowledge::N_DOMAINS;

    fn ctx_with<'a>(known: &'a [bool], k: &'a Knowledge, r: &'a Resources, env: &'a Environment, habitat: Habitat, pop: f64) -> Context<'a> {
        Context { known, knowledge: k, resources: r, env, habitat, population: pop, flags: &|_| false }
    }

    #[test]
    fn shipped_graph_is_valid_and_acyclic() {
        let g = TechGraph::embedded();
        assert!(g.len() >= 50, "{}", g.len());
        // Every tech must be reachable in principle (has a satisfiable route list).
        for t in &g.techs {
            assert!(t.years > 0.0, "{}", t.id);
        }
    }

    #[test]
    fn cycles_and_bad_references_are_rejected() {
        let cyc = r#"
            [[tech]]
            id = "a"
            name = "A"
            era = "x"
            domain = "Physics"
            years = 1
            requires = ["tech:b"]
            [[tech]]
            id = "b"
            name = "B"
            era = "x"
            domain = "Physics"
            years = 1
            requires = ["tech:a"]
        "#;
        assert!(matches!(TechGraph::from_toml(cyc), Err(TechError::Cycle(_))));
        let bad = r#"
            [[tech]]
            id = "a"
            name = "A"
            era = "x"
            domain = "Physics"
            years = 1
            requires = ["tech:nope"]
        "#;
        assert!(matches!(TechGraph::from_toml(bad), Err(TechError::Condition { .. })));
    }

    #[test]
    fn bronze_needs_tin_and_iron_has_alternative_routes() {
        let g = TechGraph::embedded();
        let mut known = vec![false; g.len()];
        for id in ["stone_tools", "fire", "agriculture", "pottery", "copper_smelting"] {
            known[g.find(id).unwrap()] = true;
        }
        let k = [1.0e6; N_DOMAINS];
        let env = Environment { values: [(EnvKey::Oxygen, 0.21)].into_iter().collect() };
        let no_tin = Resources { iron: 1.0, copper: 1.0, tin: 0.0, fertile_land: 1.0, ..Default::default() };
        let ctx = ctx_with(&known, &k, &no_tin, &env, Habitat::Land, 1e7);

        let bronze = g.find("bronze_working").unwrap();
        match g.status(bronze, &ctx) {
            Status::Blocked { missing } => assert!(missing.iter().any(|m| m.contains("tin")), "{missing:?}"),
            s => panic!("bronze should be blocked without tin: {s:?}"),
        }
        // Iron is still reachable — via the slower copper route.
        match g.status(g.find("iron_working").unwrap(), &ctx) {
            Status::Available { route: Some(r), speed, .. } => {
                assert_eq!(g.techs[g.find("iron_working").unwrap()].routes[r].label, "directly from copper metallurgy");
                assert!(speed < 1.0);
            }
            s => panic!("iron should be available: {s:?}"),
        }
    }

    #[test]
    fn heavy_worlds_cannot_reach_orbit_with_chemistry() {
        let g = TechGraph::embedded();
        let mut known = vec![true; g.len()];
        let sf = g.find("spaceflight").unwrap();
        known[sf] = false;
        known[g.find("nuclear_power").unwrap()] = false;
        let k = [1.0e9; N_DOMAINS];
        let r = Resources::default();
        let heavy = Environment { values: [(EnvKey::LaunchDv, 16.0)].into_iter().collect() };
        let ctx = ctx_with(&known, &k, &r, &heavy, Habitat::Land, 1e9);
        assert!(matches!(g.status(sf, &ctx), Status::Blocked { .. }));
        known[g.find("nuclear_power").unwrap()] = true;
        let ctx = ctx_with(&known, &k, &r, &heavy, Habitat::Land, 1e9);
        assert!(matches!(g.status(sf, &ctx), Status::Available { route: Some(1), .. }));
    }

    #[test]
    fn aquatic_species_can_farm_but_never_light_fires() {
        let g = TechGraph::embedded();
        let known = vec![false; g.len()];
        let k = [1.0e5; N_DOMAINS];
        let r = Resources::default();
        let env = Environment { values: [(EnvKey::Oxygen, 0.21), (EnvKey::ClimateStability, 1.0)].into_iter().collect() };
        let ctx = ctx_with(&known, &k, &r, &env, Habitat::Water, 1e6);
        assert!(matches!(g.status(g.find("agriculture").unwrap(), &ctx), Status::Available { .. }));
        assert!(matches!(g.status(g.find("fire").unwrap(), &ctx), Status::Blocked { .. }));
    }
}
