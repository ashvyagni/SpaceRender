//! Measured regression fixtures, distinct from blanket claims of scientific accuracy.
use crate::{
    astro::{self, dynamics::PhysicsPreset, horizons, sol},
    save, Vec3d,
};
use cosmogon_physics::{
    integrators::rk4::RK4,
    kepler, nbody,
    nbody::{Particle, Settings},
    orbital_elements,
};
use serde::Serialize;
use std::f64::consts::{FRAC_PI_2, LN_2, TAU};

mod reference {
    include!("../../data/horizons_validation_2027.rs");
}
mod reference_mid {
    include!("../../data/horizons_validation_2026.rs");
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationMetric {
    pub quantity: String,
    pub value: f64,
    pub unit: String,
    pub regression_limit: f64,
    pub passes_regression: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct ValidationCase {
    pub id: String,
    pub name: String,
    pub model: String,
    pub inputs: serde_json::Value,
    pub reference: String,
    pub metrics: Vec<ValidationMetric>,
    pub limitations: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    pub physics_engine: String,
    pub cases: Vec<ValidationCase>,
    pub passes_regression: bool,
    pub limitations: Vec<String>,
}
fn metric(quantity: impl Into<String>, value: f64, unit: &str, limit: f64) -> ValidationMetric {
    ValidationMetric {
        quantity: quantity.into(),
        value,
        unit: unit.into(),
        regression_limit: limit,
        passes_regression: value.is_finite() && value <= limit,
    }
}

/// Per-body acceptance guard for heliocentric position residuals (km), sized to
/// ~10× the largest observed residual across the mid-year and 365-day endpoint
/// samples (build-59 historical acceptance guards, not uncertainties).
fn jpl_limit_km(name: &str) -> f64 {
    match name {
        "Mercury" => 1000.0,
        "Venus" => 50.0,
        "Earth" => 6000.0,
        "Mars" => 30.0,
        "Jupiter" => 2.0,
        "Saturn" => 5.0,
        "Uranus" => 5.0,
        "Neptune" => 5.0,
        "Moon" => 8000.0,
        _ => 8000.0,
    }
}

/// Fixtures use local copies and no external download or current live-universe state.
pub fn validation_report() -> ValidationReport {
    let mut cases = Vec::new();
    let gm = astro::G * astro::SOLAR_MASS;
    let radius = astro::AU;
    let speed = (gm / radius).sqrt();
    let period = TAU * radius * (radius / gm).sqrt();
    for steps in [1000, 2000] {
        let mut particles = vec![
            Particle::new(Vec3d::ZERO, Vec3d::ZERO, gm, 0.0),
            Particle::new(
                Vec3d::new(radius, 0.0, 0.0),
                Vec3d::new(0.0, speed, 0.0),
                0.0,
                0.0,
            ),
        ];
        let dt = period / steps as f64;
        let energy0 = 0.5 * speed * speed - gm / radius;
        let h0 = radius * speed;
        for k in 0..steps {
            nbody::step(&mut particles, k as f64 * dt, dt, &Settings::default());
        }
        let p = particles[1];
        let energy1 = 0.5 * p.vel.length_squared() - gm / p.pos.length();
        cases.push(ValidationCase{id:format!("circular-orbit-{steps}"),name:format!("Circular test-particle orbit · {steps} steps"),model:"Newtonian point-mass, Yoshida-4, relativity off".into(),
            inputs:serde_json::json!({"central_gm_m3_s2":gm,"initial_radius_m":radius,"initial_speed_m_s":speed,"period_s":period,"macro_steps":steps,"dt_s":dt,"eta":Settings::default().eta,"particle_count":2,"orbiting_particle_gm":0,"collision_radii_m":0}),
            reference:"Analytic circular two-body solution; no observed ephemeris fitted".into(),
            metrics:vec![metric("return_position_error",(p.pos-Vec3d::new(radius,0.0,0.0)).length()/radius,"fraction of initial radius",if steps==2000{1e-8}else{1e-7}),metric("specific_energy_change",(energy1-energy0).abs()/energy0.abs(),"fraction of |initial specific energy|",if steps==2000{1e-12}else{1e-13}),metric("specific_angular_momentum_change",(p.pos.cross(p.vel).length()-h0).abs()/h0,"fraction of initial specific angular momentum",if steps==2000{1e-12}else{1e-13})],
            limitations:"A simple fixed-mass circular fixture. These regression limits do not bound eccentric, encounter, many-body or dissipative regimes. Numerical agreement does not validate missing physical forces.".into()});
    }
    // Elliptical two-body fixture: e = 0.2, a = 1 AU, starting at periapsis (M = 0).
    let e_orb = 0.2;
    let a_orb = radius;
    let ell_period = TAU * a_orb.powf(1.5) / gm.sqrt();
    let (r0, v0) = kepler::orbital_state_vectors(a_orb, e_orb, 0.0, 0.0, 0.0, 0.0, gm);
    let r_quarter =
        kepler::orbital_state_vectors(a_orb, e_orb, 0.0, 0.0, 0.0, FRAC_PI_2, gm).0;
    let ell_energy0 = 0.5 * v0.length_squared() - gm / r0.length();
    let ell_h0 = r0.cross(v0).length();
    let mut elliptical_return_errors = [0.0_f64; 2];
    for (idx, steps) in [1000, 2000].into_iter().enumerate() {
        let mut particles = vec![
            Particle::new(Vec3d::ZERO, Vec3d::ZERO, gm, 0.0),
            Particle::new(r0, v0, 0.0, 0.0),
        ];
        let dt = ell_period / steps as f64;
        let mut quarter_pos = None;
        for k in 0..steps {
            nbody::step(&mut particles, k as f64 * dt, dt, &Settings::default());
            if k + 1 == steps / 4 {
                quarter_pos = Some(particles[1].pos);
            }
        }
        let p = particles[1];
        let return_err = (p.pos - r0).length() / a_orb;
        elliptical_return_errors[idx] = return_err;
        let quarter_err =
            (quarter_pos.expect("quarter-period capture") - r_quarter).length() / a_orb;
        let energy1 = 0.5 * p.vel.length_squared() - gm / p.pos.length();
        let h1 = p.pos.cross(p.vel).length();
        let a_final =
            orbital_elements::state_vectors_to_elements(p.pos, p.vel, gm).semi_major_axis;
        cases.push(ValidationCase{id:format!("elliptical-orbit-{steps}"),name:format!("Elliptical test-particle orbit (e=0.2) · {steps} steps"),model:"Newtonian point-mass, Yoshida-4, relativity off".into(),
            inputs:serde_json::json!({"central_gm_m3_s2":gm,"semi_major_axis_m":a_orb,"eccentricity":e_orb,"initial_mean_anomaly_rad":0.0,"initial_radius_m":r0.length(),"period_s":ell_period,"macro_steps":steps,"dt_s":dt,"eta":Settings::default().eta,"particle_count":2,"orbiting_particle_gm":0,"collision_radii_m":0}),
            reference:"Analytic Kepler two-body solution via kepler::orbital_state_vectors; no observed ephemeris fitted".into(),
            metrics:vec![metric("return_position_error",return_err,"fraction of semi-major axis",if steps==2000{1e-8}else{1e-7}),metric("quarter_period_position_error",quarter_err,"fraction of semi-major axis",if steps==2000{1e-8}else{1e-7}),metric("specific_energy_change",(energy1-ell_energy0).abs()/ell_energy0.abs(),"fraction of |initial specific energy|",1e-13),metric("specific_angular_momentum_change",(h1-ell_h0).abs()/ell_h0,"fraction of initial specific angular momentum",if steps==2000{1e-14}else{1e-13}),metric("semi_major_axis_drift",(a_final-a_orb).abs()/a_orb,"fraction of semi-major axis",1e-13)],
            limitations:"A fixed-mass elliptical fixture at e = 0.2 starting at periapsis. These regression limits do not bound high-eccentricity, encounter, many-body or dissipative regimes. Numerical agreement does not validate missing physical forces.".into()});
        if steps == 2000 {
            let (e1000, e2000) = (elliptical_return_errors[0], elliptical_return_errors[1]);
            let order_dev = if e1000.is_finite() && e2000.is_finite() && e1000 > 0.0 && e2000 > 0.0 {
                ((e1000 / e2000).ln() / LN_2 - 4.0).abs()
            } else {
                f64::NAN
            };
            cases.last_mut().expect("elliptical-2000 case").metrics.push(metric(
                "convergence_order_deviation_from_4",
                order_dev,
                "|observed order − 4|",
                0.01,
            ));
        }
    }
    // Independent-integrator cross-check: classical RK4 (20 000 steps) vs Yoshida-4
    // (2 000 composite steps) on the same elliptical Kepler potential.
    {
        let state0 = [r0.x, r0.y, r0.z, v0.x, v0.y, v0.z];
        let f = |s: &[f64]| -> Vec<f64> {
            let r = Vec3d::new(s[0], s[1], s[2]);
            let v = Vec3d::new(s[3], s[4], s[5]);
            let d3 = r.length_squared().sqrt() * r.length_squared();
            let acc = r * (-gm / d3);
            vec![v.x, v.y, v.z, acc.x, acc.y, acc.z]
        };
        let dt_rk = ell_period / 20_000.0;
        let mut state = state0.to_vec();
        for _ in 0..20_000 {
            state = RK4::integrate(&state, dt_rk, f);
        }
        let rk4_pos = Vec3d::new(state[0], state[1], state[2]);
        let dt_y = ell_period / 2_000.0;
        let mut ps = vec![
            Particle::new(Vec3d::ZERO, Vec3d::ZERO, gm, 0.0),
            Particle::new(r0, v0, 0.0, 0.0),
        ];
        for k in 0..2_000 {
            nbody::step(&mut ps, k as f64 * dt_y, dt_y, &Settings::default());
        }
        let cross_err = (rk4_pos - ps[1].pos).length() / a_orb;
        cases.push(ValidationCase{id:"integrator-cross-check-rk4-vs-yoshida".into(),name:"Elliptical orbit · classical RK4 (20 000 steps) vs Yoshida-4 (2 000 steps)".into(),model:"Same Newtonian Kepler potential integrated by two independent algorithms".into(),
            inputs:serde_json::json!({"central_gm_m3_s2":gm,"semi_major_axis_m":a_orb,"eccentricity":e_orb,"period_s":ell_period,"rk4_steps":20_000usize,"rk4_dt_s":dt_rk,"yoshida_steps":2_000usize,"yoshida_dt_s":dt_y,"relativity":false}),
            reference:"Two independent fourth-order integrators of the same two-body problem; agreement cross-checks the algorithm implementations, not physical truth".into(),
            metrics:vec![metric("integrator_cross_position_error",cross_err,"fraction of semi-major axis",1e-8)],
            limitations:"Both integrators live in this codebase and share the same potential evaluation, so agreement cannot detect a shared force-law bug. One full period only; no encounter, many-body or dissipative coverage.".into()});
    }
    // Solar-system validation against cached JPL positions at a mid-year sample and
    // the 365-day endpoint, plus Newtonian conservation diagnostics across the interval.
    {
        let mut sys = sol::sol_system();
        let preset = PhysicsPreset::Accurate;
        let epoch = horizons::apply_to_sol(&mut sys, preset.settings());
        let mid = epoch + 181.0 * 86_400.0;
        let end = epoch + 365.0 * 86_400.0;
        let (ps_epoch, _) = sys.particles(epoch);
        let energy_epoch = nbody::energy(&ps_epoch);
        let am_epoch = nbody::angular_momentum(&ps_epoch);
        sys.sync_to(mid);
        let sun_mid = sys.star_local_position(mid);
        let mut mid_metrics = Vec::new();
        for (name, km) in reference_mid::HELIO_2026 {
            if let Some(body) = sys.find_body(name) {
                let actual = (sys.body_local_position(body, mid) - sun_mid) / 1000.0;
                let expected = Vec3d::new(km[0], km[1], km[2]);
                mid_metrics.push(metric(
                    format!("{name} heliocentric position residual (181-day mid)"),
                    (actual - expected).length(),
                    "km",
                    jpl_limit_km(name),
                ));
            }
        }
        sys.sync_to(end);
        let sun_end = sys.star_local_position(end);
        let mut end_metrics = Vec::new();
        for (name, km) in reference::HELIO_2027 {
            if let Some(body) = sys.find_body(name) {
                let actual = (sys.body_local_position(body, end) - sun_end) / 1000.0;
                let expected = Vec3d::new(km[0], km[1], km[2]);
                end_metrics.push(metric(
                    format!("{name} heliocentric position residual (365-day endpoint)"),
                    (actual - expected).length(),
                    "km",
                    jpl_limit_km(name),
                ));
            }
        }
        let interval_max = mid_metrics
            .iter()
            .chain(end_metrics.iter())
            .map(|m| m.value)
            .fold(0.0_f64, f64::max);
        let (ps_end, _) = sys.particles(end);
        let energy_end = nbody::energy(&ps_end);
        let am_end = nbody::angular_momentum(&ps_end);
        let energy_drift = (energy_end - energy_epoch).abs() / energy_epoch.abs();
        let am_drift = (am_end - am_epoch).length() / am_epoch.length();
        let mut metrics = mid_metrics;
        metrics.extend(end_metrics);
        metrics.push(metric("interval_max_position_residual_km", interval_max, "km", 8000.0));
        metrics.push(metric("relative_specific_energy_drift", energy_drift, "fraction of |energy at epoch|", 1e-7));
        metrics.push(metric("relative_angular_momentum_drift", am_drift, "fraction of |angular momentum at epoch|", 1e-8));
        let data = horizons::Dataset::embedded();
        cases.push(ValidationCase{id:"solar-lab-horizons-181-365-days".into(),name:"Solar System · mid-year and endpoint against cached JPL positions".into(),model:"Accurate N-body / rails hybrid, central-body 1PN enabled".into(),inputs:serde_json::json!({"initial_epoch":data.dataset.epoch,"initial_epoch_jd_tdb":data.dataset.epoch_jd_tdb,"initial_dataset":data.id(),"retrieved_utc":data.dataset.retrieved_utc,"provider":data.dataset.provider,"attribution":data.dataset.attribution,
            "initial_vectors":data.body.iter().map(|r|serde_json::json!({"name":r.name,"horizons_id":r.horizons_id,"ephemeris":r.ephemeris,"barycentric_position_km":r.position_km,"barycentric_velocity_km_s":r.velocity_km_s})).collect::<Vec<_>>(),
            "reference_heliocentric_positions_km_mid":reference_mid::HELIO_2026.iter().map(|(name,position)|serde_json::json!({"name":name,"position_km":position})).collect::<Vec<_>>(),
            "reference_heliocentric_positions_km":reference::HELIO_2027.iter().map(|(name,position)|serde_json::json!({"name":name,"position_km":position})).collect::<Vec<_>>(),
            "reference_epoch_mid":"2026-07-01 00:00 TDB","reference_epoch":"2027-01-01 00:00 TDB","mid_duration_s":181.0*86400.0,"duration_s":365.0*86400.0,"mid_offset_note":"181 days is half a day past the true 182.5-day midpoint","preset":"Accurate","steps_per_orbit":preset.settings().steps_per_orbit,"relativity":true,"frame":"heliocentric positions, ecliptic J2000 orientation","reference_file_mid":"data/horizons_validation_2026.rs","reference_file":"data/horizons_validation_2027.rs"}),
        reference:"https://ssd.jpl.nasa.gov/horizons/manual.html · locally cached 2026-07-01 mid-year and 2027-01-01 endpoint positions, fetched by tools/fetch_horizons.py".into(),metrics,
        limitations:"Two discrete epochs (181-day mid-year and 365-day endpoint) compared with rounded locally cached reference coordinates; the mid-year sample sits half a day after the true 182.5-day midpoint. No covariance/error propagation and no continuous interval-wide bound. Giant-planet states use system barycentres. Missing J2, tides and small-body perturbations matter; the Earth–Moon residual is not independently attributed. Energy and angular-momentum drift are Newtonian diagnostics evaluated on a 1PN configuration, so their thresholds are deliberately loose acceptance guards only. Regression thresholds are historical acceptance guards, not uncertainties or a research-precision target.".into()});
    }
    let passes = cases
        .iter()
        .all(|c| !c.metrics.is_empty() && c.metrics.iter().all(|m| m.passes_regression));
    ValidationReport{format:"cosmogon-model-validation".into(),schema_version:1,app_version:env!("CARGO_PKG_VERSION").into(),physics_engine:save::PHYSICS_ENGINE.into(),cases,passes_regression:passes,limitations:vec!["Covers only these explicitly described orbital fixtures. Climate, stellar, collision and civilization models are not validated by this report.".into(),"Results are calculated at request time using the installed engine. They do not certify every sandbox, mission planning, observational reduction or long-term accuracy.".into(),"No measured uncertainty/covariance is available for these exported cached reference fixtures.".into()]}
}

impl ValidationReport {
    /// New file only, so a previous scientific report cannot be silently replaced.
    pub fn export(&self, path: &std::path::Path) -> Result<(), String> {
        use std::io::Write;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_contain_computed_residuals_and_reproducible_conditions() {
        let a = validation_report();
        let b = validation_report();
        let failing: Vec<String> = a
            .cases
            .iter()
            .flat_map(|c| {
                c.metrics
                    .iter()
                    .filter(|m| !m.passes_regression)
                    .map(|m| {
                        format!(
                            "{} / {}: {} > {}",
                            c.id, m.quantity, m.value, m.regression_limit
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            a.passes_regression,
            "failing validation metrics: {failing:?}"
        );
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(a.cases.len(), 6);
        // Circular: return-position error shrinks with finer steps.
        assert!(a.cases[0].metrics[0].value > a.cases[1].metrics[0].value);
        // Elliptical: return-position error shrinks with finer steps.
        assert!(a.cases[2].metrics[0].value > a.cases[3].metrics[0].value);
        // Elliptical-2000 carries the observed convergence-order deviation.
        assert!(a.cases[3].metrics.iter().any(|m| {
            m.quantity == "convergence_order_deviation_from_4" && m.passes_regression
        }));
        // Independent-integrator cross-check within tolerance.
        assert_eq!(a.cases[4].metrics[0].quantity, "integrator_cross_position_error");
        assert!(a.cases[4].metrics[0].passes_regression);
        // JPL case: mid + endpoint residuals, interval max and conservation diagnostics.
        assert_eq!(a.cases[5].metrics.len(), 21);
        assert!(a.cases[5].metrics.iter().any(|m| {
            m.quantity.contains("Earth") && m.quantity.contains("mid") && m.value > 1.0
        }));
        assert!(a.cases[5].metrics.iter().any(|m| {
            m.quantity.contains("Earth")
                && m.quantity.contains("endpoint")
                && m.value > 1.0
        }));
        assert!(a.cases[5].metrics.iter().any(|m| {
            m.quantity == "interval_max_position_residual_km" && m.passes_regression
        }));
        assert!(a.cases[5].metrics.iter().any(|m| {
            m.quantity == "relative_specific_energy_drift" && m.passes_regression
        }));
        assert!(a.cases[5].metrics.iter().any(|m| {
            m.quantity == "relative_angular_momentum_drift" && m.passes_regression
        }));
        assert!(a.cases.iter().all(|c| !c.limitations.is_empty()));
    }
}
