//! Drives the authoritative simulation from the app: creation/loading on background
//! threads, per-frame advancement under a CPU budget, autosave and milestone toasts.

use std::time::Duration;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use cosmogon_sim::time::{SECONDS_PER_YEAR, SPEEDS};
use cosmogon_sim::universe::AdvanceReport;
use cosmogon_sim::{save, BodyRef, Universe, UniverseSettings};

use crate::args::Args;
use crate::persistence::{saves_dir, UserSettings};
use crate::state::AppState;

/// Something the camera can focus on or the inspector can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Star(u32),
    Body(BodyRef),
}

impl Target {
    pub fn system(self) -> u32 {
        match self {
            Target::Star(s) => s,
            Target::Body(b) => b.system,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub title: String,
    pub detail: String,
    pub target: Option<Target>,
    pub born: f64,
}

#[derive(Resource)]
pub struct Sim {
    pub universe: Universe,
    pub speed: usize,
    pub paused: bool,
    pub last: AdvanceReport,
    /// Simulated seconds per real second actually achieved (smoothed).
    pub effective_rate: f64,
    pub last_autosave: f64,
    pub seen_events: usize,
    pub toasts: Vec<Toast>,
    pub selected: Option<Target>,
    pub save_name: String,
    pub status: Option<(String, f64)>,
    pub sim_ms: f64,
}

impl Sim {
    pub fn new(universe: Universe, name: String) -> Self {
        let seen = universe.history.events.len();
        Self {
            universe,
            speed: 5,
            paused: false,
            last: AdvanceReport::default(),
            effective_rate: 0.0,
            last_autosave: 0.0,
            seen_events: seen,
            toasts: Vec::new(),
            selected: None,
            save_name: name,
            status: None,
            sim_ms: 0.0,
        }
    }

    pub fn rate(&self) -> f64 {
        SPEEDS[self.speed].rate
    }

    pub fn save(&mut self, file_stem: &str, now: f64) -> Result<std::path::PathBuf, String> {
        let path = saves_dir().join(format!("{file_stem}.{}", save::EXTENSION));
        save::write_file(&path, &self.universe, &self.save_name).map_err(|e| e.to_string())?;
        self.status = Some((format!("Saved to {}", path.display()), now));
        Ok(path)
    }
}

/// A universe being created or loaded off the main thread.
#[derive(Resource)]
pub struct PendingUniverse {
    pub task: Task<Result<(Universe, String), String>>,
    pub label: String,
}

/// Last error from creation/loading, shown in the menu.
#[derive(Resource, Default)]
pub struct MenuMessage(pub Option<String>);

pub fn begin_new(commands: &mut Commands, next: &mut NextState<AppState>, settings: UniverseSettings, advance_years: f64) {
    let label = format!("Forming {} — simulating billions of years of prehistory…", settings.scenario.label());
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let name = format!("{} #{}", settings.scenario.label(), settings.seed);
        let mut u = Universe::new(settings);
        if advance_years > 0.0 {
            u.advance_by(advance_years * SECONDS_PER_YEAR);
        }
        Ok((u, name))
    });
    commands.insert_resource(PendingUniverse { task, label });
    next.set(AppState::Generating);
}

pub fn begin_load(commands: &mut Commands, next: &mut NextState<AppState>, path: std::path::PathBuf) {
    let label = format!("Loading {}…", path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let header = save::read_header(&s).map_err(|e| e.to_string())?;
        let u = save::from_json(&s).map_err(|e| e.to_string())?;
        Ok((u, header.name))
    });
    commands.insert_resource(PendingUniverse { task, label });
    next.set(AppState::Generating);
}

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuMessage>()
            .add_systems(Startup, start_from_args)
            .add_systems(Update, poll_pending.run_if(in_state(AppState::Generating)))
            .add_systems(Update, (advance, collect_toasts, autosave).chain().in_set(crate::state::Frame::Simulate).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

fn start_from_args(mut commands: Commands, args: Res<Args>, mut next: ResMut<NextState<AppState>>) {
    if let Some(path) = &args.load {
        begin_load(&mut commands, &mut next, path.into());
    } else if let Some(scenario) = args.new {
        let (_, life, intel, _) = cosmogon_sim::universe::LIFE_PRESETS[args.life.unwrap_or(0).min(2)];
        let settings = UniverseSettings { seed: args.seed.unwrap_or(1), scenario, life_rate: life, intelligence_rate: intel, ..Default::default() };
        begin_new(&mut commands, &mut next, settings, args.advance_years);
    }
}

fn poll_pending(mut commands: Commands, pending: Option<ResMut<PendingUniverse>>, mut next: ResMut<NextState<AppState>>, mut msg: ResMut<MenuMessage>, args: Res<Args>) {
    let Some(mut pending) = pending else {
        next.set(AppState::MainMenu);
        return;
    };
    if let Some(result) = block_on(future::poll_once(&mut pending.task)) {
        commands.remove_resource::<PendingUniverse>();
        match result {
            Ok((u, name)) => {
                let mut sim = Sim::new(u, name);
                if let Some(s) = args.speed {
                    sim.speed = s.min(SPEEDS.len() - 1);
                }
                commands.insert_resource(sim);
                next.set(AppState::Observing);
            }
            Err(e) => {
                msg.0 = Some(format!("Could not open universe: {e}"));
                next.set(AppState::MainMenu);
            }
        }
    }
}

/// Index of the fastest speed at which individual years of history are still readable.
pub const MILESTONE_SPEED: usize = 7; // 100 yr/s

fn advance(mut sim: ResMut<Sim>, time: Res<Time>, settings: Res<UserSettings>) {
    let dt = time.delta_secs_f64().min(0.1);
    if sim.paused {
        sim.effective_rate = 0.0;
        return;
    }
    let before = sim.universe.time;
    let target = before + dt * sim.rate();
    let stop = (settings.auto_slow && sim.speed > MILESTONE_SPEED).then_some(5);
    let report = sim.universe.advance_to(target, Some(Duration::from_millis(9)), stop);
    if report.milestone.is_some() {
        sim.speed = MILESTONE_SPEED.min(sim.speed);
    }
    let achieved = (sim.universe.time - before) / dt.max(1e-6);
    sim.effective_rate = if sim.effective_rate == 0.0 { achieved } else { sim.effective_rate * 0.9 + achieved * 0.1 };
    sim.sim_ms = report.cpu_time.as_secs_f64() * 1000.0;
    sim.last = report;
}

fn collect_toasts(mut sim: ResMut<Sim>, time: Res<Time>) {
    let now = time.elapsed_secs_f64();
    let n = sim.universe.history.events.len();
    if sim.seen_events > n {
        sim.seen_events = n;
    }
    let new: Vec<Toast> = sim.universe.history.events[sim.seen_events..]
        .iter()
        .filter(|e| e.importance >= 5)
        .map(|e| Toast {
            title: e.title.clone(),
            detail: e.detail.clone(),
            target: match (e.system, e.body) {
                (Some(s), Some(b)) => Some(Target::Body(BodyRef { system: s, body: b })),
                (Some(s), None) => Some(Target::Star(s)),
                _ => None,
            },
            born: now,
        })
        .collect();
    sim.seen_events = n;
    sim.toasts.extend(new);
    sim.toasts.retain(|t| now - t.born < 9.0);
    let len = sim.toasts.len();
    if len > 4 {
        sim.toasts.drain(0..len - 4);
    }
}

fn autosave(mut sim: ResMut<Sim>, time: Res<Time>, settings: Res<UserSettings>) {
    let now = time.elapsed_secs_f64();
    if sim.last_autosave == 0.0 {
        sim.last_autosave = now;
    }
    if settings.autosave_minutes > 0.0 && now - sim.last_autosave > settings.autosave_minutes as f64 * 60.0 {
        sim.last_autosave = now;
        if let Err(e) = sim.save("autosave", now) {
            warn!("autosave failed: {e}");
        }
    }
}
