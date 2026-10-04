//! Drives the authoritative simulation from the app: sessions (sandbox documents and
//! read-only reference views), creation/loading on background threads, per-frame
//! advancement under a CPU budget, edits with undo/redo, trajectory prediction,
//! autosave, thumbnails and milestone toasts.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use cosmogon_physics::nbody::Particle;
use cosmogon_sim::astro::dynamics::Prediction;
use cosmogon_sim::sandbox::{Edit, EditOutcome};
use cosmogon_sim::time::{SECONDS_PER_DAY, SECONDS_PER_YEAR, SPEEDS};
use cosmogon_sim::universe::AdvanceReport;
use cosmogon_sim::{BodyRef, Universe, UniverseSettings};

use crate::args::Args;
use crate::persistence::{self, Manifest, Origin, SaveSlot, UserSettings};
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

/// What the open universe is.
#[derive(Clone, Debug)]
pub enum Session {
    /// A user experiment, saved as a sandbox document.
    Sandbox(Manifest),
    /// A read-only view of a real dataset. Clone it to experiment.
    Reference { dataset: String, title: String },
}

impl Session {
    pub fn is_reference(&self) -> bool {
        matches!(self, Session::Reference { .. })
    }
    pub fn title(&self) -> String {
        match self {
            Session::Sandbox(m) => m.name.clone(),
            Session::Reference { title, .. } => title.clone(),
        }
    }
}

struct Snapshot {
    label: String,
    universe: Box<Universe>,
}

const UNDO_LIMIT: usize = 40;

/// A trajectory prediction running or finished on a background thread.
pub struct PredictionJob {
    pub task: Option<Task<Prediction>>,
    /// The body (or `None` = the launch projectile) the newest prediction is for.
    pub subject: Option<BodyRef>,
    pub result: Option<(Option<BodyRef>, Prediction)>,
    pub started_at: f64,
    /// System of the newest projectile prediction (throw tool).
    pub system: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaunchSpec {
    pub preset: &'static str,
    pub name: String,
    pub target: Option<BodyRef>,
    /// Start distance from the target (m).
    pub distance: f64,
    /// Approach speed relative to the target (m/s).
    pub speed: f64,
    /// Impact parameter (m).
    pub miss: f64,
    /// Direction the projectile comes from (azimuth, elevation in the orbital plane, rad).
    pub azimuth: f64,
    pub elevation: f64,
}

impl Default for LaunchSpec {
    fn default() -> Self {
        Self { preset: "asteroid_1km", name: "Asteroid".into(), target: None, distance: 2.0e9, speed: 20_000.0, miss: 0.0, azimuth: 0.6, elevation: 0.1 }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Tool {
    #[default]
    Select,
    Launch(LaunchSpec),
    /// Press in space to place an object, drag to give it velocity, release to throw.
    Throw(ThrowSpec),
    /// Drag an existing object to a new place (it keeps its velocity).
    Grab,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThrowSpec {
    /// Creator preset id (`sandbox::PRESETS`).
    pub preset: &'static str,
}

#[derive(Resource)]
pub struct Sim {
    pub universe: Universe,
    pub session: Session,
    pub speed: usize,
    pub paused: bool,
    pub last: AdvanceReport,
    /// Simulated seconds per real second actually achieved (smoothed).
    pub effective_rate: f64,
    pub last_autosave: f64,
    pub seen_events: usize,
    pub toasts: Vec<Toast>,
    pub selected: Option<Target>,
    pub status: Option<(String, f64)>,
    pub sim_ms: f64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Unsaved changes since the last explicit save.
    pub dirty: bool,
    pub prediction: PredictionJob,
    pub tool: Tool,
    /// Bumped whenever the universe is replaced wholesale (undo, checkpoint restore), so
    /// derived views (trails) can reset.
    pub generation: u64,
    /// A thumbnail to capture for the sandbox, after this many more frames.
    pub thumbnail_request: Option<(std::path::PathBuf, u32)>,
}

impl Sim {
    pub fn new(universe: Universe, session: Session) -> Self {
        let seen = universe.history.events.len();
        // Dynamic systems run at human speeds by default; geological scenarios faster.
        let speed = if universe.systems.iter().any(|s| s.is_dynamic()) { 3 } else { 5 };
        // Sandboxes without a preview get one once the scene has had time to render.
        let thumbnail_request = match &session {
            Session::Sandbox(m) if m.dir().join("manifest.json").exists() && !m.thumbnail_path().exists() => Some((m.thumbnail_path(), 150)),
            _ => None,
        };
        Self {
            universe,
            session,
            speed,
            paused: false,
            last: AdvanceReport::default(),
            effective_rate: 0.0,
            last_autosave: 0.0,
            seen_events: seen,
            toasts: Vec::new(),
            selected: None,
            status: None,
            sim_ms: 0.0,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            prediction: PredictionJob { task: None, subject: None, result: None, started_at: -10.0, system: 0 },
            tool: Tool::Select,
            generation: 0,
            thumbnail_request,
        }
    }

    pub fn rate(&self) -> f64 {
        SPEEDS[self.speed].rate
    }

    pub fn is_reference(&self) -> bool {
        self.session.is_reference()
    }

    pub fn say(&mut self, msg: impl Into<String>, now: f64) {
        self.status = Some((msg.into(), now));
    }

    /// Apply a sandbox edit with an undo point. Reference views are read-only.
    pub fn edit(&mut self, edit: Edit, now: f64) -> Result<EditOutcome, String> {
        if self.is_reference() {
            let msg = "This is real data and read-only. Use “Clone to sandbox” to experiment.".to_string();
            self.say(msg.clone(), now);
            return Err(msg);
        }
        let before = Box::new(self.universe.clone());
        match self.universe.apply_edit(edit) {
            Ok(out) => {
                self.undo.push(Snapshot { label: out.summary.clone(), universe: before });
                if self.undo.len() > UNDO_LIMIT {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.dirty = true;
                self.seen_events = self.seen_events.min(self.universe.history.events.len());
                self.prediction.result = None;
                self.prediction.started_at = -10.0;
                self.say(out.summary.clone(), now);
                Ok(out)
            }
            Err(e) => {
                self.say(format!("Not applied: {e}"), now);
                Err(e)
            }
        }
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// Return to the moment before the last edit (including simulation time).
    pub fn undo(&mut self, now: f64) {
        if let Some(s) = self.undo.pop() {
            let current = std::mem::replace(&mut self.universe, *s.universe);
            self.redo.push(Snapshot { label: s.label.clone(), universe: Box::new(current) });
            self.after_replace();
            self.say(format!("Undone: {}", s.label), now);
        }
    }

    pub fn redo(&mut self, now: f64) {
        if let Some(s) = self.redo.pop() {
            let current = std::mem::replace(&mut self.universe, *s.universe);
            self.undo.push(Snapshot { label: s.label.clone(), universe: Box::new(current) });
            self.after_replace();
            self.say(format!("Redone: {}", s.label), now);
        }
    }

    /// Replace the universe (checkpoint restore) as an undoable step.
    pub fn replace_universe(&mut self, u: Universe, label: &str, now: f64) {
        let before = std::mem::replace(&mut self.universe, u);
        self.undo.push(Snapshot { label: label.into(), universe: Box::new(before) });
        self.redo.clear();
        self.after_replace();
        self.say(label.to_string(), now);
    }

    fn after_replace(&mut self) {
        self.dirty = true;
        self.generation += 1;
        self.seen_events = self.universe.history.events.len();
        self.prediction = PredictionJob { task: None, subject: None, result: None, started_at: -10.0, system: 0 };
        if let Some(Target::Body(r)) = self.selected {
            if !self.universe.systems.get(r.system as usize).is_some_and(|s| s.bodies.get(r.body as usize).is_some_and(|b| b.exists())) {
                self.selected = Some(Target::Star(r.system));
            }
        }
    }

    /// Save explicitly (sandboxes only). Returns where.
    pub fn save(&mut self, now: f64) -> Result<std::path::PathBuf, String> {
        let Session::Sandbox(m) = &mut self.session else {
            return Err("reference views are not saved; clone to a sandbox first".into());
        };
        let path = persistence::save_sandbox(m, &self.universe, SaveSlot::State)?;
        self.thumbnail_request = Some((m.thumbnail_path(), 2));
        self.dirty = false;
        let name = m.name.clone();
        self.say(format!("Saved “{name}”"), now);
        Ok(path)
    }

    pub fn autosave(&mut self) -> Result<(), String> {
        if let Session::Sandbox(m) = &mut self.session {
            // Developer starts and unsaved imports join the library only when saved.
            if !m.dir().join("manifest.json").exists() {
                return Ok(());
            }
            persistence::save_sandbox(m, &self.universe, SaveSlot::Autosave)?;
        }
        Ok(())
    }

    /// Turn the reference view into a new sandbox (the dataset is never modified).
    pub fn clone_to_sandbox(&mut self, name: &str, now: f64) -> Result<(), String> {
        let Session::Reference { dataset, title } = &self.session else { return Err("already a sandbox".into()) };
        let desc = format!("Cloned from {title} ({dataset}).");
        let mut m = Manifest::new(name, &desc, "solar_system_lab", Origin { cloned_from: Some(format!("dataset:{dataset}")), ..Default::default() });
        persistence::save_sandbox(&mut m, &self.universe, SaveSlot::State)?;
        self.thumbnail_request = Some((m.thumbnail_path(), 2));
        self.session = Session::Sandbox(m);
        self.dirty = false;
        self.say(format!("Created sandbox “{name}” — you can now edit everything."), now);
        Ok(())
    }

    /// Kick off a background trajectory prediction for `subject` (or the launch projectile).
    pub fn request_prediction(&mut self, subject: Option<BodyRef>, extra: Option<Particle>, horizon: f64, now: f64) {
        let Some(system) = subject.map(|r| r.system).or_else(|| self.launch_target().map(|t| t.system)) else { return };
        let sys = self.universe.system(system).clone();
        let t = self.universe.time;
        self.prediction.subject = subject;
        self.prediction.started_at = now;
        self.prediction.task = Some(AsyncComputeTaskPool::get().spawn(async move { sys.predict(t, extra, horizon, 600, 30_000) }));
    }

    /// Predict the path of a not-yet-created object in `system` (throw tool preview).
    pub fn request_projectile_prediction(&mut self, system: u32, extra: Particle, horizon: f64, now: f64) {
        let sys = self.universe.system(system).clone();
        let t = self.universe.time;
        self.prediction.subject = None;
        self.prediction.system = system;
        self.prediction.started_at = now;
        self.prediction.task = Some(AsyncComputeTaskPool::get().spawn(async move { sys.predict(t, Some(extra), horizon, 400, 12_000) }));
    }

    pub fn launch_target(&self) -> Option<BodyRef> {
        match &self.tool {
            Tool::Launch(l) => l.target,
            _ => None,
        }
    }
}

/// A universe being created or loaded off the main thread.
#[derive(Resource)]
pub struct PendingUniverse {
    pub task: Task<Result<(Universe, Session), String>>,
    pub label: String,
}

/// Last error from creation/loading, shown in the menu.
#[derive(Resource, Default)]
pub struct MenuMessage(pub Option<String>);

/// Everything needed to create a sandbox.
#[derive(Clone, Debug)]
pub struct NewSandbox {
    pub settings: UniverseSettings,
    pub advance_years: f64,
    pub name: String,
    pub description: String,
    pub template: String,
    /// A curated experiment applied after creation (`cosmogon_sim::sandbox::WHAT_IFS`).
    pub what_if: Option<String>,
    /// Write it to the sandbox library immediately (false for developer starts).
    pub persist: bool,
}

/// Create a new sandbox. It is written to disk as soon as it exists.
pub fn begin_new(commands: &mut Commands, next: &mut NextState<AppState>, n: NewSandbox) {
    let label = format!("Creating {} — simulating the history before your experiment begins…", n.settings.scenario.label());
    let NewSandbox { settings, advance_years, name, description, template, what_if, persist } = n;
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let mut u = Universe::new(settings);
        // Experiments start "now"; any fast-forward then shows what follows from them.
        if let Some(w) = &what_if {
            cosmogon_sim::sandbox::apply_what_if(&mut u, w)?;
        }
        if advance_years > 0.0 {
            u.advance_by(advance_years * SECONDS_PER_YEAR);
        }
        let mut m = Manifest::new(&name, &description, &template, Origin::default());
        if u.systems.iter().any(|s| s.bodies.iter().any(|b| b.provenance.source.contains("Horizons"))) {
            m.origin.cloned_from = Some(format!("dataset:{}", cosmogon_sim::astro::horizons::Dataset::embedded().id()));
        }
        if persist {
            persistence::save_sandbox(&mut m, &u, SaveSlot::State)?;
        }
        Ok((u, Session::Sandbox(m)))
    });
    commands.insert_resource(PendingUniverse { task, label });
    next.set(AppState::Generating);
}

/// Open the real Solar System (JPL Horizons) as a read-only reference.
pub fn begin_reference(commands: &mut Commands, next: &mut NextState<AppState>) {
    let label = "Loading the real Solar System from NASA/JPL Horizons data…".to_string();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let u = Universe::new(UniverseSettings { seed: 1, scenario: cosmogon_sim::Scenario::SolarSystemLab, ..Default::default() });
        let d = cosmogon_sim::astro::horizons::Dataset::embedded();
        Ok((u, Session::Reference { dataset: d.id(), title: format!("Solar System — real data ({})", d.dataset.epoch) }))
    });
    commands.insert_resource(PendingUniverse { task, label });
    next.set(AppState::Generating);
}

pub fn begin_open_sandbox(commands: &mut Commands, next: &mut NextState<AppState>, m: Manifest, file: Option<std::path::PathBuf>) {
    let label = format!("Opening “{}”…", m.name);
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let path = file.or_else(|| persistence::newest_state_file(&m)).ok_or("this sandbox has no saved state")?;
        let u = persistence::load_universe_file(&path)?;
        Ok((u, Session::Sandbox(m)))
    });
    commands.insert_resource(PendingUniverse { task, label });
    next.set(AppState::Generating);
}

/// Open an old single-file save (0.2–0.3); it becomes a sandbox when first saved.
pub fn begin_load_legacy(commands: &mut Commands, next: &mut NextState<AppState>, path: std::path::PathBuf) {
    let label = format!("Loading {}…", path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let header = cosmogon_sim::save::read_header(&s).map_err(|e| e.to_string())?;
        let u = cosmogon_sim::save::from_json(&s).map_err(|e| e.to_string())?;
        let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let m = Manifest::new(&header.name, "Imported from an earlier version of Cosmogon.", "imported", Origin { cloned_from: Some(format!("legacy-save:{file}")), ..Default::default() });
        Ok((u, Session::Sandbox(m)))
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
            .add_systems(Last, autosave_on_exit.run_if(resource_exists::<Sim>))
            .add_systems(Update, (advance, collect_toasts, autosave, poll_prediction, capture_thumbnail).chain().in_set(crate::state::Frame::Simulate).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

fn start_from_args(mut commands: Commands, args: Res<Args>, mut next: ResMut<NextState<AppState>>) {
    if args.continue_latest {
        if let Some(l) = persistence::list_sandboxes().into_iter().next() {
            begin_open_sandbox(&mut commands, &mut next, l.manifest, None);
        }
    } else if let Some(path) = &args.load {
        begin_load_legacy(&mut commands, &mut next, path.into());
    } else if let Some(scenario) = args.new {
        let (_, life, intel, _) = cosmogon_sim::universe::LIFE_PRESETS[args.life.unwrap_or(0).min(2)];
        use cosmogon_sim::astro::dynamics::PhysicsPreset;
        let physics = args.nbody.as_deref().map(|p| match p {
            "fast" => PhysicsPreset::Fast,
            "accurate" => PhysicsPreset::Accurate,
            "research" => PhysicsPreset::Research,
            _ => PhysicsPreset::Balanced,
        }.settings());
        let settings = UniverseSettings { seed: args.seed.unwrap_or(1), scenario, life_rate: life, intelligence_rate: intel, physics, ..Default::default() };
        let name = format!("{} (dev)", scenario.label());
        begin_new(&mut commands, &mut next, NewSandbox { settings, advance_years: args.advance_years, name, description: String::new(), template: "dev".into(), what_if: args.what_if.clone(), persist: args.persist });
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
            Ok((u, session)) => {
                let mut sim = Sim::new(u, session);
                if let Some(s) = args.speed {
                    sim.speed = s.min(SPEEDS.len() - 1);
                }
                if let Some(i) = args.tour.as_deref().and_then(crate::ui::tour::tour_by_id) {
                    commands.insert_resource(crate::ui::tour::ActiveTour { tour: i, step: 0, applied: None });
                }
                if args.panel.as_deref() == Some("launch") {
                    sim.tool = Tool::Launch(LaunchSpec::default());
                }
                commands.insert_resource(sim);
                next.set(AppState::Observing);
            }
            Err(e) => {
                msg.0 = Some(format!("Could not open: {e}"));
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
    if sim.universe.time > before && !sim.is_reference() {
        sim.dirty = true;
    }
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
        if let Err(e) = sim.autosave() {
            warn!("autosave failed: {e}");
        }
    }
}

/// Closing the window or quitting autosaves the open sandbox, so CONTINUE resumes it.
fn autosave_on_exit(mut exits: MessageReader<AppExit>, mut sim: ResMut<Sim>) {
    if exits.read().next().is_some() {
        match sim.autosave() {
            Ok(()) => info!("autosaved on exit"),
            Err(e) => warn!("autosave on exit failed: {e}"),
        }
    }
}

/// Horizon of a body's prediction: about one orbit, within sensible bounds.
pub fn prediction_horizon(u: &Universe, r: BodyRef) -> f64 {
    let sys = u.system(r.system);
    let el = sys.osculating(r.body as usize, u.time);
    if el.is_bound() {
        let mu = sys.parent_mu(r.body as usize) + sys.bodies[r.body as usize].mu();
        let p = std::f64::consts::TAU * (el.semi_major_axis.powi(3) / mu.max(1.0)).sqrt();
        p.clamp(2.0 * SECONDS_PER_DAY, 40.0 * SECONDS_PER_YEAR)
    } else {
        2.0 * SECONDS_PER_YEAR
    }
}

/// Keep the selected body's (or the launch tool's) predicted trajectory fresh.
fn poll_prediction(mut sim: ResMut<Sim>, time: Res<Time>) {
    let now = time.elapsed_secs_f64();
    if let Some(task) = sim.prediction.task.as_mut() {
        if let Some(p) = block_on(future::poll_once(task)) {
            let subject = sim.prediction.subject;
            sim.prediction.result = Some((subject, p));
            sim.prediction.task = None;
        }
        return;
    }
    if matches!(sim.tool, Tool::Launch(_) | Tool::Throw(_)) {
        return; // these tools request their own predictions
    }
    let Some(Target::Body(r)) = sim.selected else {
        sim.prediction.result = None;
        return;
    };
    if !sim.universe.system(r.system).is_dynamic() || !sim.universe.body(r).exists() {
        sim.prediction.result = None;
        return;
    }
    let stale = sim.prediction.result.as_ref().is_none_or(|(s, _)| *s != Some(r));
    let interval = if sim.paused { 5.0 } else { 0.75 };
    if stale || now - sim.prediction.started_at > interval {
        let horizon = prediction_horizon(&sim.universe, r);
        sim.request_prediction(Some(r), None, horizon, now);
    }
}

/// Capture a sandbox thumbnail (downscaled screenshot) after a save.
fn capture_thumbnail(mut commands: Commands, mut sim: ResMut<Sim>) {
    let Some((path, wait)) = sim.thumbnail_request.clone() else { return };
    if wait > 0 {
        sim.thumbnail_request = Some((path, wait - 1));
        return;
    }
    sim.thumbnail_request = None;
    commands.spawn(Screenshot::primary_window()).observe(move |trigger: On<ScreenshotCaptured>| {
        let img = &trigger.image;
        let (w, h) = (img.width(), img.height());
        let Some(data) = img.data.as_ref() else { return };
        // Screenshots come back as BGRA or RGBA depending on the platform surface.
        let bgra = matches!(img.texture_descriptor.format, bevy::render::render_resource::TextureFormat::Bgra8Unorm | bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb);
        let mut rgba = data.clone();
        if bgra {
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        if let Err(e) = persistence::write_thumbnail(&path, &rgba, w, h) {
            warn!("thumbnail failed: {e}");
        }
    });
}
