use std::fmt;

use crate::constants::{EARTH_RADIUS, OBLIQUITY_RAD, PI, TWO_PI};
use crate::math::Vec3d;

// ──────────────────────────────────────────────────────────────
// Reference frames (unit marker structs)
// ──────────────────────────────────────────────────────────────

/// Marker: position relative to the Sun.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heliocentric;

/// Marker: position relative to the solar system barycenter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Barycentric;

/// Marker: position relative to Earth's center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Geocentric;

/// Marker: position relative to a body's surface center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyLocal;

// ──────────────────────────────────────────────────────────────
// CartesianCoord
// ──────────────────────────────────────────────────────────────

/// A 3D Cartesian coordinate in meters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CartesianCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl CartesianCoord {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn to_vec3d(self) -> Vec3d {
        Vec3d::new(self.x, self.y, self.z)
    }

    pub fn from_vec3d(v: Vec3d) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }

    pub fn to_spherical(self) -> SphericalCoord {
        let r = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        let theta = if r == 0.0 { 0.0 } else { (self.z / r).acos() };
        let phi = self.y.atan2(self.x);
        SphericalCoord::new(r, theta, phi)
    }

    pub fn to_orbital(self, forward: Vec3d, up: Vec3d) -> OrbitalCoord {
        let r = self.to_vec3d();
        let radial = r.length();
        let fwd = forward.normalize();
        let n = fwd.cross(up.normalize()).normalize();
        let t = n.cross(fwd);
        OrbitalCoord {
            radial,
            transverse: r.dot(t),
            normal: r.dot(n),
        }
    }

    pub fn to_ecliptic(self) -> EclipticCoord {
        EclipticCoord {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl fmt::Display for CartesianCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3}, {:.3}) m", self.x, self.y, self.z)
    }
}

// ──────────────────────────────────────────────────────────────
// SphericalCoord
// ──────────────────────────────────────────────────────────────

/// Spherical coordinates: radius, polar angle (from +Z), azimuthal angle (from +X in XY plane).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SphericalCoord {
    pub radius: f64,
    pub theta: f64,
    pub phi: f64,
}

impl SphericalCoord {
    pub const fn new(radius: f64, theta: f64, phi: f64) -> Self {
        Self { radius, theta, phi }
    }

    pub fn to_cartesian(self) -> CartesianCoord {
        let sin_theta = self.theta.sin();
        CartesianCoord {
            x: self.radius * sin_theta * self.phi.cos(),
            y: self.radius * sin_theta * self.phi.sin(),
            z: self.radius * self.theta.cos(),
        }
    }
}

impl fmt::Display for SphericalCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(r={:.3}, θ={:.3}, φ={:.3})",
            self.radius, self.theta, self.phi
        )
    }
}

// ──────────────────────────────────────────────────────────────
// OrbitalCoord
// ──────────────────────────────────────────────────────────────

/// Orbital (radial, transverse, normal) coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OrbitalCoord {
    pub radial: f64,
    pub transverse: f64,
    pub normal: f64,
}

impl OrbitalCoord {
    pub const fn new(radial: f64, transverse: f64, normal: f64) -> Self {
        Self { radial, transverse, normal }
    }
}

impl fmt::Display for OrbitalCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(R={:.3}, T={:.3}, N={:.3})",
            self.radial, self.transverse, self.normal
        )
    }
}

// ──────────────────────────────────────────────────────────────
// EclipticCoord
// ──────────────────────────────────────────────────────────────

/// Ecliptic coordinates (Cartesian, aligned to the ecliptic plane).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EclipticCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl EclipticCoord {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Transform from ecliptic to equatorial (applies obliquity rotation about X axis).
    pub fn to_equatorial(self) -> EquatorialCoord {
        let cos_e = OBLIQUITY_RAD.cos();
        let sin_e = OBLIQUITY_RAD.sin();
        EquatorialCoord {
            x: self.x,
            y: self.y * cos_e - self.z * sin_e,
            z: self.y * sin_e + self.z * cos_e,
        }
    }

    pub fn to_cartesian(self) -> CartesianCoord {
        CartesianCoord::new(self.x, self.y, self.z)
    }
}

impl fmt::Display for EclipticCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ecliptic({:.3}, {:.3}, {:.3})", self.x, self.y, self.z)
    }
}

// ──────────────────────────────────────────────────────────────
// EquatorialCoord
// ──────────────────────────────────────────────────────────────

/// Equatorial coordinates (Cartesian, aligned to Earth's equator).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EquatorialCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl EquatorialCoord {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Transform from equatorial to ecliptic (inverse obliquity rotation about X axis).
    pub fn to_ecliptic(self) -> EclipticCoord {
        let cos_e = OBLIQUITY_RAD.cos();
        let sin_e = OBLIQUITY_RAD.sin();
        EclipticCoord {
            x: self.x,
            y: self.y * cos_e + self.z * sin_e,
            z: -self.y * sin_e + self.z * cos_e,
        }
    }

    pub fn to_cartesian(self) -> CartesianCoord {
        CartesianCoord::new(self.x, self.y, self.z)
    }
}

impl fmt::Display for EquatorialCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Equatorial({:.3}, {:.3}, {:.3})", self.x, self.y, self.z)
    }
}

// ──────────────────────────────────────────────────────────────
// Geodetic helpers
// ──────────────────────────────────────────────────────────────

/// Convert geodetic latitude/longitude/altitude to geocentric Cartesian position.
pub fn geodetic_to_geocentric(lat_rad: f64, lon_rad: f64, altitude_m: f64) -> CartesianCoord {
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    // Simplified spherical Earth model
    let r = EARTH_RADIUS + altitude_m;
    CartesianCoord {
        x: r * cos_lat * cos_lon,
        y: r * cos_lat * sin_lon,
        z: r * sin_lat,
    }
}

/// Wrap an angle to [0, 2π).
pub fn wrap_angle_2pi(angle: f64) -> f64 {
    let result = angle % TWO_PI;
    if result < 0.0 {
        result + TWO_PI
    } else {
        result
    }
}

/// Wrap an angle to [-π, π).
pub fn wrap_angle_pi(angle: f64) -> f64 {
    let result = angle % TWO_PI;
    if result > PI {
        result - TWO_PI
    } else if result <= -PI {
        result + TWO_PI
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn cartesian_spherical_roundtrip() {
        let c = CartesianCoord::new(1.0, 2.0, 3.0);
        let s = c.to_spherical();
        let c2 = s.to_cartesian();
        assert!(approx_eq(c.x, c2.x));
        assert!(approx_eq(c.y, c2.y));
        assert!(approx_eq(c.z, c2.z));
    }

    #[test]
    fn cartesian_distance() {
        let a = CartesianCoord::new(0.0, 0.0, 0.0);
        let b = CartesianCoord::new(3.0, 4.0, 0.0);
        assert!(approx_eq(a.distance_to(b), 5.0));
    }

    #[test]
    fn ecliptic_equatorial_roundtrip() {
        let e = EclipticCoord::new(1.0, 2.0, 3.0);
        let eq = e.to_equatorial();
        let e2 = eq.to_ecliptic();
        assert!(approx_eq(e.x, e2.x));
        assert!(approx_eq(e.y, e2.y));
        assert!(approx_eq(e.z, e2.z));
    }

    #[test]
    fn ecliptic_equatorial_obliquity() {
        // A point on the ecliptic plane (z=0) should have non-zero equatorial z
        let e = EclipticCoord::new(1.0, 1.0, 0.0);
        let eq = e.to_equatorial();
        assert!(!approx_eq(eq.z, 0.0));
        // The z in equatorial should be y_ecliptic * sin(obliquity)
        let expected_z = 1.0 * OBLIQUITY_RAD.sin();
        assert!(approx_eq(eq.z, expected_z));
    }

    #[test]
    fn wrap_angle_2pi_positive() {
        let a = wrap_angle_2pi(TWO_PI + 0.5);
        assert!(approx_eq(a, 0.5));
    }

    #[test]
    fn wrap_angle_2pi_negative() {
        let a = wrap_angle_2pi(-0.5);
        assert!(approx_eq(a, TWO_PI - 0.5));
    }

    #[test]
    fn wrap_angle_pi_range() {
        let a = wrap_angle_pi(3.0 * PI);
        assert!(a > -PI && a <= PI);
    }

    #[test]
    fn geodetic_equator() {
        // Latitude = 0, longitude = 0, altitude = 0 → on the equator at +X
        let c = geodetic_to_geocentric(0.0, 0.0, 0.0);
        assert!(approx_eq(c.x, EARTH_RADIUS));
        assert!(approx_eq(c.y, 0.0));
        assert!(approx_eq(c.z, 0.0));
    }

    #[test]
    fn geodetic_north_pole() {
        let c = geodetic_to_geocentric(PI / 2.0, 0.0, 0.0);
        assert!(approx_eq(c.x, 0.0));
        assert!(approx_eq(c.y, 0.0));
        assert!(approx_eq(c.z, EARTH_RADIUS));
    }
}
