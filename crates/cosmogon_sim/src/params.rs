//! Centralised, data-driven scientific parameters (`data/science.toml`).

use serde::{Deserialize, Serialize};

const SCIENCE_TOML: &str = include_str!("../data/science.toml");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LifeParams {
    pub prebiotic_rate: f64,
    pub abiogenesis_rate: f64,
    pub photosynthesis_rate: f64,
    pub oxygenation_timescale_myr: f64,
    pub oxygen_sink_delay_myr: f64,
    pub eukaryogenesis_rate: f64,
    pub multicellularity_rate: f64,
    pub complex_ecosystem_rate: f64,
    pub intelligence_rate: f64,
    pub coal_formation_rate: f64,
    pub oil_formation_rate: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CatastropheParams {
    pub mass_extinction_rate: f64,
    pub sterilisation_rate: f64,
    pub flare_extinction_factor: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CivParams {
    pub base_growth_rate: f64,
    pub research_exponent: f64,
    pub research_scale: f64,
    pub oral_knowledge_decay: f64,
    pub pandemic_rate: f64,
    pub war_rate: f64,
    pub disaster_rate: f64,
    pub collapse_threshold: f64,
    pub probe_speed_c: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScienceParams {
    pub life: LifeParams,
    pub catastrophes: CatastropheParams,
    pub civilization: CivParams,
}

impl ScienceParams {
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

impl Default for ScienceParams {
    fn default() -> Self {
        Self::from_toml(SCIENCE_TOML).expect("embedded data/science.toml must parse")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_params_parse() {
        let p = super::ScienceParams::default();
        assert!(p.life.abiogenesis_rate > 0.0);
    }
}
