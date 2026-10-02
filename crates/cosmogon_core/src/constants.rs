/// Gravitational constant (m^3 kg^-1 s^-2)
pub const G: f64 = 6.67430e-11;

/// Speed of light in vacuum (m/s)
pub const C: f64 = 2.99792458e8;

/// Astronomical Unit (meters)
pub const AU: f64 = 1.496e11;

/// Parsec (meters)
pub const PARSEC: f64 = 3.086e16;

/// Light-year (meters)
pub const LIGHT_YEAR: f64 = 9.461e15;

/// Solar mass (kg)
pub const SOLAR_MASS: f64 = 1.989e30;

/// Solar radius (meters)
pub const SOLAR_RADIUS: f64 = 6.957e8;

/// Solar luminosity (Watts)
pub const SOLAR_LUMINOSITY: f64 = 3.828e26;

/// Solar effective temperature (Kelvin)
pub const SOLAR_TEMPERATURE: f64 = 5778.0;

/// Earth mass (kg)
pub const EARTH_MASS: f64 = 5.972e24;

/// Earth radius (meters)
pub const EARTH_RADIUS: f64 = 6.371e6;

/// Moon mass (kg)
pub const MOON_MASS: f64 = 7.342e22;

/// Moon radius (meters)
pub const MOON_RADIUS: f64 = 1.737e6;

/// Jupiter mass (kg)
pub const JUPITER_MASS: f64 = 1.898e27;

/// Jupiter radius (meters)
pub const JUPITER_RADIUS: f64 = 6.9911e7;

/// Pi
pub const PI: f64 = core::f64::consts::PI;

/// 2 * Pi
pub const TWO_PI: f64 = core::f64::consts::TAU;

/// Degrees to radians conversion factor
pub const DEG_TO_RAD: f64 = PI / 180.0;

/// Radians to degrees conversion factor
pub const RAD_TO_DEG: f64 = 180.0 / PI;

/// Stefan-Boltzmann constant (W m^-2 K^-4)
pub const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;

/// Boltzmann constant (J/K)
pub const BOLTZMANN: f64 = 1.380649e-23;

/// Planck constant (J s)
pub const PLANCK: f64 = 6.62607015e-34;

/// Obliquity of the ecliptic (degrees)
pub const OBLIQUITY: f64 = 23.4397;

/// Obliquity of the ecliptic (radians)
pub const OBLIQUITY_RAD: f64 = OBLIQUITY * DEG_TO_RAD;
