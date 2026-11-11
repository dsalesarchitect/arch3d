//! Central Document: single source of truth for all BIM elements, levels, grids, materials, and views.

use crate::element::{
    Beam, Ceiling, Column, Dimension, Door, Element, Slab, Wall, WallOpening, Window,
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

    /// Generate complete sample building model for instant review and testing.
    pub fn create_sample_building() -> Self {
        let mut doc = Self::new_empty();

        let lvl1_id = doc.levels.values().find(|l| l.name == "Level 1").map(|l| l.id).unwrap();
        let lvl2_id = doc.levels.values().find(|l| l.name == "Level 2").map(|l| l.id).unwrap();
        let _roof_id = doc.levels.values().find(|l| l.name == "Roof").map(|l| l.id).unwrap();
        doc.active_level_id = lvl1_id;

        // 1. Grid lines: 1, 2, 3 horizontal and A, B vertical
        doc.grids.insert(
            GridId::new(),
            GridLine::new("A", vec2(0.0, -1500.0), vec2(0.0, 9500.0)),
        );
        doc.grids.insert(
            GridId::new(),
            GridLine::new("B", vec2(6000.0, -1500.0), vec2(6000.0, 9500.0)),
        );
        doc.grids.insert(
            GridId::new(),
            GridLine::new("C", vec2(12000.0, -1500.0), vec2(12000.0, 9500.0)),
        );
        doc.grids.insert(
            GridId::new(),
            GridLine::new("1", vec2(-1500.0, 0.0), vec2(13500.0, 0.0)),
        );
        doc.grids.insert(
            GridId::new(),
            GridLine::new("2", vec2(-1500.0, 8000.0), vec2(13500.0, 8000.0)),
        );

        // 2. Exterior Walls (Level 1, stopping flush at underside of Level 2 roof slab)
        let mut w1 = Wall::new(vec2(0.0, 0.0), vec2(12000.0, 0.0), lvl1_id);
        w1.top_level = None;
        w1.unconnected_height = 3300.0;
        w1.thickness = 250.0;
        w1.wall_type_name = "Exterior - Brick on CMU 250mm".into();
        let w1_id = w1.id;
        doc.elements.insert(w1_id, Element::Wall(w1));

        let mut w2 = Wall::new(vec2(12000.0, 0.0), vec2(12000.0, 8000.0), lvl1_id);
        w2.top_level = None;
        w2.unconnected_height = 3300.0;
        w2.thickness = 250.0;
        w2.wall_type_name = "Exterior - Brick on CMU 250mm".into();
        let w2_id = w2.id;
        doc.elements.insert(w2_id, Element::Wall(w2));

        let mut w3 = Wall::new(vec2(12000.0, 8000.0), vec2(0.0, 8000.0), lvl1_id);
        w3.top_level = None;
        w3.unconnected_height = 3300.0;
        w3.thickness = 250.0;
        w3.wall_type_name = "Exterior - Brick on CMU 250mm".into();
        let w3_id = w3.id;
        doc.elements.insert(w3_id, Element::Wall(w3));

        let mut w4 = Wall::new(vec2(0.0, 8000.0), vec2(0.0, 0.0), lvl1_id);
        w4.top_level = None;
        w4.unconnected_height = 3300.0;
        w4.thickness = 250.0;
        w4.wall_type_name = "Exterior - Brick on CMU 250mm".into();
        let w4_id = w4.id;
        doc.elements.insert(w4_id, Element::Wall(w4));

        // 3. Interior Partition Wall (stops at ceiling 2700mm)
        let mut w_int = Wall::new(vec2(6000.0, 0.0), vec2(6000.0, 8000.0), lvl1_id);
        w_int.top_level = None;
        w_int.unconnected_height = 2700.0;
        w_int.thickness = 150.0;
        w_int.wall_type_name = "Interior - Partition 150mm".into();
        let w_int_id = w_int.id;
        doc.elements.insert(w_int_id, Element::Wall(w_int));

        // 4. Ground Floor Slab (Level 1) — spans full outer building footprint (-125..12125, -125..8125)
        let slab_pts = vec![
            vec2(-125.0, -125.0),
            vec2(12125.0, -125.0),
            vec2(12125.0, 8125.0),
            vec2(-125.0, 8125.0),
        ];
        let mut slab1 = Slab::new(slab_pts.clone(), lvl1_id);
        slab1.thickness = 250.0;
        slab1.slab_type_name = "Foundation Slab 250mm".into();
        doc.elements.insert(slab1.id, Element::Slab(slab1));

        // 5. Ceiling on Level 1 (interior room footprint)
        let ceil_pts = vec![
            vec2(125.0, 125.0),
            vec2(11875.0, 125.0),
            vec2(11875.0, 7875.0),
            vec2(125.0, 7875.0),
        ];
        let mut ceiling1 = Ceiling::new(ceil_pts, lvl1_id);
        ceiling1.height_above_level = 2700.0;
        doc.elements.insert(ceiling1.id, Element::Ceiling(ceiling1));

        // 6. Upper Floor Slab (Level 2) - Roof slab covering entire building envelope with 250mm clear eave overhang
        let roof_pts = vec![
            vec2(-250.0, -250.0),
            vec2(12250.0, -250.0),
            vec2(12250.0, 8250.0),
            vec2(-250.0, 8250.0),
        ];
        let mut slab2 = Slab::new(roof_pts.clone(), lvl2_id);
        slab2.thickness = 200.0;
        slab2.slab_type_name = "Roof Slab - Concrete 200mm".into();
        doc.elements.insert(slab2.id, Element::Slab(slab2));

        // 7. Structural Columns in middle corridor (stops flush at underside of roof slab 3300mm)
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

        // 8. Structural Beams (hangs below the 200mm roof slab, Z in [2800, 3300])
        let mut beam1 = Beam::new(vec2(6000.0, 0.0), vec2(6000.0, 8000.0), lvl2_id);
        beam1.width = 300.0;
        beam1.depth = 500.0;
        beam1.z_offset = -200.0;
        beam1.beam_type_name = "Concrete Rect 300x500".into();
        doc.elements.insert(beam1.id, Element::Beam(beam1));

        // 9. Doors
        // Entrance door on South wall
        let mut d1 = Door::new(w1_id, 3000.0);
        d1.width = 1000.0;
        d1.height = 2100.0;
        d1.door_type_name = "Entrance Single 1000x2100mm".into();
        doc.elements.insert(d1.id, Element::Door(d1));

        // Interior door on Partition wall
        let mut d2 = Door::new(w_int_id, 4000.0);
        d2.width = 900.0;
        d2.height = 2100.0;
        doc.elements.insert(d2.id, Element::Door(d2));

        // 10. Windows
        // Windows on South wall
        for offset in [7000.0, 10000.0] {
            let mut win = Window::new(w1_id, offset);
            win.width = 1500.0;
            win.height = 1400.0;
            win.sill_height = 900.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        // Windows on North wall
        for offset in [3000.0, 9000.0] {
            let mut win = Window::new(w3_id, offset);
            win.width = 1500.0;
            win.height = 1400.0;
            win.sill_height = 900.0;
            doc.elements.insert(win.id, Element::Window(win));
        }

        // 11. Dimensions on Level 1
        let l1_view = doc.views.values().find(|v| v.name.contains("Level 1")).map(|v| v.id).unwrap();
        let dim1 = Dimension::new(l1_view, vec2(0.0, -500.0), vec2(12000.0, -500.0), 300.0);
        doc.elements.insert(dim1.id, Element::Dimension(dim1));

        // 12. Architectural Sheet Layout (Multi-Sheet Management)
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
