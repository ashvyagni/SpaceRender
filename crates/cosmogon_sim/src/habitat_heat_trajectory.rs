//! Scope of explicitly selected cumulative heat curves; saved catalogs remain immutable.
use crate::{
    habitat::Sample, habitat_comparison::Alignment, habitat_heat, habitat_recording::Recording,
};
use serde_json::{json, Value};

pub const SCOPE: &str = "Cumulative actual model counters from the retained ledger origin. Values include only executed intervals; unchanged counters during excluded time do not mean zero physical heat. Storage sums interval C×ΔT; signed path sum minus storage is numerical accounting, not absolute energy or calibrated thermodynamics. Comparisons require equal aligned origins/completed ends and complete execution coverage. Existing saved aggregate catalogs are unchanged; these metrics are explicitly selected.";

pub fn support(
    a: &Sample,
    b: &Sample,
    alignment: Alignment,
    a_first: f64,
    b_first: f64,
) -> &'static str {
    let (Some(ah), Some(bh)) = (&a.habitat.heat, &b.habitat.heat) else {
        return "unavailable_value";
    };
    if ah.validate(a.habitat.last_time).is_err() || bh.validate(b.habitat.last_time).is_err() {
        return "unavailable_value";
    }
    if alignment.coordinate(ah.origin_time_s, a_first)
        != alignment.coordinate(bh.origin_time_s, b_first)
    {
        return "different_heat_origin";
    }
    if alignment.coordinate(a.habitat.last_time, a_first)
        != alignment.coordinate(b.habitat.last_time, b_first)
    {
        return "different_heat_completed_end";
    }
    if ah.environment_missing_seconds > 0.
        || ah.start_temperature_unsupported_seconds > 0.
        || bh.environment_missing_seconds > 0.
        || bh.start_temperature_unsupported_seconds > 0.
    {
        return "incomplete_heat_coverage";
    }
    if ah.executed_seconds != bh.executed_seconds {
        return "different_heat_execution";
    }
    "matched"
}

pub fn point(row: &Sample) -> Value {
    match &row.habitat.heat {
        Some(h) if h.validate(row.habitat.last_time).is_ok() => {
            json!({"available":true,"origin_world_time_s":h.origin_time_s,"completed_habitat_time_s":row.habitat.last_time,"executed_seconds":h.executed_seconds,"environment_missing_seconds":h.environment_missing_seconds,"start_temperature_unsupported_seconds":h.start_temperature_unsupported_seconds,"complete_execution_coverage":h.environment_missing_seconds==0. && h.start_temperature_unsupported_seconds==0.,"capacity_transitions":h.capacity_transitions})
        }
        _ => {
            json!({"available":false,"reason":"No valid retained cumulative heat ledger; sparse minute budgets cannot supply this history"})
        }
    }
}

pub fn context(recordings: &[&Recording]) -> Value {
    let mut value=json!({"source":habitat_heat::source(),"scope":SCOPE,"series":recordings.iter().enumerate().map(|(i,r)|json!({"recording_index":i,"first_world_time_s":r.rows.first().map(|r|r.simulation_time_s),"observations":r.rows.iter().enumerate().map(|(i,r)|json!({"observation_index":i,"simulation_time_s":r.simulation_time_s,"coverage":point(r)})).collect::<Vec<_>>()})).collect::<Vec<_>>()});
    if recordings.iter().any(|r|r.rows.iter().any(|s|s.habitat.uses_crop())) {value["optional_crop_storage"]=crate::crop_process::source();}
    value
}

pub fn label(row: &Sample) -> String {
    match &row.habitat.heat {
        Some(h) if h.validate(row.habitat.last_time).is_ok() => format!(
            "Heat origin {:.3} s · executed {:.3} s · excluded {:.3} s{}",
            h.origin_time_s,
            h.executed_seconds,
            h.environment_missing_seconds + h.start_temperature_unsupported_seconds,
            if h.environment_missing_seconds + h.start_temperature_unsupported_seconds > 0. {
                " · modeled subset only"
            } else {
                ""
            }
        ),
        _ => "Heat history unavailable: no retained ledger; minute budgets cannot reconstruct it."
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        habitat::{Definition, Environment, Habitat},
        habitat_comparison::{pairs, report},
        habitat_factorial::{self, Secondary},
        habitat_figure::Figure,
        habitat_graph::{Layout, Scale},
        habitat_recording::Metric,
        habitat_sweep::{Parameter, Plan},
        sandbox::Edit,
        BodyRef, Scenario, Universe, UniverseSettings,
    };
    fn recording(origin: f64, skip: bool, enabled: bool) -> Recording {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0., 0.);
        d.record_heat = enabled;
        let mut h = Habitat::new(0, d, origin).unwrap();
        let mut rows = vec![Sample {
            simulation_time_s: origin,
            habitat: h.clone(),
        }];
        for i in 1..=3 {
            h.step(
                origin + i as f64 * 60.,
                if skip && i == 2 {
                    Environment::default()
                } else {
                    Environment {
                        irradiance_w_m2: Some(0.),
                        exterior_temperature_k: Some(200. + i as f64),
                        exterior_dose_rate_msv_day: None,
                    }
                },
            )
            .unwrap();
            rows.push(Sample {
                simulation_time_s: h.last_time,
                habitat: h.clone(),
            });
        }
        Recording {
            rows,
            summary: json!({"identity":"retained test inputs"}),
        }
    }
    #[test]
    fn cumulative_metric_values_preserve_signed_terms_and_immutable_catalogs() {
        let r = recording(1000., false, true);
        let metrics = [
            Metric::HeatBus,
            Metric::HeatBattery,
            Metric::HeatMetabolic,
            Metric::HeatElectrolysis,
            Metric::HeatSabatier,
            Metric::HeatCrewVapor,
            Metric::HeatCondensation,
            Metric::HeatRadiator,
            Metric::HeatConduction,
            Metric::HeatBreach,
            Metric::HeatStorage,
        ];
        let mut bus = 0.;
        let mut conduction = 0.;
        let mut storage = 0.;
        let mut radiator = 0.;
        let mut previous_temperature = r.rows[0].habitat.state.temperature_k;
        for row in r.rows.iter().skip(1) {
            bus += row.habitat.budget.delivered_w * 60.;
            conduction += row.habitat.budget.conduction_w * 60.;
            storage += row.habitat.config.heat_capacity_j_k
                * (row.habitat.state.temperature_k - previous_temperature);
            previous_temperature = row.habitat.state.temperature_k;
            radiator -= row.habitat.budget.radiator_rejection_w * 60.;
            assert_eq!(Metric::HeatBus.value(row), Some(bus));
            assert_eq!(Metric::HeatConduction.value(row), Some(conduction));
            assert_eq!(Metric::HeatRadiator.value(row), Some(radiator));
            assert!((Metric::HeatStorage.value(row).unwrap() - storage).abs() < 1e-6);
            let terms = row.habitat.heat.as_ref().unwrap().terms_j;
            for (i, m) in metrics.iter().enumerate() {
                assert_eq!(m.value(row), Some(terms[i]));
                assert_eq!(m.unit(), "J");
                assert!(!m.is_interval());
            }
            assert_eq!(Metric::HeatPaths.value(row), Some(terms[..10].iter().sum()));
            assert_eq!(
                Metric::HeatResidual.value(row),
                Some(terms[..10].iter().sum::<f64>() - terms[10])
            );
            assert_eq!(
                Metric::HeatExecuted.value(row),
                Some(row.habitat.last_time - 1000.)
            );
            assert_eq!(Metric::HeatExcluded.value(row), Some(0.));
        }
        let legacy = recording(1000., false, false);
        assert_eq!(
            Metric::for_habitat(&r.rows[0].habitat).collect::<Vec<_>>(),
            Metric::for_habitat(&legacy.rows[0].habitat).collect::<Vec<_>>()
        );
        assert!(Metric::for_habitat(&r.rows[0].habitat).all(|m| !m.is_heat()));
        for m in Metric::ALL.into_iter().filter(|m| m.is_heat() && !m.has_crop_context()) {
            assert_eq!(Metric::parse(m.id()).unwrap(), m);
            assert_eq!(m.value(&legacy.rows[3]), None);
            assert_eq!(m.value(&r.rows[0]), Some(0.));
        }
    }
    #[test]
    fn heat_comparisons_require_actual_origins_completed_ends_and_complete_coverage() {
        let a = recording(1000., false, true);
        let b = recording(2000., false, true);
        assert!(pairs(&a, &b, Metric::HeatBus, Alignment::Elapsed)
            .iter()
            .all(|p| p.b_minus_a == Some(0.)));
        let mut different = b.clone();
        // A real accumulated ledger began one minute earlier than the first retained observation.
        different.rows.remove(0);
        let p = pairs(&a, &different, Metric::HeatBus, Alignment::Elapsed);
        assert_eq!(p[0].support, "different_heat_origin");
        assert_eq!(p[0].b_minus_a, None);
        assert!(p[0].b_value.is_some());
        let partial = recording(1000., true, true);
        let p = pairs(&a, &partial, Metric::HeatBus, Alignment::World);
        assert_eq!(p[2].support, "incomplete_heat_coverage");
        assert!(p[2].a_value.is_some() && p[2].b_value.is_some());
        assert_eq!(p[2].b_minus_a, None);
        assert_eq!(
            pairs(&partial, &partial, Metric::HeatBus, Alignment::World)[3].support,
            "incomplete_heat_coverage"
        );
        let mut late = a.clone();
        late.rows[1].habitat.last_time -= 1.;
        late.rows[1].habitat.heat.as_mut().unwrap().executed_seconds -= 1.;
        assert_eq!(
            pairs(&a, &late, Metric::HeatBus, Alignment::World)[1].support,
            "different_heat_completed_end"
        );
        let legacy = recording(1000., false, false);
        assert_eq!(
            pairs(&a, &legacy, Metric::HeatBus, Alignment::World)[0].support,
            "unavailable_value"
        );
        assert!(
            report(&a, &partial, Metric::HeatBus, Alignment::World)["heat_trajectory_context"]
                ["series"][1]["observations"][2]["coverage"]["complete_execution_coverage"]
                == false
        );
        assert!(report(&a, &partial, Metric::Temperature, Alignment::World)
            .get("heat_trajectory_context")
            .is_none());
    }
    #[test]
    fn figures_retain_original_joules_partial_scope_and_protected_csv_precision() {
        let r = recording(1000., true, true);
        let layout = Layout {
            metric: "heat_conduction".into(),
            scale: Scale::SignedLog,
            ..Default::default()
        };
        let f = Figure::ordinary(&r, None, &layout).unwrap();
        assert_eq!(f.exact_samples[0], r.rows);
        assert_eq!(f.unit, "J");
        assert_eq!(f.series[0].points[2].support, "available_executed_subset");
        assert_eq!(f.series[0].points[1].value, f.series[0].points[2].value);
        assert_eq!(
            f.heat_trajectory_context.as_ref().unwrap()["series"][0]["observations"][3]["coverage"]
                ["executed_seconds"],
            120.
        );
        let root =
            std::env::temp_dir().join(format!("cosmogon-heat-curve-figure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        f.export(&root).unwrap();
        let csv = std::fs::read_to_string(root.join("points.csv")).unwrap();
        for (line, row) in csv.lines().skip(1).zip(&r.rows) {
            assert_eq!(
                line.split(',').nth(6).unwrap().parse::<f64>().unwrap(),
                row.habitat.heat.as_ref().unwrap().terms_j[8]
            );
        }
        assert!(f.export(&root).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("points.csv")).unwrap(),
            csv
        );
        assert!(f.svg().contains("executed subset only"));
        let mut log = layout.clone();
        log.scale = Scale::Log10;
        assert_eq!(
            Figure::ordinary(&r, None, &log)
                .unwrap()
                .plot()
                .supported_count,
            0
        );
        let legacy = recording(1000., false, false);
        let unavailable = Figure::ordinary(&legacy, None, &layout).unwrap();
        assert!(unavailable.series[0]
            .points
            .iter()
            .all(|p| p.value.is_none()));
        let ordinary = Figure::ordinary(&r, None, &Layout::default()).unwrap();
        assert!(serde_json::to_value(ordinary)
            .unwrap()
            .get("heat_trajectory_context")
            .is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn four_actual_heat_responses_preserve_saved_analysis_and_support_gaps() {
        let mut u = Universe::new(UniverseSettings {
            scenario: Scenario::SolarSystemLab,
            ..Default::default()
        });
        let body = BodyRef {
            system: 0,
            body: u.systems[0].find_body("Mars").unwrap() as u32,
        };
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(body, 0., 0.)),
        })
        .unwrap();
        let plan = Plan {
            format: "cosmogon-habitat-sweep-plan".into(),
            schema_version: 2,
            parameter: Parameter::RadiatorAreaM2,
            values: vec![0., 20.],
            samples: 3,
            interval_s: 3600.,
            secondary: Some(Secondary {
                parameter: Parameter::HeatCapacityJK,
                values: [5e6, 1e7],
            }),
            authored_relative_band: None,
        };
        let root =
            std::env::temp_dir().join(format!("cosmogon-heat-curve-design-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        crate::habitat_sweep::execute(&u, 0, &plan, &root).unwrap();
        let mut d = habitat_factorial::read(&root).unwrap();
        let saved = d.analysis.clone();
        let points = d.curve(Metric::HeatRadiator).unwrap();
        for (i, p) in points.iter().enumerate() {
            let y: [f64; 4] = std::array::from_fn(|b| {
                d.recordings[b].rows[i]
                    .habitat
                    .heat
                    .as_ref()
                    .unwrap()
                    .terms_j[7]
            });
            assert_eq!(p.contrasts.responses, y.map(Some));
            assert_eq!(p.contrasts.a_at_b0, Some(y[1] - y[0]));
            assert_eq!(
                p.contrasts.difference_in_differences,
                Some((y[3] - y[2]) - (y[1] - y[0]))
            );
        }
        assert!(!saved["metrics"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.starts_with("heat_")));
        assert_eq!(d.analysis, saved);
        let f = Figure::paired(
            &d,
            &Layout {
                metric: "heat_radiator".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(f.analysis.as_ref().unwrap()["final_analysis"], saved);
        assert_eq!(f.exact_samples.len(), 4);
        assert_eq!(
            f.heat_trajectory_context.as_ref().unwrap()["series"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        let mut h = d.recordings[2].rows[0].habitat.clone();
        let mut partial = d.recordings[2].clone();
        let start = h.last_time;
        for i in 1..=120 {
            h.step(
                start + i as f64 * 60.,
                if i > 60 {
                    Environment::default()
                } else {
                    Environment {
                        irradiance_w_m2: Some(0.),
                        exterior_temperature_k: Some(220.),
                        exterior_dose_rate_msv_day: None,
                    }
                },
            )
            .unwrap();
            if i == 60 {
                partial.rows[1] = Sample {
                    simulation_time_s: start + 3600.,
                    habitat: h.clone(),
                };
            }
        }
        partial.rows[2] = Sample {
            simulation_time_s: start + 7200.,
            habitat: h,
        };
        d.recordings[2] = partial;
        let points = d.curve(Metric::HeatRadiator).unwrap();
        assert_eq!(points[2].contrasts.average_a, None);
        assert!(points[2].contrasts.responses[2].is_some());
        // The independent saved source/analysis remains untouched after view derivation.
        assert_eq!(habitat_factorial::read(&root).unwrap().analysis, saved);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn reader_refuses_a_reset_ledger_even_with_individually_valid_rows() {
        let mut u = Universe::new(UniverseSettings {
            scenario: Scenario::SolarSystemLab,
            ..Default::default()
        });
        let body = BodyRef {
            system: 0,
            body: u.systems[0].find_body("Mars").unwrap() as u32,
        };
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(body, 0., 0.)),
        })
        .unwrap();
        let initial = u.clone();
        let mut rows = vec![crate::habitat::sample(&u, 0).unwrap()];
        for _ in 0..3 {
            u.advance_by(3600.);
            rows.push(crate::habitat::sample(&u, 0).unwrap());
        }
        let root = std::env::temp_dir().join(format!(
            "cosmogon-heat-curve-continuity-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        crate::habitat::export(&initial, &u, 0, &rows, &root).unwrap();
        let before = Recording::read(&root).unwrap();
        rows[1].habitat.heat = Some(habitat_heat::Ledger::new(
            rows[1].habitat.last_time,
            rows[1].habitat.config.heat_capacity_j_k,
        ));
        rows[1]
            .habitat
            .heat
            .as_ref()
            .unwrap()
            .validate(rows[1].habitat.last_time)
            .unwrap();
        std::fs::write(
            root.join("samples.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
        assert!(Recording::read(&root)
            .unwrap_err()
            .contains("heat continuity"));
        assert_eq!(before.rows[3], rows[3]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
