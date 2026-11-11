//! 2D and 3D vectors (f64, millimetres).

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A 2D point or vector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

/// A 3D point or vector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Shorthand constructor for [`Vec2`].
#[inline]
pub const fn vec2(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

/// Shorthand constructor for [`Vec3`].
#[inline]
pub const fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec2 {
    pub const ZERO: Vec2 = vec2(0.0, 0.0);
    pub const X: Vec2 = vec2(1.0, 0.0);
    pub const Y: Vec2 = vec2(0.0, 1.0);

    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }
    /// Z component of the 3D cross product (signed parallelogram area).
    #[inline]
    pub fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }
    #[inline]
    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }
    #[inline]
    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }
    pub fn try_normalized(self) -> Option<Self> {
        let l = self.length();
        (l > 1e-12).then(|| self / l)
    }
    /// Unit vector, or zero for a zero-length vector.
    pub fn normalized(self) -> Self {
        self.try_normalized().unwrap_or(Self::ZERO)
    }
    /// Rotated 90° counter-clockwise.
    #[inline]
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
    #[inline]
    pub fn distance(self, o: Self) -> f64 {
        (self - o).length()
    }
    #[inline]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }
    /// Angle from +X in radians, CCW positive.
    #[inline]
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }
    #[inline]
    pub fn from_angle(a: f64) -> Self {
        Self::new(a.cos(), a.sin())
    }
    pub fn rotated(self, a: f64) -> Self {
        let (s, c) = a.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    #[inline]
    pub fn extend(self, z: f64) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }
    #[inline]
    pub fn approx_eq(self, o: Self, tol: f64) -> bool {
        self.distance(o) <= tol
    }
    pub fn min(self, o: Self) -> Self {
        Self::new(self.x.min(o.x), self.y.min(o.y))
    }
    pub fn max(self, o: Self) -> Self {
        Self::new(self.x.max(o.x), self.y.max(o.y))
    }
}

impl Vec3 {
    pub const ZERO: Vec3 = vec3(0.0, 0.0, 0.0);
    pub const X: Vec3 = vec3(1.0, 0.0, 0.0);
    pub const Y: Vec3 = vec3(0.0, 1.0, 0.0);
    pub const Z: Vec3 = vec3(0.0, 0.0, 1.0);

    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn try_normalized(self) -> Option<Self> {
        let l = self.length();
        (l > 1e-12).then(|| self / l)
    }
    pub fn normalized(self) -> Self {
        self.try_normalized().unwrap_or(Self::ZERO)
    }
    #[inline]
    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
    #[inline]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }
    #[inline]
    pub fn distance(self, o: Self) -> f64 {
        (self - o).length()
    }
}

macro_rules! impl_ops {
    ($t:ident { $($f:ident),+ }) => {
        impl Add for $t {
            type Output = $t;
            #[inline]
            fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } }
        }
        impl Sub for $t {
            type Output = $t;
            #[inline]
            fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } }
        }
        impl Mul<f64> for $t {
            type Output = $t;
            #[inline]
            fn mul(self, s: f64) -> $t { $t { $($f: self.$f * s),+ } }
        }
        impl Div<f64> for $t {
            type Output = $t;
            #[inline]
            fn div(self, s: f64) -> $t { $t { $($f: self.$f / s),+ } }
        }
        impl Neg for $t {
            type Output = $t;
            #[inline]
            fn neg(self) -> $t { $t { $($f: -self.$f),+ } }
        }
        impl AddAssign for $t {
            #[inline]
            fn add_assign(&mut self, o: $t) { $(self.$f += o.$f;)+ }
        }
        impl SubAssign for $t {
            #[inline]
            fn sub_assign(&mut self, o: $t) { $(self.$f -= o.$f;)+ }
        }
    };
}

impl_ops!(Vec2 { x, y });
impl_ops!(Vec3 { x, y, z });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perp_is_ccw() {
        assert_eq!(Vec2::X.perp(), Vec2::Y);
        assert!((Vec2::X.cross(Vec2::Y) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cross3() {
        assert_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
    }
}
