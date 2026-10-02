use crate::math::Vec3d;

/// A wrapper around f64 3D positions providing convenient f32 conversion
/// and floating-origin support for rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct F64Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl F64Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Convert to `glam::Vec3` (f32).
    pub fn as_f32(self) -> glam::Vec3 {
        glam::Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }

    /// Create from `glam::Vec3`.
    pub fn from_f32(v: glam::Vec3) -> Self {
        Self {
            x: v.x as f64,
            y: v.y as f64,
            z: v.z as f64,
        }
    }

    /// Subtract `origin` and convert to f32 for rendering with a floating origin.
    pub fn with_floating_origin(self, origin: &F64Vec3) -> glam::Vec3 {
        glam::Vec3::new(
            (self.x - origin.x) as f32,
            (self.y - origin.y) as f32,
            (self.z - origin.z) as f32,
        )
    }

    /// Euclidean distance to another point (f64).
    pub fn distance_to(self, other: &F64Vec3) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Euclidean distance to another point, returned as f32 for rendering.
    pub fn distance_to_f32(self, other: &F64Vec3) -> f32 {
        self.distance_to(other) as f32
    }

    /// Convert from `Vec3d`.
    pub fn from_vec3d(v: Vec3d) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }

    /// Convert to `Vec3d`.
    pub fn to_vec3d(self) -> Vec3d {
        Vec3d::new(self.x, self.y, self.z)
    }
}

impl From<Vec3d> for F64Vec3 {
    fn from(v: Vec3d) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

impl From<F64Vec3> for Vec3d {
    fn from(v: F64Vec3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64vec3_as_f32() {
        let v = F64Vec3::new(1.0, 2.0, 3.0);
        let f = v.as_f32();
        assert!((f.x - 1.0f32).abs() < 1e-6);
        assert!((f.y - 2.0f32).abs() < 1e-6);
        assert!((f.z - 3.0f32).abs() < 1e-6);
    }

    #[test]
    fn f64vec3_from_f32() {
        let f = glam::Vec3::new(1.5, 2.5, 3.5);
        let v = F64Vec3::from_f32(f);
        assert!((v.x - 1.5).abs() < 1e-10);
        assert!((v.y - 2.5).abs() < 1e-10);
        assert!((v.z - 3.5).abs() < 1e-10);
    }

    #[test]
    fn f64vec3_floating_origin() {
        let pos = F64Vec3::new(1e12, 2e12, 3e12);
        let origin = F64Vec3::new(1e12, 2e12, 3e12);
        let rendered = pos.with_floating_origin(&origin);
        assert!((rendered.x).abs() < 1e-3);
        assert!((rendered.y).abs() < 1e-3);
        assert!((rendered.z).abs() < 1e-3);
    }

    #[test]
    fn f64vec3_distance() {
        let a = F64Vec3::new(0.0, 0.0, 0.0);
        let b = F64Vec3::new(3.0, 4.0, 0.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
        assert!((a.distance_to_f32(&b) - 5.0f32).abs() < 1e-5);
    }
}
