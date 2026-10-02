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
    seed: u64,
    seeds: u64,
    scenario: Scenario,
    years: f64,
    life: usize,
    systems: u32,
    min_importance: u8,
    save: Option<String>,
}

fn parse() -> Args {
    let mut a = Args { cmd: "run".into(), seed: 1, seeds: 10, scenario: Scenario::Sol, years: 200_000.0, life: 0, systems: 24, min_importance: 3, save: None };
    let mut it = std::env::args().skip(1);
    if let Some(c) = it.next() {
        a.cmd = c;
    }
    while let Some(flag) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--seed" => a.seed = val().parse().expect("seed"),
            "--seeds" => a.seeds = val().parse().expect("seeds"),
            "--years" => a.years = val().parse().expect("years"),
            "--systems" => a.systems = val().parse().expect("systems"),
            "--min-importance" => a.min_importance = val().parse().expect("importance"),
            "--save" => a.save = Some(val()),
            "--scenario" => {
                a.scenario = match val().as_str() {
                    "neighbourhood" | "neighborhood" => Scenario::Neighbourhood,
                    "garden" => Scenario::GardenWorld,
                    "sol" => Scenario::Sol,
                    s => panic!("unknown scenario {s}"),
                }
            }
            "--life" => {
                let v = val();
                a.life = LIFE_PRESETS.iter().position(|p| p.0.eq_ignore_ascii_case(&v)).expect("life preset: realistic | hopeful | teeming")
            }
            f => panic!("unknown flag {f}"),
        }
    }
    a
}

fn settings(a: &Args, seed: u64) -> UniverseSettings {
    let (_, life, intel, _) = LIFE_PRESETS[a.life];
    UniverseSettings { seed, scenario: a.scenario, system_count: a.systems, life_rate: life, intelligence_rate: intel, ..Default::default() }
}

fn main() {
    let a = parse();
    match a.cmd.as_str() {
        "run" => run(&a),
        "check" => check(&a),
        "survey" => survey(&a),
        c => eprintln!("unknown command {c}; use run | check | survey"),
    }
}

fn run(a: &Args) {
    let t0 = Instant::now();
    let mut u = Universe::new(settings(a, a.seed));
    println!("Created {} in {:.2?}: {} systems, {} biospheres", a.scenario.label(), t0.elapsed(), u.systems.len(), u.biospheres.len());
    let t1 = Instant::now();
    u.advance_by(a.years * SECONDS_PER_YEAR);
    println!("Simulated {} in {:.2?}\n", format_duration(a.years * SECONDS_PER_YEAR), t1.elapsed());
    for e in u.history.events.iter().filter(|e| e.importance >= a.min_importance) {
        println!("[{:>26}] {:<12} {}{}", cosmogon_sim::time::format_date(e.time, u.start_time, u.gregorian()), e.category.label(), e.title, if e.importance >= 5 { "  ★" } else { "" });
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
        let k: Vec<String> = cosmogon_sim::civ::knowledge::Domain::ALL.iter().map(|d| format!("{}={:.2e}", &d.name()[..4], c.knowledge[d.index()])).collect();
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
        let best = u.civs.iter().map(|c| c.discoveries.len()).max().unwrap_or(0);
        println!("seed {seed:>3}: civs {} (most techs {best}), events {}", u.civs.len(), u.history.events.len());
    }
    println!("\nuniverses with microbial life {life}/{0}, complex life {complex}/{0}, intelligence {intelligent}/{0}", a.seeds);
}
