use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

// ──────────────────────────────────────────────────────────────
// Vec3d
// ──────────────────────────────────────────────────────────────

/// A 3-component f64 vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3d {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub const fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::ZERO
        } else {
            self / len
        }
    }

    pub fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }

    pub fn clamp_length_max(self, max_len: f64) -> Self {
        let len_sq = self.length_squared();
        if len_sq > max_len * max_len {
            self * (max_len / len_sq.sqrt())
        } else {
            self
        }
    }
}

impl fmt::Display for Vec3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.6}, {:.6}, {:.6})", self.x, self.y, self.z)
    }
}

impl Add for Vec3d {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vec3d {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3d {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl SubAssign for Vec3d {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Mul<f64> for Vec3d {
    type Output = Self;
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl MulAssign<f64> for Vec3d {
    fn mul_assign(&mut self, scalar: f64) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }
}

impl Div<f64> for Vec3d {
    type Output = Self;
    fn div(self, scalar: f64) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl DivAssign<f64> for Vec3d {
    fn div_assign(&mut self, scalar: f64) {
        self.x /= scalar;
        self.y /= scalar;
        self.z /= scalar;
    }
}

impl Neg for Vec3d {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Quatd
// ──────────────────────────────────────────────────────────────

/// A double-precision quaternion (Hamilton convention: xyzw).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quatd {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Quatd {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_axis_angle(axis: Vec3d, angle_rad: f64) -> Self {
        let half = angle_rad * 0.5;
        let s = half.sin();
        let a = axis.normalize();
        Self {
            x: a.x * s,
            y: a.y * s,
            z: a.z * s,
            w: half.cos(),
        }
    }

    pub fn from_rotation_arc(from: Vec3d, to: Vec3d) -> Self {
        let from = from.normalize();
        let to = to.normalize();
        let dot = from.dot(to);

        if dot > 0.999999 {
            Self::IDENTITY
        } else if dot < -0.999999 {
            let ortho = if from.x.abs() < 0.9 {
                from.cross(Vec3d::new(1.0, 0.0, 0.0))
            } else {
                from.cross(Vec3d::new(0.0, 1.0, 0.0))
            }
            .normalize();
            Self::from_axis_angle(ortho, std::f64::consts::PI)
        } else {
            let axis = from.cross(to);
            let w = 1.0 + dot;
            Self { x: axis.x, y: axis.y, z: axis.z, w }.normalize()
        }
    }

    pub fn rotate_vec3(self, v: Vec3d) -> Vec3d {
        let qv = Vec3d::new(self.x, self.y, self.z);
        let uv = qv.cross(v);
        let uuv = qv.cross(uv);
        v + (uv * self.w + uuv) * 2.0
    }

    pub fn slerp(self, other: Self, t: f64) -> Self {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;

        let mut other = other;
        if dot < 0.0 {
            other = Self {
                x: -other.x,
                y: -other.y,
                z: -other.z,
                w: -other.w,
            };
            dot = -dot;
        }

        if dot > 0.9995 {
            let result = Self {
                x: self.x + t * (other.x - self.x),
                y: self.y + t * (other.y - self.y),
                z: self.z + t * (other.z - self.z),
                w: self.w + t * (other.w - self.w),
            };
            return result.normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Self {
            x: self.x * s0 + other.x * s1,
            y: self.y * s0 + other.y * s1,
            z: self.z * s0 + other.z * s1,
            w: self.w * s0 + other.w * s1,
        }
        .normalize()
    }

    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len == 0.0 {
            Self::IDENTITY
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
                w: self.w / len,
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Mat4d
// ──────────────────────────────────────────────────────────────

/// A column-major 4×4 f64 matrix.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mat4d {
    pub cols: [[f64; 4]; 4],
}

impl Mat4d {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub const fn from_cols(
        x_axis: [f64; 4],
        y_axis: [f64; 4],
        z_axis: [f64; 4],
        w_axis: [f64; 4],
    ) -> Self {
        Self {
            cols: [x_axis, y_axis, z_axis, w_axis],
        }
    }

    pub fn translate(x: f64, y: f64, z: f64) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3][0] = x;
        m.cols[3][1] = y;
        m.cols[3][2] = z;
        m
    }

    pub fn scale(x: f64, y: f64, z: f64) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[0][0] = x;
        m.cols[1][1] = y;
        m.cols[2][2] = z;
        m
    }

    pub fn rotate_x(angle_rad: f64) -> Self {
        let (s, c) = angle_rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[1][1] = c;
        m.cols[1][2] = s;
        m.cols[2][1] = -s;
        m.cols[2][2] = c;
        m
    }

    pub fn rotate_y(angle_rad: f64) -> Self {
        let (s, c) = angle_rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[0][0] = c;
        m.cols[0][2] = -s;
        m.cols[2][0] = s;
        m.cols[2][2] = c;
        m
    }

    pub fn rotate_z(angle_rad: f64) -> Self {
        let (s, c) = angle_rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[0][0] = c;
        m.cols[0][1] = s;
        m.cols[1][0] = -s;
        m.cols[1][1] = c;
        m
    }

    pub fn perspective(fov_y_rad: f64, aspect: f64, z_near: f64, z_far: f64) -> Self {
        let f = 1.0 / (fov_y_rad * 0.5).tan();
        let nf = 1.0 / (z_near - z_far);
        Self {
            cols: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (z_far + z_near) * nf, -1.0],
                [0.0, 0.0, 2.0 * z_far * z_near * nf, 0.0],
            ],
        }
    }

    pub fn look_at(eye: Vec3d, center: Vec3d, up: Vec3d) -> Self {
        let f = (center - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Self {
            cols: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    pub fn mul_mat4(self, rhs: Self) -> Self {
        let mut result = [[0.0f64; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[j][i] = self.cols[0][i] * rhs.cols[j][0]
                    + self.cols[1][i] * rhs.cols[j][1]
                    + self.cols[2][i] * rhs.cols[j][2]
                    + self.cols[3][i] * rhs.cols[j][3];
            }
        }
        Self { cols: result }
    }

    pub fn mul_vec4(self, v: [f64; 4]) -> [f64; 4] {
        [
            self.cols[0][0] * v[0]
                + self.cols[1][0] * v[1]
                + self.cols[2][0] * v[2]
                + self.cols[3][0] * v[3],
            self.cols[0][1] * v[0]
                + self.cols[1][1] * v[1]
                + self.cols[2][1] * v[2]
                + self.cols[3][1] * v[3],
            self.cols[0][2] * v[0]
                + self.cols[1][2] * v[1]
                + self.cols[2][2] * v[2]
                + self.cols[3][2] * v[3],
            self.cols[0][3] * v[0]
                + self.cols[1][3] * v[1]
                + self.cols[2][3] * v[2]
                + self.cols[3][3] * v[3],
        ]
    }
}

// ──────────────────────────────────────────────────────────────
// Utility functions
// ──────────────────────────────────────────────────────────────

/// Clamp `v` to `[min, max]`.
pub fn clamp(v: f64, min: f64, max: f64) -> f64 {
    v.max(min).min(max)
}

/// Linearly interpolate between `a` and `b` by `t`.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Euclidean remainder (always positive).
pub fn rem_euclid_f64(x: f64, y: f64) -> f64 {
    x - y * (x / y).floor()
}

/// Hermite smoothstep between 0 and 1.
pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    fn vec_approx_eq(a: Vec3d, b: Vec3d) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
    }

    #[test]
    fn vec3d_basic_ops() {
        let a = Vec3d::new(1.0, 2.0, 3.0);
        let b = Vec3d::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3d::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3d::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vec3d::new(2.0, 4.0, 6.0));
        assert_eq!(b / 2.0, Vec3d::new(2.0, 2.5, 3.0));
        assert_eq!(-a, Vec3d::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn vec3d_dot_cross() {
        let a = Vec3d::new(1.0, 0.0, 0.0);
        let b = Vec3d::new(0.0, 1.0, 0.0);
        assert!(approx_eq(a.dot(b), 0.0));
        assert_eq!(a.cross(b), Vec3d::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn vec3d_normalize() {
        let v = Vec3d::new(3.0, 0.0, 4.0);
        let n = v.normalize();
        assert!(approx_eq(n.length(), 1.0));
        assert!(vec_approx_eq(n, Vec3d::new(0.6, 0.0, 0.8)));
    }

    #[test]
    fn vec3d_lerp() {
        let a = Vec3d::ZERO;
        let b = Vec3d::ONE;
        let mid = a.lerp(b, 0.5);
        assert!(vec_approx_eq(mid, Vec3d::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn quatd_identity_rotate() {
        let q = Quatd::IDENTITY;
        let v = Vec3d::new(1.0, 2.0, 3.0);
        assert!(vec_approx_eq(q.rotate_vec3(v), v));
    }

    #[test]
    fn quatd_axis_angle_90z() {
        let q = Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), std::f64::consts::FRAC_PI_2);
        let v = Vec3d::new(1.0, 0.0, 0.0);
        let rotated = q.rotate_vec3(v);
        assert!(vec_approx_eq(rotated, Vec3d::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn quatd_rotation_arc() {
        let from = Vec3d::new(1.0, 0.0, 0.0);
        let to = Vec3d::new(0.0, 1.0, 0.0);
        let q = Quatd::from_rotation_arc(from, to);
        let rotated = q.rotate_vec3(from);
        assert!(vec_approx_eq(rotated.normalize(), to.normalize()));
    }

    #[test]
    fn mat4d_identity_mul() {
        let m = Mat4d::IDENTITY;
        let v = [1.0, 2.0, 3.0, 1.0];
        assert_eq!(m.mul_vec4(v), v);
    }

    #[test]
    fn mat4d_translate() {
        let m = Mat4d::translate(10.0, 20.0, 30.0);
        let v = [0.0, 0.0, 0.0, 1.0];
        let r = m.mul_vec4(v);
        assert!(approx_eq(r[0], 10.0));
        assert!(approx_eq(r[1], 20.0));
        assert!(approx_eq(r[2], 30.0));
    }

    #[test]
    fn smoothstep_boundaries() {
        assert!(approx_eq(smoothstep(0.0, 1.0, -1.0), 0.0));
        assert!(approx_eq(smoothstep(0.0, 1.0, 0.0), 0.0));
        assert!(approx_eq(smoothstep(0.0, 1.0, 1.0), 1.0));
        assert!(approx_eq(smoothstep(0.0, 1.0, 0.5), 0.5));
    }
}
