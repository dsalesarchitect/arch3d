//! Automated Structural Framing & Substructure Generation Studio:
//! 1. Place Columns on Grid Intersections
//! 2. Place Foundation Footings on Columns
//! 3. Place Beams on Grids / Columns
//! 4. Place Structural Slabs on Closed Column-Beam Bays (Planar Half-Edge Face Extraction)

use crate::theme::AppTheme;
use crate::tool_config::{ToolCategory, ToolConfigStudio};
use egui::{
    Align2, Color32, ComboBox, DragValue, FontId, Pos2, RichText, Sense, Stroke, Ui,
    Vec2 as EguiVec2, Window,
};
use plinth_geom::{polygon, Vec2};
use plinth_model::{Beam, Column, Document, Element, Footing, LevelId, Slab};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoMode {
    ColumnsOnGrids,
    FootingsOnColumns,
    BeamsOnGridsOrColumns,
    SlabsOnClosedBays,
}

impl AutoMode {
    pub fn title(self) -> &'static str {
        match self {
            Self::ColumnsOnGrids => "Place Columns on Grids",
            Self::FootingsOnColumns => "Place Foundation Footings on Columns",
            Self::BeamsOnGridsOrColumns => "Place Beams on Grids / Columns",
            Self::SlabsOnClosedBays => "Place Slabs on Closed Column-Beam Bays",
        }
    }

    pub fn short_tab(self) -> &'static str {
        match self {
            Self::ColumnsOnGrids => "1. Columns on Grids",
            Self::FootingsOnColumns => "2. Footings on Columns",
            Self::BeamsOnGridsOrColumns => "3. Beams on Grids/Cols",
            Self::SlabsOnClosedBays => "4. Slabs on Closed Bays",
        }
    }

    pub fn studio_category(self) -> ToolCategory {
        match self {
            Self::ColumnsOnGrids => ToolCategory::ColumnStruct,
            Self::FootingsOnColumns => ToolCategory::Footing,
            Self::BeamsOnGridsOrColumns => ToolCategory::Beam,
            Self::SlabsOnClosedBays => ToolCategory::SlabStruct,
        }
    }
}

#[derive(Default)]
pub struct AutoDialogSignal {
    pub open_grid_manager: bool,
    pub open_type_studio: Option<ToolCategory>,
}

// ===========================================================================
// 1. Grid Intersection & Automated Column Placement
// ===========================================================================

/// Computes all unique 2D intersections `(point, "A-1")` across datum grid segments in `doc.grids`.
pub fn compute_grid_intersections(doc: &Document) -> Vec<(Vec2, String)> {
    let grids: Vec<_> = doc.grids.values().collect();
    let mut out: Vec<(Vec2, String)> = Vec::new();

    for i in 0..grids.len() {
        let g1 = grids[i];
        let r = g1.end - g1.start;
        if r.length_sq() < 1.0 {
            continue;
        }
        for g2 in grids.iter().skip(i + 1) {
            let s = g2.end - g2.start;
            if s.length_sq() < 1.0 {
                continue;
            }
            if let Some((t, u)) = polygon::line_intersection(g1.start, r, g2.start, s) {
                if (-1e-4..=1.0001).contains(&t) && (-1e-4..=1.0001).contains(&u) {
                    let pt = g1.start + r * t;
                    if !out.iter().any(|(existing, _)| existing.distance(pt) < 15.0) {
                        let a_alpha = g1
                            .name
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_alphabetic())
                            .unwrap_or(false);
                        let b_alpha = g2
                            .name
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_alphabetic())
                            .unwrap_or(false);
                        let tag = if b_alpha && !a_alpha {
                            format!("{}-{}", g2.name, g1.name)
                        } else {
                            format!("{}-{}", g1.name, g2.name)
                        };
                        out.push((pt, tag));
                    }
                }
            }
        }
    }
    out
}

/// Places structural `Column` elements at every grid intersection on `base_level`
/// (skipping intersections that already host a column on `base_level`).
pub fn place_columns_on_grids(
    doc: &mut Document,
    studio: &ToolConfigStudio,
    base_level: LevelId,
    top_level: Option<LevelId>,
) -> usize {
    let intersections = compute_grid_intersections(doc);
    if intersections.is_empty() {
        return 0;
    }

    let existing_locs: Vec<Vec2> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Column(c) if c.base_level == base_level => Some(c.location),
            _ => None,
        })
        .collect();

    let candidates: Vec<(Vec2, String)> = intersections
        .into_iter()
        .filter(|(pt, _)| !existing_locs.iter().any(|ex| ex.distance(*pt) < 100.0))
        .collect();

    if candidates.is_empty() {
        return 0;
    }

    doc.push_undo();
    let count = candidates.len();
    for (pt, grid_tag) in candidates {
        let mut col = Column::new(pt, base_level);
        studio.configure_new_column(doc, &mut col, true);
        col.base_level = base_level;
        col.top_level = top_level;
        col.is_structural = true;
        col.tag = format!("C-{}", grid_tag);
        doc.elements.insert(col.id, Element::Column(col));
    }
    count
}

// ===========================================================================
// 2. Automated Foundation Footing Placement on Columns
// ===========================================================================

/// Returns candidate columns for foundation footings:
/// - If `level_filter` is `Some(lid)`, returns columns whose `base_level == lid`.
/// - If `level_filter` is `None`, automatically selects columns resting on the lowest column level.
pub fn find_foundation_column_targets(
    doc: &Document,
    level_filter: Option<LevelId>,
) -> Vec<(plinth_model::ElementId, Vec2, f64, LevelId, f64, f64, f64, String)> {
    let target_z = if level_filter.is_none() {
        doc.elements
            .values()
            .filter_map(|el| match el {
                Element::Column(c) => Some(doc.level_elevation(c.base_level) + c.base_offset),
                _ => None,
            })
            .fold(f64::INFINITY, f64::min)
    } else {
        0.0
    };

    let existing_footings: Vec<(Option<plinth_model::ElementId>, LevelId, Vec2)> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Footing(f) => Some((f.host_column, f.base_level, f.location)),
            _ => None,
        })
        .collect();

    let mut out = Vec::new();
    for el in doc.elements.values() {
        if let Element::Column(c) = el {
            let matches_lvl = match level_filter {
                Some(lid) => c.base_level == lid,
                None => {
                    let cz = doc.level_elevation(c.base_level) + c.base_offset;
                    target_z.is_finite() && (cz - target_z).abs() < 150.0
                }
            };
            if !matches_lvl {
                continue;
            }
            let already_has_footing = existing_footings.iter().any(|(hc, flvl, floc)| {
                *hc == Some(c.id)
                    || (*flvl == c.base_level && floc.distance(c.location) < 150.0)
            });
            if !already_has_footing {
                out.push((
                    c.id,
                    c.location,
                    c.rotation,
                    c.base_level,
                    c.base_offset,
                    c.width,
                    c.depth,
                    c.tag.clone(),
                ));
            }
        }
    }
    out
}

/// Places isolated structural `Footing` elements beneath columns (aligned with column center,
/// rotation, and base elevation, and sized with at least `min_margin_mm` clear projection per side).
pub fn place_footings_on_columns(
    doc: &mut Document,
    studio: &ToolConfigStudio,
    level_filter: Option<LevelId>,
    min_margin_mm: f64,
) -> usize {
    let targets = find_foundation_column_targets(doc, level_filter);
    if targets.is_empty() {
        return 0;
    }

    doc.push_undo();
    let margin = min_margin_mm.max(0.0);
    let count = targets.len();
    for (idx, (col_id, loc, rot, base_lvl, base_off, cw, cd, ctag)) in
        targets.into_iter().enumerate()
    {
        let mut ft = Footing::new(loc, base_lvl);
        studio.configure_new_footing(doc, &mut ft);
        ft.host_column = Some(col_id);
        ft.location = loc;
        ft.rotation = rot;
        ft.base_level = base_lvl;
        ft.base_offset = base_off;
        ft.top_level = None;
        ft.top_offset = 0.0;
        ft.width = ft.width.max(cw + 2.0 * margin);
        ft.length = ft.length.max(cd + 2.0 * margin);
        ft.tag = if !ctag.is_empty() {
            format!("FT-{}", ctag.trim_start_matches("C-"))
        } else {
            format!("FT-{:02}", idx + 1)
        };
        doc.elements.insert(ft.id, Element::Footing(ft));
    }
    count
}

// ===========================================================================
// 3. Automated Beam Framing on Grids & Columns
// ===========================================================================

fn columns_at_framing_elevation(doc: &Document, z_frame: f64) -> Vec<Vec2> {
    let mut pts: Vec<Vec2> = Vec::new();
    for el in doc.elements.values() {
        if let Element::Column(c) = el {
            let (z0, z1) = c.z_bounds(
                doc.level_elevation(c.base_level),
                c.top_level.map(|l| doc.level_elevation(l)),
            );
            if z0 - 200.0 <= z_frame && z1 + 200.0 >= z_frame {
                if !pts.iter().any(|p| p.distance(c.location) < 80.0) {
                    pts.push(c.location);
                }
            }
        }
    }
    pts
}

fn same_undirected_segment(a1: Vec2, b1: Vec2, a2: Vec2, b2: Vec2, tol: f64) -> bool {
    (a1.distance(a2) < tol && b1.distance(b2) < tol)
        || (a1.distance(b2) < tol && b1.distance(a2) < tol)
}

/// Computes candidate `(start, end)` beam spans at `framing_level` bay-by-bay along grids
/// and orthogonal column alignments (skipping already-existing beams on `framing_level`).
pub fn preview_beam_spans(
    doc: &Document,
    framing_level: LevelId,
    connect_bare_grid_intersections: bool,
    max_column_span_mm: f64,
) -> Vec<(Vec2, Vec2)> {
    let z_frame = doc.level_elevation(framing_level);
    let col_pts = columns_at_framing_elevation(doc, z_frame);
    let grid_ix = compute_grid_intersections(doc);

    let existing_beams: Vec<(Vec2, Vec2)> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Beam(b) if b.level == framing_level => Some((b.start, b.end)),
            _ => None,
        })
        .collect();

    let mut spans: Vec<(Vec2, Vec2)> = Vec::new();
    let mut push_unique = |p1: Vec2, p2: Vec2| {
        if p1.distance(p2) < 300.0 {
            return;
        }
        if existing_beams
            .iter()
            .any(|&(ea, eb)| same_undirected_segment(p1, p2, ea, eb, 120.0))
        {
            return;
        }
        if spans
            .iter()
            .any(|&(sa, sb)| same_undirected_segment(p1, p2, sa, sb, 120.0))
        {
            return;
        }
        spans.push((p1, p2));
    };

    // Pass A: Along every GridLine, connect columns lying on the grid (and optionally bare grid intersections)
    for g in doc.grids.values() {
        let dir_vec = g.end - g.start;
        let len = dir_vec.length();
        if len < 300.0 {
            continue;
        }
        let u = dir_vec / len;
        let mut nodes_t: Vec<(f64, Vec2)> = Vec::new();

        for &cp in &col_pts {
            let (proj, _) = polygon::closest_point_on_segment(cp, g.start, g.end);
            if proj.distance(cp) < 200.0 {
                let t = (cp - g.start).dot(u);
                nodes_t.push((t, cp));
            }
        }

        if connect_bare_grid_intersections || col_pts.is_empty() {
            for (ix_pt, _) in &grid_ix {
                let (proj, _) = polygon::closest_point_on_segment(*ix_pt, g.start, g.end);
                if proj.distance(*ix_pt) < 50.0 {
                    let t = (*ix_pt - g.start).dot(u);
                    if !nodes_t.iter().any(|(_, p)| p.distance(*ix_pt) < 100.0) {
                        nodes_t.push((t, *ix_pt));
                    }
                }
            }
        }

        nodes_t.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        nodes_t.dedup_by(|a, b| (a.0 - b.0).abs() < 100.0);

        for pair in nodes_t.windows(2) {
            let (p1, p2) = (pair[0].1, pair[1].1);
            if p1.distance(p2) <= max_column_span_mm {
                push_unique(p1, p2);
            }
        }
    }

    // Pass B: Orthogonal Column-to-Column framing (handles off-grid columns or gridless models!)
    if col_pts.len() >= 2 {
        // Shared X (Vertical framing lines)
        let mut used_x: Vec<f64> = Vec::new();
        for &p in &col_pts {
            if used_x.iter().any(|&ux| (ux - p.x).abs() < 120.0) {
                continue;
            }
            used_x.push(p.x);
            let mut line_cols: Vec<Vec2> = col_pts
                .iter()
                .copied()
                .filter(|q| (q.x - p.x).abs() < 120.0)
                .collect();
            line_cols.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
            for pair in line_cols.windows(2) {
                if pair[0].distance(pair[1]) <= max_column_span_mm {
                    push_unique(pair[0], pair[1]);
                }
            }
        }

        // Shared Y (Horizontal framing lines)
        let mut used_y: Vec<f64> = Vec::new();
        for &p in &col_pts {
            if used_y.iter().any(|&uy| (uy - p.y).abs() < 120.0) {
                continue;
            }
            used_y.push(p.y);
            let mut line_cols: Vec<Vec2> = col_pts
                .iter()
                .copied()
                .filter(|q| (q.y - p.y).abs() < 120.0)
                .collect();
            line_cols.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            for pair in line_cols.windows(2) {
                if pair[0].distance(pair[1]) <= max_column_span_mm {
                    push_unique(pair[0], pair[1]);
                }
            }
        }
    }

    spans
}

/// Places structural `Beam` elements bay-by-bay along grids and orthogonal column lines at `framing_level`.
pub fn place_beams_on_grids_or_columns(
    doc: &mut Document,
    studio: &ToolConfigStudio,
    framing_level: LevelId,
    connect_bare_grid_intersections: bool,
) -> usize {
    let spans = preview_beam_spans(
        doc,
        framing_level,
        connect_bare_grid_intersections,
        24_000.0,
    );
    if spans.is_empty() {
        return 0;
    }

    doc.push_undo();
    let count = spans.len();
    for (idx, (p1, p2)) in spans.into_iter().enumerate() {
        let mut beam = Beam::new(p1, p2, framing_level);
        studio.configure_new_beam(doc, &mut beam);
        beam.level = framing_level;
        beam.is_structural = true;
        beam.tag = format!("B-{:02}", idx + 1);
        doc.elements.insert(beam.id, Element::Beam(beam));
    }
    count
}

// ===========================================================================
// 4. Closed Column-Beam Bay Detection & Automated Slab Placement
// ===========================================================================

/// Extracts minimal closed counter-clockwise structural bay polygons bounded by `Beam`s
/// (and supported at corners by `Column`s when `require_corner_columns` is true) on `framing_level`
/// using exact Planar Half-Edge Face Extraction.
pub fn find_closed_structural_bays(
    doc: &Document,
    framing_level: LevelId,
    require_corner_columns: bool,
) -> Vec<Vec<Vec2>> {
    let z_frame = doc.level_elevation(framing_level);
    let col_pts = columns_at_framing_elevation(doc, z_frame);

    // Collect beams belonging to or meeting framing_level
    let raw_beams: Vec<(Vec2, Vec2)> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Beam(b) => {
                let (_, z_top) = b.z_bounds(
                    doc.level_elevation(b.level),
                    b.top_level.map(|l| doc.level_elevation(l)),
                );
                if b.level == framing_level || (z_top - z_frame).abs() < 250.0 {
                    Some((b.start, b.end))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    if raw_beams.len() < 3 {
        return Vec::new();
    }

    // Subdivide every beam at intermediate beam-beam intersections and intermediate column centers
    let mut split_edges: Vec<(Vec2, Vec2)> = Vec::new();
    for (i, &(a, b)) in raw_beams.iter().enumerate() {
        let ab = b - a;
        let len = ab.length();
        if len < 100.0 {
            continue;
        }
        let mut ts: Vec<f64> = vec![0.0, 1.0];

        // Split at other beams
        for (j, &(c, d)) in raw_beams.iter().enumerate() {
            if i == j {
                continue;
            }
            let cd = d - c;
            if let Some((t, u)) = polygon::line_intersection(a, ab, c, cd) {
                if (0.001..=0.999).contains(&t) && (-0.01..=1.01).contains(&u) {
                    ts.push(t);
                }
            }
            // T-junction endpoint touching interior of ab
            for ep in [c, d] {
                let (proj, t) = polygon::closest_point_on_segment(ep, a, b);
                if (0.001..=0.999).contains(&t) && proj.distance(ep) < 120.0 {
                    ts.push(t);
                }
            }
        }

        // Split at intermediate columns lying on ab
        for &cp in &col_pts {
            let (proj, t) = polygon::closest_point_on_segment(cp, a, b);
            if (0.001..=0.999).contains(&t) && proj.distance(cp) < 160.0 {
                ts.push(t);
            }
        }

        ts.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-3);

        for pair in ts.windows(2) {
            let p0 = a + ab * pair[0];
            let p1 = a + ab * pair[1];
            if p0.distance(p1) > 100.0 {
                split_edges.push((p0, p1));
            }
        }
    }

    // Build canonical merged vertex table (80 mm tolerance)
    let mut verts: Vec<Vec2> = Vec::new();
    fn intern_vertex(verts: &mut Vec<Vec2>, col_pts: &[Vec2], pt: Vec2) -> usize {
        if let Some(idx) = verts.iter().position(|v| v.distance(pt) < 80.0) {
            idx
        } else {
            // Snap to exact column center if within 160 mm for crisp alignment
            let snapped = col_pts
                .iter()
                .copied()
                .find(|cp| cp.distance(pt) < 160.0)
                .unwrap_or(pt);
            verts.push(snapped);
            verts.len() - 1
        }
    }

    let mut adj: Vec<Vec<usize>> = Vec::new();
    for (p0, p1) in split_edges {
        let u = intern_vertex(&mut verts, &col_pts, p0);
        let v = intern_vertex(&mut verts, &col_pts, p1);
        if adj.len() < verts.len() {
            adj.resize(verts.len(), Vec::new());
        }
        if u != v && !adj[u].contains(&v) {
            adj[u].push(v);
            adj[v].push(u);
        }
    }

    // Iteratively prune cantilever / dead-end edges (degree < 2)
    loop {
        let mut changed = false;
        for u in 0..adj.len() {
            if adj[u].len() == 1 {
                let v = adj[u][0];
                adj[u].clear();
                adj[v].retain(|&x| x != u);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let has_col: Vec<bool> = verts
        .iter()
        .map(|v| col_pts.iter().any(|cp| cp.distance(*v) < 220.0))
        .collect();

    // Sort outgoing neighbors at every vertex CCW by polar angle
    for (u, nbrs) in adj.iter_mut().enumerate() {
        let origin = verts[u];
        nbrs.sort_by(|&a, &b| {
            let da = verts[a] - origin;
            let db = verts[b] - origin;
            let ang_a = da.y.atan2(da.x);
            let ang_b = db.y.atan2(db.x);
            ang_a
                .partial_cmp(&ang_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Directed half-edges: track visited (u, v)
    let mut visited: std::collections::BTreeSet<(usize, usize)> = std::collections::BTreeSet::new();
    let mut bays: Vec<Vec<Vec2>> = Vec::new();

    for start_u in 0..adj.len() {
        for &start_v in &adj[start_u] {
            if visited.contains(&(start_u, start_v)) {
                continue;
            }

            let mut loop_verts: Vec<usize> = Vec::new();
            let mut curr_u = start_u;
            let mut curr_v = start_v;
            let mut valid = true;

            for _ in 0..=verts.len() * 2 {
                if visited.contains(&(curr_u, curr_v)) {
                    valid = curr_u == start_u && curr_v == start_v;
                    break;
                }
                visited.insert((curr_u, curr_v));
                loop_verts.push(curr_u);

                let deg = adj[curr_v].len();
                if deg < 2 {
                    valid = false;
                    break;
                }
                let Some(rev_idx) = adj[curr_v].iter().position(|&w| w == curr_u) else {
                    valid = false;
                    break;
                };
                // Next half-edge around minimal CCW interior face is one step clockwise in `adj[curr_v]`
                let next_w = adj[curr_v][(rev_idx + deg - 1) % deg];
                curr_u = curr_v;
                curr_v = next_w;

                if curr_u == start_u && curr_v == start_v {
                    break;
                }
            }

            if !valid || loop_verts.len() < 3 {
                continue;
            }

            // Reject self-touching figure-8 outer walks where a vertex repeats
            let mut uniq = loop_verts.clone();
            uniq.sort_unstable();
            uniq.dedup();
            if uniq.len() != loop_verts.len() {
                continue;
            }

            let poly_pts: Vec<Vec2> = loop_verts.iter().map(|&idx| verts[idx]).collect();
            let s_area = polygon::signed_area(&poly_pts);
            // Bounded minimal interior faces have strictly positive CCW signed area (> 0.25 m²)
            if s_area < 250_000.0 {
                continue;
            }

            // Verify columns support the structural corners of the bay when requested
            if require_corner_columns {
                let n = loop_verts.len();
                let mut corners_supported = true;
                let mut corner_count = 0usize;
                for k in 0..n {
                    let prev_p = verts[loop_verts[(k + n - 1) % n]];
                    let curr_p = verts[loop_verts[k]];
                    let next_p = verts[loop_verts[(k + 1) % n]];
                    let d1 = (curr_p - prev_p).normalized();
                    let d2 = (next_p - curr_p).normalized();
                    let is_corner = d1.cross(d2).abs() > 0.08;
                    if is_corner {
                        corner_count += 1;
                        if !has_col[loop_verts[k]] {
                            corners_supported = false;
                            break;
                        }
                    }
                }
                if !corners_supported || corner_count < 3 {
                    continue;
                }
            }

            // Simplify collinear intermediate vertices along straight bay edges for clean monolithic polygons
            let mut simplified: Vec<Vec2> = Vec::with_capacity(poly_pts.len());
            let n = poly_pts.len();
            for k in 0..n {
                let prev_p = poly_pts[(k + n - 1) % n];
                let curr_p = poly_pts[k];
                let next_p = poly_pts[(k + 1) % n];
                let d1 = (curr_p - prev_p).normalized();
                let d2 = (next_p - curr_p).normalized();
                if d1.cross(d2).abs() > 0.02 {
                    simplified.push(curr_p);
                }
            }
            if simplified.len() >= 3 {
                bays.push(simplified);
            }
        }
    }

    // Filter out bays that already contain an existing Slab on `framing_level`
    let existing_slabs: Vec<&Vec<Vec2>> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Slab(s) if s.level == framing_level && s.boundary.len() >= 3 => {
                Some(&s.boundary)
            }
            _ => None,
        })
        .collect();

    bays.retain(|bay| {
        let c = polygon::centroid(bay);
        !existing_slabs
            .iter()
            .any(|es| polygon::point_in_polygon(c, es))
    });

    bays
}

/// Insets a minimal closed counter-clockwise bay polygon so that its perimeter edges align
/// exactly with the inner vertical faces of the framing `Beam`s on `framing_level`.
pub fn align_bay_to_beam_inner_surfaces(
    doc: &Document,
    bay: &[Vec2],
    framing_level: LevelId,
) -> Vec<Vec2> {
    let n = bay.len();
    if n < 3 {
        return bay.to_vec();
    }

    let z_frame = doc.level_elevation(framing_level);

    // Collect all beams on or near framing_level
    let beams: Vec<(Vec2, Vec2, f64)> = doc
        .elements
        .values()
        .filter_map(|el| match el {
            Element::Beam(b) => {
                let (_, z_top) = b.z_bounds(
                    doc.level_elevation(b.level),
                    b.top_level.map(|l| doc.level_elevation(l)),
                );
                if b.level == framing_level || (z_top - z_frame).abs() < 250.0 {
                    Some((b.start, b.end, b.width))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    // For each directed edge k of the CCW bay polygon, find the framing beam width and compute inward offset line
    let mut edge_lines: Vec<(Vec2, Vec2)> = Vec::with_capacity(n); // (point on line, unit direction)

    for k in 0..n {
        let p0 = bay[k];
        let p1 = bay[(k + 1) % n];
        let edge_vec = p1 - p0;
        let edge_len = edge_vec.length();
        if edge_len < 1.0 {
            edge_lines.push((p0, plinth_geom::vec2(1.0, 0.0)));
            continue;
        }
        let u = edge_vec / edge_len;
        // CCW polygon: inward normal pointing into the bay is u.perp() = (-u.y, u.x)
        let inward_norm = plinth_geom::vec2(-u.y, u.x);

        // Find matching beam along this edge
        let edge_mid = (p0 + p1) * 0.5;
        let mut beam_half_w = 150.0; // default 300mm beam half-width
        for &(b_start, b_end, b_width) in &beams {
            let (proj, _) = polygon::closest_point_on_segment(edge_mid, b_start, b_end);
            if proj.distance(edge_mid) < 200.0 {
                let b_dir = (b_end - b_start).normalized();
                if u.dot(b_dir).abs() > 0.85 {
                    beam_half_w = (b_width * 0.5).max(50.0);
                    break;
                }
            }
        }

        // Shift edge line inward by beam_half_w
        let p_shifted = p0 + inward_norm * beam_half_w;
        edge_lines.push((p_shifted, u));
    }

    // Intersect consecutive shifted edge lines to produce the inner corner vertices
    let mut inner_pts: Vec<Vec2> = Vec::with_capacity(n * 2);
    for k in 0..n {
        let prev_k = (k + n - 1) % n;
        let (a_prev, u_prev) = edge_lines[prev_k];
        let (a_curr, u_curr) = edge_lines[k];

        let det = u_prev.cross(u_curr); // u_prev.x * u_curr.y - u_prev.y * u_curr.x
        let corner = if det.abs() > 0.05 {
            // Lines intersect at a crisp corner
            let dp = a_curr - a_prev;
            let t = dp.cross(u_curr) / det;
            let c = a_prev + u_prev * t;
            // Sanity check: corner shouldn't explode (e.g. within 1500mm of original vertex)
            if c.distance(bay[k]) < 1500.0 {
                c
            } else {
                bay[k] + (edge_lines[k].0 - bay[k])
            }
        } else {
            // Parallel or nearly collinear: offset directly along vertex normal
            let u = edge_lines[k].1;
            let inward_norm = plinth_geom::vec2(-u.y, u.x);
            bay[k] + inward_norm * 150.0
        };

        // Check if there is a supporting column at this corner vertex bay[k]
        let col_at_corner = doc.elements.values().find_map(|el| match el {
            Element::Column(col) => {
                let (c_z0, c_z1) = col.z_bounds(
                    doc.level_elevation(col.base_level),
                    col.top_level.map(|l| doc.level_elevation(l)),
                );
                if col.location.distance(bay[k]) < 400.0 && z_frame >= c_z0 - 100.0 && z_frame <= c_z1 + 100.0 {
                    Some(col)
                } else {
                    None
                }
            }
            _ => None,
        });

        if let Some(col) = col_at_corner {
            let col_fp = col.footprint();
            // If the beam-inner corner penetrates into the column footprint, notch the slab
            // around the column inner vertical faces so the column top cap remains 100% visible and unencroached!
            if polygon::point_in_polygon(corner, &col_fp) {
                let n_fp = col_fp.len();
                let mut p_enter = None;
                for i in 0..n_fp {
                    let e0 = col_fp[i];
                    let e1 = col_fp[(i + 1) % n_fp];
                    if let Some((t, s)) = polygon::line_intersection(a_prev, u_prev, e0, e1 - e0) {
                        if s >= -0.01 && s <= 1.01 && t > 0.0 {
                            let pt = a_prev + u_prev * t;
                            if p_enter.map_or(true, |p: Vec2| pt.distance(corner) < p.distance(corner)) {
                                p_enter = Some(pt);
                            }
                        }
                    }
                }

                let mut p_exit = None;
                for i in 0..n_fp {
                    let e0 = col_fp[i];
                    let e1 = col_fp[(i + 1) % n_fp];
                    if let Some((t, s)) = polygon::line_intersection(a_curr, u_curr, e0, e1 - e0) {
                        if s >= -0.01 && s <= 1.01 && t > -0.01 {
                            let pt = a_curr + u_curr * t;
                            if p_exit.map_or(true, |p: Vec2| pt.distance(corner) > p.distance(corner)) {
                                p_exit = Some(pt);
                            }
                        }
                    }
                }

                let inward_bisector = (plinth_geom::vec2(-u_prev.y, u_prev.x) + plinth_geom::vec2(-u_curr.y, u_curr.x)).normalized();
                let col_inner = col_fp.iter().copied().max_by(|&a, &b| {
                    (a - col.location).dot(inward_bisector).total_cmp(&(b - col.location).dot(inward_bisector))
                }).unwrap_or(corner);

                if let (Some(pe), Some(px)) = (p_enter, p_exit) {
                    if pe.distance(col_inner) > 1.0 {
                        inner_pts.push(pe);
                    }
                    inner_pts.push(col_inner);
                    if px.distance(col_inner) > 1.0 {
                        inner_pts.push(px);
                    }
                } else {
                    inner_pts.push(col_inner);
                }
                continue;
            }
        }

        inner_pts.push(corner);
    }

    let inner_pts = polygon::dedup_closed(&inner_pts, 2.0);

    // Validate that inner_pts is a valid CCW polygon with positive area (> 0.1 m²)
    if inner_pts.len() >= 3 {
        let s_area = polygon::signed_area(&inner_pts);
        if s_area > 100_000.0 {
            return inner_pts;
        }
    }

    bay.to_vec()
}

/// Places structural `Slab` elements inside every closed bay bounded by columns and beams on `framing_level`.
/// Note: Slabs automatically attach to the inner vertical faces of the bounding beams, keeping the bay
/// framed cleanly with solid column top surfaces!
pub fn place_slabs_on_closed_bays(
    doc: &mut Document,
    studio: &ToolConfigStudio,
    framing_level: LevelId,
    require_corner_columns: bool,
) -> usize {
    let bays = find_closed_structural_bays(doc, framing_level, require_corner_columns);
    if bays.is_empty() {
        return 0;
    }

    doc.push_undo();
    let mut created = 0usize;
    for (idx, mut bay) in bays.into_iter().enumerate() {
        polygon::make_ccw(&mut bay);
        let inner_bay = align_bay_to_beam_inner_surfaces(doc, &bay, framing_level);
        let mut slab = Slab::new(inner_bay, framing_level);
        studio.configure_new_slab(doc, &mut slab, true);
        slab.level = framing_level;
        slab.is_structural = true;
        slab.tag = format!("SL-{:02}", idx + 1);
        doc.elements.insert(slab.id, Element::Slab(slab));
        created += 1;
    }
    created
}

// ===========================================================================
// Structural Automation Studio Dialog UI
// ===========================================================================

pub struct StructureAutoDialog {
    pub is_open: bool,
    pub mode: AutoMode,
    pub column_base_level: Option<LevelId>,
    pub column_top_level: Option<LevelId>,
    pub footing_filter_active_level_only: bool,
    pub footing_min_margin_mm: f64,
    pub beam_framing_level: Option<LevelId>,
    pub connect_bare_grids: bool,
    pub slab_framing_level: Option<LevelId>,
    pub require_corner_columns: bool,
    pub status_msg: String,
}

impl Default for StructureAutoDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            mode: AutoMode::ColumnsOnGrids,
            column_base_level: None,
            column_top_level: None,
            footing_filter_active_level_only: false,
            footing_min_margin_mm: 300.0,
            beam_framing_level: None,
            connect_bare_grids: false,
            slab_framing_level: None,
            require_corner_columns: true,
            status_msg: String::new(),
        }
    }
}

impl StructureAutoDialog {
    pub fn open_mode(&mut self, mode: AutoMode, doc: &Document) {
        self.mode = mode;
        self.is_open = true;

        let lvls = ToolConfigStudio::level_pairs(doc);
        let active = doc.active_level_id;
        let next_up = ToolConfigStudio::next_level_above_in_slice(&lvls, active);

        if self.column_base_level.is_none()
            || !doc
                .levels
                .contains_key(&self.column_base_level.unwrap())
        {
            self.column_base_level = Some(active);
            self.column_top_level = next_up;
        }
        if self.beam_framing_level.is_none()
            || !doc
                .levels
                .contains_key(&self.beam_framing_level.unwrap())
        {
            // Default beam framing level to top of active level (Level 2) if available, else active
            self.beam_framing_level = next_up.or(Some(active));
        }
        if self.slab_framing_level.is_none()
            || !doc
                .levels
                .contains_key(&self.slab_framing_level.unwrap())
        {
            self.slab_framing_level = self.beam_framing_level.or(Some(active));
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        studio: &ToolConfigStudio,
        theme: &AppTheme,
    ) -> AutoDialogSignal {
        let mut signal = AutoDialogSignal::default();
        if !self.is_open {
            return signal;
        }

        // Keep selected levels valid if levels were edited
        let mut sorted_levels: Vec<(LevelId, String, f64)> = doc
            .levels
            .values()
            .map(|l| (l.id, l.name.clone(), l.elevation))
            .collect();
        sorted_levels.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let active_lvl = doc.active_level_id;
        let col_base = self
            .column_base_level
            .filter(|id| doc.levels.contains_key(id))
            .unwrap_or(active_lvl);
        self.column_base_level = Some(col_base);
        if let Some(ctop) = self.column_top_level {
            if !doc.levels.contains_key(&ctop) {
                self.column_top_level = None;
            }
        }
        let beam_lvl = self
            .beam_framing_level
            .filter(|id| doc.levels.contains_key(id))
            .unwrap_or(active_lvl);
        self.beam_framing_level = Some(beam_lvl);
        let slab_lvl = self
            .slab_framing_level
            .filter(|id| doc.levels.contains_key(id))
            .unwrap_or(beam_lvl);
        self.slab_framing_level = Some(slab_lvl);

        let mut open = self.is_open;
        Window::new("🏗 Structural Automation Studio")
            .open(&mut open)
            .default_size([860.0, 500.0])
            .min_width(700.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Mode Selector Tabs
                ui.horizontal_wrapped(|ui| {
                    for m in [
                        AutoMode::ColumnsOnGrids,
                        AutoMode::FootingsOnColumns,
                        AutoMode::BeamsOnGridsOrColumns,
                        AutoMode::SlabsOnClosedBays,
                    ] {
                        let sel = self.mode == m;
                        if ui
                            .selectable_label(sel, RichText::new(m.short_tab()).strong())
                            .clicked()
                        {
                            self.mode = m;
                        }
                    }
                });
                ui.separator();

                let cat = self.mode.studio_category();
                let preset_name = studio
                    .active_preset(cat)
                    .map(|p| p.type_name.clone())
                    .unwrap_or_else(|| "Standard Structural Type".into());

                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.mode.title()).strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("⚙ Configure Type Preset...").color(theme.accent))
                            .clicked()
                        {
                            signal.open_type_studio = Some(cat);
                        }
                        ui.label(format!("Active Type: {}", preset_name));
                    });
                });

                ui.add_space(4.0);

                ui.columns(2, |cols| {
                    // Left column: Mode Parameters & One-Click Execution
                    cols[0].group(|ui| {
                        match self.mode {
                            AutoMode::ColumnsOnGrids => {
                                let ix = compute_grid_intersections(doc);
                                ui.label(format!(
                                    "Detected Grid Intersections: {} (across {} Grid Lines)",
                                    ix.len(),
                                    doc.grids.len()
                                ));
                                if doc.grids.is_empty() {
                                    ui.colored_label(
                                        Color32::from_rgb(255, 180, 70),
                                        "Tip: Create an orthogonal grid first using Grid Manager!",
                                    );
                                    if ui.button("📏 Open Grid Manager...").clicked() {
                                        signal.open_grid_manager = true;
                                    }
                                }
                                ui.separator();
                                level_combo(
                                    ui,
                                    "Base Level:",
                                    "auto_col_base",
                                    &sorted_levels,
                                    &mut self.column_base_level,
                                    false,
                                );
                                level_combo(
                                    ui,
                                    "Top Level:",
                                    "auto_col_top",
                                    &sorted_levels,
                                    &mut self.column_top_level,
                                    true,
                                );

                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("⚡ Place Columns on All Grid Intersections")
                                            .strong()
                                            .size(13.0)
                                            .color(theme.accent),
                                    )
                                    .clicked()
                                {
                                    let n = place_columns_on_grids(
                                        doc,
                                        studio,
                                        self.column_base_level.unwrap_or(active_lvl),
                                        self.column_top_level,
                                    );
                                    self.status_msg = format!(
                                        "Placed {} structural column(s) at grid intersections.",
                                        n
                                    );
                                }
                            }

                            AutoMode::FootingsOnColumns => {
                                let filter = if self.footing_filter_active_level_only {
                                    Some(active_lvl)
                                } else {
                                    None
                                };
                                let targets = find_foundation_column_targets(doc, filter);
                                ui.label(format!(
                                    "Eligible Foundation Columns without Footings: {}",
                                    targets.len()
                                ));
                                ui.separator();
                                ui.checkbox(
                                    &mut self.footing_filter_active_level_only,
                                    "Only Columns on Active Level (unchecked = Lowest Level Columns)",
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Min Side Projection:");
                                    ui.add(
                                        DragValue::new(&mut self.footing_min_margin_mm)
                                            .speed(25.0)
                                            .range(0.0..=2500.0)
                                            .suffix(" mm"),
                                    );
                                });

                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("⚡ Place Foundation Footings on Columns")
                                            .strong()
                                            .size(13.0)
                                            .color(theme.accent),
                                    )
                                    .clicked()
                                {
                                    let n = place_footings_on_columns(
                                        doc,
                                        studio,
                                        filter,
                                        self.footing_min_margin_mm,
                                    );
                                    self.status_msg = format!(
                                        "Placed {} isolated foundation footing(s) beneath columns.",
                                        n
                                    );
                                }
                            }

                            AutoMode::BeamsOnGridsOrColumns => {
                                level_combo(
                                    ui,
                                    "Framing Level:",
                                    "auto_beam_lvl",
                                    &sorted_levels,
                                    &mut self.beam_framing_level,
                                    false,
                                );
                                ui.checkbox(
                                    &mut self.connect_bare_grids,
                                    "Also Connect Grid Intersections Without Columns",
                                );
                                let f_lvl = self.beam_framing_level.unwrap_or(active_lvl);
                                let preview = preview_beam_spans(
                                    doc,
                                    f_lvl,
                                    self.connect_bare_grids,
                                    24_000.0,
                                );
                                ui.label(format!("Candidate New Beam Spans: {}", preview.len()));

                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("⚡ Place Beams on Grids / Columns")
                                            .strong()
                                            .size(13.0)
                                            .color(theme.accent),
                                    )
                                    .clicked()
                                {
                                    let n = place_beams_on_grids_or_columns(
                                        doc,
                                        studio,
                                        f_lvl,
                                        self.connect_bare_grids,
                                    );
                                    self.status_msg = format!(
                                        "Placed {} structural beam span(s) at framing level.",
                                        n
                                    );
                                }
                            }

                            AutoMode::SlabsOnClosedBays => {
                                level_combo(
                                    ui,
                                    "Slab Framing Level:",
                                    "auto_slab_lvl",
                                    &sorted_levels,
                                    &mut self.slab_framing_level,
                                    false,
                                );
                                ui.checkbox(
                                    &mut self.require_corner_columns,
                                    "Require Structural Columns at Bay Corners",
                                );
                                let s_lvl = self.slab_framing_level.unwrap_or(active_lvl);
                                let bays = find_closed_structural_bays(
                                    doc,
                                    s_lvl,
                                    self.require_corner_columns,
                                );
                                ui.label(format!("Detected Closed Structural Bays: {}", bays.len()));

                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("⚡ Place Slabs on Closed Column-Beam Bays")
                                            .strong()
                                            .size(13.0)
                                            .color(theme.accent),
                                    )
                                    .clicked()
                                {
                                    let n = place_slabs_on_closed_bays(
                                        doc,
                                        studio,
                                        s_lvl,
                                        self.require_corner_columns,
                                    );
                                    self.status_msg = format!(
                                        "Placed {} structural bay slab(s) inside closed beam-column bays.",
                                        n
                                    );
                                }
                            }
                        }
                    });

                    cols[0].add_space(8.0);
                    cols[0].group(|ui| {
                        ui.label(
                            RichText::new("🚀 Full Structural Bay Pipeline (One-Click)")
                                .strong()
                                .size(11.5),
                        );
                        ui.label(
                            "Runs Columns on Grids → Footings on Columns → Upper Beams → Closed Bay Slabs:",
                        );
                        if ui
                            .button(RichText::new("🚀 Generate Complete Structural Frame").strong())
                            .clicked()
                        {
                            let base_l = self.column_base_level.unwrap_or(active_lvl);
                            let lvls = ToolConfigStudio::level_pairs(doc);
                            let top_l = self
                                .column_top_level
                                .or_else(|| ToolConfigStudio::next_level_above_in_slice(&lvls, base_l));
                            let frame_l = top_l.unwrap_or(base_l);

                            let c_n = place_columns_on_grids(doc, studio, base_l, top_l);
                            let f_n = place_footings_on_columns(
                                doc,
                                studio,
                                Some(base_l),
                                self.footing_min_margin_mm,
                            );
                            let b_n = place_beams_on_grids_or_columns(
                                doc,
                                studio,
                                frame_l,
                                self.connect_bare_grids,
                            );
                            let s_n = place_slabs_on_closed_bays(doc, studio, frame_l, true);
                            self.status_msg = format!(
                                "Full Frame Complete: +{} Columns, +{} Footings, +{} Beams, +{} Bay Slabs!",
                                c_n, f_n, b_n, s_n
                            );
                        }
                    });

                    // Right column: Live 2D Structural Automation Blueprint Preview
                    cols[1].label(
                        RichText::new("Live Candidate Automation Preview (Orange = To Generate)")
                            .strong()
                            .size(12.0),
                    );
                    draw_automation_preview(&mut cols[1], doc, self, theme);
                });

                if !self.status_msg.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status_msg).size(11.5).color(theme.accent));
                }
            });

        self.is_open = open;
        signal
    }
}

fn level_combo(
    ui: &mut Ui,
    label: &str,
    salt: &str,
    levels: &[(LevelId, String, f64)],
    selected: &mut Option<LevelId>,
    allow_unconnected: bool,
) {
    let text = match *selected {
        Some(id) => levels
            .iter()
            .find(|(lid, _, _)| *lid == id)
            .map(|(_, nm, z)| format!("{} ({:+.0} mm)", nm, z))
            .unwrap_or_else(|| "Select Level".into()),
        None => "Unconnected".into(),
    };
    ui.horizontal(|ui| {
        ui.label(label);
        ComboBox::from_id_salt(salt)
            .selected_text(text)
            .show_ui(ui, |ui| {
                if allow_unconnected {
                    ui.selectable_value(selected, None, "Unconnected");
                }
                for (lid, nm, z) in levels {
                    ui.selectable_value(selected, Some(*lid), format!("{} ({:+.0} mm)", nm, z));
                }
            });
    });
}

fn draw_automation_preview(
    ui: &mut Ui,
    doc: &Document,
    dlg: &StructureAutoDialog,
    theme: &AppTheme,
) {
    let desired = EguiVec2::new(ui.available_width().max(240.0), 310.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);

    painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 22, 29));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, theme.border_color),
        egui::StrokeKind::Inside,
    );

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut expand = |p: Vec2| {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    };

    for g in doc.grids.values() {
        expand(g.start);
        expand(g.end);
    }
    for el in doc.elements.values() {
        match el {
            Element::Column(c) => expand(c.location),
            Element::Beam(b) => {
                expand(b.start);
                expand(b.end);
            }
            Element::Footing(f) => expand(f.location),
            _ => {}
        }
    }

    if !min_x.is_finite() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Create Grids or Columns to Preview Structural Automation",
            FontId::proportional(11.0),
            Color32::from_rgb(150, 165, 185),
        );
        return;
    }

    let span_x = (max_x - min_x).max(2000.0);
    let span_y = (max_y - min_y).max(2000.0);
    let pad = 26.0_f32;
    let scale = ((rect.width() - 2.0 * pad) / span_x as f32)
        .min((rect.height() - 2.0 * pad) / span_y as f32)
        .max(0.0001);
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let to_sp = |p: Vec2| -> Pos2 {
        Pos2::new(
            rect.center().x + ((p.x - cx) as f32) * scale,
            rect.center().y - ((p.y - cy) as f32) * scale,
        )
    };

    // Draw existing grids dimly
    for g in doc.grids.values() {
        painter.line_segment(
            [to_sp(g.start), to_sp(g.end)],
            Stroke::new(0.9, Color32::from_rgba_unmultiplied(90, 145, 200, 90)),
        );
    }

    // Draw existing beams & columns in muted steel-blue
    for el in doc.elements.values() {
        match el {
            Element::Beam(b) => {
                painter.line_segment(
                    [to_sp(b.start), to_sp(b.end)],
                    Stroke::new(1.8, Color32::from_rgb(95, 150, 210)),
                );
            }
            Element::Column(c) => {
                let sp = to_sp(c.location);
                painter.rect_filled(
                    egui::Rect::from_center_size(sp, EguiVec2::splat(6.0)),
                    1.0,
                    Color32::from_rgb(130, 195, 250),
                );
            }
            _ => {}
        }
    }

    // Overlay orange candidate preview for the active automation mode
    let accent_orange = Color32::from_rgb(255, 130, 15);
    match dlg.mode {
        AutoMode::ColumnsOnGrids => {
            for (pt, _) in compute_grid_intersections(doc) {
                let sp = to_sp(pt);
                painter.rect_stroke(
                    egui::Rect::from_center_size(sp, EguiVec2::splat(9.0)),
                    1.0,
                    Stroke::new(1.8, accent_orange),
                    egui::StrokeKind::Inside,
                );
            }
        }
        AutoMode::FootingsOnColumns => {
            let filter = if dlg.footing_filter_active_level_only {
                Some(doc.active_level_id)
            } else {
                None
            };
            for (_, loc, _, _, _, _, _, _) in find_foundation_column_targets(doc, filter) {
                let sp = to_sp(loc);
                painter.rect_stroke(
                    egui::Rect::from_center_size(sp, EguiVec2::splat(13.0)),
                    1.5,
                    Stroke::new(1.8, accent_orange),
                    egui::StrokeKind::Inside,
                );
            }
        }
        AutoMode::BeamsOnGridsOrColumns => {
            let f_lvl = dlg.beam_framing_level.unwrap_or(doc.active_level_id);
            for (p1, p2) in preview_beam_spans(doc, f_lvl, dlg.connect_bare_grids, 24_000.0) {
                painter.line_segment([to_sp(p1), to_sp(p2)], Stroke::new(2.4, accent_orange));
            }
        }
        AutoMode::SlabsOnClosedBays => {
            let s_lvl = dlg.slab_framing_level.unwrap_or(doc.active_level_id);
            for bay in find_closed_structural_bays(doc, s_lvl, dlg.require_corner_columns) {
                let inner_bay = align_bay_to_beam_inner_surfaces(doc, &bay, s_lvl);
                if polygon::is_convex(&inner_bay) {
                    let pts: Vec<Pos2> = inner_bay.iter().map(|&p| to_sp(p)).collect();
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        Color32::from_rgba_unmultiplied(255, 135, 20, 65),
                        Stroke::new(1.8, accent_orange),
                    ));
                } else {
                    for pair in inner_bay.windows(2) {
                        painter.line_segment(
                            [to_sp(pair[0]), to_sp(pair[1])],
                            Stroke::new(2.0, accent_orange),
                        );
                    }
                    if let (Some(&first), Some(&last)) = (inner_bay.first(), inner_bay.last()) {
                        painter.line_segment(
                            [to_sp(last), to_sp(first)],
                            Stroke::new(2.0, accent_orange),
                        );
                    }
                }
            }
        }
    }
}
