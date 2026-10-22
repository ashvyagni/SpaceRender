//! Read-only, bounded habitat studies. Structural/source checks are not field validation
//! or a cryptographic guarantee that a trajectory was produced by this application.
use crate::{
    habitat::{self, Sample},
    research::state_fingerprint,
    save,
};
use serde::Serialize;
use serde_json::Value;
use std::{io::Read, path::Path};

pub(crate) fn read(path: &Path, limit: u64) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err("Study entries must be regular files".into());
    }
    if metadata.len() > limit {
        return Err(format!(
            "{} exceeds the {} byte study limit",
            path.display(),
            limit
        ));
    }
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Study entries must be regular files".into());
    }
    let mut bytes = Vec::new();
    f.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{} exceeds the {} byte study limit",
            path.display(),
            limit
        ));
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn json(path: &Path, limit: u64) -> Result<Value, String> {
    serde_json::from_str(&read(path, limit)?).map_err(|e| e.to_string())
}
#[derive(Clone, Debug)]
pub struct Recording {
    pub rows: Vec<Sample>,
    pub summary: Value,
}
impl Recording {
    pub fn read(path: &Path) -> Result<Self, String> {
        let m = json(&path.join("manifest.json"), 64 * 1024)?;
        if m["format"] != "cosmogon-habitat-study"
            || m["schema_version"] != 1
            || m["model"] != habitat::MODEL
        {
            return Err("Unsupported habitat study format, schema or model".into());
        }
        if m["source"] != habitat::SOURCE {
            return Err("Habitat study reference source does not match this model".into());
        }
        let count = m["sample_count"]
            .as_u64()
            .filter(|n| (1..=1024).contains(n))
            .ok_or("Habitat study requires 1–1024 samples")?;
        let id = m["habitat_id"].as_u64().ok_or("Missing habitat identity")?;
        let rows: Vec<Sample> =
            serde_json::from_str(&read(&path.join("samples.json"), 16 * 1024 * 1024)?)
                .map_err(|e| e.to_string())?;
        if rows.len() != count as usize {
            return Err("Habitat sample count disagrees with manifest".into());
        }
        let initial = save::from_json(&read(&path.join("initial.cosmo"), 32 * 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
        let final_world = save::from_json(&read(&path.join("final.cosmo"), 32 * 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
        let first = habitat::sample(&initial, id)?;
        let last = habitat::sample(&final_world, id)?;
        if rows.first() != Some(&first) || rows.last() != Some(&last) {
            return Err("Study endpoint samples disagree with exact saved worlds".into());
        }
        let initial_fp = state_fingerprint(&initial)?;
        let final_fp = state_fingerprint(&final_world)?;
        if m["initial_state_fingerprint"] != initial_fp || m["final_state_fingerprint"] != final_fp
        {
            return Err("Study state identity disagrees with manifest".into());
        }
        let definition: habitat::Definition =
            serde_json::from_value(json(&path.join("definition.json"), 64 * 1024)?)
                .map_err(|e| e.to_string())?;
        if definition != first.habitat.definition || definition != last.habitat.definition {
            return Err("Study original definition disagrees with endpoints".into());
        }
        if final_world.time < initial.time
            || final_world.time - initial.time > 30.0 * crate::time::SECONDS_PER_DAY
        {
            return Err("Study time range exceeds the 30-day reader horizon".into());
        }
        let mass = first.habitat.state.accounted_mass_kg();
        let mut previous = None;
        let mut uses_electrolysis = false;
        let mut uses_sabatier = false;
        let mut uses_moisture = false;
        let mut uses_breach = false;
        let mut uses_crop = false;
        let mut uses_radiation = false;
        for row in &rows {
            if !row.simulation_time_s.is_finite()
                || row.simulation_time_s < initial.time
                || row.simulation_time_s > final_world.time
                || previous.is_some_and(|t| row.simulation_time_s <= t)
            {
                return Err("Habitat observations must have finite increasing world times".into());
            }
            previous = Some(row.simulation_time_s);
            if row.habitat.id != id || row.habitat.definition != definition {
                return Err(
                    "Habitat identity or original definition changes within recording".into(),
                );
            }
            row.habitat.validate(row.simulation_time_s)?;
            let expected =
                mass + row.habitat.totals.inflow_gas.map(|g| g.mass_kg()).unwrap_or(0.0);
            if (row.habitat.state.accounted_mass_kg() - expected).abs() > 1e-8 * expected.max(1.0) {
                return Err("Recorded closed habitat mass disagrees with initial inventory".into());
            }
            uses_electrolysis |= row.habitat.uses_electrolysis();
            uses_sabatier |= row.habitat.uses_sabatier();
            uses_moisture |= row.habitat.uses_moisture();
            uses_breach |= row.habitat.uses_breach();
            uses_crop |= row.habitat.uses_crop();
            uses_radiation |= row.habitat.uses_radiation();
        }
        for (used, key, descriptor, expected) in [
            (
                uses_electrolysis,
                "electrolysis_source",
                "electrolysis_model_descriptor",
                crate::electrolysis::source(),
            ),
            (
                uses_sabatier,
                "sabatier_source",
                "sabatier_model_descriptor",
                crate::sabatier::source(),
            ),
            (
                uses_moisture,
                "moisture_source",
                "moisture_model_descriptor",
                crate::moisture::source(),
            ),
            (
                uses_breach,
                "breach_source",
                "breach_model_descriptor",
                crate::breach::source(),
            ),
        ] {
            if used
                && (m[key] != expected
                    || m[descriptor]["id"]
                        != if key == "breach_source" {
                            "breach"
                        } else if key == "moisture_source" {
                            "moisture"
                        } else if key == "sabatier_source" {
                            "sabatier"
                        } else {
                            "electrolysis"
                        })
            {
                return Err(
                    "Missing or mismatched recorded reaction source/model descriptor".into(),
                );
            }
            if !used && !m[key].is_null() {
                return Err("Recording declares an unused reaction source".into());
            }
        }
        if uses_crop {
            crate::habitat_crop_trajectory::validate_series(&rows)?;
            if m["crop_source"]!=crate::crop_process::source() || m["crop_model_descriptor"]!=serde_json::to_value(crate::research::PhysicsModelRegistry::get("finite-crop")).map_err(|e|e.to_string())? {return Err("Crop source/model mismatch".into());}
            if read(&path.join("crop.csv"),16*1024*1024)?!=crate::crop_process::samples_csv(&rows) {return Err("Actual crop CSV disagrees with saved observations".into());}
        } else if !m["crop_source"].is_null() || !m["crop_model_descriptor"].is_null() {return Err("Recording declares unused crop process".into());}
        if uses_radiation {
            if m["radiation_source"]!=crate::habitat_radiation::source()
                || m["radiation_model_descriptor"]!=serde_json::to_value(crate::research::PhysicsModelRegistry::get("habitat-radiation")).map_err(|e|e.to_string())?
            {
                return Err("Radiation source/model mismatch".into());
            }
        } else if !m["radiation_source"].is_null() || !m["radiation_model_descriptor"].is_null() {return Err("Recording declares unused radiation model".into());}
        if definition.record_heat {
            if m["heat_accounting_source"] != crate::habitat_heat::source()
                || m["heat_accounting_model_descriptor"]
                    != serde_json::to_value(crate::research::PhysicsModelRegistry::get(
                        "habitat-heat",
                    ))
                    .map_err(|e| e.to_string())?
            {
                return Err("Missing or mismatched recorded heat accounting source/model".into());
            }
        } else if !m["heat_accounting_source"].is_null()
            || !m["heat_accounting_model_descriptor"].is_null()
        {
            return Err("Recording declares unused heat accounting".into());
        }
        if definition.record_heat {
            crate::habitat_heat::span(&rows)
                .map_err(|e| format!("Recorded heat continuity: {e}"))?;
        }
        let report = json(&path.join("report.json"), 2 * 1024 * 1024)?;
        let expected = last.habitat.report();
        for key in [
            "format",
            "schema_version",
            "habitat",
            "oxygen_partial_pressure_pa",
            "co2_partial_pressure_pa",
            "total_pressure_pa",
            "source",
        ] {
            if report[key] != expected[key] {
                return Err(format!(
                    "Recorded report {key} disagrees with final habitat"
                ));
            }
        }
        for key in [
            "electrolysis_source",
            "sabatier_source",
            "moisture_source",
            "breach_source",
            "heat_accounting",
            "crop_source",
            "radiation_source",
        ] {
            if report[key] != expected[key] {
                return Err(format!(
                    "Recorded report {key} disagrees with final habitat"
                ));
            }
        }
        // Diagnostics are additive derived evidence. Old studies have no such key.
        if report.get("diagnostics").is_some() && report["diagnostics"] != expected["diagnostics"] {
            return Err("Recorded diagnostics disagree with the completed habitat interval".into());
        }
        // Previous 38-/49-column studies remain valid. New columns are additive; compare
        // every existing cell to the same shared formatter rather than silently trust CSV.
        let actual = read(&path.join("samples.csv"), 16 * 1024 * 1024)?;
        let expected = habitat::samples_csv(&rows);
        let a = actual.lines().collect::<Vec<_>>();
        let e = expected.lines().collect::<Vec<_>>();
        if a.len() != e.len() {
            return Err("CSV row count disagrees with JSON observations".into());
        }
        let header = a
            .first()
            .ok_or("Missing CSV header")?
            .split(',')
            .collect::<Vec<_>>();
        if !matches!(header.len(), 38 | 43 | 49 | 54 | 66 | 71 | 85 | 90 | 110 | 112 | 114 | 115 | 116 | 117 | 119 | 120 | 121 | 122 | 124 | 126) {
            return Err("Unsupported habitat CSV columns".into());
        }
        for (a, e) in a.iter().zip(&e) {
            let av = a.split(',').collect::<Vec<_>>();
            let ev = e.split(',').collect::<Vec<_>>();
            if av.len() != header.len() || av != ev[..header.len()] {
                return Err("CSV values disagree with JSON observations".into());
            }
        }
        let mut summary = serde_json::json!({"format":"cosmogon-habitat-recording-inspection","schema_version":1,"model":habitat::MODEL,"habitat_id":id,"name":definition.name,"body":definition.body,"latitude_deg":definition.latitude_deg,"longitude_east_deg":definition.longitude_east_deg,"sample_count":count,"initial_time_s":initial.time,"final_time_s":final_world.time,"duration_s":final_world.time-initial.time,"initial_state_fingerprint":initial_fp,"final_state_fingerprint":final_fp,"initial_dataset_versions":save::data_versions(&initial),"final_dataset_versions":save::data_versions(&final_world),"electrolysis_source":m["electrolysis_source"],"sabatier_source":m["sabatier_source"],"moisture_source":m["moisture_source"],"checks":"Bounded formats, sources/models, row budgets/identity/mass/time, exact endpoint worlds/fingerprints, original definition, report and CSV/JSON consistency","validation_scope":"Structural numerical consistency; no independent trajectory replay, cryptographic authenticity, predictive confidence or field calibration","metrics":Metric::for_habitat(&first.habitat).map(|v|serde_json::json!({"id":v.id(),"label":v.label(),"unit":v.unit()})).collect::<Vec<_>>()});
        if definition.record_heat {
            summary["heat_accounting_source"] = m["heat_accounting_source"].clone();
        }
        if uses_breach {
            summary["breach_source"] = m["breach_source"].clone();
        }
        if uses_crop {summary["crop_source"]=m["crop_source"].clone();summary["crop_model_descriptor"]=m["crop_model_descriptor"].clone();}
        if uses_radiation {summary["radiation_source"]=m["radiation_source"].clone();summary["radiation_model_descriptor"]=m["radiation_model_descriptor"].clone();}
        Ok(Self { rows, summary })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    #[default]
    Water,
    OxygenReserve,
    Hydrogen,
    Methane,
    RawWater,
    Temperature,
    OxygenPressure,
    Co2Pressure,
    Battery,
    DeliveredPower,
    UnservedPower,
    ElectrolysisOxygen,
    SabatierWater,
    SabatierHeat,
    Vapor,
    Condensate,
    Humidity,
    CondensationHeat,
    TotalPressure,
    EscapedGas,
    BreachLoss,
    BreachCooling,
    HeatBus,
    HeatBattery,
    HeatMetabolic,
    HeatElectrolysis,
    HeatSabatier,
    HeatCrewVapor,
    HeatCondensation,
    HeatRadiator,
    HeatConduction,
    HeatBreach,
    HeatStorage,
    HeatPaths,
    HeatResidual,
    HeatExecuted,
    HeatExcluded,
    CropProgress,
    CropNutrient,
    CropStandingEdible,
    CropStandingInedible,
    CropHarvestedEdible,
    CropHarvestedInedible,
    CropCo2,
    CropMetabolicWater,
    CropVapor,
    CropOxygen,
    CropLight,
    CropAuxiliary,
    CropChemical,
    HeatCropChemical,
    HeatCropVapor,
    RadiationExteriorDoseRate,
    RadiationInteriorDoseRate,
    RadiationShelterDoseRate,
    RadiationCumulativeDose,
}
impl Metric {
    pub const ALL: [Self; 56] = [
        Self::Water,
        Self::OxygenReserve,
        Self::Hydrogen,
        Self::Methane,
        Self::RawWater,
        Self::Temperature,
        Self::OxygenPressure,
        Self::Co2Pressure,
        Self::Battery,
        Self::DeliveredPower,
        Self::UnservedPower,
        Self::ElectrolysisOxygen,
        Self::SabatierWater,
        Self::SabatierHeat,
        Self::Vapor,
        Self::Condensate,
        Self::Humidity,
        Self::CondensationHeat,
        Self::TotalPressure,
        Self::EscapedGas,
        Self::BreachLoss,
        Self::BreachCooling,
        Self::HeatBus,
        Self::HeatBattery,
        Self::HeatMetabolic,
        Self::HeatElectrolysis,
        Self::HeatSabatier,
        Self::HeatCrewVapor,
        Self::HeatCondensation,
        Self::HeatRadiator,
        Self::HeatConduction,
        Self::HeatBreach,
        Self::HeatStorage,
        Self::HeatPaths,
        Self::HeatResidual,
        Self::HeatExecuted,
        Self::HeatExcluded,
        Self::CropProgress,
        Self::CropNutrient,
        Self::CropStandingEdible,
        Self::CropStandingInedible,
        Self::CropHarvestedEdible,
        Self::CropHarvestedInedible,
        Self::CropCo2,
        Self::CropMetabolicWater,
        Self::CropVapor,
        Self::CropOxygen,
        Self::CropLight,
        Self::CropAuxiliary,
        Self::CropChemical,
        Self::HeatCropChemical,
        Self::HeatCropVapor,
        Self::RadiationExteriorDoseRate,
        Self::RadiationInteriorDoseRate,
        Self::RadiationShelterDoseRate,
        Self::RadiationCumulativeDose,
    ];
    /// Saved catalog identity is immutable, including existing heat-enabled designs.
    /// New heat/crop metrics are explicit selections, never silently added to aggregate analyses.
    pub fn for_habitat(h: &habitat::Habitat) -> impl Iterator<Item = Self> {
        let used = h.uses_breach();
        let radiation = h.uses_radiation();
        Self::ALL.into_iter().filter(move |m| {
            if matches!(
                m,
                Self::RadiationExteriorDoseRate
                    | Self::RadiationInteriorDoseRate
                    | Self::RadiationShelterDoseRate
                    | Self::RadiationCumulativeDose
            ) {
                return radiation;
            }
            !m.is_heat()
                && !m.is_crop()
                && (used
                    || !matches!(
                        m,
                        Self::TotalPressure
                            | Self::EscapedGas
                            | Self::BreachLoss
                            | Self::BreachCooling
                    ))
        })
    }
    /// Explicit selections keep existing saved metric catalogs unchanged.
    pub fn is_crop(self) -> bool {
        matches!(self, Self::CropProgress | Self::CropNutrient | Self::CropStandingEdible | Self::CropStandingInedible | Self::CropHarvestedEdible | Self::CropHarvestedInedible | Self::CropCo2 | Self::CropMetabolicWater | Self::CropVapor | Self::CropOxygen | Self::CropLight | Self::CropAuxiliary | Self::CropChemical)
    }
    pub fn has_crop_context(self) -> bool {
        self.is_crop() || matches!(self, Self::HeatCropChemical | Self::HeatCropVapor)
    }
    fn crop_value(self, row: &Sample) -> Option<f64> {
        let c = crate::habitat_crop_trajectory::retained(row)?;
        let value = match self {
            Self::CropProgress => c.source_progress_days,
            Self::CropNutrient => c.nutrient_equivalent_kg,
            Self::CropStandingEdible => c.standing_dry_kg[0],
            Self::CropStandingInedible => c.standing_dry_kg[1],
            Self::CropHarvestedEdible => c.harvested_dry_kg[0],
            Self::CropHarvestedInedible => c.harvested_dry_kg[1],
            Self::CropCo2 => c.co2_consumed_kg,
            Self::CropMetabolicWater => c.metabolic_water_consumed_kg,
            Self::CropVapor => c.vapor_generated_kg,
            Self::CropOxygen => c.oxygen_generated_kg,
            Self::CropLight => c.light_electric_j,
            Self::CropAuxiliary => c.auxiliary_electric_j,
            Self::CropChemical => c.chemical_energy_j(),
            _ => return None,
        };
        value.is_finite().then_some(value)
    }
    pub fn is_heat(self) -> bool {
        matches!(
            self,
            Self::HeatBus
                | Self::HeatBattery
                | Self::HeatMetabolic
                | Self::HeatElectrolysis
                | Self::HeatSabatier
                | Self::HeatCrewVapor
                | Self::HeatCondensation
                | Self::HeatRadiator
                | Self::HeatConduction
                | Self::HeatBreach
                | Self::HeatStorage
                | Self::HeatPaths
                | Self::HeatResidual
                | Self::HeatExecuted
                | Self::HeatExcluded
                | Self::HeatCropChemical
                | Self::HeatCropVapor
        )
    }
    fn heat_value(self, row: &Sample) -> Option<f64> {
        let ledger = row.habitat.heat.as_ref()?;
        ledger.validate(row.habitat.last_time).ok()?;
        let value = match self {
            Self::HeatCropChemical => ledger.crop_storage_debits_j?[0],
            Self::HeatCropVapor => ledger.crop_storage_debits_j?[1],
            Self::HeatBus => ledger.terms_j[0],
            Self::HeatBattery => ledger.terms_j[1],
            Self::HeatMetabolic => ledger.terms_j[2],
            Self::HeatElectrolysis => ledger.terms_j[3],
            Self::HeatSabatier => ledger.terms_j[4],
            Self::HeatCrewVapor => ledger.terms_j[5],
            Self::HeatCondensation => ledger.terms_j[6],
            Self::HeatRadiator => ledger.terms_j[7],
            Self::HeatConduction => ledger.terms_j[8],
            Self::HeatBreach => ledger.terms_j[9],
            Self::HeatStorage => ledger.terms_j[10],
            Self::HeatPaths => ledger.heat_path_sum_j(),
            Self::HeatResidual => ledger.heat_path_sum_j() - ledger.terms_j[10],
            Self::HeatExecuted => ledger.executed_seconds,
            Self::HeatExcluded => {
                ledger.environment_missing_seconds + ledger.start_temperature_unsupported_seconds
            }
            _ => return None,
        };
        value.is_finite().then_some(value)
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::CropProgress => "crop_progress",
            Self::CropNutrient => "crop_nutrient",
            Self::CropStandingEdible => "crop_standing_edible",
            Self::CropStandingInedible => "crop_standing_inedible",
            Self::CropHarvestedEdible => "crop_harvested_edible",
            Self::CropHarvestedInedible => "crop_harvested_inedible",
            Self::CropCo2 => "crop_co2",
            Self::CropMetabolicWater => "crop_metabolic_water",
            Self::CropVapor => "crop_vapor",
            Self::CropOxygen => "crop_oxygen",
            Self::CropLight => "crop_light",
            Self::CropAuxiliary => "crop_auxiliary",
            Self::CropChemical => "crop_chemical",
            Self::HeatCropChemical => "heat_crop_chemical",
            Self::HeatCropVapor => "heat_crop_vapor",
            Self::HeatBus => "heat_bus",
            Self::HeatBattery => "heat_battery",
            Self::HeatMetabolic => "heat_metabolic",
            Self::HeatElectrolysis => "heat_electrolysis",
            Self::HeatSabatier => "heat_sabatier",
            Self::HeatCrewVapor => "heat_crew_vapor",
            Self::HeatCondensation => "heat_condensation",
            Self::HeatRadiator => "heat_radiator",
            Self::HeatConduction => "heat_conduction",
            Self::HeatBreach => "heat_breach",
            Self::HeatStorage => "heat_storage",
            Self::HeatPaths => "heat_paths",
            Self::HeatResidual => "heat_residual",
            Self::HeatExecuted => "heat_executed",
            Self::HeatExcluded => "heat_excluded",

            Self::TotalPressure => "total_pressure",
            Self::EscapedGas => "escaped_gas",
            Self::BreachLoss => "breach_loss",
            Self::BreachCooling => "breach_cooling",
            Self::Water => "water",
            Self::OxygenReserve => "oxygen_reserve",
            Self::Hydrogen => "hydrogen",
            Self::Methane => "methane",
            Self::RawWater => "raw_water",
            Self::Temperature => "temperature",
            Self::OxygenPressure => "oxygen_pressure",
            Self::Co2Pressure => "co2_pressure",
            Self::Battery => "battery",
            Self::DeliveredPower => "delivered_power",
            Self::UnservedPower => "unserved_power",
            Self::ElectrolysisOxygen => "electrolysis_oxygen",
            Self::SabatierWater => "sabatier_water",
            Self::SabatierHeat => "sabatier_heat",
            Self::Vapor => "vapor",
            Self::Condensate => "condensate",
            Self::Humidity => "humidity",
            Self::CondensationHeat => "condensation_heat",
            Self::RadiationExteriorDoseRate => "radiation_exterior_dose_rate",
            Self::RadiationInteriorDoseRate => "radiation_interior_dose_rate",
            Self::RadiationShelterDoseRate => "radiation_shelter_dose_rate",
            Self::RadiationCumulativeDose => "radiation_cumulative_dose",
        }
    }
    pub fn parse(id: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.id() == id)
            .ok_or_else(|| format!("Unknown habitat recording metric {id}"))
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::CropProgress => "Supported crop reference progress",
            Self::CropNutrient => "Retained crop nutrient-equivalent",
            Self::CropStandingEdible => "Standing edible dry crop",
            Self::CropStandingInedible => "Standing inedible dry crop",
            Self::CropHarvestedEdible => "Harvested edible dry crop",
            Self::CropHarvestedInedible => "Harvested inedible dry crop",
            Self::CropCo2 => "Cumulative crop CO₂ consumed",
            Self::CropMetabolicWater => "Cumulative crop metabolic water consumed",
            Self::CropVapor => "Cumulative crop vapor generated",
            Self::CropOxygen => "Cumulative surrogate crop oxygen",
            Self::CropLight => "Cumulative crop lighting electricity",
            Self::CropAuxiliary => "Cumulative crop auxiliary electricity",
            Self::CropChemical => "Retained dry crop chemical storage",
            Self::HeatCropChemical => "Cumulative crop chemical storage debit",
            Self::HeatCropVapor => "Cumulative crop vapor storage debit",
            Self::HeatBus => "Cumulative bus heat",
            Self::HeatBattery => "Cumulative conversion heat",
            Self::HeatMetabolic => "Cumulative metabolic heat",
            Self::HeatElectrolysis => "Cumulative electrolysis debit",
            Self::HeatSabatier => "Cumulative reduction heat",
            Self::HeatCrewVapor => "Cumulative crew vapor debit",
            Self::HeatCondensation => "Cumulative condensation heat",
            Self::HeatRadiator => "Cumulative radiator debit",
            Self::HeatConduction => "Cumulative signed conduction",
            Self::HeatBreach => "Cumulative breach cooling debit",
            Self::HeatStorage => "Cumulative interval sensible storage",
            Self::HeatPaths => "Cumulative signed heat paths",
            Self::HeatResidual => "Cumulative heat minus storage residual",
            Self::HeatExecuted => "Cumulative executed model time",
            Self::HeatExcluded => "Cumulative excluded model time",

            Self::TotalPressure => "Total cabin pressure",
            Self::EscapedGas => "Cumulative escaped cabin gas",
            Self::BreachLoss => "Supported interval gas escape",
            Self::BreachCooling => "Supported interval equivalent cooling",
            Self::Water => "Water stock",
            Self::OxygenReserve => "Oxygen reserve",
            Self::Hydrogen => "Isolated hydrogen",
            Self::Methane => "Methane product",
            Self::RawWater => "Raw liquid product",
            Self::Temperature => "Cabin temperature",
            Self::OxygenPressure => "Oxygen partial pressure",
            Self::Co2Pressure => "CO₂ partial pressure",
            Self::Battery => "Battery energy",
            Self::DeliveredPower => "Served bus power",
            Self::UnservedPower => "Unserved bus power",
            Self::ElectrolysisOxygen => "Interval electrolytic oxygen",
            Self::SabatierWater => "Interval raw water produced",
            Self::SabatierHeat => "Interval reference reaction heat",
            Self::Vapor => "Cabin water vapor",
            Self::Condensate => "Isolated condensate",
            Self::Humidity => "Relative humidity",
            Self::CondensationHeat => "Interval reference condensation heat",
            Self::RadiationExteriorDoseRate => "Exterior radiation dose rate",
            Self::RadiationInteriorDoseRate => "Cabin interior dose rate",
            Self::RadiationShelterDoseRate => "Storm-shelter dose rate",
            Self::RadiationCumulativeDose => "Cumulative interior dose",
        }
    }
    pub fn unit(self) -> &'static str {
        if self.is_heat() {
            return if matches!(self, Self::HeatExecuted | Self::HeatExcluded) {
                "s"
            } else {
                "J"
            };
        }
        match self {
            Self::CropProgress => "days",
            Self::CropLight | Self::CropAuxiliary | Self::CropChemical => "J",
            Self::Temperature => "K",
            Self::OxygenPressure | Self::Co2Pressure | Self::TotalPressure => "Pa",
            Self::Battery => "kWh",
            Self::DeliveredPower | Self::UnservedPower => "W",
            Self::SabatierHeat | Self::CondensationHeat | Self::BreachCooling => "kJ",
            Self::RadiationExteriorDoseRate
            | Self::RadiationInteriorDoseRate
            | Self::RadiationShelterDoseRate => "mSv/day",
            Self::RadiationCumulativeDose => "mSv",
            Self::Humidity => "%",
            _ => "kg",
        }
    }
    pub fn is_interval(self) -> bool {
        matches!(
            self,
            Self::DeliveredPower
                | Self::UnservedPower
                | Self::ElectrolysisOxygen
                | Self::SabatierWater
                | Self::SabatierHeat
                | Self::CondensationHeat
                | Self::BreachLoss
                | Self::BreachCooling
        )
    }
    pub fn value(self, row: &Sample) -> Option<f64> {
        if self.is_crop() { return self.crop_value(row); }
        if self.is_heat() {
            return self.heat_value(row);
        }
        let h = &row.habitat;
        let s = &h.state;
        let b = &h.budget;
        Some(match self {
            Self::TotalPressure => s.pressure_pa(&h.config),
            Self::EscapedGas => {
                if !h.uses_breach() {
                    return None;
                }
                s.escaped_gas.unwrap_or_default().mass_kg()
            }
            Self::BreachLoss | Self::BreachCooling => {
                let e = b.breach.as_ref()?;
                if !e.status.supported() {
                    return None;
                }
                if self == Self::BreachLoss {
                    e.escaped.mass_kg()
                } else {
                    e.lumped_cooling_j / 1000.0
                }
            }
            Self::RadiationExteriorDoseRate
            | Self::RadiationInteriorDoseRate
            | Self::RadiationShelterDoseRate => {
                let r = b.radiation.as_ref()?;
                match self {
                    Self::RadiationExteriorDoseRate => r.exterior_dose_rate_msv_day,
                    Self::RadiationInteriorDoseRate => r.interior_dose_rate_msv_day,
                    _ => r.storm_shelter_dose_rate_msv_day,
                }
            }
            Self::RadiationCumulativeDose => {
                if !h.uses_radiation() {
                    return None;
                }
                h.totals.cumulative_dose_msv
            }
            Self::Water => s.potable_water_kg,
            Self::Vapor => {
                if !h.uses_moisture() {
                    return None;
                }
                s.water_vapor_kg
            }
            Self::Condensate => {
                if !h.uses_moisture() {
                    return None;
                }
                s.condensate_kg
            }
            Self::Humidity => {
                if !h.uses_moisture() {
                    return None;
                }
                100.0 * s.relative_humidity(&h.config)?
            }
            Self::CondensationHeat => b.moisture.as_ref()?.condensation_heat_j / 1000.0,
            Self::OxygenReserve => s.oxygen_reserve_kg,
            Self::Hydrogen => s.hydrogen_kg,
            Self::Methane => s.methane_kg,
            Self::RawWater => s.sabatier_water_kg,
            Self::Temperature => s.temperature_k,
            Self::OxygenPressure => s.oxygen_pa(&h.config),
            Self::Co2Pressure => s.co2_pa(&h.config),
            Self::Battery => s.battery_j / 3.6e6,
            Self::DeliveredPower | Self::UnservedPower => {
                if b.seconds <= 0.0 || !b.environment_available || !b.temperature_domain_supported {
                    return None;
                }
                if self == Self::DeliveredPower {
                    b.delivered_w
                } else {
                    b.unserved_w
                }
            }
            Self::ElectrolysisOxygen => b.electrolysis.as_ref()?.oxygen_generated_kg,
            Self::SabatierWater => b.sabatier.as_ref()?.water_generated_kg,
            Self::SabatierHeat => b.sabatier.as_ref()?.reaction_heat_j / 1000.0,
            Self::CropProgress
            | Self::CropNutrient
            | Self::CropStandingEdible
            | Self::CropStandingInedible
            | Self::CropHarvestedEdible
            | Self::CropHarvestedInedible
            | Self::CropCo2
            | Self::CropMetabolicWater
            | Self::CropVapor
            | Self::CropOxygen
            | Self::CropLight
            | Self::CropAuxiliary
            | Self::CropChemical
            | Self::HeatCropChemical
            | Self::HeatCropVapor
            | Self::HeatBus
            | Self::HeatBattery
            | Self::HeatMetabolic
            | Self::HeatElectrolysis
            | Self::HeatSabatier
            | Self::HeatCrewVapor
            | Self::HeatCondensation
            | Self::HeatRadiator
            | Self::HeatConduction
            | Self::HeatBreach
            | Self::HeatStorage
            | Self::HeatPaths
            | Self::HeatResidual
            | Self::HeatExecuted
            | Self::HeatExcluded => unreachable!("Heat metrics are handled above"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sandbox::Edit, BodyRef, Scenario, Universe, UniverseSettings};
    fn bundle(name: &str, reactions: u8) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cosmogon-habitat-reader-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut u = Universe::new(UniverseSettings {
            scenario: Scenario::SolarSystemLab,
            ..Default::default()
        });
        let r = BodyRef {
            system: 0,
            body: u.systems[0].find_body("Mars").unwrap() as u32,
        };
        let mut d = habitat::Definition::at(r, 0.0, 0.0);
        if reactions > 0 {
            d.config.electrolysis = Some(crate::electrolysis::Config::default());
        }
        if reactions > 1 {
            d.config.sabatier = Some(crate::sabatier::Config::default());
        }
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        let initial = u.clone();
        let mut rows = vec![habitat::sample(&u, 0).unwrap()];
        for _ in 0..3 {
            u.advance_by(3600.0);
            rows.push(habitat::sample(&u, 0).unwrap());
        }
        habitat::export(&initial, &u, 0, &rows, &root).unwrap();
        root
    }
    #[test]
    fn valid_optional_and_legacy_columns_reopen_exact_observations_with_unavailable_flows() {
        for n in 0..=2 {
            let root = bundle(&format!("valid-{n}"), n);
            let r = Recording::read(&root).unwrap();
            assert_eq!(r.rows.len(), 4);
            assert_eq!(r.summary["sample_count"], 4);
            assert_eq!(Metric::DeliveredPower.value(&r.rows[0]), None);
            assert_eq!(Metric::Water.value(&r.rows[0]), Some(200.0));
            assert_eq!(Metric::SabatierWater.value(&r.rows[0]), None);
            assert_eq!(Metric::SabatierWater.value(&r.rows[1]).is_some(), n == 2);
            for metric in Metric::ALL {
                assert_eq!(Metric::parse(metric.id()).unwrap(), metric);
                assert!(!metric.unit().is_empty());
            }
            if n < 2 {
                let csv = std::fs::read_to_string(root.join("samples.csv")).unwrap();
                let count = if n == 0 { 38 } else { 49 };
                let old = csv
                    .lines()
                    .map(|row| row.split(',').take(count).collect::<Vec<_>>().join(","))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                std::fs::write(root.join("samples.csv"), old).unwrap();
                assert_eq!(Recording::read(&root).unwrap().rows, r.rows);
            }
            std::fs::remove_dir_all(root).unwrap();
        }
        assert!(Metric::parse("future_metric").is_err());
    }
    #[test]
    fn corrupt_sources_endpoints_reports_csv_and_observations_fail_explicitly() {
        let root = bundle("corrupt", 2);
        for (file, key, value) in [
            ("manifest.json", "schema_version", serde_json::json!(2)),
            ("manifest.json", "sample_count", serde_json::json!(1025)),
            (
                "manifest.json",
                "final_state_fingerprint",
                serde_json::json!("fake"),
            ),
            ("manifest.json", "sabatier_source", Value::Null),
            ("report.json", "total_pressure_pa", serde_json::json!(0)),
            (
                "report.json",
                "diagnostics",
                serde_json::json!({"invented":true}),
            ),
        ] {
            let p = root.join(file);
            let original = std::fs::read_to_string(&p).unwrap();
            let mut v: Value = serde_json::from_str(&original).unwrap();
            v[key] = value;
            std::fs::write(&p, v.to_string()).unwrap();
            assert!(Recording::read(&root).is_err(), "{file} {key}");
            std::fs::write(&p, original).unwrap();
        }
        let p = root.join("samples.json");
        let original = std::fs::read_to_string(&p).unwrap();
        for variant in 0..4 {
            let mut v: Value = serde_json::from_str(&original).unwrap();
            match variant {
                0 => v[1]["simulation_time_s"] = v[0]["simulation_time_s"].clone(),
                1 => v[1]["habitat"]["state"]["potable_water_kg"] = 0.into(),
                2 => v[3]["habitat"]["state"]["battery_j"] = 0.into(),
                _ => v[1]["invented"] = true.into(),
            };
            std::fs::write(&p, v.to_string()).unwrap();
            assert!(Recording::read(&root).is_err(), "variant {variant}");
        }
        std::fs::write(&p, original).unwrap();
        let p = root.join("samples.csv");
        let original = std::fs::read_to_string(&p).unwrap();
        std::fs::write(&p, original.replacen("temperature_k", "invented", 1)).unwrap();
        assert!(Recording::read(&root).is_err());
        std::fs::write(&p, original).unwrap();
        assert!(Recording::read(&root).is_ok());
        let p = root.join("report.json");
        let mut legacy: Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        legacy.as_object_mut().unwrap().remove("diagnostics");
        std::fs::write(p, legacy.to_string()).unwrap();
        assert!(
            Recording::read(&root).is_ok(),
            "Legacy reports without derived diagnostics remain valid"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn snapshot_and_bounded_input_are_distinct_from_trajectories() {
        let root = bundle("snapshot", 0);
        let u =
            save::from_json(&std::fs::read_to_string(root.join("final.cosmo")).unwrap()).unwrap();
        let snap = root.with_extension("snapshot");
        let _ = std::fs::remove_dir_all(&snap);
        habitat::export(&u, &u, 0, &[habitat::sample(&u, 0).unwrap()], &snap).unwrap();
        let r = Recording::read(&snap).unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.summary["duration_s"], 0.0);
        std::fs::write(snap.join("manifest.json"), vec![b' '; 64 * 1024 + 1]).unwrap();
        assert!(Recording::read(&snap).unwrap_err().contains("exceeds"));
        std::fs::remove_dir_all(snap).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
