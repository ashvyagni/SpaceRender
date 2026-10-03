//! Intelligent species, derived from the world they evolved on.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::Body;
use crate::names;
use crate::rng::Rng;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Habitat {
    Land,
    Water,
}

impl Habitat {
    pub fn label(self) -> &'static str {
        match self {
            Habitat::Land => "land-dwelling",
            Habitat::Water => "aquatic",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Species {
    pub name: String,
    pub habitat: Habitat,
    /// Typical adult mass (kg).
    pub mass_kg: f64,
    pub lifespan_years: f64,
    /// 0..1: how readily individuals cooperate in large groups.
    pub sociality: f64,
    pub traits: Vec<String>,
}

impl Species {
    pub fn generate(rng: &mut Rng, body: &Body) -> Self {
        let g = body.gravity_g();
        let ocean = body.hydro.ocean_fraction;
        let aquatic_p = if ocean > 0.92 { 0.7 } else { 0.12 * ocean };
        let habitat = if rng.chance(aquatic_p) { Habitat::Water } else { Habitat::Land };
        let mass_kg = 65.0 * g.max(0.1).dpowf(-0.8) * rng.normal(0.0, 0.4).dexp();
        let lifespan_years = rng.range(35.0, 120.0);
        let sociality = rng.range(0.35, 1.0);
        let mut traits = Vec::new();
        traits.push(match habitat {
            Habitat::Water => "Aquatic: breathes and builds underwater".to_string(),
            Habitat::Land => "Terrestrial".to_string(),
        });
        if g < 0.6 {
            traits.push("Tall, light-boned frame (low gravity)".into());
        } else if g > 1.5 {
            traits.push("Squat, heavily muscled build (high gravity)".into());
        }
        if body.tidally_locked {
            traits.push("Evolved in the twilight band of a tidally locked world".into());
        }
        if body.temperature < 270.0 {
            traits.push("Cold-adapted".into());
        } else if body.temperature > 305.0 {
            traits.push("Heat-adapted".into());
        }
        traits.push(if sociality > 0.8 { "Highly eusocial".into() } else if sociality < 0.5 { "Small, loosely bound kin groups".into() } else { "Tribal social structure".into() });
        Self { name: names::word(rng), habitat, mass_kg, lifespan_years, sociality, traits }
    }
}
