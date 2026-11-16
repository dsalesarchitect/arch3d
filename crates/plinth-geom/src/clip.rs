//! Polygon clipping against half-spaces.

use crate::Vec3;

/// Clips a planar polygon to the region where `f(p) >= 0` (Sutherland–Hodgman).
///
/// `hard[i]` describes edge `pts[i] -> pts[i+1]` (true = drawn outline edge).
/// Flags are carried through the clip; edges created along the clipping
/// boundary are marked soft (`false`) because they are not real model edges.
pub fn clip_polygon(pts: &[Vec3], hard: &[bool], f: impl Fn(Vec3) -> f64) -> (Vec<Vec3>, Vec<bool>) {
    let n = pts.len();
    let mut out = Vec::with_capacity(n + 2);
    let mut flags = Vec::with_capacity(n + 2);
    if n < 3 {
        return (out, flags);
    }
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        let h = hard.get(i).copied().unwrap_or(true);
        let dp = f(p);
        let dq = f(q);
        if dp >= 0.0 {
            out.push(p);
            flags.push(h);
            if dq < 0.0 {
                let t = dp / (dp - dq);
                out.push(p.lerp(q, t));
                flags.push(false);
            }
        } else if dq >= 0.0 {
            let t = dp / (dp - dq);
            out.push(p.lerp(q, t));
            flags.push(h);
        }
    }
    if out.len() < 3 {
        out.clear();
        flags.clear();
    }
    (out, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_square_in_half() {
        let sq = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let (pts, flags) = clip_polygon(&sq, &[true; 4], |p| 1.0 - p.x);
        assert_eq!(pts.len(), 4);
        assert_eq!(flags.iter().filter(|f| !**f).count(), 1);
        assert!(pts.iter().all(|p| p.x <= 1.0 + 1e-9));
    }
}
