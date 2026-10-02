use crate::constants::{
    AU, EARTH_MASS, EARTH_RADIUS, JUPITER_MASS, JUPITER_RADIUS, LIGHT_YEAR, MOON_MASS,
    MOON_RADIUS, PARSEC, PI, SOLAR_LUMINOSITY, SOLAR_MASS, SOLAR_RADIUS, STEFAN_BOLTZMANN,
};

/// Meters → Astronomical Units.
pub fn meters_to_au(m: f64) -> f64 {
    m / AU
}

/// Astronomical Units → meters.
pub fn au_to_meters(au: f64) -> f64 {
    au * AU
}

/// Meters → kilometers.
pub fn meters_to_km(m: f64) -> f64 {
    m * 1e-3
}

/// Kilometers → meters.
pub fn km_to_meters(km: f64) -> f64 {
    km * 1e3
}

/// Meters → light-years.
pub fn meters_to_ly(m: f64) -> f64 {
    m / LIGHT_YEAR
}

/// Light-years → meters.
pub fn ly_to_meters(ly: f64) -> f64 {
    ly * LIGHT_YEAR
}

/// Meters → parsecs.
pub fn meters_to_parsec(m: f64) -> f64 {
    m / PARSEC
}

/// Kilograms → solar masses.
pub fn kg_to_solar_masses(kg: f64) -> f64 {
    kg / SOLAR_MASS
}

/// Solar masses → kilograms.
pub fn solar_masses_to_kg(sm: f64) -> f64 {
    sm * SOLAR_MASS
}

/// Degrees → radians.
pub fn degrees_to_radians(deg: f64) -> f64 {
    deg * PI / 180.0
}

/// Radians → degrees.
pub fn radians_to_degrees(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// Approximate blackbody color (RGB) for a given temperature in Kelvin.
/// Returns (r, g, b) each in [0.0, 1.0].
pub fn kelvin_to_rgb(temp: f64) -> (f32, f32, f32) {
    let temp = temp.clamp(1000.0, 40000.0) / 100.0;

    let r;
    let g;
    let b;

    if temp <= 66.0 {
        r = 1.0;
        g = (99.4708025861 * temp.ln() - 161.1195681661).clamp(0.0, 255.0) / 255.0;
    } else {
        r = (329.698727446 * (temp - 60.0).powf(-0.1332047592)).clamp(0.0, 255.0) / 255.0;
        g = (288.1221695283 * (temp - 60.0).powf(-0.0755148492)).clamp(0.0, 255.0) / 255.0;
    }

    if temp >= 66.0 {
        b = 1.0;
    } else if temp <= 19.0 {
        b = 0.0;
    } else {
        b = (138.5177312231 * (temp - 10.0).ln() - 305.0447927307).clamp(0.0, 255.0) / 255.0;
    }

    (r as f32, g as f32, b as f32)
}

/// Days → seconds (86400 s/day).
pub fn days_to_seconds(days: f64) -> f64 {
    days * 86400.0
}

/// Julian years (365.25 days) → seconds.
pub fn years_to_seconds(years: f64) -> f64 {
    days_to_seconds(years * 365.25)
}

/// Seconds → days.
pub fn seconds_to_days(s: f64) -> f64 {
    s / 86400.0
}

/// Seconds → Julian years.
pub fn seconds_to_years(s: f64) -> f64 {
    seconds_to_days(s) / 365.25
}

/// Solar radius in meters.
pub const SOLAR_RADIUS_M: f64 = SOLAR_RADIUS;

/// Solar luminosity in watts.
pub const SOLAR_LUMINOSITY_W: f64 = SOLAR_LUMINOSITY;

/// Solar mass in kg.
pub const SOLAR_MASS_KG: f64 = SOLAR_MASS;

/// Earth mass in kg.
pub const EARTH_MASS_KG: f64 = EARTH_MASS;

/// Earth radius in meters.
pub const EARTH_RADIUS_M: f64 = EARTH_RADIUS;

/// Jupiter mass in kg.
pub const JUPITER_MASS_KG: f64 = JUPITER_MASS;

/// Jupiter radius in meters.
pub const JUPITER_RADIUS_M: f64 = JUPITER_RADIUS;

/// Moon mass in kg.
pub const MOON_MASS_KG: f64 = MOON_MASS;

/// Moon radius in meters.
pub const MOON_RADIUS_M: f64 = MOON_RADIUS;

/// Compute radius from mass and density (uniform sphere).
pub fn radius_from_mass_density(mass_kg: f64, density_kg_m3: f64) -> f64 {
    ((3.0 * mass_kg) / (4.0 * PI * density_kg_m3)).cbrt()
}

/// Compute luminosity from radius and temperature (Stefan-Boltzmann).
pub fn luminosity_from_radius_temp(radius_m: f64, temp_k: f64) -> f64 {
    4.0 * PI * radius_m * radius_m * STEFAN_BOLTZMANN * temp_k.powi(4)
}

/// Compute temperature from radius and luminosity.
pub fn temperature_from_radius_luminosity(radius_m: f64, luminosity_w: f64) -> f64 {
    (luminosity_w / (4.0 * PI * radius_m * radius_m * STEFAN_BOLTZMANN)).powf(0.25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AU as AU_M, SOLAR_TEMPERATURE};

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn au_conversion_roundtrip() {
        let m = 1.5 * AU_M;
        let au = meters_to_au(m);
        let m2 = au_to_meters(au);
        assert!(approx_eq(m, m2));
    }

    #[test]
    fn ly_parsec_conversion() {
        let ly = meters_to_ly(PARSEC);
        assert!((ly - 3.26156).abs() < 0.01);
    }

    #[test]
    fn kg_solar_mass_roundtrip() {
        let kg = SOLAR_MASS;
        let sm = kg_to_solar_masses(kg);
        assert!(approx_eq(sm, 1.0));
        let kg2 = solar_masses_to_kg(sm);
        assert!(approx_eq(kg, kg2));
    }

    #[test]
    fn deg_rad_roundtrip() {
        let deg = 45.0;
        let rad = degrees_to_radians(deg);
        let deg2 = radians_to_degrees(rad);
        assert!(approx_eq(deg, deg2));
    }

    #[test]
    fn kelvin_to_rgb_solar() {
        let (r, g, b) = kelvin_to_rgb(SOLAR_TEMPERATURE);
        // Sun should be roughly white-yellow: all channels > 0.5
        assert!(r > 0.5);
        assert!(g > 0.5);
        assert!(b > 0.5);
    }

    #[test]
    fn time_conversions() {
        assert!(approx_eq(days_to_seconds(1.0), 86400.0));
        assert!(approx_eq(seconds_to_days(86400.0), 1.0));
        assert!(approx_eq(years_to_seconds(1.0), 365.25 * 86400.0));
        assert!(approx_eq(seconds_to_years(365.25 * 86400.0), 1.0));
    }

    #[test]
    fn radius_from_mass_density_test() {
        // Earth-like density ~5515 kg/m^3
        let r = radius_from_mass_density(EARTH_MASS, 5515.0);
        // Should be roughly Earth radius
        assert!((r - EARTH_RADIUS).abs() / EARTH_RADIUS < 0.05);
    }

    #[test]
    fn solar_luminosity_from_radius_temp() {
        let l = luminosity_from_radius_temp(SOLAR_RADIUS, SOLAR_TEMPERATURE);
        // Should be roughly solar luminosity
        assert!((l - SOLAR_LUMINOSITY).abs() / SOLAR_LUMINOSITY < 0.05);
    }
}
