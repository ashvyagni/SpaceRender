//! Life as a sequence of broad, probabilistic transitions.
//!
//! No molecular biology: each body with a biosphere sits at a [`Stage`] and moves forward
//! by Poisson hazards whose base rates (per Gyr, `data/science.toml`) are scaled by
//! habitability, oxygen and the universe's life multipliers. Catastrophes knock stages
//! back. Oxygen, fertile land, coal and oil are *outputs* of the biosphere's history and
//! feed directly into what civilizations can later do.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::Body;
use crate::habitability::Habitability;
use crate::params::ScienceParams;
use crate::rng::Rng;
use crate::time::{SECONDS_PER_GYR, SECONDS_PER_MYR};

/// Fixed biosphere step. All biosphere updates — prehistory and live — use this size, so
/// outcomes never depend on simulation speed.
pub const STEP_YEARS: f64 = 10_000.0;
pub const STEP_SECONDS: f64 = STEP_YEARS * crate::time::SECONDS_PER_YEAR;
const DT_GYR: f64 = STEP_YEARS / 1.0e9;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Sterile,
    Prebiotic,
    Microbial,
    ComplexCells,
    Multicellular,
    ComplexEcosystems,
    Intelligent,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Sterile => "Sterile",
            Stage::Prebiotic => "Prebiotic chemistry",
            Stage::Microbial => "Microbial life",
            Stage::ComplexCells => "Complex cells",
            Stage::Multicellular => "Multicellular life",
            Stage::ComplexEcosystems => "Complex ecosystems",
            Stage::Intelligent => "Intelligent life",
        }
    }
    pub fn next(self) -> Option<Stage> {
        Some(match self {
            Stage::Sterile => Stage::Prebiotic,
            Stage::Prebiotic => Stage::Microbial,
            Stage::Microbial => Stage::ComplexCells,
            Stage::ComplexCells => Stage::Multicellular,
            Stage::Multicellular => Stage::ComplexEcosystems,
            Stage::ComplexEcosystems => Stage::Intelligent,
            Stage::Intelligent => return None,
        })
    }
    pub fn prev(self) -> Stage {
        match self {
            Stage::Sterile | Stage::Prebiotic => Stage::Sterile,
            Stage::Microbial => Stage::Prebiotic,
            Stage::ComplexCells => Stage::Microbial,
            Stage::Multicellular => Stage::ComplexCells,
            Stage::ComplexEcosystems => Stage::Multicellular,
            Stage::Intelligent => Stage::ComplexEcosystems,
        }
    }
    fn carrying_biomass(self) -> f64 {
        match self {
            Stage::Sterile | Stage::Prebiotic => 0.0,
            Stage::Microbial => 0.08,
            Stage::ComplexCells => 0.15,
            Stage::Multicellular => 0.3,
            Stage::ComplexEcosystems | Stage::Intelligent => 1.0,
        }
    }
}

/// Multipliers chosen when the universe is created.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct LifeMultipliers {
    pub life: f64,
    pub intelligence: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LifeEvent {
    StageReached(Stage),
    Oxygenation,
    MassExtinction { severity: f64, regressed: bool },
    Sterilised,
    Collapse,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Biosphere {
    pub system: u32,
    pub body: u32,
    pub stage: Stage,
    pub stage_since: f64,
    pub photosynthesis_since: Option<f64>,
    /// 0..1 relative to a mature Earth-like biosphere.
    pub biomass: f64,
    pub biodiversity: f64,
    pub extinctions: u32,
    /// Cached habitability (refreshed by the climate task).
    pub habitability: Habitability,
    /// Total Myr spent with land vegetation (drives coal formation).
    pub land_life_myr: f64,
    /// Whether an intelligent species is currently present (a civilization exists).
    pub civilization_present: bool,
}

impl Biosphere {
    pub fn new(system: u32, body: u32, habitability: Habitability, t: f64) -> Self {
        Self {
            system,
            body,
            stage: Stage::Sterile,
            stage_since: t,
            photosynthesis_since: None,
            biomass: 0.0,
            biodiversity: 0.0,
            extinctions: 0,
            habitability,
            land_life_myr: 0.0,
            civilization_present: false,
        }
    }

    pub fn vegetated(&self) -> bool {
        self.stage >= Stage::ComplexEcosystems && self.habitability.max_stage >= Stage::ComplexEcosystems
    }

    fn transition_rate(&self, p: &ScienceParams, m: LifeMultipliers, body: &Body, t: f64) -> f64 {
        let l = &p.life;
        let o2 = body.atmosphere.o2;
        let since_myr = (t - self.stage_since) / SECONDS_PER_MYR;
        let rate = match self.stage {
            Stage::Sterile => l.prebiotic_rate,
            Stage::Prebiotic => l.abiogenesis_rate,
            // Eukaryote-like cells are helped (not strictly required) by an oxidising world.
            Stage::Microbial => l.eukaryogenesis_rate * (0.15 + 4.0 * o2.min(0.21)),
            Stage::ComplexCells => l.multicellularity_rate * (0.3 + 3.3 * o2.min(0.21)),
            Stage::Multicellular => {
                if o2 < 0.05 && self.habitability.max_stage > Stage::Multicellular { 0.0 } else { l.complex_ecosystem_rate }
            }
            Stage::ComplexEcosystems => {
                // Needs time to build up diverse, large-brained lineages.
                if since_myr < 50.0 || self.biodiversity < 0.4 { 0.0 } else { l.intelligence_rate * m.intelligence / m.life.max(1e-9) }
            }
            Stage::Intelligent => 0.0,
        };
        rate * m.life * self.habitability.score
    }

    /// Advance one fixed [`STEP_YEARS`] step ending at time `t`.
    pub fn step(&mut self, body: &mut Body, p: &ScienceParams, m: LifeMultipliers, flare_activity: f64, t: f64, rng: &mut Rng) -> Vec<LifeEvent> {
        let mut events = Vec::new();
        let hab = self.habitability.score;

        // Environment collapsed (star brightening, water loss…): life declines and may vanish.
        if hab <= 0.0 {
            if self.stage > Stage::Sterile {
                self.biomass *= 0.97;
                self.biodiversity *= 0.97;
                if rng.hazard(3.0, DT_GYR) || self.biomass < 1e-3 {
                    self.stage = Stage::Sterile;
                    self.stage_since = t;
                    self.biomass = 0.0;
                    self.biodiversity = 0.0;
                    events.push(LifeEvent::Collapse);
                }
            }
            return events;
        }

        // Progress.
        if self.stage < self.habitability.max_stage && !self.civilization_present {
            let rate = self.transition_rate(p, m, body, t);
            if rng.hazard(rate, DT_GYR) {
                if let Some(next) = self.stage.next() {
                    self.stage = next;
                    self.stage_since = t;
                    events.push(LifeEvent::StageReached(next));
                }
            }
        }

        // Photosynthesis and the oxygenation of the atmosphere (surface biospheres only).
        let surface = self.habitability.max_stage > Stage::Multicellular;
        if surface && self.stage >= Stage::Microbial {
            if self.photosynthesis_since.is_none() && rng.hazard(p.life.photosynthesis_rate * m.life, DT_GYR) {
                self.photosynthesis_since = Some(t);
            }
            if let Some(since) = self.photosynthesis_since {
                if (t - since) / SECONDS_PER_MYR > p.life.oxygen_sink_delay_myr && body.atmosphere.is_present() {
                    let land = if self.stage >= Stage::ComplexEcosystems { 1.0 } else { 0.55 };
                    let target = 0.21 * self.biomass.max(0.3) * land;
                    let k = 1.0 - (-STEP_YEARS / 1e6 / p.life.oxygenation_timescale_myr).dexp();
                    let before = body.atmosphere.o2;
                    let o2 = before + (target - before) * k;
                    let a = &mut body.atmosphere;
                    let other = 1.0 - before;
                    if other > 1e-9 {
                        let scale = (1.0 - o2) / other;
                        a.n2 *= scale;
                        a.co2 *= scale;
                        a.h2o *= scale;
                        a.ch4 *= scale;
                        a.h2he *= scale;
                    }
                    a.o2 = o2;
                    if before < 0.02 && o2 >= 0.02 {
                        events.push(LifeEvent::Oxygenation);
                    }
                }
            }
        }

        // Biomass and diversity relax towards what the stage and environment support.
        let target = self.stage.carrying_biomass() * hab.sqrt();
        self.biomass += (target - self.biomass) * 0.02;
        let div_target = target * (0.6 + 0.4 * hab);
        self.biodiversity += (div_target - self.biodiversity) * 0.004;

        // Buried biomass becomes fossil fuel.
        if self.vegetated() {
            self.land_life_myr += STEP_YEARS / 1e6;
            body.resources.coal += p.life.coal_formation_rate * self.biomass * body.land_fraction().min(0.6) / 0.3 * DT_GYR;
        }
        if surface && self.stage >= Stage::Microbial {
            body.resources.oil += p.life.oil_formation_rate * self.biomass * body.hydro.ocean_fraction / 0.7 * DT_GYR;
        }

        // Catastrophes.
        let c = &p.catastrophes;
        if self.stage >= Stage::Microbial {
            if rng.hazard(c.sterilisation_rate, DT_GYR) {
                self.stage = Stage::Sterile;
                self.stage_since = t;
                self.biomass = 0.0;
                self.biodiversity = 0.0;
                self.photosynthesis_since = None;
                events.push(LifeEvent::Sterilised);
                return events;
            }
            let rate = c.mass_extinction_rate * (1.0 + c.flare_extinction_factor * flare_activity * (1.0 - hab));
            if self.stage >= Stage::Multicellular && rng.hazard(rate, DT_GYR) {
                let severity = rng.range(0.3, 0.96);
                self.biodiversity *= 1.0 - severity;
                self.biomass *= 1.0 - severity * 0.5;
                self.extinctions += 1;
                let regressed = severity > 0.85 && self.stage < Stage::Intelligent && rng.chance(0.4);
                if regressed {
                    self.stage = self.stage.prev();
                    self.stage_since = t;
                }
                events.push(LifeEvent::MassExtinction { severity, regressed });
            }
        }
        events
    }
}

/// Years needed for `stage` transitions to be meaningful at a glance (UI helper).
pub fn gyr(seconds: f64) -> f64 {
    seconds / SECONDS_PER_GYR
}
