//! Vertical prisms (extruded profiles) and their boundary faces.

use crate::{polygon, Aabb3, Vec2, Vec3, EPS};

use serde::{Deserialize, Serialize};

/// A planar boundary face of a solid.
///
/// `hard[i]` tells whether edge `pts[i] -> pts[i+1]` is a real model edge
/// (drawn in elevations / 3D) or an internal split edge that must stay hidden.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Face {
    pub pts: Vec<Vec3>,
    pub hard: Vec<bool>,
    /// Outward unit normal.
    pub normal: Vec3,
}

impl Face {
    pub fn new(pts: Vec<Vec3>, hard: Vec<bool>) -> Self {
        let normal = newell_normal(&pts);
        Self { pts, hard, normal }
    }

    /// Face whose edges are all visible.
    pub fn solid(pts: Vec<Vec3>) -> Self {
        let n = pts.len();
        Self::new(pts, vec![true; n])
    }

    /// Builds a face and flips its winding if needed so the normal agrees with `outward`.
    pub fn oriented(mut pts: Vec<Vec3>, mut hard: Vec<bool>, outward: Vec3) -> Self {
        if newell_normal(&pts).dot(outward) < 0.0 {
            reverse_polygon(&mut pts, &mut hard);
        }
        Self::new(pts, hard)
    }

    pub fn centroid(&self) -> Vec3 {
        if self.pts.is_empty() {
            return Vec3::ZERO;
        }
        self.pts.iter().fold(Vec3::ZERO, |a, p| a + *p) / self.pts.len() as f64
    }
}

/// Polygon normal by Newell's method (robust for slightly non-planar input).
pub fn newell_normal(pts: &[Vec3]) -> Vec3 {
    let n = pts.len();
    let mut v = Vec3::ZERO;
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        v.x += (p.y - q.y) * (p.z + q.z);
        v.y += (p.z - q.z) * (p.x + q.x);
        v.z += (p.x - q.x) * (p.y + q.y);
    }
    v.normalized()
}

fn reverse_polygon(pts: &mut [Vec3], hard: &mut [bool]) {
    let n = pts.len();
    if n == 0 {
        return;
    }
    pts.reverse();
    let old = hard.to_vec();
    for (j, h) in hard.iter_mut().enumerate() {
        *h = old.get((2 * n - 2 - j) % n).copied().unwrap_or(true);
    }
}

/// A 2D profile extruded vertically from `z_min` to `z_max`.
///
/// Every solid in the simplified Plinth model is a prism. This makes plan cuts
/// (horizontal slices), sections (vertical slices) and IFC export
/// (`IfcExtrudedAreaSolid`) exact.
#[derive(Clone, Debug, PartialEq)]
pub struct Prism {
    /// Counter-clockwise profile in plan.
    pub profile: Vec<Vec2>,
    pub z_min: f64,
    pub z_max: f64,
    pub soft_vertices: Vec<bool>,
}

impl Prism {
    pub fn new(mut profile: Vec<Vec2>, z0: f64, z1: f64) -> Self {
        // Remove duplicate consecutive vertices
        profile.dedup_by(|a, b| a.distance(*b) < EPS);
        // Remove closing duplicate if the first vertex is repeated at the end
        if profile.len() >= 2 {
            if let (Some(first), Some(last)) = (profile.first(), profile.last()) {
                if first.distance(*last) < EPS {
                    profile.pop();
                }
            }
        }
        polygon::make_ccw(&mut profile);
        let n = profile.len();
        Self {
            profile,
            z_min: z0.min(z1),
            z_max: z0.max(z1),
            soft_vertices: vec![false; n],
        }
    }

    /// Creates a vertical prism with specific soft edge flags, ensuring counter-clockwise winding.
    pub fn with_soft(mut profile: Vec<Vec2>, z0: f64, z1: f64, mut soft: Vec<bool>) -> Self {
        // Remove duplicate consecutive vertices
        profile.dedup_by(|a, b| a.distance(*b) < EPS);
        if profile.len() >= 2 {
            if let (Some(first), Some(last)) = (profile.first(), profile.last()) {
                if first.distance(*last) < EPS {
                    profile.pop();
                }
            }
        }
        let was_reversed = polygon::signed_area(&profile) < 0.0;
        polygon::make_ccw(&mut profile);
        if was_reversed {
            soft.reverse();
        }
        let n = profile.len();
        soft.resize(n, false);
        Self {
            profile,
            z_min: z0.min(z1),
            z_max: z0.max(z1),
            soft_vertices: soft,
        }
    }

    /// Rectangle of `size_x × size_y` centred on `center`, rotated by `angle` radians.
    pub fn rect(center: Vec2, size_x: f64, size_y: f64, angle: f64, z0: f64, z1: f64) -> Self {
        let ux = Vec2::from_angle(angle);
        let uy = ux.perp();
        let (hx, hy) = (size_x * 0.5, size_y * 0.5);
        Self::new(
            vec![
                center - ux * hx - uy * hy,
                center + ux * hx - uy * hy,
                center + ux * hx + uy * hy,
                center - ux * hx + uy * hy,
            ],
            z0,
            z1,
        )
    }

    /// Box following segment `a → b` with the given plan `width`.
    pub fn along(a: Vec2, b: Vec2, width: f64, z0: f64, z1: f64) -> Self {
        Self::along_with_caps(a, b, width, z0, z1, true, true)
    }

    /// Box following segment `a → b` with plan `width` and optional hard end-caps.
    /// When `cap_a` or `cap_b` is false, internal segment seams are marked soft so no line is drawn
    /// across coplanar continuing walls.
    pub fn along_with_caps(a: Vec2, b: Vec2, width: f64, z0: f64, z1: f64, cap_a: bool, cap_b: bool) -> Self {
        let n_vec = (b - a).normalized().perp() * (width * 0.5);
        let mut profile = vec![a - n_vec, b - n_vec, b + n_vec, a + n_vec];
        let was_reversed = polygon::signed_area(&profile) < 0.0;
        polygon::make_ccw(&mut profile);

        let mut soft = vec![false; 4];
        if !cap_b {
            soft[1] = true;
            soft[2] = true;
        }
        if !cap_a {
            soft[0] = true;
            soft[3] = true;
        }
        if was_reversed {
            soft.reverse();
        }

        Self {
            profile,
            z_min: z0.min(z1),
            z_max: z0.max(z1),
            soft_vertices: soft,
        }
    }

    pub fn height(&self) -> f64 {
        self.z_max - self.z_min
    }

    /// True if the horizontal plane at `z` passes strictly through the prism.
    pub fn spans_z(&self, z: f64) -> bool {
        self.z_min < z - EPS && self.z_max > z + EPS
    }

    pub fn volume(&self) -> f64 {
        polygon::area(&self.profile) * self.height()
    }

    pub fn bounds(&self) -> Aabb3 {
        let mut b = Aabb3::EMPTY;
        for p in &self.profile {
            b.include(p.extend(self.z_min));
            b.include(p.extend(self.z_max));
        }
        b
    }

    /// Boundary faces with outward normals.
    /// Vertical edges between adjacent facets with dihedral turn angle < 35° are marked soft (hard = false)
    /// so curved profiles (like cylinders) render smoothly without facet wireframe lines.
    pub fn faces(&self) -> Vec<Face> {
        let n = self.profile.len();
        let mut faces = Vec::with_capacity(n + 2);
        if n < 3 || self.height() < EPS {
            return faces;
        }

        // Determine if vertex i is a sharp corner (turn angle >= 35 degrees)
        let is_sharp = |i: usize| -> bool {
            let prev = self.profile[(i + n - 1) % n];
            let curr = self.profile[i];
            let next = self.profile[(i + 1) % n];
            let d1 = curr - prev;
            let d2 = next - curr;
            let len1 = d1.length();
            let len2 = d2.length();
            if len1 < EPS || len2 < EPS {
                return true;
            }
            let u1 = d1 / len1;
            let u2 = d2 / len2;
            // cos(35°) ≈ 0.81915. If dot > 0.819, turn angle is < 35° (smooth)
            u1.dot(u2) <= 0.81915
        };

        let sharp_flags: Vec<bool> = (0..n).map(|i| {
            if self.soft_vertices.get(i).copied().unwrap_or(false) {
                false
            } else {
                is_sharp(i)
            }
        }).collect();

        for i in 0..n {
            let a = self.profile[i];
            let b = self.profile[(i + 1) % n];
            if a.distance(b) < EPS {
                continue;
            }
            let pts = vec![
                a.extend(self.z_min),
                b.extend(self.z_min),
                b.extend(self.z_max),
                a.extend(self.z_max),
            ];
            // Edge 0: a(z_min) -> b(z_min) (bottom rim, sharp 90° with bottom cap)
            // Edge 1: b(z_min) -> b(z_max) (vertical seam at vertex b)
            // Edge 2: b(z_max) -> a(z_max) (top rim, sharp 90° with top cap)
            // Edge 3: a(z_max) -> a(z_min) (vertical seam at vertex a)
            let hard = vec![
                true,
                sharp_flags[(i + 1) % n],
                true,
                sharp_flags[i],
            ];
            faces.push(Face::new(pts, hard));
        }
        faces.push(Face::solid(self.profile.iter().map(|p| p.extend(self.z_max)).collect()));
        faces.push(Face::solid(self.profile.iter().rev().map(|p| p.extend(self.z_min)).collect()));
        faces
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_faces_point_outward() {
        let p = Prism::rect(Vec2::ZERO, 2.0, 2.0, 0.0, 0.0, 2.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        let faces = p.faces();
        assert_eq!(faces.len(), 6);
        for f in faces {
            assert!(f.normal.dot(f.centroid() - c) > 0.0);
        }
    }

    #[test]
    fn oriented_flips_flags_consistently() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ];
        // Edge 0 (0,0)->(0,1) is soft.
        let f = Face::oriented(pts, vec![false, true, true, true], Vec3::Z);
        assert!(f.normal.z > 0.0);
        let n = f.pts.len();
        let soft: Vec<_> = (0..n).filter(|i| !f.hard[*i]).collect();
        assert_eq!(soft.len(), 1);
        let (a, b) = (f.pts[soft[0]], f.pts[(soft[0] + 1) % n]);
        assert!(a.x.abs() < 1e-9 && b.x.abs() < 1e-9);
    }

    #[test]
    fn cylinder_facets_have_soft_vertical_edges() {
        let n = 24;
        let mut circle = Vec::new();
        for i in 0..n {
            let theta = (i as f64 / n as f64) * std::f64::consts::TAU;
            circle.push(Vec2::new(1000.0 * theta.cos(), 1000.0 * theta.sin()));
        }
        let cyl = Prism::new(circle, 0.0, 3000.0);
        let faces = cyl.faces();
        assert_eq!(faces.len(), 26); // 24 side faces + 2 caps

        // Check side faces: vertical edges must be soft (false), horizontal rims hard (true)
        for f in &faces[0..24] {
            assert!(f.hard[0], "bottom rim should be hard");
            assert!(!f.hard[1], "vertical seam should be soft");
            assert!(f.hard[2], "top rim should be hard");
            assert!(!f.hard[3], "vertical seam should be soft");
        }
    }
}
