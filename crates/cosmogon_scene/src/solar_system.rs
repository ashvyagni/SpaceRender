use crate::celestial_body::{AtmosphereDef, BodyType, CelestialBodyDef, StarData};
use cosmogon_ecs::components::orbital::OrbitalElements;

/// Return definitions for the Sun, eight planets, the Moon, and major
/// outer-planet moons.
///
/// All values are real astronomical data sourced from JPL Solar System
/// Dynamics ephemeris (DE440/441) and the IAU reference frames.
/// Orbital elements are given for the J2000.0 epoch (2000-01-01 12:00 TT).
pub fn solar_system_bodies() -> Vec<CelestialBodyDef> {
    vec![
        sun(),
        mercury(),
        venus(),
        earth(),
        mars(),
        jupiter(),
        saturn(),
        uranus(),
        neptune(),
        earth_moon(),
        io(),
        europa(),
        ganymede(),
        callisto(),
        titan(),
        enceladus(),
    ]
}

/// The Sun — G2V main-sequence star at the center of the Solar System.
pub fn sun() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Sun".into(),
        body_type: BodyType::Star,
        mass: 1.98892e30,
        radius: 6.957e8,
        color: [1.0, 0.95, 0.8, 1.0],
        orbital: None,
        rotation_period: Some(25.05 * 86400.0), // 25.05 days at equator
        axial_tilt: Some(7.25_f64.to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: Some(StarData {
            temperature: 5778.0,
            luminosity: 1.0,
            spectral_class: 'G',
        }),
    }
}

// ──────────────────────────────────────────────────────────────
//  Terrestrial planets
// ──────────────────────────────────────────────────────────────

/// Mercury — innermost planet.
pub fn mercury() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Mercury".into(),
        body_type: BodyType::Planet,
        mass: 3.30104e23,
        radius: 2.4397e6,
        color: [0.75, 0.72, 0.68, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 5.7909175e10,        // 0.38709927 AU
            eccentricity: 0.20563593,
            inclination: (7.005_f64).to_radians(),
            longitude_ascending: (48.33076593_f64).to_radians(),
            argument_perihelion: (29.12703035_f64).to_radians(),
            mean_anomaly_epoch: (174.796_f64).to_radians(),
        }),
        rotation_period: Some(58.646 * 86400.0), // 58.646 days (3:2 spin-orbit)
        axial_tilt: Some((0.034_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: None,
    }
}

/// Venus — second planet, thick CO₂ atmosphere.
pub fn venus() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Venus".into(),
        body_type: BodyType::Planet,
        mass: 4.86732e24,
        radius: 6.0518e6,
        color: [0.90, 0.78, 0.50, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.0820947e11,        // 0.72333566 AU
            eccentricity: 0.00677672,
            inclination: (3.39467605_f64).to_radians(),
            longitude_ascending: (76.67984255_f64).to_radians(),
            argument_perihelion: (54.85229144_f64).to_radians(),
            mean_anomaly_epoch: (50.416_f64).to_radians(),
        }),
        rotation_period: Some(-5832.5 * 86400.0), // retrograde, 243.025 days
        axial_tilt: Some(177.36_f64.to_radians()), // near 180° → retrograde spin
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [12.0e-6, 8.0e-6, 4.0e-6],
            rayleigh_scale_height: 15900.0,
            mie_scattering: [3.0e-6, 3.0e-6, 3.0e-6],
            mie_scale_height: 3500.0,
            atmosphere_height: 250_000.0,
        }),
        star_data: None,
    }
}

/// Earth — third planet, our home.
pub fn earth() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Earth".into(),
        body_type: BodyType::Planet,
        mass: 5.97217e24,
        radius: 6.3710e6,
        color: [0.16, 0.36, 0.74, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.49598023e11,       // 1.00000261 AU
            eccentricity: 0.01671123,
            inclination: (-0.00005_f64).to_radians(), // ~0 by definition of ecliptic
            longitude_ascending: (-11.26064_f64).to_radians(),
            argument_perihelion: (114.208_f64).to_radians(),
            mean_anomaly_epoch: (357.529_f64).to_radians(),
        }),
        rotation_period: Some(86164.1),  // 23 h 56 min 4.1 s (sidereal)
        axial_tilt: Some(23.4393_f64.to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [5.8e-6, 13.5e-6, 33.1e-6],
            rayleigh_scale_height: 8500.0,
            mie_scattering: [21.0e-6, 21.0e-6, 21.0e-6],
            mie_scale_height: 1200.0,
            atmosphere_height: 100_000.0,
        }),
        star_data: None,
    }
}

/// Mars — fourth planet, the Red Planet.
pub fn mars() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Mars".into(),
        body_type: BodyType::Planet,
        mass: 6.41693e23,
        radius: 3.3895e6,
        color: [0.91, 0.47, 0.22, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 2.2793920e11,        // 1.52368055 AU
            eccentricity: 0.09339410,
            inclination: (1.84969142_f64).to_radians(),
            longitude_ascending: (49.55953891_f64).to_radians(),
            argument_perihelion: (286.50254090_f64).to_radians(),
            mean_anomaly_epoch: (19.373_f64).to_radians(),
        }),
        rotation_period: Some(88642.663), // 24 h 37 min 22.66 s (sidereal)
        axial_tilt: Some(25.1886_f64.to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [7.0e-6, 5.0e-6, 3.0e-6],
            rayleigh_scale_height: 11100.0,
            mie_scattering: [6.0e-6, 6.0e-6, 6.0e-6],
            mie_scale_height: 2100.0,
            atmosphere_height: 80_000.0,
        }),
        star_data: None,
    }
}

// ──────────────────────────────────────────────────────────────
//  Gas / ice giants
// ──────────────────────────────────────────────────────────────

/// Jupiter — largest planet, gas giant.
pub fn jupiter() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Jupiter".into(),
        body_type: BodyType::Planet,
        mass: 1.89813e27,
        radius: 6.9911e7,
        color: [0.83, 0.72, 0.53, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 7.7834082e11,        // 5.20288700 AU
            eccentricity: 0.04838624,
            inclination: (1.30439695_f64).to_radians(),
            longitude_ascending: (100.47390909_f64).to_radians(),
            argument_perihelion: (273.86687493_f64).to_radians(),
            mean_anomaly_epoch: (20.020_f64).to_radians(),
        }),
        rotation_period: Some(35729.7),  // 9 h 55 min 29.7 s (System III)
        axial_tilt: Some(3.1296_f64.to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [25.0e-6, 20.0e-6, 14.0e-6],
            rayleigh_scale_height: 27000.0,
            mie_scattering: [5.0e-6, 5.0e-6, 5.0e-6],
            mie_scale_height: 7000.0,
            atmosphere_height: 500_000.0,
        }),
        star_data: None,
    }
}

/// Saturn — ringed gas giant.
pub fn saturn() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Saturn".into(),
        body_type: BodyType::Planet,
        mass: 5.68319e26,
        radius: 5.8232e7,
        color: [0.87, 0.79, 0.55, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.4335305e12,        // 9.53667594 AU
            eccentricity: 0.05386179,
            inclination: (2.48599187_f64).to_radians(),
            longitude_ascending: (113.66242448_f64).to_radians(),
            argument_perihelion: (339.39150664_f64).to_radians(),
            mean_anomaly_epoch: (317.020_f64).to_radians(),
        }),
        rotation_period: Some(32689.1),  // 10 h 42 min 29.1 s (System III)
        axial_tilt: Some(26.728_f64.to_radians()),
        has_rings: true,
        ring_inner_radius: Some(7.0e7),  // D ring (very faint inner)
        ring_outer_radius: Some(1.4e8),  // A ring outer edge
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [20.0e-6, 17.0e-6, 12.0e-6],
            rayleigh_scale_height: 27000.0,
            mie_scattering: [4.0e-6, 4.0e-6, 4.0e-6],
            mie_scale_height: 6000.0,
            atmosphere_height: 400_000.0,
        }),
        star_data: None,
    }
}

/// Uranus — ice giant, extreme axial tilt.
pub fn uranus() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Uranus".into(),
        body_type: BodyType::Planet,
        mass: 8.68103e25,
        radius: 2.5362e7,
        color: [0.55, 0.77, 0.83, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 2.8722813e12,        // 19.18916464 AU
            eccentricity: 0.04725744,
            inclination: (0.77263783_f64).to_radians(),
            longitude_ascending: (74.01692503_f64).to_radians(),
            argument_perihelion: (96.99885784_f64).to_radians(),
            mean_anomaly_epoch: (142.238_f64).to_radians(),
        }),
        rotation_period: Some(-50803.8), // retrograde, 17 h 14 min 14.8 s
        axial_tilt: Some(97.769_f64.to_radians()), // "rolls" on its orbit
        has_rings: true,
        ring_inner_radius: Some(4.0e7),
        ring_outer_radius: Some(5.2e7),
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [15.0e-6, 18.0e-6, 25.0e-6],
            rayleigh_scale_height: 27700.0,
            mie_scattering: [3.0e-6, 3.0e-6, 3.0e-6],
            mie_scale_height: 5000.0,
            atmosphere_height: 300_000.0,
        }),
        star_data: None,
    }
}

/// Neptune — outermost major planet, ice giant.
pub fn neptune() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Neptune".into(),
        body_type: BodyType::Planet,
        mass: 1.02410e26,
        radius: 2.4622e7,
        color: [0.22, 0.43, 0.85, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 4.4950613e12,        // 30.06992276 AU
            eccentricity: 0.00859048,
            inclination: (1.77004347_f64).to_radians(),
            longitude_ascending: (131.78422574_f64).to_radians(),
            argument_perihelion: (276.33602408_f64).to_radians(),
            mean_anomaly_epoch: (256.228_f64).to_radians(),
        }),
        rotation_period: Some(52601.0),  // 16 h 6 min 36 s (prograde)
        axial_tilt: Some(28.32_f64.to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [12.0e-6, 16.0e-6, 30.0e-6],
            rayleigh_scale_height: 19400.0,
            mie_scattering: [4.0e-6, 4.0e-6, 4.0e-6],
            mie_scale_height: 4000.0,
            atmosphere_height: 300_000.0,
        }),
        star_data: None,
    }
}

// ──────────────────────────────────────────────────────────────
//  The Moon
// ──────────────────────────────────────────────────────────────

/// Earth's Moon — the only natural satellite of Earth.
pub fn earth_moon() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Moon".into(),
        body_type: BodyType::Moon,
        mass: 7.342e22,
        radius: 1.7374e6,
        color: [0.62, 0.62, 0.62, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 3.84399e8,           // 384 399 km
            eccentricity: 0.0549,
            inclination: (5.145_f64).to_radians(),
            longitude_ascending: (125.08_f64).to_radians(),
            argument_perihelion: (318.15_f64).to_radians(),
            mean_anomaly_epoch: (0.0_f64).to_radians(),
        }),
        rotation_period: Some(27.321661 * 86400.0), // tidally locked
        axial_tilt: Some((1.5424_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: None,
    }
}

// ──────────────────────────────────────────────────────────────
//  Galilean moons of Jupiter
// ──────────────────────────────────────────────────────────────

/// Io — innermost Galilean moon, volcanically active.
pub fn io() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Io".into(),
        body_type: BodyType::Moon,
        mass: 8.932e22,
        radius: 1.8216e6,
        color: [0.88, 0.78, 0.32, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 4.21700e8,
            eccentricity: 0.0041,
            inclination: (0.040_f64).to_radians(),
            longitude_ascending: (43.977_f64).to_radians(),
            argument_perihelion: (84.129_f64).to_radians(),
            mean_anomaly_epoch: (180.232_f64).to_radians(),
        }),
        rotation_period: Some(1.769137786 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [1.0e-7, 1.0e-7, 1.0e-7],
            rayleigh_scale_height: 1000.0,
            mie_scattering: [5.0e-8, 5.0e-8, 5.0e-8],
            mie_scale_height: 500.0,
            atmosphere_height: 50_000.0,
        }),
        star_data: None,
    }
}

/// Europa — ice-covered moon, possible subsurface ocean.
pub fn europa() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Europa".into(),
        body_type: BodyType::Moon,
        mass: 4.800e22,
        radius: 1.5608e6,
        color: [0.85, 0.82, 0.75, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 6.70900e8,
            eccentricity: 0.0090,
            inclination: (0.466_f64).to_radians(),
            longitude_ascending: (219.126_f64).to_radians(),
            argument_perihelion: (260.684_f64).to_radians(),
            mean_anomaly_epoch: (12.437_f64).to_radians(),
        }),
        rotation_period: Some(3.551181 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: None,
    }
}

/// Ganymede — largest moon in the Solar System.
pub fn ganymede() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Ganymede".into(),
        body_type: BodyType::Moon,
        mass: 1.482e23,
        radius: 2.6341e6,
        color: [0.70, 0.66, 0.58, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.070400e9,
            eccentricity: 0.0013,
            inclination: (0.177_f64).to_radians(),
            longitude_ascending: (63.523_f64).to_radians(),
            argument_perihelion: (192.452_f64).to_radians(),
            mean_anomaly_epoch: (310.157_f64).to_radians(),
        }),
        rotation_period: Some(7.15455259 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: None,
    }
}

/// Callisto — outermost Galilean moon, heavily cratered.
pub fn callisto() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Callisto".into(),
        body_type: BodyType::Moon,
        mass: 1.076e23,
        radius: 2.4103e6,
        color: [0.55, 0.52, 0.47, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.882700e9,
            eccentricity: 0.0074,
            inclination: (0.186_f64).to_radians(),
            longitude_ascending: (298.848_f64).to_radians(),
            argument_perihelion: (52.643_f64).to_radians(),
            mean_anomaly_epoch: (182.414_f64).to_radians(),
        }),
        rotation_period: Some(16.68901844 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None,
        star_data: None,
    }
}

// ──────────────────────────────────────────────────────────────
//  Saturnian moons
// ──────────────────────────────────────────────────────────────

/// Titan — Saturn's largest moon, thick nitrogen atmosphere.
pub fn titan() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Titan".into(),
        body_type: BodyType::Moon,
        mass: 1.3452e23,
        radius: 2.5747e6,
        color: [0.75, 0.65, 0.42, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 1.221870e9,
            eccentricity: 0.0288,
            inclination: (0.330_f64).to_radians(),
            longitude_ascending: (28.060_f64).to_radians(),
            argument_perihelion: (180.534_f64).to_radians(),
            mean_anomaly_epoch: (120.072_f64).to_radians(),
        }),
        rotation_period: Some(15.945 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: Some(AtmosphereDef {
            rayleigh_scattering: [6.0e-6, 8.0e-6, 12.0e-6],
            rayleigh_scale_height: 15000.0,
            mie_scattering: [4.0e-6, 4.0e-6, 4.0e-6],
            mie_scale_height: 2500.0,
            atmosphere_height: 300_000.0,
        }),
        star_data: None,
    }
}

/// Enceladus — small icy moon with cryovolcanic plumes.
pub fn enceladus() -> CelestialBodyDef {
    CelestialBodyDef {
        name: "Enceladus".into(),
        body_type: BodyType::Moon,
        mass: 1.08020e20,
        radius: 2.521e5,
        color: [0.92, 0.91, 0.88, 1.0],
        orbital: Some(OrbitalElements {
            semi_major_axis: 2.37948e8,
            eccentricity: 0.0047,
            inclination: (0.009_f64).to_radians(),
            longitude_ascending: (28.768_f64).to_radians(),
            argument_perihelion: (116.425_f64).to_radians(),
            mean_anomaly_epoch: (0.524_f64).to_radians(),
        }),
        rotation_period: Some(1.370218 * 86400.0), // tidally locked
        axial_tilt: Some((0.0_f64).to_radians()),
        has_rings: false,
        ring_inner_radius: None,
        ring_outer_radius: None,
        atmosphere: None, // tenuous water-vapor plumes only
        star_data: None,
    }
}
