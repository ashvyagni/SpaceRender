//! Cosmogon desktop application.
//!
//! The app is a *view* onto `cosmogon_sim::Universe`: the simulation is authoritative and
//! engine-independent; everything in this crate reads from it and turns it into pixels.
//! See ARCHITECTURE.md and RENDERING.md.

// No console window for release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;
mod camera;
mod capture;
mod interact;
mod persistence;
mod render;
mod sim;
mod state;
mod timeline;
mod ui;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy_egui::EguiPlugin;

fn main() {
    let args = args::Args::parse();
    let settings = persistence::UserSettings::load();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cosmogon".into(),
                        resolution: WindowResolution::new(1600, 960),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin { filter: "wgpu=error,naga=warn,bevy_render=warn,cosmogon=info".into(), ..default() }),
        )
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin::default())
        .insert_resource(args)
        .insert_resource(settings)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            state::StatePlugin,
            sim::SimPlugin,
            render::RenderPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            capture::CapturePlugin,
            timeline::TimelinePlugin,
            interact::InteractPlugin,
        ))
        .run();
}
