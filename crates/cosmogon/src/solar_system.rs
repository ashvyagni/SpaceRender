use bevy::prelude::*;
use big_space::prelude::*;

use crate::components::*;

pub struct SolarSystemPlugin;

impl Plugin for SolarSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_solar_system);
    }
}

fn spawn_solar_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning solar system...");

    let sun_mesh = meshes.add(Sphere::new(40.0));
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(10000.0, 8000.0, 3000.0, 1.0),
        ..default()
    });

    let planet_configs: Vec<(&str, CelestialBodyType, f32, f64, f64)> = vec![
        ("Mercury", CelestialBodyType::RockyPlanet, 2.5, 0.387, 87.969 * 24.0 * 3600.0),
        ("Venus", CelestialBodyType::RockyPlanet, 4.0, 0.723, 224.701 * 24.0 * 3600.0),
        ("Earth", CelestialBodyType::RockyPlanet, 4.5, 1.0, 365.256 * 24.0 * 3600.0),
        ("Mars", CelestialBodyType::RockyPlanet, 3.5, 1.524, 686.980 * 24.0 * 3600.0),
        ("Jupiter", CelestialBodyType::GasGiant, 12.0, 5.203, 4332.59 * 24.0 * 3600.0),
        ("Saturn", CelestialBodyType::GasGiant, 10.0, 9.537, 10759.22 * 24.0 * 3600.0),
        ("Uranus", CelestialBodyType::IceGiant, 8.0, 19.191, 30688.5 * 24.0 * 3600.0),
        ("Neptune", CelestialBodyType::IceGiant, 7.5, 30.069, 60182.0 * 24.0 * 3600.0),
    ];

    commands.spawn_big_space_default(|root_grid| {
        // Spawn Sun at the center
        root_grid.spawn_spatial((
            Mesh3d(sun_mesh),
            MeshMaterial3d(sun_material),
            CelestialBody {
                name: "Sun".to_string(),
                body_type: CelestialBodyType::Star,
                mass: 1.989e30,
                radius: 696340000.0,
                axial_tilt: 7.25,
                rotation_period: 25.05 * 24.0 * 3600.0,
            },
            CelestialRender {
                visual_radius: 40.0,
                color: Color::WHITE,
                emissive: Some(Color::srgb(1.0, 0.8, 0.3)),
                texture_path: None,
            },
        ));

        // Spawn planets
        for (name, body_type, visual_radius, semi_major_axis, orbital_period) in &planet_configs {
            let orbit_distance = *semi_major_axis as f32 * 100.0;

            let mesh = meshes.add(Sphere::new(*visual_radius));
            let color = match body_type {
                CelestialBodyType::RockyPlanet => Color::srgb(0.6, 0.5, 0.4),
                CelestialBodyType::GasGiant => Color::srgb(0.8, 0.7, 0.5),
                CelestialBodyType::IceGiant => Color::srgb(0.4, 0.6, 0.8),
                _ => Color::srgb(0.5, 0.5, 0.5),
            };
            let material = materials.add(StandardMaterial {
                base_color: color,
                ..default()
            });

            root_grid.spawn_spatial((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(orbit_distance, 0.0, 0.0),
                CelestialBody {
                    name: name.to_string(),
                    body_type: *body_type,
                    mass: 0.0,
                    radius: 0.0,
                    axial_tilt: 0.0,
                    rotation_period: 0.0,
                },
                OrbitalElements {
                    semi_major_axis: *semi_major_axis,
                    eccentricity: 0.0,
                    inclination: 0.0,
                    longitude_of_ascending: 0.0,
                    argument_of_periapsis: 0.0,
                    mean_anomaly_epoch: 0.0,
                    orbital_period: *orbital_period,
                    epoch: 2451545.0,
                },
                CelestialRender {
                    visual_radius: *visual_radius as f64,
                    color,
                    emissive: None,
                    texture_path: None,
                },
            ));
        }

        // Spawn camera with FloatingOrigin inside the big_space hierarchy
        root_grid.with_grid_default(|camera_grid| {
            camera_grid.insert((
                FloatingOrigin,
                BigSpaceCameraController::default()
                    .with_speed_bounds([0.1, 10e35])
                    .with_smoothness(0.98, 0.98)
                    .with_speed(1.0),
                Transform::from_xyz(0.0, 100.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));

            camera_grid.spawn_spatial((
                Camera3d::default(),
                Camera {
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
            ));
        });
    });

    info!("Solar system spawned with 9 celestial bodies and camera");
}
