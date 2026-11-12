//! Core architectural, structural, and annotation BIM elements.

use std::collections::HashMap;
use crate::id::{ElementId, LevelId, MaterialId, ViewId};
use plinth_geom::{vec2, Face, Prism, Vec2, Vec3, Xform2, EPS};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Compound Assembly Layering (Walls, Floors, Roofs, Ceilings)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerFunction {
    ExteriorFinish,
    ThermalInsulation,
    AirGap,
    MembraneBarrier,
    #[default]
    StructuralCore,
    Substrate,
    InteriorFinish,
}

impl LayerFunction {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExteriorFinish => "Finish 1 [Exterior]",
            Self::ThermalInsulation => "Thermal / Insulation",
            Self::AirGap => "Air / Ventilation Gap",
            Self::MembraneBarrier => "Membrane / Vapor Barrier",
            Self::StructuralCore => "Structure [Core]",
            Self::Substrate => "Substrate / Decking",
            Self::InteriorFinish => "Finish 2 [Interior]",
        }
    }

    pub fn preview_rgb(self) -> [u8; 3] {
        match self {
            Self::ExteriorFinish => [198, 108, 78],
            Self::ThermalInsulation => [238, 198, 72],
            Self::AirGap => [145, 198, 232],
            Self::MembraneBarrier => [90, 140, 215],
            Self::StructuralCore => [155, 162, 172],
            Self::Substrate => [182, 148, 108],
            Self::InteriorFinish => [232, 234, 238],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssemblyLayer {
    pub name: String,
    pub function: LayerFunction,
    pub material_name: String,
    pub thickness: f64,            // mm (0.0 allowed for thin membranes)
    pub thermal_conductivity: f64, // W/(m·K)
}

impl AssemblyLayer {
    pub fn new(
        name: impl Into<String>,
        function: LayerFunction,
        material_name: impl Into<String>,
        thickness: f64,
        thermal_conductivity: f64,
    ) -> Self {
        Self {
            name: name.into(),
            function,
            material_name: material_name.into(),
            thickness,
            thermal_conductivity,
        }
    }

    /// Thermal resistance R = d(m) / lambda(W/mK) in m²·K/W
    pub fn thermal_resistance(&self) -> f64 {
        if self.thickness <= 0.0 || self.thermal_conductivity <= 1e-4 {
            0.02
        } else {
            (self.thickness / 1000.0) / self.thermal_conductivity
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StairStyle {
    #[default]
    Monolithic,
    SolidStepped,
    OpenRiser,
}

impl StairStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Monolithic => "Monolithic Cast-in-Place (Sloped Waist)",
            Self::SolidStepped => "Solid Stepped Mass",
            Self::OpenRiser => "Open Riser (Twin Stringer)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RampStyle {
    #[default]
    SlopedSlab,
    SolidWedge,
}

impl RampStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::SlopedSlab => "Inclined Structural Slab",
            Self::SolidWedge => "Solid Monolithic Wedge",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootingStyle {
    #[default]
    Pad,
    Stepped,
    Sloped,
}

impl FootingStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pad => "Isolated Pad Footing",
            Self::Stepped => "Two-Tier Stepped Footing",
            Self::Sloped => "Sloped Bell Footing",
        }
    }
}

fn default_stair_waist() -> f64 {
    180.0
}

fn default_rail_count() -> usize {
    1
}

// ---------------------------------------------------------------------------
// Wall
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub id: ElementId,
    pub name: String,
    pub start: Vec2,
    pub end: Vec2,
    pub thickness: f64,
    pub base_level: LevelId,
    pub base_offset: f64,
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub unconnected_height: f64,
    pub is_structural: bool,
    pub wall_type_name: String,
    pub material_id: Option<MaterialId>,
    #[serde(default)]
    pub assembly_layers: Vec<AssemblyLayer>,
    // IFC4 Property Sets (Pset_WallCommon) & Identification:
    pub tag: String,
    pub description: String,
    pub is_external: bool,
    pub combustible: bool,
    pub compartmentation: bool,
    pub fire_rating: String,
    pub surface_spread_of_flame: String,
    pub acoustic_rating: String,
    pub thermal_transmittance: f64,
}

impl Wall {
    pub fn new(start: Vec2, end: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Basic Wall".into(),
            start,
            end,
            thickness: 200.0,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            unconnected_height: 3000.0,
            is_structural: false,
            wall_type_name: "Generic - 200mm".into(),
            material_id: None,
            assembly_layers: Vec::new(),
            tag: "W-01".into(),
            description: "Standard Exterior Enclosure Wall".into(),
            is_external: true,
            combustible: false,
            compartmentation: true,
            fire_rating: "2 HR".into(),
            surface_spread_of_flame: "Class 1".into(),
            acoustic_rating: "STC 50".into(),
            thermal_transmittance: 0.28,
        }
    }

    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn direction(&self) -> Vec2 {
        (self.end - self.start).normalized()
    }

    pub fn normal(&self) -> Vec2 {
        self.direction().perp()
    }

    /// 2D rectangular footprint in plan.
    pub fn footprint(&self) -> Vec<Vec2> {
        let n = self.normal() * (self.thickness * 0.5);
        vec![
            self.start - n,
            self.end - n,
            self.end + n,
            self.start + n,
        ]
    }

    /// Computes a clean mitered/trimmed 2D footprint accounting for L-corners, T-joints, and intersections.
    /// Returns: (footprint_polygon, start_is_joined, end_is_joined)
    pub fn mitered_footprint(&self, all_walls: &[&Wall]) -> (Vec<Vec2>, bool, bool) {
        let dir = self.direction();
        let norm = self.normal();
        let half_t = self.thickness * 0.5;

        // Default untrimmed corners
        let mut p_start_left = self.start - norm * half_t;
        let mut p_start_right = self.start + norm * half_t;
        let mut p_end_left = self.end - norm * half_t;
        let mut p_end_right = self.end + norm * half_t;

        let mut start_joined = false;
        let mut end_joined = false;

        for other in all_walls {
            if other.id == self.id || other.base_level != self.base_level {
                continue;
            }

            let other_dir = other.direction();
            let other_norm = other.normal();
            let other_half_t = other.thickness * 0.5;

            // 1. Check L-Corner at self.start
            let start_meets_other_end = self.start.distance(other.end) < (self.thickness + other.thickness) * 0.6;
            let start_meets_other_start = self.start.distance(other.start) < (self.thickness + other.thickness) * 0.6;

            if !start_joined && (start_meets_other_end || start_meets_other_start) {
                // When other.end meets self.start, other flows continuously into self,
                // so other's -norm/+norm sides match self's -norm/+norm sides.
                // When other.start meets self.start, both diverge from the corner, so sides are opposite.
                let (o_start, o_dir, o_norm) = if start_meets_other_end {
                    (other.end, other_dir, other_norm)
                } else {
                    (other.start, -other_dir, -other_norm)
                };

                let cross = dir.x * o_dir.y - dir.y * o_dir.x;
                if cross.abs() > 0.15 {
                    if let Some(pt_left) = line_line_intersection(
                        self.start - norm * half_t, dir,
                        o_start - o_norm * other_half_t, o_dir,
                    ) {
                        if pt_left.distance(self.start) < (self.thickness + other.thickness) * 1.5 {
                            p_start_left = pt_left;
                            start_joined = true;
                        }
                    }
                    if let Some(pt_right) = line_line_intersection(
                        self.start + norm * half_t, dir,
                        o_start + o_norm * other_half_t, o_dir,
                    ) {
                        if pt_right.distance(self.start) < (self.thickness + other.thickness) * 1.5 {
                            p_start_right = pt_right;
                            start_joined = true;
                        }
                    }
                }
            } else if !start_joined {
                // T-Joint at self.start against other's body
                let dist_to_other = point_to_segment_dist(self.start, other.start, other.end);
                let cross = dir.x * other_dir.y - dir.y * other_dir.x;
                if dist_to_other < (other_half_t + half_t) * 0.85 && cross.abs() > 0.15 {
                    let sign = if dir.dot(other_norm) >= 0.0 { 1.0 } else { -1.0 };
                    let trim_face_pt = other.start + other_norm * (sign * other_half_t);
                    if let (Some(pt_l), Some(pt_r)) = (
                        line_line_intersection(self.start - norm * half_t, dir, trim_face_pt, other_dir),
                        line_line_intersection(self.start + norm * half_t, dir, trim_face_pt, other_dir),
                    ) {
                        if pt_l.distance(self.start) < (self.thickness + other.thickness) * 1.5
                            && pt_r.distance(self.start) < (self.thickness + other.thickness) * 1.5
                        {
                            p_start_left = pt_l;
                            p_start_right = pt_r;
                            start_joined = true;
                        }
                    }
                }
            }

            // 2. Check L-Corner at self.end
            let end_meets_other_start = self.end.distance(other.start) < (self.thickness + other.thickness) * 0.6;
            let end_meets_other_end = self.end.distance(other.end) < (self.thickness + other.thickness) * 0.6;

            if !end_joined && (end_meets_other_start || end_meets_other_end) {
                let (o_end, o_dir, o_norm) = if end_meets_other_start {
                    (other.start, other_dir, other_norm)
                } else {
                    (other.end, -other_dir, -other_norm)
                };

                let cross = dir.x * o_dir.y - dir.y * o_dir.x;
                if cross.abs() > 0.15 {
                    if let Some(pt_left) = line_line_intersection(
                        self.end - norm * half_t, dir,
                        o_end - o_norm * other_half_t, o_dir,
                    ) {
                        if pt_left.distance(self.end) < (self.thickness + other.thickness) * 1.5 {
                            p_end_left = pt_left;
                            end_joined = true;
                        }
                    }
                    if let Some(pt_right) = line_line_intersection(
                        self.end + norm * half_t, dir,
                        o_end + o_norm * other_half_t, o_dir,
                    ) {
                        if pt_right.distance(self.end) < (self.thickness + other.thickness) * 1.5 {
                            p_end_right = pt_right;
                            end_joined = true;
                        }
                    }
                }
            } else if !end_joined {
                // T-Joint at self.end against other's body
                let dist_to_other = point_to_segment_dist(self.end, other.start, other.end);
                let cross = dir.x * other_dir.y - dir.y * other_dir.x;
                if dist_to_other < (other_half_t + half_t) * 0.85 && cross.abs() > 0.15 {
                    let sign = if (-dir).dot(other_norm) >= 0.0 { 1.0 } else { -1.0 };
                    let trim_face_pt = other.start + other_norm * (sign * other_half_t);
                    if let (Some(pt_l), Some(pt_r)) = (
                        line_line_intersection(self.end - norm * half_t, dir, trim_face_pt, other_dir),
                        line_line_intersection(self.end + norm * half_t, dir, trim_face_pt, other_dir),
                    ) {
                        if pt_l.distance(self.end) < (self.thickness + other.thickness) * 1.5
                            && pt_r.distance(self.end) < (self.thickness + other.thickness) * 1.5
                        {
                            p_end_left = pt_l;
                            p_end_right = pt_r;
                            end_joined = true;
                        }
                    }
                }
            }
        }

        let poly = vec![p_start_left, p_end_left, p_end_right, p_start_right];
        (poly, start_joined, end_joined)
    }

    /// Detects cross-joint (X-intersection) intervals along this wall where a higher-priority wall crosses through.
    pub fn cross_joint_spans(&self, all_walls: &[&Wall]) -> Vec<(f64, f64)> {
        let dir = self.direction();
        let wall_len = self.length();
        let half_t = self.thickness * 0.5;
        let mut spans = Vec::new();

        for other in all_walls {
            // Deterministic tie-breaker: only the wall with larger ID trims around the crossing wall
            if other.id == self.id || other.base_level != self.base_level || self.id.0 < other.id.0 {
                continue;
            }
            let other_dir = other.direction();
            let other_len = other.length();
            let other_half_t = other.thickness * 0.5;
            let cross = (dir.x * other_dir.y - dir.y * other_dir.x).abs();
            if cross <= 0.2 {
                continue;
            }
            if let Some(pt) = line_line_intersection(self.start, dir, other.start, other_dir) {
                let t_self = (pt - self.start).dot(dir);
                let t_other = (pt - other.start).dot(other_dir);
                if t_self > half_t + other_half_t
                    && t_self < wall_len - (half_t + other_half_t)
                    && t_other > half_t + other_half_t
                    && t_other < other_len - (half_t + other_half_t)
                {
                    let half_span = other_half_t / cross;
                    spans.push(((t_self - half_span).max(0.0), (t_self + half_span).min(wall_len)));
                }
            }
        }
        spans.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        spans
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.unconnected_height + self.top_offset,
        }
        .max(z0 + 10.0);
        (z0.min(z1), z0.max(z1))
    }

    /// IFC4 Physical Quantities (Qto_WallBaseQuantities)
    pub fn gross_footprint_area(&self) -> f64 {
        self.length() * self.thickness
    }

    pub fn gross_volume(&self, base_z: f64, top_z: Option<f64>) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        self.length() * self.thickness * (z1 - z0)
    }

    pub fn net_volume(&self, base_z: f64, top_z: Option<f64>, openings: &[WallOpening]) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let mut vol = self.gross_volume(base_z, top_z);
        for op in openings {
            let op_z_bot = (z0 + op.sill_height).clamp(z0, z1);
            let op_z_top = (op_z_bot + op.height).clamp(z0, z1);
            let h = (op_z_top - op_z_bot).max(0.0);
            vol -= op.width * self.thickness * h;
        }
        vol.max(0.0)
    }

    pub fn gross_side_area(&self, base_z: f64, top_z: Option<f64>) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        self.length() * (z1 - z0)
    }

    pub fn net_side_area(&self, base_z: f64, top_z: Option<f64>, openings: &[WallOpening]) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let mut area = self.gross_side_area(base_z, top_z);
        for op in openings {
            let op_z_bot = (z0 + op.sill_height).clamp(z0, z1);
            let op_z_top = (op_z_bot + op.height).clamp(z0, z1);
            let h = (op_z_top - op_z_bot).max(0.0);
            area -= op.width * h;
        }
        area.max(0.0)
    }

    /// Solid geometry for the wall, taking into account openings from hosted doors and windows.
    pub fn build_solids(
        &self,
        base_z: f64,
        top_z: Option<f64>,
        openings: &[WallOpening],
    ) -> Vec<Prism> {
        self.build_solids_with_joins(base_z, top_z, openings, &[])
    }

    /// Solid geometry for the wall with seamless corner miter joints.
    pub fn build_solids_with_joins(
        &self,
        base_z: f64,
        top_z: Option<f64>,
        openings: &[WallOpening],
        all_walls: &[&Wall],
    ) -> Vec<Prism> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let wall_len = self.length();
        if wall_len < EPS || z1 - z0 < EPS {
            return Vec::new();
        }

        let dir = self.direction();
        let norm = self.normal();
        let half_t = self.thickness * 0.5;
        let (poly, _start_joined, _end_joined) = self.mitered_footprint(all_walls);

        // Combine hosted openings with any full-height cross-joint gaps
        let mut sorted = openings.to_vec();
        for (cs, ce) in self.cross_joint_spans(all_walls) {
            sorted.push(WallOpening {
                offset_along: (cs + ce) * 0.5,
                width: ce - cs,
                height: z1 - z0,
                sill_height: 0.0,
            });
        }

        // If no openings or cross-joints, return single watertight solid prism with mitered footprint
        if sorted.is_empty() {
            return vec![Prism::with_soft(poly, z0, z1, vec![false; 4])];
        }

        // Sort openings by distance along wall
        sorted.sort_by(|a, b| a.offset_along.partial_cmp(&b.offset_along).unwrap_or(std::cmp::Ordering::Equal));

        let mut prisms = Vec::new();
        let mut curr_dist = 0.0;

        for op in &sorted {
            let op_start = (op.offset_along - op.width * 0.5).max(0.0);
            let op_end = (op.offset_along + op.width * 0.5).min(wall_len);

            if op_start > curr_dist + 1.0 {
                // Segment before opening
                let p_cut_l = self.start + dir * op_start - norm * half_t;
                let p_cut_r = self.start + dir * op_start + norm * half_t;
                let seg_poly = if curr_dist < 1.0 {
                    vec![poly[0], p_cut_l, p_cut_r, poly[3]]
                } else {
                    let p_prev_l = self.start + dir * curr_dist - norm * half_t;
                    let p_prev_r = self.start + dir * curr_dist + norm * half_t;
                    vec![p_prev_l, p_cut_l, p_cut_r, p_prev_r]
                };
                prisms.push(Prism::with_soft(seg_poly, z0, z1, vec![false; 4]));
            }

            // Opening gap: below opening (sill) and above opening (lintel/header)
            let op_z_bottom = z0 + op.sill_height;
            let op_z_top = op_z_bottom + op.height;
            let p_op0_l = self.start + dir * op_start - norm * half_t;
            let p_op0_r = self.start + dir * op_start + norm * half_t;
            let p_op1_l = self.start + dir * op_end - norm * half_t;
            let p_op1_r = self.start + dir * op_end + norm * half_t;
            let op_poly = vec![p_op0_l, p_op1_l, p_op1_r, p_op0_r];

            // Sill portion below opening
            if op_z_bottom > z0 + 1.0 {
                prisms.push(Prism::with_soft(op_poly.clone(), z0, op_z_bottom, vec![false; 4]));
            }
            // Lintel portion above opening
            if z1 > op_z_top + 1.0 {
                prisms.push(Prism::with_soft(op_poly, op_z_top, z1, vec![false; 4]));
            }

            curr_dist = op_end;
        }

        // Remaining wall segment after last opening
        if wall_len > curr_dist + 1.0 {
            let p_prev_l = self.start + dir * curr_dist - norm * half_t;
            let p_prev_r = self.start + dir * curr_dist + norm * half_t;
            let seg_poly = vec![p_prev_l, poly[1], poly[2], p_prev_r];
            prisms.push(Prism::with_soft(seg_poly, z0, z1, vec![false; 4]));
        }

        prisms
    }

    /// Generates exact boundary B-Rep faces for the wall with zero internal cross-section faces
    /// and zero coplanar vertical seam lines above/below doors and windows.
    pub fn boundary_faces(
        &self,
        base_z: f64,
        top_z: Option<f64>,
        openings: &[WallOpening],
        all_walls: &[&Wall],
        render_top_cap: bool,
    ) -> Vec<Face> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let wall_len = self.length();
        if wall_len < EPS || z1 - z0 < EPS {
            return Vec::new();
        }

        let dir = self.direction();
        let norm = self.normal();
        let half_t = self.thickness * 0.5;
        let (poly, start_joined, end_joined) = self.mitered_footprint(all_walls);
        let p_start_left = poly[0];
        let p_end_left = poly[1];
        let p_end_right = poly[2];
        let p_start_right = poly[3];

        let pt_left = |d: f64| -> Vec2 {
            if d <= 0.5 {
                p_start_left
            } else if d >= wall_len - 0.5 {
                p_end_left
            } else {
                self.start + dir * d - norm * half_t
            }
        };
        let pt_right = |d: f64| -> Vec2 {
            if d <= 0.5 {
                p_start_right
            } else if d >= wall_len - 0.5 {
                p_end_right
            } else {
                self.start + dir * d + norm * half_t
            }
        };

        let mut sorted = openings.to_vec();
        for (cs, ce) in self.cross_joint_spans(all_walls) {
            sorted.push(WallOpening {
                offset_along: (cs + ce) * 0.5,
                width: ce - cs,
                height: z1 - z0,
                sill_height: 0.0,
            });
        }
        sorted.sort_by(|a, b| a.offset_along.partial_cmp(&b.offset_along).unwrap_or(std::cmp::Ordering::Equal));

        let mut faces = Vec::new();
        let mut curr_dist = 0.0;

        for op in &sorted {
            let op_start = (op.offset_along - op.width * 0.5).max(0.0);
            let op_end = (op.offset_along + op.width * 0.5).min(wall_len);

            if op_start > curr_dist + 1.0 {
                let is_start = curr_dist <= 0.5;
                let is_end = op_start >= wall_len - 0.5;
                // Left (-norm) pier face
                faces.push(Face::new(
                    vec![
                        pt_left(curr_dist).extend(z0),
                        pt_left(op_start).extend(z0),
                        pt_left(op_start).extend(z1),
                        pt_left(curr_dist).extend(z1),
                    ],
                    vec![true, is_end, true, is_start],
                ));
                // Right (+norm) pier face
                faces.push(Face::new(
                    vec![
                        pt_right(op_start).extend(z0),
                        pt_right(curr_dist).extend(z0),
                        pt_right(curr_dist).extend(z1),
                        pt_right(op_start).extend(z1),
                    ],
                    vec![true, is_start, true, is_end],
                ));
            }

            let op_z_bot = (z0 + op.sill_height).clamp(z0, z1);
            let op_z_top = (op_z_bot + op.height).clamp(z0, z1);
            let is_start = op_start <= 0.5;
            let is_end = op_end >= wall_len - 0.5;

            // Sill faces
            if op_z_bot > z0 + 1.0 {
                faces.push(Face::new(
                    vec![
                        pt_left(op_start).extend(z0),
                        pt_left(op_end).extend(z0),
                        pt_left(op_end).extend(op_z_bot),
                        pt_left(op_start).extend(op_z_bot),
                    ],
                    vec![true, is_end, true, is_start],
                ));
                faces.push(Face::new(
                    vec![
                        pt_right(op_end).extend(z0),
                        pt_right(op_start).extend(z0),
                        pt_right(op_start).extend(op_z_bot),
                        pt_right(op_end).extend(op_z_bot),
                    ],
                    vec![true, is_start, true, is_end],
                ));
                // Sill Top Reveal
                faces.push(Face::solid(vec![
                    pt_left(op_start).extend(op_z_bot),
                    pt_left(op_end).extend(op_z_bot),
                    pt_right(op_end).extend(op_z_bot),
                    pt_right(op_start).extend(op_z_bot),
                ]));
            }

            // Lintel faces
            if z1 > op_z_top + 1.0 {
                faces.push(Face::new(
                    vec![
                        pt_left(op_start).extend(op_z_top),
                        pt_left(op_end).extend(op_z_top),
                        pt_left(op_end).extend(z1),
                        pt_left(op_start).extend(z1),
                    ],
                    vec![true, is_end, true, is_start],
                ));
                faces.push(Face::new(
                    vec![
                        pt_right(op_end).extend(op_z_top),
                        pt_right(op_start).extend(op_z_top),
                        pt_right(op_start).extend(z1),
                        pt_right(op_end).extend(z1),
                    ],
                    vec![true, is_start, true, is_end],
                ));
                // Lintel Bottom Soffit Reveal
                faces.push(Face::solid(vec![
                    pt_right(op_start).extend(op_z_top),
                    pt_right(op_end).extend(op_z_top),
                    pt_left(op_end).extend(op_z_top),
                    pt_left(op_start).extend(op_z_top),
                ]));
            }

            // Opening Jamb Reveals (only for partial-height openings, not full-height cross-joints)
            if op_z_top - op_z_bot < (z1 - z0) - 1.0 {
                faces.push(Face::solid(vec![
                    pt_left(op_start).extend(op_z_bot),
                    pt_right(op_start).extend(op_z_bot),
                    pt_right(op_start).extend(op_z_top),
                    pt_left(op_start).extend(op_z_top),
                ]));
                faces.push(Face::solid(vec![
                    pt_right(op_end).extend(op_z_bot),
                    pt_left(op_end).extend(op_z_bot),
                    pt_left(op_end).extend(op_z_top),
                    pt_right(op_end).extend(op_z_top),
                ]));
            }

            curr_dist = op_end;
        }

        // Remaining pier after last opening
        if wall_len > curr_dist + 1.0 {
            let is_start = curr_dist <= 0.5;
            faces.push(Face::new(
                vec![
                    pt_left(curr_dist).extend(z0),
                    pt_left(wall_len).extend(z0),
                    pt_left(wall_len).extend(z1),
                    pt_left(curr_dist).extend(z1),
                ],
                vec![true, true, true, is_start],
            ));
            faces.push(Face::new(
                vec![
                    pt_right(wall_len).extend(z0),
                    pt_right(curr_dist).extend(z0),
                    pt_right(curr_dist).extend(z1),
                    pt_right(wall_len).extend(z1),
                ],
                vec![true, is_start, true, true],
            ));
        }

        // Unjoined Start and End Caps
        if !start_joined {
            faces.push(Face::solid(vec![
                p_start_right.extend(z0),
                p_start_left.extend(z0),
                p_start_left.extend(z1),
                p_start_right.extend(z1),
            ]));
        }
        if !end_joined {
            faces.push(Face::solid(vec![
                p_end_left.extend(z0),
                p_end_right.extend(z0),
                p_end_right.extend(z1),
                p_end_left.extend(z1),
            ]));
        }

        // Top Cap
        if render_top_cap {
            faces.push(Face::new(
                vec![
                    p_start_left.extend(z1),
                    p_end_left.extend(z1),
                    p_end_right.extend(z1),
                    p_start_right.extend(z1),
                ],
                vec![true, !end_joined, true, !start_joined],
            ));
        }

        faces
    }
}

fn line_line_intersection(p1: Vec2, d1: Vec2, p2: Vec2, d2: Vec2) -> Option<Vec2> {
    let cross = d1.x * d2.y - d1.y * d2.x;
    if cross.abs() < 1e-4 {
        return None;
    }
    let dp = p2 - p1;
    let t = (dp.x * d2.y - dp.y * d2.x) / cross;
    Some(p1 + d1 * t)
}

fn point_to_segment_dist(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[derive(Clone, Copy, Debug)]
pub struct WallOpening {
    pub offset_along: f64,
    pub width: f64,
    pub height: f64,
    pub sill_height: f64,
}

// ---------------------------------------------------------------------------
// Column
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: ElementId,
    pub name: String,
    pub location: Vec2,
    pub rotation: f64, // radians
    pub width: f64,    // mm (X in local coords)
    pub depth: f64,    // mm (Y in local coords)
    pub is_round: bool,
    pub base_level: LevelId,
    pub base_offset: f64,
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub unconnected_height: f64,
    pub is_structural: bool,
    pub column_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_ColumnCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub load_bearing_capacity: f64, // kN
}

impl Column {
    pub fn new(location: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Rectangular Column".into(),
            location,
            rotation: 0.0,
            width: 400.0,
            depth: 400.0,
            is_round: false,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            unconnected_height: 3000.0,
            is_structural: true,
            column_type_name: "400 x 400mm".into(),
            material_id: None,
            tag: "C-01".into(),
            is_external: false,
            fire_rating: "2 HR".into(),
            load_bearing_capacity: 450.0,
        }
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        if self.is_round {
            let r = self.width * 0.5;
            let segs = 16;
            (0..segs)
                .map(|i| {
                    let a = std::f64::consts::TAU * (i as f64) / (segs as f64);
                    self.location + vec2(r * a.cos(), r * a.sin())
                })
                .collect()
        } else {
            let ux = Vec2::from_angle(self.rotation);
            let uy = ux.perp();
            let (hx, hy) = (self.width * 0.5, self.depth * 0.5);
            vec![
                self.location - ux * hx - uy * hy,
                self.location + ux * hx - uy * hy,
                self.location + ux * hx + uy * hy,
                self.location - ux * hx + uy * hy,
            ]
        }
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.unconnected_height + self.top_offset,
        }
        .max(z0 + 10.0);
        (z0.min(z1), z0.max(z1))
    }

    pub fn solid(&self, base_z: f64, top_z: Option<f64>) -> Prism {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        Prism::new(self.footprint(), z0, z1)
    }
}

// ---------------------------------------------------------------------------
// Footing (Structural Foundation — Pad / Stepped / Sloped)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Footing {
    pub id: ElementId,
    pub name: String,
    pub location: Vec2,
    pub width: f64,     // mm (local X)
    pub length: f64,    // mm (local Y)
    pub thickness: f64, // mm (total foundation depth along Z)
    pub rotation: f64,  // radians
    #[serde(default)]
    pub footing_style: FootingStyle,
    #[serde(default)]
    pub host_column: Option<ElementId>,
    pub base_level: LevelId,
    pub base_offset: f64, // top of footing offset relative to base_level
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub footing_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_FootingCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub soil_bearing_capacity: f64, // kPa
    pub concrete_grade: String,
    pub load_bearing_capacity: f64, // kN
}

impl Footing {
    pub fn new(location: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Isolated Pad Footing".into(),
            location,
            width: 1400.0,
            length: 1400.0,
            thickness: 450.0,
            rotation: 0.0,
            footing_style: FootingStyle::Pad,
            host_column: None,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            footing_type_name: "RC Pad Footing 1400x1400x450".into(),
            material_id: None,
            tag: "FT-01".into(),
            is_external: true,
            soil_bearing_capacity: 200.0,
            concrete_grade: "C30/37".into(),
            load_bearing_capacity: 900.0,
        }
    }

    pub fn rect_corners(&self, w: f64, l: f64) -> [Vec2; 4] {
        let ux = Vec2::from_angle(self.rotation);
        let uy = ux.perp();
        let (hx, hy) = (w * 0.5, l * 0.5);
        [
            self.location - ux * hx - uy * hy,
            self.location + ux * hx - uy * hy,
            self.location + ux * hx + uy * hy,
            self.location - ux * hx + uy * hy,
        ]
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        self.rect_corners(self.width.max(100.0), self.length.max(100.0))
            .to_vec()
    }

    pub fn gross_area(&self) -> f64 {
        self.width.max(100.0) * self.length.max(100.0)
    }

    pub fn gross_volume(&self) -> f64 {
        let a = self.gross_area();
        let h = self.thickness.max(50.0);
        match self.footing_style {
            FootingStyle::Pad => a * h,
            FootingStyle::Stepped => {
                let h1 = h * 0.55;
                let h2 = h - h1;
                a * h1 + (a * 0.36) * h2
            }
            FootingStyle::Sloped => {
                let h1 = h * 0.45;
                let h2 = h - h1;
                let a2 = a * (0.55 * 0.55);
                a * h1 + (h2 / 3.0) * (a + a2 + (a * a2).sqrt())
            }
        }
    }

    /// Resolves `(z_bot, z_top)` in world mm.
    /// By default (`top_level = None`), the footing top sits at `base_z + base_offset + top_offset`
    /// and extends downward by `thickness` so it supports columns resting on `base_z + base_offset`.
    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let base_ref = base_z + self.base_offset;
        match top_z {
            Some(tz) => {
                let z1 = tz + self.top_offset;
                if z1 > base_ref + 50.0 {
                    (base_ref, z1)
                } else {
                    (z1 - self.thickness.max(50.0), z1)
                }
            }
            None => {
                let z1 = base_ref + self.top_offset;
                let z0 = z1 - self.thickness.max(50.0);
                (z0, z1)
            }
        }
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let h = (z1 - z0).max(50.0);
        match self.footing_style {
            FootingStyle::Pad | FootingStyle::Sloped => {
                vec![Prism::new(self.footprint(), z0, z1)]
            }
            FootingStyle::Stepped => {
                let z_mid = z0 + h * 0.55;
                let upper = self
                    .rect_corners((self.width * 0.60).max(80.0), (self.length * 0.60).max(80.0))
                    .to_vec();
                vec![
                    Prism::new(self.footprint(), z0, z_mid),
                    Prism::new(upper, z_mid, z1),
                ]
            }
        }
    }

    /// Generates strictly convex, outward-oriented watertight 3D faces for `Pad`, `Stepped`, and `Sloped` footings.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let h = (z1 - z0).max(50.0);
        let z1 = z0 + h;

        match self.footing_style {
            FootingStyle::Pad => Prism::new(self.footprint(), z0, z1).faces(),
            FootingStyle::Stepped => {
                let z_mid = z0 + h * 0.55;
                let mut out = Prism::new(self.footprint(), z0, z_mid).faces();
                let upper = self
                    .rect_corners((self.width * 0.60).max(80.0), (self.length * 0.60).max(80.0))
                    .to_vec();
                out.extend(Prism::new(upper, z_mid, z1).faces());
                out
            }
            FootingStyle::Sloped => {
                let z_mid = z0 + h * 0.45;
                let outer = self.rect_corners(self.width.max(100.0), self.length.max(100.0));
                let inner = self.rect_corners(
                    (self.width * 0.55).max(80.0),
                    (self.length * 0.55).max(80.0),
                );
                let mut faces = Vec::with_capacity(10);

                // Bottom cap (normal -Z)
                faces.push(Face::oriented(
                    vec![
                        outer[3].extend(z0),
                        outer[2].extend(z0),
                        outer[1].extend(z0),
                        outer[0].extend(z0),
                    ],
                    vec![true; 4],
                    Vec3::new(0.0, 0.0, -1.0),
                ));

                // 4 vertical lower base quads + 4 sloped upper frustum quads
                for i in 0..4 {
                    let j = (i + 1) % 4;
                    let b0 = outer[i].extend(z0);
                    let b1 = outer[j].extend(z0);
                    let m1 = outer[j].extend(z_mid);
                    let m0 = outer[i].extend(z_mid);
                    let edge_dir = (outer[j] - outer[i]).normalized();
                    let out_n2 = Vec2::new(edge_dir.y, -edge_dir.x);
                    faces.push(Face::oriented(
                        vec![b0, b1, m1, m0],
                        vec![true; 4],
                        Vec3::new(out_n2.x, out_n2.y, 0.0),
                    ));

                    let t1 = inner[j].extend(z1);
                    let t0 = inner[i].extend(z1);
                    let slope_n = (m1 - m0).cross(t0 - m0).normalized();
                    faces.push(Face::oriented(vec![m0, m1, t1, t0], vec![true; 4], slope_n));
                }

                // Top pedestal cap (normal +Z)
                faces.push(Face::oriented(
                    vec![
                        inner[0].extend(z1),
                        inner[1].extend(z1),
                        inner[2].extend(z1),
                        inner[3].extend(z1),
                    ],
                    vec![true; 4],
                    Vec3::Z,
                ));

                faces
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Beam (Structural Framing)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beam {
    pub id: ElementId,
    pub name: String,
    pub start: Vec2,
    pub end: Vec2,
    pub width: f64, // mm
    pub depth: f64, // mm
    pub level: LevelId,
    pub z_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub is_structural: bool,
    pub beam_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_BeamCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub load_bearing_capacity: f64, // kN/m
}

impl Beam {
    pub fn new(start: Vec2, end: Vec2, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Structural Beam".into(),
            start,
            end,
            width: 300.0,
            depth: 500.0,
            level,
            z_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            is_structural: true,
            beam_type_name: "Concrete Beam 300x500".into(),
            material_id: None,
            tag: "B-01".into(),
            is_external: false,
            fire_rating: "2 HR".into(),
            load_bearing_capacity: 25.0,
        }
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        let norm = (self.end - self.start).normalized().perp() * (self.width * 0.5);
        vec![
            self.start - norm,
            self.end - norm,
            self.end + norm,
            self.start + norm,
        ]
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let base_ref = base_z + self.z_offset;
        match top_z {
            Some(tz) => {
                let z_top = tz + self.top_offset;
                if z_top > base_ref + 10.0 {
                    (base_ref, z_top)
                } else {
                    let z_bot = z_top - self.depth.max(10.0);
                    (z_bot, z_top)
                }
            }
            None => {
                let z_top = base_ref + self.top_offset;
                let z_bot = z_top - self.depth.max(10.0);
                (z_bot, z_top)
            }
        }
    }

    pub fn solid(&self, level_z: f64) -> Prism {
        self.solid_with_bounds(level_z, None)
    }

    pub fn solid_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Prism {
        let (z_bot, z_top) = self.z_bounds(base_z, top_z);
        Prism::new(self.footprint(), z_bot, z_top)
    }
}

// ---------------------------------------------------------------------------
// Slab (Floor / Roof / Foundation)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slab {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub thickness: f64, // mm
    pub level: LevelId,
    pub offset: f64, // mm (base offset relative to level elevation)
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    #[serde(default)]
    pub assembly_layers: Vec<AssemblyLayer>,
    pub is_structural: bool,
    pub slab_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_SlabCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub thermal_transmittance: f64,
    pub acoustic_rating: String,
    pub load_bearing_capacity: f64, // kN/m2
    pub compartmentation: bool,
}

impl Slab {
    pub fn new(boundary: Vec<Vec2>, level: LevelId) -> Self {
        let mut clean_boundary = plinth_geom::polygon::dedup_closed(&boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean_boundary);
        Self {
            id: ElementId::new(),
            name: "Floor Slab".into(),
            boundary: clean_boundary,
            thickness: 200.0,
            level,
            offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            assembly_layers: Vec::new(),
            is_structural: true,
            slab_type_name: "Concrete Cast-in-Place 200mm".into(),
            material_id: None,
            tag: "FL-01".into(),
            is_external: false,
            fire_rating: "2 HR".into(),
            thermal_transmittance: 0.35,
            acoustic_rating: "Ln,w 52 dB".into(),
            load_bearing_capacity: 2.5,
            compartmentation: true,
        }
    }

    pub fn gross_area(&self) -> f64 {
        plinth_geom::polygon::area(&self.boundary).abs()
    }

    pub fn gross_volume(&self) -> f64 {
        self.gross_area() * self.thickness
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let base_ref = base_z + self.offset;
        let z_top = match top_z {
            Some(tz) => tz + self.top_offset,
            None => base_ref + self.top_offset,
        };
        let z_bot = z_top - self.thickness.max(10.0);
        (z_bot, z_top)
    }

    pub fn solid(&self, level_z: f64) -> Prism {
        self.solid_with_bounds(level_z, None)
    }

    pub fn solid_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Prism {
        let (z_bot, z_top) = self.z_bounds(base_z, top_z);
        Prism::new(self.boundary.clone(), z_bot, z_top)
    }

    /// Generates watertight 3D solid faces for the slab. Non-convex boundaries (L-shaped, U-shaped,
    /// notched, stepped) are ear-clipped triangulated on top and bottom caps with soft internal diagonals,
    /// eliminating starburst fan rays and flaps across notches/voids.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let (z_bot, z_top) = self.z_bounds(base_z, top_z);
        let mut clean = plinth_geom::polygon::dedup_closed(&self.boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        let n = clean.len();
        if n < 3 || z_top - z_bot < 1e-4 {
            return Vec::new();
        }

        let mut faces = Vec::new();

        // 1. Vertical perimeter rim faces (strictly outward normals)
        for i in 0..n {
            let a = clean[i];
            let b = clean[(i + 1) % n];
            if a.distance(b) < 1e-4 {
                continue;
            }
            let pts = vec![
                a.extend(z_bot),
                b.extend(z_bot),
                b.extend(z_top),
                a.extend(z_top),
            ];
            faces.push(Face::solid(pts));
        }

        // 2. Top and Bottom caps with ear-clipping triangulation for non-convex shapes
        let is_poly_convex = n == 3 || (n == 4 && plinth_geom::polygon::is_convex(&clean));
        if is_poly_convex {
            faces.push(Face::solid(clean.iter().map(|p| p.extend(z_top)).collect()));
            faces.push(Face::solid(clean.iter().rev().map(|p| p.extend(z_bot)).collect()));
        } else {
            let tris = plinth_geom::polygon::triangulate(&clean);
            let is_perimeter = |i: usize, j: usize| -> bool {
                (i + 1) % n == j || (j + 1) % n == i
            };
            for t in &tris {
                let (ia, ib, ic) = (t[0], t[1], t[2]);
                // Top cap face (+Z)
                let pts_top = vec![
                    clean[ia].extend(z_top),
                    clean[ib].extend(z_top),
                    clean[ic].extend(z_top),
                ];
                let hard_top = vec![
                    is_perimeter(ia, ib),
                    is_perimeter(ib, ic),
                    is_perimeter(ic, ia),
                ];
                faces.push(Face::oriented(pts_top, hard_top, plinth_geom::Vec3::Z));

                // Bottom cap face (-Z)
                let pts_bot = vec![
                    clean[ia].extend(z_bot),
                    clean[ic].extend(z_bot),
                    clean[ib].extend(z_bot),
                ];
                let hard_bot = vec![
                    is_perimeter(ia, ic),
                    is_perimeter(ic, ib),
                    is_perimeter(ib, ia),
                ];
                faces.push(Face::oriented(pts_bot, hard_bot, -plinth_geom::Vec3::Z));
            }
        }

        faces
    }
}

// ---------------------------------------------------------------------------
// Roof (Pitched / Flat / Hipped Envelope)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoofShape {
    Hip,
    Gable,
    Shed,
    Flat,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Roof {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub thickness: f64, // mm
    pub level: LevelId,
    pub offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    #[serde(default)]
    pub assembly_layers: Vec<AssemblyLayer>,
    pub overhang: f64,
    pub slope_degrees: f64,
    pub is_structural: bool,
    pub shape: RoofShape,
    pub roof_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_RoofCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub thermal_transmittance: f64,
    pub acoustic_rating: String,
    pub solar_reflectance_index: f64,
    pub snow_load_capacity: f64, // kN/m2
}

impl Roof {
    pub fn new(boundary: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Basic Roof".into(),
            boundary,
            thickness: 200.0,
            level,
            offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            assembly_layers: Vec::new(),
            overhang: 300.0,
            slope_degrees: 25.0,
            is_structural: true,
            shape: RoofShape::Hip,
            roof_type_name: "Warm Pitched Roof 200mm".into(),
            material_id: None,
            tag: "RF-01".into(),
            is_external: true,
            fire_rating: "1 HR".into(),
            thermal_transmittance: 0.18,
            acoustic_rating: "Rw 42 dB".into(),
            solar_reflectance_index: 78.0,
            snow_load_capacity: 1.2,
        }
    }

    pub fn gross_area(&self) -> f64 {
        let plan_area = plinth_geom::polygon::area(&self.boundary).abs();
        let slope_rad = self.slope_degrees.to_radians();
        if self.shape == RoofShape::Flat || slope_rad.cos().abs() < 1e-4 {
            plan_area
        } else {
            plan_area / slope_rad.cos()
        }
    }

    pub fn gross_volume(&self) -> f64 {
        self.gross_area() * self.thickness
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z_bot = base_z + self.offset;
        let span = if self.boundary.len() >= 3 {
            let xs: Vec<f64> = self.boundary.iter().map(|p| p.x).collect();
            let ys: Vec<f64> = self.boundary.iter().map(|p| p.y).collect();
            let w = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) - xs.iter().copied().fold(f64::INFINITY, f64::min);
            let h = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) - ys.iter().copied().fold(f64::INFINITY, f64::min);
            w.min(h).max(100.0)
        } else {
            1000.0
        };
        let pitch_h = if self.shape != RoofShape::Flat {
            (span * 0.5 * self.slope_degrees.to_radians().tan()).max(0.0)
        } else {
            0.0
        };
        let z_top = match top_z {
            Some(tz) => (tz + self.top_offset + pitch_h).max(z_bot + self.thickness.max(20.0)),
            None => (z_bot + self.thickness + self.top_offset + pitch_h).max(z_bot + 20.0),
        };
        (z_bot, z_top)
    }

    pub fn solid(&self, level_z: f64) -> Prism {
        self.solid_with_bounds(level_z, None)
    }

    pub fn solid_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Prism {
        let z_bot = base_z + self.offset;
        let z_top = match top_z {
            Some(tz) => (tz + self.top_offset).max(z_bot + self.thickness.max(20.0)),
            None => (z_bot + self.thickness + self.top_offset).max(z_bot + 20.0),
        };
        Prism::new(self.boundary.clone(), z_bot, z_top)
    }

    /// Generates exact 3D solid boundary faces for Hip, Gable, Shed, and Flat roofs.
    pub fn faces(&self, level_z: f64) -> Vec<Face> {
        self.faces_with_bounds(level_z, None)
    }

    /// Generates exact 3D solid boundary faces honoring optional `top_level` and `top_offset`.
    pub fn faces_with_bounds(&self, level_z: f64, top_z: Option<f64>) -> Vec<Face> {
        if self.boundary.len() < 3 {
            return Vec::new();
        }
        let min_x = self.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = self.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = self.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = self.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let oh = self.overhang.max(0.0);
        let x0 = min_x - oh;
        let x1 = max_x + oh;
        let y0 = min_y - oh;
        let y1 = max_y + oh;
        let w = (x1 - x0).max(10.0);
        let h = (y1 - y0).max(10.0);

        let z_eave = level_z + self.offset;
        let t = self.thickness.max(20.0);
        let slope_rad = (self.slope_degrees.clamp(0.0, 75.0)).to_radians();
        let default_tan_slope = slope_rad.tan();
        // If top_level is set (and above eave), compute effective slope so ridge reaches top_z + top_offset.
        let peak_target = top_z.map(|tz| tz + self.top_offset);
        let span_run = match self.shape {
            RoofShape::Shed => h,
            RoofShape::Flat => 1.0,
            RoofShape::Gable | RoofShape::Hip => w.min(h) * 0.5,
        }
        .max(10.0);
        let tan_slope = if let Some(pt) = peak_target {
            if pt > z_eave + 10.0 {
                (pt - z_eave) / span_run
            } else {
                default_tan_slope
            }
        } else if self.top_offset.abs() > 1e-4 && self.shape != RoofShape::Flat {
            ((default_tan_slope * span_run + self.top_offset).max(0.0)) / span_run
        } else {
            default_tan_slope
        };

        match self.shape {
            RoofShape::Flat => {
                let z_bot = z_eave;
                let z_top = match peak_target {
                    Some(pt) => pt.max(z_eave + 20.0),
                    None => (z_eave + t + self.top_offset).max(z_eave + 20.0),
                };
                vec![
                    // Top Cap
                    Face::solid(vec![
                        Vec3::new(x0, y0, z_top),
                        Vec3::new(x1, y0, z_top),
                        Vec3::new(x1, y1, z_top),
                        Vec3::new(x0, y1, z_top),
                    ]),
                    // Bottom Cap
                    Face::solid(vec![
                        Vec3::new(x0, y1, z_bot),
                        Vec3::new(x1, y1, z_bot),
                        Vec3::new(x1, y0, z_bot),
                        Vec3::new(x0, y0, z_bot),
                    ]),
                    // South rim
                    Face::solid(vec![
                        Vec3::new(x0, y0, z_bot),
                        Vec3::new(x1, y0, z_bot),
                        Vec3::new(x1, y0, z_top),
                        Vec3::new(x0, y0, z_top),
                    ]),
                    // East rim
                    Face::solid(vec![
                        Vec3::new(x1, y0, z_bot),
                        Vec3::new(x1, y1, z_bot),
                        Vec3::new(x1, y1, z_top),
                        Vec3::new(x1, y0, z_top),
                    ]),
                    // North rim
                    Face::solid(vec![
                        Vec3::new(x1, y1, z_bot),
                        Vec3::new(x0, y1, z_bot),
                        Vec3::new(x0, y1, z_top),
                        Vec3::new(x1, y1, z_top),
                    ]),
                    // West rim
                    Face::solid(vec![
                        Vec3::new(x0, y1, z_bot),
                        Vec3::new(x0, y0, z_bot),
                        Vec3::new(x0, y0, z_top),
                        Vec3::new(x0, y1, z_top),
                    ]),
                ]
            }

            RoofShape::Gable => {
                let mut faces = Vec::new();
                let z_eave_bot = z_eave - t;

                if w >= h {
                    let ym = (y0 + y1) * 0.5;
                    let z_peak = z_eave + tan_slope * (h * 0.5);
                    let z_peak_bot = z_peak - t;

                    // South slope top (normal out and up: -Y, +Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x1, ym, z_peak),
                        Vec3::new(x0, ym, z_peak),
                    ]));
                    // North slope top (normal out and up: +Y, +Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x0, ym, z_peak),
                        Vec3::new(x1, ym, z_peak),
                    ]));
                    // South soffit (normal out and down: +Y, -Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x0, ym, z_peak_bot),
                        Vec3::new(x1, ym, z_peak_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                    ]));
                    // North soffit (normal out and down: -Y, -Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x1, ym, z_peak_bot),
                        Vec3::new(x0, ym, z_peak_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                    ]));
                    // South eave fascia (normal South: -Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x0, y0, z_eave),
                    ]));
                    // North eave fascia (normal North: +Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x1, y1, z_eave),
                    ]));
                    // West South rake fascia (bargeboard at x0, normal West: -X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(x0, ym, z_peak),
                        Vec3::new(x0, ym, z_peak_bot),
                    ]));
                    // West North rake fascia (bargeboard at x0, normal West: -X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, ym, z_peak_bot),
                        Vec3::new(x0, ym, z_peak),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x0, y1, z_eave_bot),
                    ]));
                    // East South rake fascia (bargeboard at x1, normal East: +X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, ym, z_peak_bot),
                        Vec3::new(x1, ym, z_peak),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x1, y0, z_eave_bot),
                    ]));
                    // East North rake fascia (bargeboard at x1, normal East: +X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x1, ym, z_peak),
                        Vec3::new(x1, ym, z_peak_bot),
                    ]));
                } else {
                    let xm = (x0 + x1) * 0.5;
                    let z_peak = z_eave + tan_slope * (w * 0.5);
                    let z_peak_bot = z_peak - t;

                    // West slope top (normal out and up: -X, +Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(xm, y0, z_peak),
                        Vec3::new(xm, y1, z_peak),
                    ]));
                    // East slope top (normal out and up: +X, +Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(xm, y1, z_peak),
                        Vec3::new(xm, y0, z_peak),
                    ]));
                    // West soffit (normal out and down: +X, -Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(xm, y1, z_peak_bot),
                        Vec3::new(xm, y0, z_peak_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                    ]));
                    // East soffit (normal out and down: -X, -Z)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(xm, y0, z_peak_bot),
                        Vec3::new(xm, y1, z_peak_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                    ]));
                    // West eave fascia (normal West: -X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(x0, y1, z_eave),
                    ]));
                    // East eave fascia (normal East: +X)
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x1, y0, z_eave),
                    ]));
                    // South West rake fascia (bargeboard at y0, normal South: -Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(xm, y0, z_peak_bot),
                        Vec3::new(xm, y0, z_peak),
                        Vec3::new(x0, y0, z_eave),
                    ]));
                    // South East rake fascia (bargeboard at y0, normal South: -Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, y0, z_peak_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(xm, y0, z_peak),
                    ]));
                    // North West rake fascia (bargeboard at y1, normal North: +Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(xm, y1, z_peak),
                        Vec3::new(xm, y1, z_peak_bot),
                    ]));
                    // North East rake fascia (bargeboard at y1, normal North: +Y)
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, y1, z_peak_bot),
                        Vec3::new(xm, y1, z_peak),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x1, y1, z_eave_bot),
                    ]));
                }
                faces
            }

            RoofShape::Hip => {
                let mut faces = Vec::new();
                let z_eave_bot = z_eave - t;

                if w >= h {
                    let d = h * 0.5;
                    let ym = (y0 + y1) * 0.5;
                    let xr0 = (x0 + d).min((x0 + x1) * 0.5);
                    let xr1 = (x1 - d).max((x0 + x1) * 0.5);
                    let z_peak = z_eave + tan_slope * d;
                    let z_peak_bot = z_peak - t;

                    // South slope
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(xr1, ym, z_peak),
                        Vec3::new(xr0, ym, z_peak),
                    ]));
                    // North slope
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(xr0, ym, z_peak),
                        Vec3::new(xr1, ym, z_peak),
                    ]));
                    // West hip
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(xr0, ym, z_peak),
                    ]));
                    // East hip
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(xr1, ym, z_peak),
                    ]));

                    // Soffits (exact reverse of top slopes so outward normals point downward (-Z) into building envelope)
                    faces.push(Face::solid(vec![
                        Vec3::new(xr0, ym, z_peak_bot),
                        Vec3::new(xr1, ym, z_peak_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xr1, ym, z_peak_bot),
                        Vec3::new(xr0, ym, z_peak_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xr0, ym, z_peak_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xr1, ym, z_peak_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                    ]));
                } else {
                    let d = w * 0.5;
                    let xm = (x0 + x1) * 0.5;
                    let yr0 = (y0 + d).min((y0 + y1) * 0.5);
                    let yr1 = (y1 - d).max((y0 + y1) * 0.5);
                    let z_peak = z_eave + tan_slope * d;
                    let z_peak_bot = z_peak - t;

                    // West slope
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(xm, yr0, z_peak),
                        Vec3::new(xm, yr1, z_peak),
                    ]));
                    // East slope
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(xm, yr1, z_peak),
                        Vec3::new(xm, yr0, z_peak),
                    ]));
                    // South hip
                    faces.push(Face::solid(vec![
                        Vec3::new(x0, y0, z_eave),
                        Vec3::new(x1, y0, z_eave),
                        Vec3::new(xm, yr0, z_peak),
                    ]));
                    // North hip
                    faces.push(Face::solid(vec![
                        Vec3::new(x1, y1, z_eave),
                        Vec3::new(x0, y1, z_eave),
                        Vec3::new(xm, yr1, z_peak),
                    ]));

                    // Soffits (exact reverse of top slopes so outward normals point downward (-Z) into building envelope)
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, yr1, z_peak_bot),
                        Vec3::new(xm, yr0, z_peak_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, yr0, z_peak_bot),
                        Vec3::new(xm, yr1, z_peak_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, yr0, z_peak_bot),
                        Vec3::new(x1, y0, z_eave_bot),
                        Vec3::new(x0, y0, z_eave_bot),
                    ]));
                    faces.push(Face::solid(vec![
                        Vec3::new(xm, yr1, z_peak_bot),
                        Vec3::new(x0, y1, z_eave_bot),
                        Vec3::new(x1, y1, z_eave_bot),
                    ]));
                }

                // Eave fascias on all 4 perimeter sides
                faces.push(Face::solid(vec![
                    Vec3::new(x0, y0, z_eave_bot),
                    Vec3::new(x1, y0, z_eave_bot),
                    Vec3::new(x1, y0, z_eave),
                    Vec3::new(x0, y0, z_eave),
                ]));
                faces.push(Face::solid(vec![
                    Vec3::new(x1, y0, z_eave_bot),
                    Vec3::new(x1, y1, z_eave_bot),
                    Vec3::new(x1, y1, z_eave),
                    Vec3::new(x1, y0, z_eave),
                ]));
                faces.push(Face::solid(vec![
                    Vec3::new(x1, y1, z_eave_bot),
                    Vec3::new(x0, y1, z_eave_bot),
                    Vec3::new(x0, y1, z_eave),
                    Vec3::new(x1, y1, z_eave),
                ]));
                faces.push(Face::solid(vec![
                    Vec3::new(x0, y1, z_eave_bot),
                    Vec3::new(x0, y0, z_eave_bot),
                    Vec3::new(x0, y0, z_eave),
                    Vec3::new(x0, y1, z_eave),
                ]));
                faces
            }

            RoofShape::Shed => {
                let z_low = z_eave;
                let z_high = z_eave + tan_slope * h;
                let z_low_bot = z_low - t;
                let z_high_bot = z_high - t;

                vec![
                    // Sloped Top Face
                    Face::solid(vec![
                        Vec3::new(x0, y0, z_low),
                        Vec3::new(x1, y0, z_low),
                        Vec3::new(x1, y1, z_high),
                        Vec3::new(x0, y1, z_high),
                    ]),
                    // Bottom Soffit
                    Face::solid(vec![
                        Vec3::new(x0, y1, z_high_bot),
                        Vec3::new(x1, y1, z_high_bot),
                        Vec3::new(x1, y0, z_low_bot),
                        Vec3::new(x0, y0, z_low_bot),
                    ]),
                    // Low Eave Fascia (at y0)
                    Face::solid(vec![
                        Vec3::new(x0, y0, z_low_bot),
                        Vec3::new(x1, y0, z_low_bot),
                        Vec3::new(x1, y0, z_low),
                        Vec3::new(x0, y0, z_low),
                    ]),
                    // High Ridge Fascia (at y1)
                    Face::solid(vec![
                        Vec3::new(x1, y1, z_high_bot),
                        Vec3::new(x0, y1, z_high_bot),
                        Vec3::new(x0, y1, z_high),
                        Vec3::new(x1, y1, z_high),
                    ]),
                    // West Rake Face (at x0)
                    Face::solid(vec![
                        Vec3::new(x0, y1, z_high_bot),
                        Vec3::new(x0, y0, z_low_bot),
                        Vec3::new(x0, y0, z_low),
                        Vec3::new(x0, y1, z_high),
                    ]),
                    // East Rake Face (at x1)
                    Face::solid(vec![
                        Vec3::new(x1, y0, z_low_bot),
                        Vec3::new(x1, y1, z_high_bot),
                        Vec3::new(x1, y1, z_high),
                        Vec3::new(x1, y0, z_low),
                    ]),
                ]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fascia (Eave Board Trim)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fascia {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub level: LevelId,
    pub height: f64,
    pub thickness: f64,
    pub vertical_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub material_name: String,
}

impl Fascia {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Fascia Board".into(),
            path,
            level,
            height: 200.0,
            thickness: 25.0,
            vertical_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            material_name: "Timber / PVC Fascia".into(),
        }
    }

    pub fn length(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.path.len().saturating_sub(1) {
            total += self.path[i].distance(self.path[i + 1]);
        }
        total
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.vertical_offset;
        let z1 = match top_z {
            Some(tz) => (tz + self.top_offset).max(z0 + 10.0),
            None => (z0 + self.height + self.top_offset).max(z0 + 10.0),
        };
        (z0, z1)
    }

    pub fn solids(&self, level_z: f64) -> Vec<Prism> {
        self.solids_with_bounds(level_z, None)
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let mut prisms = Vec::new();
        let (z0, z1) = self.z_bounds(base_z, top_z);
        for i in 0..self.path.len().saturating_sub(1) {
            let p1 = self.path[i];
            let p2 = self.path[i + 1];
            if p1.distance(p2) > 1.0 {
                prisms.push(Prism::along(p1, p2, self.thickness, z0, z1));
            }
        }
        prisms
    }
}

// ---------------------------------------------------------------------------
// Gutter (Rainwater Eave Drainage Channel)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gutter {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub level: LevelId,
    pub width: f64,
    pub depth: f64,
    pub vertical_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub material_name: String,
}

impl Gutter {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Eave Gutter".into(),
            path,
            level,
            width: 125.0,
            depth: 80.0,
            vertical_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            material_name: "Aluminium / PVC Gutter".into(),
        }
    }

    pub fn length(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.path.len().saturating_sub(1) {
            total += self.path[i].distance(self.path[i + 1]);
        }
        total
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let base_ref = base_z + self.vertical_offset;
        match top_z {
            Some(tz) => {
                let z1 = tz + self.top_offset;
                if z1 > base_ref + 10.0 {
                    (base_ref, z1)
                } else {
                    (z1 - self.depth.max(10.0), z1)
                }
            }
            None => {
                let z1 = base_ref + self.top_offset;
                let z0 = z1 - self.depth.max(10.0);
                (z0, z1)
            }
        }
    }

    pub fn solids(&self, level_z: f64) -> Vec<Prism> {
        self.solids_with_bounds(level_z, None)
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let mut prisms = Vec::new();
        let (z0, z1) = self.z_bounds(base_z, top_z);
        for i in 0..self.path.len().saturating_sub(1) {
            let p1 = self.path[i];
            let p2 = self.path[i + 1];
            if p1.distance(p2) > 1.0 {
                prisms.push(Prism::along(p1, p2, self.width, z0, z1));
            }
        }
        prisms
    }
}

// ---------------------------------------------------------------------------
// Stair (Monolithic / Stepped / Open-Riser Circulation)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stair {
    pub id: ElementId,
    pub name: String,
    pub start: Vec2,
    pub end: Vec2,
    pub width: f64,
    pub base_level: LevelId,
    #[serde(default)]
    pub base_offset: f64,
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub height: f64,
    pub riser_count: usize,
    #[serde(default = "default_stair_waist")]
    pub waist_thickness: f64,
    #[serde(default)]
    pub stair_style: StairStyle,
    pub stair_type_name: String,
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
}

impl Stair {
    pub fn new(start: Vec2, end: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Monolithic Stair".into(),
            start,
            end,
            width: 1000.0,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            height: 3000.0,
            riser_count: 18,
            waist_thickness: 150.0,
            stair_style: StairStyle::Monolithic,
            stair_type_name: "Cast-In-Place Concrete Stair".into(),
            tag: "ST-01".into(),
            is_external: false,
            fire_rating: "2 HR".into(),
        }
    }

    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn direction(&self) -> Vec2 {
        let d = self.end - self.start;
        if d.length() < 1e-4 {
            Vec2::X
        } else {
            d.normalized()
        }
    }

    pub fn normal(&self) -> Vec2 {
        self.direction().perp()
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        let norm = self.normal() * (self.width * 0.5);
        vec![
            self.start - norm,
            self.end - norm,
            self.end + norm,
            self.start + norm,
        ]
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.height + self.top_offset,
        }
        .max(z0 + 50.0);
        (z0, z1)
    }

    pub fn riser_height(&self, base_z: f64, top_z: Option<f64>) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        (z1 - z0) / (self.riser_count.max(2) as f64)
    }

    pub fn tread_going(&self) -> f64 {
        self.length() / (self.riser_count.max(2) as f64)
    }

    pub fn solids(&self, base_z: f64) -> Vec<Prism> {
        self.solids_with_bounds(base_z, None)
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let n = self.riser_count.max(2);
        let dir = self.direction();
        let norm = self.normal() * (self.width * 0.5);
        let run_len = self.length().max(100.0);
        let total_h = (z1 - z0).max(50.0);
        let step_len = run_len / (n as f64);
        let step_height = total_h / (n as f64);
        let hypot = run_len.hypot(total_h);
        let v_drop = match self.stair_style {
            StairStyle::Monolithic => (self.waist_thickness.max(60.0) * hypot / run_len).min(total_h * 0.9),
            StairStyle::SolidStepped => total_h,
            StairStyle::OpenRiser => step_height.min(60.0),
        };

        let mut prisms = Vec::with_capacity(n);
        for i in 0..n {
            let d0 = i as f64 * step_len;
            let d1 = (i + 1) as f64 * step_len;
            let p0 = self.start + dir * d0;
            let p1 = self.start + dir * d1;
            let z_top = z0 + (i + 1) as f64 * step_height;
            let z_bot = match self.stair_style {
                StairStyle::SolidStepped => z0,
                StairStyle::Monolithic => (z0 + (d0 / run_len) * total_h - v_drop).max(z0),
                StairStyle::OpenRiser => (z_top - v_drop).max(z0),
            };
            let boundary = vec![p0 - norm, p1 - norm, p1 + norm, p0 + norm];
            prisms.push(Prism::new(boundary, z_bot, z_top));
        }
        prisms
    }

    /// Generates watertight, convex, outward-oriented 3D `Face`s for Monolithic,
    /// SolidStepped, and OpenRiser stairs with zero internal seam edges on side cheeks.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let n = self.riser_count.max(2);
        let run_len = self.length().max(100.0);
        let total_h = (z1 - z0).max(50.0);
        let step_len = run_len / (n as f64);
        let step_h = total_h / (n as f64);

        let dir = self.direction();
        let norm = self.normal();
        let half_w = (self.width * 0.5).max(50.0);

        let pt = |d: f64, w: f64, z: f64| -> Vec3 {
            let p = self.start + dir * d + norm * w;
            Vec3::new(p.x, p.y, z)
        };

        let n_dir = Vec3::new(dir.x, dir.y, 0.0);
        let n_norm = Vec3::new(norm.x, norm.y, 0.0);

        let mut faces = Vec::new();

        if self.stair_style == StairStyle::OpenRiser {
            let stringer_w = (60.0_f64).min(self.width * 0.18).max(30.0);
            let hypot = run_len.hypot(total_h);
            let v_drop = (self.waist_thickness.clamp(120.0, 320.0) * hypot / run_len).min(total_h * 0.85);
            let d_toe = (v_drop * run_len / total_h).clamp(10.0, run_len * 0.9);
            let n_top_slope = Vec3::new(-total_h * dir.x, -total_h * dir.y, run_len).normalized();
            let n_bot_slope = -n_top_slope;

            // Dual sloped side stringers (left: [-half_w, -half_w + stringer_w], right: [half_w - stringer_w, half_w])
            for &(w_a, w_b) in &[(-half_w, -half_w + stringer_w), (half_w - stringer_w, half_w)] {
                // Stringer top sloped face
                faces.push(Face::oriented(
                    vec![
                        pt(0.0, w_a, z0),
                        pt(run_len, w_a, z1 - step_h),
                        pt(run_len, w_b, z1 - step_h),
                        pt(0.0, w_b, z0),
                    ],
                    vec![true; 4],
                    n_top_slope,
                ));
                // Stringer bottom toe seat
                faces.push(Face::oriented(
                    vec![
                        pt(0.0, w_a, z0),
                        pt(d_toe, w_a, z0),
                        pt(d_toe, w_b, z0),
                        pt(0.0, w_b, z0),
                    ],
                    vec![true; 4],
                    Vec3::new(0.0, 0.0, -1.0),
                ));
                // Stringer bottom sloped soffit
                faces.push(Face::oriented(
                    vec![
                        pt(d_toe, w_a, z0),
                        pt(run_len, w_a, (z1 - step_h - v_drop).max(z0)),
                        pt(run_len, w_b, (z1 - step_h - v_drop).max(z0)),
                        pt(d_toe, w_b, z0),
                    ],
                    vec![true; 4],
                    n_bot_slope,
                ));
                // Stringer top end cap
                faces.push(Face::oriented(
                    vec![
                        pt(run_len, w_a, (z1 - step_h - v_drop).max(z0)),
                        pt(run_len, w_b, (z1 - step_h - v_drop).max(z0)),
                        pt(run_len, w_b, z1 - step_h),
                        pt(run_len, w_a, z1 - step_h),
                    ],
                    vec![true; 4],
                    n_dir,
                ));
                // Stringer outer & inner cheeks (convex 4-gons)
                for &(w_s, out_n) in &[(w_a, -n_norm), (w_b, n_norm)] {
                    faces.push(Face::oriented(
                        vec![
                            pt(0.0, w_s, z0),
                            pt(d_toe, w_s, z0),
                            pt(run_len, w_s, (z1 - step_h - v_drop).max(z0)),
                            pt(run_len, w_s, z1 - step_h),
                        ],
                        vec![true; 4],
                        out_n,
                    ));
                }
            }

            // Solid floating treads
            let tread_t = (50.0_f64).min(step_h * 0.55).max(25.0);
            for i in 0..n {
                let d0 = (i as f64 * step_len - 20.0).max(0.0); // slight nosing
                let d1 = (i + 1) as f64 * step_len;
                let zt = z0 + (i + 1) as f64 * step_h;
                let zb = zt - tread_t;
                let p0 = self.start + dir * d0;
                let p1 = self.start + dir * d1;
                let wv = norm * half_w;
                let tread_prism = Prism::new(vec![p0 - wv, p1 - wv, p1 + wv, p0 + wv], zb, zt);
                faces.extend(tread_prism.faces());
            }
            return faces;
        }

        // Monolithic or SolidStepped stair
        let hypot = run_len.hypot(total_h);
        let is_stepped_block = self.stair_style == StairStyle::SolidStepped;
        let v_drop = if is_stepped_block {
            total_h
        } else {
            (self.waist_thickness.clamp(80.0, 600.0) * hypot / run_len).clamp(60.0, total_h * 0.88)
        };
        let d_toe = if is_stepped_block {
            run_len
        } else {
            (v_drop * run_len / total_h).clamp(0.0, run_len)
        };

        let z_soffit = |d: f64| -> f64 {
            if is_stepped_block || d <= d_toe {
                z0
            } else {
                (z0 + (d / run_len) * total_h - v_drop).max(z0)
            }
        };

        // 1. Horizontal Treads & Vertical Risers
        for i in 0..n {
            let da = i as f64 * step_len;
            let db = (i + 1) as f64 * step_len;
            let z_prev = z0 + i as f64 * step_h;
            let z_curr = z0 + (i + 1) as f64 * step_h;

            // Riser i (facing -dir)
            faces.push(Face::oriented(
                vec![
                    pt(da, -half_w, z_prev),
                    pt(da, half_w, z_prev),
                    pt(da, half_w, z_curr),
                    pt(da, -half_w, z_curr),
                ],
                vec![true; 4],
                -n_dir,
            ));

            // Tread i (facing +Z)
            faces.push(Face::oriented(
                vec![
                    pt(da, -half_w, z_curr),
                    pt(db, -half_w, z_curr),
                    pt(db, half_w, z_curr),
                    pt(da, half_w, z_curr),
                ],
                vec![true; 4],
                Vec3::new(0.0, 0.0, 1.0),
            ));
        }

        // 2. Bottom Toe Cap (z = z0, d in 0..d_toe)
        if d_toe > 1.0 {
            faces.push(Face::oriented(
                vec![
                    pt(0.0, -half_w, z0),
                    pt(d_toe, -half_w, z0),
                    pt(d_toe, half_w, z0),
                    pt(0.0, half_w, z0),
                ],
                vec![true; 4],
                Vec3::new(0.0, 0.0, -1.0),
            ));
        }

        // 3. Inclined Soffit (d in d_toe..run_len)
        if !is_stepped_block && run_len - d_toe > 1.0 {
            let z_end_bot = z_soffit(run_len);
            let n_soffit = Vec3::new(total_h * dir.x, total_h * dir.y, -run_len).normalized();
            faces.push(Face::oriented(
                vec![
                    pt(d_toe, -half_w, z0),
                    pt(run_len, -half_w, z_end_bot),
                    pt(run_len, half_w, z_end_bot),
                    pt(d_toe, half_w, z0),
                ],
                vec![true; 4],
                n_soffit,
            ));
        }

        // 4. Top Vertical Back Landing Face (at d = run_len)
        let z_end_bot = z_soffit(run_len);
        if z1 - z_end_bot > 1.0 {
            faces.push(Face::oriented(
                vec![
                    pt(run_len, -half_w, z_end_bot),
                    pt(run_len, half_w, z_end_bot),
                    pt(run_len, half_w, z1),
                    pt(run_len, -half_w, z1),
                ],
                vec![true; 4],
                n_dir,
            ));
        }

        // 5. Left (+norm) & Right (-norm) Side Cheeks decomposed into exact convex quads
        // with internal vertical chord seams marked `hard = false`.
        for &(w_side, out_n) in &[(half_w, n_norm), (-half_w, -n_norm)] {
            for i in 0..n {
                let da = i as f64 * step_len;
                let db = (i + 1) as f64 * step_len;
                let z_curr = z0 + (i + 1) as f64 * step_h;

                // Split at `d_toe` if `d_toe` falls strictly inside `(da, db)` so bottom edge stays exact
                let mut sub_spans = Vec::with_capacity(2);
                if !is_stepped_block && d_toe > da + 1.0 && d_toe < db - 1.0 {
                    sub_spans.push((da, d_toe, true, false));
                    sub_spans.push((d_toe, db, false, true));
                } else {
                    sub_spans.push((da, db, true, true));
                }

                for (sa, sb, is_step_start, is_step_end) in sub_spans {
                    let zba = z_soffit(sa);
                    // Overlap right internal seam slightly by 1.5mm into next taller step to prevent
                    // sub-pixel anti-aliasing hairlines when rendered as convex polygons
                    let sb_fill = if i + 1 < n && is_step_end {
                        (sb + 1.5).min(run_len)
                    } else if !is_step_end {
                        (sb + 1.5).min(db)
                    } else {
                        sb
                    };
                    let zbb_fill = z_soffit(sb_fill);

                    let left_hard = i == 0 && is_step_start;
                    let right_hard = i + 1 == n && is_step_end;
                    // pts: [bot-left, bot-right, top-right, top-left]
                    // edges: 0=bottom(soffit), 1=right(vertical), 2=top(tread), 3=left(vertical)
                    // Note: at left edge of step i>0, the portion from z_prev..z_curr is an outer riser edge,
                    // which is already drawn crisply by Riser i's perimeter!
                    let pts = vec![
                        pt(sa, w_side, zba),
                        pt(sb_fill, w_side, zbb_fill),
                        pt(sb_fill, w_side, z_curr),
                        pt(sa, w_side, z_curr),
                    ];
                    let hard = vec![true, right_hard, true, left_hard];
                    faces.push(Face::oriented(pts, hard, out_n));
                }
            }
        }

        faces
    }
}

// ---------------------------------------------------------------------------
// Ramp (Accessible Incline)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ramp {
    pub id: ElementId,
    pub name: String,
    pub start: Vec2,
    pub end: Vec2,
    pub width: f64,
    pub thickness: f64,
    pub height: f64,
    pub base_level: LevelId,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    #[serde(default)]
    pub ramp_style: RampStyle,
    pub ramp_type_name: String,
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
}

impl Ramp {
    pub fn new(start: Vec2, end: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Access Ramp".into(),
            start,
            end,
            width: 1200.0,
            thickness: 150.0,
            height: 600.0,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            ramp_style: RampStyle::SlopedSlab,
            ramp_type_name: "Cast-in-Place Concrete Ramp 1:12".into(),
            tag: "RP-01".into(),
            is_external: true,
            fire_rating: "2 HR".into(),
        }
    }

    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn direction(&self) -> Vec2 {
        let d = self.end - self.start;
        if d.length() < 1e-4 {
            Vec2::X
        } else {
            d.normalized()
        }
    }

    pub fn normal(&self) -> Vec2 {
        self.direction().perp()
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        let norm = self.normal() * (self.width * 0.5);
        vec![
            self.start - norm,
            self.end - norm,
            self.end + norm,
            self.start + norm,
        ]
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.height + self.top_offset,
        }
        .max(z0 + 20.0);
        (z0, z1)
    }

    /// Slope ratio `1 : X` (run / rise).
    pub fn slope_ratio(&self, base_z: f64, top_z: Option<f64>) -> f64 {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let rise = (z1 - z0).max(1.0);
        self.length() / rise
    }

    pub fn solids(&self, base_z: f64) -> Vec<Prism> {
        self.solids_with_bounds(base_z, None)
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let dir = self.direction();
        let norm = self.normal() * (self.width * 0.5);
        let run_len = self.length().max(100.0);
        let boundary = vec![
            self.start - norm,
            self.start + dir * run_len - norm,
            self.start + dir * run_len + norm,
            self.start + norm,
        ];
        vec![Prism::new(boundary, z0, z1)]
    }

    /// Generates exact, watertight, 100% convex 3D inclined solid faces for the Ramp
    /// (`SlopedSlab` or `SolidWedge`) with zero step artifacts and zero seams.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let run_len = self.length().max(100.0);
        let rise = (z1 - z0).max(20.0);
        let dir = self.direction();
        let norm = self.normal();
        let half_w = (self.width * 0.5).max(50.0);

        let pt = |d: f64, w: f64, z: f64| -> Vec3 {
            let p = self.start + dir * d + norm * w;
            Vec3::new(p.x, p.y, z)
        };

        let n_dir = Vec3::new(dir.x, dir.y, 0.0);
        let n_norm = Vec3::new(norm.x, norm.y, 0.0);
        let n_top = Vec3::new(-rise * dir.x, -rise * dir.y, run_len).normalized();
        let n_soffit = -n_top;

        let hypot = run_len.hypot(rise);
        let t_vert = self.thickness.max(40.0) * (hypot / run_len);

        let mut faces = Vec::with_capacity(6);

        // 1. Monolithic Inclined Top Deck (from (0, z0) to (run_len, z1))
        faces.push(Face::oriented(
            vec![
                pt(0.0, -half_w, z0),
                pt(run_len, -half_w, z1),
                pt(run_len, half_w, z1),
                pt(0.0, half_w, z0),
            ],
            vec![true; 4],
            n_top,
        ));

        if self.ramp_style == RampStyle::SolidWedge || t_vert >= rise - 5.0 {
            // Solid triangular wedge sitting flat on z0
            // 2. Flat Bottom Cap
            faces.push(Face::oriented(
                vec![
                    pt(0.0, -half_w, z0),
                    pt(run_len, -half_w, z0),
                    pt(run_len, half_w, z0),
                    pt(0.0, half_w, z0),
                ],
                vec![true; 4],
                Vec3::new(0.0, 0.0, -1.0),
            ));
            // 3. Top Vertical Back Face
            faces.push(Face::oriented(
                vec![
                    pt(run_len, -half_w, z0),
                    pt(run_len, half_w, z0),
                    pt(run_len, half_w, z1),
                    pt(run_len, -half_w, z1),
                ],
                vec![true; 4],
                n_dir,
            ));
            // 4 & 5. Monolithic Triangular Side Cheeks
            for &(w_s, out_n) in &[(half_w, n_norm), (-half_w, -n_norm)] {
                faces.push(Face::oriented(
                    vec![pt(0.0, w_s, z0), pt(run_len, w_s, z0), pt(run_len, w_s, z1)],
                    vec![true; 3],
                    out_n,
                ));
            }
        } else {
            // Sloped Structural Ramp Slab clipped cleanly at floor toe z0
            let d_toe = (t_vert * run_len / rise).clamp(10.0, run_len - 10.0);
            let z_end_bot = z1 - t_vert;

            // 2. Flat Bottom Toe Seat (0..d_toe at z0)
            faces.push(Face::oriented(
                vec![
                    pt(0.0, -half_w, z0),
                    pt(d_toe, -half_w, z0),
                    pt(d_toe, half_w, z0),
                    pt(0.0, half_w, z0),
                ],
                vec![true; 4],
                Vec3::new(0.0, 0.0, -1.0),
            ));
            // 3. Monolithic Inclined Soffit (d_toe..run_len)
            faces.push(Face::oriented(
                vec![
                    pt(d_toe, -half_w, z0),
                    pt(run_len, -half_w, z_end_bot),
                    pt(run_len, half_w, z_end_bot),
                    pt(d_toe, half_w, z0),
                ],
                vec![true; 4],
                n_soffit,
            ));
            // 4. Top Vertical Landing Header Face (at d = run_len)
            faces.push(Face::oriented(
                vec![
                    pt(run_len, -half_w, z_end_bot),
                    pt(run_len, half_w, z_end_bot),
                    pt(run_len, half_w, z1),
                    pt(run_len, -half_w, z1),
                ],
                vec![true; 4],
                n_dir,
            ));
            // 5 & 6. Monolithic Convex 4-gon Side Cheeks
            for &(w_s, out_n) in &[(half_w, n_norm), (-half_w, -n_norm)] {
                faces.push(Face::oriented(
                    vec![
                        pt(0.0, w_s, z0),
                        pt(d_toe, w_s, z0),
                        pt(run_len, w_s, z_end_bot),
                        pt(run_len, w_s, z1),
                    ],
                    vec![true; 4],
                    out_n,
                ));
            }
        }

        faces
    }
}

// ---------------------------------------------------------------------------
// Railing (Handrail / Guardrail)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Railing {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub level: LevelId,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub height: f64,
    pub baluster_spacing: f64,
    #[serde(default = "default_rail_count")]
    pub mid_rail_count: usize,
    pub railing_type_name: String,
    pub tag: String,
    pub is_external: bool,
    pub material_name: String,
}

impl Railing {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Pipe Railing".into(),
            path,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            height: 900.0,
            baluster_spacing: 300.0,
            mid_rail_count: 1,
            railing_type_name: "Steel Pipe Railing 900mm".into(),
            tag: "RL-01".into(),
            is_external: false,
            material_name: "Stainless Steel".into(),
        }
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.height + self.top_offset,
        }
        .max(z0 + 100.0);
        (z0, z1)
    }

    pub fn solids(&self, level_z: f64) -> Vec<Prism> {
        self.solids_with_bounds(level_z, None)
    }

    pub fn solids_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let mut prisms = Vec::new();
        let (z_base, z_top) = self.z_bounds(base_z, top_z);
        let rail_h = (z_top - z_base).max(100.0);
        let rail_w = 40.0;
        let z_handrail_bot = z_top - 40.0;
        let z_bot_rail = z_base + (100.0_f64).min(rail_h * 0.25);
        let z_bot_rail_top = z_bot_rail + 30.0;

        for i in 0..self.path.len().saturating_sub(1) {
            let p1 = self.path[i];
            let p2 = self.path[i + 1];
            let dist = p1.distance(p2);
            if dist < 1.0 {
                continue;
            }
            // Top Handrail
            prisms.push(Prism::along(p1, p2, rail_w, z_handrail_bot, z_top));
            // Bottom Rail
            prisms.push(Prism::along(p1, p2, rail_w * 0.7, z_bot_rail, z_bot_rail_top));

            // Intermediate Mid-Rails
            if self.mid_rail_count > 0 && z_handrail_bot > z_bot_rail_top + 40.0 {
                let clear_span = z_handrail_bot - z_bot_rail_top;
                for m in 1..=self.mid_rail_count {
                    let zm = z_bot_rail_top + clear_span * (m as f64) / ((self.mid_rail_count + 1) as f64);
                    prisms.push(Prism::along(p1, p2, rail_w * 0.6, zm - 12.0, zm + 12.0));
                }
            }

            // Baluster posts
            let dir = (p2 - p1).normalized();
            let num_posts = ((dist / self.baluster_spacing.max(50.0)).floor() as usize).max(1);
            let spacing = dist / (num_posts as f64);
            for j in 0..=num_posts {
                let post_center = p1 + dir * (j as f64 * spacing);
                let half_p = 15.0;
                let post_bounds = vec![
                    plinth_geom::vec2(post_center.x - half_p, post_center.y - half_p),
                    plinth_geom::vec2(post_center.x + half_p, post_center.y - half_p),
                    plinth_geom::vec2(post_center.x + half_p, post_center.y + half_p),
                    plinth_geom::vec2(post_center.x - half_p, post_center.y + half_p),
                ];
                prisms.push(Prism::new(post_bounds, z_base, z_top));
            }
        }
        prisms
    }
}

// ---------------------------------------------------------------------------
// Ceiling (Reflected Ceiling Plan element)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ceiling {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub height_above_level: f64, // mm (Base Offset above Base Level, e.g. 2600 mm)
    pub thickness: f64,          // mm (e.g. 20 mm)
    pub level: LevelId,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    #[serde(default)]
    pub assembly_layers: Vec<AssemblyLayer>,
    pub grid_spacing_x: f64, // e.g. 600 mm
    pub grid_spacing_y: f64, // e.g. 600 mm
    pub ceiling_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_CeilingCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub acoustic_rating: String,
    pub light_reflectance: f64,
}

impl Ceiling {
    pub fn new(boundary: Vec<Vec2>, level: LevelId) -> Self {
        let mut clean_boundary = plinth_geom::polygon::dedup_closed(&boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean_boundary);
        Self {
            id: ElementId::new(),
            name: "Compound Ceiling".into(),
            boundary: clean_boundary,
            height_above_level: 2600.0,
            thickness: 20.0,
            level,
            top_level: None,
            top_offset: 0.0,
            assembly_layers: Vec::new(),
            grid_spacing_x: 600.0,
            grid_spacing_y: 600.0,
            ceiling_type_name: "Acoustic Tile 600x600mm".into(),
            material_id: None,
            tag: "C-01".into(),
            is_external: false,
            fire_rating: "1 HR".into(),
            acoustic_rating: "CAC 35 dB".into(),
            light_reflectance: 0.85,
        }
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z_bot = base_z + self.height_above_level;
        let z_top = match top_z {
            Some(tz) => (tz + self.top_offset).max(z_bot + 5.0),
            None => (z_bot + self.thickness + self.top_offset).max(z_bot + 5.0),
        };
        (z_bot, z_top)
    }

    pub fn solid(&self, level_z: f64) -> Prism {
        self.solid_with_bounds(level_z, None)
    }

    pub fn solid_with_bounds(&self, base_z: f64, top_z: Option<f64>) -> Prism {
        let (z_bot, z_top) = self.z_bounds(base_z, top_z);
        Prism::new(self.boundary.clone(), z_bot, z_top)
    }

    /// Generates watertight 3D solid faces for the slab. Non-convex boundaries (L-shaped, U-shaped,
    /// notched, stepped) are ear-clipped triangulated on top and bottom caps with soft internal diagonals,
    /// eliminating starburst fan rays and flaps across notches/voids.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let (z_bot, z_top) = self.z_bounds(base_z, top_z);
        let mut clean = plinth_geom::polygon::dedup_closed(&self.boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        let n = clean.len();
        if n < 3 || z_top - z_bot < 1e-4 {
            return Vec::new();
        }

        let mut faces = Vec::new();

        // 1. Vertical perimeter rim faces (strictly outward normals)
        for i in 0..n {
            let a = clean[i];
            let b = clean[(i + 1) % n];
            if a.distance(b) < 1e-4 {
                continue;
            }
            let pts = vec![
                a.extend(z_bot),
                b.extend(z_bot),
                b.extend(z_top),
                a.extend(z_top),
            ];
            faces.push(Face::solid(pts));
        }

        // 2. Top and Bottom caps with ear-clipping triangulation for non-convex shapes
        let is_poly_convex = n == 3 || (n == 4 && plinth_geom::polygon::is_convex(&clean));
        if is_poly_convex {
            faces.push(Face::solid(clean.iter().map(|p| p.extend(z_top)).collect()));
            faces.push(Face::solid(clean.iter().rev().map(|p| p.extend(z_bot)).collect()));
        } else {
            let tris = plinth_geom::polygon::triangulate(&clean);
            let is_perimeter = |i: usize, j: usize| -> bool {
                (i + 1) % n == j || (j + 1) % n == i
            };
            for t in &tris {
                let (ia, ib, ic) = (t[0], t[1], t[2]);
                // Top cap face (+Z)
                let pts_top = vec![
                    clean[ia].extend(z_top),
                    clean[ib].extend(z_top),
                    clean[ic].extend(z_top),
                ];
                let hard_top = vec![
                    is_perimeter(ia, ib),
                    is_perimeter(ib, ic),
                    is_perimeter(ic, ia),
                ];
                faces.push(Face::oriented(pts_top, hard_top, plinth_geom::Vec3::Z));

                // Bottom cap face (-Z)
                let pts_bot = vec![
                    clean[ia].extend(z_bot),
                    clean[ic].extend(z_bot),
                    clean[ib].extend(z_bot),
                ];
                let hard_bot = vec![
                    is_perimeter(ia, ic),
                    is_perimeter(ic, ib),
                    is_perimeter(ib, ia),
                ];
                faces.push(Face::oriented(pts_bot, hard_bot, -plinth_geom::Vec3::Z));
            }
        }

        faces
    }
}

// ---------------------------------------------------------------------------
// Door
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub id: ElementId,
    pub name: String,
    pub host_wall: ElementId,
    pub offset_along_wall: f64, // mm along wall centerline
    pub width: f64,             // mm (e.g. 900.0)
    pub height: f64,            // mm (e.g. 2100.0)
    pub sill_height: f64,       // mm (e.g. 0.0)
    #[serde(default)]
    pub base_level: Option<LevelId>,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub flipped_hand: bool,     // swing left vs right
    pub flipped_face: bool,     // swing in vs out
    pub door_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_DoorCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub is_fire_exit: bool,
    pub fire_rating: String,
    pub acoustic_rating: String,
    pub security_rating: String,
    pub thermal_transmittance: f64,
    pub operation_type: String,
}

impl Door {
    pub fn new(host_wall: ElementId, offset_along_wall: f64) -> Self {
        Self {
            id: ElementId::new(),
            name: "Single-Flush Door".into(),
            host_wall,
            offset_along_wall,
            width: 900.0,
            height: 2100.0,
            sill_height: 0.0,
            base_level: None,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            flipped_hand: false,
            flipped_face: false,
            door_type_name: "Single-Flush 900 x 2100mm".into(),
            material_id: None,
            tag: "D-01".into(),
            is_external: false,
            is_fire_exit: false,
            fire_rating: "FD30".into(),
            acoustic_rating: "Rw 32 dB".into(),
            security_rating: "PAS 24".into(),
            thermal_transmittance: 1.8,
            operation_type: "SINGLE_SWING_LEFT".into(),
        }
    }

    /// Computes exact `(z_bot, z_top)` in world Z given resolved base level Z (or host wall base Z)
    /// and optional top level Z.
    pub fn z_bounds(&self, resolved_base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = resolved_base_z + self.base_offset + self.sill_height;
        let z1 = match top_z {
            Some(tz) => (tz + self.top_offset).max(z0 + 100.0),
            None => (z0 + self.height + self.top_offset).max(z0 + 100.0),
        };
        (z0, z1)
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub id: ElementId,
    pub name: String,
    pub host_wall: ElementId,
    pub offset_along_wall: f64, // mm along wall centerline
    pub width: f64,             // mm (e.g. 1200.0)
    pub height: f64,            // mm (e.g. 1500.0)
    pub sill_height: f64,       // mm (e.g. 900.0)
    #[serde(default)]
    pub base_level: Option<LevelId>,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub window_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_WindowCommon) & Identification:
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub acoustic_rating: String,
    pub thermal_transmittance: f64,
    pub solar_heat_gain_coefficient: f64,
    pub operation_type: String,
}

impl Window {
    pub fn new(host_wall: ElementId, offset_along_wall: f64) -> Self {
        Self {
            id: ElementId::new(),
            name: "Fixed Window".into(),
            host_wall,
            offset_along_wall,
            width: 1200.0,
            height: 1500.0,
            sill_height: 900.0,
            base_level: None,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            window_type_name: "Fixed 1200 x 1500mm".into(),
            material_id: None,
            tag: "W-01".into(),
            is_external: true,
            fire_rating: "Unrated".into(),
            acoustic_rating: "Rw 36 dB".into(),
            thermal_transmittance: 1.40,
            solar_heat_gain_coefficient: 0.40,
            operation_type: "SIDE_HUNG_RIGHTHAND".into(),
        }
    }

    /// Computes exact `(z_bot, z_top)` in world Z given resolved base level Z (or host wall base Z)
    /// and optional top level Z.
    pub fn z_bounds(&self, resolved_base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = resolved_base_z + self.base_offset + self.sill_height;
        let z1 = match top_z {
            Some(tz) => (tz + self.top_offset).max(z0 + 100.0),
            None => (z0 + self.height + self.top_offset).max(z0 + 100.0),
        };
        (z0, z1)
    }
}

// ---------------------------------------------------------------------------
// Curtain Wall & Storefront System
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurtainWall {
    pub id: ElementId,
    pub name: String,
    pub start: Vec2,
    pub end: Vec2,
    pub base_level: LevelId,
    pub base_offset: f64,
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub unconnected_height: f64,
    pub host_wall: Option<ElementId>,
    pub automatically_embed: bool,
    pub mullion_width: f64,         // mm along wall direction (e.g. 50.0)
    pub mullion_depth: f64,         // mm perpendicular to wall (e.g. 150.0)
    pub panel_thickness: f64,       // mm glass thickness (e.g. 28.0)
    pub vertical_grid_spacing: f64,   // mm (e.g. 1500.0)
    pub horizontal_grid_spacing: f64, // mm (e.g. 1200.0)
    pub curtain_wall_type_name: String,
    pub material_id: Option<MaterialId>,
    // IFC4 Property Sets (Pset_CurtainWallCommon):
    pub tag: String,
    pub is_external: bool,
    pub fire_rating: String,
    pub acoustic_rating: String,
    pub thermal_transmittance: f64, // W/(m²·K)
}

impl CurtainWall {
    pub fn new(start: Vec2, end: Vec2, base_level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Curtain Wall".into(),
            start,
            end,
            base_level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            unconnected_height: 3000.0,
            host_wall: None,
            automatically_embed: true,
            mullion_width: 50.0,
            mullion_depth: 150.0,
            panel_thickness: 28.0,
            vertical_grid_spacing: 1500.0,
            horizontal_grid_spacing: 1200.0,
            curtain_wall_type_name: "Storefront 1500 x 1200mm".into(),
            material_id: None,
            tag: "CW-01".into(),
            is_external: true,
            fire_rating: "1 HR".into(),
            acoustic_rating: "38 dB".into(),
            thermal_transmittance: 1.25,
        }
    }

    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn direction(&self) -> Vec2 {
        let len = self.length();
        if len > 1e-6 {
            (self.end - self.start) / len
        } else {
            Vec2::new(1.0, 0.0)
        }
    }

    pub fn normal(&self) -> Vec2 {
        self.direction().perp()
    }

    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        let z0 = base_z + self.base_offset;
        let z1 = match top_z {
            Some(tz) => tz + self.top_offset,
            None => z0 + self.unconnected_height + self.top_offset,
        }
        .max(z0 + 100.0);
        (z0.min(z1), z0.max(z1))
    }

    /// Outer bounding profile (4 points) of the curtain wall frame.
    pub fn footprint(&self) -> Vec<Vec2> {
        let norm = self.normal();
        let half_d = self.mullion_depth * 0.5;
        vec![
            self.start - norm * half_d,
            self.end - norm * half_d,
            self.end + norm * half_d,
            self.start + norm * half_d,
        ]
    }

    /// Generates solid prisms for vertical mullions, horizontal transoms, and glass panels.
    pub fn solids(&self, base_z: f64, top_z: Option<f64>) -> Vec<Prism> {
        let (z0, z1) = self.z_bounds(base_z, top_z);
        let len = self.length();
        let h = z1 - z0;
        if len < 10.0 || h < 10.0 {
            return Vec::new();
        }

        let dir = self.direction();
        let norm = self.normal();
        let half_d = self.mullion_depth * 0.5;
        let half_w = (self.mullion_width * 0.5).min(len * 0.2);

        let n_v = (len / self.vertical_grid_spacing.max(200.0)).round().max(1.0) as usize;
        let bay_w = len / (n_v as f64);

        let mut prisms = Vec::new();

        // 1. Vertical mullions
        for i in 0..=n_v {
            let cx = i as f64 * bay_w;
            let (d0, d1) = if i == 0 {
                (0.0, half_w * 2.0)
            } else if i == n_v {
                (len - half_w * 2.0, len)
            } else {
                (cx - half_w, cx + half_w)
            };
            let p0 = self.start + dir * d0;
            let p1 = self.start + dir * d1;
            let profile = vec![
                p0 - norm * half_d,
                p1 - norm * half_d,
                p1 + norm * half_d,
                p0 + norm * half_d,
            ];
            prisms.push(Prism::new(profile, z0, z1));
        }

        // 2. Horizontal rails/transoms and glass panels in bays
        let n_h = (h / self.horizontal_grid_spacing.max(200.0)).round().max(1.0) as usize;
        let tier_h = h / (n_h as f64);
        let half_t_glass = self.panel_thickness * 0.5;

        for i in 0..n_v {
            let bay_start = if i == 0 { half_w * 2.0 } else { i as f64 * bay_w + half_w };
            let bay_end = if i + 1 == n_v { len - half_w * 2.0 } else { (i + 1) as f64 * bay_w - half_w };
            if bay_end <= bay_start + 1.0 {
                continue;
            }

            let bp0 = self.start + dir * bay_start;
            let bp1 = self.start + dir * bay_end;

            // Bottom sill and top head rails
            let rail_profile = vec![
                bp0 - norm * half_d,
                bp1 - norm * half_d,
                bp1 + norm * half_d,
                bp0 + norm * half_d,
            ];
            prisms.push(Prism::new(rail_profile.clone(), z0, (z0 + half_w * 2.0).min(z1)));
            prisms.push(Prism::new(rail_profile.clone(), (z1 - half_w * 2.0).max(z0), z1));

            // Intermediate horizontal transoms
            for j in 1..n_h {
                let tz = z0 + j as f64 * tier_h;
                let tz0 = (tz - half_w).max(z0);
                let tz1 = (tz + half_w).min(z1);
                if tz1 > tz0 + 1.0 {
                    prisms.push(Prism::new(rail_profile.clone(), tz0, tz1));
                }
            }

            // Glass panels in this bay for each tier
            let glass_profile = vec![
                bp0 - norm * half_t_glass,
                bp1 - norm * half_t_glass,
                bp1 + norm * half_t_glass,
                bp0 + norm * half_t_glass,
            ];

            for j in 0..n_h {
                let gz0 = if j == 0 { z0 + half_w * 2.0 } else { z0 + j as f64 * tier_h + half_w };
                let gz1 = if j + 1 == n_h { z1 - half_w * 2.0 } else { z0 + (j + 1) as f64 * tier_h - half_w };
                if gz1 > gz0 + 1.0 {
                    prisms.push(Prism::new(glass_profile.clone(), gz0, gz1));
                }
            }
        }

        prisms
    }

    /// Boundary faces for 3D rendering.
    pub fn faces(&self, base_z: f64, top_z: Option<f64>) -> Vec<Face> {
        let mut all_faces = Vec::new();
        for prism in self.solids(base_z, top_z) {
            all_faces.extend(prism.faces());
        }
        all_faces
    }
}

// ---------------------------------------------------------------------------
// Dimension & Annotation Elements (Linear, Circular, Angular, Text, Leader)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionKind {
    Linear,
    Circular,
    Angular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickMark {
    Tick45,    // 45° architectural slash
    Arrow,     // Filled arrowhead
    Dot,       // Filled circular dot
    OpenArrow, // Open stroke arrowhead
    None,      // Clean dimension without ticks
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    Regular,
    Bold,
    Italic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub id: ElementId,
    pub view_id: ViewId,
    pub kind: DimensionKind,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Option<Vec2>, // Apex for Angular, or third arc point
    pub offset: f64,      // Dimension line perpendicular distance / radius offset / arc radius
    pub tick_mark: TickMark,
    pub font_size: f32,
    pub font_style: FontStyle,
    pub color: [u8; 4],
    pub text_override: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub is_diameter: bool,
}

impl Dimension {
    pub fn new(view_id: ViewId, p1: Vec2, p2: Vec2, offset: f64) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            kind: DimensionKind::Linear,
            p1,
            p2,
            p3: None,
            offset,
            tick_mark: TickMark::Tick45,
            font_size: 12.0,
            font_style: FontStyle::Regular,
            color: [40, 45, 55, 255],
            text_override: None,
            prefix: None,
            suffix: None,
            is_diameter: false,
        }
    }

    pub fn new_linear(
        view_id: ViewId,
        p1: Vec2,
        p2: Vec2,
        offset: f64,
        tick_mark: TickMark,
        font_size: f32,
        font_style: FontStyle,
        color: [u8; 4],
    ) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            kind: DimensionKind::Linear,
            p1,
            p2,
            p3: None,
            offset,
            tick_mark,
            font_size,
            font_style,
            color,
            text_override: None,
            prefix: None,
            suffix: None,
            is_diameter: false,
        }
    }

    pub fn new_circular(
        view_id: ViewId,
        center: Vec2,
        rim_pt: Vec2,
        offset: f64,
        is_diameter: bool,
        tick_mark: TickMark,
        font_size: f32,
        font_style: FontStyle,
        color: [u8; 4],
    ) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            kind: DimensionKind::Circular,
            p1: center,
            p2: rim_pt,
            p3: None,
            offset,
            tick_mark,
            font_size,
            font_style,
            color,
            text_override: None,
            prefix: Some(if is_diameter { "Ø ".to_string() } else { "R ".to_string() }),
            suffix: None,
            is_diameter,
        }
    }

    pub fn new_angular(
        view_id: ViewId,
        apex: Vec2,
        p1: Vec2,
        p2: Vec2,
        arc_radius: f64,
        tick_mark: TickMark,
        font_size: f32,
        font_style: FontStyle,
        color: [u8; 4],
    ) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            kind: DimensionKind::Angular,
            p1: apex,
            p2: p1,
            p3: Some(p2),
            offset: arc_radius,
            tick_mark,
            font_size,
            font_style,
            color,
            text_override: None,
            prefix: None,
            suffix: Some("°".to_string()),
            is_diameter: false,
        }
    }

    pub fn measured_length(&self) -> f64 {
        match self.kind {
            DimensionKind::Linear => self.p1.distance(self.p2),
            DimensionKind::Circular => {
                let r = self.p1.distance(self.p2);
                if self.is_diameter { r * 2.0 } else { r }
            }
            DimensionKind::Angular => self.measured_angle_deg(),
        }
    }

    pub fn measured_angle_deg(&self) -> f64 {
        if let Some(p3) = self.p3 {
            let v1 = self.p2 - self.p1;
            let v2 = p3 - self.p1;
            let a1 = v1.y.atan2(v1.x);
            let a2 = v2.y.atan2(v2.x);
            let mut diff = (a2 - a1).to_degrees().abs();
            if diff > 180.0 {
                diff = 360.0 - diff;
            }
            diff
        } else {
            0.0
        }
    }

    pub fn formatted_text(&self) -> String {
        if let Some(ref text) = self.text_override {
            return text.clone();
        }
        let val = self.measured_length();
        let pre = self.prefix.as_deref().unwrap_or("");
        let suf = self.suffix.as_deref().unwrap_or("");
        match self.kind {
            DimensionKind::Angular => format!("{}{:.1}{}", pre, val, suf),
            _ => format!("{}{:.0}{}", pre, val, suf),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextNote {
    pub id: ElementId,
    pub view_id: ViewId,
    pub pos: Vec2,
    pub text: String,
    pub font_size: f32,
    pub font_style: FontStyle,
    pub color: [u8; 4],
    pub angle_deg: f64,
}

impl TextNote {
    pub fn new(view_id: ViewId, pos: Vec2, text: impl Into<String>) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            pos,
            text: text.into(),
            font_size: 14.0,
            font_style: FontStyle::Regular,
            color: [40, 45, 55, 255],
            angle_deg: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Leader {
    pub id: ElementId,
    pub view_id: ViewId,
    pub target_pt: Vec2,   // Arrow pointing point
    pub shoulder_pt: Vec2, // Elbow corner
    pub text_pt: Vec2,     // Landing shelf endpoint
    pub text: String,
    pub tick_mark: TickMark,
    pub font_size: f32,
    pub font_style: FontStyle,
    pub color: [u8; 4],
}

impl Leader {
    pub fn new(
        view_id: ViewId,
        target_pt: Vec2,
        shoulder_pt: Vec2,
        text_pt: Vec2,
        text: impl Into<String>,
        tick_mark: TickMark,
        font_size: f32,
        font_style: FontStyle,
        color: [u8; 4],
    ) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            target_pt,
            shoulder_pt,
            text_pt,
            text: text.into(),
            tick_mark,
            font_size,
            font_style,
            color,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailLine {
    pub id: ElementId,
    pub view_id: ViewId,
    pub start: Vec2,
    pub end: Vec2,
    pub style: String, // "Thin", "Medium", "Wide", "Overhead", "Hidden"
}

impl DetailLine {
    pub fn new(view_id: ViewId, start: Vec2, end: Vec2, style: impl Into<String>) -> Self {
        Self {
            id: ElementId::new(),
            view_id,
            start,
            end,
            style: style.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Freeform 2D Planar Surface (for SketchUp-style Push/Pull Extrusion)
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeformSurface {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub elevation: f64,
    pub level: LevelId,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub material_id: Option<MaterialId>,
    #[serde(default)]
    pub ifc_category: Option<String>,
    #[serde(default)]
    pub is_hole: bool,
    #[serde(default)]
    pub boundary_3d: Option<Vec<Vec3>>,
    #[serde(default)]
    pub host_wall: Option<ElementId>,
}

impl FreeformSurface {
    pub fn new_rect(corner1: Vec2, corner2: Vec2, elevation: f64, level: LevelId) -> Self {
        let min_x = corner1.x.min(corner2.x);
        let max_x = corner1.x.max(corner2.x);
        let min_y = corner1.y.min(corner2.y);
        let max_y = corner1.y.max(corner2.y);
        let boundary = vec![
            plinth_geom::vec2(min_x, min_y),
            plinth_geom::vec2(max_x, min_y),
            plinth_geom::vec2(max_x, max_y),
            plinth_geom::vec2(min_x, max_y),
        ];
        Self {
            id: ElementId::new(),
            name: "Planar Rectangle Surface".into(),
            boundary,
            elevation,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            material_id: None,
            ifc_category: None,
            is_hole: false,
            boundary_3d: None,
            host_wall: None,
        }
    }

    pub fn new_circle(center: Vec2, radius: f64, segments: usize, elevation: f64, level: LevelId) -> Self {
        let n = segments.max(16);
        let mut boundary = Vec::with_capacity(n);
        for i in 0..n {
            let theta = (i as f64 / n as f64) * std::f64::consts::TAU;
            boundary.push(plinth_geom::vec2(center.x + radius * theta.cos(), center.y + radius * theta.sin()));
        }
        Self {
            id: ElementId::new(),
            name: "Circular Surface".into(),
            boundary,
            elevation,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            material_id: None,
            ifc_category: None,
            is_hole: false,
            boundary_3d: None,
            host_wall: None,
        }
    }

    pub fn new_polygon(boundary: Vec<Vec2>, elevation: f64, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Polygon Surface".into(),
            boundary,
            elevation,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            material_id: None,
            ifc_category: None,
            is_hole: false,
            boundary_3d: None,
            host_wall: None,
        }
    }

    pub fn area(&self) -> f64 {
        plinth_geom::polygon::area(&self.boundary).abs()
    }

    pub fn faces(&self) -> Vec<Face> {
        if let Some(pts3d) = &self.boundary_3d {
            if pts3d.len() >= 3 {
                let n = pts3d.len();
                let norm = plinth_geom::newell_normal(pts3d);
                return vec![Face::oriented(pts3d.clone(), vec![true; n], norm)];
            }
        }
        let n = self.boundary.len();
        if n >= 3 {
            let pts: Vec<Vec3> = self
                .boundary
                .iter()
                .map(|p| Vec3::new(p.x, p.y, self.elevation))
                .collect();
            vec![Face::oriented(pts, vec![true; n], Vec3::Z)]
        } else {
            Vec::new()
        }
    }

    /// Extrude (Push/Pull) this 2D surface into a 3D FreeformSolid
    pub fn extrude(&self, height: f64) -> FreeformSolid {
        if let Some(pts3d) = &self.boundary_3d {
            if pts3d.len() >= 3 {
                let n = pts3d.len();
                let norm = plinth_geom::newell_normal(pts3d);
                let ext_vec = norm * height;
                let mut faces = Vec::new();

                // Bottom face
                faces.push(Face::oriented(pts3d.clone(), vec![true; n], -norm));

                // Top face
                let top_pts: Vec<Vec3> = pts3d.iter().map(|p| *p + ext_vec).collect();
                faces.push(Face::oriented(top_pts, vec![true; n], norm));

                // Side faces
                for i in 0..n {
                    let p0 = pts3d[i];
                    let p1 = pts3d[(i + 1) % n];
                    let side_pts = vec![p0, p1, p1 + ext_vec, p0 + ext_vec];
                    let side_norm = (p1 - p0).cross(ext_vec).normalized();
                    faces.push(Face::oriented(side_pts, vec![true, true, true, true], side_norm));
                }

                let mut min_z = f64::MAX;
                let mut max_z = f64::MIN;
                for p in pts3d {
                    min_z = min_z.min(p.z).min(p.z + ext_vec.z);
                    max_z = max_z.max(p.z).max(p.z + ext_vec.z);
                }

                return FreeformSolid {
                    id: ElementId::new(),
                    name: format!("Extruded {}", self.name),
                    boundary: self.boundary.clone(),
                    z_min: min_z,
                    z_max: max_z,
                    level: self.level,
                    base_offset: self.base_offset,
                    top_level: self.top_level,
                    top_offset: self.top_offset,
                    primitive: PrimitiveKind::Mesh { faces },
                    material_id: self.material_id,
                    group_name: self.ifc_category.clone(),
                    is_group: false,
                    ifc_category: self.ifc_category.clone(),
                    host_wall: self.host_wall,
                };
            }
        }

        let (z0, z1) = if height >= 0.0 {
            (self.elevation, self.elevation + height)
        } else {
            (self.elevation + height, self.elevation)
        };
        FreeformSolid {
            id: ElementId::new(),
            name: format!("Extruded {}", self.name),
            boundary: self.boundary.clone(),
            z_min: z0,
            z_max: z1,
            level: self.level,
            base_offset: self.base_offset,
            top_level: self.top_level,
            top_offset: self.top_offset,
            primitive: PrimitiveKind::CustomExtrusion,
            material_id: self.material_id,
            group_name: self.ifc_category.clone(),
            is_group: false,
            ifc_category: self.ifc_category.clone(),
            host_wall: self.host_wall,
        }
    }
}

// ---------------------------------------------------------------------------
// Freeform 3D Solid & Geometric Primitives
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveKind {
    CustomExtrusion,
    Box { width: f64, length: f64, height: f64 },
    Cylinder { radius: f64, height: f64 },
    Sphere { radius: f64 },
    Mesh { faces: Vec<Face> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeformSolid {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub z_min: f64,
    pub z_max: f64,
    pub level: LevelId,
    #[serde(default)]
    pub base_offset: f64,
    #[serde(default)]
    pub top_level: Option<LevelId>,
    #[serde(default)]
    pub top_offset: f64,
    pub primitive: PrimitiveKind,
    pub material_id: Option<MaterialId>,
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default)]
    pub ifc_category: Option<String>,
    #[serde(default)]
    pub host_wall: Option<ElementId>,
}

impl FreeformSolid {
    pub fn z_bounds(&self, base_z: f64, top_z: Option<f64>) -> (f64, f64) {
        if self.base_offset != 0.0 || self.top_level.is_some() || self.top_offset != 0.0 {
            let z_bot = base_z + self.base_offset;
            let z_top = match top_z {
                Some(tz) => tz + self.top_offset,
                None => {
                    let h = (self.z_max - self.z_min).abs().max(10.0);
                    z_bot + h + self.top_offset
                }
            };
            (z_bot, z_top)
        } else {
            (self.z_min, self.z_max)
        }
    }

    pub fn new_box(corner1: Vec2, corner2: Vec2, z_min: f64, z_max: f64, level: LevelId) -> Self {
        let min_x = corner1.x.min(corner2.x);
        let max_x = corner1.x.max(corner2.x);
        let min_y = corner1.y.min(corner2.y);
        let max_y = corner1.y.max(corner2.y);
        let boundary = vec![
            plinth_geom::vec2(min_x, min_y),
            plinth_geom::vec2(max_x, min_y),
            plinth_geom::vec2(max_x, max_y),
            plinth_geom::vec2(min_x, max_y),
        ];
        Self {
            id: ElementId::new(),
            name: "Box Primitive".into(),
            boundary,
            z_min: z_min.min(z_max),
            z_max: z_min.max(z_max),
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            primitive: PrimitiveKind::Box {
                width: max_x - min_x,
                length: max_y - min_y,
                height: (z_max - z_min).abs(),
            },
            material_id: None,
            group_name: None,
            is_group: false,
            ifc_category: None,
            host_wall: None,
        }
    }

    pub fn new_cylinder(center: Vec2, radius: f64, z_min: f64, z_max: f64, segments: usize, level: LevelId) -> Self {
        let n = segments.max(16);
        let mut boundary = Vec::with_capacity(n);
        for i in 0..n {
            let theta = (i as f64 / n as f64) * std::f64::consts::TAU;
            boundary.push(plinth_geom::vec2(center.x + radius * theta.cos(), center.y + radius * theta.sin()));
        }
        Self {
            id: ElementId::new(),
            name: "Cylinder Primitive".into(),
            boundary,
            z_min: z_min.min(z_max),
            z_max: z_min.max(z_max),
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            primitive: PrimitiveKind::Cylinder {
                radius,
                height: (z_max - z_min).abs(),
            },
            material_id: None,
            group_name: None,
            is_group: false,
            ifc_category: None,
            host_wall: None,
        }
    }

    pub fn new_sphere(center: Vec2, radius: f64, z_center: f64, level: LevelId) -> Self {
        let n = 24;
        let mut boundary = Vec::with_capacity(n);
        for i in 0..n {
            let theta = (i as f64 / n as f64) * std::f64::consts::TAU;
            boundary.push(plinth_geom::vec2(center.x + radius * theta.cos(), center.y + radius * theta.sin()));
        }
        Self {
            id: ElementId::new(),
            name: "Sphere Primitive".into(),
            boundary,
            z_min: z_center - radius,
            z_max: z_center + radius,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            primitive: PrimitiveKind::Sphere { radius },
            material_id: None,
            group_name: None,
            is_group: false,
            ifc_category: None,
            host_wall: None,
        }
    }

    pub fn new_mesh(name: impl Into<String>, faces: Vec<Face>, level: LevelId) -> Self {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_z = f64::NEG_INFINITY;

        for f in &faces {
            for pt in &f.pts {
                min_x = min_x.min(pt.x);
                max_x = max_x.max(pt.x);
                min_y = min_y.min(pt.y);
                max_y = max_y.max(pt.y);
                min_z = min_z.min(pt.z);
                max_z = max_z.max(pt.z);
            }
        }

        if min_x.is_infinite() {
            min_x = 0.0;
            max_x = 0.0;
            min_y = 0.0;
            max_y = 0.0;
            min_z = 0.0;
            max_z = 0.0;
        }

        let boundary = vec![
            plinth_geom::vec2(min_x, min_y),
            plinth_geom::vec2(max_x, min_y),
            plinth_geom::vec2(max_x, max_y),
            plinth_geom::vec2(min_x, max_y),
        ];

        Self {
            id: ElementId::new(),
            name: name.into(),
            boundary,
            z_min: min_z,
            z_max: max_z,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            primitive: PrimitiveKind::Mesh { faces },
            material_id: None,
            group_name: None,
            is_group: false,
            ifc_category: None,
            host_wall: None,
        }
    }

    pub fn new_custom_extrusion(
        name: impl Into<String>,
        boundary: Vec<Vec2>,
        z_min: f64,
        z_max: f64,
        level: LevelId,
        group: Option<String>,
    ) -> Self {
        Self {
            id: ElementId::new(),
            name: name.into(),
            boundary,
            z_min,
            z_max,
            level,
            base_offset: 0.0,
            top_level: None,
            top_offset: 0.0,
            primitive: PrimitiveKind::CustomExtrusion,
            material_id: None,
            group_name: group.clone(),
            is_group: false,
            ifc_category: group,
            host_wall: None,
        }
    }

    pub fn is_mesh(&self) -> bool {
        matches!(self.primitive, PrimitiveKind::Mesh { .. })
    }

    pub fn is_sphere(&self) -> bool {
        matches!(self.primitive, PrimitiveKind::Sphere { .. })
    }

    pub fn sphere_radius(&self) -> Option<f64> {
        match self.primitive {
            PrimitiveKind::Sphere { radius } => Some(radius),
            _ => None,
        }
    }

    pub fn sphere_center(&self) -> Option<Vec3> {
        match self.primitive {
            PrimitiveKind::Sphere { .. } => {
                if self.boundary.is_empty() {
                    return None;
                }
                let avg = self.boundary.iter().fold(Vec2::ZERO, |a, &b| a + b) / self.boundary.len() as f64;
                let z_center = (self.z_min + self.z_max) * 0.5;
                Some(Vec3::new(avg.x, avg.y, z_center))
            }
            _ => None,
        }
    }

    pub fn height(&self) -> f64 {
        self.z_max - self.z_min
    }

    pub fn volume(&self) -> f64 {
        match &self.primitive {
            PrimitiveKind::Sphere { radius } => (4.0 / 3.0) * std::f64::consts::PI * radius.powi(3),
            PrimitiveKind::Cylinder { radius, height } => std::f64::consts::PI * radius * radius * height,
            PrimitiveKind::Box { width, length, height } => width * length * height,
            PrimitiveKind::CustomExtrusion => plinth_geom::polygon::area(&self.boundary).abs() * self.height(),
            PrimitiveKind::Mesh { faces } => {
                let mut vol = 0.0;
                for f in faces {
                    if f.pts.len() >= 3 {
                        let p0 = f.pts[0];
                        for i in 1..f.pts.len().saturating_sub(1) {
                            let p1 = f.pts[i];
                            let p2 = f.pts[i + 1];
                            vol += p0.dot(p1.cross(p2)) / 6.0;
                        }
                    }
                }
                vol.abs()
            }
        }
    }

    pub fn surface_area(&self) -> f64 {
        match &self.primitive {
            PrimitiveKind::Sphere { radius } => 4.0 * std::f64::consts::PI * radius * radius,
            PrimitiveKind::Cylinder { radius, height } => 2.0 * std::f64::consts::PI * radius * (radius + height),
            PrimitiveKind::Box { width, length, height } => 2.0 * (width * length + width * height + length * height),
            PrimitiveKind::CustomExtrusion => {
                let base_area = plinth_geom::polygon::area(&self.boundary).abs() * 2.0;
                let perimeter: f64 = self.boundary.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>()
                    + self.boundary.last().unwrap_or(&Vec2::ZERO).distance(*self.boundary.first().unwrap_or(&Vec2::ZERO));
                base_area + perimeter * self.height()
            }
            PrimitiveKind::Mesh { faces } => {
                let mut area = 0.0;
                for f in faces {
                    if f.pts.len() >= 3 {
                        for i in 1..f.pts.len().saturating_sub(1) {
                            let v1 = f.pts[i] - f.pts[0];
                            let v2 = f.pts[i + 1] - f.pts[0];
                            area += v1.cross(v2).length() * 0.5;
                        }
                    }
                }
                area
            }
        }
    }

    pub fn solid(&self) -> Prism {
        Prism::new(self.boundary.clone(), self.z_min, self.z_max)
    }

    /// Boundary 3D faces. Generates a true 3D spherical mesh for spheres,
    /// or smooth/hard prism faces for extruded profiles and primitives.
    pub fn faces(&self) -> Vec<Face> {
        match &self.primitive {
            PrimitiveKind::Mesh { faces } => faces.clone(),
            PrimitiveKind::Sphere { radius } => {
                let radius = *radius;
                let center = self.sphere_center().unwrap_or(Vec3::ZERO);
                let rings = 16;
                let sectors = 24;
                let mut faces = Vec::with_capacity(rings * sectors);

                let pt_on_sphere = |phi: f64, theta: f64| -> Vec3 {
                    Vec3::new(
                        center.x + radius * phi.cos() * theta.cos(),
                        center.y + radius * phi.cos() * theta.sin(),
                        center.z + radius * phi.sin(),
                    )
                };

                for i in 0..rings {
                    let phi1 = -std::f64::consts::FRAC_PI_2 + (i as f64 / rings as f64) * std::f64::consts::PI;
                    let phi2 = -std::f64::consts::FRAC_PI_2 + ((i + 1) as f64 / rings as f64) * std::f64::consts::PI;

                    for j in 0..sectors {
                        let theta1 = (j as f64 / sectors as f64) * std::f64::consts::TAU;
                        let theta2 = ((j + 1) as f64 / sectors as f64) * std::f64::consts::TAU;

                        let p1 = pt_on_sphere(phi1, theta1);
                        let p2 = pt_on_sphere(phi1, theta2);
                        let p3 = pt_on_sphere(phi2, theta2);
                        let p4 = pt_on_sphere(phi2, theta1);

                        if i == 0 {
                            // South pole triangle
                            let pts = vec![p1, p3, p4];
                            let centroid = (p1 + p3 + p4) / 3.0;
                            let outward = (centroid - center).normalized();
                            faces.push(Face::oriented(pts, vec![false, false, false], outward));
                        } else if i == rings - 1 {
                            // North pole triangle
                            let pts = vec![p1, p2, p3];
                            let centroid = (p1 + p2 + p3) / 3.0;
                            let outward = (centroid - center).normalized();
                            faces.push(Face::oriented(pts, vec![false, false, false], outward));
                        } else {
                            // Quad
                            let pts = vec![p1, p2, p3, p4];
                            let centroid = (p1 + p2 + p3 + p4) / 4.0;
                            let outward = (centroid - center).normalized();
                            faces.push(Face::oriented(pts, vec![false, false, false, false], outward));
                        }
                    }
                }
                faces
            }
            _ => self.solid().faces(),
        }
    }

    pub fn push_pull(&mut self, delta: f64) {
        self.z_max = (self.z_max + delta).max(self.z_min + 10.0);
    }
}

// ---------------------------------------------------------------------------
// Site Elements: BuildingPad, Toposolid, PropertyLine
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingPad {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub level: LevelId,
    pub elevation_offset: f64,
    pub thickness: f64,
    pub slope_deg: f64,
    pub pad_type_name: String,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl BuildingPad {
    pub fn new(boundary: Vec<Vec2>, level: LevelId) -> Self {
        let mut clean = plinth_geom::polygon::dedup_closed(&boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        Self {
            id: ElementId::new(),
            name: "Building Pad".into(),
            boundary: clean,
            level,
            elevation_offset: 0.0,
            thickness: 300.0,
            slope_deg: 0.0,
            pad_type_name: "Pad 300mm Excavation".into(),
            ifc_psets: HashMap::new(),
        }
    }

    pub fn z_bounds(&self, base_z: f64) -> (f64, f64) {
        let top_z = base_z + self.elevation_offset;
        let bot_z = top_z - self.thickness.max(50.0);
        (bot_z, top_z)
    }

    pub fn faces(&self, base_z: f64) -> Vec<Face> {
        let (z_bot, z_top) = self.z_bounds(base_z);
        if self.boundary.len() < 3 || z_top - z_bot < 1.0 {
            return Vec::new();
        }
        Prism::new(self.boundary.clone(), z_bot, z_top).faces()
    }

    pub fn area(&self) -> f64 {
        plinth_geom::polygon::signed_area(&self.boundary).abs()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToposolidPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl ToposolidPoint {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn from_vec2(pos: Vec2, elevation: f64) -> Self {
        Self { x: pos.x, y: pos.y, z: elevation }
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContourLine {
    pub elevation: f64,
    pub points: Vec<Vec2>,
    pub is_primary: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Toposolid {
    pub id: ElementId,
    pub name: String,
    pub boundary: Vec<Vec2>,
    pub level: LevelId,
    pub base_depth: f64,
    pub elevation_points: Vec<ToposolidPoint>,
    pub primary_contour_interval: f64,
    pub secondary_contour_interval: f64,
    pub show_contours: bool,
    pub toposolid_type_name: String,
    pub pad_ids: Vec<ElementId>,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl Toposolid {
    pub fn new(boundary: Vec<Vec2>, level: LevelId) -> Self {
        let mut clean = plinth_geom::polygon::dedup_closed(&boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        Self {
            id: ElementId::new(),
            name: "Toposolid 1".into(),
            boundary: clean,
            level,
            base_depth: 3000.0,
            elevation_points: Vec::new(),
            primary_contour_interval: 1000.0,
            secondary_contour_interval: 200.0,
            show_contours: true,
            toposolid_type_name: "Toposolid Grass & Earth 3000mm".into(),
            pad_ids: Vec::new(),
            ifc_psets: HashMap::new(),
        }
    }

    pub fn with_depth(mut self, depth: f64) -> Self {
        self.base_depth = depth;
        self
    }

    pub fn area(&self) -> f64 {
        if self.boundary.len() < 3 {
            return 0.0;
        }
        plinth_geom::polygon::signed_area(&self.boundary).abs()
    }

    /// Evaluates terrain surface elevation at coordinate (x, y) relative to site datum.
    /// Uses Inverse Distance Weighting (IDW) interpolation from elevation_points,
    /// or returns 0.0 if no points exist.
    pub fn elevation_at(&self, x: f64, y: f64) -> f64 {
        if self.elevation_points.is_empty() {
            return 0.0;
        }
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;
        for p in &self.elevation_points {
            let dist_sq = (p.x - x).powi(2) + (p.y - y).powi(2);
            if dist_sq < 1.0 {
                return p.z;
            }
            let w = 1.0 / dist_sq;
            total_weight += w;
            weighted_sum += p.z * w;
        }
        if total_weight > 1e-9 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Triangulates the undulating top surface honoring elevation points.
    pub fn triangulate_top_mesh(&self, base_z: f64) -> Vec<Face> {
        let mut faces = Vec::new();
        if self.boundary.len() < 3 {
            return faces;
        }

        let min_x = self.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = self.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = self.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = self.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let grid_res = 12;
        let dx = (max_x - min_x) / (grid_res as f64);
        let dy = (max_y - min_y) / (grid_res as f64);

        for gy in 0..grid_res {
            for gx in 0..grid_res {
                let x0 = min_x + (gx as f64) * dx;
                let x1 = x0 + dx;
                let y0 = min_y + (gy as f64) * dy;
                let y1 = y0 + dy;

                let c = Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                if !plinth_geom::polygon::point_in_polygon(c, &self.boundary) {
                    continue;
                }

                let z00 = base_z + self.elevation_at(x0, y0);
                let z10 = base_z + self.elevation_at(x1, y0);
                let z11 = base_z + self.elevation_at(x1, y1);
                let z01 = base_z + self.elevation_at(x0, y1);

                let p00 = Vec3::new(x0, y0, z00);
                let p10 = Vec3::new(x1, y0, z10);
                let p11 = Vec3::new(x1, y1, z11);
                let p01 = Vec3::new(x0, y1, z01);

                // Triangle 1: p00, p10, p11
                let e1 = p10 - p00;
                let e2 = p11 - p00;
                let n1 = e1.cross(e2).normalized();
                faces.push(Face::oriented(vec![p00, p10, p11], vec![false, false, false], n1));

                // Triangle 2: p00, p11, p01
                let e3 = p11 - p00;
                let e4 = p01 - p00;
                let n2 = e3.cross(e4).normalized();
                faces.push(Face::oriented(vec![p00, p11, p01], vec![false, false, false], n2));
            }
        }

        faces
    }

    /// Generates 3D watertight solid faces for the volumetric toposolid.
    pub fn faces(&self, base_z: f64) -> Vec<Face> {
        if self.boundary.len() < 3 {
            return Vec::new();
        }
        let mut clean = plinth_geom::polygon::dedup_closed(&self.boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        let n = clean.len();

        let min_elevation = self.elevation_points.iter().map(|p| p.z).fold(0.0f64, f64::min);
        let bottom_z = base_z + min_elevation - self.base_depth.max(500.0);

        let mut faces = Vec::new();

        // 1. Vertical perimeter skirts: drop from edge elevation to bottom_z
        for i in 0..n {
            let next = (i + 1) % n;
            let p0 = clean[i];
            let p1 = clean[next];
            let z0 = base_z + self.elevation_at(p0.x, p0.y);
            let z1 = base_z + self.elevation_at(p1.x, p1.y);

            let v_top_left = Vec3::new(p0.x, p0.y, z0);
            let v_top_right = Vec3::new(p1.x, p1.y, z1);
            let v_bot_right = Vec3::new(p1.x, p1.y, bottom_z);
            let v_bot_left = Vec3::new(p0.x, p0.y, bottom_z);

            let edge_vec = v_top_right - v_top_left;
            let down_vec = v_bot_left - v_top_left;
            let norm = edge_vec.cross(down_vec).normalized();

            faces.push(Face::oriented(
                vec![v_top_left, v_top_right, v_bot_right, v_bot_left],
                vec![true, true, true, true],
                norm,
            ));
        }

        // 2. Bottom flat cap face (CW viewed from top, normal -Z)
        let mut bot_pts = Vec::with_capacity(n);
        for p in clean.iter().rev() {
            bot_pts.push(Vec3::new(p.x, p.y, bottom_z));
        }
        faces.push(Face::oriented(
            bot_pts,
            vec![true; n],
            Vec3::new(0.0, 0.0, -1.0),
        ));

        // 3. Top surface
        if self.elevation_points.is_empty() {
            let top_pts: Vec<Vec3> = clean
                .iter()
                .map(|p| Vec3::new(p.x, p.y, base_z))
                .collect();
            faces.push(Face::oriented(
                top_pts,
                vec![true; n],
                Vec3::new(0.0, 0.0, 1.0),
            ));
        } else {
            let top_tris = self.triangulate_top_mesh(base_z);
            faces.extend(top_tris);
        }

        faces
    }

    /// Computes contour lines at secondary and primary intervals.
    pub fn compute_contour_lines(&self) -> Vec<ContourLine> {
        let mut out = Vec::new();
        if self.boundary.len() < 3 {
            return out;
        }

        let min_x = self.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = self.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = self.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = self.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let min_z = self.elevation_points.iter().map(|p| p.z).fold(0.0f64, f64::min);
        let max_z = self.elevation_points.iter().map(|p| p.z).fold(0.0f64, f64::max);

        let sec_int = self.secondary_contour_interval.max(100.0);
        let pri_int = self.primary_contour_interval.max(sec_int);

        if max_z - min_z < 10.0 {
            return out;
        }

        let grid_res = 20;
        let dx = (max_x - min_x) / (grid_res as f64);
        let dy = (max_y - min_y) / (grid_res as f64);

        let mut elev = (min_z / sec_int).floor() * sec_int;
        while elev <= max_z + 1.0 {
            let is_primary = ((elev / pri_int).round() * pri_int - elev).abs() < 1e-3;

            // Marching squares over grid cells
            for gy in 0..grid_res {
                for gx in 0..grid_res {
                    let x0 = min_x + (gx as f64) * dx;
                    let x1 = x0 + dx;
                    let y0 = min_y + (gy as f64) * dy;
                    let y1 = y0 + dy;

                    let c = Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                    if !plinth_geom::polygon::point_in_polygon(c, &self.boundary) {
                        continue;
                    }

                    let z00 = self.elevation_at(x0, y0);
                    let z10 = self.elevation_at(x1, y0);
                    let z11 = self.elevation_at(x1, y1);
                    let z01 = self.elevation_at(x0, y1);

                    // Check if isoline intersects this quad
                    let b00 = z00 >= elev;
                    let b10 = z10 >= elev;
                    let b11 = z11 >= elev;
                    let b01 = z01 >= elev;

                    let case = (b00 as u8) | ((b10 as u8) << 1) | ((b11 as u8) << 2) | ((b01 as u8) << 3);
                    if case == 0 || case == 15 {
                        continue;
                    }

                    // Linear interpolation along edges
                    let interp = |v0: f64, v1: f64, p0: Vec2, p1: Vec2| -> Vec2 {
                        let t = if (v1 - v0).abs() > 1e-5 {
                            ((elev - v0) / (v1 - v0)).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };
                        p0 + (p1 - p0) * t
                    };

                    let p_bot = interp(z00, z10, Vec2::new(x0, y0), Vec2::new(x1, y0));
                    let p_right = interp(z10, z11, Vec2::new(x1, y0), Vec2::new(x1, y1));
                    let p_top = interp(z01, z11, Vec2::new(x0, y1), Vec2::new(x1, y1));
                    let p_left = interp(z00, z01, Vec2::new(x0, y0), Vec2::new(x0, y1));

                    match case {
                        1 | 14 => out.push(ContourLine { elevation: elev, points: vec![p_left, p_bot], is_primary }),
                        2 | 13 => out.push(ContourLine { elevation: elev, points: vec![p_bot, p_right], is_primary }),
                        3 | 12 => out.push(ContourLine { elevation: elev, points: vec![p_left, p_right], is_primary }),
                        4 | 11 => out.push(ContourLine { elevation: elev, points: vec![p_right, p_top], is_primary }),
                        5 => {
                            out.push(ContourLine { elevation: elev, points: vec![p_left, p_top], is_primary });
                            out.push(ContourLine { elevation: elev, points: vec![p_bot, p_right], is_primary });
                        }
                        6 | 9 => out.push(ContourLine { elevation: elev, points: vec![p_bot, p_top], is_primary }),
                        7 | 8 => out.push(ContourLine { elevation: elev, points: vec![p_left, p_top], is_primary }),
                        10 => {
                            out.push(ContourLine { elevation: elev, points: vec![p_left, p_bot], is_primary });
                            out.push(ContourLine { elevation: elev, points: vec![p_right, p_top], is_primary });
                        }
                        _ => {}
                    }
                }
            }

            elev += sec_int;
        }

        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BearingQuadrant {
    NorthEast, // N ... E
    SouthEast, // S ... E
    SouthWest, // S ... W
    NorthWest, // N ... W
}

impl BearingQuadrant {
    pub fn label(self) -> &'static str {
        match self {
            Self::NorthEast => "NE",
            Self::SouthEast => "SE",
            Self::SouthWest => "SW",
            Self::NorthWest => "NW",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::NorthEast => "North-East (NE)",
            Self::SouthEast => "South-East (SE)",
            Self::SouthWest => "South-West (SW)",
            Self::NorthWest => "North-West (NW)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurveySegment {
    pub quadrant: BearingQuadrant,
    pub degrees: u32,
    pub minutes: u32,
    pub seconds: f64,
    pub distance_m: f64,
    pub radius_m: Option<f64>,
}

impl SurveySegment {
    pub fn new(quadrant: BearingQuadrant, deg: u32, min: u32, sec: f64, dist_m: f64) -> Self {
        Self {
            quadrant,
            degrees: deg,
            minutes: min,
            seconds: sec,
            distance_m: dist_m,
            radius_m: None,
        }
    }

    /// Converts quadrant bearing to azimuth radians (clockwise from North = +Y).
    pub fn azimuth_rad(&self) -> f64 {
        let dec_deg = (self.degrees as f64) + (self.minutes as f64 / 60.0) + (self.seconds / 3600.0);
        let az_deg = match self.quadrant {
            BearingQuadrant::NorthEast => dec_deg,
            BearingQuadrant::SouthEast => 180.0 - dec_deg,
            BearingQuadrant::SouthWest => 180.0 + dec_deg,
            BearingQuadrant::NorthWest => 360.0 - dec_deg,
        };
        az_deg.to_radians()
    }

    /// Computes delta (dx, dy) in millimeters from azimuth and distance.
    /// In CAD right-handed coordinates: +Y is North, +X is East.
    pub fn delta_mm(&self) -> Vec2 {
        let az = self.azimuth_rad();
        let dist_mm = self.distance_m * 1000.0;
        let dx = dist_mm * az.sin();
        let dy = dist_mm * az.cos();
        Vec2::new(dx, dy)
    }

    pub fn to_delta_mm(&self) -> Vec2 {
        self.delta_mm()
    }

    /// Formats bearing string e.g. "N 45° 30' 00\" E"
    pub fn format_bearing(&self) -> String {
        let (prefix, suffix) = match self.quadrant {
            BearingQuadrant::NorthEast => ("N", "E"),
            BearingQuadrant::SouthEast => ("S", "E"),
            BearingQuadrant::SouthWest => ("S", "W"),
            BearingQuadrant::NorthWest => ("N", "W"),
        };
        format!("{} {}°{:02}'{:02.0}\" {}", prefix, self.degrees, self.minutes, self.seconds, suffix)
    }

    pub fn bearing_string(&self) -> String {
        self.format_bearing()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyLineCreationMode {
    BySketch,
    BySurveyTable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyLine {
    pub id: ElementId,
    pub name: String,
    pub mode: PropertyLineCreationMode,
    pub boundary: Vec<Vec2>,
    pub survey_table: Vec<SurveySegment>,
    pub start_point: Vec2,
    pub level: LevelId,
    pub property_line_type_name: String,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl PropertyLine {
    pub fn new_sketch(boundary: Vec<Vec2>, level: LevelId) -> Self {
        let mut clean = plinth_geom::polygon::dedup_closed(&boundary, 1.0);
        plinth_geom::polygon::make_ccw(&mut clean);
        let start = clean.first().copied().unwrap_or(Vec2::ZERO);
        let mut line = Self {
            id: ElementId::new(),
            name: "Property Line".into(),
            mode: PropertyLineCreationMode::BySketch,
            boundary: clean,
            survey_table: Vec::new(),
            start_point: start,
            level,
            property_line_type_name: "Property Boundary Line".into(),
            ifc_psets: HashMap::new(),
        };
        line.reconstruct_survey_from_boundary();
        line
    }

    pub fn new_survey(start: Vec2, table: Vec<SurveySegment>, level: LevelId) -> Self {
        let mut line = Self {
            id: ElementId::new(),
            name: "Surveyed Lot Boundary".into(),
            mode: PropertyLineCreationMode::BySurveyTable,
            boundary: Vec::new(),
            survey_table: table,
            start_point: start,
            level,
            property_line_type_name: "Surveyed Lot Boundary".into(),
            ifc_psets: HashMap::new(),
        };
        line.reconstruct_boundary_from_survey();
        line
    }

    /// Reconstructs the 2D boundary polygon from the survey table and start point.
    pub fn reconstruct_boundary_from_survey(&mut self) {
        if self.survey_table.is_empty() {
            return;
        }
        let mut pts = Vec::with_capacity(self.survey_table.len() + 1);
        let mut cur = self.start_point;
        pts.push(cur);
        for seg in &self.survey_table {
            cur = cur + seg.delta_mm();
            pts.push(cur);
        }
        self.boundary = pts;
    }

    /// Reconstructs survey bearing table from sketched boundary segments.
    pub fn reconstruct_survey_from_boundary(&mut self) {
        if self.boundary.len() < 2 {
            return;
        }
        let n = self.boundary.len();
        let mut table = Vec::new();
        for i in 0..n {
            let next = (i + 1) % n;
            let p0 = self.boundary[i];
            let p1 = self.boundary[next];
            let delta = p1 - p0;
            let dist_mm = delta.length();
            if dist_mm < 10.0 {
                continue;
            }
            let dist_m = dist_mm / 1000.0;
            let angle_rad = delta.x.atan2(delta.y); // radians from North (+Y)
            let mut deg = angle_rad.to_degrees();
            if deg < 0.0 {
                deg += 360.0;
            }
            let (quad, local_deg) = if deg >= 0.0 && deg < 90.0 {
                (BearingQuadrant::NorthEast, deg)
            } else if deg >= 90.0 && deg < 180.0 {
                (BearingQuadrant::SouthEast, 180.0 - deg)
            } else if deg >= 180.0 && deg < 270.0 {
                (BearingQuadrant::SouthWest, deg - 180.0)
            } else {
                (BearingQuadrant::NorthWest, 360.0 - deg)
            };
            let d_int = local_deg.floor() as u32;
            let rem_min = (local_deg - d_int as f64) * 60.0;
            let m_int = rem_min.floor() as u32;
            let sec = (rem_min - m_int as f64) * 60.0;

            table.push(SurveySegment::new(quad, d_int, m_int, sec, dist_m));
        }
        self.survey_table = table;
    }

    pub fn area_sqm(&self) -> f64 {
        if self.boundary.len() < 3 {
            return 0.0;
        }
        (plinth_geom::polygon::signed_area(&self.boundary).abs()) / 1_000_000.0
    }

    pub fn perimeter_m(&self) -> f64 {
        if self.boundary.len() < 2 {
            return 0.0;
        }
        let mut total_mm = 0.0;
        let n = self.boundary.len();
        for i in 0..n {
            let next = (i + 1) % n;
            total_mm += (self.boundary[next] - self.boundary[i]).length();
        }
        total_mm / 1000.0
    }

    pub fn closure_error_mm(&self) -> f64 {
        if self.boundary.len() < 3 {
            return 0.0;
        }
        let first = self.boundary[0];
        let last = self.boundary[self.boundary.len() - 1];
        (last - first).length()
    }

    pub fn closure_precision_ratio(&self) -> usize {
        let perim_mm = self.perimeter_m() * 1000.0;
        let err_mm = self.closure_error_mm();
        if err_mm < 0.1 {
            100_000
        } else {
            (perim_mm / err_mm).round() as usize
        }
    }

    pub fn from_sketch(boundary: Vec<Vec2>, level: LevelId) -> Self {
        Self::new_sketch(boundary, level)
    }

    pub fn from_survey_segments(table: Vec<SurveySegment>, start: Vec2, level: LevelId) -> Self {
        Self::new_survey(start, table, level)
    }
}

// ---------------------------------------------------------------------------
// Site Corridor Generator (Roads, Sidewalks, Pathwalks)
// ---------------------------------------------------------------------------

/// Offsets a polyline path symmetrically by width/2 on both sides to generate
/// a watertight, closed 2D polygon with clean mitered vertices.
pub fn corridor_polygon(path: &[Vec2], width: f64) -> Vec<Vec2> {
    if path.len() < 2 {
        return Vec::new();
    }
    let half_w = width * 0.5;
    let n = path.len();

    if n == 2 {
        let d = path[1] - path[0];
        let len = d.length();
        if len < 1e-4 {
            return Vec::new();
        }
        let dir = d * (1.0 / len);
        let perp = Vec2::new(-dir.y, dir.x) * half_w;
        let mut poly = vec![
            path[0] + perp,
            path[1] + perp,
            path[1] - perp,
            path[0] - perp,
        ];
        plinth_geom::polygon::make_ccw(&mut poly);
        return poly;
    }

    // Multi-segment polyline with mitered vertices
    let mut seg_perps = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let d = path[i + 1] - path[i];
        let len = d.length();
        let dir = if len > 1e-4 { d * (1.0 / len) } else { Vec2::X };
        let perp = Vec2::new(-dir.y, dir.x);
        seg_perps.push(perp);
    }

    let mut left_pts = Vec::with_capacity(n);
    let mut right_pts = Vec::with_capacity(n);

    // First vertex
    left_pts.push(path[0] + seg_perps[0] * half_w);
    right_pts.push(path[0] - seg_perps[0] * half_w);

    // Intermediate vertices with miter clamp
    for i in 1..n - 1 {
        let p1 = seg_perps[i - 1];
        let p2 = seg_perps[i];
        let sum = p1 + p2;
        let miter_dir = if sum.length_sq() > 1e-6 { sum.normalized() } else { p1 };
        let dot = miter_dir.dot(p1);
        let miter_len = if dot > 0.25 {
            (half_w / dot).min(half_w * 2.5)
        } else {
            half_w
        };
        left_pts.push(path[i] + miter_dir * miter_len);
        right_pts.push(path[i] - miter_dir * miter_len);
    }

    // Last vertex
    left_pts.push(path[n - 1] + seg_perps[n - 2] * half_w);
    right_pts.push(path[n - 1] - seg_perps[n - 2] * half_w);

    // Combine left and reversed right to form closed CCW polygon
    let mut poly = left_pts;
    right_pts.reverse();
    poly.extend(right_pts);

    let mut clean = plinth_geom::polygon::dedup_closed(&poly, 1.0);
    plinth_geom::polygon::make_ccw(&mut clean);
    clean
}

// ---------------------------------------------------------------------------
// Road
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoadCurb {
    None,
    Barrier,
    Mountable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub width: f64,
    pub lanes: u32,
    pub lane_width: f64,
    pub thickness: f64,
    pub curb_type: RoadCurb,
    pub curb_width: f64,
    pub curb_height: f64,
    pub crown_slope_pct: f64,
    pub has_centerline: bool,
    pub level: LevelId,
    pub base_offset: f64,
    pub road_type_name: String,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl Road {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Road 1".into(),
            path,
            width: 7300.0,
            lanes: 2,
            lane_width: 3650.0,
            thickness: 200.0,
            curb_type: RoadCurb::Barrier,
            curb_width: 150.0,
            curb_height: 150.0,
            crown_slope_pct: 2.0,
            has_centerline: true,
            level,
            base_offset: 0.0,
            road_type_name: "Two-Lane Asphalt Road (7.3m)".into(),
            ifc_psets: HashMap::new(),
        }
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        corridor_polygon(&self.path, self.width)
    }

    pub fn z_bounds(&self, level_z: f64) -> (f64, f64) {
        let z_top = level_z + self.base_offset;
        (z_top - self.thickness, z_top)
    }

    pub fn faces(&self, level_z: f64) -> Vec<Face> {
        let poly = self.footprint();
        if poly.len() < 3 {
            return Vec::new();
        }
        let (z_bot, z_top) = self.z_bounds(level_z);
        Prism::new(poly, z_bot, z_top).faces()
    }

    pub fn length_m(&self) -> f64 {
        if self.path.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for i in 0..self.path.len() - 1 {
            total += (self.path[i + 1] - self.path[i]).length();
        }
        total / 1000.0
    }
}

// ---------------------------------------------------------------------------
// Sidewalk
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SidewalkEdgeProfile {
    Square,
    Rounded { radius: f64 },
    Beveled { chamfer: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sidewalk {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub width: f64,
    pub thickness: f64,
    pub curb_height: f64,
    pub edge_profile: SidewalkEdgeProfile,
    pub has_top_groove: bool,
    pub groove_spacing: f64,
    pub groove_depth: f64,
    pub level: LevelId,
    pub base_offset: f64,
    pub sidewalk_type_name: String,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl Sidewalk {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Sidewalk 1".into(),
            path,
            width: 1800.0,
            thickness: 150.0,
            curb_height: 150.0,
            edge_profile: SidewalkEdgeProfile::Rounded { radius: 25.0 },
            has_top_groove: true,
            groove_spacing: 1500.0,
            groove_depth: 10.0,
            level,
            base_offset: 0.0,
            sidewalk_type_name: "Concrete Sidewalk with Bullnose Curb (1.8m)".into(),
            ifc_psets: HashMap::new(),
        }
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        corridor_polygon(&self.path, self.width)
    }

    pub fn z_bounds(&self, level_z: f64) -> (f64, f64) {
        let z_top = level_z + self.base_offset;
        (z_top - self.thickness, z_top)
    }

    pub fn faces(&self, level_z: f64) -> Vec<Face> {
        let poly = self.footprint();
        if poly.len() < 3 {
            return Vec::new();
        }
        let (z_bot, z_top) = self.z_bounds(level_z);
        Prism::new(poly, z_bot, z_top).faces()
    }

    pub fn length_m(&self) -> f64 {
        if self.path.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for i in 0..self.path.len() - 1 {
            total += (self.path[i + 1] - self.path[i]).length();
        }
        total / 1000.0
    }
}

// ---------------------------------------------------------------------------
// Pathwalk
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathwalkProfile {
    PaverBlocks,
    Cobblestone,
    Flagstone,
    GravelBed,
    TimberBoardwalk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pathwalk {
    pub id: ElementId,
    pub name: String,
    pub path: Vec<Vec2>,
    pub width: f64,
    pub thickness: f64,
    pub profile: PathwalkProfile,
    pub has_edging: bool,
    pub edging_width: f64,
    pub groove_width: f64,
    pub level: LevelId,
    pub base_offset: f64,
    pub pathwalk_type_name: String,
    pub ifc_psets: HashMap<String, HashMap<String, String>>,
}

impl Pathwalk {
    pub fn new(path: Vec<Vec2>, level: LevelId) -> Self {
        Self {
            id: ElementId::new(),
            name: "Pathwalk 1".into(),
            path,
            width: 1500.0,
            thickness: 100.0,
            profile: PathwalkProfile::PaverBlocks,
            has_edging: true,
            edging_width: 100.0,
            groove_width: 8.0,
            level,
            base_offset: 0.0,
            pathwalk_type_name: "Interlocking Paver Pathwalk (1.5m)".into(),
            ifc_psets: HashMap::new(),
        }
    }

    pub fn footprint(&self) -> Vec<Vec2> {
        corridor_polygon(&self.path, self.width)
    }

    pub fn z_bounds(&self, level_z: f64) -> (f64, f64) {
        let z_top = level_z + self.base_offset;
        (z_top - self.thickness, z_top)
    }

    pub fn faces(&self, level_z: f64) -> Vec<Face> {
        let poly = self.footprint();
        if poly.len() < 3 {
            return Vec::new();
        }
        let (z_bot, z_top) = self.z_bounds(level_z);
        Prism::new(poly, z_bot, z_top).faces()
    }

    pub fn length_m(&self) -> f64 {
        if self.path.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for i in 0..self.path.len() - 1 {
            total += (self.path[i + 1] - self.path[i]).length();
        }
        total / 1000.0
    }
}

// ---------------------------------------------------------------------------
// Unified Element Enum
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Element {
    Wall(Wall),
    Column(Column),
    Beam(Beam),
    Slab(Slab),
    Roof(Roof),
    Fascia(Fascia),
    Gutter(Gutter),
    Ceiling(Ceiling),
    Stair(Stair),
    Ramp(Ramp),
    Railing(Railing),
    Footing(Footing),
    Door(Door),
    Window(Window),
    CurtainWall(CurtainWall),
    BuildingPad(BuildingPad),
    Toposolid(Toposolid),
    PropertyLine(PropertyLine),
    Road(Road),
    Sidewalk(Sidewalk),
    Pathwalk(Pathwalk),
    Dimension(Dimension),
    DetailLine(DetailLine),
    TextNote(TextNote),
    Leader(Leader),
    FreeformSurface(FreeformSurface),
    FreeformSolid(FreeformSolid),
}

impl Element {
    pub fn id(&self) -> ElementId {
        match self {
            Element::Wall(e) => e.id,
            Element::Column(e) => e.id,
            Element::Beam(e) => e.id,
            Element::Slab(e) => e.id,
            Element::Roof(e) => e.id,
            Element::Fascia(e) => e.id,
            Element::Gutter(e) => e.id,
            Element::Ceiling(e) => e.id,
            Element::Stair(e) => e.id,
            Element::Ramp(e) => e.id,
            Element::Railing(e) => e.id,
            Element::Footing(e) => e.id,
            Element::Door(e) => e.id,
            Element::Window(e) => e.id,
            Element::CurtainWall(e) => e.id,
            Element::BuildingPad(e) => e.id,
            Element::Toposolid(e) => e.id,
            Element::PropertyLine(e) => e.id,
            Element::Road(e) => e.id,
            Element::Sidewalk(e) => e.id,
            Element::Pathwalk(e) => e.id,
            Element::Dimension(e) => e.id,
            Element::DetailLine(e) => e.id,
            Element::TextNote(e) => e.id,
            Element::Leader(e) => e.id,
            Element::FreeformSurface(e) => e.id,
            Element::FreeformSolid(e) => e.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Element::Wall(e) => &e.name,
            Element::Column(e) => &e.name,
            Element::Beam(e) => &e.name,
            Element::Slab(e) => &e.name,
            Element::Roof(e) => &e.name,
            Element::Fascia(e) => &e.name,
            Element::Gutter(e) => &e.name,
            Element::Ceiling(e) => &e.name,
            Element::Stair(e) => &e.name,
            Element::Ramp(e) => &e.name,
            Element::Railing(e) => &e.name,
            Element::Footing(e) => &e.name,
            Element::Door(e) => &e.name,
            Element::Window(e) => &e.name,
            Element::CurtainWall(e) => &e.name,
            Element::BuildingPad(e) => &e.name,
            Element::Toposolid(e) => &e.name,
            Element::PropertyLine(e) => &e.name,
            Element::Road(e) => &e.name,
            Element::Sidewalk(e) => &e.name,
            Element::Pathwalk(e) => &e.name,
            Element::Dimension(_) => "Dimension",
            Element::DetailLine(_) => "Detail Line",
            Element::TextNote(_) => "Text Annotation",
            Element::Leader(_) => "Leader Annotation",
            Element::FreeformSurface(e) => &e.name,
            Element::FreeformSolid(e) => &e.name,
        }
    }

    pub fn category_name(&self) -> &'static str {
        match self {
            Element::Wall(w) => if w.is_structural { "Structural Walls" } else { "Walls" },
            Element::Column(c) => if c.is_structural { "Structural Columns" } else { "Columns" },
            Element::Beam(_) => "Structural Framing",
            Element::Slab(s) => if s.is_structural { "Structural Floors" } else { "Floors" },
            Element::Roof(_) => "Roofs",
            Element::Fascia(_) => "Fascias",
            Element::Gutter(_) => "Gutters",
            Element::Ceiling(_) => "Ceilings",
            Element::Stair(_) => "Stairs",
            Element::Ramp(_) => "Ramps",
            Element::Railing(_) => "Railings",
            Element::Footing(_) => "Structural Foundations",
            Element::Door(_) => "Doors",
            Element::Window(_) => "Windows",
            Element::CurtainWall(_) => "Curtain Walls",
            Element::BuildingPad(_) => "Building Pads",
            Element::Toposolid(_) => "Toposolids",
            Element::PropertyLine(_) => "Property Lines",
            Element::Road(_) => "Roads",
            Element::Sidewalk(_) => "Sidewalks",
            Element::Pathwalk(_) => "Pathwalks",
            Element::Dimension(_) => "Dimensions",
            Element::DetailLine(_) => "Detail Lines",
            Element::TextNote(_) => "Text Annotations",
            Element::Leader(_) => "Leaders",
            Element::FreeformSurface(_) => "Freeform Surfaces",
            Element::FreeformSolid(_) => "Freeform Solids",
        }
    }

    pub fn level_id(&self) -> Option<LevelId> {
        match self {
            Element::Wall(e) => Some(e.base_level),
            Element::Column(e) => Some(e.base_level),
            Element::Beam(e) => Some(e.level),
            Element::Slab(e) => Some(e.level),
            Element::Roof(e) => Some(e.level),
            Element::Fascia(e) => Some(e.level),
            Element::Gutter(e) => Some(e.level),
            Element::Ceiling(e) => Some(e.level),
            Element::Stair(e) => Some(e.base_level),
            Element::Ramp(e) => Some(e.base_level),
            Element::Railing(e) => Some(e.level),
            Element::Footing(e) => Some(e.base_level),
            Element::FreeformSurface(e) => Some(e.level),
            Element::FreeformSolid(e) => Some(e.level),
            Element::Door(d) => d.base_level,
            Element::Window(w) => w.base_level,
            Element::CurtainWall(cw) => Some(cw.base_level),
            Element::BuildingPad(bp) => Some(bp.level),
            Element::Toposolid(ts) => Some(ts.level),
            Element::PropertyLine(pl) => Some(pl.level),
            Element::Road(r) => Some(r.level),
            Element::Sidewalk(sw) => Some(sw.level),
            Element::Pathwalk(pw) => Some(pw.level),
            Element::Dimension(_) | Element::DetailLine(_) | Element::TextNote(_) | Element::Leader(_) => None,
        }
    }

    pub fn is_structural(&self) -> bool {
        match self {
            Element::Wall(w) => w.is_structural,
            Element::Column(c) => c.is_structural,
            Element::Beam(_) => true,
            Element::Footing(_) => true,
            Element::Slab(s) => s.is_structural,
            Element::Roof(r) => r.is_structural,
            _ => false,
        }
    }

    /// Apply a 2D affine transformation (Move, Rotate, Mirror) in plan.
    pub fn transform2d(&mut self, xf: &Xform2) {
        match self {
            Element::Wall(w) => {
                w.start = xf.apply(w.start);
                w.end = xf.apply(w.end);
            }
            Element::Column(c) => {
                c.location = xf.apply(c.location);
                c.rotation = xf.apply_angle(c.rotation);
            }
            Element::Beam(b) => {
                b.start = xf.apply(b.start);
                b.end = xf.apply(b.end);
            }
            Element::Slab(s) => {
                for p in &mut s.boundary {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    s.boundary.reverse();
                }
            }
            Element::Roof(r) => {
                for p in &mut r.boundary {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    r.boundary.reverse();
                }
            }
            Element::Fascia(f) => {
                for p in &mut f.path {
                    *p = xf.apply(*p);
                }
            }
            Element::Gutter(g) => {
                for p in &mut g.path {
                    *p = xf.apply(*p);
                }
            }
            Element::Ceiling(c) => {
                for p in &mut c.boundary {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    c.boundary.reverse();
                }
            }
            Element::Stair(s) => {
                s.start = xf.apply(s.start);
                s.end = xf.apply(s.end);
            }
            Element::Ramp(r) => {
                r.start = xf.apply(r.start);
                r.end = xf.apply(r.end);
            }
            Element::Railing(rl) => {
                for p in &mut rl.path {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    rl.path.reverse();
                }
            }
            Element::Footing(f) => {
                f.location = xf.apply(f.location);
                f.rotation = xf.apply_angle(f.rotation);
            }
            Element::FreeformSurface(s) => {
                for p in &mut s.boundary {
                    *p = xf.apply(*p);
                }
                if let Some(ref mut b3) = s.boundary_3d {
                    for p in b3 {
                        let v2 = xf.apply(Vec2::new(p.x, p.y));
                        p.x = v2.x;
                        p.y = v2.y;
                    }
                }
                if xf.is_mirror() {
                    s.boundary.reverse();
                    if let Some(ref mut b3) = s.boundary_3d {
                        b3.reverse();
                    }
                }
            }
            Element::FreeformSolid(s) => {
                for p in &mut s.boundary {
                    *p = xf.apply(*p);
                }
                if let PrimitiveKind::Mesh { ref mut faces } = s.primitive {
                    for f in faces {
                        for v in &mut f.pts {
                            let v2 = xf.apply(Vec2::new(v.x, v.y));
                            v.x = v2.x;
                            v.y = v2.y;
                        }
                    }
                }
                if xf.is_mirror() {
                    s.boundary.reverse();
                }
            }
            Element::Door(d) => {
                // If mirroring, flip hand
                if xf.is_mirror() {
                    d.flipped_hand = !d.flipped_hand;
                }
            }
            Element::Window(_) => {}
            Element::CurtainWall(cw) => {
                cw.start = xf.apply(cw.start);
                cw.end = xf.apply(cw.end);
            }
            Element::BuildingPad(bp) => {
                for p in &mut bp.boundary {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    bp.boundary.reverse();
                }
            }
            Element::Toposolid(ts) => {
                for p in &mut ts.boundary {
                    *p = xf.apply(*p);
                }
                for ep in &mut ts.elevation_points {
                    let v2 = xf.apply(Vec2::new(ep.x, ep.y));
                    ep.x = v2.x;
                    ep.y = v2.y;
                }
                if xf.is_mirror() {
                    ts.boundary.reverse();
                }
            }
            Element::PropertyLine(pl) => {
                for p in &mut pl.boundary {
                    *p = xf.apply(*p);
                }
                pl.start_point = xf.apply(pl.start_point);
                if xf.is_mirror() {
                    pl.boundary.reverse();
                }
                pl.reconstruct_survey_from_boundary();
            }
            Element::Road(rd) => {
                for p in &mut rd.path {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    rd.path.reverse();
                }
            }
            Element::Sidewalk(sw) => {
                for p in &mut sw.path {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    sw.path.reverse();
                }
            }
            Element::Pathwalk(pw) => {
                for p in &mut pw.path {
                    *p = xf.apply(*p);
                }
                if xf.is_mirror() {
                    pw.path.reverse();
                }
            }
            Element::Dimension(d) => {
                d.p1 = xf.apply(d.p1);
                d.p2 = xf.apply(d.p2);
                if let Some(ref mut p3) = d.p3 {
                    *p3 = xf.apply(*p3);
                }
            }
            Element::DetailLine(l) => {
                l.start = xf.apply(l.start);
                l.end = xf.apply(l.end);
            }
            Element::TextNote(t) => {
                t.pos = xf.apply(t.pos);
            }
            Element::Leader(ld) => {
                ld.target_pt = xf.apply(ld.target_pt);
                ld.shoulder_pt = xf.apply(ld.shoulder_pt);
                ld.text_pt = xf.apply(ld.text_pt);
            }
        }
    }

    /// Translate element vertically along Z axis (elevation offset).
    pub fn translate_z(&mut self, dz: f64) {
        match self {
            Element::Wall(w) => w.base_offset += dz,
            Element::Column(c) => c.base_offset += dz,
            Element::Beam(b) => b.z_offset += dz,
            Element::Slab(s) => s.offset += dz,
            Element::Roof(r) => r.offset += dz,
            Element::Fascia(f) => f.vertical_offset += dz,
            Element::Gutter(g) => g.vertical_offset += dz,
            Element::Ceiling(c) => c.height_above_level += dz,
            Element::Stair(s) => s.base_offset += dz,
            Element::Ramp(r) => r.base_offset += dz,
            Element::Railing(rl) => rl.base_offset += dz,
            Element::Footing(f) => f.base_offset += dz,
            Element::FreeformSurface(s) => {
                s.elevation += dz;
                s.base_offset += dz;
                if let Some(ref mut b3) = s.boundary_3d {
                    for p in b3 {
                        p.z += dz;
                    }
                }
            }
            Element::FreeformSolid(s) => {
                s.base_offset += dz;
                s.z_min += dz;
                s.z_max += dz;
                if let PrimitiveKind::Mesh { ref mut faces } = s.primitive {
                    for f in faces {
                        for v in &mut f.pts {
                            v.z += dz;
                        }
                    }
                }
            }
            Element::Door(d) => d.sill_height += dz,
            Element::Window(w) => w.sill_height += dz,
            Element::CurtainWall(cw) => cw.base_offset += dz,
            Element::BuildingPad(bp) => bp.elevation_offset += dz,
            Element::Toposolid(ts) => {
                for ep in &mut ts.elevation_points {
                    ep.z += dz;
                }
            }
            Element::PropertyLine(_) => {}
            Element::Road(rd) => rd.base_offset += dz,
            Element::Sidewalk(sw) => sw.base_offset += dz,
            Element::Pathwalk(pw) => pw.base_offset += dz,
            Element::Dimension(_) => {}
            Element::DetailLine(_) => {}
            Element::TextNote(_) => {}
            Element::Leader(_) => {}
        }
    }

    /// Translate element in 3D world coordinates by (dx, dy, dz).
    pub fn translate_3d(&mut self, delta: plinth_geom::Vec3) {
        if delta.x != 0.0 || delta.y != 0.0 {
            self.transform2d(&Xform2::translation(plinth_geom::vec2(delta.x, delta.y)));
        }
        if delta.z != 0.0 {
            self.translate_z(delta.z);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_geom::vec2;

    #[test]
    fn test_surface_creation_and_push_pull_extrusion() {
        let lvl = LevelId::new();
        // 1. Create 2D Rectangle Surface (4000mm x 5000mm at elevation 0mm)
        let surf = FreeformSurface::new_rect(vec2(0.0, 0.0), vec2(4000.0, 5000.0), 0.0, lvl);
        assert_eq!(surf.boundary.len(), 4);
        assert_eq!(surf.area(), 20_000_000.0); // 20 m²

        // 2. Extrude (Push/Pull) by 3000mm into 3D Solid
        let mut solid = surf.extrude(3000.0);
        assert_eq!(solid.height(), 3000.0);
        assert_eq!(solid.volume(), 60_000_000_000.0); // 60 m³

        // 3. Push/Pull solid top face by +500mm
        solid.push_pull(500.0);
        assert_eq!(solid.height(), 3500.0);
        assert_eq!(solid.volume(), 70_000_000_000.0); // 70 m³
    }

    #[test]
    fn test_3d_primitives() {
        let lvl = LevelId::new();
        let b = FreeformSolid::new_box(vec2(0.0, 0.0), vec2(2000.0, 2000.0), 0.0, 2000.0, lvl);
        assert_eq!(b.height(), 2000.0);
        assert_eq!(b.volume(), 8_000_000_000.0); // 8 m³

        let cyl = FreeformSolid::new_cylinder(vec2(0.0, 0.0), 1000.0, 0.0, 3000.0, 24, lvl);
        assert_eq!(cyl.height(), 3000.0);
        assert!(cyl.volume() > 0.0);

        let sph = FreeformSolid::new_sphere(vec2(0.0, 0.0), 1000.0, 1000.0, lvl);
        assert_eq!(sph.height(), 2000.0);
    }

    #[test]
    fn test_roof_shapes_and_ifc_data() {
        let lvl = LevelId::new();
        let pts = vec![
            vec2(0.0, 0.0),
            vec2(10000.0, 0.0),
            vec2(10000.0, 6000.0),
            vec2(0.0, 6000.0),
        ];
        let mut roof = Roof::new(pts, lvl);
        assert_eq!(roof.shape, RoofShape::Hip);
        assert_eq!(roof.tag, "RF-01");
        assert!(roof.is_external);

        // 1. Hip Roof: 4 slopes meeting at a ridge, watertight solid faces
        let hip_faces = roof.faces(3000.0);
        assert!(!hip_faces.is_empty());
        assert!(hip_faces.len() >= 8); // top slopes, soffits, and eave fascias

        // 2. Gable Roof
        roof.shape = RoofShape::Gable;
        let gable_faces = roof.faces(3000.0);
        assert_eq!(gable_faces.len(), 10); // 2 slopes + 2 soffits + 2 eave fascias + 4 rake bargeboards

        // 3. Shed Roof (mono-pitch)
        roof.shape = RoofShape::Shed;
        let shed_faces = roof.faces(3000.0);
        assert_eq!(shed_faces.len(), 6); // top + soffit + 4 perimeter fascias/rakes

        // 4. Flat Roof
        roof.shape = RoofShape::Flat;
        let flat_faces = roof.faces(3000.0);
        assert_eq!(flat_faces.len(), 6); // top cap + bottom cap + 4 rim fascias
    }

    #[test]
    fn test_mesh_primitive_solid() {
        let lvl = LevelId::new();
        // Tetrahedron or pyramid mesh
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(2000.0, 0.0, 0.0);
        let p2 = Vec3::new(1000.0, 2000.0, 0.0);
        let apex = Vec3::new(1000.0, 1000.0, 3000.0);

        let f_base = Face::solid(vec![p0, p2, p1]);
        let f_side1 = Face::solid(vec![p0, p1, apex]);
        let f_side2 = Face::solid(vec![p1, p2, apex]);
        let f_side3 = Face::solid(vec![p2, p0, apex]);

        let mesh_solid = FreeformSolid::new_mesh(
            "Tetrahedron".to_string(),
            vec![f_base, f_side1, f_side2, f_side3],
            lvl,
        );

        assert!(mesh_solid.is_mesh());
        assert_eq!(mesh_solid.height(), 3000.0);
        assert_eq!(mesh_solid.z_min, 0.0);
        assert_eq!(mesh_solid.z_max, 3000.0);
        assert_eq!(mesh_solid.faces().len(), 4);
        assert!(mesh_solid.volume() > 0.0);
        assert!(mesh_solid.surface_area() > 0.0);
    }

    #[test]
    fn test_ramp_and_stair_solid_faces_and_level_offsets() {
        let lvl1 = LevelId::new();
        let lvl2 = LevelId::new();

        // 1. Ramp generates smooth inclined solid faces (never 12 stair steps!)
        let mut ramp = Ramp::new(vec2(0.0, 0.0), vec2(7200.0, 0.0), lvl1);
        ramp.base_offset = 100.0;
        ramp.top_level = Some(lvl2);
        ramp.top_offset = -200.0;
        let (rz0, rz1) = ramp.z_bounds(0.0, Some(900.0));
        assert_eq!(rz0, 100.0);
        assert_eq!(rz1, 700.0); // 900 - 200
        let r_faces = ramp.faces(0.0, Some(900.0));
        assert_eq!(r_faces.len(), 6); // top deck + bottom toe + soffit + back + 2 convex side cheeks
        for f in &r_faces {
            assert!(f.pts.len() >= 3 && f.pts.len() <= 4);
            assert!(f.normal.length() > 0.99);
        }

        // SolidWedge ramp has 5 faces (deck + base + back + 2 triangular cheeks)
        ramp.ramp_style = RampStyle::SolidWedge;
        assert_eq!(ramp.faces(0.0, Some(900.0)).len(), 5);

        // 2. Stair generates convex solid treads, risers, soffit, and seam-free side cheeks
        let mut stair = Stair::new(vec2(0.0, 0.0), vec2(4500.0, 0.0), lvl1);
        stair.base_offset = -50.0;
        stair.top_level = Some(lvl2);
        stair.top_offset = 100.0;
        let (sz0, sz1) = stair.z_bounds(0.0, Some(3000.0));
        assert_eq!(sz0, -50.0);
        assert_eq!(sz1, 3100.0);
        let s_faces = stair.faces(0.0, Some(3000.0));
        assert!(s_faces.len() > 36);
        for f in &s_faces {
            assert_eq!(f.pts.len(), 4);
            assert!(f.normal.length() > 0.99);
        }
    }

    #[test]
    fn test_custom_concave_slab_and_ceiling_triangulation() {
        let lvl = LevelId::new();
        // V-notched floor slab
        let v_notch_boundary = vec![
            vec2(0.0, 0.0),
            vec2(4000.0, 0.0),
            vec2(4000.0, 3000.0),
            vec2(2000.0, 1500.0), // Indented V-notch
            vec2(0.0, 3000.0),
        ];
        let slab = Slab::new(v_notch_boundary.clone(), lvl);
        let faces = slab.faces(0.0, None);

        // 5 perimeter side rim faces + 3 top cap triangles + 3 bottom cap triangles = 11 faces
        assert_eq!(faces.len(), 5 + 3 + 3);

        // Verify top cap triangles: centroids must all be inside the slab boundary
        let top_faces: Vec<&Face> = faces.iter().filter(|f| f.normal.z > 0.9).collect();
        assert_eq!(top_faces.len(), 3);
        for f in &top_faces {
            assert_eq!(f.pts.len(), 3);
            let c = (f.pts[0] + f.pts[1] + f.pts[2]) / 3.0;
            assert!(
                plinth_geom::polygon::point_in_polygon(vec2(c.x, c.y), &v_notch_boundary),
                "Top cap triangle centroid {:?} is outside the slab in the notch!",
                c
            );
        }

        // Verify ceiling with L-shape boundary
        let l_shape = vec![
            vec2(0.0, 0.0),
            vec2(6000.0, 0.0),
            vec2(6000.0, 3000.0),
            vec2(3000.0, 3000.0),
            vec2(3000.0, 6000.0),
            vec2(0.0, 6000.0),
        ];
        let ceiling = Ceiling::new(l_shape.clone(), lvl);
        let c_faces = ceiling.faces(0.0, None);
        // 6 perimeter faces + 4 top triangles + 4 bottom triangles = 14 faces
        assert_eq!(c_faces.len(), 6 + 4 + 4);
        let bot_faces: Vec<&Face> = c_faces.iter().filter(|f| f.normal.z < -0.9).collect();
        assert_eq!(bot_faces.len(), 4);
        for f in &bot_faces {
            assert_eq!(f.pts.len(), 3);
            let c = (f.pts[0] + f.pts[1] + f.pts[2]) / 3.0;
            assert!(
                plinth_geom::polygon::point_in_polygon(vec2(c.x, c.y), &l_shape),
                "Ceiling cap triangle centroid {:?} is outside the L-shape!",
                c
            );
        }
    }

    #[test]
    fn test_site_elements_building_pad_toposolid_property_line() {
        let lvl = LevelId::new();

        // 1. Building Pad
        let pad_pts = vec![
            vec2(0.0, 0.0),
            vec2(10000.0, 0.0),
            vec2(10000.0, 8000.0),
            vec2(0.0, 8000.0),
        ];
        let pad = BuildingPad::new(pad_pts.clone(), lvl);
        assert_eq!(pad.area(), 80_000_000.0); // 80 m²
        let pad_faces = pad.faces(0.0);
        assert_eq!(pad_faces.len(), 6); // 4 side faces + top + bottom

        // 2. Toposolid with Spot Elevations and Contours
        let topo_pts = vec![
            vec2(-5000.0, -5000.0),
            vec2(25000.0, -5000.0),
            vec2(25000.0, 20000.0),
            vec2(-5000.0, 20000.0),
        ];
        let mut topo = Toposolid::new(topo_pts, lvl);
        topo.elevation_points = vec![
            ToposolidPoint { x: 0.0, y: 0.0, z: 500.0 },
            ToposolidPoint { x: 10000.0, y: 5000.0, z: 2500.0 },
            ToposolidPoint { x: 20000.0, y: 15000.0, z: 4000.0 },
        ];
        let topo_faces = topo.faces(0.0);
        assert!(!topo_faces.is_empty());
        let contours = topo.compute_contour_lines();
        assert!(!contours.is_empty());

        // 3. Property Line with Metes and Bounds
        let table = vec![
            SurveySegment::new(BearingQuadrant::NorthEast, 90, 0, 0.0, 50.0), // East 50m
            SurveySegment::new(BearingQuadrant::NorthWest, 0, 0, 0.0, 30.0),  // North 30m
            SurveySegment::new(BearingQuadrant::SouthWest, 90, 0, 0.0, 50.0), // West 50m
            SurveySegment::new(BearingQuadrant::SouthEast, 0, 0, 0.0, 30.0),  // South 30m
        ];
        let prop = PropertyLine::new_survey(vec2(0.0, 0.0), table, lvl);
        assert_eq!(prop.boundary.len(), 5);
        assert!((prop.area_sqm() - 1500.0).abs() < 1.0); // 50m * 30m = 1500 m²
        assert!((prop.perimeter_m() - 160.0).abs() < 1.0);
        assert!(prop.closure_error_mm() < 1.0);
    }
}
