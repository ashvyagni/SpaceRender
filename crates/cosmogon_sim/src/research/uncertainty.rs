//! Authored input-band envelopes for controlled comparison studies.
//!
//! A band is an author-declared relative range around each swept input value,
//! executed as three independent deterministic branches (low / central / high).
//! Reports carry the actual per-metric outcome triples and, for two banded
//! recordings, a conservative B−A interval by interval arithmetic. This is
//! modeled sensitivity to an authored input range — never statistical
//! uncertainty, confidence intervals, covariance or field calibration.
use crate::habitat_sweep::Parameter;
use serde::{Deserialize, Serialize};

pub const FORMAT_RUN_BAND: &str = "cosmogon-study-run-band";
pub const FORMAT_UNCERTAINTY: &str = "cosmogon-study-uncertainty";
pub const FORMAT_COMPARISON: &str = "cosmogon-study-band-comparison";
pub const METHOD_INPUT_BAND: &str = "authored-input-band";
pub const METHOD_INTERVAL_ARITHMETIC: &str = "authored-input-band-interval-arithmetic";

/// Per-run provenance written beside a banded sweep recording (`band.json`).
/// Every run of a value triple carries the same self-contained triple finals so
/// any single folder can support a band comparison without its sweep root.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBand {
    pub format: String,
    pub schema_version: u32,
    pub parameter: Parameter,
    pub relative_band: f64,
    pub role: BandRole,
    /// The actual value applied to this run (swept value × band multiplier).
    pub value: f64,
    /// The unmultiplied swept value this run's triple belongs to.
    pub swept_value: f64,
    /// Metric id → unit and the triple's final-observation values.
    pub triple_final_metrics: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandRole {
    Low,
    #[default]
    Central,
    High,
}
impl BandRole {
    pub fn id(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Central => "central",
            Self::High => "high",
        }
    }
    pub fn multiplier(self, relative_band: f64) -> f64 {
        match self {
            Self::Low => 1.0 - relative_band,
            Self::Central => 1.0,
            Self::High => 1.0 + relative_band,
        }
    }
}

impl RunBand {
    /// Band metadata beside a recording, or `None` when no band.json exists.
    pub fn read_optional(path: &std::path::Path) -> Result<Option<Self>, String> {
        if !path.join("band.json").exists() {
            return Ok(None);
        }
        Self::read(path).map(Some)
    }
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let file = path.join("band.json");
        let metadata = std::fs::metadata(&file).map_err(|_| "No authored band metadata (band.json) beside this recording".to_string())?;
        if !metadata.is_file() || metadata.len() > 262_144 {
            return Err("Band metadata must be a regular file within 256 KiB".into());
        }
        let f = std::fs::File::open(&file).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        f.take(262_145).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() > 262_144 {
            return Err("Band metadata exceeds 256 KiB".into());
        }
        let band: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        band.validate()?;
        Ok(band)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT_RUN_BAND || self.schema_version != 1 {
            return Err("Unsupported band metadata format or schema".into());
        }
        validate_relative_band(self.relative_band)?;
        if !self.value.is_finite() || !self.swept_value.is_finite() {
            return Err("Band values must be finite".into());
        }
        let expected = self.swept_value * self.role.multiplier(self.relative_band);
        if self.value != expected {
            return Err("Band run value does not match its swept value, role and relative band".into());
        }
        for (id, entry) in &self.triple_final_metrics {
            if id.is_empty() {
                return Err("Band metric ids must not be empty".into());
            }
            for key in ["at_low", "at_central", "at_high"] {
                if let Some(v) = entry.get(key).and_then(|v| v.as_f64()) {
                    if !v.is_finite() {
                        return Err(format!("Band metric {id} {key} must be finite or absent"));
                    }
                }
            }
        }
        Ok(())
    }
    /// Conservative interval on B−A at each run's own final observation.
    pub fn comparison_block(
        a: &Self,
        b: &Self,
        same_horizon: bool,
    ) -> serde_json::Value {
        let mut block = serde_json::json!({
            "format": FORMAT_COMPARISON,
            "schema_version": 1,
            "method": METHOD_INTERVAL_ARITHMETIC,
            "parameter": a.parameter.id(),
            "relative_band": a.relative_band,
            "a_role": a.role.id(),
            "b_role": b.role.id(),
            "a_swept_value": a.swept_value,
            "b_swept_value": b.swept_value,
            "evaluation": "per_run_final_observation",
            "same_horizon": same_horizon,
            "assumption": "Independent authored input bands; interval arithmetic on final-observation outcomes (low = B_low − A_high, high = B_high − A_low). No covariance, interpolation, extrapolation or statistical confidence.",
            "scope": "Authored ±relative input range on each compared run's swept parameter only; all other assumptions stay fixed. Deterministic modeled sensitivity, not predictive uncertainty or field calibration.",
        });
        if a.parameter != b.parameter || a.relative_band != b.relative_band {
            block["unavailable"] = serde_json::json!(
                "Compared recordings carry different swept parameters or band fractions; a shared input band is required for interval arithmetic"
            );
            return block;
        }
        let mut metrics = serde_json::Map::new();
        for (id, a_entry) in &a.triple_final_metrics {
            let Some(b_entry) = b.triple_final_metrics.get(id) else { continue };
            let get = |entry: &serde_json::Value, key: &str| entry.get(key).and_then(|v| v.as_f64());
            let (Some(a_low), Some(a_c), Some(a_high), Some(b_low), Some(b_c), Some(b_high)) = (
                get(a_entry, "at_low"), get(a_entry, "at_central"), get(a_entry, "at_high"),
                get(b_entry, "at_low"), get(b_entry, "at_central"), get(b_entry, "at_high"),
            ) else {
                metrics.insert(id.clone(), serde_json::json!({"unavailable":"Triple final values incomplete on one side"}));
                continue;
            };
            let unit = a_entry.get("unit").cloned().unwrap_or(serde_json::Value::Null);
            metrics.insert(id.clone(), serde_json::json!({
                "unit": unit,
                "central_b_minus_a": b_c - a_c,
                "at_low": b_low - a_high,
                "at_high": b_high - a_low,
                "min": (b_low - a_high).min(b_high - a_low),
                "max": (b_low - a_high).max(b_high - a_low),
            }));
        }
        block["metrics"] = serde_json::Value::Object(metrics);
        block
    }
}

pub fn validate_relative_band(b: f64) -> Result<(), String> {
    if !b.is_finite() || !(0.0 < b && b < 1.0) {
        return Err("Authored band must be a finite fraction strictly between 0 and 1".into());
    }
    Ok(())
}

/// Manifest uncertainty block for a completed single-setting banded sweep.
/// `values` carries one entry per swept value with per-metric outcome triples.
pub fn manifest_block(
    parameter: Parameter,
    unit: &str,
    relative_band: f64,
    values: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "format": FORMAT_UNCERTAINTY,
        "schema_version": 1,
        "method": METHOD_INPUT_BAND,
        "parameter": parameter.id(),
        "unit": unit,
        "relative_band": relative_band,
        "evaluation": "final_observation",
        "values": values,
        "scope": "Authored ±relative input band around each swept value, executed as three independent deterministic branches per value (low = ×(1−band), central = ×1, high = ×(1+band)). Modeled output sensitivity to an authored input range — not statistical uncertainty, confidence intervals, covariance, calibration or prediction error.",
        "limits": "The band applies only to the swept parameter; every other assumption stays fixed. Endpoints outside a parameter's allowed domain refuse the whole plan before any output is written. Paired (two-setting) designs and zero swept values are not supported with bands.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(low: f64, c: f64, high: f64) -> serde_json::Value {
        serde_json::json!({"unit":"kg","at_low":low,"at_central":c,"at_high":high})
    }
    fn band(role: BandRole, triple: serde_json::Value) -> RunBand {
        let swept = 1000.0;
        RunBand {
            format: FORMAT_RUN_BAND.into(),
            schema_version: 1,
            parameter: Parameter::FirmPowerW,
            relative_band: 0.1,
            role,
            value: swept * role.multiplier(0.1),
            swept_value: swept,
            triple_final_metrics: serde_json::from_value(triple).unwrap(),
        }
    }
    #[test]
    fn relative_band_bounds_and_run_consistency_are_enforced() {
        assert!(validate_relative_band(0.0).is_err());
        assert!(validate_relative_band(1.0).is_err());
        assert!(validate_relative_band(f64::NAN).is_err());
        assert!(validate_relative_band(0.25).is_ok());
        let mut b = band(BandRole::Central, serde_json::json!({"m": entry(1., 2., 3.)}));
        b.validate().unwrap();
        b.value = 5.0;
        assert!(b.validate().is_err());
        let mut b = band(BandRole::Low, serde_json::json!({"m": entry(1., 2., 3.)}));
        b.validate().unwrap();
        assert!((b.value - 900.0).abs() < 1e-12);
        b.format = "other".into();
        assert!(b.validate().is_err());
    }
    #[test]
    fn comparison_interval_arithmetic_is_conservative_and_scoped() {
        let a = band(BandRole::Central, serde_json::json!({"methane": entry(8., 10., 12.)}));
        let b = band(BandRole::Central, serde_json::json!({"methane": entry(18., 20., 22.)}));
        let block = RunBand::comparison_block(&a, &b, true);
        assert_eq!(block["method"], METHOD_INTERVAL_ARITHMETIC);
        assert_eq!(block["same_horizon"], true);
        let m = &block["metrics"]["methane"];
        assert_eq!(m["central_b_minus_a"], 10.0);
        assert_eq!(m["at_low"], 18.0 - 12.0);
        assert_eq!(m["at_high"], 22.0 - 8.0);
        let mut other = a.clone();
        other.relative_band = 0.2;
        let block = RunBand::comparison_block(&a, &other, false);
        assert!(block["unavailable"].is_string());
        assert_eq!(block["same_horizon"], false);
        let mut a2 = a.clone();
        a2.triple_final_metrics.get_mut("methane").unwrap()["at_low"] = serde_json::Value::Null;
        let block = RunBand::comparison_block(&a2, &b, true);
        assert!(block["metrics"]["methane"]["unavailable"].is_string());
    }
}
