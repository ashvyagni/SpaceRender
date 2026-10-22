//! Headless runner. Useful for tuning models, CI determinism checks and batch experiments.
//!
//! ```text
//! cosmogon-cli run   [--seed N] [--scenario neighbourhood|garden|sol] [--years Y]
//!                    [--life realistic|hopeful|teeming] [--systems N] [--min-importance I]
//!                    [--save PATH]
//! cosmogon-cli check [--seed N] [--scenario ...] [--years Y]   # determinism self-check
//! cosmogon-cli survey [--seeds N] [--scenario ...] [--years Y] [--life ...]  # outcome statistics
//! ```

use std::time::Instant;

use cosmogon_sim::civ::tech::TechGraph;
use cosmogon_sim::time::{format_duration, group_digits, SECONDS_PER_YEAR};
use cosmogon_sim::universe::LIFE_PRESETS;
use cosmogon_sim::{save, Scenario, Universe, UniverseSettings};

struct Args {
    cmd: String,
    samples: usize,
    interval_days: f64,
    target: String,
    output: Option<String>,
    initial: Option<String>,
    recording: Option<String>,
    geographic_start: Option<String>,
    contact_hardware: Option<String>,
    cargo_ledgers: bool,
    handling: bool,
    launch: Option<String>,
    entry: Option<String>,
    landing: Option<String>,
    mission_span: bool,
    transfer: Vec<String>,
    entry_interface_altitude: Option<f64>,
    nose_radius: Option<f64>,
    heat_flux_coefficient: Option<f64>,
    maximum_heat_flux: Option<f64>,
    maximum_total_heat: Option<f64>,
    maximum_touchdown_vertical: Option<f64>,
    maximum_touchdown_horizontal: Option<f64>,
    landing_liftoff_altitude: Option<f64>,
    thrust_time: Option<f64>,
    thrust_newtons: Option<f64>,
    maximum_acceleration: Option<f64>,
    liftoff_altitude: Option<f64>,
    maximum_start_altitude: Option<f64>,
    static_shear: bool,
    operation_window: Option<f64>,
    branches: Option<[usize;2]>,
    observation: Option<usize>,
    reference_index: Option<usize>,
    from_index: Option<usize>,
    reference: Option<String>,
    site: Option<String>,
    mars_terrain: Option<String>,
    habitat_id: Option<u64>,
    nutrient_kg: Option<f64>,
    store_kg: Option<f64>,
    material_kg: Option<f64>,
    from_id: Option<u64>,
    to_id: Option<u64>,
    oxygen_kg: Option<f64>,
    water_kg: Option<f64>,
    food_dry_kg: Option<f64>,
    food_water_kg: Option<f64>,
    from_site: Option<String>,
    to_site: Option<String>,
    latitude: f64,
    longitude: f64,
    definition: Option<String>,
    alignment: String,
    parameter: String,
    sweep_start: Option<f64>,
    sweep_end: Option<f64>,
    sweep_runs: usize,
    band: Option<f64>,
    measure_target: Option<String>,
    measure: Option<String>,
    seed: u64,
    seeds: u64,
    scenario: Scenario,
    years: f64,
    life: usize,
    systems: u32,
    min_importance: u8,
    save: Option<String>,
}

fn number<T: std::str::FromStr>(text: String, flag: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{flag}: invalid number {text}"))
}
fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        cmd: "run".into(),
        samples: 60,
        interval_days: 1.0,
        target: "Earth".into(),
        output: None,
        initial: None,
        recording: None,
        geographic_start: None,
        contact_hardware: None,
        cargo_ledgers: false,
        handling: false,
        launch: None,
        entry: None,
        landing: None,
        mission_span: false,
        transfer: vec![],
        entry_interface_altitude: None,
        nose_radius: None,
        heat_flux_coefficient: None,
        maximum_heat_flux: None,
        maximum_total_heat: None,
        maximum_touchdown_vertical: None,
        maximum_touchdown_horizontal: None,
        landing_liftoff_altitude: None,
        thrust_time: None,
        thrust_newtons: None,
        maximum_acceleration: None,
        liftoff_altitude: None,
        maximum_start_altitude: None,
        static_shear: false,
        operation_window: None,
        branches: None,
        observation: None,
        reference_index: None,
        from_index: None,
        reference: None,
        site: None,
        mars_terrain: None,
        habitat_id: None,
        nutrient_kg: None,
        store_kg: None,
        material_kg: None,
        from_id: None,
        to_id: None,
        oxygen_kg: None,
        water_kg: None,
        food_dry_kg: None,
        food_water_kg: None,
        from_site: None,
        to_site: None,
        latitude: 0.0,
        longitude: 0.0,
        definition: None,
        alignment: "elapsed".into(),
        parameter: "albedo".into(),
        sweep_start: None,
        sweep_end: None,
        sweep_runs: 5,
        band: None,
        measure_target: None,
        measure: None,
        seed: 1,
        seeds: 10,
        scenario: Scenario::Sol,
        years: 200_000.0,
        life: 0,
        systems: 24,
        min_importance: 3,
        save: None,
    };
    let mut it = args;
    let mut explicit_scenario = false;
    let mut explicit_seed = false;
    let mut explicit_target = false;
    let mut explicit_coordinates = false;
    let mut explicit_samples = false;
    let mut explicit_sweep = false;
    if let Some(c) = it.next() {
        a.cmd = c;
        if matches!(a.cmd.as_str(),"habitat"|"habitat-sweep") {a.samples=25;a.interval_days=1.0/24.0;a.scenario=Scenario::SolarSystemLab;}
        if matches!(a.cmd.as_str(),"flight-definition"|"flight-study"|"contact-hardware") {a.scenario=Scenario::SolarSystemLab;a.target="Mars".into();a.samples=121;a.interval_days=1.0/86400.0;}
        if a.cmd=="flight-approach" {a.target.clear();}
        if a.cmd=="habitat-sweep" {a.parameter="firm_power_w".into();}
        if matches!(a.cmd.as_str(),"habitat-definition"|"habitat-crop-definition"|"habitat-setup") {a.scenario=Scenario::SolarSystemLab;a.target="Mars".into();}
        if a.cmd=="launch-definition" {a.scenario=Scenario::SolarSystemLab;a.target="Mars".into();}
        if matches!(a.cmd.as_str(),"entry-definition"|"landing-definition") {a.scenario=Scenario::SolarSystemLab;a.target="Mars".into();}
    }
    if matches!(a.cmd.as_str(), "--help" | "-h" | "help") {
        a.cmd = "help".into();
        return Ok(a);
    }
    while let Some(flag) = it.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            a.cmd = "help".into();
            return Ok(a);
        }
        let mut val = || {
            it.next()
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        if matches!(a.cmd.as_str(),"greenhouse"|"greenhouse-supplies") && !matches!(flag.as_str(),"--definition"|"--output"|"--initial"|"--habitat-id") {return Err(format!("{flag} is not a greenhouse planning option"));}
        if matches!(a.cmd.as_str(),"greenhouse-definition"|"greenhouse-supply-definition") && flag!="--output" {return Err(format!("{flag} is not a greenhouse-definition option"));}
        if matches!(a.cmd.as_str(),"crop-chemistry"|"crop-chemistry-inspect"|"greenhouse-supply-inspect") && !matches!(flag.as_str(),"--recording"|"--output") {return Err(format!("{flag} is not a crop supply/chemistry inspection option"));}
        if matches!(a.cmd.as_str(),"habitat-diagnose"|"habitat-crop-harvest") && !matches!(flag.as_str(),"--initial"|"--habitat-id"|"--output") {return Err(format!("{flag} is not a habitat diagnosis option"));}
        if a.cmd=="habitat-crop-install" && !matches!(flag.as_str(),"--initial"|"--habitat-id"|"--output"|"--definition"|"--nutrient-kg") {return Err(format!("{flag} is not a greenhouse installation option"));}
        if a.cmd=="habitat-crop-store" && !matches!(flag.as_str(),"--initial"|"--habitat-id"|"--output"|"--store-kg") {return Err(format!("{flag} is not a crop food store option"));}
        if a.cmd=="habitat-construction-supply" && !matches!(flag.as_str(),"--initial"|"--habitat-id"|"--output"|"--material-kg") {return Err(format!("{flag} is not a construction material supply option"));}
        if a.cmd=="habitat-stock-transfer" && !matches!(flag.as_str(),"--initial"|"--from-id"|"--to-id"|"--output"|"--oxygen-kg"|"--water-kg"|"--food-dry-kg"|"--food-water-kg") {return Err(format!("{flag} is not a habitat stock transfer option"));}
        if a.cmd=="contact-hardware" && !matches!(flag.as_str(),"--initial"|"--target"|"--seed"|"--scenario"|"--output"|"--geographic-start"|"--static-shear"|"--operation-window") {return Err(format!("{flag} is not a contact hardware option"));}
        if a.cmd=="flight-definition" && !matches!(flag.as_str(),"--initial"|"--definition"|"--output"|"--target"|"--seed"|"--scenario"|"--samples"|"--interval-seconds"|"--geographic-start"|"--contact-hardware"|"--cargo-ledgers"|"--handling"|"--launch"|"--entry"|"--landing"|"--mission-span"|"--transfer") {return Err(format!("{flag} is not a world flight definition option"));}
        if a.cmd=="launch-definition" && !matches!(flag.as_str(),"--initial"|"--output"|"--target"|"--latitude"|"--longitude"|"--seed"|"--scenario"|"--thrust-time"|"--thrust-newtons"|"--maximum-acceleration"|"--liftoff-altitude"|"--maximum-start-altitude") {return Err(format!("{flag} is not a launch-definition option"));}
        if a.cmd=="entry-definition" && !matches!(flag.as_str(),"--initial"|"--output"|"--target"|"--latitude"|"--longitude"|"--seed"|"--scenario"|"--entry-interface-altitude"|"--nose-radius"|"--heat-flux-coefficient"|"--maximum-heat-flux"|"--maximum-total-heat") {return Err(format!("{flag} is not an entry-definition option"));}
        if a.cmd=="landing-definition" && !matches!(flag.as_str(),"--initial"|"--output"|"--target"|"--latitude"|"--longitude"|"--seed"|"--scenario"|"--maximum-touchdown-vertical"|"--maximum-touchdown-horizontal"|"--liftoff-altitude") {return Err(format!("{flag} is not a landing-definition option"));}
        if a.cmd=="flight-study" && !matches!(flag.as_str(),"--initial"|"--definition"|"--output") {return Err(format!("{flag} is not a world flight study option"));}
        if matches!(a.cmd.as_str(),"arrival-start"|"contact-start") && flag!="--output" {return Err(format!("{flag} is not a geographic start option"));}
        if a.cmd=="flight-approach" && !matches!(flag.as_str(),"--recording"|"--target"|"--output") {return Err(format!("{flag} is not a recorded approach option"));}
        if a.cmd=="flight-rendezvous" && !matches!(flag.as_str(),"--recording"|"--reference"|"--output") {return Err(format!("{flag} is not a flight-rendezvous option"));}
        if a.cmd=="approach-inspect" && flag!="--recording" {return Err(format!("{flag} is not an approach inspection option"));}
        if a.cmd=="support-definition" && flag!="--output" {return Err(format!("{flag} is not a support-definition option"));}
        if a.cmd=="flight-support" && !matches!(flag.as_str(),"--recording"|"--definition"|"--output") {return Err(format!("{flag} is not a recorded support option"));}
        if a.cmd=="support-inspect" && flag!="--recording" {return Err(format!("{flag} is not a support-inspection option"));}

    if a.contact_hardware.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Contact hardware requires a nonempty file".into());}
    if a.cmd=="contact-hardware" && [&a.initial,&a.output].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Contact files require nonempty paths".into());}
    if a.cmd=="flight-inspect" && flag!="--recording" {return Err(format!("{flag} is not a world flight recording inspection option"));}
        if a.cmd=="vehicle-definition" && flag!="--output" {return Err(format!("{flag} is not a vehicle-definition option"));}
        if a.cmd=="vehicle-study" && !matches!(flag.as_str(),"--definition"|"--output") {return Err(format!("{flag} is not a finite vehicle study option"));}
        if a.cmd=="vehicle-inspect" && flag!="--recording" {return Err(format!("{flag} is not a vehicle recording inspection option"));}
        if a.cmd=="cargo-epochs-definition" && flag!="--output" {return Err(format!("{flag} is not a cargo-epochs-definition option"));}
        if a.cmd=="cargo-epochs-study" && !matches!(flag.as_str(),"--definition"|"--output") {return Err(format!("{flag} is not a cargo-epochs study option"));}
        if a.cmd=="cargo-epochs-inspect" && flag!="--recording" {return Err(format!("{flag} is not a cargo-epochs inspection option"));}
    if a.cmd=="habitat-rerun" && !matches!(flag.as_str(),"--recording"|"--definition"|"--output"){return Err(format!("{flag} is not a saved-origin habitat rerun option"));}
        if a.cmd=="habitat-verify" && !matches!(flag.as_str(),"--recording"|"--output") {return Err(format!("{flag} is not a habitat replay verification option"));}
        if a.cmd=="habitat-balance" && !matches!(flag.as_str(),"--recording"|"--from-index"|"--observation"|"--output") {return Err(format!("{flag} is not a recorded balance option"));}
        if a.cmd=="habitat-figure" && !matches!(flag.as_str(),"--recording"|"--reference"|"--definition"|"--output"|"--branches") {return Err(format!("{flag} is not a recorded figure option"));}
        if a.cmd=="habitat-pressure" && !matches!(flag.as_str(),"--recording"|"--observation"|"--reference-index"|"--output") {return Err(format!("{flag} is not a recorded pressure option"));}
        if a.cmd=="habitat-inspect" && !matches!(flag.as_str(),"--recording"|"--observation"|"--output") {return Err(format!("{flag} is not a habitat inspection option"));}
        if a.cmd=="habitat" && !matches!(flag.as_str(),"--definition"|"--habitat-id"|"--initial"|"--output"|"--samples"|"--interval-days"|"--seed"|"--scenario") {return Err(format!("{flag} is not a habitat study option"));}
        if matches!(a.cmd.as_str(),"habitat-definition"|"habitat-crop-definition"|"habitat-setup") && !matches!(flag.as_str(),"--initial"|"--output"|"--target"|"--latitude"|"--longitude"|"--seed"|"--scenario") {return Err(format!("{flag} is not a habitat-definition option"));}
        if a.cmd=="habitat-sweep" && !matches!(flag.as_str(),"--definition"|"--habitat-id"|"--initial"|"--output"|"--samples"|"--interval-days"|"--parameter"|"--from"|"--to"|"--runs"|"--band") {return Err(format!("{flag} is not a habitat sweep option"));}
        if a.cmd=="habitat-compare" && !matches!(flag.as_str(),"--recording"|"--reference"|"--measure"|"--alignment"|"--output") {return Err(format!("{flag} is not a habitat comparison option"));}
        if matches!(flag.as_str(),"--parameter"|"--from"|"--to"|"--runs"|"--samples"|"--interval-days") {explicit_sweep=true;}
        match flag.as_str() {
            "--static-shear" if a.cmd=="contact-hardware" => a.static_shear=true,
            "--operation-window" if a.cmd=="contact-hardware" => {
                let seconds=number::<f64>(val()?,&flag)?;
                if !seconds.is_finite() || !(0.0..=1.0e7).contains(&seconds) {return Err("--operation-window requires a finite window between 0 and 1e7 seconds".into());}
                a.operation_window=Some(seconds);
            },
            "--contact-hardware" if a.cmd=="flight-definition" => a.contact_hardware=Some(val()?),
            "--cargo-ledgers" if a.cmd=="flight-definition" => a.cargo_ledgers=true,
            "--handling" if a.cmd=="flight-definition" => a.handling=true,
            "--launch" if a.cmd=="flight-definition" => a.launch=Some(val()?),
            "--entry" if a.cmd=="flight-definition" => a.entry=Some(val()?),
            "--landing" if a.cmd=="flight-definition" => a.landing=Some(val()?),
            "--mission-span" if a.cmd=="flight-definition" => a.mission_span=true,
            "--transfer" if a.cmd=="flight-definition" => a.transfer.push(val()?),
            "--entry-interface-altitude" if a.cmd=="entry-definition" => {
                let altitude=number::<f64>(val()?,&flag)?;
                if !altitude.is_finite() || !(0.0..=1.0e7).contains(&altitude) {return Err("--entry-interface-altitude requires a finite height between 0 and 1e7 m".into());}
                a.entry_interface_altitude=Some(altitude);
            },
            "--nose-radius" if a.cmd=="entry-definition" => {
                let radius=number::<f64>(val()?,&flag)?;
                if !radius.is_finite() || !(1.0e-3..=1.0e4).contains(&radius) {return Err("--nose-radius requires a finite radius between 0.001 and 10000 m".into());}
                a.nose_radius=Some(radius);
            },
            "--heat-flux-coefficient" if a.cmd=="entry-definition" => {
                let coefficient=number::<f64>(val()?,&flag)?;
                if !coefficient.is_finite() || !(0.0..=1.0e-1).contains(&coefficient) {return Err("--heat-flux-coefficient requires a finite coefficient between 0 and 0.1".into());}
                a.heat_flux_coefficient=Some(coefficient);
            },
            "--maximum-heat-flux" if a.cmd=="entry-definition" => {
                let flux=number::<f64>(val()?,&flag)?;
                if !flux.is_finite() || !(1.0e-30..=1.0e12).contains(&flux) {return Err("--maximum-heat-flux requires a finite bound between 1e-30 and 1e12 W/m²".into());}
                a.maximum_heat_flux=Some(flux);
            },
            "--maximum-total-heat" if a.cmd=="entry-definition" => {
                let heat=number::<f64>(val()?,&flag)?;
                if !heat.is_finite() || !(1.0e-30..=1.0e18).contains(&heat) {return Err("--maximum-total-heat requires a finite bound between 1e-30 and 1e18 J/m²".into());}
                a.maximum_total_heat=Some(heat);
            },
            "--maximum-touchdown-vertical" if a.cmd=="landing-definition" => {
                let speed=number::<f64>(val()?,&flag)?;
                if !speed.is_finite() || !(0.0..=1.0e4).contains(&speed) {return Err("--maximum-touchdown-vertical requires a finite speed between 0 and 1e4 m/s".into());}
                a.maximum_touchdown_vertical=Some(speed);
            },
            "--maximum-touchdown-horizontal" if a.cmd=="landing-definition" => {
                let speed=number::<f64>(val()?,&flag)?;
                if !speed.is_finite() || !(0.0..=1.0e4).contains(&speed) {return Err("--maximum-touchdown-horizontal requires a finite speed between 0 and 1e4 m/s".into());}
                a.maximum_touchdown_horizontal=Some(speed);
            },
            "--liftoff-altitude" if a.cmd=="landing-definition" => {
                let altitude=number::<f64>(val()?,&flag)?;
                if !altitude.is_finite() || !(0.0..=1.0e5).contains(&altitude) {return Err("--liftoff-altitude requires a finite height between 0 and 1e5 m".into());}
                a.landing_liftoff_altitude=Some(altitude);
            },
            "--thrust-time" if a.cmd=="launch-definition" => {
                let seconds=number::<f64>(val()?,&flag)?;
                if !seconds.is_finite() || !(0.0..=1.0e7).contains(&seconds) {return Err("--thrust-time requires a finite duration between 0 and 1e7 seconds".into());}
                a.thrust_time=Some(seconds);
            },
            "--thrust-newtons" if a.cmd=="launch-definition" => {
                let newtons=number::<f64>(val()?,&flag)?;
                if !newtons.is_finite() || !(0.0..=1.0e9).contains(&newtons) {return Err("--thrust-newtons requires a finite thrust between 0 and 1e9 N".into());}
                a.thrust_newtons=Some(newtons);
            },
            "--maximum-acceleration" if a.cmd=="launch-definition" => {
                let accel=number::<f64>(val()?,&flag)?;
                if !accel.is_finite() || !(0.0..=1.0e4).contains(&accel) {return Err("--maximum-acceleration requires a finite bound between 0 and 1e4 m/s²".into());}
                a.maximum_acceleration=Some(accel);
            },
            "--liftoff-altitude" if a.cmd=="launch-definition" => {
                let altitude=number::<f64>(val()?,&flag)?;
                if !altitude.is_finite() || !(0.0..=1.0e5).contains(&altitude) {return Err("--liftoff-altitude requires a finite height between 0 and 1e5 m".into());}
                a.liftoff_altitude=Some(altitude);
            },
            "--maximum-start-altitude" if a.cmd=="launch-definition" => {
                let altitude=number::<f64>(val()?,&flag)?;
                if !altitude.is_finite() || !(0.0..=1.0e5).contains(&altitude) {return Err("--maximum-start-altitude requires a finite height between 0 and 1e5 m".into());}
                a.maximum_start_altitude=Some(altitude);
            },
            "--geographic-start" if matches!(a.cmd.as_str(),"flight-definition"|"contact-hardware") => a.geographic_start=Some(val()?),
            "--samples" => {
                let requested=number::<usize>(val()?,&flag)?;explicit_samples=true;
                if a.cmd=="section" && !(2..=1024).contains(&requested) {return Err("Section samples must be between 2 and 1024".into());}
                if matches!(a.cmd.as_str(),"habitat"|"habitat-sweep") && !(1..=1024).contains(&requested) {return Err("Habitat samples must be between 1 and 1024".into());}
                a.samples=if matches!(a.cmd.as_str(),"habitat"|"habitat-sweep") {requested} else {requested.clamp(2,4096)};
            },
            "--interval-seconds" if a.cmd=="flight-definition" => a.interval_days=number::<f64>(val()?, &flag)?/86400.,
            "--interval-days" => a.interval_days = number(val()?, &flag)?,
            "--target" => { a.target = val()?; explicit_target = true; },
            "--mars-terrain" => {let v=val()?;if !matches!(v.as_str(),"4"|"16") {return Err("--mars-terrain requires 4 or 16 pixels per degree".into());}a.mars_terrain=Some(v);},
            "--site" => a.site = Some(val()?),
            "--from-site" => a.from_site = Some(val()?),
            "--to-site" => a.to_site = Some(val()?),
            "--output" => a.output = Some(val()?),
            "--latitude" => { a.latitude = number(val()?, &flag)?; explicit_coordinates = true; },
            "--longitude" => { a.longitude = number(val()?, &flag)?; explicit_coordinates = true; },
            "--reference" => a.reference = Some(val()?),
            "--branches" => {if a.cmd!="habitat-figure"{return Err("--branches requires habitat-figure with a saved single-setting design".into());}a.branches=Some(cosmogon_sim::habitat_design::parse_branches(&val()?)?);},
            "--recording" => a.recording = Some(val()?),
            "--observation" => {if !matches!(a.cmd.as_str(),"habitat-inspect"|"habitat-pressure"|"habitat-balance") {return Err("--observation requires habitat-inspect or habitat-pressure".into());}let n=number::<usize>(val()?,&flag)?;if n>=1024 {return Err("Observation index must be between 0 and 1023".into());}a.observation=Some(n);},
            "--from-index" => {if a.cmd!="habitat-balance" {return Err("--from-index requires habitat-balance".into());} let n=number::<usize>(val()?,&flag)?;if n>=1024{return Err("Initial index must be between 0 and 1023".into());}a.from_index=Some(n);},
            "--reference-index" => {if a.cmd!="habitat-pressure" {return Err("--reference-index requires habitat-pressure".into());}let n=number::<usize>(val()?,&flag)?;if n>=1024 {return Err("Reference index must be between 0 and 1023".into());}a.reference_index=Some(n);},
            "--initial" => a.initial = Some(val()?),
            "--habitat-id" => {if !matches!(a.cmd.as_str(),"habitat"|"habitat-sweep"|"greenhouse"|"greenhouse-supplies"|"habitat-diagnose"|"habitat-crop-harvest"|"habitat-crop-install"|"habitat-crop-store"|"habitat-construction-supply") {return Err("--habitat-id requires habitat, habitat-sweep, habitat-diagnose, greenhouse, crop or construction edits".into());}a.habitat_id=Some(number(val()?,&flag)?);},
            "--definition" => a.definition = Some(val()?),
            "--nutrient-kg" => {if a.cmd!="habitat-crop-install" {return Err("--nutrient-kg requires habitat-crop-install".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--nutrient-kg requires a finite charge between 0 and 1e7 kg".into());}a.nutrient_kg=Some(v);},
            "--store-kg" => {if a.cmd!="habitat-crop-store" {return Err("--store-kg requires habitat-crop-store".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--store-kg requires a finite request between 0 and 1e7 kg".into());}a.store_kg=Some(v);},
            "--material-kg" => {if a.cmd!="habitat-construction-supply" {return Err("--material-kg requires habitat-construction-supply".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--material-kg requires a finite charge between 0 and 1e7 kg".into());}a.material_kg=Some(v);},
            "--from-id" => {if a.cmd!="habitat-stock-transfer" {return Err("--from-id requires habitat-stock-transfer".into());}a.from_id=Some(number(val()?, &flag)?);},
            "--to-id" => {if a.cmd!="habitat-stock-transfer" {return Err("--to-id requires habitat-stock-transfer".into());}a.to_id=Some(number(val()?, &flag)?);},
            "--oxygen-kg" => {if a.cmd!="habitat-stock-transfer" {return Err("--oxygen-kg requires habitat-stock-transfer".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--oxygen-kg requires a finite amount between 0 and 1e7 kg".into());}a.oxygen_kg=Some(v);},
            "--water-kg" => {if a.cmd!="habitat-stock-transfer" {return Err("--water-kg requires habitat-stock-transfer".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--water-kg requires a finite amount between 0 and 1e7 kg".into());}a.water_kg=Some(v);},
            "--food-dry-kg" => {if a.cmd!="habitat-stock-transfer" {return Err("--food-dry-kg requires habitat-stock-transfer".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--food-dry-kg requires a finite amount between 0 and 1e7 kg".into());}a.food_dry_kg=Some(v);},
            "--food-water-kg" => {if a.cmd!="habitat-stock-transfer" {return Err("--food-water-kg requires habitat-stock-transfer".into());}let v=number::<f64>(val()?,&flag)?;if !v.is_finite() || !(0.0..=1.0e7).contains(&v) {return Err("--food-water-kg requires a finite amount between 0 and 1e7 kg".into());}a.food_water_kg=Some(v);},
            "--alignment" => {if a.cmd!="habitat-compare" {return Err("--alignment requires habitat-compare".into());}a.alignment=val()?;},
            "--parameter" => a.parameter = val()?,
            "--from" => a.sweep_start = Some(number(val()?, &flag)?),
            "--to" => a.sweep_end = Some(number(val()?, &flag)?),
            "--runs" => a.sweep_runs = number(val()?, &flag)?,
            "--band" => {if a.cmd!="habitat-sweep" {return Err("--band requires habitat-sweep".into());}a.band=Some(number(val()?, &flag)?);},
            "--measure-target" => a.measure_target = Some(val()?),
            "--measure" => a.measure = Some(val()?),
            "--seed" => {a.seed = number(val()?, &flag)?;explicit_seed=true;},
            "--seeds" => a.seeds = number(val()?, &flag)?,
            "--years" => a.years = number(val()?, &flag)?,
            "--systems" => a.systems = number(val()?, &flag)?,
            "--min-importance" => a.min_importance = number(val()?, &flag)?,
            "--save" => a.save = Some(val()?),
            "--scenario" => {
                explicit_scenario = true;
                a.scenario = match val()?.as_str() {
                    "neighbourhood" | "neighborhood" => Scenario::Neighbourhood,
                    "garden" => Scenario::GardenWorld,
                    "sol" => Scenario::Sol,
                    "lab" => Scenario::SolarSystemLab,
                    "system" => Scenario::StarSystem,
                    "empty" => Scenario::EmptySystem,
                    name => return Err(format!("Unknown scenario {name}")),
                };
            }
            "--life" => {
                let v = val()?;
                a.life = LIFE_PRESETS
                    .iter()
                    .position(|p| p.0.eq_ignore_ascii_case(&v))
                    .ok_or("Choose life: realistic, hopeful or teeming")?;
            }
            unknown => return Err(format!("Unknown flag {unknown}; use --help")),
        }
    }
if matches!(a.cmd.as_str(),"arrival-start"|"contact-start"|"flight-approach") && a.output.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Arrival output requires a nonempty fresh path".into());}
if a.cmd=="flight-rendezvous" && a.output.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Rendezvous output requires a nonempty fresh path".into());}
if a.cmd=="flight-rendezvous" && (a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) || a.reference.as_deref().is_none_or(|p|p.trim().is_empty())) {return Err("flight-rendezvous requires --recording LEFT_DIR and --reference RIGHT_DIR".into());}
if matches!(a.cmd.as_str(),"flight-support"|"support-inspect") && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("Support commands require --recording COMPLETE_DIR".into());}
if matches!(a.cmd.as_str(),"support-definition"|"flight-support") && [&a.output,&a.definition].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Support files require nonempty paths".into());}
if a.cmd=="flight-approach" && explicit_target && a.target.trim().is_empty() {return Err("Choose a nonempty approach target".into());}
if matches!(a.cmd.as_str(),"flight-approach"|"approach-inspect") && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("Approach commands require --recording COMPLETE_DIR".into());}
if a.geographic_start.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Geographic start path must not be empty".into());}
    if a.contact_hardware.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Contact hardware requires a nonempty file".into());}
    if a.cmd=="contact-hardware" && [&a.initial,&a.output].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Contact files require nonempty paths".into());}
    if a.cmd=="flight-inspect" && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("flight-inspect requires --recording COMPLETE_DIR".into());}
    if matches!(a.cmd.as_str(),"flight-definition"|"flight-study") {
        if [&a.initial,&a.definition,&a.output].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Flight files require nonempty paths".into());}
        if a.cmd=="flight-study" && a.definition.is_none() {return Err("flight-study requires --definition PINNED_STUDY".into());}
        if a.launch.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Launch definition requires a nonempty file".into());}
        if a.cmd=="flight-definition" && a.launch.is_some() && (a.geographic_start.is_some() || a.contact_hardware.is_some() || a.definition.is_some()) {return Err("Choose one --launch definition file or circular/geographic flight options, not both".into());}
        if a.entry.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Entry definition requires a nonempty file".into());}
        if a.landing.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Landing definition requires a nonempty file".into());}
        if a.cmd=="flight-definition" && (a.entry.is_some() || a.landing.is_some()) && !(a.launch.is_some() || a.contact_hardware.is_some()) {return Err("Entry heating and terminal landing require contact gear (--contact-hardware or --launch FILE)".into());}
        if a.cmd=="flight-definition" && a.mission_span && a.launch.is_none() {return Err("Mission span requires an authored launch definition (--launch FILE)".into());}
    }
    if matches!(a.cmd.as_str(),"entry-definition"|"landing-definition") {
        if [&a.initial,&a.output].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Definition files require nonempty paths".into());}
        if a.output.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err(format!("{} requires --output NEW_FILE",a.cmd));}
    }
    if a.cmd=="launch-definition" {
        if [&a.initial,&a.output].iter().any(|p|p.as_deref().is_some_and(|s|s.trim().is_empty())) {return Err("Launch definition files require nonempty paths".into());}
        if a.output.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("launch-definition requires --output NEW_FILE".into());}
        if a.thrust_time.is_none_or(|t| t <= 0.) {return Err("launch-definition requires a positive --thrust-time".into());}
        if a.thrust_newtons.is_none_or(|t| t <= 0.) {return Err("launch-definition requires a positive --thrust-newtons".into());}
    }
        if a.cmd=="vehicle-inspect" && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("vehicle-inspect requires --recording COMPLETE_DIR".into());}
    if matches!(a.cmd.as_str(),"vehicle-definition"|"vehicle-study") && a.output.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Vehicle output requires a nonempty new file/folder".into());}
    if a.cmd=="vehicle-study" && a.definition.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Choose a nonempty vehicle plan file".into());}
    if a.cmd=="cargo-epochs-inspect" && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("cargo-epochs-inspect requires --recording COMPLETE_DIR".into());}
    if matches!(a.cmd.as_str(),"cargo-epochs-definition"|"cargo-epochs-study") && a.output.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Cargo-epochs output requires a nonempty new file/folder".into());}
    if a.cmd=="cargo-epochs-study" && a.definition.as_deref().is_some_and(|p|p.trim().is_empty()) {return Err("Choose a nonempty cargo-epochs plan file".into());}
    if matches!(a.cmd.as_str(),"flight-definition"|"contact-hardware"|"habitat"|"habitat-definition"|"habitat-crop-definition"|"habitat-setup"|"launch-definition"|"entry-definition"|"landing-definition") && a.initial.is_some() && (explicit_scenario || explicit_seed) {return Err("Choose saved --initial world or new --scenario/--seed, not both".into());}
    if a.cmd=="habitat-setup" && a.output.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err("habitat-setup requires --output NEW_FILE".into());}
    if matches!(a.cmd.as_str(),"greenhouse"|"greenhouse-supplies") && a.habitat_id.is_some() && a.initial.is_none() {return Err("Greenhouse habitat context requires --initial saved world".into());}
    if a.cmd=="habitat-sweep" {
        if a.initial.is_none() || a.output.is_none() {return Err("habitat-sweep requires --initial SAVE and --output NEW_DIR".into());}
        if a.definition.is_some() && explicit_sweep {return Err("Choose a saved sweep --definition or parameter/range/sample flags".into());}
        if a.definition.is_none() {habitat_sweep_plan(&a)?;}
    }
    if a.cmd=="habitat-compare" {
        if a.recording.is_none() || a.reference.is_none() {return Err("habitat-compare requires --recording A and --reference B".into());}
        cosmogon_sim::habitat_comparison::Alignment::parse(&a.alignment)?;
        cosmogon_sim::habitat_recording::Metric::parse(a.measure.as_deref().unwrap_or("water"))?;
    }
    if a.cmd=="habitat-crop-harvest" && (a.initial.is_none() || a.habitat_id.is_none() || a.output.is_none()) {return Err("habitat-crop-harvest requires --initial SAVE --habitat-id ID --output NEW_DIR".into());}
    if a.cmd=="habitat-crop-install" && (a.initial.is_none() || a.habitat_id.is_none() || a.definition.is_none() || a.nutrient_kg.is_none() || a.output.is_none()) {return Err("habitat-crop-install requires --initial SAVE --habitat-id ID --definition CROP_CONFIG --nutrient-kg KG --output NEW_DIR".into());}
    if a.cmd=="habitat-crop-store" && (a.initial.is_none() || a.habitat_id.is_none() || a.output.is_none()) {return Err("habitat-crop-store requires --initial SAVE --habitat-id ID --output NEW_DIR".into());}
    if a.cmd=="habitat-construction-supply" && (a.initial.is_none() || a.habitat_id.is_none() || a.material_kg.is_none() || a.output.is_none()) {return Err("habitat-construction-supply requires --initial SAVE --habitat-id ID --material-kg KG --output NEW_DIR".into());}
    if a.cmd=="habitat-stock-transfer" && (a.initial.is_none() || a.from_id.is_none() || a.to_id.is_none() || a.output.is_none() || (a.oxygen_kg.is_none() && a.water_kg.is_none() && a.food_dry_kg.is_none() && a.food_water_kg.is_none())) {return Err("habitat-stock-transfer requires --initial SAVE --from-id ID --to-id ID, at least one species amount flag and --output NEW_DIR".into());}
    if a.cmd=="habitat-diagnose" && (a.initial.is_none() || a.habitat_id.is_none()) {return Err("habitat-diagnose requires --initial SAVE and explicit --habitat-id ID".into());}
    if a.cmd=="habitat-rerun" && (a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) || a.output.as_deref().is_none_or(|p|p.trim().is_empty())){return Err("habitat-rerun requires --recording COMPLETE_DESIGN --output NEW_DIR".into());}
    if a.cmd=="habitat-verify" && a.recording.is_none() {return Err("habitat-verify requires --recording COMPLETE_PAIRED_FOLDER".into());}
    if a.cmd=="habitat-pressure" && (a.recording.is_none() || a.observation.is_none()) {return Err("habitat-pressure requires --recording ORDINARY_BRANCH_FOLDER and --observation INDEX".into());}
    if a.cmd=="habitat-balance" && a.recording.is_none() {return Err("habitat-balance requires --recording FOLDER".into());}
    if a.cmd=="habitat-balance" && a.from_index.zip(a.observation).is_some_and(|(a,b)|a>b) {return Err("Choose ordered balance indices".into());}
    if a.cmd=="habitat-figure" && (a.recording.is_none() || a.definition.is_none() || a.output.is_none()) {return Err("habitat-figure requires --recording FOLDER --definition GRAPH_LAYOUT --output NEW_DIR".into());}
    if matches!(a.cmd.as_str(),"crop-chemistry"|"crop-chemistry-inspect"|"greenhouse-supply-inspect") && a.recording.as_deref().is_none_or(|p|p.trim().is_empty()) {return Err(format!("{} requires --recording FILE_OR_FOLDER",a.cmd));}
    if a.cmd=="habitat-inspect" && a.recording.is_none() {return Err("habitat-inspect requires --recording STUDY_FOLDER".into());}
    if a.cmd=="habitat" {
        if a.definition.is_none() && a.initial.is_none() {return Err("habitat requires --definition FILE or --initial saved world".into());}
        if a.definition.is_some() && a.habitat_id.is_some() {return Err("Choose a new --definition or existing --habitat-id".into());}
        if !a.interval_days.is_finite() || a.interval_days<1.0/86400.0 || a.interval_days>30.0 || a.interval_days*(a.samples.saturating_sub(1)) as f64>30.0 {return Err("Habitat studies require a positive finite interval and at most 30 days total".into());}
    }
    if a.mars_terrain.is_some() && !matches!(a.cmd.as_str(),"terrain"|"surface"|"section") {return Err("--mars-terrain is a terrain, surface or section option".into());}
    if a.cmd == "terrain" && (a.mars_terrain.is_none() || a.save.is_none() || explicit_coordinates || explicit_target || a.definition.is_some()) {return Err("terrain requires --mars-terrain 4|16 --save NEW_FILE; optionally --initial SAVE".into());}
    if a.cmd != "section" && (a.from_site.is_some() || a.to_site.is_some()) {return Err("--from-site and --to-site are terrain section options".into());}
    if a.cmd == "section" {
        if explicit_target && !a.target.eq_ignore_ascii_case("Mars") {return Err("Terrain sections require Mars with the pinned MOLA field".into());}
        a.target="Mars".into();
        if explicit_coordinates {return Err("For custom section endpoints supply a section --definition file".into());}
        if a.definition.is_some() && (explicit_samples || a.from_site.is_some() || a.to_site.is_some()) {return Err("Choose a saved section definition or endpoint/sample flags".into());}
    }
    if let Some(id) = &a.site {
        if a.cmd != "surface" { return Err("--site is a surface probe option".into()); }
        let site = cosmogon_sim::planet::landing::find(id).ok_or_else(||format!("Unknown landing reference: {id}"))?;
        if explicit_coordinates || (explicit_target && !a.target.eq_ignore_ascii_case("Mars")) { return Err("--site supplies Mars and its source coordinates; do not combine with explicit coordinates or another target".into()); }
        a.target = "Mars".into(); a.latitude = site.latitude_north_deg; a.longitude = site.longitude_east_deg;
    }
    if !explicit_scenario && matches!(a.cmd.as_str(), "experiment" | "sweep") {
        a.scenario = Scenario::SolarSystemLab;
    }
    if matches!(a.cmd.as_str(), "run" | "check" | "survey" | "profile")
        && (!a.years.is_finite() || a.years < 0.0)
    {
        return Err("--years must be finite and nonnegative".into());
    }
    if !(1..=4096).contains(&a.systems) {
        return Err("--systems must be between 1 and 4096".into());
    }
    if a.cmd == "survey" && !(1..=1_000_000).contains(&a.seeds) {
        return Err("--seeds must be between 1 and 1000000".into());
    }
    Ok(a)
}
fn help() {
    eprintln!("cosmogon-cli contact-start [--output NEW_FILE]\ncosmogon-cli contact-hardware [--static-shear --operation-window SECONDS --initial WORLD --target Mars --geographic-start FILE --output NEW_FILE]\nflight-definition --contact-hardware FILE explicitly enables versioned terrain/rigid-body motion; --operation-window adds the modeled solver-integrated surface-operation ledger; no safe landing or delivery");
    eprintln!("cosmogon-cli support-definition [--output NEW_FILE]\ncosmogon-cli flight-support --recording COMPLETE_FLIGHT_DIR [--definition LIMITS_FILE --output NEW_DIR]\ncosmogon-cli support-inspect --recording COMPLETE_SUPPORT_DIR\nConditional actual footprint/force evidence; no attachment or cargo permission");
    eprintln!("cosmogon-cli arrival-start [--output NEW_FILE]\ncosmogon-cli flight-approach --recording COMPLETE_FLIGHT_DIR [--target BODY --output NEW_DIR]\ncosmogon-cli approach-inspect --recording COMPLETE_APPROACH_DIR\nGeographic flight-definition starts: --geographic-start FILE (explicit spherical altitude and east/north/up velocities)");
    eprintln!("cosmogon-cli flight-rendezvous --recording LEFT_DIR --reference RIGHT_DIR [--output NEW_DIR]\nClosest shared-observation-time evidence between two complete replay-verified flights from one origin world; sample-grid evidence only, never continuous closest approach, intercept or docking");
    eprintln!("cosmogon-cli flight-definition [--initial WORLD --definition PROPULSION_PLAN --target Mars --samples 121 --interval-seconds 1 --cargo-ledgers --handling --launch FILE --entry FILE --landing FILE --mission-span --transfer FILE --output NEW_FILE]\ncosmogon-cli flight-study --definition PINNED_STUDY [--initial WORLD --output NEW_DIR]\ncosmogon-cli flight-inspect --recording COMPLETE_DIR   # exact full-world flight replay; --cargo-ledgers opts into retained cargo mass ledgers; --handling opts into finite surface-handling equipment for co-located habitat transfers; --launch FILE consumes an authored surface-launch definition; --entry/--landing FILE consume authored heating/touchdown limits (contact gear required); --mission-span admits the raised work bounds and consolidated mission evidence for one authored end-to-end launch-to-delivery study (requires --launch); --transfer FILE authors an atomic habitat transfer at an exact sample");
    eprintln!("cosmogon-cli launch-definition --thrust-time SECONDS --thrust-newtons N [--maximum-acceleration M_S2 --liftoff-altitude M --maximum-start-altitude M --initial WORLD --target Mars --latitude N --longitude E --seed N --scenario S] --output NEW_FILE   # authored surface launch definition: fixed inertial zenith burn on contact gear with structural specific-force limits and liftoff/burnout/apogee evidence");
    eprintln!("cosmogon-cli entry-definition [--entry-interface-altitude M --nose-radius M --heat-flux-coefficient K --maximum-heat-flux W_M2 --maximum-total-heat J_M2 --initial WORLD --target Mars --latitude N --longitude E --seed N --scenario S] --output NEW_FILE   # authored entry-heating definition: Sutton-Graves stagnation flux with thermal limits on an atmospheric starting body");
    eprintln!("cosmogon-cli landing-definition [--maximum-touchdown-vertical M_S --maximum-touchdown-horizontal M_S --liftoff-altitude M --initial WORLD --target Mars --latitude N --longitude E --seed N --scenario S] --output NEW_FILE   # authored terminal-landing definition: touchdown-speed limits with a versioned liftoff baseline on contact gear");
    eprintln!("cosmogon-cli habitat-setup [--initial WORLD --target Mars --latitude N --longitude E --seed N --scenario S] --output NEW_FILE   # world containing one finite habitat, for flight transfer studies");
    eprintln!("cosmogon-cli vehicle-definition [--output NEW_FILE]   # explicit assumed finite vehicle and burn plan\ncosmogon-cli vehicle-study [--definition PLAN --output NEW_DIR]   # ideal free-space finite propulsion, no launch/delivery\ncosmogon-cli vehicle-inspect --recording COMPLETE_DIR   # exact independent numeric/source replay");
    eprintln!("cosmogon-cli cargo-epochs-definition [--output NEW_FILE]   # explicit burn → species removal → burn sequence\ncosmogon-cli cargo-epochs-study [--definition PLAN --output NEW_DIR]   # piecewise mass epochs, no world delivery\ncosmogon-cli cargo-epochs-inspect --recording COMPLETE_DIR   # exact independent epoch replay");
    eprintln!("cosmogon-cli habitat-rerun --recording COMPLETE_DESIGN --output NEW_DIR [--definition NEW_TYPED_PLAN]   # exact original common starting world");
    eprintln!("cosmogon-cli habitat-balance --recording FOLDER [--from-index N] [--observation N] [--output NEW_DIR]");
    eprintln!("cosmogon-cli habitat-figure --recording FOLDER --definition GRAPH_LAYOUT --output NEW_DIR [--reference ORDINARY_B]");
    eprintln!("cosmogon-cli habitat-sweep --initial SAVE --output NEW_DIR --parameter firm_power_w --from 0 --to 4000 --runs 3 [--band 0.1] [--samples 25 --interval-days 0.041666666666666664]\n  or --definition SWEEP_PLAN; optional --habitat-id ID\ncosmogon-cli habitat-compare --recording A --reference B [--measure water --alignment elapsed|world --output NEW_DIR]");
    eprintln!("cosmogon-cli habitat-diagnose --initial SAVE --habitat-id ID [--output NEW_DIR]   # explain completed power/water/heat");
    eprintln!("cosmogon-cli habitat-crop-install --initial SAVE --habitat-id ID --definition CROP_CONFIG --nutrient-kg KG --output NEW_DIR   # explicit runtime greenhouse installation granting only its nutrient charge\ncosmogon-cli habitat-crop-store --initial SAVE --habitat-id ID [--store-kg KG] --output NEW_DIR   # bounded 1:1 conversion of harvested edible dry material into the habitat dry-food tank\ncosmogon-cli habitat-construction-supply --initial SAVE --habitat-id ID --material-kg KG --output NEW_DIR   # explicit bounded supply of processed construction material\ncosmogon-cli habitat-stock-transfer --initial SAVE --from-id ID --to-id ID [--oxygen-kg KG --water-kg KG --food-dry-kg KG --food-water-kg KG] --output NEW_DIR   # conserved same-body transfer of finite habitat supply stocks");
    eprintln!("cosmogon-cli habitat-verify --recording COMPLETE_PAIRED_FOLDER [--output NEW_REPORT_DIR]   # recompute every actual row and endpoint");
    eprintln!("cosmogon-cli habitat-pressure --recording BRANCH_FOLDER --observation INDEX [--reference-index INDEX] [--output NEW_DIR]\ncosmogon-cli habitat-inspect --recording STUDY_FOLDER [--observation INDEX --output NEW_TRAJECTORY_DIR]   # validate and inspect recorded habitat observations");
    println!(
        r#"Cosmogon headless tools
  surface --target Mars          Sample authoritative MOLA height and source-cell coverage JSON
  section [--output NEW_DIR]      Calculate/export a source-supported MOLA terrain section
  sites                          Print historical landing references, sources and registration limits
  crop-chemistry --recording SUPPLY_FILE_OR_FOLDER [--output NEW_DIR]  Conservative CHON surrogate
  crop-chemistry-inspect --recording CHEMISTRY_FOLDER [--output NEW_DIR]  Verify exact chemistry evidence
  greenhouse-supply-definition [--output NEW_FILE]  Emit explicit finite crop supply inputs
  greenhouse-supplies [--definition FILE --initial SAVE --habitat-id N --output NEW_DIR]  Daily resource support
  greenhouse-supply-inspect --recording STUDY_FOLDER [--output NEW_DIR]  Verify and reopen exact supply study
  greenhouse-definition [--output NEW_FILE]  Emit source-pinned editable crop planning inputs
  greenhouse [--definition FILE --output NEW_DIR]  31 controlled lettuce forecast rows
      [--initial SAVE --habitat-id ID]   Optional exact habitat/world context, no resource credit
  habitat-crop-harvest --initial SAVE --habitat-id ID --output NEW_DIR  Explicit completed-cycle transfer
  habitat-crop-install --initial SAVE --habitat-id ID --definition CROP_CONFIG
      --nutrient-kg KG --output NEW_DIR  Runtime installation; grants only its nutrient charge.
  habitat-crop-store --initial SAVE --habitat-id ID [--store-kg KG]
      --output NEW_DIR  Bounded 1:1 harvested-edible → habitat dry-food conversion.
  habitat-construction-supply --initial SAVE --habitat-id ID --material-kg KG
      --output NEW_DIR  Explicit bounded charge of processed construction material.
  habitat-stock-transfer --initial SAVE --from-id ID --to-id ID
      [--oxygen-kg --water-kg --food-dry-kg --food-water-kg]
      --output NEW_DIR  Conserved same-body transfer of finite habitat supply stocks.
  habitat-crop-definition [--target Mars --output NEW_FILE]  Explicit finite crop setup; maintained assumption starts false
  habitat-definition [--target Mars --latitude N --longitude E] [--initial SAVE]
      [--output NEW_FILE]        Create a complete editable preset for this world.
  habitat-setup [--target Mars --latitude N --longitude E --seed N --scenario S]
      --output NEW_FILE        Save a world containing one finite habitat (transfer studies).
  habitat --definition FILE [--initial SAVE] [--output NEW_DIR]
      [--samples 1..1024 --interval-days D]  At most 30 days; default 25 hourly samples.
      --initial SAVE without definition resumes an existing habitat; --habitat-id selects among many.
  terrain --mars-terrain 4|16 --save NEW_FILE  Explicit source edit; optionally --initial SAVE
  datasets                       Print embedded MOLA source/version/checksum metadata
  models                         Print model/source/parameter registry JSON
  inspect --recording DIR         Validate and print saved numeric recording data
  experiment --output NEW_DIR    Record a saved or new universe at uniform simulation times
  sweep --output NEW_DIR         Run independent branches from the same starting state
  replay --output NEW_DIR        Run an exported definition against its frozen base
  validate [--output NEW_FILE]   Calculate analytic and cached JPL regression measurements
  run | check | survey | profile Run the released simulation/determinism tools

Terrain source: --mars-terrain 4|16 on surface/section explicitly edits that world.
Surface coordinates: --latitude -90..90 --longitude -360..360 (degrees, east positive)
Or: surface --site perseverance|curiosity|insight|viking-1 (historical touchdown, not current rover position)
Sections: --from-site curiosity --to-site insight --samples 2..1024
  Or --definition definition.json; keeps its chosen reference sphere radius and samples.
  Named sections use the selected Mars body radius (default Sol); --initial can supply a saved state.
  Exports contain definition.json, section.json, samples.csv and sources.json. No local-route safety claim.

Research inputs:
  --initial BASE.cosmo           Use a complete saved state; overrides scenario/seed
  --scenario lab|sol|garden|neighbourhood|system|empty --seed N
  --target NAME                  Observed object, or changed object for a sweep
  --samples 2..4096 --interval-days DAYS (86400 seconds per day)
  --parameter KEY --from VALUE --to VALUE --runs 2..64
  --measure-target NAME --measure KEY
  --recording DIR [--reference DIR --measure KEY] (inspect actual rows/differences)
  --definition definition.json   Replay requires this and --initial
Values use canonical units listed by models. Missing uncertainty is not inferred.
Research creation defaults to Solar System Lab. Results are exploratory implemented models;
use measured validation conditions to assess suitability. Existing exports are protected.

Simulation inputs: --years Y --life realistic|hopeful|teeming --systems N
                   --seeds N --min-importance N --save FILE
"#
    );
}

fn settings(a: &Args, seed: u64) -> UniverseSettings {
    let (_, life, intel, _) = LIFE_PRESETS[a.life];
    UniverseSettings {
        seed,
        scenario: a.scenario,
        system_count: a.systems,
        life_rate: life,
        intelligence_rate: intel,
        ..Default::default()
    }
}

fn greenhouse_definition(a:&Args)->Result<(),String> {
    use std::io::Write;
    let d=cosmogon_sim::greenhouse::Definition::default();let json=serde_json::to_string_pretty(&d).map_err(|e|e.to_string())?;
    if let Some(path)=&a.output {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|e.to_string())?;f.write_all(json.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{json}");Ok(())
}
fn greenhouse(a:&Args)->Result<(),String> {
    let d=match &a.definition {Some(p)=>cosmogon_sim::greenhouse::Definition::read(std::path::Path::new(p))?,None=>Default::default()};
    let report=d.calculate()?;
    if a.output.as_ref().is_some_and(|p|std::path::Path::new(p).exists()) {return Err("Greenhouse output folder already exists".into());}
    let world=a.initial.as_ref().map(|p|save::read_file(std::path::Path::new(p)).map_err(|e|e.to_string())).transpose()?;
    let id=a.habitat_id.or_else(||world.as_ref().filter(|u|u.habitats.len()==1).map(|u|u.habitats[0].id));
    if let (Some(u),Some(id))=(&world,id) {cosmogon_sim::habitat::sample(u,id)?;}
    if let Some(path)=&a.output {cosmogon_sim::greenhouse::export(&d,world.as_ref().map(|u|(u,id)),std::path::Path::new(path))?;}
    println!("{}",serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?);Ok(())
}

fn greenhouse_supply_definition(a:&Args)->Result<(),String> {
    use std::io::Write;
    let json=serde_json::to_string_pretty(&cosmogon_sim::greenhouse_supplies::Definition::default()).map_err(|e|e.to_string())?;
    if let Some(p)=&a.output {std::fs::OpenOptions::new().write(true).create_new(true).open(p).map_err(|e|e.to_string())?.write_all(json.as_bytes()).map_err(|e|e.to_string())?;}println!("{json}");Ok(())
}
fn greenhouse_supplies(a:&Args)->Result<(),String> {
    use cosmogon_sim::greenhouse_supplies::{Bundle,Definition};
    let b=if a.cmd=="greenhouse-supply-inspect" {Bundle::read(std::path::Path::new(a.recording.as_deref().ok_or("Choose a recorded supply study")?))?}else{
        let d=match &a.definition {Some(p)=>Definition::read(std::path::Path::new(p))?,None=>Default::default()};
        let u=a.initial.as_ref().map(|p|save::read_file(std::path::Path::new(p)).map_err(|e|e.to_string())).transpose()?;
        let id=a.habitat_id.or_else(||u.as_ref().filter(|u|u.habitats.len()==1).map(|u|u.habitats[0].id));
        Bundle::calculate(&d,u.as_ref().map(|u|(u,id)))?
    };
    if let Some(p)=&a.output {b.export(std::path::Path::new(p))?;}
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"report":b.report,"manifest":b.manifest})).map_err(|e|e.to_string())?);Ok(())
}

fn crop_chemistry(a:&Args)->Result<(),String> {
    use cosmogon_sim::{crop_chemistry as chemistry,greenhouse_supplies::Bundle};
    let path=std::path::Path::new(a.recording.as_deref().ok_or("Choose an original supply study or verified chemistry folder")?);
    let (b,r)=if a.cmd=="crop-chemistry-inspect" {chemistry::read(path)?}else{let b=Bundle::read(path)?;let r=chemistry::calculate(&b)?;(b,r)};
    if let Some(p)=&a.output {chemistry::export(&b,std::path::Path::new(p))?;}
    println!("{}",serde_json::to_string_pretty(&r).map_err(|e|e.to_string())?);Ok(())
}

fn main() {
    let a = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    match a.cmd.as_str() {
        "help" => help(),
        "crop-chemistry"|"crop-chemistry-inspect" => {if let Err(e)=crop_chemistry(&a) {eprintln!("Crop chemistry failed: {e}");std::process::exit(1);}},
        "greenhouse-supply-definition" => {if let Err(e)=greenhouse_supply_definition(&a) {eprintln!("Crop supply definition failed: {e}");std::process::exit(1);}},
        "greenhouse-supplies"|"greenhouse-supply-inspect" => {if let Err(e)=greenhouse_supplies(&a) {eprintln!("Crop supply study failed: {e}");std::process::exit(1);}},
        "greenhouse-definition" => {if let Err(e)=greenhouse_definition(&a) {eprintln!("Greenhouse definition failed: {e}");std::process::exit(1);}},
        "greenhouse" => {if let Err(e)=greenhouse(&a) {eprintln!("Greenhouse planning failed: {e}");std::process::exit(1);}},
        "habitat-crop-harvest" => {if let Err(e)=habitat_crop_harvest(&a){eprintln!("Crop harvest failed: {e}");std::process::exit(1);}},
        "habitat-crop-install" => {if let Err(e)=habitat_crop_install(&a){eprintln!("Crop install failed: {e}");std::process::exit(1);}},
        "habitat-crop-store" => {if let Err(e)=habitat_crop_store(&a){eprintln!("Crop food store failed: {e}");std::process::exit(1);}},
        "habitat-construction-supply" => {if let Err(e)=habitat_construction_supply(&a){eprintln!("Construction material supply failed: {e}");std::process::exit(1);}},
        "habitat-stock-transfer" => {if let Err(e)=habitat_stock_transfer(&a){eprintln!("Habitat stock transfer failed: {e}");std::process::exit(1);}},
        "habitat-diagnose" => {if let Err(e)=habitat_diagnose(&a) {eprintln!("Habitat diagnosis failed: {e}");std::process::exit(1);}},
        "habitat-verify" => {
            match habitat_verify(&a) {Ok(v)=>{println!("{}",serde_json::to_string_pretty(&v).unwrap());if v["verified"]!=true {eprintln!("Replay differs from saved observations; results are preserved.");std::process::exit(1);}},Err(e)=>{eprintln!("Habitat replay did not complete: {e}");std::process::exit(1);}}
        },
        "habitat-pressure" => {match habitat_pressure(&a) {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).unwrap()),Err(e)=>{eprintln!("Recorded pressure failed: {e}");std::process::exit(1);}}},
        "habitat-inspect" => {
            let inspected=habitat_inspect(&a);
            match inspected {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).unwrap()),Err(e)=>{eprintln!("Habitat inspection failed: {e}");std::process::exit(1);}}
        },
        "habitat-balance" => {
            let result=(||->Result<serde_json::Value,String>{
                let b=cosmogon_sim::habitat_balance::Bundle::read(std::path::Path::new(a.recording.as_deref().expect("validated recording")),a.from_index,a.observation)?;
                if let Some(output)=&a.output {b.export(std::path::Path::new(output))?;}
                serde_json::to_value(b).map_err(|e|e.to_string())
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).unwrap()),Err(e)=>{eprintln!("Recorded balance failed: {e}");std::process::exit(1);}}
        }
        "habitat-figure" => {
            let result=(|| {let layout=cosmogon_sim::habitat_graph::Layout::read(std::path::Path::new(a.definition.as_deref().expect("validated layout")))?;
                let recording=std::path::Path::new(a.recording.as_deref().expect("validated recording"));
                let figure=if let Some([left,right])=a.branches {
                    if a.reference.is_some(){return Err("Design branch selection does not accept an external reference".into());}
                    cosmogon_sim::habitat_design::read_single(recording)?.figure(left,right,&layout)?
                } else {cosmogon_sim::habitat_figure::Figure::read(recording,a.reference.as_deref().map(std::path::Path::new),&layout)?};
                figure.export(std::path::Path::new(a.output.as_deref().expect("validated output")))})();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).unwrap()),Err(e)=>{eprintln!("Recorded figure failed: {e}");std::process::exit(1);}}
        },
        "support-definition" | "flight-support" | "support-inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                use cosmogon_sim::{vehicle_support::Definition,vehicle_support_recording::Report,world_vehicle_recording::Bundle};
                if a.cmd=="support-definition" {let d=Definition::default();if let Some(path)=&a.output {d.write(std::path::Path::new(path))?;}return serde_json::to_value(d).map_err(|e|e.to_string());}
                let path=std::path::Path::new(a.recording.as_deref().expect("validated support source"));
                if a.cmd=="support-inspect" {return Ok(Report::read(path)?.manifest);}
                let definition=if let Some(path)=&a.definition {Definition::read(std::path::Path::new(path))?} else {Definition::default()};
                let report=Report::run(&Bundle::read(path)?,definition)?;
                if let Some(path)=&a.output {report.export(std::path::Path::new(path))}else{Ok(report.manifest)}
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite support evidence")),Err(e)=>{eprintln!("Support operation failed: {e}");std::process::exit(1);}}
        },
        "arrival-start" | "contact-start" | "flight-approach" | "approach-inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                use cosmogon_sim::{vehicle_arrival,vehicle_arrival_recording::Report,world_vehicle_recording::Bundle};
                if matches!(a.cmd.as_str(),"arrival-start"|"contact-start") {
                    let start=if a.cmd=="contact-start" {vehicle_arrival::GeographicStart::contact_example()} else {vehicle_arrival::GeographicStart::default()};let value=serde_json::to_value(&start).map_err(|e|e.to_string())?;
                    if let Some(path)=&a.output {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|e.to_string())?;use std::io::Write;f.write_all(&serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}
                    return Ok(value);
                }
                let path=std::path::Path::new(a.recording.as_deref().expect("validated recording"));
                if a.cmd=="approach-inspect" {return Ok(Report::read(path)?.manifest);}
                let flight=Bundle::read(path)?;
                let target=if a.target.is_empty() {flight.study.flight.parent} else {let sys=flight.study.flight.parent.system;let body=flight.origin.systems[sys as usize].find_body(&a.target).ok_or("No matching approach target in flight system")?;cosmogon_sim::BodyRef {system:sys,body:body as u32}};
                let report=Report::run(&flight,target)?;
                if let Some(path)=&a.output {report.export(std::path::Path::new(path))} else {Ok(report.manifest)}
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite approach evidence")),Err(e)=>{eprintln!("Arrival operation failed: {e}");std::process::exit(1);}}
        },
        "flight-rendezvous" => {
            let result=(||->Result<serde_json::Value,String>{
                use cosmogon_sim::{world_vehicle_recording::Bundle,world_vehicle_rendezvous::Report};
                let left=Bundle::read(std::path::Path::new(a.recording.as_deref().expect("validated recording")))?;
                let right=Bundle::read(std::path::Path::new(a.reference.as_deref().expect("validated reference")))?;
                let report=Report::run(&left,&right)?;
                if let Some(path)=&a.output {report.export(std::path::Path::new(path))} else {Ok(report.manifest())}
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite rendezvous evidence")),Err(e)=>{eprintln!("Flight rendezvous failed: {e}");std::process::exit(1);}}
        },
        "contact-hardware" => {
            let result=(||->Result<serde_json::Value,String>{
                use cosmogon_sim::{world_vehicle,world_vehicle_recording as flight,vehicle_contact_dynamics::Hardware};
                let world=if let Some(path)=&a.initial {flight::read_world(std::path::Path::new(path))?} else {Universe::new(UniverseSettings {scenario:a.scenario,seed:a.seed,..Default::default()})};
                let parent=world.systems.iter().enumerate().find_map(|(i,s)|s.find_body(&a.target).map(|j|cosmogon_sim::BodyRef {system:i as u32,body:j as u32})).ok_or("No contact starting body")?;
                let plan=cosmogon_sim::supply_vehicle::Plan::example();
                let point=if let Some(path)=&a.geographic_start {cosmogon_sim::vehicle_arrival::GeographicStart::read(std::path::Path::new(path))?.absolute(&world,parent)?.position_m} else {let definition=world_vehicle::Definition::circular_example(&world,parent,plan)?;world.systems[parent.system as usize].body_state(parent.body as usize,world.time).pos+definition.initial_relative_position_m};
                let hardware=Hardware::example(&world,parent,point)?;
                let hardware=if a.static_shear {hardware.with_shear(cosmogon_sim::vehicle_shear::Definition::example())?} else {hardware};
                let hardware=if let Some(seconds)=a.operation_window {hardware.with_operation(seconds)?} else {hardware};
                let value=serde_json::to_value(&hardware).map_err(|e|e.to_string())?;
                if let Some(path)=&a.output {
                    use std::io::Write;
                    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|format!("Contact hardware output is protected: {e}"))?;
                    file.write_all(serde_json::to_string_pretty(&value).map_err(|e|e.to_string())?.as_bytes()).map_err(|e|e.to_string())?;
                }
                Ok(value)
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite authored hardware")),Err(e)=>{eprintln!("Contact hardware failed: {e}");std::process::exit(1);}}
        },
        "flight-definition" | "flight-study" | "flight-inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                use cosmogon_sim::{world_vehicle,world_vehicle_recording as flight};
                if a.cmd=="flight-inspect" { return Ok(flight::Bundle::read(std::path::Path::new(a.recording.as_deref().expect("validated recording")))?.manifest); }
                let world=if let Some(path)=&a.initial {flight::read_world(std::path::Path::new(path))?} else {Universe::new(UniverseSettings {scenario:a.scenario,seed:a.seed,..Default::default()})};
                if a.cmd=="flight-definition" {
                    let parent=world.systems.iter().enumerate().find_map(|(i,s)|s.find_body(&a.target).map(|j|cosmogon_sim::BodyRef {system:i as u32,body:j as u32})).ok_or("No matching flight starting body")?;
                    let d=if let Some(path)=&a.launch {
                        let mut d=world_vehicle::Definition::read(std::path::Path::new(path))?;
                        if a.cargo_ledgers {d=d.with_cargo_ledgers()?;}
                        if a.handling {d=d.with_handling()?;}
                        d
                    } else {
                    let plan=if let Some(path)=&a.definition {cosmogon_sim::supply_vehicle::Plan::read(std::path::Path::new(path))?} else {cosmogon_sim::supply_vehicle::Plan::example()};
                    let hardware=a.contact_hardware.as_ref().map(|path|cosmogon_sim::vehicle_contact_dynamics::Hardware::read(std::path::Path::new(path))).transpose()?;
                    let d=if let Some(path)=&a.geographic_start {
                        let start=cosmogon_sim::vehicle_arrival::GeographicStart::read(std::path::Path::new(path))?;
                        if let Some(h)=hardware {start.contact_flight(&world,parent,plan,h)?} else {start.flight(&world,parent,plan)?}
                    } else {let d=world_vehicle::Definition::circular_example(&world,parent,plan)?;if let Some(h)=hardware {d.with_contact(&world,h)?} else {d}};
                    let d=if a.cargo_ledgers {d.with_cargo_ledgers()?} else {d};
                    if a.handling {d.with_handling()?} else {d}
                    };
                    let mut d=d;
                    if let Some(path)=&a.entry {d.entry=Some(world_vehicle::Definition::read(std::path::Path::new(path))?.entry.ok_or("Entry file must be an entry definition containing entry limits")?);}
                    if let Some(path)=&a.landing {d.landing=Some(world_vehicle::Definition::read(std::path::Path::new(path))?.landing.ok_or("Landing file must be a landing definition containing touchdown limits")?);}
                    let mut study=if a.mission_span {flight::Study::new_mission(&world,d,a.samples,a.interval_days*86400.)?} else {flight::Study::new(&world,d,a.samples,a.interval_days*86400.)?};
                    for path in &a.transfer {
                        study.transfers.push(serde_json::from_value::<cosmogon_sim::world_vehicle_recording::AuthoredTransfer>(serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(path).map_err(|e|format!("Transfer definition read failed: {e}"))?).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?);
                    }
                    if let Some(path)=&a.output {study.write(&world,std::path::Path::new(path))?;}
                    return serde_json::to_value(&study).map_err(|e|e.to_string());
                }
                let study=flight::Study::read(std::path::Path::new(a.definition.as_deref().expect("validated definition")))?;
                let bundle=flight::Bundle::run(&world,study)?;
                if let Some(output)=&a.output {bundle.export(std::path::Path::new(output))?;}
                Ok(bundle.manifest)
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite actual flight result")),Err(e)=>{eprintln!("World flight operation failed: {e}");std::process::exit(1);}}
        },
        "vehicle-definition" => {
            let plan=cosmogon_sim::supply_vehicle::Plan::example();
            if let Some(path)=&a.output {if let Err(e)=plan.write(std::path::Path::new(path)){eprintln!("Vehicle definition failed: {e}");std::process::exit(1);}}
            println!("{}",serde_json::to_string_pretty(&plan).expect("finite validated example"));
        },
        "vehicle-study" | "vehicle-inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                if a.cmd=="vehicle-inspect" {return cosmogon_sim::supply_vehicle::read_recording(std::path::Path::new(a.recording.as_deref().expect("validated recording")));}
                let plan=if let Some(path)=&a.definition {cosmogon_sim::supply_vehicle::Plan::read(std::path::Path::new(path))?}else{cosmogon_sim::supply_vehicle::Plan::example()};
                if let Some(output)=&a.output {plan.export(std::path::Path::new(output))}else{plan.summary()}
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite validated result")),Err(e)=>{eprintln!("Vehicle study failed: {e}");std::process::exit(1);}}
        },
        "cargo-epochs-definition" => {
            let plan=cosmogon_sim::vehicle_cargo_mass::Plan::example();
            if let Some(path)=&a.output {if let Err(e)=plan.write(std::path::Path::new(path)){eprintln!("Cargo-epochs definition failed: {e}");std::process::exit(1);}}
            println!("{}",serde_json::to_string_pretty(&plan).expect("finite validated example sequence"));
        },
        "cargo-epochs-study" | "cargo-epochs-inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                if a.cmd=="cargo-epochs-inspect" {return cosmogon_sim::vehicle_cargo_mass::read_recording(std::path::Path::new(a.recording.as_deref().expect("validated recording")));}
                let plan=if let Some(path)=&a.definition {cosmogon_sim::vehicle_cargo_mass::Plan::read(std::path::Path::new(path))?}else{cosmogon_sim::vehicle_cargo_mass::Plan::example()};
                if let Some(output)=&a.output {plan.export(std::path::Path::new(output))}else{plan.summary()}
            })();
            match result {Ok(v)=>println!("{}",serde_json::to_string_pretty(&v).expect("finite validated cargo-epochs result")),Err(e)=>{eprintln!("Cargo-epochs study failed: {e}");std::process::exit(1);}}
        },
        "habitat-rerun" => {if let Err(e)=habitat_rerun(&a){eprintln!("Saved-origin rerun failed: {e}");std::process::exit(1);}},
        "habitat-sweep" => {if let Err(e)=habitat_sweep(&a) {eprintln!("Habitat sweep failed: {e}");std::process::exit(1);}},
        "habitat-compare" => {if let Err(e)=habitat_compare(&a) {eprintln!("Habitat comparison failed: {e}");std::process::exit(1);}},
        "habitat-definition"|"habitat-crop-definition" => {if let Err(e)=habitat_definition(&a) {eprintln!("Habitat definition failed: {e}");std::process::exit(1);}},
        "habitat-setup" => {if let Err(e)=habitat_setup(&a) {eprintln!("Habitat setup failed: {e}");std::process::exit(1);}},
        "launch-definition" => {if let Err(e)=launch_definition(&a) {eprintln!("Launch definition failed: {e}");std::process::exit(1);}},
        "entry-definition" => {if let Err(e)=entry_definition(&a) {eprintln!("Entry definition failed: {e}");std::process::exit(1);}},
        "landing-definition" => {if let Err(e)=landing_definition(&a) {eprintln!("Landing definition failed: {e}");std::process::exit(1);}},
        "habitat" => {if let Err(e)=habitat(&a) {eprintln!("Habitat study failed: {e}");std::process::exit(1);}},
        "section" => {if let Err(e)=section(&a) {eprintln!("Terrain section failed: {e}");std::process::exit(1);}},
        "sites" => println!("{}", serde_json::to_string_pretty(&cosmogon_sim::planet::landing::registry()).unwrap()),
        "datasets" => println!("{}", serde_json::to_string_pretty(&cosmogon_sim::planet::mola::registry()).unwrap()),
        "terrain" => {if let Err(e)=terrain(&a) {eprintln!("Terrain change failed: {e}");std::process::exit(1);}},
        "surface" => { if let Err(e) = surface(&a) { eprintln!("Surface probe failed: {e}"); std::process::exit(1); } }
        "models" => println!("{}", serde_json::to_string_pretty(&serde_json::json!({"format":"cosmogon-model-registry", "schema_version":1, "models":cosmogon_sim::research::PhysicsModelRegistry::models(), "sources":cosmogon_sim::research::SCIENTIFIC_SOURCES, "parameters":cosmogon_sim::research::PARAMETERS})).unwrap()),
        "inspect" => {
            let result=(||->Result<serde_json::Value,String>{
                let path=a.recording.as_ref().ok_or("inspect requires --recording DIR")?;
                let table=cosmogon_sim::research::SavedRecording::read(std::path::Path::new(path))?;
                if let Some(reference)=&a.reference {
                    let reference=cosmogon_sim::research::SavedRecording::read(std::path::Path::new(reference))?;
                    let key=a.measure.as_ref().ok_or("Difference inspection requires --measure KEY")?;
                    let parameter=cosmogon_sim::research::PARAMETERS.iter().find(|p|p.key.eq_ignore_ascii_case(key)||format!("{:?}",p.id).eq_ignore_ascii_case(key)).ok_or("Unknown measurement; use models")?;
                    let difference=table.difference_from(&reference,parameter.id)?;
                    Ok(serde_json::json!({"format":"cosmogon-recording-difference","schema_version":1,"parameter":parameter.key,"unit":parameter.unit,"target":table.target_name,"reference":reference.target_name,"points":difference,"alignment":"Identical elapsed sample times since each recording's own start; no interpolation or absolute-epoch alignment","limitations":"Difference is not a physical error estimate or uncertainty. Shared initial conditions are not inferred."}))
                } else {serde_json::to_value(&table).map_err(|e|e.to_string())}
            })();
            match result {Ok(data)=>println!("{}",serde_json::to_string_pretty(&data).unwrap()),Err(e)=>{eprintln!("Recording inspect failed: {e}");std::process::exit(1);}}
        }
        "experiment" => { if let Err(e) = experiment(&a) { eprintln!("Experiment failed: {e}"); std::process::exit(1); } }
        "sweep" | "replay" => { if let Err(e) = batch(&a) { eprintln!("Batch failed: {e}"); std::process::exit(1); } }
        "validate" => {
            let report=cosmogon_sim::research::validation_report();
            if let Some(path)=&a.output {if let Err(e)=report.export(std::path::Path::new(path)){eprintln!("Validation export failed: {e}");std::process::exit(1);}}
            else {println!("{}",serde_json::to_string_pretty(&report).unwrap());}
            if !report.passes_regression {std::process::exit(1);}
        }
        "run" => run(&a),
        "check" => check(&a),
        "survey" => survey(&a),
        "profile" => profile(&a),
        c => { eprintln!("unknown command {c}; use run | check | survey | profile | models | experiment | sweep | replay | validate"); std::process::exit(1); }
    }
}

fn run(a: &Args) {
    let t0 = Instant::now();
    let mut u = Universe::new(settings(a, a.seed));
    println!(
        "Created {} in {:.2?}: {} systems, {} biospheres",
        a.scenario.label(),
        t0.elapsed(),
        u.systems.len(),
        u.biospheres.len()
    );
    let t1 = Instant::now();
    u.advance_by(a.years * SECONDS_PER_YEAR);
    println!(
        "Simulated {} in {:.2?}\n",
        format_duration(a.years * SECONDS_PER_YEAR),
        t1.elapsed()
    );
    for e in u
        .history
        .events
        .iter()
        .filter(|e| e.importance >= a.min_importance)
    {
        println!(
            "[{:>26}] {:<12} {}{}",
            cosmogon_sim::time::format_date(e.time, u.start_time, u.gregorian()),
            e.category.label(),
            e.title,
            if e.importance >= 5 { "  ★" } else { "" }
        );
    }
    let graph = TechGraph::embedded();
    for c in &u.civs {
        println!(
            "\n{} ({}) — {:?}\n  population {}  capacity {}  stability {:.2}  era {}  techs {}/{}  settlements {}  energy {:.2e} W",
            c.name,
            c.species.name,
            c.status,
            group_digits(c.population),
            group_digits(c.capacity),
            c.stability,
            c.era(graph),
            c.discoveries.len(),
            graph.len(),
            c.sites.iter().filter(|s| s.active()).count(),
            c.total_power_w()
        );
        let k: Vec<String> = cosmogon_sim::civ::knowledge::Domain::ALL
            .iter()
            .map(|d| format!("{}={:.2e}", &d.name()[..4], c.knowledge[d.index()]))
            .collect();
        println!("  knowledge: {}", k.join(" "));
    }
    if let Some(path) = &a.save {
        save::write_file(std::path::Path::new(path), &u, "cli").expect("save");
        println!("\nSaved to {path}");
    }
}

fn check(a: &Args) {
    let mut x = Universe::new(settings(a, a.seed));
    let mut y = Universe::new(settings(a, a.seed));
    let total = a.years * SECONDS_PER_YEAR;
    x.advance_by(total);
    let mut done = 0.0;
    let mut i = 0u64;
    while done < total {
        let step = (1.0 + (i % 7) as f64 * 13.7) * SECONDS_PER_YEAR;
        done = (done + step).min(total);
        y.advance_to(y.start_time + done, None, None);
        i += 1;
    }
    let reloaded = save::from_json(&save::to_json(&x, "check").unwrap()).unwrap();
    let same = serde_json::to_string(&x).unwrap() == serde_json::to_string(&y).unwrap();
    let rt = reloaded == x;
    println!("chunking-independent: {same}\nsave round-trip exact: {rt}");
    if !(same && rt) {
        std::process::exit(1);
    }
}

fn survey(a: &Args) {
    let mut life = 0;
    let mut complex = 0;
    let mut intelligent = 0;
    for seed in 0..a.seeds {
        let mut u = Universe::new(settings(a, seed));
        // Geological time in 1 Myr chunks until intelligence appears, then 20 kyr of history.
        let end = u.start_time + a.years * SECONDS_PER_YEAR;
        let mut emerged_at = None;
        while u.time < end && u.civs.is_empty() {
            u.advance_to((u.time + 1.0e6 * SECONDS_PER_YEAR).min(end), None, None);
        }
        if !u.civs.is_empty() {
            emerged_at = Some(u.time - u.start_time);
            u.advance_by(20_000.0 * SECONDS_PER_YEAR);
        }
        if let Some(t) = emerged_at {
            println!("seed {seed:>3}: intelligence after {}", format_duration(t));
        }
        let any = |s| u.biospheres.iter().any(|b| b.stage >= s);
        life += any(cosmogon_sim::life::Stage::Microbial) as u32;
        complex += any(cosmogon_sim::life::Stage::ComplexEcosystems) as u32;
        intelligent += !u.civs.is_empty() as u32;
        let best = u
            .civs
            .iter()
            .map(|c| c.discoveries.len())
            .max()
            .unwrap_or(0);
        println!(
            "seed {seed:>3}: civs {} (most techs {best}), events {}",
            u.civs.len(),
            u.history.events.len()
        );
    }
    println!("\nuniverses with microbial life {life}/{0}, complex life {complex}/{0}, intelligence {intelligent}/{0}", a.seeds);
}

/// Advance in chunks and report wall time per chunk with each civilization's step size.
fn profile(a: &Args) {
    let mut u = Universe::new(settings(a, a.seed));
    let chunk = (a.years / 40.0).max(1.0) * SECONDS_PER_YEAR;
    let end = u.start_time + a.years * SECONDS_PER_YEAR;
    while u.time < end {
        let t0 = Instant::now();
        let r = u.advance_to((u.time + chunk).min(end), None, None);
        let civs: Vec<String> = u
            .civs
            .iter()
            .map(|c| format!("{}:{:?} stride {} pop {} techs {} press f{:.2} d{:.2} w{:.2} short {:.2} cap% {:.0}", c.species.name, c.status, c.stride_years, cosmogon_sim::time::group_digits(c.population), c.discoveries.len(), c.pressures.food, c.pressures.disease, c.pressures.war, c.energy_shortfall, 100.0 * c.population / c.capacity.max(1.0)))
            .collect();
        println!(
            "{:>14} {:>8.1?} steps {:>9}  {}",
            format_duration(u.time - u.start_time),
            t0.elapsed(),
            r.steps,
            civs.join(" | ")
        );
    }
}

/// Offline observer samples use exact requested simulation times; there is no renderer.
fn experiment(a: &Args) -> Result<(), String> {
    use cosmogon_sim::research::Experiment;
    if !a.interval_days.is_finite() || !(1e-6..=3652.5).contains(&a.interval_days) {
        return Err("Choose 0.000001–3652.5 days between samples".into());
    }
    let output = a
        .output
        .as_deref()
        .ok_or("experiment requires --output NEW_DIRECTORY")?;
    if std::path::Path::new(output).exists() {
        return Err("The output directory already exists; choose a new path".into());
    }
    let mut u = initial_universe(a)?;
    let t = find_target(&u, &a.target).ok_or_else(|| format!("No object named {}", a.target))?;
    let mut recording = Experiment::new(&u, t, a.samples)?;
    recording.sampling_policy = format!(
        "Offline observer: uniform requested interval {} days (86400 seconds per day)",
        a.interval_days
    );
    let start = u.time;
    for i in 1..a.samples {
        u.advance_to(start + i as f64 * a.interval_days * 86400.0, None, None);
        recording.capture(&u)?;
    }
    recording.stop_reason = Some("Requested offline sample count reached".into());
    recording.export(std::path::Path::new(output))?;
    println!(
        "Recorded {} states of {} to {}",
        recording.samples.len(),
        recording.target_name,
        output
    );
    Ok(())
}
fn initial_universe(a: &Args) -> Result<Universe, String> {
    if let Some(path) = &a.initial {
        save::read_file(std::path::Path::new(path)).map_err(|e| e.to_string())
    } else {
        Ok(Universe::new(settings(a, a.seed)))
    }
}
fn batch(a: &Args) -> Result<(), String> {
    use cosmogon_sim::research::{ExperimentDefinition, ParameterSweep, PARAMETERS};
    let output = a
        .output
        .as_deref()
        .ok_or("Batch requires --output NEW_DIRECTORY")?;
    let u = initial_universe(a)?;
    let definition = if a.cmd == "replay" {
        if a.initial.is_none() {
            return Err(
                "Replay requires --initial base.cosmo and --definition definition.json".into(),
            );
        }
        ExperimentDefinition::read(std::path::Path::new(
            a.definition
                .as_deref()
                .ok_or("Replay requires --definition definition.json")?,
        ))?
    } else {
        let target =
            find_target(&u, &a.target).ok_or_else(|| format!("No object named {}", a.target))?;
        let parameter = PARAMETERS
            .iter()
            .find(|p| {
                p.key == a.parameter || format!("{:?}", p.id).eq_ignore_ascii_case(&a.parameter)
            })
            .ok_or("Unknown parameter; use a canonical key from models")?
            .id;
        let measured = if let Some(name) = &a.measure_target {
            find_target(&u, name).ok_or_else(|| format!("No measurement object named {name}"))?
        } else {
            target
        };
        let mut d = ExperimentDefinition::new(measured);
        if let Some(key) = &a.measure {
            d.primary_measurement = Some(
                PARAMETERS
                    .iter()
                    .find(|p| p.key == key || format!("{:?}", p.id).eq_ignore_ascii_case(key))
                    .ok_or("Unknown measurement; use a canonical key from models")?
                    .id,
            );
        }
        d.initial_fingerprint = Some(cosmogon_sim::research::state_fingerprint(&u)?);
        d.name = format!(
            "{} {} sweep → {}",
            a.target,
            parameter.descriptor().name,
            measured.name(&u).unwrap_or("measurement")
        );
        d.samples = a.samples;
        d.interval_s = a.interval_days * 86400.0;
        d.sweep = Some(ParameterSweep {
            target,
            parameter,
            values: ExperimentDefinition::linear_values(
                a.sweep_start.ok_or("Sweep requires --from")?,
                a.sweep_end.ok_or("Sweep requires --to")?,
                a.sweep_runs,
            )?,
        });
        d
    };
    let mut last = 0;
    let results = cosmogon_sim::research::run_batch(
        &u,
        &definition,
        std::path::Path::new(output),
        || false,
        |p| {
            if p.completed_runs > last {
                last = p.completed_runs;
                eprintln!("Completed {} / {} runs", last, p.total_runs);
            }
        },
    )?;
    println!("Exported {} independent runs to {}", results.len(), output);
    Ok(())
}
fn find_target(u: &Universe, name: &str) -> Option<cosmogon_sim::research::ResearchTarget> {
    use cosmogon_sim::research::ResearchTarget;
    for (s, sys) in u.systems.iter().enumerate() {
        if sys.star.name.eq_ignore_ascii_case(name) {
            return Some(ResearchTarget::Star(s as u32));
        }
        if let Some((b, _)) = sys
            .bodies
            .iter()
            .enumerate()
            .find(|(_, b)| b.exists() && b.name.eq_ignore_ascii_case(name))
        {
            return Some(ResearchTarget::Body(cosmogon_sim::BodyRef {
                system: s as u32,
                body: b as u32,
            }));
        }
    }
    None
}

fn terrain_world(a: &Args) -> Result<Universe,String> {
    let mut u=if let Some(path)=&a.initial {save::read_file(std::path::Path::new(path)).map_err(|e|e.to_string())?} else {Universe::new(settings(a,a.seed))};
    if let Some(ppd)=&a.mars_terrain {
        let r=u.systems.iter().find_map(|s|s.bodies.iter().enumerate().find(|(_,b)|b.exists() && b.name.eq_ignore_ascii_case("Mars")).map(|(j,_)|cosmogon_sim::BodyRef {system:s.id,body:j as u32})).ok_or("Mars is absent from this world")?;
        let dataset=if ppd=="16" {cosmogon_sim::planet::terrain::ElevationData::MarsMola16} else {cosmogon_sim::planet::terrain::ElevationData::MarsMola};
        u.apply_edit(cosmogon_sim::sandbox::Edit::SetMarsTerrain {body:r,dataset})?;
    }
    Ok(u)
}
fn terrain(a: &Args) -> Result<(),String> {
    use std::io::Write;
    let u=terrain_world(a)?;
    let bytes=save::to_json(&u,"Mars terrain source choice").map_err(|e|e.to_string())?;
    let path=std::path::Path::new(a.save.as_ref().ok_or("--save required")?);
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|format!("Existing saves are protected: {e}"))?;
    file.write_all(bytes.as_bytes()).map_err(|e|e.to_string())?;
    println!("Saved explicit terrain source change to {}",path.display());Ok(())
}
fn surface(a: &Args) -> Result<(), String> {
    use cosmogon_sim::planet::terrain::ElevationData;
    let u = terrain_world(a)?;
    let body = u.systems.iter().flat_map(|s| &s.bodies).find(|b|b.exists() && b.name.eq_ignore_ascii_case(&a.target)).ok_or("Target body not found")?;
    let dataset=body.elevation_data.and_then(ElevationData::mola).ok_or("This probe requires a supported MOLA grid; older procedural saves retain their terrain")?;
    let mut record = cosmogon_sim::planet::landing::probe_dataset(dataset,a.latitude,a.longitude,a.site.as_deref())?;
    record["target"]=serde_json::json!(body.name);
    println!("{}",serde_json::to_string_pretty(&record).map_err(|e|e.to_string())?);
    Ok(())
}

fn section(a: &Args) -> Result<(),String> {
    use cosmogon_sim::planet::{section::{Definition,Point},terrain::ElevationData};
    let u=terrain_world(a)?;
    let body=u.systems.iter().flat_map(|s|&s.bodies).find(|b|b.exists() && b.name.eq_ignore_ascii_case("Mars")).ok_or("Mars is absent from this world")?;
    let dataset=body.elevation_data.and_then(ElevationData::mola).ok_or("Terrain sections require a pinned MOLA field; this saved terrain is preserved")?;
    let definition=if let Some(path)=&a.definition {Definition::read(std::path::Path::new(path))?} else {Definition {terrain_dataset:dataset.id().into(),start:Point::site(a.from_site.as_deref().unwrap_or("curiosity"))?,end:Point::site(a.to_site.as_deref().unwrap_or("insight"))?,reference_radius_m:body.radius,samples:a.samples,..Default::default()}};
    if definition.terrain_dataset != dataset.id() {return Err("Section pins a different terrain dataset than this world; restore that source explicitly or select the correct saved world".into());}
    let section=definition.calculate()?;
    if let Some(path)=&a.output {section.export(std::path::Path::new(path))?;}
    println!("{}",serde_json::to_string_pretty(&section).map_err(|e|e.to_string())?);Ok(())
}

fn habitat_diagnose(a:&Args)->Result<(),String> {
    let u=save::read_file(std::path::Path::new(a.initial.as_deref().expect("validated initial"))).map_err(|e|e.to_string())?;
    let id=a.habitat_id.expect("validated identity");
    let h=u.habitats.iter().find(|h|h.id==id).ok_or("No such habitat in saved world")?;
    let report=cosmogon_sim::habitat_diagnostics::report(h,u.time)?;
    if let Some(path)=&a.output {cosmogon_sim::habitat_diagnostics::export(h,u.time,std::path::Path::new(path))?;}
    println!("{}",serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?);Ok(())
}

fn habitat_sweep_plan(a:&Args)->Result<cosmogon_sim::habitat_sweep::Plan,String> {
    use cosmogon_sim::habitat_sweep::{Parameter,Plan};
    let mut plan=if let Some(p)=&a.definition {Plan::read(std::path::Path::new(p))?} else {Plan::linear(Parameter::parse(&a.parameter)?,a.sweep_start.ok_or("Sweep requires --from")?,a.sweep_end.ok_or("Sweep requires --to")?,a.sweep_runs,a.samples,a.interval_days*cosmogon_sim::time::SECONDS_PER_DAY)?};
    if let Some(b)=a.band {plan.authored_relative_band=Some(b);plan.validate()?;}
    Ok(plan)
}
fn habitat_sweep(a:&Args)->Result<(),String> {
    let plan=habitat_sweep_plan(a)?;let base=save::read_file(std::path::Path::new(a.initial.as_deref().expect("validated initial"))).map_err(|e|e.to_string())?;
    let id=a.habitat_id.or_else(||(base.habitats.len()==1).then(||base.habitats[0].id)).ok_or("Choose --habitat-id when the common world has multiple habitats")?;
    let report=cosmogon_sim::habitat_sweep::execute(&base,id,&plan,std::path::Path::new(a.output.as_deref().expect("validated output")))?;
    println!("{}",serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?);Ok(())
}
fn habitat_rerun(a:&Args)->Result<(),String> {
    let path=std::path::Path::new(a.recording.as_deref().expect("validated design"));
    let origin=cosmogon_sim::habitat_origin::Origin::read(path)?;
    let plan=if let Some(definition)=&a.definition{cosmogon_sim::habitat_sweep::Plan::read(std::path::Path::new(definition))?}else{cosmogon_sim::habitat_sweep::Plan::read(&path.join("plan.json"))?};
    let result=cosmogon_sim::habitat_sweep::execute(&origin.load()?,origin.habitat_id,&plan,std::path::Path::new(a.output.as_deref().expect("validated new output")))?;
    println!("{}",serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?);Ok(())
}
fn habitat_verify(a:&Args)->Result<serde_json::Value,String> {
    let report=cosmogon_sim::habitat_replay::verify(std::path::Path::new(a.recording.as_deref().expect("validated replay folder")))?;
    if let Some(output)=&a.output {cosmogon_sim::habitat_replay::export(&report,std::path::Path::new(output))?;}
    Ok(report)
}

fn habitat_pressure(a:&Args)->Result<serde_json::Value,String> {
    let path=std::path::Path::new(a.recording.as_deref().expect("validated recording"));
    if path.join("plan.json").exists() {return Err("Pressure accounting uses one actual branch; choose the paired folder's run-000, run-001, run-002 or run-003 directory".into());}
    let recording=cosmogon_sim::habitat_recording::Recording::read(path)?;
    let report=cosmogon_sim::habitat_pressure::explain(&recording,a.observation.expect("validated observation"),a.reference_index)?;
    if let Some(output)=&a.output {cosmogon_sim::habitat_pressure::export(&report,std::path::Path::new(output))?;}
    serde_json::to_value(&report).map_err(|e|e.to_string())
}

fn habitat_inspect(a:&Args)->Result<serde_json::Value,String> {
    let path=std::path::Path::new(a.recording.as_deref().expect("validated recording"));
    if path.join("plan.json").exists() {
        match cosmogon_sim::habitat_design::read(path)? {
            cosmogon_sim::habitat_design::Design::Paired(design)=>{
                let report=if let Some(index)=a.observation {design.report_at(index)?} else {design.analysis.clone()};
                if let Some(output)=&a.output {design.export_trajectories(std::path::Path::new(output))?;}
                Ok(report)
            },
            cosmogon_sim::habitat_design::Design::Single(design)=>{
                let report=if let Some(index)=a.observation {design.report_at(index)?} else {design.manifest.clone()};
                if let Some(output)=&a.output {design.export_trajectories(std::path::Path::new(output))?;}
                Ok(report)
            },
        }
    } else {
        if a.observation.is_some() || a.output.is_some() {return Err("--observation/--output require a complete paired design folder".into());}
        Ok(cosmogon_sim::habitat_recording::Recording::read(path)?.summary)
    }
}

fn habitat_compare(a:&Args)->Result<(),String> {
    use cosmogon_sim::{habitat_comparison as comparison,habitat_recording::{Metric,Recording}};
    let left_path=std::path::Path::new(a.recording.as_deref().expect("validated A"));let right_path=std::path::Path::new(a.reference.as_deref().expect("validated B"));
    let left=Recording::read(left_path)?;let right=Recording::read(right_path)?;
    let left_band=cosmogon_sim::research::RunBand::read_optional(left_path)?;let right_band=cosmogon_sim::research::RunBand::read_optional(right_path)?;
    let metric=Metric::parse(a.measure.as_deref().unwrap_or("water"))?;let alignment=comparison::Alignment::parse(&a.alignment)?;
    if let Some(path)=&a.output {comparison::export_with_bands(&left,&right,metric,alignment,left_band.as_ref(),right_band.as_ref(),std::path::Path::new(path))?;}
    println!("{}",serde_json::to_string_pretty(&comparison::report_with_bands(&left,&right,metric,alignment,left_band.as_ref(),right_band.as_ref())).map_err(|e|e.to_string())?);Ok(())
}

fn habitat(a:&Args)->Result<(),String> {
    use cosmogon_sim::habitat::{self,Definition};
    if a.output.as_ref().is_some_and(|p|std::path::Path::new(p).exists()) {return Err("Existing exports are protected; choose a new folder".into());}
    let definition=a.definition.as_ref().map(|p|Definition::read(std::path::Path::new(p))).transpose()?;
    let mut u=if let Some(path)=&a.initial {save::read_file(std::path::Path::new(path)).map_err(|e|e.to_string())?} else {Universe::new(settings(a,a.seed))};
    let id=if let Some(definition)=definition {
        u.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(definition)})?;u.habitats.last().expect("created").id
    } else if let Some(id)=a.habitat_id {if !u.habitats.iter().any(|h|h.id==id) {return Err("No such habitat in saved world".into());}id}
    else if u.habitats.len()==1 {u.habitats[0].id}
    else {return Err("Saved world needs one habitat, or explicit --habitat-id".into());};
    let initial=u.clone();let mut rows=Vec::with_capacity(a.samples);
    for i in 0..a.samples {u.advance_to(initial.time+i as f64*a.interval_days*cosmogon_sim::time::SECONDS_PER_DAY,None,None);rows.push(habitat::sample(&u,id)?);}
    if let Some(path)=&a.output {habitat::export(&initial,&u,id,&rows,std::path::Path::new(path))?;}
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"format":"cosmogon-habitat-study-result","schema_version":1,"samples":rows.len(),"interval_s":a.interval_days*cosmogon_sim::time::SECONDS_PER_DAY,"initial_state_fingerprint":cosmogon_sim::research::state_fingerprint(&initial)?,"final_state_fingerprint":cosmogon_sim::research::state_fingerprint(&u)?,"report":u.habitats.iter().find(|h|h.id==id).expect("sampled").report()})).map_err(|e|e.to_string())?);Ok(())
}

fn habitat_crop_harvest(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=save::read_file(std::path::Path::new(a.initial.as_deref().ok_or("Initial world required")?)).map_err(|e|e.to_string())?;
    let before=u.clone();let id=a.habitat_id.ok_or("Habitat identity required")?;
    let system=u.habitats.iter().find(|h|h.id==id).ok_or("No such habitat")?.definition.body.system;
    let edit=cosmogon_sim::sandbox::Edit::HarvestHabitatCrop {system,id};
    let action=serde_json::json!({"format":"cosmogon-crop-harvest","schema_version":1,"edit":edit,"before_state_fingerprint":cosmogon_sim::research::state_fingerprint(&before)?});
    u.apply_edit(edit)?;
    let p=std::path::Path::new(a.output.as_deref().ok_or("New output folder required")?);
    let row=cosmogon_sim::habitat::sample(&u,id)?;
    cosmogon_sim::habitat::export(&u,&u,id,&[row],p)?;
    for (name,text) in [("before-harvest.cosmo",save::to_json(&before,"Before explicit crop harvest").map_err(|e|e.to_string())?),("harvest-action.json",serde_json::to_string_pretty(&action).map_err(|e|e.to_string())?)] {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p.join(name)).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{}",serde_json::to_string_pretty(&u.habitats.iter().find(|h|h.id==id).expect("harvested").report()).map_err(|e|e.to_string())?);Ok(())
}

/// Runtime installation of a finite greenhouse process into an existing
/// cropless habitat. The edit grants only its declared bounded nutrient charge;
/// the before-world copy, action JSON and ordinary export are protected.
fn habitat_crop_install(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=save::read_file(std::path::Path::new(a.initial.as_deref().ok_or("Initial world required")?)).map_err(|e|e.to_string())?;
    let before=u.clone();let id=a.habitat_id.ok_or("Habitat identity required")?;
    let system=u.habitats.iter().find(|h|h.id==id).ok_or("No such habitat")?.definition.body.system;
    let bytes=std::fs::read(std::path::Path::new(a.definition.as_deref().ok_or("Crop configuration required")?)).map_err(|e|e.to_string())?;
    let config:cosmogon_sim::crop_process::Config=serde_json::from_slice(&bytes).map_err(|e|format!("Crop configuration must be finite crop process JSON: {e}"))?;
    let nutrient_kg=a.nutrient_kg.ok_or("Nutrient charge required")?;
    let edit=cosmogon_sim::sandbox::Edit::InstallHabitatCrop {system,id,config:Box::new(config),nutrient_kg};
    let action=serde_json::json!({"format":"cosmogon-crop-install","schema_version":1,"edit":edit,"before_state_fingerprint":cosmogon_sim::research::state_fingerprint(&before)?});
    u.apply_edit(edit)?;
    let p=std::path::Path::new(a.output.as_deref().ok_or("New output folder required")?);
    let row=cosmogon_sim::habitat::sample(&u,id)?;
    cosmogon_sim::habitat::export(&u,&u,id,&[row],p)?;
    for (name,text) in [("before-habitat-crop-install.cosmo",save::to_json(&before,"Before explicit greenhouse installation").map_err(|e|e.to_string())?),("install-action.json",serde_json::to_string_pretty(&action).map_err(|e|e.to_string())?)] {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p.join(name)).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{}",serde_json::to_string_pretty(&u.habitats.iter().find(|h|h.id==id).expect("installed").report()).map_err(|e|e.to_string())?);Ok(())
}

/// Bounded explicit conversion of held harvested edible dry material into the
/// habitat's finite dry-food store. Without --store-kg the full held edible
/// amount is requested; storage capacity keeps any rejected remainder.
fn habitat_crop_store(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=save::read_file(std::path::Path::new(a.initial.as_deref().ok_or("Initial world required")?)).map_err(|e|e.to_string())?;
    let before=u.clone();let id=a.habitat_id.ok_or("Habitat identity required")?;
    let system=u.habitats.iter().find(|h|h.id==id).ok_or("No such habitat")?.definition.body.system;
    let requested_kg=a.store_kg.unwrap_or_else(||u.habitats.iter().find(|h|h.id==id).expect("habitat").state.crop.as_ref().map_or(0.,|c|c.harvested_dry_kg[0]));
    let edit=cosmogon_sim::sandbox::Edit::StoreHabitatCropFood {system,id,requested_kg};
    let action=serde_json::json!({"format":"cosmogon-crop-store","schema_version":1,"edit":edit,"before_state_fingerprint":cosmogon_sim::research::state_fingerprint(&before)?});
    u.apply_edit(edit)?;
    let p=std::path::Path::new(a.output.as_deref().ok_or("New output folder required")?);
    let row=cosmogon_sim::habitat::sample(&u,id)?;
    cosmogon_sim::habitat::export(&u,&u,id,&[row],p)?;
    for (name,text) in [("before-habitat-crop-store.cosmo",save::to_json(&before,"Before explicit crop food store").map_err(|e|e.to_string())?),("store-action.json",serde_json::to_string_pretty(&action).map_err(|e|e.to_string())?)] {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p.join(name)).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{}",serde_json::to_string_pretty(&u.habitats.iter().find(|h|h.id==id).expect("stored").report()).map_err(|e|e.to_string())?);Ok(())
}

/// Explicit bounded supply of processed construction material into an existing
/// habitat's finite construction stock. The edit grants only the declared
/// charge; nothing beyond the requested amount or installed capacity moves.
fn habitat_construction_supply(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=save::read_file(std::path::Path::new(a.initial.as_deref().ok_or("Initial world required")?)).map_err(|e|e.to_string())?;
    let before=u.clone();let id=a.habitat_id.ok_or("Habitat identity required")?;
    let system=u.habitats.iter().find(|h|h.id==id).ok_or("No such habitat")?.definition.body.system;
    let requested_kg=a.material_kg.ok_or("Material charge required")?;
    let edit=cosmogon_sim::sandbox::Edit::SupplyHabitatConstructionMaterial {system,id,requested_kg};
    let action=serde_json::json!({"format":"cosmogon-construction-supply","schema_version":1,"edit":edit,"before_state_fingerprint":cosmogon_sim::research::state_fingerprint(&before)?});
    u.apply_edit(edit)?;
    let p=std::path::Path::new(a.output.as_deref().ok_or("New output folder required")?);
    let row=cosmogon_sim::habitat::sample(&u,id)?;
    cosmogon_sim::habitat::export(&u,&u,id,&[row],p)?;
    for (name,text) in [("before-construction-supply.cosmo",save::to_json(&before,"Before explicit construction material supply").map_err(|e|e.to_string())?),("supply-action.json",serde_json::to_string_pretty(&action).map_err(|e|e.to_string())?)] {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p.join(name)).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{}",serde_json::to_string_pretty(&u.habitats.iter().find(|h|h.id==id).expect("supplied").report()).map_err(|e|e.to_string())?);Ok(())
}

/// Atomic conserved transfer of the four finite supply stocks between two
/// habitats of the same body. Per-species accepted amounts are bounded by the
/// sender's held stock and the recipient's free storage; refusal leaves both
/// sides unchanged and both habitats keep paired retained records.
fn habitat_stock_transfer(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=save::read_file(std::path::Path::new(a.initial.as_deref().ok_or("Initial world required")?)).map_err(|e|e.to_string())?;
    let before=u.clone();let from=a.from_id.ok_or("Source habitat identity required")?;let to=a.to_id.ok_or("Recipient habitat identity required")?;
    let system=u.habitats.iter().find(|h|h.id==from).ok_or("No such source habitat")?.definition.body.system;
    let requested=cosmogon_sim::supply_vehicle::Cargo {
        oxygen_reserve_kg:a.oxygen_kg.unwrap_or(0.0),
        potable_water_kg:a.water_kg.unwrap_or(0.0),
        food_dry_kg:a.food_dry_kg.unwrap_or(0.0),
        food_water_kg:a.food_water_kg.unwrap_or(0.0),
    };
    let edit=cosmogon_sim::sandbox::Edit::TransferHabitatStock {system,from,to,requested:requested.clone()};
    let action=serde_json::json!({"format":"cosmogon-habitat-stock-transfer","schema_version":1,"edit":edit,"before_state_fingerprint":cosmogon_sim::research::state_fingerprint(&before)?});
    u.apply_edit(edit)?;
    let p=std::path::Path::new(a.output.as_deref().ok_or("New output folder required")?);
    let row=cosmogon_sim::habitat::sample(&u,to)?;
    cosmogon_sim::habitat::export(&u,&u,to,&[row],p)?;
    for (name,text) in [("before-stock-transfer.cosmo",save::to_json(&before,"Before explicit habitat stock transfer").map_err(|e|e.to_string())?),("transfer-action.json",serde_json::to_string_pretty(&action).map_err(|e|e.to_string())?)] {let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p.join(name)).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    let sender=u.habitats.iter().find(|h|h.id==from).expect("source");let recipient=u.habitats.iter().find(|h|h.id==to).expect("recipient");
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"format":"cosmogon-habitat-stock-transfer-result","schema_version":1,"requested":requested,"moved":sender.stock_transfers.last().map(|r|&r.moved),"source_report":sender.report(),"recipient_report":recipient.report()})).map_err(|e|e.to_string())?);Ok(())
}

/// Create a world containing one finite habitat, saved as a protected
/// `.cosmo` file. This is the headless entry point for habitat-bearing
/// worlds used by flight transfer studies.
fn habitat_setup(a:&Args)->Result<(),String> {
    use std::io::Write;
    let mut u=if let Some(path)=&a.initial {save::read_file(std::path::Path::new(path)).map_err(|e|e.to_string())?} else {Universe::new(settings(a,a.seed))};
    let r=u.systems.iter().find_map(|s|s.bodies.iter().enumerate().find(|(_,b)|b.exists() && b.kind.has_surface() && b.name.eq_ignore_ascii_case(&a.target)).map(|(j,_)|cosmogon_sim::BodyRef {system:s.id,body:j as u32})).ok_or("Target must identify an existing solid world")?;
    let d=cosmogon_sim::habitat::Definition::at(r,a.latitude,a.longitude);
    d.validate()?;
    u.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(d)})?;
    let habitat=u.habitats.last().expect("created");
    let bytes=save::to_json(&u,"Habitat setup world").map_err(|e|e.to_string())?;
    let path=std::path::Path::new(a.output.as_deref().ok_or("--output NEW_FILE required")?);
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|format!("Existing saves are protected: {e}"))?;
    file.write_all(bytes.as_bytes()).map_err(|e|e.to_string())?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"format":"cosmogon-habitat-setup","schema_version":1,"target":u.systems[r.system as usize].bodies[r.body as usize].name,"latitude_north_deg":a.latitude,"longitude_east_deg":a.longitude,"habitat_id":habitat.id,"habitat":habitat.report(),"world":path.display().to_string()})).map_err(|e|e.to_string())?);Ok(())
}

fn launch_definition(a:&Args)->Result<(),String> {
    use cosmogon_sim::supply_vehicle::{Command,Plan};
    let world=if let Some(path)=&a.initial {cosmogon_sim::world_vehicle_recording::read_world(std::path::Path::new(path))?} else {Universe::new(settings(a,a.seed))};
    let parent=world.systems.iter().enumerate().find_map(|(i,s)|s.find_body(&a.target).map(|j|cosmogon_sim::BodyRef {system:i as u32,body:j as u32})).ok_or("No matching launch starting body")?;
    let thrust_time=a.thrust_time.ok_or("launch-definition requires --thrust-time")?;
    let thrust_newtons=a.thrust_newtons.ok_or("launch-definition requires --thrust-newtons")?;
    // Resolve the pad on the actual local terrain: probe the site, then start
    // a small fixed clearance above the terrain along the local radial.
    let mut start=cosmogon_sim::vehicle_arrival::GeographicStart::contact_example();
    start.latitude_deg=a.latitude;
    start.longitude_east_deg=a.longitude;
    start.east_velocity_m_s=0.;
    start.north_velocity_m_s=0.;
    start.up_velocity_m_s=0.;
    start.spherical_altitude_m=10000.;
    let above=start.absolute(&world,parent)?;
    let probe=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,above.position_m,1.)?;
    start.spherical_altitude_m=10000.-probe.radial_clearance_m+1.6;
    let pad=start.absolute(&world,parent)?;
    let surface=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,pad.position_m,1.)?;
    let up=surface.outward_normal;
    let mut plan=Plan::example();
    plan.definition.rated_thrust_n=thrust_newtons;
    plan.commands=vec![Command {duration_s:thrust_time,throttle:1.,direction:[up.x,up.y,up.z]}];
    let hardware=cosmogon_sim::vehicle_contact_dynamics::Hardware::example(&world,parent,pad.position_m)?;
    let mut definition=start.contact_flight(&world,parent,plan,hardware)?;
    let limits=cosmogon_sim::vehicle_launch::Limits {
        maximum_acceleration_m_s2:a.maximum_acceleration.unwrap_or(15.),
        liftoff_altitude_m:a.liftoff_altitude.unwrap_or(1.),
        maximum_start_altitude_m:a.maximum_start_altitude.unwrap_or(100.),
    };
    let peak=thrust_newtons/definition.plan.definition.inert_mass_kg();
    if peak>limits.maximum_acceleration_m_s2 {
        return Err(format!("Authored thrust {thrust_newtons} N on inert mass {} kg senses {peak:.3} m/s², above the structural limit {} m/s²; lower --thrust-newtons or raise --maximum-acceleration",definition.plan.definition.inert_mass_kg(),limits.maximum_acceleration_m_s2));
    }
    definition.launch=Some(limits);
    definition.validate(&world)?;
    cosmogon_sim::world_vehicle::Flight::new(0,definition.clone(),&world)?;
    let path=std::path::Path::new(a.output.as_deref().ok_or("--output NEW_FILE required")?);
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    definition.write(path)?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "format":"cosmogon-launch-definition","schema_version":1,
        "target":world.systems[parent.system as usize].bodies[parent.body as usize].name,
        "latitude_north_deg":a.latitude,"longitude_east_deg":a.longitude,
        "thrust_time_s":thrust_time,"thrust_newtons":thrust_newtons,
        "burn_direction":"Fixed inertial local zenith at ignition; no attitude controller",
        "limits":definition.launch.expect("written limits"),
        "definition":path.display().to_string(),
        "scope":"Authored engine-specific-force limits and trajectory evidence on the existing contact solver; not atmospheric ascent losses, pad hazards or mission feasibility"
    })).map_err(|e|e.to_string())?);Ok(())
}

fn entry_definition(a:&Args)->Result<(),String> {
    use cosmogon_sim::supply_vehicle::{Command,Plan};
    let world=if let Some(path)=&a.initial {cosmogon_sim::world_vehicle_recording::read_world(std::path::Path::new(path))?} else {Universe::new(settings(a,a.seed))};
    let parent=world.systems.iter().enumerate().find_map(|(i,s)|s.find_body(&a.target).map(|j|cosmogon_sim::BodyRef {system:i as u32,body:j as u32})).ok_or("No matching entry starting body")?;
    let mut start=cosmogon_sim::vehicle_arrival::GeographicStart::contact_example();
    start.latitude_deg=a.latitude;
    start.longitude_east_deg=a.longitude;
    start.east_velocity_m_s=0.;
    start.north_velocity_m_s=0.;
    start.up_velocity_m_s=0.;
    start.spherical_altitude_m=10000.;
    let above=start.absolute(&world,parent)?;
    let probe=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,above.position_m,1.)?;
    start.spherical_altitude_m=10000.-probe.radial_clearance_m+1.6;
    let pad=start.absolute(&world,parent)?;
    let surface=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,pad.position_m,1.)?;
    let up=surface.outward_normal;
    let mut plan=Plan::example();
    plan.commands=vec![Command {duration_s:5_000.,throttle:0.,direction:[up.x,up.y,up.z]}];
    let hardware=cosmogon_sim::vehicle_contact_dynamics::Hardware::example(&world,parent,pad.position_m)?;
    let mut definition=start.contact_flight(&world,parent,plan,hardware)?;
    let entry=cosmogon_sim::vehicle_entry::Entry {
        entry_interface_altitude_m:a.entry_interface_altitude.unwrap_or(125_000.),
        nose_radius_m:a.nose_radius.unwrap_or(1.),
        heat_flux_coefficient:a.heat_flux_coefficient.unwrap_or(1.7415e-4),
        maximum_heat_flux_w_m2:a.maximum_heat_flux.unwrap_or(1e7),
        maximum_total_heat_j_m2:a.maximum_total_heat.unwrap_or(1e10),
    };
    entry.validate()?;
    definition.entry=Some(entry);
    definition.validate(&world)?;
    cosmogon_sim::world_vehicle::Flight::new(0,definition.clone(),&world)?;
    let path=std::path::Path::new(a.output.as_deref().ok_or("--output NEW_FILE required")?);
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    definition.write(path)?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "format":"cosmogon-entry-definition","schema_version":1,
        "target":world.systems[parent.system as usize].bodies[parent.body as usize].name,
        "latitude_north_deg":a.latitude,"longitude_east_deg":a.longitude,
        "limits":definition.entry.expect("written limits"),
        "definition":path.display().to_string(),
        "scope":"Authored Sutton-Graves stagnation heat flux with thermal limits on the existing exponential atmosphere and flight clock; not aerodynamic forces, attitude, or trajectory control"
    })).map_err(|e|e.to_string())?);Ok(())
}

fn landing_definition(a:&Args)->Result<(),String> {
    use cosmogon_sim::supply_vehicle::{Command,Plan};
    let world=if let Some(path)=&a.initial {cosmogon_sim::world_vehicle_recording::read_world(std::path::Path::new(path))?} else {Universe::new(settings(a,a.seed))};
    let parent=world.systems.iter().enumerate().find_map(|(i,s)|s.find_body(&a.target).map(|j|cosmogon_sim::BodyRef {system:i as u32,body:j as u32})).ok_or("No matching landing starting body")?;
    let mut start=cosmogon_sim::vehicle_arrival::GeographicStart::contact_example();
    start.latitude_deg=a.latitude;
    start.longitude_east_deg=a.longitude;
    start.east_velocity_m_s=0.;
    start.north_velocity_m_s=0.;
    start.up_velocity_m_s=0.;
    start.spherical_altitude_m=10000.;
    let above=start.absolute(&world,parent)?;
    let probe=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,above.position_m,1.)?;
    start.spherical_altitude_m=10000.-probe.radial_clearance_m+1.6;
    let pad=start.absolute(&world,parent)?;
    let surface=cosmogon_sim::vehicle_contact::Surface::sample(&world,parent,world.time,pad.position_m,1.)?;
    let up=surface.outward_normal;
    let mut plan=Plan::example();
    plan.commands=vec![Command {duration_s:5_000.,throttle:0.,direction:[up.x,up.y,up.z]}];
    let hardware=cosmogon_sim::vehicle_contact_dynamics::Hardware::example(&world,parent,pad.position_m)?;
    let mut definition=start.contact_flight(&world,parent,plan,hardware)?;
    let limits=cosmogon_sim::vehicle_landing::Limits {
        maximum_touchdown_vertical_speed_m_s:a.maximum_touchdown_vertical.unwrap_or(10.),
        maximum_touchdown_horizontal_speed_m_s:a.maximum_touchdown_horizontal.unwrap_or(5.),
        liftoff_altitude_m:a.landing_liftoff_altitude.unwrap_or(10.),
    };
    limits.validate()?;
    definition.landing=Some(limits);
    definition.validate(&world)?;
    cosmogon_sim::world_vehicle::Flight::new(0,definition.clone(),&world)?;
    let path=std::path::Path::new(a.output.as_deref().ok_or("--output NEW_FILE required")?);
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    definition.write(path)?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "format":"cosmogon-landing-definition","schema_version":1,
        "target":world.systems[parent.system as usize].bodies[parent.body as usize].name,
        "latitude_north_deg":a.latitude,"longitude_east_deg":a.longitude,
        "limits":definition.landing.expect("written limits"),
        "definition":path.display().to_string(),
        "scope":"Authored touchdown-speed limits and a versioned liftoff baseline on the existing bounded gear solver; not terrain hazards, structural certification, or tip-over modeling"
    })).map_err(|e|e.to_string())?);Ok(())
}

fn habitat_definition(a:&Args)->Result<(),String> {
    use std::io::Write;
    let u=if let Some(p)=&a.initial {save::read_file(std::path::Path::new(p)).map_err(|e|e.to_string())?} else {Universe::new(settings(a,a.seed))};
    let r=u.systems.iter().find_map(|s|s.bodies.iter().enumerate().find(|(_,b)|b.exists() && b.kind.has_surface() && b.name.eq_ignore_ascii_case(&a.target)).map(|(j,_)|cosmogon_sim::BodyRef {system:s.id,body:j as u32})).ok_or("Target must identify an existing solid world")?;
    let d=if a.cmd=="habitat-crop-definition" {cosmogon_sim::crop_process::definition_at(r,a.latitude,a.longitude)} else {cosmogon_sim::habitat::Definition::at(r,a.latitude,a.longitude)};d.validate()?;
    let text=serde_json::to_string_pretty(&d).map_err(|e|e.to_string())?;
    if let Some(path)=&a.output {let p=std::path::Path::new(path);if let Some(parent)=p.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(p).map_err(|e|format!("Existing definitions are protected: {e}"))?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;}
    println!("{text}");Ok(())
}

#[cfg(test)]
mod argument_tests {
    use super::*;
    fn args(values: &[&str]) -> Result<Args, String> {
        parse_args(values.iter().map(|s| s.to_string()))
    }

    #[test]
    fn support_commands_refuse_ignored_flags_and_missing_sources() {
        for values in [vec!["flight-support"],vec!["support-inspect"],vec!["flight-support","--recording"," "],vec!["flight-support","--recording","r","--definition"," "],vec!["flight-support","--recording","r","--target","Mars"],vec!["support-inspect","--recording","r","--output","o"],vec!["support-definition","--definition","d"],vec!["support-definition","--output"," "]] {assert!(args(&values).is_err());}
        assert!(args(&["support-definition"]).is_ok());assert!(args(&["flight-support","--recording","r","--definition","d","--output","o"]).is_ok());assert!(args(&["support-inspect","--recording","r"]).is_ok());
    }
    #[test]
    fn contact_arguments_require_explicit_typed_files_and_reject_ignored_inputs() {
        for values in [vec!["contact-hardware","--definition","ignored"],vec!["contact-hardware","--output"," "],vec!["contact-hardware","--initial","s","--seed","2"],vec!["flight-study","--definition","d","--contact-hardware","h"],vec!["flight-definition","--contact-hardware"," "]] {assert!(args(&values).is_err());}
        assert!(args(&["contact-hardware","--target","Mars","--geographic-start","g","--output","new"]).is_ok());
        assert!(args(&["contact-start","--output","new"]).is_ok());assert!(args(&["contact-start","--target","Mars"]).is_err());
        assert!(args(&["contact-hardware","--static-shear"]).unwrap().static_shear);
        assert!(args(&["contact-hardware","--static-shear","true"]).is_err());
        assert!(args(&["flight-definition","--static-shear"]).is_err());
        assert_eq!(args(&["contact-hardware","--operation-window","5"]).unwrap().operation_window,Some(5.));
        assert!(args(&["contact-hardware","--operation-window"]).is_err());
        assert!(args(&["contact-hardware","--operation-window","-1"]).is_err());
        assert!(args(&["flight-definition","--operation-window","5"]).is_err());
        assert!(args(&["flight-definition","--geographic-start","g","--contact-hardware","h","--samples","5","--interval-seconds","0.5"]).is_ok());
    }
    #[test]
    fn arrival_commands_require_complete_sources_and_reject_ignored_controls() {
        for v in [vec!["flight-approach"],vec!["approach-inspect"],vec!["flight-approach","--recording","r","--initial","s"],vec!["approach-inspect","--recording","r","--output","o"],vec!["arrival-start","--target","Moon"],vec!["flight-study","--definition","d","--geographic-start","g"],vec!["flight-definition","--geographic-start"," "]] {assert!(args(&v).is_err());}
        assert!(args(&["arrival-start","--output","new"]).is_ok());assert!(args(&["flight-approach","--recording","r","--target","Moon","--output","new"]).is_ok());assert!(args(&["flight-definition","--target","Moon","--geographic-start","g"]).is_ok());
    }
    #[test]
    fn flight_commands_refuse_missing_definitions_and_ignored_origin_controls() {
        for v in [vec!["flight-study"],vec!["flight-inspect"],vec!["flight-study","--definition"," "],vec!["flight-inspect","--recording"," "],vec!["flight-inspect","--recording","r","--output","new"],vec!["flight-study","--definition","d","--samples","3"],vec!["flight-definition","--initial","s","--seed","2"],vec!["flight-definition","--interval-days","1"]] {assert!(args(&v).is_err());}
        assert!(args(&["flight-definition","--samples","9","--interval-seconds","0.5","--target","Mars"]).is_ok());assert!(args(&["flight-study","--definition","d","--initial","s","--output","new"]).is_ok());assert!(args(&["flight-inspect","--recording","r"]).is_ok());
        assert!(args(&["flight-definition","--mission-span"]).is_err());
        assert!(args(&["flight-definition","--mission-span","--launch","l"]).is_ok());
        assert!(args(&["flight-definition","--transfer","t"]).is_ok());
        assert!(args(&["flight-study","--mission-span"]).is_err());
        assert!(args(&["flight-definition","--mission-span","--launch","l","--transfer","t"]).unwrap().mission_span);
        assert_eq!(args(&["flight-definition","--transfer","t1","--transfer","t2"]).unwrap().transfer,vec!["t1".to_string(),"t2".to_string()]);
    }
    #[test]
    fn transfer_arguments_are_opt_in_and_refuse_ignored_flags() {
        assert!(args(&["flight-definition","--handling"]).unwrap().handling);
        assert!(args(&["flight-study","--definition","d","--handling"]).is_err());
        for v in [vec!["habitat-setup"],vec!["habitat-setup","--samples","3"],vec!["habitat-setup","--output","o","--habitat-id","0"],vec!["habitat-setup","--output","o","--initial","s","--seed","2"]] {assert!(args(&v).is_err(),"{v:?}");}
        let a=args(&["habitat-setup","--target","Mars","--latitude","-4.59","--longitude","137.44","--output","world.cosmo"]).unwrap();
        assert_eq!(a.target,"Mars");assert_eq!(a.latitude,-4.59);assert_eq!(a.longitude,137.44);assert_eq!(a.output.as_deref(),Some("world.cosmo"));
        assert!(args(&["flight-definition","--initial","w","--handling","--cargo-ledgers","--geographic-start","g"]).is_ok());
    }
    #[test]
    fn launch_arguments_are_explicit_and_refuse_ignored_flags() {
        for v in [vec!["launch-definition"],vec!["launch-definition","--thrust-time","10"],vec!["launch-definition","--output","o"],vec!["launch-definition","--output","o","--thrust-time","10"],vec!["launch-definition","--output","o","--thrust-time","10","--thrust-newtons","-1"],vec!["launch-definition","--output","o","--thrust-time","10","--thrust-newtons","25000","--samples","3"],vec!["launch-definition","--output","o","--thrust-time","10","--thrust-newtons","25000","--initial","s","--seed","2"]] {assert!(args(&v).is_err(),"{v:?}");}
        let a=args(&["launch-definition","--output","launch.json","--thrust-time","10","--thrust-newtons","25000","--maximum-acceleration","15","--latitude","0","--longitude","0"]).unwrap();
        assert_eq!(a.output.as_deref(),Some("launch.json"));
        assert_eq!(a.thrust_time,Some(10.));assert_eq!(a.thrust_newtons,Some(25000.));assert_eq!(a.maximum_acceleration,Some(15.));
        assert_eq!(a.target,"Mars");
        assert!(args(&["flight-definition","--launch","launch.json"]).is_ok());
        assert!(args(&["flight-definition","--launch","launch.json","--geographic-start","g"]).is_err());
        assert!(args(&["flight-definition","--launch","launch.json","--contact-hardware","h"]).is_err());
        assert!(args(&["flight-definition","--launch","launch.json","--definition","p"]).is_err());
        assert!(args(&["flight-study","--definition","d","--launch","launch.json"]).is_err());
    }
    #[test]
    fn entry_and_landing_arguments_are_explicit_and_refuse_ignored_flags() {
        for v in [vec!["entry-definition"],vec!["landing-definition"],vec!["entry-definition","--output","e","--samples","3"],vec!["landing-definition","--output","l","--definition","p"],vec!["entry-definition","--output","e","--entry-interface-altitude","-1"],vec!["entry-definition","--output","e","--nose-radius","0"],vec!["landing-definition","--output","l","--maximum-touchdown-vertical","-2"],vec!["landing-definition","--output","l","--liftoff-altitude","1e9"],vec!["entry-definition","--output","e","--initial","s","--seed","2"]] {assert!(args(&v).is_err(),"{v:?}");}
        assert!(args(&["entry-definition","--output","e"]).is_ok());
        assert!(args(&["landing-definition","--output","l"]).is_ok());
        let e=args(&["entry-definition","--output","entry.json","--entry-interface-altitude","110000","--nose-radius","0.5","--maximum-heat-flux","5e6"]).unwrap();
        assert_eq!(e.output.as_deref(),Some("entry.json"));
        assert_eq!(e.entry_interface_altitude,Some(110000.));assert_eq!(e.nose_radius,Some(0.5));assert_eq!(e.maximum_heat_flux,Some(5e6));
        assert_eq!(e.target,"Mars");
        let l=args(&["landing-definition","--output","landing.json","--maximum-touchdown-vertical","8","--liftoff-altitude","5"]).unwrap();
        assert_eq!(l.maximum_touchdown_vertical,Some(8.));assert_eq!(l.landing_liftoff_altitude,Some(5.));
        assert!(args(&["flight-definition","--entry","entry.json"]).is_err());
        assert!(args(&["flight-definition","--landing","landing.json"]).is_err());
        assert!(args(&["flight-definition","--entry","entry.json","--contact-hardware","h"]).is_ok());
        assert!(args(&["flight-definition","--landing","landing.json","--contact-hardware","h"]).is_ok());
        assert!(args(&["flight-definition","--entry"," ","--contact-hardware","h"]).is_err());
        assert!(args(&["flight-study","--definition","d","--entry","e"]).is_err());
        assert!(args(&["flight-study","--definition","d","--landing","l"]).is_err());
    }
    #[test]
    fn rendezvous_arguments_are_explicit_and_refuse_ignored_flags() {
        for v in [vec!["flight-rendezvous"],vec!["flight-rendezvous","--recording","l"],vec!["flight-rendezvous","--reference","r"],vec!["flight-rendezvous","--recording","l","--reference"," "],vec!["flight-rendezvous","--recording"," ","--reference","r"],vec!["flight-rendezvous","--recording","l","--reference","r","--output"," "],vec!["flight-rendezvous","--recording","l","--reference","r","--target","Mars"],vec!["flight-rendezvous","--recording","l","--reference","r","--samples","3"],vec!["flight-rendezvous","--recording","l","--reference","r","--initial","w"]] {assert!(args(&v).is_err(),"{v:?}");}
        let a=args(&["flight-rendezvous","--recording","left","--reference","right","--output","out"]).unwrap();
        assert_eq!(a.recording.as_deref(),Some("left"));
        assert_eq!(a.reference.as_deref(),Some("right"));
        assert_eq!(a.output.as_deref(),Some("out"));
    }
    #[test]
    fn vehicle_commands_refuse_missing_recordings_and_unrelated_world_controls() {
        for v in [vec!["vehicle-inspect"],vec!["vehicle-inspect","--recording"," "],vec!["vehicle-study","--initial","s"],vec!["vehicle-study","--samples","5"],vec!["vehicle-definition","--seed","2"],vec!["vehicle-study","--output"," "],vec!["vehicle-inspect","--recording","x","--output","y"]] {assert!(args(&v).is_err());}
        assert!(args(&["vehicle-study","--definition","plan","--output","new"]).is_ok());assert!(args(&["vehicle-definition","--output","new"]).is_ok());assert!(args(&["vehicle-inspect","--recording","study"]).is_ok());
        for v in [vec!["cargo-epochs-inspect"],vec!["cargo-epochs-inspect","--recording"," "],vec!["cargo-epochs-study","--initial","s"],vec!["cargo-epochs-definition","--seed","2"],vec!["cargo-epochs-study","--output"," "],vec!["cargo-epochs-study","--definition"," "],vec!["cargo-epochs-inspect","--recording","x","--output","y"]] {assert!(args(&v).is_err());}
        assert!(args(&["cargo-epochs-study","--definition","plan","--output","new"]).is_ok());assert!(args(&["cargo-epochs-definition","--output","new"]).is_ok());assert!(args(&["cargo-epochs-inspect","--recording","study"]).is_ok());
    }
    #[test]
    fn saved_origin_reruns_refuse_ignored_or_incomplete_inputs() {
        for v in [vec!["habitat-rerun"],vec!["habitat-rerun","--recording","x"],vec!["habitat-rerun","--recording","x","--output"," "],vec!["habitat-rerun","--recording","x","--output","new","--initial","other"],vec!["habitat-rerun","--recording","x","--output","new","--samples","2"]] {assert!(args(&v).is_err());}
        let a=args(&["habitat-rerun","--recording","x","--output","new","--definition","plan"]).unwrap();assert_eq!(a.definition.as_deref(),Some("plan"));
    }
    #[test]
    fn installed_crop_commands_refuse_ignored_or_ambiguous_inputs() {
        assert!(args(&["habitat-crop-definition","--output","x"]).is_ok());
        assert!(args(&["habitat-crop-definition","--samples","2"]).is_err());
        assert!(args(&["habitat-crop-definition","--initial","s","--seed","4"]).is_err());
        for v in [vec!["habitat-crop-harvest"],vec!["habitat-crop-harvest","--initial","s","--habitat-id","0"],vec!["habitat-crop-harvest","--initial","s","--habitat-id","0","--output","o","--samples","3"]] {assert!(args(&v).is_err());}
        assert!(args(&["habitat-crop-harvest","--initial","s","--habitat-id","0","--output","o"]).is_ok());
        for v in [vec!["habitat-crop-install"],vec!["habitat-crop-install","--initial","s","--habitat-id","0"],vec!["habitat-crop-install","--initial","s","--habitat-id","0","--definition","c","--output","o"],vec!["habitat-crop-install","--initial","s","--habitat-id","0","--definition","c","--nutrient-kg","2"],vec!["habitat-crop-install","--initial","s","--habitat-id","0","--definition","c","--nutrient-kg","-1","--output","o"],vec!["habitat-crop-install","--initial","s","--habitat-id","0","--definition","c","--nutrient-kg","2","--output","o","--samples","3"],vec!["habitat-crop-store"],vec!["habitat-crop-store","--initial","s","--habitat-id","0","--store-kg","1"],vec!["habitat-crop-store","--initial","s","--habitat-id","0","--output","o","--definition","c"],vec!["habitat-crop-store","--store-kg","1"]] {assert!(args(&v).is_err(),"{v:?}");}
        let i=args(&["habitat-crop-install","--initial","s","--habitat-id","2","--definition","crop.json","--nutrient-kg","2.5","--output","o"]).unwrap();
        assert_eq!(i.definition.as_deref(),Some("crop.json"));assert_eq!(i.nutrient_kg,Some(2.5));assert_eq!(i.habitat_id,Some(2));
        let s=args(&["habitat-crop-store","--initial","s","--habitat-id","0","--store-kg","1","--output","o"]).unwrap();
        assert_eq!(s.store_kg,Some(1.));
        assert!(args(&["habitat-crop-store","--initial","s","--habitat-id","0","--output","o"]).is_ok());
    }
    #[test]
    fn settlement_commands_refuse_ignored_or_incomplete_inputs() {
        for v in [
            vec!["habitat-construction-supply"],
            vec!["habitat-construction-supply","--initial","s","--habitat-id","0"],
            vec!["habitat-construction-supply","--initial","s","--habitat-id","0","--material-kg","50"],
            vec!["habitat-construction-supply","--initial","s","--habitat-id","0","--material-kg","-1","--output","o"],
            vec!["habitat-construction-supply","--initial","s","--habitat-id","0","--material-kg","50","--output","o","--samples","3"],
            vec!["habitat-construction-supply","--store-kg","1"],
            vec!["habitat-stock-transfer"],
            vec!["habitat-stock-transfer","--initial","s","--from-id","0","--to-id","1","--output","o"],
            vec!["habitat-stock-transfer","--initial","s","--from-id","0","--output","o","--oxygen-kg","10"],
            vec!["habitat-stock-transfer","--initial","s","--from-id","0","--to-id","1","--oxygen-kg","-5","--output","o"],
            vec!["habitat-stock-transfer","--initial","s","--from-id","0","--to-id","1","--oxygen-kg","10","--output","o","--habitat-id","3"],
        ] {
            assert!(args(&v).is_err(), "{v:?}");
        }
        let c = args(&[
            "habitat-construction-supply",
            "--initial",
            "s",
            "--habitat-id",
            "7",
            "--material-kg",
            "125.5",
            "--output",
            "o",
        ])
        .unwrap();
        assert_eq!(c.material_kg, Some(125.5));
        assert_eq!(c.habitat_id, Some(7));
        let t = args(&[
            "habitat-stock-transfer",
            "--initial",
            "s",
            "--from-id",
            "0",
            "--to-id",
            "1",
            "--water-kg",
            "30",
            "--food-dry-kg",
            "5",
            "--output",
            "o",
        ])
        .unwrap();
        assert_eq!(t.from_id, Some(0));
        assert_eq!(t.to_id, Some(1));
        assert_eq!(t.water_kg, Some(30.0));
        assert_eq!(t.food_dry_kg, Some(5.0));
        assert!(t.oxygen_kg.is_none());
    }
    #[test]
    fn crop_chemistry_requires_explicit_source_and_refuses_ignored_arguments() {
        for cmd in ["crop-chemistry","crop-chemistry-inspect"] {
            assert!(args(&[cmd]).is_err());
            assert!(args(&[cmd,"--recording",""]).is_err());
            for flag in ["--initial","--definition","--samples","--scenario","--habitat-id"] {assert!(args(&[cmd,"--recording","x",flag,"1"]).is_err());}
            assert!(args(&[cmd,"--recording","x","--output","y"]).is_ok());
        }
    }
    #[test]
    fn balance_arguments_are_explicit_and_ordered() {
        for v in [vec!["habitat-balance"],vec!["habitat-balance","--recording","x","--from-index","nan"],vec!["habitat-balance","--recording","x","--from-index","1024"],vec!["habitat-balance","--recording","x","--from-index","3","--observation","2"],vec!["habitat-balance","--recording","x","--samples","3"],vec!["habitat-balance","--recording","x","--reference","b"]] {assert!(args(&v).is_err());}
        let a=args(&["habitat-balance","--recording","x","--from-index","2","--observation","3","--output","new"]).unwrap();assert_eq!(a.from_index,Some(2));assert_eq!(a.observation,Some(3));
    }
    #[test]
    fn figures_require_exact_layout_recording_and_new_output_and_refuse_ignored_flags() {
        for v in [vec!["habitat-figure"],vec!["habitat-figure","--recording","x","--output","o"],vec!["habitat-figure","--recording","x","--definition","d"],vec!["habitat-figure","--recording","x","--definition","d","--output","o","--samples","3"],vec!["habitat-figure","--recording","x","--definition","d","--output","o","--measure","water"]] {assert!(args(&v).is_err());}
        let a=args(&["habitat-figure","--recording","x","--definition","d","--output","o","--reference","b"]).unwrap();assert_eq!(a.reference.as_deref(),Some("b"));
    }
    #[test]
    fn diagnosis_requires_exact_saved_context_and_refuses_ignored_options() {
        for v in [vec!["habitat-diagnose"],vec!["habitat-diagnose","--initial","x"],vec!["habitat-diagnose","--habitat-id","0"],vec!["habitat-diagnose","--initial","x","--habitat-id","0","--samples","3"],vec!["habitat-diagnose","--initial","x","--habitat-id","0","--scenario","sol"]] {assert!(args(&v).is_err());}
        let a=args(&["habitat-diagnose","--initial","x","--habitat-id","0","--output","new"]).unwrap();assert_eq!(a.habitat_id,Some(0));assert_eq!(a.output.as_deref(),Some("new"));
    }
    #[test]
    fn source_selection_is_explicit_and_rejects_ignored_options() {
        for values in [vec!["surface","--mars-terrain","32"],vec!["run","--mars-terrain","16"],vec!["terrain","--mars-terrain","16"],vec!["terrain","--save","x"],vec!["terrain","--mars-terrain","4","--save","x","--target","Earth"]] {assert!(args(&values).is_err());}
        assert_eq!(args(&["surface","--site","curiosity","--mars-terrain","16"]).unwrap().mars_terrain.as_deref(),Some("16"));
        assert!(args(&["terrain","--mars-terrain","16","--save","x"]).is_ok());
    }
    #[test]
    fn malformed_requests_return_errors_and_research_defaults_to_lab() {
        for values in [
            vec!["sweep", "--samples", "bad"],
            vec!["sweep", "--target"],
            vec!["run", "--years", "NaN"],
            vec!["run", "--systems", "999999999"],
            vec!["run", "--scenario", "missing"],
        ] {
            assert!(args(&values).is_err());
        }
        assert_eq!(args(&["sweep"]).unwrap().scenario, Scenario::SolarSystemLab);
        assert_eq!(args(&["run"]).unwrap().scenario, Scenario::Sol);
        assert_eq!(
            args(&["sweep", "--scenario", "sol"]).unwrap().scenario,
            Scenario::Sol
        );
        assert_eq!(args(&["sweep", "--help"]).unwrap().cmd, "help");
    }
    #[test]
    fn crop_supply_commands_refuse_ignored_or_missing_requests() {
        for a in [vec!["greenhouse-supplies","--samples","2"],vec!["greenhouse-supplies","--habitat-id","0"],vec!["greenhouse-supply-definition","--initial","x"],vec!["greenhouse-supply-inspect"],vec!["greenhouse-supply-inspect","--recording","x","--initial","y"]] {assert!(args(&a).is_err());}
        assert!(args(&["greenhouse-supplies","--definition","x","--initial","y","--habitat-id","0","--output","z"]).is_ok());
        assert!(args(&["greenhouse-supply-inspect","--recording","x","--output","z"]).is_ok());
    }
    #[test]
    fn greenhouse_planning_rejects_ignored_world_and_time_options() {
        for values in [vec!["greenhouse","--samples","3"],vec!["greenhouse","--scenario","lab"],vec!["greenhouse","--habitat-id","0"],vec!["greenhouse-definition","--definition","x"]] {assert!(args(&values).is_err());}
        assert!(args(&["greenhouse","--initial","x","--habitat-id","0"]).is_ok());
        assert!(args(&["greenhouse","--definition","x","--output","new"]).is_ok());
    }
    #[test]
    fn habitat_studies_have_bounded_horizon_and_reject_ignored_options() {
        for values in [vec!["habitat"],vec!["habitat","--definition","x","--samples","0"],vec!["habitat","--definition","x","--samples","1025"],vec!["habitat","--definition","x","--interval-days","nan"],vec!["habitat","--definition","x","--interval-days","31"],vec!["habitat","--definition","x","--years","1"],vec!["habitat","--definition","x","--target","Mars"],vec!["habitat","--definition","x","--habitat-id","1"],vec!["run","--habitat-id","0"]] {assert!(args(&values).is_err());}
        let a=args(&["habitat","--definition","x"]).unwrap();assert_eq!(a.samples,25);assert_eq!(a.scenario,Scenario::SolarSystemLab);assert_eq!(a.interval_days,1.0/24.0);
        assert_eq!(args(&["habitat","--initial","x","--samples","1"]).unwrap().samples,1);
    }
    #[test]
    fn terrain_sections_reject_ignored_parameters_and_keep_definition_settings() {
        for values in [vec!["section","--samples","1"],vec!["section","--samples","1025"],vec!["section","--definition","saved.json","--samples","20"],vec!["section","--latitude","1"],vec!["section","--target","Earth"],vec!["run","--from-site","curiosity"]] {assert!(args(&values).is_err());}
        assert_eq!(args(&["section","--from-site","curiosity","--to-site","insight","--samples","1024"]).unwrap().samples,1024);
        assert_eq!(args(&["section","--definition","saved.json"]).unwrap().target,"Mars");
    }
    #[test]
    fn named_surface_probes_use_verified_coordinates_and_reject_conflicts() {
        let a=args(&["surface","--site","curiosity"]).unwrap();
        assert_eq!(a.target,"Mars");assert_eq!(a.latitude,-4.59);assert_eq!(a.longitude,137.44);
        for values in [vec!["surface","--site","unknown"],vec!["surface","--site","curiosity","--target","Earth"],vec!["surface","--site","curiosity","--latitude","0"],vec!["run","--site","curiosity"]] {assert!(args(&values).is_err());}
    }
    #[test]
    fn replay_verification_requires_a_folder_and_refuses_unrelated_options() {
        for v in [vec!["habitat-verify"],vec!["habitat-verify","--recording","x","--observation","0"],vec!["habitat-verify","--recording","x","--samples","3"],vec!["habitat-verify","--recording","x","--measure","water"],vec!["habitat-verify","--recording","x","--scenario","sol"]] {assert!(args(&v).is_err());}
        assert_eq!(args(&["habitat-verify","--recording","design","--output","new"]).unwrap().recording.as_deref(),Some("design"));
    }
    #[test]
    fn paired_inspection_options_are_bounded_and_do_not_apply_to_other_commands() {
        let a=args(&["habitat-inspect","--recording","design","--observation","0","--output","new"]).unwrap();assert_eq!(a.observation,Some(0));assert_eq!(a.output.as_deref(),Some("new"));
        for v in [vec!["habitat-inspect","--recording","x","--observation","1024"],vec!["habitat-inspect","--recording","x","--observation","-1"],vec!["inspect","--recording","x","--observation","1"],vec!["habitat-inspect","--recording","x","--observation"]] {assert!(args(&v).is_err());}
    }
    #[test]
    fn recorded_pressure_requires_explicit_observation_and_refuses_ignored_options() {
        for v in [vec!["habitat-pressure"],vec!["habitat-pressure","--recording","x"],vec!["habitat-pressure","--recording","x","--observation","1024"],vec!["habitat-pressure","--recording","x","--observation","0","--reference-index","-1"],vec!["habitat-pressure","--recording","x","--observation","0","--samples","3"],vec!["habitat-inspect","--recording","x","--reference-index","0"],vec!["run","--reference-index","0"]] {assert!(args(&v).is_err());}
        let a=args(&["habitat-pressure","--recording","branch","--observation","12","--reference-index","0","--output","new"]).unwrap();assert_eq!(a.observation,Some(12));assert_eq!(a.reference_index,Some(0));
    }
    #[test]
    fn habitat_inspection_requires_a_folder_and_rejects_ignored_options() {
        for v in [vec!["habitat-inspect"],vec!["habitat-inspect","--recording","x","--samples","3"],vec!["habitat-inspect","--recording","x","--scenario","sol"],vec!["habitat-inspect","--output","x"]] {assert!(args(&v).is_err());}
        assert_eq!(args(&["habitat-inspect","--recording","x"]).unwrap().recording.as_deref(),Some("x"));
    }

    #[test]
    fn habitat_sweep_and_comparison_reject_ignored_and_ambiguous_options() {
        for v in [vec!["habitat-sweep"],vec!["habitat-sweep","--initial","s","--output","o","--from","0","--to","1","--runs","17"],vec!["habitat-sweep","--initial","s","--output","o","--definition","p","--samples","25"],vec!["habitat-sweep","--initial","s","--output","o","--definition","p","--seed","2"],vec!["habitat-compare","--recording","a"],vec!["habitat-compare","--recording","a","--reference","b","--alignment","nearest"],vec!["habitat-compare","--recording","a","--reference","b","--samples","25"]] {assert!(args(&v).is_err(),"{:?}",v);}
        assert!(args(&["habitat-sweep","--initial","s","--output","o","--from","0","--to","4000","--samples","1"]).is_ok());
        assert!(args(&["habitat-sweep","--initial","s","--output","o","--definition","p","--habitat-id","0"]).is_ok());
        assert!(args(&["habitat-compare","--recording","a","--reference","b","--measure","methane","--alignment","world"]).is_ok());
    }

}
