//! Developer capture tool: `--capture out.png [--capture-after N] [--quit]` saves a
//! screenshot from the GPU (no OS screen-recording permission needed). Used for visual
//! regression checks and documentation images.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use crate::args::Args;
use crate::state::AppState;

pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture.run_if(in_state(AppState::Observing).or(in_state(AppState::MainMenu))));
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
