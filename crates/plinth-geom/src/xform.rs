//! 2D affine transforms used by the Modify tools (move, rotate, mirror).

use crate::Vec2;

/// 2D affine transform `p' = M·p + t` with `M = [[a, b], [c, d]]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xform2 {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub t: Vec2,
}

impl Default for Xform2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Xform2 {
    pub const IDENTITY: Self = Self { a: 1.0, b: 0.0, c: 0.0, d: 1.0, t: Vec2::ZERO };

    pub fn translation(v: Vec2) -> Self {
        Self { t: v, ..Self::IDENTITY }
    }

    /// Rotation by `angle` radians (CCW) about `center`.
    pub fn rotation_about(center: Vec2, angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let m = Self { a: c, b: -s, c: s, d: c, t: Vec2::ZERO };
        Self { t: center - m.apply_vec(center), ..m }
    }

    /// Reflection about the infinite line through `p` and `q`.
    pub fn mirror_about(p: Vec2, q: Vec2) -> Self {
        let u = (q - p).normalized();
        if u == Vec2::ZERO {
            return Self::IDENTITY;
        }
        let m = Self {
            a: 2.0 * u.x * u.x - 1.0,
            b: 2.0 * u.x * u.y,
            c: 2.0 * u.x * u.y,
            d: 2.0 * u.y * u.y - 1.0,
            t: Vec2::ZERO,
        };
        Self { t: p - m.apply_vec(p), ..m }
    }

    #[inline]
    pub fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(self.a * p.x + self.b * p.y + self.t.x, self.c * p.x + self.d * p.y + self.t.y)
    }

    #[inline]
    pub fn apply_vec(&self, v: Vec2) -> Vec2 {
        Vec2::new(self.a * v.x + self.b * v.y, self.c * v.x + self.d * v.y)
    }

    pub fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// True if the transform flips orientation (contains a reflection).
    pub fn is_mirror(&self) -> bool {
        self.det() < 0.0
    }

    /// Transform an angle (radians) – the direction `from_angle(a)` is mapped and re-measured.
    pub fn apply_angle(&self, a: f64) -> f64 {
        self.apply_vec(Vec2::from_angle(a)).angle()
    }

    /// Composition: first `self`, then `next`.
    pub fn then(&self, next: &Xform2) -> Xform2 {
        Xform2 {
            a: next.a * self.a + next.b * self.c,
            b: next.a * self.b + next.b * self.d,
            c: next.c * self.a + next.d * self.c,
            d: next.c * self.b + next.d * self.d,
            t: next.apply(self.t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_quarter_turn() {
        let xf = Xform2::rotation_about(Vec2::new(1.0, 1.0), std::f64::consts::FRAC_PI_2);
        let p = xf.apply(Vec2::new(2.0, 1.0));
        assert!(p.approx_eq(Vec2::new(1.0, 2.0), 1e-9));
    }

    #[test]
    fn mirror_about_y_axis() {
        let xf = Xform2::mirror_about(Vec2::ZERO, Vec2::Y);
        assert!(xf.apply(Vec2::new(3.0, 4.0)).approx_eq(Vec2::new(-3.0, 4.0), 1e-9));
        assert!(xf.is_mirror());
    }
}
