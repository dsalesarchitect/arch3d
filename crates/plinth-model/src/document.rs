//! Central Document: single source of truth for all BIM elements, levels, grids, materials, and views.

use crate::element::{
    Beam, BuildingPad, Ceiling, Column, CurtainWall, Dimension, Door, Element, FontStyle, Leader,
    Pathwalk, Railing, Road, Roof, RoofShape, Sidewalk, Slab, Stair, TextNote, TickMark, Toposolid,
    ToposolidPoint, Wall, WallOpening, Window,
};
use crate::grid::GridLine;
use crate::id::{ElementId, GridId, LevelId, MaterialId, SheetId, ViewId, ViewportId};
use crate::level::Level;
use crate::material::Material;
use crate::view_def::{ViewDef, ViewKind};
use plinth_geom::{vec2, Face, Prism, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSize {
    A1,
    A2,
    A3,
    ArchD,
}

impl PaperSize {
    /// Paper dimensions in millimeters (width, height) in landscape orientation
    pub fn dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::A1 => (841.0, 594.0),
            Self::A2 => (594.0, 420.0),
            Self::A3 => (420.0, 297.0),
            Self::ArchD => (914.4, 609.6), // 36" x 24" standard architectural sheet
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::A1 => "A1 (841 x 594 mm)",
            Self::A2 => "A2 (594 x 420 mm)",
            Self::A3 => "A3 (420 x 297 mm)",
            Self::ArchD => "Arch D (36 x 24 in)",
        }
    }
}

fn default_vp_width() -> f64 {
    340.0
}

fn default_vp_height() -> f64 {
    240.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedViewport {
    pub id: ViewportId,
    pub view_id: ViewId,
    pub position_mm: Vec2, // Position on the sheet paper in mm from bottom-left
    #[serde(default = "default_vp_width")]
    pub width_mm: f64,     // Viewport width on paper in mm
    #[serde(default = "default_vp_height")]
    pub height_mm: f64,    // Viewport height on paper in mm
    pub scale: u32,        // Scale denominator, e.g. 50, 100, 200
    pub title: String,     // Drawing title, e.g. "1 - FLOOR PLAN: LEVEL 1"
    pub number: u32,       // Detail/drawing number, e.g. 1
    pub show_title: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub id: SheetId,
    pub sheet_number: String, // e.g. "A101"
    pub name: String,         // e.g. "ARCHITECTURAL OVERVIEW"
    pub paper_size: PaperSize,
    pub viewports: Vec<PlacedViewport>,
}

impl Sheet {
    pub fn new(sheet_number: impl Into<String>, name: impl Into<String>, paper_size: PaperSize) -> Self {
        Self {
            id: SheetId::new(),
            sheet_number: sheet_number.into(),
            name: name.into(),
            paper_size,
            viewports: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub number: String,
    pub client: String,
    pub address: String,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            name: "Commercial Office Building".into(),
            number: "PRJ-2026-001".into(),
            client: "Acme Development Group".into(),
            address: "100 Architecture Plaza".into(),
        }
    }
}

/// The Single Source of Truth BIM document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub project_info: ProjectInfo,
    pub levels: BTreeMap<LevelId, Level>,
    pub grids: BTreeMap<GridId, GridLine>,
    pub materials: BTreeMap<MaterialId, Material>,
    pub elements: BTreeMap<ElementId, Element>,
    pub views: BTreeMap<ViewId, ViewDef>,
    pub sheets: BTreeMap<SheetId, Sheet>,
    pub active_view_id: ViewId,
    pub active_level_id: LevelId,
    pub active_sheet_id: Option<SheetId>,

    #[serde(skip)]
    undo_stack: Vec<DocumentSnapshot>,
    #[serde(skip)]
    redo_stack: Vec<DocumentSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
struct DocumentSnapshot {
    elements: BTreeMap<ElementId, Element>,
    levels: BTreeMap<LevelId, Level>,
    grids: BTreeMap<GridId, GridLine>,
    materials: BTreeMap<MaterialId, Material>,
    views: BTreeMap<ViewId, ViewDef>,
    sheets: BTreeMap<SheetId, Sheet>,
    active_level_id: LevelId,
}

impl Document {
    pub fn new_empty() -> Self {
        let mut doc = Self {
            project_info: ProjectInfo::default(),
            levels: BTreeMap::new(),
            grids: BTreeMap::new(),
            materials: BTreeMap::new(),
            elements: BTreeMap::new(),
            views: BTreeMap::new(),
            sheets: BTreeMap::new(),
            active_view_id: ViewId::nil(),
            active_level_id: LevelId::nil(),
            active_sheet_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };

        // Add default materials
        let mat_conc = Material::default_concrete();
        let mat_masonry = Material::default_masonry();
        let mat_steel = Material::default_steel();
        let mat_glass = Material::default_glass();
        let mat_drywall = Material::default_drywall();
        let mat_timber = Material::default_timber();

        doc.materials.insert(mat_conc.id, mat_conc);
        doc.materials.insert(mat_masonry.id, mat_masonry);
        doc.materials.insert(mat_steel.id, mat_steel);
        doc.materials.insert(mat_glass.id, mat_glass);
        doc.materials.insert(mat_drywall.id, mat_drywall);
        doc.materials.insert(mat_timber.id, mat_timber);

        // Add default levels
        let lvl1 = Level::new("Level 1", 0.0, 3500.0);
        let lvl2 = Level::new("Level 2", 3500.0, 3500.0);
        let lvl_roof = Level::new("Roof", 7000.0, 0.0);

        let lvl1_id = lvl1.id;
        let lvl2_id = lvl2.id;
        let lvl_roof_id = lvl_roof.id;

        doc.levels.insert(lvl1_id, lvl1);
        doc.levels.insert(lvl2_id, lvl2);
        doc.levels.insert(lvl_roof_id, lvl_roof);
        doc.active_level_id = lvl1_id;

        // Default standard views (Plan views, Ceiling plans, Elevations, Sections, 3D)
        let v_l1 = ViewDef::new_floor_plan("Floor Plan: Level 1", lvl1_id);
        let v_l2 = ViewDef::new_floor_plan("Floor Plan: Level 2", lvl2_id);
        let v_rcp1 = ViewDef::new_ceiling_plan("Ceiling Plan: Level 1", lvl1_id);
        let v_rcp2 = ViewDef::new_ceiling_plan("Ceiling Plan: Level 2", lvl2_id);
        let v_north = ViewDef::new_elevation("Elevation: North", ViewKind::ElevationNorth);
        let v_south = ViewDef::new_elevation("Elevation: South", ViewKind::ElevationSouth);
        let v_east = ViewDef::new_elevation("Elevation: East", ViewKind::ElevationEast);
        let v_west = ViewDef::new_elevation("Elevation: West", ViewKind::ElevationWest);
        let v_section = ViewDef::new_section("Section 1", vec2(-2000.0, 4000.0), vec2(14000.0, 4000.0));
        let v_3d = ViewDef::new_3d("{3D}");

        let active_id = v_l1.id;
        doc.views.insert(v_l1.id, v_l1);
        doc.views.insert(v_l2.id, v_l2);
        doc.views.insert(v_rcp1.id, v_rcp1);
        doc.views.insert(v_rcp2.id, v_rcp2);
        doc.views.insert(v_north.id, v_north);
        doc.views.insert(v_south.id, v_south);
        doc.views.insert(v_east.id, v_east);
        doc.views.insert(v_west.id, v_west);
        doc.views.insert(v_section.id, v_section);
        doc.views.insert(v_3d.id, v_3d);

        doc.active_view_id = active_id;
        doc
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Record an undo state prior to making a mutation.
    pub fn push_undo(&mut self) {
        let snapshot = DocumentSnapshot {
            elements: self.elements.clone(),
            levels: self.levels.clone(),
            grids: self.grids.clone(),
            materials: self.materials.clone(),
            views: self.views.clone(),
            sheets: self.sheets.clone(),
            active_level_id: self.active_level_id,
        };
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let curr = DocumentSnapshot {
                elements: self.elements.clone(),
                levels: self.levels.clone(),
                grids: self.grids.clone(),
                materials: self.materials.clone(),
                views: self.views.clone(),
                sheets: self.sheets.clone(),
                active_level_id: self.active_level_id,
            };
            self.redo_stack.push(curr);
            self.elements = prev.elements;
            self.levels = prev.levels;
            self.grids = prev.grids;
            self.materials = prev.materials;
            self.views = prev.views;
            self.sheets = prev.sheets;
            self.active_level_id = prev.active_level_id;
            if !self.levels.contains_key(&self.active_level_id) {
                if let Some(&first_lid) = self.levels.keys().next() {
                    self.active_level_id = first_lid;
                }
            }
            if !self.views.contains_key(&self.active_view_id) {
                if let Some(&first_vid) = self.views.keys().next() {
                    self.active_view_id = first_vid;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let curr = DocumentSnapshot {
                elements: self.elements.clone(),
                levels: self.levels.clone(),
                grids: self.grids.clone(),
                materials: self.materials.clone(),
                views: self.views.clone(),
                sheets: self.sheets.clone(),
                active_level_id: self.active_level_id,
            };
            self.undo_stack.push(curr);
            self.elements = next.elements;
            self.levels = next.levels;
            self.grids = next.grids;
            self.materials = next.materials;
            self.views = next.views;
            self.sheets = next.sheets;
            self.active_level_id = next.active_level_id;
            if !self.levels.contains_key(&self.active_level_id) {
                if let Some(&first_lid) = self.levels.keys().next() {
                    self.active_level_id = first_lid;
                }
            }
            if !self.views.contains_key(&self.active_view_id) {
                if let Some(&first_vid) = self.views.keys().next() {
                    self.active_view_id = first_vid;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn level_elevation(&self, level_id: LevelId) -> f64 {
        self.levels.get(&level_id).map(|l| l.elevation).unwrap_or(0.0)
    }

    pub fn hosted_openings_for_wall(&self, wall_id: ElementId) -> Vec<WallOpening> {
        let wall_z0 = match self.elements.get(&wall_id) {
            Some(Element::Wall(w)) => {
                let bz = self.level_elevation(w.base_level);
                let tz = w.top_level.map(|lvl| self.level_elevation(lvl));
                w.z_bounds(bz, tz).0
            }
            _ => 0.0,
        };

        let mut openings = Vec::new();
        for el in self.elements.values() {
            match el {
                Element::Door(d) if d.host_wall == wall_id => {
                    let base_z = d
                        .base_level
                        .map(|lvl| self.level_elevation(lvl))
                        .unwrap_or(wall_z0);
                    let top_z = d.top_level.map(|lvl| self.level_elevation(lvl));
                    let (z0, z1) = d.z_bounds(base_z, top_z);
                    openings.push(WallOpening {
                        offset_along: d.offset_along_wall,
                        width: d.width,
                        height: (z1 - z0).max(100.0),
                        sill_height: z0 - wall_z0,
                    });
                }
                Element::Window(w) if w.host_wall == wall_id => {
                    let base_z = w
                        .base_level
                        .map(|lvl| self.level_elevation(lvl))
                        .unwrap_or(wall_z0);
                    let top_z = w.top_level.map(|lvl| self.level_elevation(lvl));
                    let (z0, z1) = w.z_bounds(base_z, top_z);
                    openings.push(WallOpening {
                        offset_along: w.offset_along_wall,
                        width: w.width,
                        height: (z1 - z0).max(100.0),
                        sill_height: z0 - wall_z0,
                    });
                }
                Element::CurtainWall(cw) if cw.automatically_embed => {
                    let is_hosted = cw.host_wall == Some(wall_id);
                    let is_collinear = if cw.host_wall.is_none() {
                        if let Some(Element::Wall(hw)) = self.elements.get(&wall_id) {
                            let hw_dir = hw.direction();
                            let hw_norm = hw.normal();
                            cw.direction().dot(hw_dir).abs() > 0.98
                                && (cw.start - hw.start).dot(hw_norm).abs() < 50.0
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_hosted || is_collinear {
                        if let Some(Element::Wall(hw)) = self.elements.get(&wall_id) {
                            let hw_dir = hw.direction();
                            let s_dist = (cw.start - hw.start).dot(hw_dir);
                            let e_dist = (cw.end - hw.start).dot(hw_dir);
                            let s = s_dist.min(e_dist).max(0.0);
                            let e = s_dist.max(e_dist).min(hw.length());
                            if e > s + 1.0 {
                                let cw_base_z = self.level_elevation(cw.base_level);
                                let cw_top_z = cw.top_level.map(|lvl| self.level_elevation(lvl));
                                let (z0, z1) = cw.z_bounds(cw_base_z, cw_top_z);
                                openings.push(WallOpening {
                                    offset_along: (s + e) * 0.5,
                                    width: e - s,
                                    height: (z1 - z0).max(100.0),
                                    sill_height: z0 - wall_z0,
                                });
                            }
                        }
                    }
                }
                Element::FreeformSurface(s) if s.is_hole && s.host_wall == Some(wall_id) => {
                    if let Some(Element::Wall(hw)) = self.elements.get(&wall_id) {
                        let hw_dir = hw.direction();
                        if let Some(pts) = &s.boundary_3d {
                            let mut min_dist = f64::MAX;
                            let mut max_dist = f64::MIN;
                            let mut min_z = f64::MAX;
                            let mut max_z = f64::MIN;
                            for p in pts {
                                let p2 = plinth_geom::vec2(p.x, p.y);
                                let d = (p2 - hw.start).dot(hw_dir);
                                min_dist = min_dist.min(d);
                                max_dist = max_dist.max(d);
                                min_z = min_z.min(p.z);
                                max_z = max_z.max(p.z);
                            }
                            let s_along = min_dist.max(0.0);
                            let e_along = max_dist.min(hw.length());
                            if e_along > s_along + 1.0 && max_z > min_z + 1.0 {
                                openings.push(WallOpening {
                                    offset_along: (s_along + e_along) * 0.5,
                                    width: e_along - s_along,
                                    height: max_z - min_z,
                                    sill_height: min_z - wall_z0,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        openings
    }

    /// Generates 3D solid representation(s) for a specific element.
    pub fn element_solids(&self, id: ElementId) -> Vec<Prism> {
        let Some(el) = self.elements.get(&id) else {
            return Vec::new();
        };

        match el {
            Element::Wall(w) => {
                let base_z = self.level_elevation(w.base_level);
                let top_z = w.top_level.map(|lvl| self.level_elevation(lvl));
                let openings = self.hosted_openings_for_wall(w.id);
                let all_walls: Vec<&Wall> = self.elements.values().filter_map(|e| match e {
                    Element::Wall(other) => Some(other),
                    _ => None,
                }).collect();
                w.build_solids_with_joins(base_z, top_z, &openings, &all_walls)
            }
            Element::Column(c) => {
                let base_z = self.level_elevation(c.base_level);
                let top_z = c.top_level.map(|lvl| self.level_elevation(lvl));
                vec![c.solid(base_z, top_z)]
            }
            Element::Beam(b) => {
                let base_z = self.level_elevation(b.level);
                let top_z = b.top_level.map(|lvl| self.level_elevation(lvl));
                vec![b.solid_with_bounds(base_z, top_z)]
            }
            Element::Slab(s) => {
                let base_z = self.level_elevation(s.level);
                let top_z = s.top_level.map(|lvl| self.level_elevation(lvl));
                vec![s.solid_with_bounds(base_z, top_z)]
            }
            Element::Roof(r) => {
                let base_z = self.level_elevation(r.level);
                let top_z = r.top_level.map(|lvl| self.level_elevation(lvl));
                vec![r.solid_with_bounds(base_z, top_z)]
            }
            Element::Fascia(f) => {
                let base_z = self.level_elevation(f.level);
                let top_z = f.top_level.map(|lvl| self.level_elevation(lvl));
                f.solids_with_bounds(base_z, top_z)
            }
            Element::Gutter(g) => {
                let base_z = self.level_elevation(g.level);
                let top_z = g.top_level.map(|lvl| self.level_elevation(lvl));
                g.solids_with_bounds(base_z, top_z)
            }
            Element::Ceiling(c) => {
                let base_z = self.level_elevation(c.level);
                let top_z = c.top_level.map(|lvl| self.level_elevation(lvl));
                vec![c.solid_with_bounds(base_z, top_z)]
            }
            Element::Stair(s) => {
                let base_z = self.level_elevation(s.base_level);
                let top_z = s.top_level.map(|lvl| self.level_elevation(lvl));
                s.solids_with_bounds(base_z, top_z)
            }
            Element::Ramp(r) => {
                let base_z = self.level_elevation(r.base_level);
                let top_z = r.top_level.map(|lvl| self.level_elevation(lvl));
                r.solids_with_bounds(base_z, top_z)
            }
            Element::Railing(rl) => {
                let base_z = self.level_elevation(rl.level);
                let top_z = rl.top_level.map(|lvl| self.level_elevation(lvl));
                rl.solids_with_bounds(base_z, top_z)
            }
            Element::Footing(f) => {
                let base_z = self.level_elevation(f.base_level);
                let top_z = f.top_level.map(|lvl| self.level_elevation(lvl));
                f.solids_with_bounds(base_z, top_z)
            }
            Element::FreeformSolid(f) => {
                if f.group_name.as_deref() == Some("Space") || f.ifc_category.as_deref() == Some("IFCSPACE") {
                    Vec::new()
                } else if f.is_sphere() || f.is_mesh() {
                    Vec::new()
                } else {
                    vec![f.solid()]
                }
            }
            Element::Window(win) => {
                // Glass pane solid hosted inside wall opening
                if let Some(Element::Wall(w)) = self.elements.get(&win.host_wall) {
                    let wall_z0 = w.z_bounds(
                        self.level_elevation(w.base_level),
                        w.top_level.map(|lvl| self.level_elevation(lvl)),
                    ).0;
                    let base_z = win.base_level.map(|lvl| self.level_elevation(lvl)).unwrap_or(wall_z0);
                    let top_z = win.top_level.map(|lvl| self.level_elevation(lvl));
                    let (z_bot, z_top) = win.z_bounds(base_z, top_z);
                    let dir = w.direction();
                    let op_start = (win.offset_along_wall - win.width * 0.5).max(0.0);
                    let op_end = (win.offset_along_wall + win.width * 0.5).min(w.length());
                    let p0 = w.start + dir * op_start;
                    let p1 = w.start + dir * op_end;
                    vec![Prism::along(p0, p1, 40.0, z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::Door(d) => {
                // Door panel solid hosted inside wall opening
                if let Some(Element::Wall(w)) = self.elements.get(&d.host_wall) {
                    let wall_z0 = w.z_bounds(
                        self.level_elevation(w.base_level),
                        w.top_level.map(|lvl| self.level_elevation(lvl)),
                    ).0;
                    let base_z = d.base_level.map(|lvl| self.level_elevation(lvl)).unwrap_or(wall_z0);
                    let top_z = d.top_level.map(|lvl| self.level_elevation(lvl));
                    let (z_bot, z_top) = d.z_bounds(base_z, top_z);
                    let dir = w.direction();
                    let op_start = (d.offset_along_wall - d.width * 0.5).max(0.0);
                    let op_end = (d.offset_along_wall + d.width * 0.5).min(w.length());
                    let p0 = w.start + dir * op_start;
                    let p1 = w.start + dir * op_end;
                    vec![Prism::along(p0, p1, 50.0, z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::CurtainWall(cw) => {
                let base_z = self.level_elevation(cw.base_level);
                let top_z = cw.top_level.map(|lvl| self.level_elevation(lvl));
                cw.solids(base_z, top_z)
            }
            Element::BuildingPad(bp) => {
                let base_z = self.level_elevation(bp.level);
                let (z_bot, z_top) = bp.z_bounds(base_z);
                if bp.boundary.len() >= 3 && z_top > z_bot {
                    vec![Prism::new(bp.boundary.clone(), z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::Road(rd) => {
                let base_z = self.level_elevation(rd.level);
                let (z_bot, z_top) = rd.z_bounds(base_z);
                let poly = rd.footprint();
                if poly.len() >= 3 && z_top > z_bot {
                    vec![Prism::new(poly, z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::Sidewalk(sw) => {
                let base_z = self.level_elevation(sw.level);
                let (z_bot, z_top) = sw.z_bounds(base_z);
                let poly = sw.footprint();
                if poly.len() >= 3 && z_top > z_bot {
                    vec![Prism::new(poly, z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::Pathwalk(pw) => {
                let base_z = self.level_elevation(pw.level);
                let (z_bot, z_top) = pw.z_bounds(base_z);
                let poly = pw.footprint();
                if poly.len() >= 3 && z_top > z_bot {
                    vec![Prism::new(poly, z_bot, z_top)]
                } else {
                    Vec::new()
                }
            }
            Element::Toposolid(_) | Element::PropertyLine(_) => Vec::new(),
            Element::FreeformSurface(_)
            | Element::Dimension(_)
            | Element::DetailLine(_)
            | Element::TextNote(_)
            | Element::Leader(_) => Vec::new(),
        }
    }

    /// Generates boundary 3D faces for a specific element (including exact spherical faces for spheres,
    /// pitched/flat roofs, inclined ramps, and monolithic/stepped/open-riser stairs).
    pub fn element_faces(&self, id: ElementId) -> Vec<Face> {
        let Some(el) = self.elements.get(&id) else {
            return Vec::new();
        };
        match el {
            Element::Wall(w) => {
                let base_z = self.level_elevation(w.base_level);
                let top_z = w.top_level.map(|lvl| self.level_elevation(lvl));
                let openings = self.hosted_openings_for_wall(w.id);
                let all_walls: Vec<&Wall> = self
                    .elements
                    .values()
                    .filter_map(|e| match e {
                        Element::Wall(other) => Some(other),
                        _ => None,
                    })
                    .collect();
                w.boundary_faces(base_z, top_z, &openings, &all_walls, true)
            }
            Element::FreeformSolid(f) => f.faces(),
            Element::Roof(r) => {
                let base_z = self.level_elevation(r.level);
                let top_z = r.top_level.map(|lvl| self.level_elevation(lvl));
                r.faces_with_bounds(base_z, top_z)
            }
            Element::Stair(s) => {
                let base_z = self.level_elevation(s.base_level);
                let top_z = s.top_level.map(|lvl| self.level_elevation(lvl));
                s.faces(base_z, top_z)
            }
            Element::Ramp(r) => {
                let base_z = self.level_elevation(r.base_level);
                let top_z = r.top_level.map(|lvl| self.level_elevation(lvl));
                r.faces(base_z, top_z)
            }
            Element::Footing(f) => {
                let base_z = self.level_elevation(f.base_level);
                let top_z = f.top_level.map(|lvl| self.level_elevation(lvl));
                f.faces(base_z, top_z)
            }
            Element::CurtainWall(cw) => {
                let base_z = self.level_elevation(cw.base_level);
                let top_z = cw.top_level.map(|lvl| self.level_elevation(lvl));
                cw.faces(base_z, top_z)
            }
            Element::Slab(s) => {
                let base_z = self.level_elevation(s.level);
                let top_z = s.top_level.map(|lvl| self.level_elevation(lvl));
                s.faces(base_z, top_z)
            }
            Element::Ceiling(c) => {
                let base_z = self.level_elevation(c.level);
                let top_z = c.top_level.map(|lvl| self.level_elevation(lvl));
                c.faces(base_z, top_z)
            }
            Element::BuildingPad(bp) => {
                let base_z = self.level_elevation(bp.level);
                bp.faces(base_z)
            }
            Element::Toposolid(ts) => {
                let base_z = self.level_elevation(ts.level);
                ts.faces(base_z)
            }
            Element::Road(rd) => {
                let base_z = self.level_elevation(rd.level);
                rd.faces(base_z)
            }
            Element::Sidewalk(sw) => {
                let base_z = self.level_elevation(sw.level);
                sw.faces(base_z)
            }
            Element::Pathwalk(pw) => {
                let base_z = self.level_elevation(pw.level);
                pw.faces(base_z)
            }
            Element::FreeformSurface(s) => {
                if let Some(ref pts3d) = s.boundary_3d {
                    if pts3d.len() >= 3 {
                        vec![Face::solid(pts3d.clone())]
                    } else {
                        Vec::new()
                    }
                } else if s.boundary.len() >= 3 {
                    let pts: Vec<plinth_geom::Vec3> = s
                        .boundary
                        .iter()
                        .map(|p| plinth_geom::Vec3::new(p.x, p.y, s.elevation))
                        .collect();
                    vec![Face::solid(pts)]
                } else {
                    Vec::new()
                }
            }
            _ => {
                let solids = self.element_solids(id);
                let mut faces = Vec::new();
                for s in solids {
                    faces.extend(s.faces());
                }
                faces
            }
        }
    }

    /// Generate complete sample building model for instant review and testing:
    /// A professional 5-Storey Urban Apartment Mid-Rise with Ground Floor Lobby,
    /// Storefront Curtain Walls, 4 Residential Floors with South Balconies & Railings,
    /// Penthouse Suites, Rooftop Terrace Deck with Pergola Canopy, Landscaped Site Terrain,
    /// Urban Two-Lane Roadway, Concrete Sidewalk & Bullnose Curb, and Full 2D/3D Annotations.
    pub fn create_sample_building() -> Self {
        let mut doc = Self {
            project_info: ProjectInfo {
                name: "Arch3D Luxury Urban Residences".into(),
                number: "A-2026-05".into(),
                client: "Metropolitan Living Group".into(),
                address: "450 Skyline Boulevard".into(),
            },
            levels: BTreeMap::new(),
            grids: BTreeMap::new(),
            materials: BTreeMap::new(),
            elements: BTreeMap::new(),
            views: BTreeMap::new(),
            sheets: BTreeMap::new(),
            active_view_id: ViewId::nil(),
            active_level_id: LevelId::nil(),
            active_sheet_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };

        // Standard architectural & structural materials
        let mat_conc = Material::default_concrete();
        let mat_masonry = Material::default_masonry();
        let mat_steel = Material::default_steel();
        let mat_glass = Material::default_glass();
        let mat_drywall = Material::default_drywall();
        let mat_timber = Material::default_timber();

        doc.materials.insert(mat_conc.id, mat_conc);
        doc.materials.insert(mat_masonry.id, mat_masonry);
        doc.materials.insert(mat_steel.id, mat_steel);
        doc.materials.insert(mat_glass.id, mat_glass);
        doc.materials.insert(mat_drywall.id, mat_drywall);
        doc.materials.insert(mat_timber.id, mat_timber);

        // -------------------------------------------------------------------
        // 1. Establish 6 Building Storey Datums
        // -------------------------------------------------------------------
        let lvl1 = Level::new("Level 1", 0.0, 3600.0);
        let lvl2 = Level::new("Level 2", 3600.0, 3200.0);
        let lvl3 = Level::new("Level 3", 6800.0, 3200.0);
        let lvl4 = Level::new("Level 4", 10000.0, 3200.0);
        let lvl5 = Level::new("Level 5", 13200.0, 3400.0);
        let lvl_roof = Level::new("Roof Deck", 16600.0, 3000.0);
        let lvl_bulkhead = Level::new("Roof Bulkhead", 19600.0, 0.0);

        let lvl1_id = lvl1.id;
        let lvl2_id = lvl2.id;
        let lvl3_id = lvl3.id;
        let lvl4_id = lvl4.id;
        let lvl5_id = lvl5.id;
        let lvl_roof_id = lvl_roof.id;
        let lvl_bulkhead_id = lvl_bulkhead.id;

        doc.levels.insert(lvl1_id, lvl1);
        doc.levels.insert(lvl2_id, lvl2);
        doc.levels.insert(lvl3_id, lvl3);
        doc.levels.insert(lvl4_id, lvl4);
        doc.levels.insert(lvl5_id, lvl5);
        doc.levels.insert(lvl_roof_id, lvl_roof);
        doc.levels.insert(lvl_bulkhead_id, lvl_bulkhead);
        doc.active_level_id = lvl1_id;

        // -------------------------------------------------------------------
        // 2. Structural Grids (X-Axis: A, B, C, D | Y-Axis: 1, 2, 3)
        // -------------------------------------------------------------------
        doc.grids.insert(GridId::new(), GridLine::new("A", vec2(0.0, -4000.0), vec2(0.0, 16000.0)));
        doc.grids.insert(GridId::new(), GridLine::new("B", vec2(6000.0, -4000.0), vec2(6000.0, 16000.0)));
        doc.grids.insert(GridId::new(), GridLine::new("C", vec2(12000.0, -4000.0), vec2(12000.0, 16000.0)));
        doc.grids.insert(GridId::new(), GridLine::new("D", vec2(18000.0, -4000.0), vec2(18000.0, 16000.0)));

        doc.grids.insert(GridId::new(), GridLine::new("1", vec2(-3000.0, 0.0), vec2(21000.0, 0.0)));
        doc.grids.insert(GridId::new(), GridLine::new("2", vec2(-3000.0, 6000.0), vec2(21000.0, 6000.0)));
        doc.grids.insert(GridId::new(), GridLine::new("3", vec2(-3000.0, 12000.0), vec2(21000.0, 12000.0)));

        // -------------------------------------------------------------------
        // 3. Site Context: Toposolid Terrain, Building Pad, Road & Sidewalk
        // -------------------------------------------------------------------
        // A) Green Landscaped Parcel (Toposolid)
        let site_poly = vec![
            vec2(-8000.0, -10000.0),
            vec2(26000.0, -10000.0),
            vec2(26000.0, 18000.0),
            vec2(-8000.0, 18000.0),
        ];
        let mut topo = Toposolid::new(site_poly, lvl1_id);
        topo.name = "Property Site & Lawn".into();
        topo.base_depth = 400.0;
        topo.elevation_points = vec![
            ToposolidPoint::new(-8000.0, -10000.0, -150.0),
            ToposolidPoint::new(26000.0, -10000.0, -150.0),
            ToposolidPoint::new(26000.0, 18000.0, 0.0),
            ToposolidPoint::new(-8000.0, 18000.0, 0.0),
        ];
        doc.elements.insert(topo.id, Element::Toposolid(topo));

        // B) Excavated Building Foundation Pad
        let pad_poly = vec![
            vec2(-400.0, -400.0),
            vec2(18400.0, -400.0),
            vec2(18400.0, 12400.0),
            vec2(-400.0, 12400.0),
        ];
        let mut pad = BuildingPad::new(pad_poly, lvl1_id);
        pad.thickness = 300.0;
        pad.elevation_offset = 0.0;
        pad.name = "Subgrade Foundation Pad".into();
        doc.elements.insert(pad.id, Element::BuildingPad(pad));

        // C) Urban Two-Lane Asphalt Roadway (South Frontage)
        let mut road = Road::new(vec![vec2(-10000.0, -7000.0), vec2(28000.0, -7000.0)], lvl1_id);
        road.width = 7300.0;
        road.thickness = 200.0;
        road.name = "Skyline Boulevard (7.3m)".into();
        doc.elements.insert(road.id, Element::Road(road));

        // D) Pedestrian Sidewalk with Bullnose Curb
        let mut sw = Sidewalk::new(vec![vec2(-10000.0, -2500.0), vec2(28000.0, -2500.0)], lvl1_id);
        sw.width = 2200.0;
        sw.thickness = 150.0;
        sw.name = "North Pedestrian Promenade".into();
        doc.elements.insert(sw.id, Element::Sidewalk(sw));

        // E) Main Lobby Entry Pathwalk Plaza
        let mut path = Pathwalk::new(vec![vec2(9000.0, -2500.0), vec2(9000.0, 0.0)], lvl1_id);
        path.width = 3000.0;
        path.thickness = 100.0;
        path.name = "Residential Entry Plaza".into();
        doc.elements.insert(path.id, Element::Pathwalk(path));

        // -------------------------------------------------------------------
        // 4. Structural Columns (12 columns per floor x 5 levels = 60 columns)
        // -------------------------------------------------------------------
        let col_coords = [
            vec2(0.0, 0.0), vec2(6000.0, 0.0), vec2(12000.0, 0.0), vec2(18000.0, 0.0),
            vec2(0.0, 6000.0), vec2(6000.0, 6000.0), vec2(12000.0, 6000.0), vec2(18000.0, 6000.0),
            vec2(0.0, 12000.0), vec2(6000.0, 12000.0), vec2(12000.0, 12000.0), vec2(18000.0, 12000.0),
        ];

        let floor_levels = [
            (lvl1_id, 3600.0),
            (lvl2_id, 3200.0),
            (lvl3_id, 3200.0),
            (lvl4_id, 3200.0),
            (lvl5_id, 3400.0),
        ];

        for &(lvl_id, h) in &floor_levels {
            for &pt in &col_coords {
                let mut col = Column::new(pt, lvl_id);
                col.is_structural = true;
                col.width = 400.0;
                col.depth = 400.0;
                col.top_level = None;
                col.unconnected_height = h;
                col.column_type_name = "Concrete Column 400x400mm".into();
                doc.elements.insert(col.id, Element::Column(col));
            }
        }

        // -------------------------------------------------------------------
        // 5. Structural Beams Framing Ceiling Bays
        // -------------------------------------------------------------------
        let beam_spans = [
            // X-direction primary beams (Y = 0, 6000, 12000)
            (vec2(0.0, 0.0), vec2(18000.0, 0.0)),
            (vec2(0.0, 6000.0), vec2(18000.0, 6000.0)),
            (vec2(0.0, 12000.0), vec2(18000.0, 12000.0)),
            // Y-direction transverse framing beams (X = 0, 6000, 12000, 18000)
            (vec2(0.0, 0.0), vec2(0.0, 12000.0)),
            (vec2(6000.0, 0.0), vec2(6000.0, 12000.0)),
            (vec2(12000.0, 0.0), vec2(12000.0, 12000.0)),
            (vec2(18000.0, 0.0), vec2(18000.0, 12000.0)),
        ];

        for &(lvl_id, _h) in &floor_levels[1..] {
            for &(p0, p1) in &beam_spans {
                let mut beam = Beam::new(p0, p1, lvl_id);
                beam.width = 300.0;
                beam.depth = 500.0;
                beam.z_offset = -200.0;
                beam.beam_type_name = "Concrete Girder 300x500mm".into();
                doc.elements.insert(beam.id, Element::Beam(beam));
            }
        }

        // -------------------------------------------------------------------
        // 6. Floor Slabs & Cantilevered Balconies
        // -------------------------------------------------------------------
        // Level 1 Podium Slab
        let mut slab1 = Slab::new(
            vec![
                vec2(-150.0, -150.0),
                vec2(18150.0, -150.0),
                vec2(18150.0, 12150.0),
                vec2(-150.0, 12150.0),
            ],
            lvl1_id,
        );
        slab1.thickness = 250.0;
        slab1.slab_type_name = "Ground Foundation Slab 250mm".into();
        doc.elements.insert(slab1.id, Element::Slab(slab1));

        // Levels 2, 3, 4 Slabs (Main floor + South Balcony Projections)
        for &(lvl_id, _) in &floor_levels[1..4] {
            // Main Floor Slab
            let mut fl_slab = Slab::new(
                vec![
                    vec2(0.0, 0.0),
                    vec2(18000.0, 0.0),
                    vec2(18000.0, 12000.0),
                    vec2(0.0, 12000.0),
                ],
                lvl_id,
            );
            fl_slab.thickness = 200.0;
            fl_slab.slab_type_name = "Post-Tensioned Floor Slab 200mm".into();
            doc.elements.insert(fl_slab.id, Element::Slab(fl_slab));

            // Balcony A (West Suite Balcony)
            let mut balc_a = Slab::new(
                vec![
                    vec2(1500.0, -1500.0),
                    vec2(4500.0, -1500.0),
                    vec2(4500.0, 0.0),
                    vec2(1500.0, 0.0),
                ],
                lvl_id,
            );
            balc_a.thickness = 180.0;
            balc_a.slab_type_name = "Cantilevered Balcony Slab 180mm".into();
            doc.elements.insert(balc_a.id, Element::Slab(balc_a));

            // Balcony A Railing
            let mut rail_a = Railing::new(
                vec![vec2(1500.0, 0.0), vec2(1500.0, -1500.0), vec2(4500.0, -1500.0), vec2(4500.0, 0.0)],
                lvl_id,
            );
            rail_a.height = 1050.0;
            rail_a.railing_type_name = "Glass & Stainless Steel Balcony Railing 1050mm".into();
            doc.elements.insert(rail_a.id, Element::Railing(rail_a));

            // Balcony B (East Suite Balcony)
            let mut balc_b = Slab::new(
                vec![
                    vec2(13500.0, -1500.0),
                    vec2(16500.0, -1500.0),
                    vec2(16500.0, 0.0),
                    vec2(13500.0, 0.0),
                ],
                lvl_id,
            );
            balc_b.thickness = 180.0;
            balc_b.slab_type_name = "Cantilevered Balcony Slab 180mm".into();
            doc.elements.insert(balc_b.id, Element::Slab(balc_b));

            // Balcony B Railing
            let mut rail_b = Railing::new(
                vec![vec2(13500.0, 0.0), vec2(13500.0, -1500.0), vec2(16500.0, -1500.0), vec2(16500.0, 0.0)],
                lvl_id,
            );
            rail_b.height = 1050.0;
            rail_b.railing_type_name = "Glass & Stainless Steel Balcony Railing 1050mm".into();
            doc.elements.insert(rail_b.id, Element::Railing(rail_b));
        }

        // Level 5 Penthouse Slab
        let mut slab5 = Slab::new(
            vec![
                vec2(0.0, 0.0),
                vec2(18000.0, 0.0),
                vec2(18000.0, 12000.0),
                vec2(0.0, 12000.0),
            ],
            lvl5_id,
        );
        slab5.thickness = 200.0;
        slab5.slab_type_name = "Penthouse Suspended Slab 200mm".into();
        doc.elements.insert(slab5.id, Element::Slab(slab5));

        // Roof Deck Floor Slab (Level 6)
        let mut slab_roof = Slab::new(
            vec![
                vec2(0.0, 0.0),
                vec2(18000.0, 0.0),
                vec2(18000.0, 12000.0),
                vec2(0.0, 12000.0),
            ],
            lvl_roof_id,
        );
        slab_roof.thickness = 220.0;
        slab_roof.slab_type_name = "Rooftop Deck Terrace Slab 220mm".into();
        doc.elements.insert(slab_roof.id, Element::Slab(slab_roof));

        // Rooftop Deck Perimeter Safety Railing
        let mut roof_rail = Railing::new(
            vec![
                vec2(50.0, 50.0),
                vec2(17950.0, 50.0),
                vec2(17950.0, 11950.0),
                vec2(50.0, 11950.0),
                vec2(50.0, 50.0),
            ],
            lvl_roof_id,
        );
        roof_rail.height = 1100.0;
        roof_rail.railing_type_name = "Rooftop Glass Perimeter Guard 1100mm".into();
        doc.elements.insert(roof_rail.id, Element::Railing(roof_rail));

        // -------------------------------------------------------------------
        // 7. Ceilings on All 5 Storeys
        // -------------------------------------------------------------------
        for &(lvl_id, h) in &floor_levels {
            let mut ceil = Ceiling::new(
                vec![
                    vec2(150.0, 150.0),
                    vec2(17850.0, 150.0),
                    vec2(17850.0, 11850.0),
                    vec2(150.0, 11850.0),
                ],
                lvl_id,
            );
            ceil.height_above_level = h - 500.0;
            ceil.ceiling_type_name = "Acoustic Tile Ceiling Grid 600x600mm".into();
            doc.elements.insert(ceil.id, Element::Ceiling(ceil));
        }

        // -------------------------------------------------------------------
        // 8. Ground Floor (Level 1) Walls, Storefront Curtain Wall, Doors & Windows
        // -------------------------------------------------------------------
        // Central Storefront Curtain Wall Entrance on South Facade (6000..12000)
        let mut cw_lobby = CurtainWall::new(vec2(6000.0, 0.0), vec2(12000.0, 0.0), lvl1_id);
        cw_lobby.unconnected_height = 3600.0;
        cw_lobby.mullion_width = 50.0;
        cw_lobby.mullion_depth = 150.0;
        cw_lobby.vertical_grid_spacing = 1500.0;
        cw_lobby.horizontal_grid_spacing = 1200.0;
        cw_lobby.curtain_wall_type_name = "Glazed Entrance Storefront (CW-01)".into();
        doc.elements.insert(cw_lobby.id, Element::CurtainWall(cw_lobby));

        // South Left Exterior Wall (0..6000)
        let mut w1_sl = Wall::new(vec2(0.0, 0.0), vec2(6000.0, 0.0), lvl1_id);
        w1_sl.unconnected_height = 3600.0;
        w1_sl.thickness = 250.0;
        w1_sl.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        let w1_sl_id = w1_sl.id;
        doc.elements.insert(w1_sl_id, Element::Wall(w1_sl));

        // Window on South Left Wall
        let mut win_sl = Window::new(w1_sl_id, 3000.0);
        win_sl.width = 1800.0;
        win_sl.height = 1800.0;
        win_sl.sill_height = 900.0;
        doc.elements.insert(win_sl.id, Element::Window(win_sl));

        // South Right Exterior Wall (12000..18000)
        let mut w1_sr = Wall::new(vec2(12000.0, 0.0), vec2(18000.0, 0.0), lvl1_id);
        w1_sr.unconnected_height = 3600.0;
        w1_sr.thickness = 250.0;
        w1_sr.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        let w1_sr_id = w1_sr.id;
        doc.elements.insert(w1_sr_id, Element::Wall(w1_sr));

        // Window on South Right Wall
        let mut win_sr = Window::new(w1_sr_id, 3000.0);
        win_sr.width = 1800.0;
        win_sr.height = 1800.0;
        win_sr.sill_height = 900.0;
        doc.elements.insert(win_sr.id, Element::Window(win_sr));

        // East Exterior Wall (0..12000 at X = 18000)
        let mut w1_e = Wall::new(vec2(18000.0, 0.0), vec2(18000.0, 12000.0), lvl1_id);
        w1_e.unconnected_height = 3600.0;
        w1_e.thickness = 250.0;
        w1_e.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        let w1_e_id = w1_e.id;
        doc.elements.insert(w1_e_id, Element::Wall(w1_e));

        for offset in [3500.0, 8500.0] {
            let mut win = Window::new(w1_e_id, offset);
            win.width = 1500.0;
            win.height = 1500.0;
            win.sill_height = 1000.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        // North Rear Exterior Wall (18000..0 at Y = 12000)
        let mut w1_n = Wall::new(vec2(18000.0, 12000.0), vec2(0.0, 12000.0), lvl1_id);
        w1_n.unconnected_height = 3600.0;
        w1_n.thickness = 250.0;
        w1_n.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        let w1_n_id = w1_n.id;
        doc.elements.insert(w1_n_id, Element::Wall(w1_n));

        // Emergency Rear Exit Door on North Wall
        let mut d_rear = Door::new(w1_n_id, 9000.0);
        d_rear.width = 1000.0;
        d_rear.height = 2100.0;
        d_rear.door_type_name = "Emergency Egress Metal Door 1000x2100mm".into();
        doc.elements.insert(d_rear.id, Element::Door(d_rear));

        // Windows on North Wall
        for offset in [3500.0, 14500.0] {
            let mut win = Window::new(w1_n_id, offset);
            win.width = 1500.0;
            win.height = 1500.0;
            win.sill_height = 1000.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        // West Exterior Wall (12000..0 at X = 0)
        let mut w1_w = Wall::new(vec2(0.0, 12000.0), vec2(0.0, 0.0), lvl1_id);
        w1_w.unconnected_height = 3600.0;
        w1_w.thickness = 250.0;
        w1_w.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        let w1_w_id = w1_w.id;
        doc.elements.insert(w1_w_id, Element::Wall(w1_w));

        for offset in [3500.0, 8500.0] {
            let mut win = Window::new(w1_w_id, offset);
            win.width = 1500.0;
            win.height = 1500.0;
            win.sill_height = 1000.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        // Level 1 Interior Core Walls & Partitions
        // Central Corridor Demising Wall
        let mut w1_corr = Wall::new(vec2(0.0, 6000.0), vec2(6000.0, 6000.0), lvl1_id);
        w1_corr.unconnected_height = 3000.0;
        w1_corr.thickness = 150.0;
        w1_corr.wall_type_name = "Interior - Partition 150mm".into();
        let w1_corr_id = w1_corr.id;
        doc.elements.insert(w1_corr_id, Element::Wall(w1_corr));

        let mut d_off = Door::new(w1_corr_id, 3000.0);
        d_off.width = 900.0;
        d_off.height = 2100.0;
        d_off.door_type_name = "Office Interior Door 900x2100mm".into();
        doc.elements.insert(d_off.id, Element::Door(d_off));

        // Elevator & Stair Core Shaft Walls
        let core_walls = [
            (vec2(6000.0, 6000.0), vec2(12000.0, 6000.0)),
            (vec2(12000.0, 6000.0), vec2(12000.0, 12000.0)),
            (vec2(6000.0, 12000.0), vec2(6000.0, 6000.0)),
        ];
        for &(p0, p1) in &core_walls {
            let mut w_core = Wall::new(p0, p1, lvl1_id);
            w_core.unconnected_height = 3600.0;
            w_core.thickness = 200.0;
            w_core.wall_type_name = "Shaftwall - 2HR Concrete Core 200mm".into();
            doc.elements.insert(w_core.id, Element::Wall(w_core));
        }

        // Core Staircase (Level 1 to Level 2)
        let mut stair1 = Stair::new(vec2(7000.0, 7000.0), vec2(7000.0, 11000.0), lvl1_id);
        stair1.width = 1200.0;
        stair1.height = 3600.0;
        stair1.riser_count = 20;
        stair1.name = "Central Egress Core Stair".into();
        doc.elements.insert(stair1.id, Element::Stair(stair1));

        // -------------------------------------------------------------------
        // 9. Levels 2, 3, 4 (Typical Residential Floors)
        // -------------------------------------------------------------------
        for &(lvl_id, h) in &floor_levels[1..4] {
            // South Wall
            let mut w_s = Wall::new(vec2(0.0, 0.0), vec2(18000.0, 0.0), lvl_id);
            w_s.unconnected_height = h;
            w_s.thickness = 250.0;
            w_s.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
            let w_s_id = w_s.id;
            doc.elements.insert(w_s_id, Element::Wall(w_s));

            // Balcony Sliding Access Doors at X = 3000 and X = 15000
            for offset in [3000.0, 15000.0] {
                let mut d_balc = Door::new(w_s_id, offset);
                d_balc.width = 1800.0;
                d_balc.height = 2200.0;
                d_balc.door_type_name = "Balcony Sliding Glass Door 1800x2200mm".into();
                doc.elements.insert(d_balc.id, Element::Door(d_balc));
            }

            // East Wall with Windows
            let mut w_e = Wall::new(vec2(18000.0, 0.0), vec2(18000.0, 12000.0), lvl_id);
            w_e.unconnected_height = h;
            w_e.thickness = 250.0;
            w_e.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
            let w_e_id = w_e.id;
            doc.elements.insert(w_e_id, Element::Wall(w_e));

            for offset in [3500.0, 8500.0] {
                let mut win = Window::new(w_e_id, offset);
                win.width = 1500.0;
                win.height = 1500.0;
                win.sill_height = 900.0;
                doc.elements.insert(win.id, Element::Window(win));
            }

            // North Wall with Windows
            let mut w_n = Wall::new(vec2(18000.0, 12000.0), vec2(0.0, 12000.0), lvl_id);
            w_n.unconnected_height = h;
            w_n.thickness = 250.0;
            w_n.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
            let w_n_id = w_n.id;
            doc.elements.insert(w_n_id, Element::Wall(w_n));

            for offset in [3500.0, 9000.0, 14500.0] {
                let mut win = Window::new(w_n_id, offset);
                win.width = 1500.0;
                win.height = 1500.0;
                win.sill_height = 900.0;
                doc.elements.insert(win.id, Element::Window(win));
            }

            // West Wall with Windows
            let mut w_w = Wall::new(vec2(0.0, 12000.0), vec2(0.0, 0.0), lvl_id);
            w_w.unconnected_height = h;
            w_w.thickness = 250.0;
            w_w.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
            let w_w_id = w_w.id;
            doc.elements.insert(w_w_id, Element::Wall(w_w));

            for offset in [3500.0, 8500.0] {
                let mut win = Window::new(w_w_id, offset);
                win.width = 1500.0;
                win.height = 1500.0;
                win.sill_height = 900.0;
                doc.elements.insert(win.id, Element::Window(win));
            }

            // Central Core Demising Wall
            let mut w_core_div = Wall::new(vec2(6000.0, 6000.0), vec2(12000.0, 6000.0), lvl_id);
            w_core_div.unconnected_height = h;
            w_core_div.thickness = 200.0;
            w_core_div.wall_type_name = "Shaftwall - 2HR Concrete Core 200mm".into();
            let w_core_div_id = w_core_div.id;
            doc.elements.insert(w_core_div_id, Element::Wall(w_core_div));

            // Apartment A Entrance Door
            let mut d_apt_a = Door::new(w_core_div_id, 1500.0);
            d_apt_a.width = 900.0;
            d_apt_a.height = 2100.0;
            d_apt_a.door_type_name = "Apartment Suite Entry Door 900x2100mm".into();
            doc.elements.insert(d_apt_a.id, Element::Door(d_apt_a));

            // Apartment B Entrance Door
            let mut d_apt_b = Door::new(w_core_div_id, 4500.0);
            d_apt_b.width = 900.0;
            d_apt_b.height = 2100.0;
            d_apt_b.door_type_name = "Apartment Suite Entry Door 900x2100mm".into();
            doc.elements.insert(d_apt_b.id, Element::Door(d_apt_b));
        }

        // -------------------------------------------------------------------
        // 10. Level 5 (Penthouse Floor) Walls & Panoramic Curtain Wall
        // -------------------------------------------------------------------
        // Panoramic Curtain Wall facing South Penthouse Terrace (6000..18000)
        let mut cw_penthouse = CurtainWall::new(vec2(6000.0, 0.0), vec2(18000.0, 0.0), lvl5_id);
        cw_penthouse.unconnected_height = 3400.0;
        cw_penthouse.mullion_width = 50.0;
        cw_penthouse.mullion_depth = 150.0;
        cw_penthouse.vertical_grid_spacing = 2000.0;
        cw_penthouse.horizontal_grid_spacing = 1700.0;
        cw_penthouse.curtain_wall_type_name = "Penthouse Panoramic Curtain Wall (CW-PH)".into();
        doc.elements.insert(cw_penthouse.id, Element::CurtainWall(cw_penthouse));

        // West Exterior Wall
        let mut w5_w = Wall::new(vec2(0.0, 12000.0), vec2(0.0, 0.0), lvl5_id);
        w5_w.unconnected_height = 3400.0;
        w5_w.thickness = 250.0;
        w5_w.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        doc.elements.insert(w5_w.id, Element::Wall(w5_w));

        // North Exterior Wall
        let mut w5_n = Wall::new(vec2(18000.0, 12000.0), vec2(0.0, 12000.0), lvl5_id);
        w5_n.unconnected_height = 3400.0;
        w5_n.thickness = 250.0;
        w5_n.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        doc.elements.insert(w5_n.id, Element::Wall(w5_n));

        // East Exterior Wall
        let mut w5_e = Wall::new(vec2(18000.0, 0.0), vec2(18000.0, 12000.0), lvl5_id);
        w5_e.unconnected_height = 3400.0;
        w5_e.thickness = 250.0;
        w5_e.wall_type_name = "Exterior - Insulated Architectural Concrete 250mm".into();
        doc.elements.insert(w5_e.id, Element::Wall(w5_e));

        // -------------------------------------------------------------------
        // 11. Roof Deck Bulkhead Enclosure & Shade Pergola Canopy
        // -------------------------------------------------------------------
        // Stair & Elevator Machine Penthouse Bulkhead (6000..12000, 6000..12000)
        let bulkhead_walls = [
            (vec2(6000.0, 6000.0), vec2(12000.0, 6000.0)),
            (vec2(12000.0, 6000.0), vec2(12000.0, 12000.0)),
            (vec2(12000.0, 12000.0), vec2(6000.0, 12000.0)),
            (vec2(6000.0, 12000.0), vec2(6000.0, 6000.0)),
        ];

        for &(p0, p1) in &bulkhead_walls {
            let mut w_bulk = Wall::new(p0, p1, lvl_roof_id);
            w_bulk.unconnected_height = 3000.0;
            w_bulk.thickness = 200.0;
            w_bulk.wall_type_name = "Rooftop Elevator Bulkhead Wall 200mm".into();
            doc.elements.insert(w_bulk.id, Element::Wall(w_bulk));
        }

        // Roof Bulkhead Exit Door
        let mut w_bulk_front = Wall::new(vec2(6000.0, 6000.0), vec2(9000.0, 6000.0), lvl_roof_id);
        w_bulk_front.unconnected_height = 3000.0;
        w_bulk_front.thickness = 200.0;
        let w_bf_id = w_bulk_front.id;
        doc.elements.insert(w_bf_id, Element::Wall(w_bulk_front));

        let mut d_roof_exit = Door::new(w_bf_id, 1500.0);
        d_roof_exit.width = 900.0;
        d_roof_exit.height = 2100.0;
        d_roof_exit.door_type_name = "Rooftop Terrace Weather Door 900x2100mm".into();
        doc.elements.insert(d_roof_exit.id, Element::Door(d_roof_exit));

        // Machine Bulkhead Roof Slab
        let mut slab_bulkhead = Slab::new(
            vec![
                vec2(5800.0, 5800.0),
                vec2(12200.0, 5800.0),
                vec2(12200.0, 12200.0),
                vec2(5800.0, 12200.0),
            ],
            lvl_bulkhead_id,
        );
        slab_bulkhead.thickness = 200.0;
        slab_bulkhead.slab_type_name = "Concrete Bulkhead Cap 200mm".into();
        doc.elements.insert(slab_bulkhead.id, Element::Slab(slab_bulkhead));

        // Rooftop Pergola Canopy Roof (Flat/Shed modern steel canopy over lounge area)
        let mut canopy = Roof::new(
            vec![
                vec2(1000.0, 1000.0),
                vec2(5500.0, 1000.0),
                vec2(5500.0, 5500.0),
                vec2(1000.0, 5500.0),
            ],
            lvl_roof_id,
        );
        canopy.offset = 2600.0;
        canopy.thickness = 100.0;
        canopy.shape = RoofShape::Flat;
        canopy.roof_type_name = "Rooftop Garden Pergola Canopy (Steel & Timber)".into();
        doc.elements.insert(canopy.id, Element::Roof(canopy));

        // -------------------------------------------------------------------
        // 12. Standard Architectural Plan, Elevation, Section & 3D Views
        // -------------------------------------------------------------------
        let v_l1 = ViewDef::new_floor_plan("Floor Plan: Level 1", lvl1_id);
        let v_l2 = ViewDef::new_floor_plan("Floor Plan: Level 2 (Typical)", lvl2_id);
        let v_l3 = ViewDef::new_floor_plan("Floor Plan: Level 3", lvl3_id);
        let v_l4 = ViewDef::new_floor_plan("Floor Plan: Level 4", lvl4_id);
        let v_l5 = ViewDef::new_floor_plan("Floor Plan: Level 5 (Penthouse)", lvl5_id);
        let v_roof = ViewDef::new_floor_plan("Floor Plan: Roof Deck", lvl_roof_id);

        let v_rcp1 = ViewDef::new_ceiling_plan("Ceiling Plan: Level 1", lvl1_id);
        let v_rcp2 = ViewDef::new_ceiling_plan("Ceiling Plan: Level 2", lvl2_id);

        let v_south = ViewDef::new_elevation("Elevation: South (Front Facade)", ViewKind::ElevationSouth);
        let v_north = ViewDef::new_elevation("Elevation: North (Rear Facade)", ViewKind::ElevationNorth);
        let v_east = ViewDef::new_elevation("Elevation: East", ViewKind::ElevationEast);
        let v_west = ViewDef::new_elevation("Elevation: West", ViewKind::ElevationWest);

        let v_sec = ViewDef::new_section("Section 1: Longitudinal Core", vec2(-2000.0, 6000.0), vec2(20000.0, 6000.0));
        let mut v_3d = ViewDef::new_3d("3D View");
        if let Some(cam) = v_3d.camera_3d.as_mut() {
            cam.target_x = 9000.0;
            cam.target_y = 6000.0;
            cam.target_z = 8500.0;
            cam.distance = 42000.0;
            cam.yaw = -0.65;
            cam.pitch = 0.48;
        }

        let l1_view_id = v_l1.id;
        let l2_view_id = v_l2.id;
        let roof_view_id = v_roof.id;
        let v_south_id = v_south.id;
        let v_sec_id = v_sec.id;
        let v_3d_id = v_3d.id;

        doc.views.insert(l1_view_id, v_l1);
        doc.views.insert(l2_view_id, v_l2);
        doc.views.insert(v_l3.id, v_l3);
        doc.views.insert(v_l4.id, v_l4);
        doc.views.insert(v_l5.id, v_l5);
        doc.views.insert(roof_view_id, v_roof);

        doc.views.insert(v_rcp1.id, v_rcp1);
        doc.views.insert(v_rcp2.id, v_rcp2);

        doc.views.insert(v_south_id, v_south);
        doc.views.insert(v_north.id, v_north);
        doc.views.insert(v_east.id, v_east);
        doc.views.insert(v_west.id, v_west);

        doc.views.insert(v_sec_id, v_sec);
        doc.views.insert(v_3d_id, v_3d);

        // Default active view set to 3D for immediate visual impact
        doc.active_view_id = v_3d_id;

        // -------------------------------------------------------------------
        // 13. Professional 2D Plan Dimensions, Room Labels & Leaders
        // -------------------------------------------------------------------
        // Overall Building Width Dimension (18000mm)
        let dim_w = Dimension::new(l1_view_id, vec2(0.0, -1000.0), vec2(18000.0, -1000.0), 350.0);
        doc.elements.insert(dim_w.id, Element::Dimension(dim_w));

        // Structural Bay Dimensions (6000 + 6000 + 6000mm)
        let dim_b1 = Dimension::new(l1_view_id, vec2(0.0, -500.0), vec2(6000.0, -500.0), 200.0);
        let dim_b2 = Dimension::new(l1_view_id, vec2(6000.0, -500.0), vec2(12000.0, -500.0), 200.0);
        let dim_b3 = Dimension::new(l1_view_id, vec2(12000.0, -500.0), vec2(18000.0, -500.0), 200.0);
        doc.elements.insert(dim_b1.id, Element::Dimension(dim_b1));
        doc.elements.insert(dim_b2.id, Element::Dimension(dim_b2));
        doc.elements.insert(dim_b3.id, Element::Dimension(dim_b3));

        // Building Depth Dimension (12000mm)
        let dim_d = Dimension::new(l1_view_id, vec2(-800.0, 0.0), vec2(-800.0, 12000.0), 300.0);
        doc.elements.insert(dim_d.id, Element::Dimension(dim_d));

        // Level 1 Architectural Room Text Labels
        let labels_l1 = [
            (vec2(9000.0, 2500.0), "MAIN RESIDENTIAL LOBBY & RECEPTION"),
            (vec2(3000.0, 2500.0), "LEASING & MANAGEMENT OFFICE"),
            (vec2(15000.0, 2500.0), "PACKAGE & MAIL SUITE"),
            (vec2(9000.0, 9000.0), "CENTRAL ELEVATOR & STAIR CORE"),
            (vec2(3000.0, 9000.0), "RESIDENTS LOUNGE & COWORKING"),
            (vec2(15000.0, 9000.0), "BUILDING SERVICES & MEP CORE"),
            (vec2(9000.0, -1200.0), "ENTRY PLAZA & DROP-OFF"),
            (vec2(9000.0, -7000.0), "SKYLINE BOULEVARD (7.3M TWO-LANE ROAD)"),
        ];

        for (pos, text) in labels_l1 {
            let note = TextNote::new(l1_view_id, pos, text);
            doc.elements.insert(note.id, Element::TextNote(note));
        }

        // Leader Callouts on Level 1
        let leader1 = Leader::new(
            l1_view_id,
            vec2(9000.0, 0.0),
            vec2(10500.0, -600.0),
            vec2(12500.0, -600.0),
            "GLAZED STOREFRONT ENTRANCE (CW-01)",
            TickMark::Arrow,
            11.0,
            FontStyle::Bold,
            [0, 120, 215, 255],
        );
        doc.elements.insert(leader1.id, Element::Leader(leader1));

        let leader2 = Leader::new(
            l1_view_id,
            vec2(14000.0, -2500.0),
            vec2(15500.0, -3200.0),
            vec2(17500.0, -3200.0),
            "CONCRETE SIDEWALK WITH BULLNOSE CURB",
            TickMark::Arrow,
            10.0,
            FontStyle::Regular,
            [50, 50, 60, 255],
        );
        doc.elements.insert(leader2.id, Element::Leader(leader2));

        // Level 2 Typical Floor Room Labels
        let labels_l2 = [
            (vec2(3000.0, 3000.0), "SUITE 201 - 2-BEDROOM APARTMENT"),
            (vec2(15000.0, 3000.0), "SUITE 202 - 2-BEDROOM APARTMENT"),
            (vec2(3000.0, -750.0), "BALCONY TERRACE A"),
            (vec2(15000.0, -750.0), "BALCONY TERRACE B"),
            (vec2(9000.0, 4500.0), "CENTRAL RESIDENTIAL CORRIDOR"),
        ];
        for (pos, text) in labels_l2 {
            let note = TextNote::new(l2_view_id, pos, text);
            doc.elements.insert(note.id, Element::TextNote(note));
        }

        // Roof Deck Plan Labels
        let labels_roof = [
            (vec2(3000.0, 3000.0), "ROOFTOP GARDEN & SUN DECK"),
            (vec2(9000.0, 9000.0), "STAIR / ELEVATOR PENTHOUSE BULKHEAD"),
            (vec2(15000.0, 6000.0), "OUTDOOR SKY LOUNGE & DINING"),
        ];
        for (pos, text) in labels_roof {
            let note = TextNote::new(roof_view_id, pos, text);
            doc.elements.insert(note.id, Element::TextNote(note));
        }

        // -------------------------------------------------------------------
        // 14. Professional Multi-Sheet Architectural Layout (A101 & A102)
        // -------------------------------------------------------------------
        // Sheet A101: Ground Floor Plan & 3D Building Overview
        let mut sheet1 = Sheet::new("A101", "Ground Floor Plan & 3D Overview", PaperSize::A1);
        sheet1.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: l1_view_id,
            position_mm: vec2(60.0, 160.0),
            width_mm: 350.0,
            height_mm: 250.0,
            scale: 100,
            title: "GROUND FLOOR PLAN - LEVEL 1".into(),
            number: 1,
            show_title: true,
        });
        sheet1.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: l2_view_id,
            position_mm: vec2(60.0, 440.0),
            width_mm: 350.0,
            height_mm: 250.0,
            scale: 100,
            title: "TYPICAL RESIDENTIAL PLAN - LEVEL 2".into(),
            number: 2,
            show_title: true,
        });
        sheet1.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: v_3d_id,
            position_mm: vec2(440.0, 200.0),
            width_mm: 360.0,
            height_mm: 340.0,
            scale: 100,
            title: "3D AXONOMETRIC MODEL OVERVIEW".into(),
            number: 3,
            show_title: true,
        });
        doc.sheets.insert(sheet1.id, sheet1);

        // Sheet A102: Building Elevations & Longitudinal Section
        let mut sheet2 = Sheet::new("A102", "Elevations & Building Section", PaperSize::A1);
        sheet2.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: v_south_id,
            position_mm: vec2(60.0, 320.0),
            width_mm: 420.0,
            height_mm: 220.0,
            scale: 100,
            title: "SOUTH ELEVATION - MAIN FACADE".into(),
            number: 1,
            show_title: true,
        });
        sheet2.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: v_sec_id,
            position_mm: vec2(60.0, 80.0),
            width_mm: 420.0,
            height_mm: 200.0,
            scale: 100,
            title: "BUILDING SECTION 1 - LONGITUDINAL CORE".into(),
            number: 2,
            show_title: true,
        });
        sheet2.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: roof_view_id,
            position_mm: vec2(500.0, 160.0),
            width_mm: 300.0,
            height_mm: 220.0,
            scale: 100,
            title: "ROOF DECK & TERRACE PLAN".into(),
            number: 3,
            show_title: true,
        });
        doc.sheets.insert(sheet2.id, sheet2);

        doc
    }

    /// Single-storey 12m x 8m test house used for isolated unit tests
    pub fn create_test_box_building() -> Self {
        let mut doc = Self {
            project_info: ProjectInfo::default(),
            levels: BTreeMap::new(),
            grids: BTreeMap::new(),
            materials: BTreeMap::new(),
            elements: BTreeMap::new(),
            views: BTreeMap::new(),
            sheets: BTreeMap::new(),
            active_view_id: ViewId::nil(),
            active_level_id: LevelId::nil(),
            active_sheet_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };

        // Add default materials
        let mat_conc = Material::default_concrete();
        let mat_masonry = Material::default_masonry();
        let mat_steel = Material::default_steel();
        let mat_glass = Material::default_glass();
        let mat_drywall = Material::default_drywall();
        let mat_timber = Material::default_timber();

        doc.materials.insert(mat_conc.id, mat_conc);
        doc.materials.insert(mat_masonry.id, mat_masonry);
        doc.materials.insert(mat_steel.id, mat_steel);
        doc.materials.insert(mat_glass.id, mat_glass);
        doc.materials.insert(mat_drywall.id, mat_drywall);
        doc.materials.insert(mat_timber.id, mat_timber);

        // 1. Levels
        let lvl1 = Level::new("Level 1", 0.0, 3000.0);
        let lvl1_id = lvl1.id;
        doc.levels.insert(lvl1_id, lvl1);

        let lvl2 = Level::new("Level 2", 3000.0, 3000.0);
        let lvl2_id = lvl2.id;
        doc.levels.insert(lvl2_id, lvl2);

        doc.active_level_id = lvl1_id;

        // 2. Grids (A, B along X at 0, 12000; 1, 2 along Y at 0, 8000)
        let g_a = GridLine::new("A", vec2(0.0, -2000.0), vec2(0.0, 10000.0));
        let g_b = GridLine::new("B", vec2(12000.0, -2000.0), vec2(12000.0, 10000.0));
        let g_1 = GridLine::new("1", vec2(-2000.0, 0.0), vec2(14000.0, 0.0));
        let g_2 = GridLine::new("2", vec2(-2000.0, 8000.0), vec2(14000.0, 8000.0));
        doc.grids.insert(g_a.id, g_a);
        doc.grids.insert(g_b.id, g_b);
        doc.grids.insert(g_1.id, g_1);
        doc.grids.insert(g_2.id, g_2);

        // Views
        let v_floor1 = ViewDef::new_floor_plan("Floor Plan: Level 1", lvl1_id);
        let v_floor1_id = v_floor1.id;
        doc.views.insert(v_floor1_id, v_floor1);

        let v_floor2 = ViewDef::new_floor_plan("Floor Plan: Level 2", lvl2_id);
        doc.views.insert(v_floor2.id, v_floor2);

        let v_elev_e = ViewDef::new_elevation("Elevation: East", ViewKind::ElevationEast);
        let v_elev_n = ViewDef::new_elevation("Elevation: North", ViewKind::ElevationNorth);
        let v_elev_w = ViewDef::new_elevation("Elevation: West", ViewKind::ElevationWest);
        let v_elev_s = ViewDef::new_elevation("Elevation: South", ViewKind::ElevationSouth);
        doc.views.insert(v_elev_e.id, v_elev_e);
        doc.views.insert(v_elev_n.id, v_elev_n);
        doc.views.insert(v_elev_w.id, v_elev_w);
        doc.views.insert(v_elev_s.id, v_elev_s);

        let v_3d = ViewDef::new_3d("3D View");
        let v_3d_id = v_3d.id;
        doc.views.insert(v_3d_id, v_3d);

        doc.active_view_id = v_floor1_id;

        // 3. Walls (12m x 8m rectangular envelope)
        let mut w1 = Wall::new(vec2(0.0, 0.0), vec2(12000.0, 0.0), lvl1_id);
        w1.top_level = Some(lvl2_id);
        w1.thickness = 250.0;
        let w1_id = w1.id;
        doc.elements.insert(w1_id, Element::Wall(w1));

        let mut w2 = Wall::new(vec2(12000.0, 0.0), vec2(12000.0, 8000.0), lvl1_id);
        w2.top_level = Some(lvl2_id);
        w2.thickness = 250.0;
        let w2_id = w2.id;
        doc.elements.insert(w2_id, Element::Wall(w2));

        let mut w3 = Wall::new(vec2(12000.0, 8000.0), vec2(0.0, 8000.0), lvl1_id);
        w3.top_level = Some(lvl2_id);
        w3.thickness = 250.0;
        let w3_id = w3.id;
        doc.elements.insert(w3_id, Element::Wall(w3));

        let mut w4 = Wall::new(vec2(0.0, 8000.0), vec2(0.0, 0.0), lvl1_id);
        w4.top_level = Some(lvl2_id);
        w4.thickness = 250.0;
        let w4_id = w4.id;
        doc.elements.insert(w4_id, Element::Wall(w4));

        let mut w_int = Wall::new(vec2(6000.0, 0.0), vec2(6000.0, 8000.0), lvl1_id);
        w_int.top_level = Some(lvl2_id);
        w_int.thickness = 150.0;
        let w_int_id = w_int.id;
        doc.elements.insert(w_int_id, Element::Wall(w_int));

        let slab_pts = vec![
            vec2(-125.0, -125.0),
            vec2(12125.0, -125.0),
            vec2(12125.0, 8125.0),
            vec2(-125.0, 8125.0),
        ];
        let mut slab1 = Slab::new(slab_pts, lvl1_id);
        slab1.thickness = 250.0;
        slab1.slab_type_name = "Foundation Slab 250mm".into();
        doc.elements.insert(slab1.id, Element::Slab(slab1));

        let ceil_pts = vec![
            vec2(125.0, 125.0),
            vec2(11875.0, 125.0),
            vec2(11875.0, 7875.0),
            vec2(125.0, 7875.0),
        ];
        let mut ceiling1 = Ceiling::new(ceil_pts, lvl1_id);
        ceiling1.height_above_level = 2700.0;
        doc.elements.insert(ceiling1.id, Element::Ceiling(ceiling1));

        let roof_pts = vec![
            vec2(-250.0, -250.0),
            vec2(12250.0, -250.0),
            vec2(12250.0, 8250.0),
            vec2(-250.0, 8250.0),
        ];
        let mut slab2 = Slab::new(roof_pts, lvl2_id);
        slab2.thickness = 200.0;
        slab2.slab_type_name = "Roof Slab - Concrete 200mm".into();
        doc.elements.insert(slab2.id, Element::Slab(slab2));

        for col_pt in [vec2(6000.0, 2000.0), vec2(6000.0, 6000.0)] {
            let mut col = Column::new(col_pt, lvl1_id);
            col.is_structural = true;
            col.width = 400.0;
            col.depth = 400.0;
            col.top_level = None;
            col.unconnected_height = 3300.0;
            col.column_type_name = "Concrete Column 400x400".into();
            doc.elements.insert(col.id, Element::Column(col));
        }

        let mut beam1 = Beam::new(vec2(6000.0, 0.0), vec2(6000.0, 8000.0), lvl2_id);
        beam1.width = 300.0;
        beam1.depth = 500.0;
        beam1.z_offset = -200.0;
        beam1.beam_type_name = "Concrete Rect 300x500".into();
        doc.elements.insert(beam1.id, Element::Beam(beam1));

        let mut d1 = Door::new(w1_id, 3000.0);
        d1.width = 1000.0;
        d1.height = 2100.0;
        d1.door_type_name = "Entrance Single 1000x2100mm".into();
        doc.elements.insert(d1.id, Element::Door(d1));

        let mut d2 = Door::new(w_int_id, 4000.0);
        d2.width = 900.0;
        d2.height = 2100.0;
        doc.elements.insert(d2.id, Element::Door(d2));

        for offset in [7000.0, 10000.0] {
            let mut win = Window::new(w1_id, offset);
            win.width = 1500.0;
            win.height = 1400.0;
            win.sill_height = 900.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        for offset in [3000.0, 9000.0] {
            let mut win = Window::new(w3_id, offset);
            win.width = 1500.0;
            win.height = 1400.0;
            win.sill_height = 900.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        let l1_view = doc.views.values().find(|v| v.name.contains("Level 1")).map(|v| v.id).unwrap();
        let dim1 = Dimension::new(l1_view, vec2(0.0, -500.0), vec2(12000.0, -500.0), 300.0);
        doc.elements.insert(dim1.id, Element::Dimension(dim1));

        let mut sheet1 = Sheet::new("A101", "Level 1 Plan & 3D Overview", PaperSize::A1);
        sheet1.viewports.push(PlacedViewport {
            id: ViewportId::new(),
            view_id: l1_view,
            position_mm: vec2(60.0, 160.0),
            width_mm: 340.0,
            height_mm: 240.0,
            scale: 100,
            title: "FLOOR PLAN - LEVEL 1".into(),
            number: 1,
            show_title: true,
        });
        if let Some(v_3d) = doc.views.values().find(|v| v.kind == ViewKind::ThreeD) {
            sheet1.viewports.push(PlacedViewport {
                id: ViewportId::new(),
                view_id: v_3d.id,
                position_mm: vec2(440.0, 160.0),
                width_mm: 340.0,
                height_mm: 240.0,
                scale: 100,
                title: "3D AXONOMETRIC OVERVIEW".into(),
                number: 2,
                show_title: true,
            });
        }
        doc.sheets.insert(sheet1.id, sheet1);

        doc
    }
}
