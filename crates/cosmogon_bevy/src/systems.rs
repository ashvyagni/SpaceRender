use bevy::prelude::*;

use crate::components::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
           .add_systems(Update, (
               update_simulation_time,
               propagate_orbits,
           ));
    }
}

fn spawn_camera(_commands: Commands) {
    // Camera will be spawned inside the big_space hierarchy
    // by the solar system plugin or separately
    info!("Camera plugin loaded");
}

fn update_simulation_time(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<SimulationTime>,
) {
    // Handle time controls
    if keyboard_input.just_pressed(KeyCode::Space) {
        time.paused = !time.paused;
    }
    
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        time.acceleration = 1.0;
    }
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        time.acceleration = 10.0;
    }
    if keyboard_input.just_pressed(KeyCode::Digit3) {
        time.acceleration = 100.0;
    }
    if keyboard_input.just_pressed(KeyCode::Digit4) {
        time.acceleration = 1000.0;
    }
    if keyboard_input.just_pressed(KeyCode::Digit5) {
        time.acceleration = 10000.0;
    }
}

fn propagate_orbits(
    time: Res<SimulationTime>,
    mut query: Query<(&mut Transform, &OrbitalElements, &CelestialBody)>,
) {
    if time.paused {
        return;
    }

    for (mut transform, orbit, body) in &mut query {
        // Skip stars (they don't orbit)
        if body.body_type == CelestialBodyType::Star {
            continue;
        }

        // Simple orbital propagation
        let mean_motion = 2.0 * std::f64::consts::PI / orbit.orbital_period;
        let mean_anomaly = orbit.mean_anomaly_epoch + mean_motion * time.elapsed;
        
        let angle = mean_anomaly;
        let orbit_distance = orbit.semi_major_axis as f32 * 100.0;
        
        let x = orbit_distance * angle.cos() as f32;
        let z = orbit_distance * angle.sin() as f32;
        
        transform.translation.x = x;
        transform.translation.z = z;
    }
}
