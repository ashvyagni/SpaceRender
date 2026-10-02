use bevy::prelude::*;
use big_space::prelude::*;

mod components;
mod systems;
mod solar_system;
mod loading_screen;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Cosmogon — Universe Simulator".into(),
                    resolution: bevy::window::WindowResolution::new(1920, 1080),
                    ..default()
                }),
                ..default()
            })
            .disable::<bevy::transform::TransformPlugin>()
        )
        .add_plugins(BigSpaceDefaultPlugins)
        .add_plugins(loading_screen::LoadingScreenPlugin)
        .add_plugins(solar_system::SolarSystemPlugin)
        .add_plugins(systems::CameraPlugin)
        .insert_resource(components::SimulationTime::default())
        .run();
}
