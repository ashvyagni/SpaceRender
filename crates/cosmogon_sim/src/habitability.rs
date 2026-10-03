//! Multi-factor habitability.
//!
//! Habitability is a score in 0..1 with an itemised breakdown, plus the most complex stage
//! of life the environment can plausibly support (a subsurface ocean can host microbes or
//! perhaps simple multicellular life, but nothing will ever light a fire there). The score
//! *modulates rates* in the life model — it never decides outcomes on its own.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::{BodyKind, StarSystem};
use crate::astro::star::StellarPhase;
use crate::life::Stage;
use crate::planet::environment::{insolation, liquid_water_stable};
use crate::time::SECONDS_PER_GYR;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Factor {
    pub name: String,
    pub value: f64,
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Habitability {
    pub score: f64,
    pub max_stage: Stage,
    pub environment: String,
    pub factors: Vec<Factor>,
}

impl Habitability {
    pub fn sterile(reason: &str) -> Self {
        Self { score: 0.0, max_stage: Stage::Sterile, environment: reason.into(), factors: Vec::new() }
    }

    pub fn label(&self) -> &'static str {
        match self.score {
            s if s <= 0.0 => "Uninhabitable",
            s if s < 0.15 => "Marginal",
            s if s < 0.4 => "Poor",
            s if s < 0.65 => "Moderate",
            s if s < 0.85 => "Good",
            _ => "Excellent",
        }
    }
}

fn gauss(x: f64, mu: f64, sigma: f64) -> f64 {
    (-((x - mu) / sigma).powi(2)).dexp()
}

pub fn assess(sys: &StarSystem, body_idx: usize, t: f64) -> Habitability {
    let b = &sys.bodies[body_idx];
    if !b.kind.has_surface() {
        return Habitability::sterile("No solid surface (giant planet)");
    }
    let p = b.atmosphere.pressure_bar;
    let surface_water = b.hydro.ocean_fraction > 0.0 && liquid_water_stable(b.temperature, p);
    let subsurface = b.hydro.subsurface_ocean && !surface_water;
    if !surface_water && !subsurface {
        let why = if b.hydro.water_inventory < 1e-3 {
            "No water"
        } else if b.temperature <= 273.15 {
            "Water is frozen"
        } else if p < 0.006 {
            "Atmosphere too thin for liquid water"
        } else {
            "Too hot for liquid water"
        };
        return Habitability::sterile(why);
    }

    let mut f = Vec::new();
    let mut push = |name: &str, value: f64, note: String| f.push(Factor { name: name.into(), value: value.clamp(0.0, 1.0), note });

    if surface_water {
        push("Temperature", gauss(b.temperature, 292.0, 38.0), format!("{:.0} K mean surface", b.temperature));
        let o = b.hydro.ocean_fraction;
        let water = if o < 0.1 { o / 0.1 * 0.7 } else if o > 0.95 { 0.75 } else { 1.0 };
        push("Liquid water", water, format!("{:.0}% ocean cover", o * 100.0));
        let pr = if (0.2..=10.0).contains(&p) { 1.0 } else { gauss(p.max(1e-6).dlog10(), 0.0, 1.2) };
        push("Surface pressure", pr, format!("{p:.2} bar"));
        let shield = (0.45 * b.magnetic_field.min(1.0) + 0.55 * p.min(1.0)).min(1.0);
        let flare = sys.star.flare_activity_at(t);
        push("Radiation", 1.0 - 0.75 * flare * (1.0 - shield) - 0.2 * (1.0 - shield), format!("magnetic field {:.2}× Earth, stellar activity {:.2}", b.magnetic_field, flare));
        let regulation = if b.geology > 0.3 && b.geology < 4.0 { 1.0 } else { 0.4 + b.geology.min(1.0) * 0.5 };
        push("Climate regulation", regulation, format!("geological activity {:.2}× Earth (carbon–silicate cycle)", b.geology));
        if b.tidally_locked && b.parent.is_none() {
            push("Day–night cycle", 0.55, "tidally locked: life concentrated near the terminator".into());
        }
        let s = insolation(&sys.star, t, sys.stellar_distance(body_idx));
        push("Stellar energy", gauss(s.max(1e-6).dln(), 0.0, 0.9), format!("{s:.2}× Earth's insolation"));
    } else {
        push("Subsurface ocean", 0.55, "liquid water beneath an ice shell".into());
        push("Energy", (b.geology / 1.0).clamp(0.1, 1.0), format!("tidal/radiogenic heating {:.2}× Earth", b.geology));
    }

    let remaining = (sys.star.lifetime - sys.star.age(t)) / SECONDS_PER_GYR;
    let stellar = match sys.star.phase(t) {
        StellarPhase::MainSequence => (remaining / 1.0).clamp(0.05, 1.0),
        _ => 0.0,
    };
    push("Stellar stability", stellar, format!("{} — {:.1} Gyr of main sequence left", sys.star.phase(t).label(), remaining.max(0.0)));
    let chem = (10f64.dpowf(sys.star.metallicity)).clamp(0.4, 1.0);
    push("Chemistry", chem, format!("[Fe/H] = {:+.2}", sys.star.metallicity));

    // Weighted geometric mean: one terrible factor drags everything down.
    let ln_sum: f64 = f.iter().map(|x| x.value.max(0.01).dln()).sum();
    let score = if stellar <= 0.0 { 0.0 } else { (ln_sum / f.len() as f64).dexp() };

    let max_stage = if surface_water {
        if b.kind == BodyKind::Rocky { Stage::Intelligent } else { Stage::ComplexEcosystems }
    } else {
        Stage::Multicellular
    };
    let environment = if surface_water { "Surface oceans".into() } else { "Subsurface ocean".into() };
    Habitability { score, max_stage, environment, factors: f }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::sol;

    #[test]
    fn sol_habitability_ranking() {
        let s = sol::sol_system();
        let h = |n: &str| assess(&s, s.find_body(n).unwrap(), 0.0);
        let earth = h("Earth");
        assert!(earth.score > 0.75, "{earth:?}");
        assert_eq!(earth.max_stage, Stage::Intelligent);
        assert_eq!(h("Mars").score, 0.0);
        assert_eq!(h("Venus").score, 0.0);
        assert_eq!(h("Jupiter").score, 0.0);
        let europa = h("Europa");
        assert!(europa.score > 0.0 && europa.score < earth.score);
        assert_eq!(europa.max_stage, Stage::Multicellular);
    }
}
