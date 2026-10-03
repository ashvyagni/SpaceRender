//! Versioned save files.
//!
//! A save is JSON with a small header and the full [`Universe`] state, which is made only of
//! explicit simulation structures (no runtime/engine objects). Static data that ships with
//! the app (technology graph) is *not* saved; the scientific parameters *are*, so an old
//! save keeps the science it was created with.
//!
//! When the format changes: bump [`CURRENT_VERSION`] and add a migration step to
//! [`MIGRATIONS`] that rewrites version N JSON into version N+1.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::universe::Universe;

pub const FORMAT: &str = "cosmogon-save";
pub const CURRENT_VERSION: u32 = 2;
/// Version of the physics engine (integrator and collision model) recorded in saves.
pub const PHYSICS_ENGINE: &str = "nbody-yoshida4/1";
pub const EXTENSION: &str = "cosmo";

/// `MIGRATIONS[i]` upgrades a version `i + 1` save to version `i + 2`.
const MIGRATIONS: &[fn(&mut Value) -> Result<(), SaveError>] = &[v1_to_v2];

/// v2 (0.4) adds sandbox state — N-body dynamics, provenance, object classes, removals,
/// impacts and the edit journal. Every new field has a default, so v1 data is valid v2
/// data unchanged: old universes load with analytic orbits, exactly as before.
fn v1_to_v2(_v: &mut Value) -> Result<(), SaveError> {
    Ok(())
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("not a Cosmogon save file")]
    NotASave,
    #[error("save version {0} is newer than this application supports ({CURRENT_VERSION}); please update Cosmogon")]
    TooNew(u32),
    #[error("save is corrupt: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error("migration failed: {0}")]
    Migration(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SaveHeader {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    pub name: String,
    pub saved_at_unix: u64,
    pub seed: u64,
    pub scenario: String,
    pub date: String,
    pub civilizations: u32,
    /// Integrator / collision model version (reproducibility).
    #[serde(default)]
    pub physics_engine: String,
    /// Datasets (id → version) this universe was built from.
    #[serde(default)]
    pub data_versions: std::collections::BTreeMap<String, String>,
    /// Simulation time (s since J2000).
    #[serde(default)]
    pub sim_time: f64,
}

#[derive(Serialize)]
struct SaveOut<'a> {
    header: SaveHeader,
    universe: &'a Universe,
}

#[derive(Deserialize)]
struct HeaderOnly {
    header: SaveHeader,
}

pub fn header_for(u: &Universe, name: &str) -> SaveHeader {
    SaveHeader {
        format: FORMAT.into(),
        version: CURRENT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        name: name.into(),
        saved_at_unix: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        seed: u.settings.seed,
        scenario: u.settings.scenario.label().into(),
        date: u.date_label(),
        civilizations: u.civs.iter().filter(|c| c.is_alive()).count() as u32,
        physics_engine: PHYSICS_ENGINE.into(),
        data_versions: data_versions(u),
        sim_time: u.time,
    }
}

/// The external datasets a universe depends on.
pub fn data_versions(u: &Universe) -> std::collections::BTreeMap<String, String> {
    let mut m = std::collections::BTreeMap::new();
    let sol = u.systems.iter().any(|s| s.bodies.iter().any(|b| b.real));
    if sol {
        m.insert("nasa-planetary-fact-sheets".into(), "sol.toml@1".into());
    }
    if u.systems.iter().any(|s| s.bodies.iter().any(|b| b.provenance.source.contains("Horizons"))) {
        let d = crate::astro::horizons::Dataset::embedded();
        m.insert("jpl-horizons-sol".into(), format!("{} (epoch {})", d.dataset.version, d.dataset.epoch));
    }
    if u.systems.iter().any(|s| s.bodies.iter().any(|b| b.elevation_data.is_some())) {
        m.insert("noaa-etopo5-earth".into(), "1".into());
    }
    m
}

pub fn to_json(u: &Universe, name: &str) -> Result<String, SaveError> {
    Ok(serde_json::to_string(&SaveOut { header: header_for(u, name), universe: u })?)
}

pub fn from_json(s: &str) -> Result<Universe, SaveError> {
    let mut v: Value = serde_json::from_str(s)?;
    let version = check_header(&v)?;
    for step in &MIGRATIONS[(version as usize - 1)..] {
        step(&mut v)?;
    }
    let universe = v.get_mut("universe").map(Value::take).ok_or(SaveError::NotASave)?;
    Ok(serde_json::from_value(universe)?)
}

fn check_header(v: &Value) -> Result<u32, SaveError> {
    let h = v.get("header").ok_or(SaveError::NotASave)?;
    if h.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(SaveError::NotASave);
    }
    let version = h.get("version").and_then(Value::as_u64).ok_or(SaveError::NotASave)? as u32;
    if version == 0 {
        return Err(SaveError::NotASave);
    }
    if version > CURRENT_VERSION {
        return Err(SaveError::TooNew(version));
    }
    Ok(version)
}

pub fn read_header(s: &str) -> Result<SaveHeader, SaveError> {
    let h: HeaderOnly = serde_json::from_str(s)?;
    if h.header.format != FORMAT {
        return Err(SaveError::NotASave);
    }
    Ok(h.header)
}

/// Write atomically: a crash mid-save never destroys the previous file.
pub fn write_file(path: &Path, u: &Universe, name: &str) -> Result<(), SaveError> {
    let json = to_json(u, name)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn read_file(path: &Path) -> Result<Universe, SaveError> {
    from_json(&std::fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SECONDS_PER_KYR;
    use crate::universe::{Scenario, UniverseSettings};

    #[test]
    fn round_trip_then_continue_matches_uninterrupted_run() {
        let settings = UniverseSettings { seed: 5, scenario: Scenario::Sol, system_count: 4, ..Default::default() };
        let mut a = Universe::new(settings);
        a.advance_by(2.0 * SECONDS_PER_KYR);
        let json = to_json(&a, "test").unwrap();
        let mut b = from_json(&json).unwrap();
        assert_eq!(a, b);
        a.advance_by(2.0 * SECONDS_PER_KYR);
        b.advance_by(2.0 * SECONDS_PER_KYR);
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
        assert_eq!(read_header(&json).unwrap().name, "test");
    }

    #[test]
    fn version_1_saves_still_load() {
        let u = Universe::new(UniverseSettings { seed: 9, scenario: Scenario::GardenWorld, system_count: 3, ..Default::default() });
        let json = to_json(&u, "old").unwrap().replace(&format!("\"version\":{CURRENT_VERSION}"), "\"version\":1");
        assert!(json.contains("\"version\":1"));
        let back = from_json(&json).unwrap();
        assert_eq!(back, u);
        assert!(back.systems.iter().all(|s| !s.is_dynamic()));
    }

    #[test]
    fn rejects_foreign_and_future_files() {
        assert!(matches!(from_json(r#"{"hello":1}"#), Err(SaveError::NotASave)));
        let future = format!(r#"{{"header":{{"format":"{FORMAT}","version":99}},"universe":{{}}}}"#);
        assert!(matches!(from_json(&future), Err(SaveError::TooNew(99))));
    }
}

#[cfg(test)]
mod golden {
    use crate::time::SECONDS_PER_KYR;
    use crate::universe::{Scenario, UniverseSettings};
    use crate::Universe;

    /// FNV-1a over the serialised state: a compact fingerprint of an entire universe.
    pub fn fingerprint(u: &Universe) -> u64 {
        let json = serde_json::to_string(u).unwrap();
        json.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
    }

    /// Cross-platform determinism guard. CI runs this on macOS (ARM), Linux (x86-64) and
    /// Windows (x86-64): the same seed must yield bit-identical universes everywhere.
    /// If a deliberate model change alters results, update the constant (and say so in the
    /// changelog — old saves still load, but replays of old seeds will differ).
    #[test]
    fn golden_universe_fingerprint() {
        let mut u = Universe::new(UniverseSettings { seed: 2026, scenario: Scenario::Sol, system_count: 5, ..Default::default() });
        u.advance_by(5.0 * SECONDS_PER_KYR);
        let fp = fingerprint(&u);
        println!("golden fingerprint: {fp:#018x}");
        assert_eq!(fp, GOLDEN, "universe fingerprint changed: {fp:#018x}");
    }

    const GOLDEN: u64 = 0x8094_860f_b438_e6ba;
}
