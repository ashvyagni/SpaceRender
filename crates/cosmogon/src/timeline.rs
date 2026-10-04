//! Timeline and rewind.
//!
//! While the clock runs, a compressed snapshot of the universe is taken every few seconds
//! of real time (on a background thread). The strip above the clock shows the whole
//! history of this session with its milestones; click a snapshot to rewind there. A rewind
//! is an ordinary undoable step, so the abandoned future is one Ctrl/⌘-Z away.

use std::io::{Read, Write};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::history::Category;
use cosmogon_sim::time::format_date;
use cosmogon_sim::Universe;

use crate::sim::{Sim, Target};
use crate::state::AppState;
use crate::ui::{UiState, ACCENT, CIV, DANGER, LIFE, MUTED, PANEL, TEXT};

/// Snapshots kept per session (thinned evenly when full).
pub const MAX_SNAPSHOTS: usize = 40;
/// Real seconds between snapshots while the clock runs.
pub const INTERVAL_S: f64 = 4.0;

pub struct TimelineSnap {
    pub time: f64,
    pub data: Arc<Vec<u8>>,
}

#[derive(Resource, Default)]
pub struct Timeline {
    pub snaps: Vec<TimelineSnap>,
    pending: Option<Task<Option<TimelineSnap>>>,
    last_real: f64,
    generation: u64,
    /// The session the snapshots belong to (cleared when another universe opens).
    session_start: f64,
}

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Timeline>()
            .add_systems(OnEnter(AppState::Observing), |mut tl: ResMut<Timeline>| *tl = Timeline::default())
            .add_systems(Update, record.run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

pub fn compress(u: &Universe) -> Option<Vec<u8>> {
    let json = cosmogon_sim::save::to_json(u, "timeline").ok()?;
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(json.as_bytes()).ok()?;
    enc.finish().ok()
}

pub fn decompress(data: &[u8]) -> Option<Universe> {
    let mut s = String::new();
    flate2::read::DeflateDecoder::new(data).read_to_string(&mut s).ok()?;
    cosmogon_sim::save::from_json(&s).ok()
}

/// Drop the snapshot whose neighbours are closest together (keeps the spread even); never
/// the first or the newest.
fn thin(snaps: &mut Vec<TimelineSnap>) {
    while snaps.len() > MAX_SNAPSHOTS {
        let i = (1..snaps.len() - 1).min_by(|&a, &b| (snaps[a + 1].time - snaps[a - 1].time).total_cmp(&(snaps[b + 1].time - snaps[b - 1].time))).unwrap_or(1);
        snaps.remove(i);
    }
}

fn record(mut tl: ResMut<Timeline>, sim: Res<Sim>, time: Res<Time>) {
    let now = time.elapsed_secs_f64();
    if tl.session_start == 0.0 {
        tl.session_start = sim.universe.start_time.max(f64::MIN_POSITIVE);
        tl.generation = sim.generation;
    }
    // A rewind, undo or edit replaced the universe: snapshots after "now" belong to a future
    // that no longer exists.
    if tl.generation != sim.generation {
        tl.generation = sim.generation;
        let t = sim.universe.time;
        tl.snaps.retain(|s| s.time <= t + 1.0);
    }
    if let Some(task) = tl.pending.as_mut() {
        if let Some(result) = block_on(future::poll_once(task)) {
            tl.pending = None;
            if let Some(snap) = result {
                if tl.snaps.last().is_none_or(|l| snap.time > l.time) {
                    tl.snaps.push(snap);
                    thin(&mut tl.snaps);
                }
            }
        }
        return;
    }
    let due = tl.snaps.is_empty() || (!sim.paused && now - tl.last_real >= INTERVAL_S && tl.snaps.last().is_some_and(|l| sim.universe.time > l.time));
    if !due {
        return;
    }
    tl.last_real = now;
    let u = sim.universe.clone();
    tl.pending = Some(AsyncComputeTaskPool::get().spawn(async move { compress(&u).map(|d| TimelineSnap { time: u.time, data: Arc::new(d) }) }));
}

/// The strip above the clock: milestones and snapshots across the session's history.
pub fn timeline_strip(mut contexts: EguiContexts, ui_state: Res<UiState>, mut sim: ResMut<Sim>, tl: Res<Timeline>, time: Res<Time>) -> Result {
    if ui_state.hidden {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let now = time.elapsed_secs_f64();
    let mut rewind: Option<usize> = None;
    let mut go: Option<Target> = None;
    let frame = egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(14, 4));
    egui::TopBottomPanel::bottom("history_strip").exact_height(24.0).frame(frame).show(ctx, |ui| {
        let u = &sim.universe;
        let (t0, t1) = (tl.snaps.first().map(|s| s.time).unwrap_or(u.start_time).min(u.start_time), u.time.max(u.start_time + 1.0));
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let p = ui.painter_at(rect);
        let x_of = |t: f64| rect.left() + ((t - t0) / (t1 - t0)).clamp(0.0, 1.0) as f32 * rect.width();
        let y = rect.center().y;
        p.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(30, 38, 56)));
        p.line_segment([egui::pos2(rect.left(), y), egui::pos2(x_of(u.time), y)], egui::Stroke::new(2.0_f32, ACCENT.gamma_multiply(0.45)));
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let clicked = ui.input(|i| i.pointer.primary_clicked());
        let mut tip: Option<String> = None;
        // Milestones.
        for e in u.history.events.iter().filter(|e| e.importance >= 5 && e.time >= t0) {
            let x = x_of(e.time);
            let col = match e.category {
                Category::Life => LIFE,
                Category::Disaster | Category::War => DANGER,
                Category::Astronomy => TEXT,
                _ => CIV,
            };
            p.line_segment([egui::pos2(x, y - 6.0), egui::pos2(x, y + 6.0)], egui::Stroke::new(1.5_f32, col.gamma_multiply(0.85)));
            if pointer.is_some_and(|pp| (pp.x - x).abs() < 3.0 && rect.contains(pp)) {
                tip = Some(format!("{} — {}", format_date(e.time, u.start_time, u.gregorian()), e.title));
                if clicked {
                    if let (Some(s), Some(b)) = (e.system, e.body) {
                        go = Some(Target::Body(cosmogon_sim::BodyRef { system: s, body: b }));
                    }
                }
            }
        }
        // Snapshots you can return to.
        for (i, s) in tl.snaps.iter().enumerate() {
            let c = egui::pos2(x_of(s.time), y);
            let hovered = pointer.is_some_and(|pp| pp.distance(c) < 7.0);
            let r = if hovered { 6.0 } else { 4.0 };
            let pts = vec![c + egui::vec2(0.0, -r), c + egui::vec2(r, 0.0), c + egui::vec2(0.0, r), c + egui::vec2(-r, 0.0)];
            p.add(egui::Shape::convex_polygon(pts, if hovered { ACCENT } else { egui::Color32::from_rgb(120, 130, 155) }, egui::Stroke::NONE));
            if hovered {
                tip = Some(format!("Rewind to {} (undoable)", format_date(s.time, u.start_time, u.gregorian())));
                if clicked {
                    rewind = Some(i);
                }
            }
        }
        // Now.
        let xn = x_of(u.time);
        p.circle_filled(egui::pos2(xn, y), 4.5, ACCENT);
        if let Some(t) = tip {
            if let Some(pp) = pointer {
                egui::Area::new(egui::Id::new("timeline_tip")).fixed_pos(pp + egui::vec2(10.0, -30.0)).order(egui::Order::Tooltip).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new(t).size(11.5));
                    });
                });
            }
        } else if pointer.is_some_and(|pp| rect.contains(pp)) && tl.snaps.is_empty() {
            p.text(rect.right_center() - egui::vec2(4.0, 0.0), egui::Align2::RIGHT_CENTER, "Snapshots appear here while time runs", egui::FontId::proportional(10.5), MUTED);
        }
    });
    if let Some(i) = rewind {
        let label = format!("Rewound to {}", format_date(tl.snaps[i].time, sim.universe.start_time, sim.universe.gregorian()));
        match decompress(&tl.snaps[i].data) {
            Some(u) => {
                sim.replace_universe(u, &label, now);
                sim.paused = true;
            }
            None => sim.status = Some(("That snapshot could not be restored".into(), now)),
        }
    }
    if let Some(t) = go {
        sim.selected = Some(t);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_round_trip_and_stay_small() {
        let u = Universe::new(cosmogon_sim::UniverseSettings { seed: 3, scenario: cosmogon_sim::Scenario::SolarSystemLab, ..Default::default() });
        let data = compress(&u).unwrap();
        println!("lab snapshot: {} KiB", data.len() / 1024);
        assert!(data.len() < 8 * 1024 * 1024);
        let back = decompress(&data).unwrap();
        assert_eq!(back.time, u.time);
        assert_eq!(back.civs.len(), u.civs.len());
    }

    #[test]
    fn thinning_keeps_ends_and_an_even_spread() {
        let mut v: Vec<TimelineSnap> = (0..100).map(|i| TimelineSnap { time: (i * i) as f64, data: Arc::new(Vec::new()) }).collect();
        thin(&mut v);
        assert_eq!(v.len(), MAX_SNAPSHOTS);
        assert_eq!(v.first().unwrap().time, 0.0);
        assert_eq!(v.last().unwrap().time, 99.0 * 99.0);
    }
}
