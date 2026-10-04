//! Civilizations beyond their homeworld: megastructures, terraforming and the stars.
//!
//! * **Dyson swarm** (`dyson` flag): orbiting collectors capture a growing share of the
//!   star's output — logistic growth over a few centuries towards 90 %. The captured power
//!   counts towards the civilization's energy use, so its Kardashev rating climbs towards
//!   K ≈ 2. The swarm orbits well inside the home world's orbit but out of its line of
//!   sight to the star, so the home climate is unaffected; from afar the star dims.
//! * **Terraforming** (`terraform` flag): colony worlds get thicker nitrogen–oxygen air and
//!   imported water over centuries; the climate model then decides whether seas form.
//! * **Generation ships** (`starships` flag): once a probe has scouted another star and
//!   found a world fit to live on, settlers follow at a fraction of the probe's speed. On
//!   arrival they found a new branch of the civilization, which inherits its parent's
//!   knowledge and from then on lives — and diverges — on its own.
//!
//! Deterministic: everything is driven by the simulation clock and seeded streams.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::{BodyKind, LIGHT_YEAR};
use cosmogon_core::constants::SOLAR_LUMINOSITY;
use crate::civ::CivStatus;
use crate::habitability::assess;
use crate::history::{Category, Event};
use crate::planet::terrain;
use crate::time::SECONDS_PER_YEAR;
use crate::universe::{refresh_climate, Universe};
use crate::BodyRef;

/// Fraction of the probe speed a crewed generation ship manages.
pub const SHIP_SPEED_FRACTION: f64 = 0.3;
/// Years between generation-ship launches from one civilization.
pub const SHIP_INTERVAL_YEARS: f64 = 250.0;
/// Minimum habitability score for a world worth crossing light-years for.
pub const SETTLE_SCORE: f64 = 0.35;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Starship {
    pub civ: u32,
    pub name: String,
    pub from: u32,
    pub to: BodyRef,
    pub launched: f64,
    /// m/s
    pub speed: f64,
    pub settlers: f64,
    pub arrived: bool,
}

impl Starship {
    pub fn progress(&self, u: &Universe) -> f64 {
        let d = (u.systems[self.to.system as usize].position - u.systems[self.from as usize].position).length();
        ((u.time - self.launched) * self.speed / d.max(1.0)).clamp(0.0, 1.0)
    }
}

impl Universe {
    /// Megastructures and terraforming for civilization `ci`, stepped `dt` years at `t`.
    pub(crate) fn step_megastructures(&mut self, ci: usize, t: f64, dt: f64) {
        let c = &self.civs[ci];
        if !c.is_alive() {
            return;
        }
        let (s, home) = (c.system as usize, c.body);
        // Dyson swarm.
        if c.flags.contains("dyson") {
            let before = c.dyson;
            // Logistic growth (exact over the step): ~40-year e-folding while collectors
            // build more collectors, levelling off at 90 %.
            let c0 = before.max(0.002);
            let cov = 0.9 / (1.0 + (0.9 / c0 - 1.0) * (-dt / 40.0).dexp());
            let lum = self.systems[s].star.luminosity(t) * SOLAR_LUMINOSITY;
            let civ = &mut self.civs[ci];
            civ.dyson = cov;
            civ.dyson_power_w = cov * lum;
            let star = self.systems[s].star.name.clone();
            for (mark, title, detail) in [
                (0.0, format!("Construction of a Dyson swarm begins around {star}"), "The first solar collectors are placed in orbit, mined from asteroids and moons."),
                (0.1, format!("The Dyson swarm captures a tenth of {star}'s light"), "Seen from other stars, the sun has visibly dimmed."),
                (0.5, format!("Half of {star}'s output is harvested"), "The civilization now commands more power than its world receives from its star."),
            ] {
                if before <= mark && cov > mark {
                    let name = civ.name.clone();
                    self.history.push(Event { time: t, category: Category::Technology, importance: 5, title, detail: format!("{detail} ({name})"), system: Some(s as u32), body: Some(home), civ: Some(ci as u32) });
                }
            }
        }
        // Terraforming the colonies.
        if self.civs[ci].flags.contains("terraform") {
            let colonies: Vec<u32> = self.civs[ci].colonies.iter().map(|c| c.body).collect();
            for b in colonies {
                let bi = b as usize;
                let Some(body) = self.systems[s].bodies.get(bi) else { continue };
                if !body.exists() || matches!(body.kind, BodyKind::GasGiant | BodyKind::IceGiant) || crate::astro::G * body.mass / (body.radius * body.radius) < 1.5 {
                    continue;
                }
                let was = assess(&self.systems[s], bi, t).score;
                let k = 1.0 - (-dt / 400.0).dexp();
                let body = &mut self.systems[s].bodies[bi];
                let first = body.atmosphere.o2 < 0.01 && body.atmosphere.pressure_bar < 0.9;
                let a = &mut body.atmosphere;
                a.pressure_bar += (1.0 - a.pressure_bar) * k;
                a.n2 += (0.78 - a.n2) * k;
                a.o2 += (0.21 - a.o2) * k;
                a.co2 += (0.001 - a.co2) * k;
                a.ch4 -= a.ch4 * k;
                a.h2he -= a.h2he * k;
                let w = body.hydro.water_inventory;
                body.hydro.water_inventory = w + (0.25f64.max(w) - w) * k;
                body.sea_level = terrain::Terrain::of(body).sea_level_for(body.hydro.ocean_fraction);
                refresh_climate(&mut self.systems[s], bi, t);
                let now = assess(&self.systems[s], bi, t).score;
                let (name, civ) = (self.systems[s].bodies[bi].name.clone(), self.civs[ci].name.clone());
                if first {
                    self.history.push(Event { time: t, category: Category::Technology, importance: 5, title: format!("Terraforming of {name} begins"), detail: format!("{civ} start thickening its air and importing water from icy bodies."), system: Some(s as u32), body: Some(b), civ: Some(ci as u32) });
                }
                if was < 0.5 && now >= 0.5 {
                    self.history.push(Event { time: t, category: Category::Life, importance: 5, title: format!("{name} is habitable"), detail: format!("After centuries of work by {civ}, people walk {name} without pressure suits."), system: Some(s as u32), body: Some(b), civ: Some(ci as u32) });
                }
            }
        }
    }

    /// Launch generation ships to scouted worlds, and found new branches on arrival.
    pub(crate) fn step_starships(&mut self, t: f64) {
        for ci in 0..self.civs.len() {
            let c = &self.civs[ci];
            if !c.is_alive() || !c.flags.contains("starships") || (t - c.starship_timer) < SHIP_INTERVAL_YEARS * SECONDS_PER_YEAR {
                continue;
            }
            self.civs[ci].starship_timer = t;
            let c = &self.civs[ci];
            let home = self.systems[c.system as usize].position;
            // Stars scouted by this civilization's probes, nearest first.
            let mut scouted: Vec<u32> = self.probes.iter().filter(|p| p.civ == ci as u32 && p.arrived).map(|p| p.to).collect();
            scouted.sort_by(|a, b| (self.systems[*a as usize].position - home).length().total_cmp(&(self.systems[*b as usize].position - home).length()));
            let occupied = |u: &Universe, sys: u32| u.civs.iter().any(|x| x.is_alive() && x.system == sys) || u.starships.iter().any(|s| !s.arrived && s.to.system == sys);
            let mut target = None;
            for sid in scouted {
                if occupied(self, sid) {
                    continue;
                }
                let sys = &self.systems[sid as usize];
                let best = (0..sys.bodies.len()).filter(|&i| sys.bodies[i].exists() && !sys.bodies[i].kind.is_stellar()).map(|i| (i, assess(sys, i, t).score)).filter(|(_, sc)| *sc >= SETTLE_SCORE).max_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((i, _)) = best {
                    target = Some(BodyRef { system: sid, body: i as u32 });
                    break;
                }
            }
            let Some(to) = target else { continue };
            let n = self.starships.iter().filter(|s| s.civ == ci as u32).count() + 1;
            let speed = self.params.civilization.probe_speed_c * SHIP_SPEED_FRACTION * crate::astro::SPEED_OF_LIGHT;
            let name = format!("{} Ark {n}", self.civs[ci].species.name);
            let dist = (self.systems[to.system as usize].position - home).length();
            self.starships.push(Starship { civ: ci as u32, name: name.clone(), from: self.civs[ci].system, to, launched: t, speed, settlers: 10_000.0, arrived: false });
            let (world, star) = (self.body(to).name.clone(), self.systems[to.system as usize].star.name.clone());
            self.history.push(Event {
                time: t,
                category: Category::Space,
                importance: 5,
                title: format!("Generation ship {name} departs for {star}"),
                detail: format!("Ten thousand settlers leave for {world}, {:.1} light-years away; their great-great-grandchildren will arrive in about {:.0} years.", dist / LIGHT_YEAR, dist / speed / SECONDS_PER_YEAR),
                system: Some(self.civs[ci].system),
                body: Some(self.civs[ci].body),
                civ: Some(ci as u32),
            });
        }
        // Arrivals.
        for i in 0..self.starships.len() {
            let sh = &self.starships[i];
            if sh.arrived {
                continue;
            }
            let dist = (self.systems[sh.to.system as usize].position - self.systems[sh.from as usize].position).length();
            if (t - sh.launched) * sh.speed < dist {
                continue;
            }
            self.starships[i].arrived = true;
            let sh = self.starships[i].clone();
            let alive_here = self.civs.iter().any(|x| x.is_alive() && x.system == sh.to.system && x.body == sh.to.body);
            if !self.body(sh.to).exists() || alive_here {
                continue;
            }
            self.found_branch(sh.civ as usize, sh.to, sh.settlers, &sh.name, t);
        }
    }

    /// A new civilization on `to`, carrying the knowledge of civilization `parent`.
    fn found_branch(&mut self, parent: usize, to: BodyRef, settlers: f64, ship: &str, t: f64) {
        let species = self.civs[parent].species.clone();
        let mut civ = self.make_civ(to, species, t);
        let p = &self.civs[parent];
        let star = self.systems[to.system as usize].star.name.clone();
        civ.name = format!("{} of {star}", p.species.name);
        civ.parent = Some(parent as u32);
        civ.population = settlers;
        civ.knowledge = p.knowledge;
        civ.discoveries = p.discoveries.clone();
        civ.flags = p.flags.clone();
        civ.capacity_mult = p.capacity_mult;
        civ.growth_bonus = p.growth_bonus;
        civ.research_mult = p.research_mult;
        civ.energy_per_capita = p.energy_per_capita;
        civ.urban_target = p.urban_target;
        civ.base_stability = p.base_stability;
        civ.reach = p.reach;
        civ.focus_boost = p.focus_boost;
        // They arrive knowing their way home: radio from day one, and the parent's signals heard.
        civ.radio_since = Some(t);
        civ.detected.push(parent as u32);
        civ.next_k = self.scheduler.tasks.get(crate::universe::TASK_CIV).map(|task| task.steps).unwrap_or(0);
        civ.status = CivStatus::Thriving;
        let id = civ.id;
        let world = self.body(to).name.clone();
        let parent_name = self.civs[parent].name.clone();
        if let Some(bio) = self.biospheres.iter_mut().find(|b| b.system == to.system && b.body == to.body) {
            bio.civilization_present = true;
        }
        self.civs.push(civ);
        self.history.push(Event {
            time: t,
            category: Category::Civilization,
            importance: 5,
            title: format!("{ship} reaches {world}: a new branch of {parent_name}"),
            detail: format!("The settlers found a colony around {star}. From now on they live — and change — on their own."),
            system: Some(to.system),
            body: Some(to.body),
            civ: Some(id),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scenario, UniverseSettings};

    fn sol() -> Universe {
        Universe::new(UniverseSettings { seed: 7, scenario: Scenario::SolarSystemLab, ..Default::default() })
    }

    fn humanity(u: &Universe) -> usize {
        u.civs.iter().position(|c| c.is_alive()).expect("present-day Earth has humanity")
    }

    #[test]
    fn a_dyson_swarm_lifts_a_civilization_towards_kardashev_two() {
        let mut u = sol();
        let ci = humanity(&u);
        let k0 = u.civs[ci].kardashev();
        u.civs[ci].flags.insert("dyson".into());
        let t = u.time;
        for y in 0..600 {
            u.step_megastructures(ci, t + y as f64 * SECONDS_PER_YEAR, 1.0);
        }
        let c = &u.civs[ci];
        assert!(c.dyson > 0.85 && c.dyson <= 0.9, "coverage {}", c.dyson);
        assert!(c.kardashev() > 1.9 && k0 < 1.0, "K {k0} -> {}", c.kardashev());
        assert!(u.history.events.iter().any(|e| e.title.contains("Dyson swarm")));
    }

    #[test]
    fn terraforming_gives_mars_air_and_water() {
        let mut u = sol();
        let ci = humanity(&u);
        let mars = (0..u.systems[0].bodies.len()).find(|&i| u.systems[0].bodies[i].name == "Mars").unwrap() as u32;
        u.civs[ci].colonies.push(crate::civ::Colony { body: mars, founded: u.time, population: 1.0e5 });
        u.civs[ci].flags.insert("terraform".into());
        let p0 = u.systems[0].bodies[mars as usize].atmosphere.pressure_bar;
        let t = u.time;
        for y in 0..200 {
            u.step_megastructures(ci, t + y as f64 * 10.0 * SECONDS_PER_YEAR, 10.0);
        }
        let m = &u.systems[0].bodies[mars as usize];
        assert!(p0 < 0.02 && m.atmosphere.pressure_bar > 0.9, "pressure {p0} -> {}", m.atmosphere.pressure_bar);
        assert!(m.atmosphere.o2 > 0.18 && m.hydro.water_inventory >= 0.2);
        assert!(u.history.events.iter().any(|e| e.title.contains("Terraforming of Mars")));
    }

    #[test]
    fn generation_ships_found_new_branches_at_scouted_stars() {
        let mut u = Universe::new(UniverseSettings { seed: 7, scenario: Scenario::SolarSystemLab, ..Default::default() });
        let ci = humanity(&u);
        // A habitable world around a nearby star: clone Earth's system 4.2 ly away.
        let mut other = u.systems[0].clone();
        other.id = u.systems.len() as u32;
        other.name = "Proxima".into();
        other.star.name = "Proxima".into();
        other.position = u.systems[0].position + crate::Vec3d::new(4.2 * LIGHT_YEAR, 0.0, 0.0);
        other.nebulae.clear();
        let target_sys = other.id;
        u.systems.push(other);
        u.probes.push(crate::universe::Probe { civ: ci as u32, from: 0, to: target_sys, launched: u.time - 100.0 * SECONDS_PER_YEAR, speed: 0.05 * crate::astro::SPEED_OF_LIGHT, arrived: true });
        u.civs[ci].flags.insert("starships".into());
        u.civs[ci].starship_timer = u.time - 1000.0 * SECONDS_PER_YEAR;
        let n0 = u.civs.len();
        u.step_starships(u.time);
        assert_eq!(u.starships.len(), 1, "a ship departs");
        let trip = 4.2 * LIGHT_YEAR / u.starships[0].speed;
        u.step_starships(u.time + trip + SECONDS_PER_YEAR);
        assert_eq!(u.civs.len(), n0 + 1, "a new branch is founded");
        let b = u.civs.last().unwrap();
        assert_eq!(b.system, target_sys);
        assert_eq!(b.parent, Some(ci as u32));
        assert_eq!(b.discoveries.len(), u.civs[ci].discoveries.len());
        assert!(b.name.contains("Proxima"));
    }
}
