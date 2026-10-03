//! Cross-platform deterministic math.
//!
//! `f64::sin`, `exp`, `powf`… call the platform's C math library, whose last-bit results
//! differ between operating systems and CPU architectures. For a simulation that must
//! reproduce the same universe from the same seed on every machine (and continue a save
//! identically), the simulation uses these methods instead: they are implemented in pure
//! Rust by the `libm` crate and give bit-identical results everywhere.
//! (`sqrt`, `floor`, `abs`, `+ - * /` are exactly specified by IEEE 754 and need no wrapper.)

pub trait DMath: Sized {
    fn dsin(self) -> Self;
    fn dcos(self) -> Self;
    fn dsin_cos(self) -> (Self, Self);
    fn dtan(self) -> Self;
    fn dasin(self) -> Self;
    fn dacos(self) -> Self;
    fn datan(self) -> Self;
    fn datan2(self, x: Self) -> Self;
    fn dexp(self) -> Self;
    fn dln(self) -> Self;
    fn dlog10(self) -> Self;
    fn dpowf(self, e: Self) -> Self;
    fn dcbrt(self) -> Self;
}

impl DMath for f64 {
    #[inline]
    fn dsin(self) -> f64 {
        libm::sin(self)
    }
    #[inline]
    fn dcos(self) -> f64 {
        libm::cos(self)
    }
    #[inline]
    fn dsin_cos(self) -> (f64, f64) {
        libm::sincos(self)
    }
    #[inline]
    fn dtan(self) -> f64 {
        libm::tan(self)
    }
    #[inline]
    fn dasin(self) -> f64 {
        libm::asin(self)
    }
    #[inline]
    fn dacos(self) -> f64 {
        libm::acos(self)
    }
    #[inline]
    fn datan(self) -> f64 {
        libm::atan(self)
    }
    #[inline]
    fn datan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
    #[inline]
    fn dexp(self) -> f64 {
        libm::exp(self)
    }
    #[inline]
    fn dln(self) -> f64 {
        libm::log(self)
    }
    #[inline]
    fn dlog10(self) -> f64 {
        libm::log10(self)
    }
    #[inline]
    fn dpowf(self, e: f64) -> f64 {
        libm::pow(self, e)
    }
    #[inline]
    fn dcbrt(self) -> f64 {
        libm::cbrt(self)
    }
}

#[cfg(test)]
mod tests {
    use super::DMath;

    #[test]
    fn matches_std_closely() {
        for i in 0..1000 {
            let x = i as f64 * 0.0137 - 5.0;
            assert!((x.dsin() - x.sin()).abs() < 1e-15);
            assert!((x.dexp() - x.exp()).abs() <= 1e-15 * x.exp().max(1.0));
            assert!(((x.abs() + 0.1).dpowf(0.777) - (x.abs() + 0.1).powf(0.777)).abs() < 1e-14);
        }
    }
}
