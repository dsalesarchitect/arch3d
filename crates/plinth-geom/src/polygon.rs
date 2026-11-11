//! 2D polygon and segment utilities.

use crate::{Vec2, EPS};

/// Signed area (positive for counter-clockwise polygons).
pub fn signed_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        a += pts[i].cross(pts[(i + 1) % n]);
    }
    a * 0.5
}

pub fn area(pts: &[Vec2]) -> f64 {
    signed_area(pts).abs()
}

pub fn is_ccw(pts: &[Vec2]) -> bool {
    signed_area(pts) > 0.0
}

/// Reverses the polygon in place if it is clockwise.
pub fn make_ccw(pts: &mut [Vec2]) {
    if signed_area(pts) < 0.0 {
        pts.reverse();
    }
}

pub fn perimeter(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    if n < 2 {
        return 0.0;
    }
    (0..n).map(|i| pts[i].distance(pts[(i + 1) % n])).sum()
}

/// Area centroid; falls back to the vertex average for degenerate polygons.
pub fn centroid(pts: &[Vec2]) -> Vec2 {
    let n = pts.len();
    if n == 0 {
        return Vec2::ZERO;
    }
    let a = signed_area(pts);
    if a.abs() < EPS {
        return pts.iter().fold(Vec2::ZERO, |acc, p| acc + *p) / n as f64;
    }
    let mut c = Vec2::ZERO;
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        let k = p.cross(q);
        c.x += (p.x + q.x) * k;
        c.y += (p.y + q.y) * k;
    }
    c / (6.0 * a)
}

/// Even-odd point in polygon test.
pub fn point_in_polygon(p: Vec2, pts: &[Vec2]) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (pi, pj) = (pts[i], pts[j]);
        if (pi.y > p.y) != (pj.y > p.y) && p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y) + pi.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Closest point on segment `ab` to `p`, with its parameter `t` in `[0, 1]`.
pub fn closest_point_on_segment(p: Vec2, a: Vec2, b: Vec2) -> (Vec2, f64) {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < EPS {
        return (a, 0.0);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (a + ab * t, t)
}

pub fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    closest_point_on_segment(p, a, b).0.distance(p)
}

pub fn distance_to_polyline(p: Vec2, pts: &[Vec2], closed: bool) -> f64 {
    let n = pts.len();
    match n {
        0 => f64::INFINITY,
        1 => pts[0].distance(p),
        _ => {
            let segs = if closed { n } else { n - 1 };
            (0..segs)
                .map(|i| distance_to_segment(p, pts[i], pts[(i + 1) % n]))
                .fold(f64::INFINITY, f64::min)
        }
    }
}

/// Intersection of infinite lines `p + t·r` and `q + u·s`. Returns `(t, u)`.
pub fn line_intersection(p: Vec2, r: Vec2, q: Vec2, s: Vec2) -> Option<(f64, f64)> {
    let denom = r.cross(s);
    if denom.abs() < 1e-12 {
        return None;
    }
    let w = q - p;
    Some((w.cross(s) / denom, w.cross(r) / denom))
}

/// Parameter intervals `(t0, t1)` (along `origin + t·dir`) where the infinite
/// line lies inside `poly`. Works for concave polygons. Sorted ascending.
pub fn line_polygon_intervals(origin: Vec2, dir: Vec2, poly: &[Vec2]) -> Vec<(f64, f64)> {
    let n = poly.len();
    let mut ts = Vec::new();
    if n < 3 {
        return Vec::new();
    }
    let side = |p: Vec2| dir.cross(p - origin) >= 0.0;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        if side(a) != side(b) {
            if let Some((t, _)) = line_intersection(origin, dir, a, b - a) {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ts.chunks_exact(2).map(|c| (c[0], c[1])).filter(|(a, b)| b - a > EPS).collect()
}

/// True if the polygon is convex (either orientation).
pub fn is_convex(pts: &[Vec2]) -> bool {
    let n = pts.len();
    if n < 4 {
        return n == 3;
    }
    let mut sign = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let c = pts[(i + 2) % n];
        let z = (b - a).cross(c - b);
        if z.abs() > 1e-9 {
            if sign == 0.0 {
                sign = z.signum();
            } else if z.signum() != sign {
                return false;
            }
        }
    }
    true
}

/// Removes consecutive duplicate points (and a duplicated closing point).
pub fn dedup_closed(pts: &[Vec2], tol: f64) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_none_or(|q| !q.approx_eq(*p, tol)) {
            out.push(*p);
        }
    }
    while out.len() > 1 && out[0].approx_eq(*out.last().unwrap(), tol) {
        out.pop();
    }
    out
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let c1 = (b - a).cross(p - a);
    let c2 = (c - b).cross(p - b);
    let c3 = (a - c).cross(p - c);
    c1 >= -1e-9 && c2 >= -1e-9 && c3 >= -1e-9
}

/// Ear-clipping triangulation of a simple polygon (either orientation).
/// Robust against duplicate consecutive vertices, closing loop repeats, and reflex notches.
/// Returns triangles as index triples into `pts`.
pub fn triangulate(pts: &[Vec2]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }

    // Build unique vertex indices (skipping consecutive duplicates within 1e-4 and closing duplicate)
    let mut clean_idx: Vec<usize> = Vec::with_capacity(n);
    for (i, p) in pts.iter().enumerate() {
        if let Some(&last_i) = clean_idx.last() {
            if pts[last_i].distance(*p) < 1e-4 {
                continue;
            }
        }
        clean_idx.push(i);
    }
    while clean_idx.len() > 1 && pts[clean_idx[0]].distance(pts[*clean_idx.last().unwrap()]) < 1e-4 {
        clean_idx.pop();
    }

    let m_pts = clean_idx.len();
    if m_pts < 3 {
        return Vec::new();
    }
    if m_pts == 3 {
        return vec![[clean_idx[0], clean_idx[1], clean_idx[2]]];
    }

    let clean_pts: Vec<Vec2> = clean_idx.iter().map(|&i| pts[i]).collect();
    let mut idx: Vec<usize> = (0..m_pts).collect();
    if signed_area(&clean_pts) < 0.0 {
        idx.reverse();
    }

    let mut tris = Vec::with_capacity(m_pts - 2);
    let mut guard = 0usize;
    while idx.len() > 3 && guard < m_pts * m_pts {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let ia = idx[(i + m - 1) % m];
            let ib = idx[i];
            let ic = idx[(i + 1) % m];
            let (a, b, c) = (clean_pts[ia], clean_pts[ib], clean_pts[ic]);
            if (b - a).cross(c - b) <= 1e-9 {
                continue; // reflex or degenerate corner
            }
            let blocked = idx
                .iter()
                .any(|&j| j != ia && j != ib && j != ic && point_in_triangle(clean_pts[j], a, b, c));
            if blocked {
                continue;
            }
            tris.push([clean_idx[ia], clean_idx[ib], clean_idx[ic]]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }

    if idx.len() == 3 {
        tris.push([clean_idx[idx[0]], clean_idx[idx[1]], clean_idx[idx[2]]]);
    } else if idx.len() > 3 {
        // Fallback: verify that remaining sub-part is strictly convex before fanning
        // to prevent false diagonal chords from fanning across concave bays or voids.
        let rem_pts: Vec<Vec2> = idx.iter().map(|&i| clean_pts[i]).collect();
        if is_convex(&rem_pts) {
            for k in 1..idx.len().saturating_sub(1) {
                tris.push([clean_idx[idx[0]], clean_idx[idx[k]], clean_idx[idx[k + 1]]]);
            }
        }
    }
    tris
}

/// Clips a 2D line segment `[p0, p1]` against an axis-aligned bounding box `[min_pt, max_pt]`
/// using the Liang-Barsky parametric line clipping algorithm.
///
/// Returns `Some((clipped_p0, clipped_p1))` if any part of the segment lies within the box,
/// or `None` if the segment is completely outside.
pub fn clip_segment_to_box(
    p0: Vec2,
    p1: Vec2,
    min_pt: Vec2,
    max_pt: Vec2,
) -> Option<(Vec2, Vec2)> {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;

    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;

    let p = [-dx, dx, -dy, dy];
    let q = [
        p0.x - min_pt.x,
        max_pt.x - p0.x,
        p0.y - min_pt.y,
        max_pt.y - p0.y,
    ];

    for i in 0..4 {
        if p[i].abs() < 1e-12 {
            if q[i] < 0.0 {
                return None; // Parallel and outside
            }
        } else {
            let t = q[i] / p[i];
            if p[i] < 0.0 {
                if t > t1 {
                    return None;
                }
                if t > t0 {
                    t0 = t;
                }
            } else {
                if t < t0 {
                    return None;
                }
                if t < t1 {
                    t1 = t;
                }
            }
        }
    }

    if t0 <= t1 {
        Some((
            Vec2::new(p0.x + t0 * dx, p0.y + t0 * dy),
            Vec2::new(p0.x + t1 * dx, p0.y + t1 * dy),
        ))
    } else {
        None
    }
}

/// Clips a 2D polygon against an axis-aligned bounding box `[min_pt, max_pt]`
/// using the Sutherland-Hodgman polygon clipping algorithm.
///
/// Guarantees that every vertex of the output polygon is strictly within the box,
/// completely preventing screen-splitting chords, rays to infinity, and tessellator overflows.
pub fn clip_polygon_to_box(
    pts: &[Vec2],
    min_pt: Vec2,
    max_pt: Vec2,
) -> Vec<Vec2> {
    if pts.len() < 3 {
        return Vec::new();
    }

    let clip_stage = |input: Vec<Vec2>, is_inside: fn(Vec2, f64) -> bool, intersect: fn(Vec2, Vec2, f64) -> Vec2, bound: f64| -> Vec<Vec2> {
        if input.is_empty() {
            return Vec::new();
        }
        let n = input.len();
        let mut out = Vec::with_capacity(n + 2);
        let mut prev = input[n - 1];
        let mut prev_inside = is_inside(prev, bound);

        for &curr in &input {
            let curr_inside = is_inside(curr, bound);
            if curr_inside {
                if !prev_inside {
                    out.push(intersect(prev, curr, bound));
                }
                out.push(curr);
            } else if prev_inside {
                out.push(intersect(prev, curr, bound));
            }
            prev = curr;
            prev_inside = curr_inside;
        }
        out
    };

    // 1. Left boundary: x >= min_pt.x
    let p1 = clip_stage(
        pts.to_vec(),
        |p, b| p.x >= b,
        |p1, p2, b| {
            let dx = p2.x - p1.x;
            let t = if dx.abs() > 1e-12 { (b - p1.x) / dx } else { 0.0 };
            Vec2::new(b, p1.y + t * (p2.y - p1.y))
        },
        min_pt.x,
    );

    // 2. Right boundary: x <= max_pt.x
    let p2 = clip_stage(
        p1,
        |p, b| p.x <= b,
        |p1, p2, b| {
            let dx = p2.x - p1.x;
            let t = if dx.abs() > 1e-12 { (b - p1.x) / dx } else { 0.0 };
            Vec2::new(b, p1.y + t * (p2.y - p1.y))
        },
        max_pt.x,
    );

    // 3. Bottom boundary: y >= min_pt.y
    let p3 = clip_stage(
        p2,
        |p, b| p.y >= b,
        |p1, p2, b| {
            let dy = p2.y - p1.y;
            let t = if dy.abs() > 1e-12 { (b - p1.y) / dy } else { 0.0 };
            Vec2::new(p1.x + t * (p2.x - p1.x), b)
        },
        min_pt.y,
    );

    // 4. Top boundary: y <= max_pt.y
    let p4 = clip_stage(
        p3,
        |p, b| p.y <= b,
        |p1, p2, b| {
            let dy = p2.y - p1.y;
            let t = if dy.abs() > 1e-12 { (b - p1.y) / dy } else { 0.0 };
            Vec2::new(p1.x + t * (p2.x - p1.x), b)
        },
        max_pt.y,
    );

    dedup_closed(&p4, 1e-4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec2;

    fn l_shape() -> Vec<Vec2> {
        vec![
            vec2(0.0, 0.0),
            vec2(2.0, 0.0),
            vec2(2.0, 1.0),
            vec2(1.0, 1.0),
            vec2(1.0, 2.0),
            vec2(0.0, 2.0),
        ]
    }

    #[test]
    fn area_and_centroid() {
        let sq = vec![vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(2.0, 2.0), vec2(0.0, 2.0)];
        assert!((signed_area(&sq) - 4.0).abs() < 1e-9);
        assert!(centroid(&sq).approx_eq(vec2(1.0, 1.0), 1e-9));
    }

    #[test]
    fn triangulation_preserves_area() {
        let poly = l_shape();
        let tris = triangulate(&poly);
        assert_eq!(tris.len(), 4);
        let sum: f64 = tris.iter().map(|t| area(&[poly[t[0]], poly[t[1]], poly[t[2]]])).sum();
        assert!((sum - area(&poly)).abs() < 1e-9);
    }

    #[test]
    fn concave_line_intervals() {
        let poly = l_shape();
        let iv = line_polygon_intervals(vec2(-1.0, 1.5), vec2(1.0, 0.0), &poly);
        assert_eq!(iv.len(), 1);
        assert!((iv[0].0 - 1.0).abs() < 1e-9 && (iv[0].1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn v_notch_triangulation() {
        let v_notch = vec![
            vec2(0.0, 0.0),
            vec2(4.0, 0.0),
            vec2(4.0, 3.0),
            vec2(2.0, 1.5), // V-notch indented inward
            vec2(0.0, 3.0),
        ];
        let tris = triangulate(&v_notch);
        assert_eq!(tris.len(), 3);
        let sum: f64 = tris.iter().map(|t| area(&[v_notch[t[0]], v_notch[t[1]], v_notch[t[2]]])).sum();
        assert!((sum - area(&v_notch)).abs() < 1e-9);

        // Every triangle centroid must be strictly INSIDE the polygon (never in the notch!)
        for t in &tris {
            let c = (v_notch[t[0]] + v_notch[t[1]] + v_notch[t[2]]) / 3.0;
            assert!(point_in_polygon(c, &v_notch), "Triangle centroid {:?} is outside the polygon!", c);
        }
    }

    #[test]
    fn stepped_courtyard_triangulation() {
        // U-shaped / courtyard building footprint (concave cutout in the center)
        let u_shape = vec![
            vec2(0.0, 0.0),
            vec2(6.0, 0.0),
            vec2(6.0, 6.0),
            vec2(4.0, 6.0),
            vec2(4.0, 2.0), // inner courtyard corner
            vec2(2.0, 2.0), // inner courtyard corner
            vec2(2.0, 6.0),
            vec2(0.0, 6.0),
        ];
        let tris = triangulate(&u_shape);
        assert_eq!(tris.len(), 6);
        let sum: f64 = tris.iter().map(|t| area(&[u_shape[t[0]], u_shape[t[1]], u_shape[t[2]]])).sum();
        assert!((sum - area(&u_shape)).abs() < 1e-9);

        // Every triangle centroid must be strictly INSIDE the U-shape (never in the courtyard void!)
        for t in &tris {
            let c = (u_shape[t[0]] + u_shape[t[1]] + u_shape[t[2]]) / 3.0;
            assert!(point_in_polygon(c, &u_shape), "Triangle centroid {:?} is outside U-shape in the courtyard!", c);
        }
    }

    #[test]
    fn point_inside() {
        let poly = l_shape();
        assert!(point_in_polygon(vec2(0.5, 1.5), &poly));
        assert!(!point_in_polygon(vec2(1.5, 1.5), &poly));
    }

    #[test]
    fn test_clip_segment_to_box() {
        let min_b = vec2(0.0, 0.0);
        let max_b = vec2(10.0, 10.0);

        // Segment completely inside
        let seg1 = clip_segment_to_box(vec2(2.0, 2.0), vec2(8.0, 8.0), min_b, max_b);
        assert!(seg1.is_some());
        let (p0, p1) = seg1.unwrap();
        assert!(p0.approx_eq(vec2(2.0, 2.0), 1e-4));
        assert!(p1.approx_eq(vec2(8.0, 8.0), 1e-4));

        // Segment crossing the box
        let seg2 = clip_segment_to_box(vec2(-5.0, 5.0), vec2(15.0, 5.0), min_b, max_b);
        assert!(seg2.is_some());
        let (p0, p1) = seg2.unwrap();
        assert!(p0.approx_eq(vec2(0.0, 5.0), 1e-4));
        assert!(p1.approx_eq(vec2(10.0, 5.0), 1e-4));

        // Segment completely outside
        let seg3 = clip_segment_to_box(vec2(-10.0, 5.0), vec2(-2.0, 5.0), min_b, max_b);
        assert!(seg3.is_none());
    }

    #[test]
    fn test_clip_polygon_to_box() {
        let min_b = vec2(0.0, 0.0);
        let max_b = vec2(10.0, 10.0);

        // Huge polygon covering and extending far outside the box
        let huge_poly = vec![
            vec2(-50.0, -50.0),
            vec2(50.0, -50.0),
            vec2(50.0, 50.0),
            vec2(-50.0, 50.0),
        ];

        let clipped = clip_polygon_to_box(&huge_poly, min_b, max_b);
        assert_eq!(clipped.len(), 4);
        assert!((area(&clipped) - 100.0).abs() < 1e-4);
        for p in &clipped {
            assert!(p.x >= min_b.x - 1e-4 && p.x <= max_b.x + 1e-4);
            assert!(p.y >= min_b.y - 1e-4 && p.y <= max_b.y + 1e-4);
        }

        // Polygon completely outside
        let outside_poly = vec![
            vec2(20.0, 20.0),
            vec2(30.0, 20.0),
            vec2(30.0, 30.0),
        ];
        let clipped_outside = clip_polygon_to_box(&outside_poly, min_b, max_b);
        assert!(clipped_outside.is_empty());
    }
}
