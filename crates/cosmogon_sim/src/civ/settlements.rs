//! Representative settlements and transport links.
//!
//! A civilization of billions is not simulated person by person. It owns a fixed set of
//! candidate sites (chosen from the authoritative terrain), of which a growing number are
//! occupied. Population is distributed over occupied sites by a Zipf-like rule whose
//! steepness rises with urbanisation, so villages become towns, cities and megacities.
//! Expansion is limited by *reach*: walking, roads, rail, ships and aircraft.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use super::species::Habitat;
use crate::astro::{Body, ResourceKind};
use crate::names;
use crate::planet::terrain::{dir_from_lat_lon, fibonacci_sphere, lat_lon_from_dir, SurfaceContext};
use crate::rng::Rng;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Camp,
    Village,
    Town,
    City,
    Metropolis,
    Megacity,
}

impl Tier {
    pub fn from_population(p: f64) -> Tier {
        match p {
            p if p < 1_000.0 => Tier::Camp,
            p if p < 10_000.0 => Tier::Village,
            p if p < 100_000.0 => Tier::Town,
            p if p < 1_000_000.0 => Tier::City,
            p if p < 10_000_000.0 => Tier::Metropolis,
            _ => Tier::Megacity,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tier::Camp => "Camp",
            Tier::Village => "Village",
            Tier::Town => "Town",
            Tier::City => "City",
            Tier::Metropolis => "Metropolis",
            Tier::Megacity => "Megacity",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Site {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub score: f64,
    pub coastal: bool,
    pub near_deposit: Option<ResourceKind>,
    pub founded: Option<f64>,
    pub population: f64,
}

impl Site {
    pub fn dir(&self) -> [f64; 3] {
        dir_from_lat_lon(self.lat, self.lon)
    }
    pub fn tier(&self) -> Tier {
        Tier::from_population(self.population)
    }
    pub fn active(&self) -> bool {
        self.founded.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Road,
    Rail,
    Sea,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub a: u16,
    pub b: u16,
    pub kind: LinkKind,
}

fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).dacos()
}

fn slerp_mid(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let m = [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let l = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt().max(1e-12);
    [m[0] / l, m[1] / l, m[2] / l]
}

pub const MAX_SITES: usize = 260;

/// Choose candidate sites from the terrain. Sites are sorted best-first.
pub fn candidate_sites(body: &Body, surface: &SurfaceContext, habitat: Habitat, rng: &mut Rng) -> Vec<Site> {
    let n = 1400;
    let step = 0.6 * (4.0 * std::f64::consts::PI / n as f64).sqrt();
    let mut out: Vec<Site> = Vec::new();
    for d in fibonacci_sphere(n) {
        let s = surface.sample(d);
        let comfort = (-((s.temperature - 290.0) / 25.0).powi(2)).dexp();
        let neighbours_wet = [[step, 0.0, 0.0], [-step, 0.0, 0.0], [0.0, step, 0.0], [0.0, -step, 0.0], [0.0, 0.0, step]]
            .iter()
            .any(|o| {
                let q = [d[0] + o[0], d[1] + o[1], d[2] + o[2]];
                let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
                !surface.sample([q[0] / l, q[1] / l, q[2] / l]).biome.is_land()
            });
        let (lat, lon) = lat_lon_from_dir(d);
        let deposit = body
            .deposits
            .iter()
            .filter(|dep| angle(d, dir_from_lat_lon(dep.lat, dep.lon)) < step * 1.5)
            .max_by(|a, b| a.richness.total_cmp(&b.richness))
            .map(|dep| dep.kind);
        let score = match habitat {
            Habitat::Land => {
                if !s.biome.is_land() {
                    continue;
                }
                let fert = s.biome.fertility();
                if fert < 0.03 && deposit.is_none() {
                    continue;
                }
                0.55 * fert + 0.25 * f64::from(neighbours_wet) + 0.2 * comfort + if deposit.is_some() { 0.12 } else { 0.0 }
            }
            Habitat::Water => {
                if s.biome.is_land() || s.height < -0.25 {
                    continue;
                }
                0.5 * comfort + 0.3 * f64::from(neighbours_wet) + 0.2 * (1.0 + s.height / 0.25)
            }
        };
        if score > 0.08 {
            out.push(Site { name: String::new(), lat, lon, score: score * rng.range(0.85, 1.15), coastal: neighbours_wet, near_deposit: deposit, founded: None, population: 0.0 });
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out.truncate(MAX_SITES);
    for s in &mut out {
        s.name = names::word(rng);
    }
    out
}

/// Precomputed neighbourhood graph between candidate sites: for each site, nearby sites
/// with their arc distance and whether the straight path between them stays in the species'
/// native medium (land for land-dwellers). Derived from terrain, so never saved.
#[derive(Clone, Debug, Default)]
pub struct Adjacency {
    pub near: Vec<Vec<(u16, f32, bool)>>,
}

impl PartialEq for Adjacency {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Adjacency {
    pub const MAX_ARC: f64 = 0.9;

    pub fn is_empty(&self) -> bool {
        self.near.is_empty()
    }

    pub fn build(sites: &[Site], surface: &SurfaceContext, habitat: Habitat) -> Self {
        let dirs: Vec<[f64; 3]> = sites.iter().map(Site::dir).collect();
        let near = (0..sites.len())
            .map(|i| {
                let mut v: Vec<(u16, f32, bool)> = (0..sites.len())
                    .filter(|&j| j != i)
                    .filter_map(|j| {
                        let d = angle(dirs[i], dirs[j]);
                        (d < Self::MAX_ARC).then(|| {
                            let native = d < 0.02 || surface.sample(slerp_mid(dirs[i], dirs[j])).biome.is_land() == (habitat == Habitat::Land);
                            (j as u16, d as f32, native)
                        })
                    })
                    .collect();
                v.sort_by(|a, b| a.1.total_cmp(&b.1));
                v.truncate(24);
                v
            })
            .collect();
        Self { near }
    }
}

pub struct SpreadParams {
    /// Max expansion distance per update (radians of arc).
    pub reach: f64,
    pub seafaring: bool,
    /// Rafts: water gaps up to ~0.2 rad can be crossed.
    pub short_crossings: bool,
    pub roads: bool,
    pub rail: bool,
    pub urbanisation: f64,
    /// Largest dispersed (non-urban) community: bands before farming, villages after.
    pub rural_cap: f64,
}

/// Re-distribute population and expand into new sites. Returns newly reached tiers.
pub fn update(sites: &mut [Site], links: &mut Vec<Link>, adj: &Adjacency, population: f64, p: &SpreadParams, t: f64) -> Vec<(usize, Tier)> {
    if sites.is_empty() {
        return Vec::new();
    }
    if !sites.iter().any(|s| s.active()) {
        sites[0].founded = Some(t);
    }
    // Expansion: occupy more sites as the population grows, but only within reach, and
    // across water (for land-dwellers) only with seafaring.
    let target_active = ((population.sqrt() / 5.0) as usize).clamp(1, sites.len());
    let mut active = sites.iter().filter(|s| s.active()).count();
    let mut added = 0;
    while active < target_active && added < 8 {
        let mut best: Option<usize> = None;
        for (i, s) in sites.iter().enumerate() {
            if !s.active() {
                continue;
            }
            for &(j, d, native) in &adj.near[i] {
                let j = j as usize;
                if (d as f64) <= p.reach && (native || p.seafaring || (p.short_crossings && d < 0.2)) && !sites[j].active() && best.is_none_or(|b| sites[j].score > sites[b].score) {
                    best = Some(j);
                }
            }
        }
        match best {
            Some(j) => {
                sites[j].founded = Some(t);
                active += 1;
                added += 1;
            }
            None => break,
        }
    }

    // Urban population follows a Zipf-like rule by founding order (old settlements are the
    // big ones); the rural remainder is spread evenly, capped per community.
    let mut order: Vec<usize> = (0..sites.len()).filter(|&i| sites[i].active()).collect();
    order.sort_by(|&a, &b| sites[a].founded.unwrap().total_cmp(&sites[b].founded.unwrap()).then(sites[b].score.total_cmp(&sites[a].score)));
    let urban = population * p.urbanisation.clamp(0.0, 1.0);
    let rural_each = ((population - urban) / order.len().max(1) as f64).min(p.rural_cap);
    let alpha = 0.55 + 0.5 * p.urbanisation.clamp(0.0, 1.0);
    let weights: Vec<f64> = order.iter().enumerate().map(|(rank, &i)| sites[i].score * ((rank + 1) as f64).dpowf(-alpha)).collect();
    let total: f64 = weights.iter().sum::<f64>().max(1e-12);
    let mut promotions = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let before = sites[i].tier();
        sites[i].population = urban * weights[k] / total + rural_each;
        let after = sites[i].tier();
        if after > before {
            promotions.push((i, after));
        }
    }

    // Transport links between nearby significant settlements.
    links.clear();
    let significant: Vec<bool> = sites.iter().map(|s| s.active() && s.tier() >= Tier::Town).collect();
    let linked = |links: &Vec<Link>, a: usize, b: usize| links.iter().any(|l| (l.a as usize, l.b as usize) == (b, a) || (l.a as usize, l.b as usize) == (a, b));
    for i in order.iter().copied().filter(|&i| significant[i]).take(120) {
        let mut land_links = 0;
        let mut sea_links = 0;
        for &(j, d, native) in &adj.near[i] {
            let j = j as usize;
            if !significant[j] || linked(links, i, j) {
                continue;
            }
            if native && land_links < 2 && d < 0.35 && (p.roads || p.rail) {
                links.push(Link { a: i as u16, b: j as u16, kind: if p.rail { LinkKind::Rail } else { LinkKind::Road } });
                land_links += 1;
            } else if !native && sea_links < 1 && p.seafaring && sites[i].coastal && sites[j].coastal {
                links.push(Link { a: i as u16, b: j as u16, kind: LinkKind::Sea });
                sea_links += 1;
            }
        }
    }
    promotions
}
