//! Scope of actual finite crop trajectories; no source forecast replaces retained state.
use crate::{
    crop_process, habitat::Sample, habitat_comparison::Alignment, habitat_recording::Recording,
};
use serde_json::{json, Value};

pub const SCOPE: &str = "Actual retained finite reference-process state. Supported source progress is not biological age or chronological elapsed time. Pauses do not predict survival or reversible stress. Maintained chamber conditions and chemical energy are user assumptions. Original daily curves are interpolated by the installed model; graph lines are only guides between stored observations. Harvest moves dry stores and supplies no nominal crew diet; only an explicit bounded store conversion moves harvested edible dry material 1:1 into the habitat dry-food tank, and inedible material never becomes food. A runtime greenhouse installation appears as an explicit origin: leading observations may predate it, after which crop state never disappears or changes origin. Comparisons require aligned installation origins and completed habitat ends; differing original assumptions remain explicit engineering scenarios, not calibrated biology. Existing saved aggregate catalogs remain unchanged.";

/// Full source/curve/material validation occurs when the recording is read. This fast
/// projection does not repeat source quadrature on every graph frame.
pub fn retained(row: &Sample) -> Option<&crop_process::State> {
    let c = row.habitat.state.crop.as_ref()?;
    (c.model == crop_process::MODEL
        && c.chemistry_model == crate::crop_chemistry::MODEL
        && c.atomic_model == crate::crop_chemistry::ATOMIC_MODEL
        && c.source_snapshot_sha256 == crate::greenhouse::SOURCE_SHA256
        && c.recipe_snapshot_sha256 == crate::greenhouse_supplies::SOURCE_SHA256
        && c.origin_time_s.is_finite()
        && row.habitat.last_time.is_finite()
        && c.origin_time_s <= row.habitat.last_time)
        .then_some(c)
}

/// Cross-observation invariants supplement each row's full source/material validation.
/// Structural checks cannot prove that all intervening simulation was executed.
/// Leading rows without crop state are a legitimate pre-installation span; from the
/// first installed observation onward, state, origin and counters stay consistent.
pub fn validate_series(rows: &[Sample]) -> Result<(), String> {
    let Some(first) = rows.first() else {
        return Err("Empty crop trajectory".into());
    };
    if first.habitat.state.crop.is_some() && retained(first).is_none() {
        return Err("Crop state lacks valid pins inside a retained trajectory".into());
    }
    let mut previous: Option<&crop_process::State> = None;
    for row in rows {
        if row.habitat.state.crop.is_none() {
            if previous.is_some() {
                return Err("Crop state disappears inside a retained trajectory".into());
            }
            continue;
        }
        let now = retained(row).ok_or("Crop state lacks valid pins inside a retained trajectory")?;
        if let Some(previous) = previous {
            if now.origin_time_s != previous.origin_time_s {
                return Err("Crop installation origin changes inside a retained trajectory".into());
            }
            let counters = |c: &crop_process::State| {
                [
                    c.source_progress_seconds,
                    c.source_progress_days,
                    c.co2_consumed_kg,
                    c.metabolic_water_consumed_kg,
                    c.vapor_generated_kg,
                    c.oxygen_generated_kg,
                    c.light_electric_j,
                    c.auxiliary_electric_j,
                    c.dry_mass_kg() + c.food_stored_kg,
                ]
            };
            if counters(now)
                .into_iter()
                .zip(counters(previous))
                .any(|(a, b)| a < b)
                || now.nutrient_equivalent_kg > previous.nutrient_equivalent_kg
                || now.food_stored_kg < previous.food_stored_kg
                || previous.harvested && !now.harvested
            {
                return Err("Crop counters or harvest reverse inside a retained trajectory".into());
            }
        }
        previous = Some(now);
    }
    Ok(())
}

pub fn support(
    a: &Sample,
    b: &Sample,
    alignment: Alignment,
    a_first: f64,
    b_first: f64,
) -> &'static str {
    let (Some(ac), Some(bc)) = (retained(a), retained(b)) else {
        return "unavailable_value";
    };
    if alignment.coordinate(ac.origin_time_s, a_first)
        != alignment.coordinate(bc.origin_time_s, b_first)
    {
        return "different_crop_origin";
    }
    if alignment.coordinate(a.habitat.last_time, a_first)
        != alignment.coordinate(b.habitat.last_time, b_first)
    {
        return "different_crop_completed_end";
    }
    "matched"
}

pub fn point(row: &Sample) -> Value {
    let Some(c) = retained(row) else {
        return json!({"available":false,"reason":"No retained installed crop state; planner forecasts cannot supply actual crop evidence"});
    };
    json!({"available":true,"origin_world_time_s":c.origin_time_s,"completed_habitat_time_s":row.habitat.last_time,"elapsed_since_installation_s":row.habitat.last_time-c.origin_time_s,"supported_source_progress_days":c.source_progress_days,"harvested":c.harvested,"original_source_crop":c.source_crop,"original_source_curve":c.source_curve,"dry_chemical_energy_j_kg":c.dry_chemical_energy_j_kg,"initial_nutrient_equivalent_kg":c.initial_nutrient_equivalent_kg,"current_crop_configuration":row.habitat.config.crop,"last_interval_crop_configuration":row.habitat.budget.configuration.as_ref().and_then(|cfg|cfg.crop.as_ref()),"last_interval_crop_budget":row.habitat.budget.crop})
}

pub fn context(recordings: &[&Recording]) -> Value {
    json!({"source":crop_process::source(),"scope":SCOPE,"series":recordings.iter().enumerate().map(|(i,r)|json!({"recording_index":i,"first_world_time_s":r.rows.first().map(|s|s.simulation_time_s),"observations":r.rows.iter().enumerate().map(|(j,s)|json!({"observation_index":j,"simulation_time_s":s.simulation_time_s,"retained_crop":point(s)})).collect::<Vec<_>>()})).collect::<Vec<_>>()})
}

pub fn label(row: &Sample) -> String {
    match retained(row) {
        Some(c) => format!(
            "Crop progress {:.6} d · elapsed {:.3} d{} · {}",
            c.source_progress_days,
            (row.habitat.last_time - c.origin_time_s) / crate::time::SECONDS_PER_DAY,
            if c.harvested { " · harvested" } else { "" },
            match &row.habitat.config.crop {
                Some(config) if config.assume_maintained_conditions =>
                    "chamber maintained by assumption",
                Some(_) => "chamber assumption off",
                None => "disabled; retained state",
            }
        ),
        None => {
            "Actual crop evidence unavailable: no installed state; planner grants no stocks.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        habitat::{Environment, Habitat},
        habitat_comparison::{pairs, report},
        habitat_factorial::{self as factorial, Secondary},
        habitat_figure::Figure,
        habitat_graph::{Layout, Scale},
        habitat_recording::Metric,
        habitat_sweep::{Parameter, Plan},
    };
    fn recording(origin: f64, enabled: bool, maintained: bool) -> Recording {
        let mut d = crop_process::definition_at(crate::BodyRef { system: 0, body: 0 }, 0., 0.);
        if !enabled {
            d.config.crop = None;
            d.initial.crop_nutrient_equivalent_kg = 0.;
        }
        if let Some(c) = &mut d.config.crop {
            c.assume_maintained_conditions = maintained;
        }
        d.config.crew = 0;
        d.config.base_load_w = 0.;
        d.config.scrubber_power_w = 0.;
        d.config.recycler_power_w = 0.;
        d.config.scrubber_rate_kg_s = 0.;
        d.config.recycler_rate_kg_s = 0.;
        d.config.heater_limit_w = 0.;
        d.config.radiator_pump_w = 0.;
        d.config.radiator_area_m2 = 0.;
        d.config.conductance_w_k = 0.;
        d.config.firm_power_w = 5000.;
        d.config.heat_capacity_j_k = 1e9;
        d.initial.battery_j = 0.;
        let m = d.config.moisture.as_mut().unwrap();
        m.power_limit_w = 0.;
        m.rate_kg_s = 0.;
        let mut h = Habitat::new(0, d, origin).unwrap();
        let mut rows = vec![Sample {
            simulation_time_s: origin,
            habitat: h.clone(),
        }];
        for n in 1..=2 * 1440 {
            h.step(
                origin + n as f64 * 60.,
                Environment {
                    irradiance_w_m2: Some(0.),
                    exterior_temperature_k: Some(210.),
                    exterior_dose_rate_msv_day: None,
                },
            )
            .unwrap();
            if n % 1440 == 0 {
                h.validate(h.last_time).unwrap();
                rows.push(Sample {
                    simulation_time_s: h.last_time,
                    habitat: h.clone(),
                });
            }
        }
        Recording {
            rows,
            summary: json!({"source":"actual finite crop fixture"}),
        }
    }
    #[test]
    fn actual_crop_metrics_keep_absence_zero_and_supported_progress_distinct() {
        let active = recording(1000., true, true);
        let off = recording(1000., true, false);
        let legacy = recording(1000., false, false);
        let c = active.rows[2].habitat.state.crop.as_ref().unwrap();
        assert_eq!(Metric::CropProgress.value(&active.rows[2]), Some(2.));
        assert!(c.standing_dry_kg[0] > 0.);
        assert_eq!(
            Metric::CropChemical.value(&active.rows[2]),
            Some(c.dry_mass_kg() * 20e6)
        );
        assert!(
            (Metric::CropCo2.value(&active.rows[2]).unwrap()
                - (1.82 * c.standing_dry_kg[0] + 1.68 * c.standing_dry_kg[1]))
                .abs()
                < 1e-12
        );
        assert!(
            (Metric::CropLight.value(&active.rows[2]).unwrap() - 1600. * 2. * 57600.).abs() < 1e-4
        );
        assert!(
            (Metric::HeatCropChemical.value(&active.rows[2]).unwrap() + c.chemical_energy_j())
                .abs()
                < 1e-6
        );
        assert!(
            (Metric::HeatCropVapor.value(&active.rows[2]).unwrap()
                + c.vapor_generated_kg * crate::moisture::REFERENCE_PHASE_J_KG)
                .abs()
                < 1e-6
        );
        for m in Metric::ALL.into_iter().filter(|m| m.has_crop_context()) {
            assert_eq!(Metric::parse(m.id()).unwrap(), m);
            assert_eq!(m.value(&legacy.rows[2]), None);
            assert!(!m.is_interval());
            let initial = m.value(&active.rows[0]).unwrap();
            assert_eq!(initial, if m == Metric::CropNutrient { 1. } else { 0. });
            assert_eq!(m.value(&off.rows[2]), Some(initial));
        }
        assert_eq!(
            Metric::for_habitat(&active.rows[0].habitat).collect::<Vec<_>>(),
            Metric::for_habitat(&legacy.rows[0].habitat).collect::<Vec<_>>()
        );
        assert!(Metric::for_habitat(&active.rows[0].habitat).all(|m| !m.has_crop_context()));
        let mut disabled = active.clone();
        disabled.rows[2].habitat.config.crop = None;
        assert_eq!(
            Metric::CropStandingEdible.value(&disabled.rows[2]),
            Some(c.standing_dry_kg[0])
        );
        assert_eq!(point(&off.rows[2])["supported_source_progress_days"], 0.);
        assert!(label(&off.rows[2]).contains("assumption off"));
        assert!(label(&disabled.rows[2]).contains("disabled; retained"));
        assert_eq!(point(&off.rows[2])["elapsed_since_installation_s"], 172800.);
    }
    #[test]
    fn crop_differences_need_aligned_installation_and_completed_clocks() {
        let a = recording(1000., true, true);
        let b = recording(2000., true, false);
        let p = pairs(&a, &b, Metric::CropProgress, Alignment::Elapsed);
        assert_eq!(p[2].b_minus_a, Some(-2.));
        assert!(pairs(&a, &b, Metric::CropProgress, Alignment::World)
            .iter()
            .all(|p| p.support == "no_exact_timestamp"));
        let mut shifted = b.clone();
        for s in &mut shifted.rows {
            s.habitat.state.crop.as_mut().unwrap().origin_time_s -= 60.;
        }
        assert_eq!(
            pairs(&a, &shifted, Metric::CropProgress, Alignment::Elapsed)[2].support,
            "different_crop_origin"
        );
        let mut late = b.clone();
        late.rows[2].habitat.last_time -= 1.;
        let p = pairs(&a, &late, Metric::CropNutrient, Alignment::Elapsed);
        assert_eq!(p[2].support, "different_crop_completed_end");
        assert!(p[2].a_value.is_some() && p[2].b_value.is_some());
        assert_eq!(p[2].b_minus_a, None);
        let missing = recording(1000., false, false);
        assert_eq!(
            pairs(&a, &missing, Metric::CropProgress, Alignment::World)[0].support,
            "unavailable_value"
        );
        let comparison = report(&a, &b, Metric::CropProgress, Alignment::Elapsed);
        assert_eq!(
            comparison["crop_trajectory_context"]["series"][1]["observations"][2]["retained_crop"]
                ["current_crop_configuration"]["assume_maintained_conditions"],
            false
        );
        assert!(report(&a, &b, Metric::Water, Alignment::Elapsed)
            .get("crop_trajectory_context")
            .is_none());
    }
    #[test]
    fn crop_figures_retain_precise_raw_values_original_context_and_scale_omissions() {
        let r = recording(1000., true, true);
        let missing = recording(1000., false, false);
        let layout = Layout {
            metric: "crop_chemical".into(),
            scale: Scale::Log10,
            ..Default::default()
        };
        let f = Figure::ordinary(&r, Some(&missing), &layout).unwrap();
        assert_eq!(f.exact_samples, vec![r.rows.clone(), missing.rows.clone()]);
        assert_eq!(f.series[0].points[0].value, Some(0.));
        assert_eq!(f.series[1].points[2].value, None);
        assert!(
            f.crop_trajectory_context.as_ref().unwrap()["series"][0]["observations"][0]
                ["retained_crop"]["original_source_curve"]
                .is_array()
        );
        let root =
            std::env::temp_dir().join(format!("cosmogon-crop-curve-figure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        f.export(&root).unwrap();
        let csv = std::fs::read_to_string(root.join("points.csv")).unwrap();
        let lines = csv.lines().skip(1).take(3).collect::<Vec<_>>();
        for (line, s) in lines.iter().zip(&r.rows) {
            assert_eq!(
                line.split(',').nth(6).unwrap().parse::<f64>().unwrap(),
                s.habitat.state.crop.as_ref().unwrap().chemical_energy_j()
            );
        }
        assert!(lines[0].ends_with("scale_omitted"));
        assert!(f.svg().contains("not biological age"));
        assert!(f.export(&root).is_err());
        let old = Figure::ordinary(&r, None, &Layout::default()).unwrap();
        assert!(serde_json::to_value(old)
            .unwrap()
            .get("crop_trajectory_context")
            .is_none());
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn selected_paired_crop_contrasts_preserve_original_catalogs_and_support_gaps() {
        let a = recording(1000., true, true);
        let b = recording(1000., true, false);
        let plan = Plan {
            format: "cosmogon-habitat-sweep-plan".into(),
            schema_version: 2,
            parameter: Parameter::FirmPowerW,
            values: vec![0., 5000.],
            samples: 3,
            interval_s: 86400.,
            secondary: Some(Secondary {
                parameter: Parameter::HeatCapacityJK,
                values: [1e8, 1e9],
            }),
            authored_relative_band: None,
        };
        let mut recordings = vec![b.clone(), a.clone(), b, a];
        let report = factorial::report(&plan, &recordings).unwrap();
        assert!(report["metrics"].get("crop_progress").is_none());
        let curve = factorial::curve(&plan, &recordings, Metric::CropProgress).unwrap();
        assert_eq!(
            curve[2].contrasts.responses,
            [Some(0.), Some(2.), Some(0.), Some(2.)]
        );
        assert_eq!(curve[2].contrasts.average_a, Some(2.));
        assert_eq!(curve[2].contrasts.standard_ab_effect, Some(0.));
        recordings[3].rows[2]
            .habitat
            .state
            .crop
            .as_mut()
            .unwrap()
            .origin_time_s -= 60.;
        let curve = factorial::curve(&plan, &recordings, Metric::CropProgress).unwrap();
        assert_eq!(curve[2].branch_support[3], "different_crop_origin");
        assert_eq!(curve[2].contrasts.average_a, None);
        assert!(curve[2].contrasts.responses[3].is_some());
        assert_eq!(factorial::report(&plan, &recordings).unwrap(), report);
    }
    #[test]
    fn individually_valid_crop_origin_reset_and_counter_rewind_are_refused() {
        let r = recording(100000., true, true);
        validate_series(&r.rows).unwrap();
        let mut shifted = r.clone();
        shifted.rows[1]
            .habitat
            .state
            .crop
            .as_mut()
            .unwrap()
            .origin_time_s -= 86400.;
        shifted.rows[1]
            .habitat
            .validate(shifted.rows[1].simulation_time_s)
            .unwrap();
        assert!(validate_series(&shifted.rows)
            .unwrap_err()
            .contains("origin"));
        let mut backwards = r.clone();
        let c = backwards.rows[0].habitat.state.crop.clone();
        backwards.rows[2].habitat.state.crop = c;
        assert!(validate_series(&backwards.rows)
            .unwrap_err()
            .contains("reverse"));
        let mut absent = r.clone();
        absent.rows[1].habitat.state.crop = None;
        assert!(validate_series(&absent.rows)
            .unwrap_err()
            .contains("disappears"));
    }
}
