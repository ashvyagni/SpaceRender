use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component for celestial bodies (planets, moons, stars)
#[derive(Component, Debug, Clone, Reflect)]
pub struct CelestialBody {
    pub name: String,
    pub body_type: CelestialBodyType,
    pub mass: f64,           // kg
    pub radius: f64,         // meters
    pub axial_tilt: f64,     // degrees
    pub rotation_period: f64, // seconds
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum CelestialBodyType {
    Star,
    GasGiant,
    IceGiant,
    RockyPlanet,
    DwarfPlanet,
    Moon,
    Asteroid,
    Comet,
}

/// Component for orbital mechanics
#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitalElements {
    pub semi_major_axis: f64,    // AU
    pub eccentricity: f64,       // 0-1
    pub inclination: f64,        // degrees
    pub longitude_of_ascending: f64, // degrees
    pub argument_of_periapsis: f64,   // degrees
    pub mean_anomaly_epoch: f64,      // degrees
    pub orbital_period: f64,     // seconds
    pub epoch: f64,              // Julian date
}

/// Component for physical properties
#[derive(Component, Debug, Clone, Reflect)]
pub struct PhysicalProperties {
    pub surface_temperature: f64,  // Kelvin
    pub atmosphere: Option<Atmosphere>,
    pub composition: Vec<String>,
}

#[derive(Debug, Clone, Reflect)]
pub struct Atmosphere {
    pub pressure: f64,      // Pascals
    pub density: f64,       // kg/m³
    pub composition: Vec<(String, f64)>, // (gas, percentage)
    pub scale_height: f64,  // meters
}

/// Component for rendering
#[derive(Component, Debug, Clone, Reflect)]
pub struct CelestialRender {
    pub visual_radius: f64,    // render units
    pub color: Color,
    pub emissive: Option<Color>,
    pub texture_path: Option<String>,
}

/// Resource for simulation time
#[derive(Resource, Debug, Clone, Reflect)]
pub struct SimulationTime {
    pub elapsed: f64,      // seconds since epoch
    pub acceleration: f64, // time acceleration factor
    pub paused: bool,
    pub epoch: f64,        // Julian date
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            acceleration: 1.0,
            paused: false,
            epoch: 2451545.0, // J2000.0 epoch
        }
    }
}

/// Component for camera state
#[derive(Component, Debug, Clone, Reflect)]
pub struct CameraState {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 200.0,
            yaw: 0.3,
            pitch: 0.4,
            sensitivity: 0.003,
        }
    }
}
