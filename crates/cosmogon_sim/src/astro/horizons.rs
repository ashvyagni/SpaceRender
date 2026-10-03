//! The cached JPL Horizons dataset (`data/horizons_sol.toml`, written by
//! `tools/fetch_horizons.py`): real barycentric state vectors of the Solar System at one
//! epoch. See docs/DATA_SOURCES.md.

use serde::Deserialize;

use super::dynamics::{PhysicsSettings, State};
use super::StarSystem;
use crate::time::SECONDS_PER_DAY;
use crate::Vec3d;

const HORIZONS_TOML: &str = include_str!("../../data/horizons_sol.toml");
/// Julian date of J2000.0 (TDB).
const JD_J2000: f64 = 2_451_545.0;

#[derive(Deserialize, Clone, Debug)]
pub struct DatasetInfo {
    pub provider: String,
    pub name: String,
    pub version: u32,
    pub epoch: String,
    pub epoch_jd_tdb: f64,
    pub retrieved_utc: String,
    pub license: String,
    pub attribution: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Row {
    pub name: String,
    pub horizons_id: String,
    pub ephemeris: String,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

#[derive(Deserialize, Clone, Debug)]
pub struct Dataset {
    pub dataset: DatasetInfo,
    pub body: Vec<Row>,
}

impl Dataset {
    pub fn embedded() -> Dataset {
        toml::from_str(HORIZONS_TOML).expect("embedded data/horizons_sol.toml must parse")
    }

    /// Dataset id used in provenance and save manifests.
    pub fn id(&self) -> String {
        format!("jpl-horizons-sol@{}", self.dataset.version)
    }

    /// Epoch in simulation seconds (TDB seconds since J2000.0).
    pub fn epoch_seconds(&self) -> f64 {
        (self.dataset.epoch_jd_tdb - JD_J2000) * SECONDS_PER_DAY
    }

    pub fn state(&self, name: &str) -> Option<State> {
        self.body.iter().find(|r| r.name == name).map(|r| State {
            pos: Vec3d::new(r.position_km[0], r.position_km[1], r.position_km[2]) * 1000.0,
            vel: Vec3d::new(r.velocity_km_s[0], r.velocity_km_s[1], r.velocity_km_s[2]) * 1000.0,
        })
    }

    pub fn source_label(&self, row: &Row) -> String {
        format!("NASA/JPL Horizons ({}, {}) — dataset {} epoch {}", row.ephemeris, row.horizons_id, self.id(), self.dataset.epoch)
    }
}

/// Put `sys` (the Sol system) into N-body mode with the measured states of the dataset.
pub fn apply_to_sol(sys: &mut StarSystem, settings: PhysicsSettings) -> f64 {
    let data = Dataset::embedded();
    let t = data.epoch_seconds();
    let star = data.state("Sun").expect("dataset has the Sun");
    let mut states = Vec::new();
    for (i, b) in sys.bodies.iter_mut().enumerate() {
        if let Some(row) = data.body.iter().find(|r| r.name == b.name) {
            states.push((i, data.state(&row.name).unwrap()));
            b.provenance.source = format!("{} · physical data: NASA planetary fact sheets", data.source_label(row));
            for f in ["position", "velocity", "orbit"] {
                b.mark(f, super::Quality::Measured);
            }
        }
    }
    sys.activate_from_states(t, settings, star, &states);
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::sol::sol_system;
    use crate::astro::AU;

    #[test]
    fn dataset_loads_and_is_consistent() {
        let d = Dataset::embedded();
        assert_eq!(d.body.len(), 16);
        let t = d.epoch_seconds();
        assert!((t / (365.25 * 86400.0) - 26.0).abs() < 0.01, "epoch {t}");
        let earth = d.state("Earth").unwrap();
        let sun = d.state("Sun").unwrap();
        let r = (earth.pos - sun.pos).length() / AU;
        assert!((0.98..1.02).contains(&r), "{r}");
    }

    /// Integrate the real Solar System for one year and compare with JPL's own ephemeris
    /// one year later (values fetched from Horizons for 2027-01-01 00:00 TDB).
    #[test]
    fn one_year_matches_jpl_ephemeris() {
        let mut sys = sol_system();
        let t0 = apply_to_sol(&mut sys, crate::astro::dynamics::PhysicsPreset::Accurate.settings());
        let t1 = t0 + 365.0 * SECONDS_PER_DAY;
        sys.sync_to(t1);
        // Heliocentric positions from Horizons for 2027-01-01 TDB (km), see tools/fetch_horizons.py.
        let expected = super::validation::HELIO_2027;
        let sun = sys.star_local_position(t1);
        for (name, km) in expected {
            let i = sys.find_body(name).unwrap();
            let p = (sys.body_local_position(i, t1) - sun) / 1000.0;
            let err = (p - Vec3d::new(km[0], km[1], km[2])).length();
            let tol = if *name == "Moon" { 3_000.0 } else { 1_500.0 };
            println!("{name:8} error after one year: {err:9.0} km");
            assert!(err < tol, "{name}: {err} km");
        }
    }
}

#[cfg(test)]
mod validation {
    include!("../../data/horizons_validation_2027.rs");
}
