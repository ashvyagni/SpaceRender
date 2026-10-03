/// Gravitational constant (m^3 kg^-1 s^-2)
pub const G: f64 = 6.67430e-11;

/// Speed of light in vacuum (m/s)
pub const C: f64 = 2.99792458e8;

/// Astronomical unit (m), IAU 2012 Resolution B2 (exact).
pub const AU: f64 = 1.495_978_707e11;

/// Parsec (m), IAU 2015 Resolution B2 (648000/π AU).
pub const PARSEC: f64 = 3.085_677_581_491_367e16;

/// Light-year (m): c × Julian year.
pub const LIGHT_YEAR: f64 = 9.460_730_472_580_8e15;

/// Nominal solar mass parameter GM☉ (m³/s²), IAU 2015 Resolution B3.
pub const GM_SUN: f64 = 1.327_124_4e20;

/// Solar mass (kg) = GM☉ / G (consistent with [`G`], so N-body orbits use the exact GM☉).
pub const SOLAR_MASS: f64 = GM_SUN / G;

/// Solar radius (meters)
pub const SOLAR_RADIUS: f64 = 6.957e8;

/// Solar luminosity (Watts)
pub const SOLAR_LUMINOSITY: f64 = 3.828e26;

/// Solar effective temperature (Kelvin)
pub const SOLAR_TEMPERATURE: f64 = 5778.0;

/// Earth mass (kg) = GM⊕ / G with GM⊕ = 3.986004418e14 m³/s² (IERS 2010).
pub const EARTH_MASS: f64 = 5.972_17e24;

/// Earth radius (meters)
pub const EARTH_RADIUS: f64 = 6.371e6;

/// Moon mass (kg)
pub const MOON_MASS: f64 = 7.342e22;

/// Moon radius (meters)
pub const MOON_RADIUS: f64 = 1.737e6;

/// Jupiter mass (kg), NASA planetary fact sheet.
pub const JUPITER_MASS: f64 = 1.898_13e27;

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
