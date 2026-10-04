//! Screenshots.
//!
//! * Photo mode (camera button / P): hides the interface for a moment and saves a clean,
//!   full-resolution PNG to `~/Pictures/Cosmogon`.
//! * Developer capture: `--capture out.png [--capture-after N] [--quit]` saves a screenshot
//!   from the GPU (no OS screen-recording permission needed). Used for visual regression
//!   checks and documentation images.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use crate::args::Args;
use crate::state::AppState;

pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Photo>()
            .add_systems(Update, capture.run_if(in_state(AppState::Observing).or(in_state(AppState::MainMenu))))
            .add_systems(Update, take_photo.run_if(in_state(AppState::Observing)));
    }
}

/// A pending photo: the interface is hidden for a couple of frames, then the frame is saved.
#[derive(Resource, Default)]
pub struct Photo {
    pub requested: bool,
    frames: u32,
    restore_hidden: Option<bool>,
}

/// Where photos go: `~/Pictures/Cosmogon/cosmogon-<unix time>.png`.
pub fn photo_path() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from).unwrap_or_else(|| ".".into());
    let dir = home.join("Pictures").join("Cosmogon");
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    dir.join(format!("cosmogon-{secs}.png"))
}

fn take_photo(mut commands: Commands, mut photo: ResMut<Photo>, mut ui: ResMut<crate::ui::UiState>, mut sim: ResMut<crate::sim::Sim>, time: Res<Time>, mut gizmos: ResMut<GizmoConfigStore>) {
    if !photo.requested {
        if !ui.photo_requested {
            return;
        }
        ui.photo_requested = false;
        photo.requested = true;
    }
    if photo.restore_hidden.is_none() {
        photo.restore_hidden = Some(ui.hidden);
        ui.hidden = true;
        ui.photo_mode = true;
        // Orbits, trails and predictions are gizmos: switch them off for the shot.
        gizmos.config_mut::<DefaultGizmoConfigGroup>().0.enabled = false;
        photo.frames = 0;
        return;
    }
    photo.frames += 1;
    // Two frames for the interface to disappear from the swap chain.
    if photo.frames == 2 {
        let path = photo_path();
        let ok = path.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok());
        if ok {
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
            sim.status = Some((format!("Photo saved to {}", path.display()), time.elapsed_secs_f64()));
        } else {
            sim.status = Some(("Couldn't create the Pictures/Cosmogon folder".into(), time.elapsed_secs_f64()));
        }
    }
    if photo.frames >= 4 {
        ui.hidden = photo.restore_hidden.take().unwrap_or(false);
        ui.photo_mode = false;
        gizmos.config_mut::<DefaultGizmoConfigGroup>().0.enabled = true;
        photo.requested = false;
    }
}

fn capture(mut commands: Commands, args: Res<Args>, mut frames: Local<u32>, mut done: Local<bool>, mut exit: MessageWriter<AppExit>, state: Res<State<AppState>>) {
    let Some(path) = &args.capture else { return };
    // Wait for the requested universe rather than capturing the menu.
    if (args.new.is_some() || args.load.is_some()) && *state.get() != AppState::Observing {
        return;
    }
    *frames += 1;
    if !*done && *frames == args.capture_after {
        *done = true;
        info!("capturing screenshot to {path}");
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
    }
    if *done && args.quit_after_capture && *frames > args.capture_after + 20 && (std::path::Path::new(path).exists() || *frames > args.capture_after + 2000) {
        exit.write(AppExit::Success);
    }
}
