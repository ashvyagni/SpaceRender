use bevy_ecs::prelude::*;

/// Classical Keplerian orbital elements.
#[derive(Component, Debug, Clone, Copy)]
pub struct OrbitalElements {
    /// Semi-major axis in meters.
    pub semi_major_axis: f64,
    /// Eccentricity: 0 = circle, 0–1 = ellipse, >1 = hyperbola.
    pub eccentricity: f64,
    /// Inclination in radians.
    pub inclination: f64,
    /// Longitude of the ascending node (Ω) in radians.
    pub longitude_ascending: f64,
    /// Argument of perihelion (ω) in radians.
    pub argument_perihelion: f64,
    /// Mean anomaly at epoch (M₀) in radians.
    pub mean_anomaly_epoch: f64,
}

impl OrbitalElements {
    /// Mean anomaly at a given time.
    ///
    /// # Arguments
    /// * `t` – elapsed time since epoch in seconds.
    /// * `period` – orbital period in seconds.
    pub fn mean_anomaly_at_time(&self, t: f64, period: f64) -> f64 {
        let mean_motion = std::f64::consts::TAU / period;
        (self.mean_anomaly_epoch + mean_motion * t) % std::f64::consts::TAU
    }

    /// Orbital period in seconds given the standard gravitational parameter μ (m³/s²).
    pub fn period(&self, gravitational_param: f64) -> f64 {
        std::f64::consts::TAU * (self.semi_major_axis.powi(3) / gravitational_param).sqrt()
    }
}

impl Default for OrbitalElements {
    fn default() -> Self {
        Self {
            semi_major_axis: 1.0,
            eccentricity: 0.0,
            inclination: 0.0,
            longitude_ascending: 0.0,
            argument_perihelion: 0.0,
            mean_anomaly_epoch: 0.0,
        }
    }
}
