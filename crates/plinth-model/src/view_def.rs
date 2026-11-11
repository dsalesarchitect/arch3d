//! View definitions: Floor Plans, Reflected Ceiling Plans, Elevations, Sections, 3D, and Details.

use crate::id::{LevelId, ViewId};
use plinth_geom::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewKind {
    FloorPlan,
    CeilingPlan,
    ElevationNorth,
    ElevationSouth,
    ElevationEast,
    ElevationWest,
    Section,
    ThreeD,
    Detail,
}

impl ViewKind {
    pub fn is_plan_or_rcp(&self) -> bool {
        matches!(self, ViewKind::FloorPlan | ViewKind::CeilingPlan)
    }

    pub fn is_elevation(&self) -> bool {
        matches!(
            self,
            ViewKind::ElevationNorth
                | ViewKind::ElevationSouth
                | ViewKind::ElevationEast
                | ViewKind::ElevationWest
        )
    }

    pub fn is_section(&self) -> bool {
        matches!(self, ViewKind::Section)
    }

    pub fn is_elevation_or_section(&self) -> bool {
        self.is_elevation() || self.is_section()
    }

    pub fn is_3d(&self) -> bool {
        matches!(self, ViewKind::ThreeD)
    }
}

/// Camera state for interactive 3D navigation (orbit, pan, zoom).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera3D {
    pub target_x: f64,
    pub target_y: f64,
    pub target_z: f64,
    /// Distance from target in world mm.
    pub distance: f64,
    /// Azimuth angle in radians (horizontal orbit).
    pub yaw: f64,
    /// Pitch angle in radians (vertical orbit, -89° to +89°).
    pub pitch: f64,
    /// Field of view in degrees (0.0 means orthographic / axonometric projection).
    pub fov_degrees: f64,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            target_x: 5000.0,
            target_y: 4000.0,
            target_z: 1500.0,
            distance: 25000.0,
            yaw: -std::f64::consts::FRAC_PI_4, // 45 degrees
            pitch: 0.55,                      // ~31.5 degrees (classic Revit isometric angle)
            fov_degrees: 0.0,                 // standard orthographic 3D
        }
    }
}

/// View definition within the project browser.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewDef {
    pub id: ViewId,
    pub name: String,
    pub kind: ViewKind,
    /// Associated building level (for plans and ceiling plans).
    pub level_id: Option<LevelId>,
    /// Cut plane elevation relative to level in mm (default 1200mm for floor plans, 2300mm for RCP).
    pub cut_offset: f64,
    /// View range top offset above level in mm (e.g. 2300mm).
    pub top_offset: f64,
    /// View range bottom offset below level in mm (e.g. 0.0mm).
    pub bottom_offset: f64,
    /// Standard view scale: 100 means 1:100 scale, 50 means 1:50 scale.
    pub scale: u32,
    /// For sections: the 2D cut line in plan (start, end).
    pub section_line: Option<(Vec2, Vec2)>,
    /// For sections: viewing depth in mm.
    pub section_depth: f64,
    /// 3D camera parameters (if kind == ThreeD).
    pub camera_3d: Option<Camera3D>,
}

impl ViewDef {
    pub fn new_floor_plan(name: impl Into<String>, level_id: LevelId) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind: ViewKind::FloorPlan,
            level_id: Some(level_id),
            cut_offset: 1200.0,
            top_offset: 2300.0,
            bottom_offset: 0.0,
            scale: 100,
            section_line: None,
            section_depth: 0.0,
            camera_3d: None,
        }
    }

    pub fn new_ceiling_plan(name: impl Into<String>, level_id: LevelId) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind: ViewKind::CeilingPlan,
            level_id: Some(level_id),
            cut_offset: 2300.0,
            top_offset: 3000.0,
            bottom_offset: 1200.0,
            scale: 100,
            section_line: None,
            section_depth: 0.0,
            camera_3d: None,
        }
    }

    pub fn new_elevation(name: impl Into<String>, kind: ViewKind) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind,
            level_id: None,
            cut_offset: 0.0,
            top_offset: 0.0,
            bottom_offset: 0.0,
            scale: 100,
            section_line: None,
            section_depth: 100_000.0, // 100m default view depth
            camera_3d: None,
        }
    }

    pub fn new_section(name: impl Into<String>, start: Vec2, end: Vec2) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind: ViewKind::Section,
            level_id: None,
            cut_offset: 0.0,
            top_offset: 0.0,
            bottom_offset: 0.0,
            scale: 50,
            section_line: Some((start, end)),
            section_depth: 15_000.0, // 15m view depth
            camera_3d: None,
        }
    }

    pub fn new_3d(name: impl Into<String>) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind: ViewKind::ThreeD,
            level_id: None,
            cut_offset: 0.0,
            top_offset: 0.0,
            bottom_offset: 0.0,
            scale: 100,
            section_line: None,
            section_depth: 0.0,
            camera_3d: Some(Camera3D::default()),
        }
    }

    pub fn new_detail(name: impl Into<String>) -> Self {
        Self {
            id: ViewId::new(),
            name: name.into(),
            kind: ViewKind::Detail,
            level_id: None,
            cut_offset: 0.0,
            top_offset: 0.0,
            bottom_offset: 0.0,
            scale: 20, // 1:20 typical detail scale
            section_line: None,
            section_depth: 0.0,
            camera_3d: None,
        }
    }
}
