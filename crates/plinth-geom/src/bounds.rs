//! Axis-aligned bounding boxes.

use crate::{Vec2, Vec3};

/// 2D axis-aligned bounding box. Starts out "empty" (inverted infinities).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Default for Aabb2 {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb2 {
    pub const EMPTY: Self = Self {
        min: Vec2::new(f64::INFINITY, f64::INFINITY),
        max: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    pub fn from_points(pts: impl IntoIterator<Item = Vec2>) -> Self {
        let mut b = Self::EMPTY;
        for p in pts {
            b.include(p);
        }
        b
    }
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self { min: a.min(b), max: a.max(b) }
    }
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }
    pub fn include(&mut self, p: Vec2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    pub fn union(&mut self, o: &Aabb2) {
        if o.is_valid() {
            self.include(o.min);
            self.include(o.max);
        }
    }
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
    pub fn expanded(&self, m: f64) -> Self {
        Self { min: self.min - Vec2::new(m, m), max: self.max + Vec2::new(m, m) }
    }
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn contains_box(&self, o: &Aabb2) -> bool {
        o.is_valid() && self.contains(o.min) && self.contains(o.max)
    }
    pub fn intersects(&self, o: &Aabb2) -> bool {
        self.is_valid()
            && o.is_valid()
            && self.min.x <= o.max.x
            && o.min.x <= self.max.x
            && self.min.y <= o.max.y
            && o.min.y <= self.max.y
    }
}

/// 3D axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb3 {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb3 {
    pub const EMPTY: Self = Self {
        min: Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        max: Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
    pub fn include(&mut self, p: Vec3) {
        self.min = Vec3::new(self.min.x.min(p.x), self.min.y.min(p.y), self.min.z.min(p.z));
        self.max = Vec3::new(self.max.x.max(p.x), self.max.y.max(p.y), self.max.z.max(p.z));
    }
    pub fn union(&mut self, o: &Aabb3) {
        if o.is_valid() {
            self.include(o.min);
            self.include(o.max);
        }
    }
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
    pub fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(b.x, b.y, b.z),
            Vec3::new(a.x, b.y, b.z),
        ]
    }
    /// Footprint in plan.
    pub fn xy(&self) -> Aabb2 {
        Aabb2 { min: self.min.xy(), max: self.max.xy() }
    }
}
