//! Cleanroom Architecture & Structure Tool Configuration Studio (`tool_config.rs`).
//!
//! Opens when double-clicking any Architecture or Structure tool on the Ribbon (or via
//! Properties / Status Bar). Provides a unified, zero-nesting workflow for:
//! - Browsing rich architectural & structural Type Libraries
//! - One-click **Duplicate Type** & **New Custom Type** creation
//! - Editing multi-layer compound assemblies (Wall, Floor, Slab, Roof, Ceiling) with live
//!   thermal resistance ($R$) and transmittance ($U$-value) calculation
//! - Configuring Stair ergonomics (Blondel $2R + G$ gauge & `StairStyle`), Ramp accessibility
//!   grade (`1:X` & `RampStyle`), Railing safety spacing, Structural profiles, and Openings
//! - Setting default Base/Top Level constraints & offsets and IFC4 `Pset_*Common` standards.

use crate::icons::{draw_icon, IconKind};
use crate::theme::AppTheme;
use crate::tool::Tool;
use egui::{
    Align2, CollapsingHeader, Color32, ComboBox, Context, DragValue, FontId, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2, Window as EguiWindow,
};
use plinth_model::{
    AssemblyLayer, Beam, Ceiling, Column, CurtainWall, Document, Door, Element, ElementId, Fascia, Footing,
    FootingStyle, Gutter, LayerFunction, LevelId, Railing, Ramp, RampStyle, Roof, RoofShape, Slab,
    Stair, StairStyle, Wall, Window,
};
use std::collections::{BTreeSet, HashMap};

/// All 18 Architecture & Structure tool categories configurable in the Type & Assembly Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolCategory {
    // Architecture (13)
    WallArch,
    CurtainWall,
    Door,
    Window,
    ColumnArch,
    FloorArch,
    Ceiling,
    Stair,
    Ramp,
    Railing,
    Roof,
    Fascia,
    Gutter,
    // Structure (5)
    ColumnStruct,
    Footing,
    Beam,
    WallStruct,
    SlabStruct,
    // Site & Civil (3)
    Road,
    Sidewalk,
    Pathwalk,
    // Annotation & Drafting (6)
    DimensionLinear,
    DimensionCircular,
    DimensionAngular,
    TextNote,
    Leader,
    DetailLine,
}

impl ToolCategory {
    pub const ALL_ARCH: [ToolCategory; 13] = [
        Self::WallArch,
        Self::CurtainWall,
        Self::Door,
        Self::Window,
        Self::ColumnArch,
        Self::FloorArch,
        Self::Ceiling,
        Self::Stair,
        Self::Ramp,
        Self::Railing,
        Self::Roof,
        Self::Fascia,
        Self::Gutter,
    ];

    pub const ALL_STRUCT: [ToolCategory; 5] = [
        Self::ColumnStruct,
        Self::Footing,
        Self::Beam,
        Self::WallStruct,
        Self::SlabStruct,
    ];

    pub const ALL_SITE: [ToolCategory; 3] = [
        Self::Road,
        Self::Sidewalk,
        Self::Pathwalk,
    ];

    pub const ALL_ANNOTATION: [ToolCategory; 6] = [
        Self::DimensionLinear,
        Self::DimensionCircular,
        Self::DimensionAngular,
        Self::TextNote,
        Self::Leader,
        Self::DetailLine,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Self::WallArch => "Architectural Wall",
            Self::CurtainWall => "Curtain Wall & Storefront",
            Self::Door => "Door Assembly",
            Self::Window => "Window & Glazing",
            Self::ColumnArch => "Architectural Column",
            Self::FloorArch => "Architectural Floor",
            Self::Ceiling => "Reflected Ceiling",
            Self::Stair => "Circulation Stair",
            Self::Ramp => "Inclined Ramp",
            Self::Railing => "Guardrail & Balustrade",
            Self::Roof => "Roof System",
            Self::Fascia => "Eave Fascia",
            Self::Gutter => "Rainwater Gutter",
            Self::ColumnStruct => "Structural Column",
            Self::Footing => "Foundation Footing",
            Self::Beam => "Structural Beam",
            Self::WallStruct => "Structural Wall",
            Self::SlabStruct => "Structural Slab",
            Self::DimensionLinear => "Linear Dimension",
            Self::DimensionCircular => "Circular / Radial Dimension",
            Self::DimensionAngular => "Angular Dimension",
            Self::TextNote => "Text Annotation Note",
            Self::Leader => "Leader Callout",
            Self::DetailLine => "2D Detail Drafting Line",
            Self::Road => "Road Corridor",
            Self::Sidewalk => "Sidewalk Walkway",
            Self::Pathwalk => "Pathwalk Paver Trail",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::WallArch => "Wall",
            Self::CurtainWall => "Curtain Wall",
            Self::Door => "Door",
            Self::Window => "Window",
            Self::ColumnArch => "Column",
            Self::FloorArch => "Floor",
            Self::Ceiling => "Ceiling",
            Self::Stair => "Stair",
            Self::Ramp => "Ramp",
            Self::Railing => "Railing",
            Self::Roof => "Roof",
            Self::Fascia => "Fascia",
            Self::Gutter => "Gutter",
            Self::ColumnStruct => "Col (Str)",
            Self::Footing => "Footing",
            Self::Beam => "Beam",
            Self::WallStruct => "Wall (Str)",
            Self::SlabStruct => "Slab (Str)",
            Self::DimensionLinear => "Linear Dim",
            Self::DimensionCircular => "Circular Dim",
            Self::DimensionAngular => "Angular Dim",
            Self::TextNote => "Text Note",
            Self::Leader => "Leader",
            Self::DetailLine => "Detail Line",
            Self::Road => "Road",
            Self::Sidewalk => "Sidewalk",
            Self::Pathwalk => "Pathwalk",
        }
    }

    pub fn is_structural_discipline(&self) -> bool {
        matches!(
            self,
            Self::ColumnStruct | Self::Footing | Self::Beam | Self::WallStruct | Self::SlabStruct
        )
    }

    pub fn is_site_discipline(&self) -> bool {
        matches!(self, Self::Road | Self::Sidewalk | Self::Pathwalk)
    }

    pub fn is_annotation_discipline(&self) -> bool {
        matches!(
            self,
            Self::DimensionLinear
                | Self::DimensionCircular
                | Self::DimensionAngular
                | Self::TextNote
                | Self::Leader
                | Self::DetailLine
        )
    }

    pub fn icon_kind(&self) -> IconKind {
        match self {
            Self::WallArch | Self::WallStruct => IconKind::Wall,
            Self::CurtainWall => IconKind::CurtainWall,
            Self::Door => IconKind::Door,
            Self::Window => IconKind::Window,
            Self::ColumnArch | Self::ColumnStruct => IconKind::Column,
            Self::FloorArch | Self::SlabStruct => IconKind::Slab,
            Self::Ceiling => IconKind::Ceiling,
            Self::Stair => IconKind::Stair,
            Self::Ramp => IconKind::Ramp,
            Self::Railing => IconKind::Railing,
            Self::Roof => IconKind::Roof,
            Self::Fascia => IconKind::Fascia,
            Self::Gutter => IconKind::Gutter,
            Self::Footing => IconKind::Footing,
            Self::Beam => IconKind::Beam,
            Self::DimensionLinear => IconKind::Dimension,
            Self::DimensionCircular => IconKind::CircularDimension,
            Self::DimensionAngular => IconKind::AngularDimension,
            Self::TextNote => IconKind::TextAnnotation,
            Self::Leader => IconKind::Leader,
            Self::DetailLine => IconKind::DetailLine,
            Self::Road => IconKind::Road,
            Self::Sidewalk => IconKind::Sidewalk,
            Self::Pathwalk => IconKind::Pathwalk,
        }
    }

    pub fn discipline_label(&self) -> &'static str {
        if self.is_site_discipline() {
            "Site & Civil • IFC4"
        } else if self.is_annotation_discipline() {
            "Annotation & Drafting • ISO 128"
        } else if self.is_structural_discipline() {
            "Structure • IFC4"
        } else {
            "Architecture • IFC4"
        }
    }

    pub fn has_compound_layers(&self) -> bool {
        matches!(
            self,
            Self::WallArch
                | Self::WallStruct
                | Self::FloorArch
                | Self::SlabStruct
                | Self::Roof
                | Self::Ceiling
                | Self::Road
                | Self::Sidewalk
                | Self::Pathwalk
        )
    }

    pub fn pset_name(&self) -> &'static str {
        match self {
            Self::WallArch | Self::WallStruct => "Pset_WallCommon",
            Self::CurtainWall => "Pset_CurtainWallCommon",
            Self::Door => "Pset_DoorCommon",
            Self::Window => "Pset_WindowCommon",
            Self::ColumnArch | Self::ColumnStruct => "Pset_ColumnCommon",
            Self::Footing => "Pset_FootingCommon",
            Self::Beam => "Pset_BeamCommon",
            Self::FloorArch | Self::SlabStruct => "Pset_SlabCommon",
            Self::Ceiling => "Pset_CeilingCommon",
            Self::Stair => "Pset_StairCommon",
            Self::Ramp => "Pset_RampCommon",
            Self::Railing => "Pset_RailingCommon",
            Self::Roof => "Pset_RoofCommon",
            Self::Fascia | Self::Gutter => "Pset_BuildingElementProxyCommon",
            Self::Road => "Pset_RoadCommon",
            Self::Sidewalk => "Pset_SidewalkCommon",
            Self::Pathwalk => "Pset_PathwalkCommon",
            Self::DimensionLinear | Self::DimensionCircular | Self::DimensionAngular => "Pset_AnnotationDimensionCommon",
            Self::TextNote | Self::Leader | Self::DetailLine => "Pset_AnnotationDraftingCommon",
        }
    }

    pub fn from_tool(tool: &Tool) -> Option<Self> {
        match tool {
            Tool::DrawWall {
                is_structural: false,
                ..
            } => Some(Self::WallArch),
            Tool::DrawWall {
                is_structural: true,
                ..
            } => Some(Self::WallStruct),
            Tool::DrawCurtainWall { .. } => Some(Self::CurtainWall),
            Tool::PlaceDoor => Some(Self::Door),
            Tool::PlaceWindow => Some(Self::Window),
            Tool::DrawColumn {
                is_structural: false,
            } => Some(Self::ColumnArch),
            Tool::DrawColumn {
                is_structural: true,
            } => Some(Self::ColumnStruct),
            Tool::DrawFooting => Some(Self::Footing),
            Tool::DrawBeam { .. } => Some(Self::Beam),
            Tool::DrawSlab {
                is_structural: false,
                ..
            } => Some(Self::FloorArch),
            Tool::DrawSlab {
                is_structural: true,
                ..
            } => Some(Self::SlabStruct),
            Tool::DrawCeiling { .. } => Some(Self::Ceiling),
            Tool::DrawStair { .. } => Some(Self::Stair),
            Tool::DrawRamp { .. } => Some(Self::Ramp),
            Tool::DrawRailing { .. } => Some(Self::Railing),
            Tool::DrawRoof { .. } => Some(Self::Roof),
            Tool::DrawFascia { .. } => Some(Self::Fascia),
            Tool::DrawGutter { .. } => Some(Self::Gutter),
            Tool::DrawDimension { .. } => Some(Self::DimensionLinear),
            Tool::DrawCircularDimension { .. } => Some(Self::DimensionCircular),
            Tool::DrawAngularDimension { .. } => Some(Self::DimensionAngular),
            Tool::DrawTextAnnotation { .. } => Some(Self::TextNote),
            Tool::DrawLeader { .. } => Some(Self::Leader),
            Tool::DrawDetailLine { .. } => Some(Self::DetailLine),
            Tool::DrawRoad { .. } => Some(Self::Road),
            Tool::DrawSidewalk { .. } => Some(Self::Sidewalk),
            Tool::DrawPathwalk { .. } => Some(Self::Pathwalk),
            _ => None,
        }
    }

    pub fn from_element(el: &Element) -> Option<Self> {
        match el {
            Element::Wall(w) => Some(if w.is_structural {
                Self::WallStruct
            } else {
                Self::WallArch
            }),
            Element::CurtainWall(_) => Some(Self::CurtainWall),
            Element::Door(_) => Some(Self::Door),
            Element::Window(_) => Some(Self::Window),
            Element::Column(c) => Some(if c.is_structural {
                Self::ColumnStruct
            } else {
                Self::ColumnArch
            }),
            Element::Footing(_) => Some(Self::Footing),
            Element::Beam(_) => Some(Self::Beam),
            Element::Slab(s) => Some(if s.is_structural {
                Self::SlabStruct
            } else {
                Self::FloorArch
            }),
            Element::Ceiling(_) => Some(Self::Ceiling),
            Element::Stair(_) => Some(Self::Stair),
            Element::Ramp(_) => Some(Self::Ramp),
            Element::Railing(_) => Some(Self::Railing),
            Element::Roof(_) => Some(Self::Roof),
            Element::Fascia(_) => Some(Self::Fascia),
            Element::Gutter(_) => Some(Self::Gutter),
            Element::Dimension(d) => match d.kind {
                plinth_model::DimensionKind::Linear => Some(Self::DimensionLinear),
                plinth_model::DimensionKind::Circular => Some(Self::DimensionCircular),
                plinth_model::DimensionKind::Angular => Some(Self::DimensionAngular),
            },
            Element::TextNote(_) => Some(Self::TextNote),
            Element::Leader(_) => Some(Self::Leader),
            Element::DetailLine(_) => Some(Self::DetailLine),
            Element::Road(_) => Some(Self::Road),
            Element::Sidewalk(_) => Some(Self::Sidewalk),
            Element::Pathwalk(_) => Some(Self::Pathwalk),
            _ => None,
        }
    }

    pub fn to_tool(&self, roof_shape: RoofShape) -> Tool {
        match self {
            Self::WallArch => Tool::DrawWall {
                start: None,
                is_structural: false,
            },
            Self::WallStruct => Tool::DrawWall {
                start: None,
                is_structural: true,
            },
            Self::CurtainWall => Tool::DrawCurtainWall { start: None },
            Self::Door => Tool::PlaceDoor,
            Self::Window => Tool::PlaceWindow,
            Self::ColumnArch => Tool::DrawColumn {
                is_structural: false,
            },
            Self::ColumnStruct => Tool::DrawColumn {
                is_structural: true,
            },
            Self::Footing => Tool::DrawFooting,
            Self::Beam => Tool::DrawBeam { start: None },
            Self::FloorArch => Tool::DrawSlab {
                points: Vec::new(),
                is_structural: false,
            },
            Self::SlabStruct => Tool::DrawSlab {
                points: Vec::new(),
                is_structural: true,
            },
            Self::Ceiling => Tool::DrawCeiling { points: Vec::new() },
            Self::Stair => Tool::DrawStair { start: None },
            Self::Ramp => Tool::DrawRamp { start: None },
            Self::Railing => Tool::DrawRailing { points: Vec::new() },
            Self::Roof => Tool::DrawRoof {
                shape: roof_shape,
                points: Vec::new(),
            },
            Self::Fascia => Tool::DrawFascia { points: Vec::new() },
            Self::Gutter => Tool::DrawGutter { points: Vec::new() },
            Self::DimensionLinear => Tool::DrawDimension { start: None, p2: None, locked_offset: None },
            Self::DimensionCircular => Tool::DrawCircularDimension { center: None },
            Self::DimensionAngular => Tool::DrawAngularDimension { apex: None, first_ray: None },
            Self::TextNote => Tool::DrawTextAnnotation { pos: None },
            Self::Leader => Tool::DrawLeader { target_pt: None, shoulder_pt: None },
            Self::DetailLine => Tool::DrawDetailLine { start: None },
            Self::Road => Tool::DrawRoad { points: Vec::new() },
            Self::Sidewalk => Tool::DrawSidewalk { points: Vec::new() },
            Self::Pathwalk => Tool::DrawPathwalk { points: Vec::new() },
        }
    }

    pub fn matches_element(&self, el: &Element) -> bool {
        matches!(
            (self, el),
            (Self::WallArch | Self::WallStruct, Element::Wall(_))
                | (Self::CurtainWall, Element::CurtainWall(_))
                | (Self::Door, Element::Door(_))
                | (Self::Window, Element::Window(_))
                | (Self::ColumnArch | Self::ColumnStruct, Element::Column(_))
                | (Self::Footing, Element::Footing(_))
                | (Self::Beam, Element::Beam(_))
                | (Self::FloorArch | Self::SlabStruct, Element::Slab(_))
                | (Self::Ceiling, Element::Ceiling(_))
                | (Self::Stair, Element::Stair(_))
                | (Self::Ramp, Element::Ramp(_))
                | (Self::Railing, Element::Railing(_))
                | (Self::Roof, Element::Roof(_))
                | (Self::Fascia, Element::Fascia(_))
                | (Self::Gutter, Element::Gutter(_))
                | (Self::DimensionLinear | Self::DimensionCircular | Self::DimensionAngular, Element::Dimension(_))
                | (Self::TextNote, Element::TextNote(_))
                | (Self::Leader, Element::Leader(_))
                | (Self::DetailLine, Element::DetailLine(_))
        )
    }
}

fn make_layer(func: LayerFunction, mat: &str, thickness: f64, lambda: f64) -> AssemblyLayer {
    AssemblyLayer::new(func.label(), func, mat, thickness, lambda)
}

/// Parametric type definition stored in the Type & Assembly Studio library.
#[derive(Clone, Debug)]
pub struct TypePreset {
    pub category: ToolCategory,
    pub type_name: String,
    pub tag_prefix: String,
    pub description: String,
    pub is_builtin: bool,

    // Compound assembly layers (Wall, Floor, Slab, Roof, Ceiling)
    pub layers: Vec<AssemblyLayer>,
    pub auto_sync_layers: bool,

    // Primary geometric parameters (mm / deg)
    pub thickness: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub sill_height: f64,
    pub is_round: bool,

    // Opening parameters
    pub operation_type: String,
    pub flipped_hand: bool,
    pub flipped_face: bool,
    pub is_fire_exit: bool,
    pub security_rating: String,
    pub shgc: f64,

    // Roof & Ceiling parameters
    pub roof_shape: RoofShape,
    pub slope_degrees: f64,
    pub overhang: f64,
    pub solar_reflectance_index: f64,
    pub snow_load_capacity: f64,
    pub grid_spacing_x: f64,
    pub grid_spacing_y: f64,
    pub light_reflectance: f64,

    // Stair, Ramp & Railing parameters
    pub stair_style: StairStyle,
    pub riser_count: usize,
    pub target_going: f64,
    pub waist_thickness: f64,
    pub ramp_style: RampStyle,
    pub footing_style: FootingStyle,
    pub soil_bearing_capacity: f64,
    pub concrete_grade: String,
    pub baluster_spacing: f64,
    pub mid_rail_count: usize,
    pub material_name: String,

    // Default Level Hosting & Offsets
    pub base_offset: f64,
    pub top_to_next_level: bool,
    pub top_offset: f64,
    pub unconnected_height: f64,

    // IFC4 Pset Common Standards
    pub is_structural: bool,
    pub is_external: bool,
    pub fire_rating: String,
    pub acoustic_rating: String,
    pub thermal_transmittance: f64,
    pub load_bearing_capacity: f64,

    // Annotation & Drafting Styling
    pub tick_mark: plinth_model::TickMark,
    pub font_size: f32,
    pub font_style: plinth_model::FontStyle,
    pub color: [u8; 4],
    pub dim_offset: f64,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub is_diameter: bool,
    pub continuous_string: bool,
    pub line_weight: f32,
    pub line_style: String,

    // Road, Sidewalk & Pathwalk parameters
    pub lanes: u32,
    pub lane_width: f64,
    pub road_curb: plinth_model::RoadCurb,
    pub curb_width: f64,
    pub curb_height: f64,
    pub crown_slope_pct: f64,
    pub has_centerline: bool,
    pub sidewalk_edge: plinth_model::SidewalkEdgeProfile,
    pub has_top_groove: bool,
    pub groove_spacing: f64,
    pub groove_depth: f64,
    pub pathwalk_profile: plinth_model::PathwalkProfile,
    pub has_edging: bool,
    pub edging_width: f64,
    pub groove_width: f64,
}

impl TypePreset {
    pub fn base(category: ToolCategory, name: &str, prefix: &str, desc: &str) -> Self {
        Self {
            category,
            type_name: name.to_string(),
            tag_prefix: prefix.to_string(),
            description: desc.to_string(),
            is_builtin: true,
            layers: Vec::new(),
            auto_sync_layers: true,
            thickness: 200.0,
            width: 900.0,
            depth: 400.0,
            height: 2100.0,
            sill_height: 0.0,
            is_round: false,
            operation_type: "SINGLE_SWING_LEFT".to_string(),
            flipped_hand: false,
            flipped_face: false,
            is_fire_exit: false,
            security_rating: "Standard".to_string(),
            shgc: 0.40,
            roof_shape: RoofShape::Hip,
            slope_degrees: 30.0,
            overhang: 450.0,
            solar_reflectance_index: 42.0,
            snow_load_capacity: 1.5,
            grid_spacing_x: 600.0,
            grid_spacing_y: 600.0,
            light_reflectance: 0.85,
            stair_style: StairStyle::Monolithic,
            riser_count: 18,
            target_going: 280.0,
            waist_thickness: 160.0,
            ramp_style: RampStyle::SlopedSlab,
            footing_style: FootingStyle::Pad,
            soil_bearing_capacity: 200.0,
            concrete_grade: "C30/37".to_string(),
            baluster_spacing: 100.0,
            mid_rail_count: 2,
            material_name: "Architectural Concrete".to_string(),
            base_offset: 0.0,
            top_to_next_level: false,
            top_offset: 0.0,
            unconnected_height: 3000.0,
            is_structural: category.is_structural_discipline(),
            is_external: true,
            fire_rating: "REI 60".to_string(),
            acoustic_rating: "Rw 48 dB".to_string(),
            thermal_transmittance: 0.32,
            load_bearing_capacity: 25.0,
            tick_mark: plinth_model::TickMark::Tick45,
            font_size: 2.5,
            font_style: plinth_model::FontStyle::Regular,
            color: [30, 30, 30, 255],
            dim_offset: 600.0,
            prefix: None,
            suffix: None,
            is_diameter: false,
            continuous_string: true,
            line_weight: 0.25,
            line_style: "Medium".to_string(),
            lanes: 2,
            lane_width: 3650.0,
            road_curb: plinth_model::RoadCurb::Barrier,
            curb_width: 150.0,
            curb_height: 150.0,
            crown_slope_pct: 2.0,
            has_centerline: true,
            sidewalk_edge: plinth_model::SidewalkEdgeProfile::Rounded { radius: 25.0 },
            has_top_groove: true,
            groove_spacing: 1500.0,
            groove_depth: 10.0,
            pathwalk_profile: plinth_model::PathwalkProfile::PaverBlocks,
            has_edging: true,
            edging_width: 100.0,
            groove_width: 8.0,
        }
    }

    /// Computes total layer thickness (mm) and steady-state thermal transmittance $U$ ($\text{W/m}^2\text{K}$).
    pub fn sync_from_layers(&mut self) {
        if self.layers.is_empty() || !self.auto_sync_layers {
            return;
        }
        let total_t: f64 = self.layers.iter().map(|l| l.thickness.max(0.0)).sum();
        if total_t > 1.0 {
            self.thickness = total_t;
        }
        let r_surface = if self.is_external { 0.17 } else { 0.26 }; // R_si + R_se (m²K/W)
        let r_layers: f64 = self.layers.iter().map(|l| l.thermal_resistance()).sum();
        let r_total = (r_surface + r_layers).max(0.10);
        self.thermal_transmittance = ((1.0 / r_total) * 100.0).round() / 100.0;
    }

    pub fn summary_badge(&self) -> String {
        match self.category {
            ToolCategory::WallArch
            | ToolCategory::WallStruct
            | ToolCategory::FloorArch
            | ToolCategory::SlabStruct => {
                format!(
                    "{:.0} mm • {} Layers • U={:.2}",
                    self.thickness,
                    self.layers.len(),
                    self.thermal_transmittance
                )
            }
            ToolCategory::CurtainWall => format!(
                "{:.0}×{:.0} Grid • {:.0}mm Glass",
                self.grid_spacing_x, self.grid_spacing_y, self.thickness
            ),
            ToolCategory::Roof => format!(
                "{:?} {:.0}° • {:.0} mm",
                self.roof_shape, self.slope_degrees, self.thickness
            ),
            ToolCategory::Ceiling => format!(
                "{:.0}×{:.0} Grid • +{:.0} mm",
                self.grid_spacing_x, self.grid_spacing_y, self.base_offset
            ),
            ToolCategory::Door => format!("{:.0} × {:.0} mm", self.width, self.height),
            ToolCategory::Window => format!(
                "{:.0} × {:.0} mm (Sill {:.0})",
                self.width, self.height, self.sill_height
            ),
            ToolCategory::ColumnArch | ToolCategory::ColumnStruct => {
                if self.is_round {
                    format!("Ø {:.0} mm", self.width)
                } else {
                    format!("{:.0} × {:.0} mm", self.width, self.depth)
                }
            }
            ToolCategory::Footing => format!(
                "{} • {:.0}×{:.0}×{:.0} mm",
                self.footing_style.label(),
                self.width,
                self.depth,
                self.thickness
            ),
            ToolCategory::Beam => format!("{:.0} × {:.0} mm", self.width, self.depth),
            ToolCategory::Stair => format!(
                "{} • {}R • W {:.0} mm",
                self.stair_style.label(),
                self.riser_count,
                self.width
            ),
            ToolCategory::Ramp => format!(
                "{} • W {:.0} mm",
                self.ramp_style.label(),
                self.width
            ),
            ToolCategory::Railing => format!(
                "H {:.0} mm • @ {:.0} mm",
                self.unconnected_height, self.baluster_spacing
            ),
            ToolCategory::Fascia => format!("{:.0} × {:.0} mm", self.height, self.thickness),
            ToolCategory::Gutter => format!("{:.0} × {:.0} mm", self.width, self.depth),
            ToolCategory::DimensionLinear | ToolCategory::DimensionCircular | ToolCategory::DimensionAngular => {
                format!("Font {:.1}mm • Tick {:?}", self.font_size, self.tick_mark)
            }
            ToolCategory::TextNote => format!("Font {:.1}mm • {:?}", self.font_size, self.font_style),
            ToolCategory::Leader => format!("Font {:.1}mm • Tick {:?}", self.font_size, self.tick_mark),
            ToolCategory::DetailLine => format!("Weight {:.2}mm • {}", self.line_weight, self.line_style),
            ToolCategory::Road => format!(
                "{:.1}m Wide • {} Lanes • Curb: {:?}",
                self.width / 1000.0,
                self.lanes,
                self.road_curb
            ),
            ToolCategory::Sidewalk => format!(
                "{:.1}m Wide • Edge: {:?} • Grooves",
                self.width / 1000.0,
                match self.sidewalk_edge {
                    plinth_model::SidewalkEdgeProfile::Rounded { radius } => format!("R{:.0}", radius),
                    plinth_model::SidewalkEdgeProfile::Square => "Square".into(),
                    plinth_model::SidewalkEdgeProfile::Beveled { chamfer } => format!("C{:.0}", chamfer),
                }
            ),
            ToolCategory::Pathwalk => format!(
                "{:.1}m Wide • {:?} • Edging: {}mm",
                self.width / 1000.0,
                self.pathwalk_profile,
                if self.has_edging { self.edging_width as i32 } else { 0 }
            ),
        }
    }
}

/// Persistent Studio state held in `PlinthApp`.
pub struct ToolConfigStudio {
    pub is_open: bool,
    pub active_category: ToolCategory,
    pub open_categories: BTreeSet<ToolCategory>,
    pub search_query_by_cat: HashMap<ToolCategory, String>,
    pub search_query: String,
    pub presets: Vec<TypePreset>,
    pub selected_preset_by_cat: HashMap<ToolCategory, usize>,
    pub sequence_counter: u32,
}

impl Default for ToolConfigStudio {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolConfigStudio {
    pub fn new() -> Self {
        let mut presets = build_default_type_library();
        for p in &mut presets {
            p.sync_from_layers();
        }
        let mut selected_preset_by_cat = HashMap::new();
        for cat in ToolCategory::ALL_ARCH
            .iter()
            .chain(ToolCategory::ALL_STRUCT.iter())
            .chain(ToolCategory::ALL_ANNOTATION.iter())
            .chain(ToolCategory::ALL_SITE.iter())
        {
            if let Some(idx) = presets.iter().position(|p| p.category == *cat) {
                selected_preset_by_cat.insert(*cat, idx);
            }
        }
        Self {
            is_open: false,
            active_category: ToolCategory::WallArch,
            open_categories: BTreeSet::new(),
            search_query_by_cat: HashMap::new(),
            search_query: String::new(),
            presets,
            selected_preset_by_cat,
            sequence_counter: 1,
        }
    }

    pub fn open_for(&mut self, category: ToolCategory) {
        self.active_category = category;
        self.ensure_valid_selection(category);
        self.open_categories.insert(category);
        self.is_open = true;
    }

    pub fn close_category(&mut self, category: ToolCategory) {
        self.open_categories.remove(&category);
        self.is_open = !self.open_categories.is_empty();
    }

    pub fn is_category_open(&self, category: ToolCategory) -> bool {
        self.open_categories.contains(&category)
    }

    fn ensure_valid_selection(&mut self, cat: ToolCategory) {
        let valid = self
            .selected_preset_by_cat
            .get(&cat)
            .and_then(|&idx| self.presets.get(idx))
            .map_or(false, |p| p.category == cat);
        if !valid {
            if let Some(first_idx) = self.presets.iter().position(|p| p.category == cat) {
                self.selected_preset_by_cat.insert(cat, first_idx);
            }
        }
    }

    pub fn active_preset(&self, cat: ToolCategory) -> Option<&TypePreset> {
        self.selected_preset_by_cat
            .get(&cat)
            .and_then(|&idx| self.presets.get(idx))
            .filter(|p| p.category == cat)
            .or_else(|| self.presets.iter().find(|p| p.category == cat))
    }

    pub fn duplicate_type_for(&mut self, cat: ToolCategory) -> Option<usize> {
        self.ensure_valid_selection(cat);
        let idx = *self.selected_preset_by_cat.get(&cat)?;
        let mut cloned = self.presets.get(idx)?.clone();
        cloned.is_builtin = false;
        let base_name = cloned
            .type_name
            .trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ' || c == '(' || c == ')')
            .trim();
        let candidate = format!("{} (Type {})", base_name, self.sequence_counter);
        self.sequence_counter += 1;
        cloned.type_name = candidate;
        let new_idx = self.presets.len();
        self.presets.push(cloned);
        self.selected_preset_by_cat.insert(cat, new_idx);
        Some(new_idx)
    }

    pub fn duplicate_active_type(&mut self) -> Option<usize> {
        self.duplicate_type_for(self.active_category)
    }

    pub fn create_blank_type_for(&mut self, cat: ToolCategory) -> usize {
        let name = format!("Custom {} {}", cat.short_label(), self.sequence_counter);
        self.sequence_counter += 1;
        let mut p = TypePreset::base(cat, &name, "CT", "User-defined custom BIM type");
        p.is_builtin = false;
        if cat.has_compound_layers() {
            p.layers.push(make_layer(
                LayerFunction::StructuralCore,
                "Primary Core Layer",
                200.0,
                1.40,
            ));
            p.sync_from_layers();
        }
        let new_idx = self.presets.len();
        self.presets.push(p);
        self.selected_preset_by_cat.insert(cat, new_idx);
        new_idx
    }

    pub fn create_blank_type(&mut self) -> usize {
        self.create_blank_type_for(self.active_category)
    }

    /// Finds the next storey strictly above `base_level` in `(LevelId, elevation)` pairs.
    pub fn next_level_above_in_slice(
        levels: &[(LevelId, f64)],
        base_level: LevelId,
    ) -> Option<LevelId> {
        let base_z = levels
            .iter()
            .find(|(id, _)| *id == base_level)
            .map(|(_, z)| *z)
            .unwrap_or(0.0);
        levels
            .iter()
            .filter(|(id, z)| *id != base_level && *z > base_z + 1.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| *id)
    }

    pub fn level_pairs(doc: &Document) -> Vec<(LevelId, f64)> {
        doc.levels.values().map(|l| (l.id, l.elevation)).collect()
    }

    pub fn preview_wall_thickness(&self, is_structural: bool) -> f64 {
        let cat = if is_structural {
            ToolCategory::WallStruct
        } else {
            ToolCategory::WallArch
        };
        self.active_preset(cat)
            .map(|p| p.thickness)
            .unwrap_or(if is_structural { 250.0 } else { 200.0 })
    }

    pub fn preview_wall_height(&self, is_structural: bool) -> f64 {
        let cat = if is_structural {
            ToolCategory::WallStruct
        } else {
            ToolCategory::WallArch
        };
        self.active_preset(cat)
            .map(|p| p.unconnected_height)
            .unwrap_or(3000.0)
    }

    // =========================================================================
    // Preset Application to Newly Drawn Elements & Existing Selected Elements
    // =========================================================================

    pub fn configure_new_wall(&self, doc: &Document, w: &mut Wall, is_structural: bool) {
        let cat = if is_structural {
            ToolCategory::WallStruct
        } else {
            ToolCategory::WallArch
        };
        if let Some(p) = self.active_preset(cat) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_wall(p, &lvls, w);
        }
    }

    pub fn configure_new_curtain_wall(&self, doc: &Document, cw: &mut CurtainWall) {
        if let Some(p) = self.active_preset(ToolCategory::CurtainWall) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_curtain_wall(p, &lvls, cw);
        }
    }

    pub fn configure_new_door(&self, d: &mut Door) {
        if let Some(p) = self.active_preset(ToolCategory::Door) {
            apply_preset_to_door(p, d);
        }
    }

    pub fn configure_new_window(&self, w: &mut Window) {
        if let Some(p) = self.active_preset(ToolCategory::Window) {
            apply_preset_to_window(p, w);
        }
    }

    pub fn configure_new_column(&self, doc: &Document, c: &mut Column, is_structural: bool) {
        let cat = if is_structural {
            ToolCategory::ColumnStruct
        } else {
            ToolCategory::ColumnArch
        };
        if let Some(p) = self.active_preset(cat) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_column(p, &lvls, c);
        }
    }

    pub fn configure_new_footing(&self, doc: &Document, f: &mut Footing) {
        if let Some(p) = self.active_preset(ToolCategory::Footing) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_footing(p, &lvls, f);
        }
    }

    pub fn configure_new_beam(&self, doc: &Document, b: &mut Beam) {
        if let Some(p) = self.active_preset(ToolCategory::Beam) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_beam(p, &lvls, b);
        }
    }

    pub fn configure_new_slab(&self, doc: &Document, s: &mut Slab, is_structural: bool) {
        let cat = if is_structural {
            ToolCategory::SlabStruct
        } else {
            ToolCategory::FloorArch
        };
        if let Some(p) = self.active_preset(cat) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_slab(p, &lvls, s);
        }
    }

    pub fn configure_new_ceiling(&self, doc: &Document, c: &mut Ceiling) {
        if let Some(p) = self.active_preset(ToolCategory::Ceiling) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_ceiling(p, &lvls, c);
        }
    }

    pub fn configure_new_stair(&self, doc: &Document, s: &mut Stair) {
        if let Some(p) = self.active_preset(ToolCategory::Stair) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_stair(p, &lvls, s);
        }
    }

    pub fn configure_new_ramp(&self, doc: &Document, r: &mut Ramp) {
        if let Some(p) = self.active_preset(ToolCategory::Ramp) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_ramp(p, &lvls, r);
        }
    }

    pub fn configure_new_railing(&self, doc: &Document, rl: &mut Railing) {
        if let Some(p) = self.active_preset(ToolCategory::Railing) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_railing(p, &lvls, rl);
        }
    }

    pub fn configure_new_roof(&self, doc: &Document, r: &mut Roof) {
        if let Some(p) = self.active_preset(ToolCategory::Roof) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_roof(p, &lvls, r);
        }
    }

    pub fn configure_new_fascia(&self, doc: &Document, f: &mut Fascia) {
        if let Some(p) = self.active_preset(ToolCategory::Fascia) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_fascia(p, &lvls, f);
        }
    }

    pub fn configure_new_gutter(&self, doc: &Document, g: &mut Gutter) {
        if let Some(p) = self.active_preset(ToolCategory::Gutter) {
            let lvls = Self::level_pairs(doc);
            apply_preset_to_gutter(p, &lvls, g);
        }
    }

    pub fn configure_new_dimension(&self, dim: &mut plinth_model::Dimension) {
        let cat = match dim.kind {
            plinth_model::DimensionKind::Linear => ToolCategory::DimensionLinear,
            plinth_model::DimensionKind::Circular => ToolCategory::DimensionCircular,
            plinth_model::DimensionKind::Angular => ToolCategory::DimensionAngular,
        };
        if let Some(p) = self.active_preset(cat) {
            dim.tick_mark = p.tick_mark;
            dim.font_size = p.font_size;
            dim.font_style = p.font_style;
            dim.color = p.color;
            if dim.offset.abs() < 1e-3 {
                dim.offset = p.dim_offset;
            }
            if p.prefix.is_some() {
                dim.prefix = p.prefix.clone();
            }
            if p.suffix.is_some() {
                dim.suffix = p.suffix.clone();
            }
            dim.is_diameter = p.is_diameter;
        }
    }

    pub fn configure_new_text_note(&self, note: &mut plinth_model::TextNote) {
        if let Some(p) = self.active_preset(ToolCategory::TextNote) {
            note.font_size = p.font_size;
            note.font_style = p.font_style;
            note.color = p.color;
        }
    }

    pub fn configure_new_leader(&self, leader: &mut plinth_model::Leader) {
        if let Some(p) = self.active_preset(ToolCategory::Leader) {
            leader.tick_mark = p.tick_mark;
            leader.font_size = p.font_size;
            leader.font_style = p.font_style;
            leader.color = p.color;
        }
    }

    pub fn configure_new_detail_line(&self, line: &mut plinth_model::DetailLine) {
        if let Some(p) = self.active_preset(ToolCategory::DetailLine) {
            line.style = p.line_style.clone();
        }
    }

    pub fn configure_new_road(&self, rd: &mut plinth_model::Road) {
        if let Some(p) = self.active_preset(ToolCategory::Road) {
            rd.width = p.width;
            rd.lanes = p.lanes;
            rd.lane_width = p.lane_width;
            rd.thickness = p.thickness;
            rd.curb_type = p.road_curb;
            rd.curb_width = p.curb_width;
            rd.curb_height = p.curb_height;
            rd.crown_slope_pct = p.crown_slope_pct;
            rd.has_centerline = p.has_centerline;
            rd.base_offset = p.base_offset;
            rd.road_type_name = p.type_name.clone();
        }
    }

    pub fn configure_new_sidewalk(&self, sw: &mut plinth_model::Sidewalk) {
        if let Some(p) = self.active_preset(ToolCategory::Sidewalk) {
            sw.width = p.width;
            sw.thickness = p.thickness;
            sw.curb_height = p.curb_height;
            sw.edge_profile = p.sidewalk_edge;
            sw.has_top_groove = p.has_top_groove;
            sw.groove_spacing = p.groove_spacing;
            sw.groove_depth = p.groove_depth;
            sw.base_offset = p.base_offset;
            sw.sidewalk_type_name = p.type_name.clone();
        }
    }

    pub fn configure_new_pathwalk(&self, pw: &mut plinth_model::Pathwalk) {
        if let Some(p) = self.active_preset(ToolCategory::Pathwalk) {
            pw.width = p.width;
            pw.thickness = p.thickness;
            pw.profile = p.pathwalk_profile;
            pw.has_edging = p.has_edging;
            pw.edging_width = p.edging_width;
            pw.groove_width = p.groove_width;
            pw.base_offset = p.base_offset;
            pw.pathwalk_type_name = p.type_name.clone();
        }
    }
}

pub fn apply_preset_to_element(
    p: &TypePreset,
    levels: &[(LevelId, f64)],
    el: &mut Element,
) -> bool {
    match el {
        Element::Wall(w)
            if matches!(p.category, ToolCategory::WallArch | ToolCategory::WallStruct) =>
        {
            apply_preset_to_wall(p, levels, w);
            true
        }
        Element::CurtainWall(cw) if p.category == ToolCategory::CurtainWall => {
            apply_preset_to_curtain_wall(p, levels, cw);
            true
        }
        Element::Door(d) if p.category == ToolCategory::Door => {
            apply_preset_to_door(p, d);
            true
        }
        Element::Window(w) if p.category == ToolCategory::Window => {
            apply_preset_to_window(p, w);
            true
        }
        Element::Column(c)
            if matches!(
                p.category,
                ToolCategory::ColumnArch | ToolCategory::ColumnStruct
            ) =>
        {
            apply_preset_to_column(p, levels, c);
            true
        }
        Element::Beam(b) if p.category == ToolCategory::Beam => {
            apply_preset_to_beam(p, levels, b);
            true
        }
        Element::Footing(f) if p.category == ToolCategory::Footing => {
            apply_preset_to_footing(p, levels, f);
            true
        }
        Element::Slab(s)
            if matches!(
                p.category,
                ToolCategory::FloorArch | ToolCategory::SlabStruct
            ) =>
        {
            apply_preset_to_slab(p, levels, s);
            true
        }
        Element::Ceiling(c) if p.category == ToolCategory::Ceiling => {
            apply_preset_to_ceiling(p, levels, c);
            true
        }
        Element::Stair(s) if p.category == ToolCategory::Stair => {
            apply_preset_to_stair(p, levels, s);
            true
        }
        Element::Ramp(r) if p.category == ToolCategory::Ramp => {
            apply_preset_to_ramp(p, levels, r);
            true
        }
        Element::Railing(rl) if p.category == ToolCategory::Railing => {
            apply_preset_to_railing(p, levels, rl);
            true
        }
        Element::Roof(r) if p.category == ToolCategory::Roof => {
            apply_preset_to_roof(p, levels, r);
            true
        }
        Element::Fascia(f) if p.category == ToolCategory::Fascia => {
            apply_preset_to_fascia(p, levels, f);
            true
        }
        Element::Gutter(g) if p.category == ToolCategory::Gutter => {
            apply_preset_to_gutter(p, levels, g);
            true
        }
        Element::Dimension(d)
            if matches!(
                p.category,
                ToolCategory::DimensionLinear
                    | ToolCategory::DimensionCircular
                    | ToolCategory::DimensionAngular
            ) =>
        {
            d.tick_mark = p.tick_mark;
            d.font_size = p.font_size;
            d.font_style = p.font_style;
            d.color = p.color;
            d.prefix = p.prefix.clone();
            d.suffix = p.suffix.clone();
            d.is_diameter = p.is_diameter;
            true
        }
        Element::TextNote(t) if p.category == ToolCategory::TextNote => {
            t.font_size = p.font_size;
            t.font_style = p.font_style;
            t.color = p.color;
            true
        }
        Element::Leader(l) if p.category == ToolCategory::Leader => {
            l.tick_mark = p.tick_mark;
            l.font_size = p.font_size;
            l.font_style = p.font_style;
            l.color = p.color;
            true
        }
        Element::DetailLine(dl) if p.category == ToolCategory::DetailLine => {
            dl.style = p.line_style.clone();
            true
        }
        Element::Road(rd) if p.category == ToolCategory::Road => {
            rd.width = p.width;
            rd.lanes = p.lanes;
            rd.lane_width = p.lane_width;
            rd.thickness = p.thickness;
            rd.curb_type = p.road_curb;
            rd.curb_width = p.curb_width;
            rd.curb_height = p.curb_height;
            rd.crown_slope_pct = p.crown_slope_pct;
            rd.has_centerline = p.has_centerline;
            rd.base_offset = p.base_offset;
            rd.road_type_name = p.type_name.clone();
            true
        }
        Element::Sidewalk(sw) if p.category == ToolCategory::Sidewalk => {
            sw.width = p.width;
            sw.thickness = p.thickness;
            sw.curb_height = p.curb_height;
            sw.edge_profile = p.sidewalk_edge;
            sw.has_top_groove = p.has_top_groove;
            sw.groove_spacing = p.groove_spacing;
            sw.groove_depth = p.groove_depth;
            sw.base_offset = p.base_offset;
            sw.sidewalk_type_name = p.type_name.clone();
            true
        }
        Element::Pathwalk(pw) if p.category == ToolCategory::Pathwalk => {
            pw.width = p.width;
            pw.thickness = p.thickness;
            pw.profile = p.pathwalk_profile;
            pw.has_edging = p.has_edging;
            pw.edging_width = p.edging_width;
            pw.groove_width = p.groove_width;
            pw.base_offset = p.base_offset;
            pw.pathwalk_type_name = p.type_name.clone();
            true
        }
        _ => false,
    }
}

fn resolve_top_level(
    p: &TypePreset,
    levels: &[(LevelId, f64)],
    base_level: LevelId,
) -> Option<LevelId> {
    if p.top_to_next_level {
        ToolConfigStudio::next_level_above_in_slice(levels, base_level)
    } else {
        None
    }
}

fn apply_preset_to_wall(p: &TypePreset, levels: &[(LevelId, f64)], w: &mut Wall) {
    w.wall_type_name = p.type_name.clone();
    w.description = p.description.clone();
    w.thickness = p.thickness;
    w.assembly_layers = p.layers.clone();
    w.base_offset = p.base_offset;
    w.top_level = resolve_top_level(p, levels, w.base_level);
    w.top_offset = p.top_offset;
    w.unconnected_height = p.unconnected_height;
    w.is_structural = p.is_structural;
    w.is_external = p.is_external;
    w.fire_rating = p.fire_rating.clone();
    w.acoustic_rating = p.acoustic_rating.clone();
    w.thermal_transmittance = p.thermal_transmittance;
}

pub fn apply_preset_to_curtain_wall(p: &TypePreset, levels: &[(LevelId, f64)], cw: &mut CurtainWall) {
    cw.curtain_wall_type_name = p.type_name.clone();
    cw.mullion_width = p.width;
    cw.mullion_depth = p.depth;
    cw.panel_thickness = p.thickness;
    cw.vertical_grid_spacing = p.grid_spacing_x;
    cw.horizontal_grid_spacing = p.grid_spacing_y;
    cw.base_offset = p.base_offset;
    cw.top_level = resolve_top_level(p, levels, cw.base_level);
    cw.top_offset = p.top_offset;
    cw.unconnected_height = p.unconnected_height;
    cw.is_external = p.is_external;
    cw.fire_rating = p.fire_rating.clone();
    cw.acoustic_rating = p.acoustic_rating.clone();
    cw.thermal_transmittance = p.thermal_transmittance;
}

fn apply_preset_to_door(p: &TypePreset, d: &mut Door) {
    d.door_type_name = p.type_name.clone();
    d.width = p.width;
    d.height = p.height;
    d.sill_height = p.sill_height;
    d.base_offset = p.base_offset;
    d.top_offset = p.top_offset;
    d.operation_type = p.operation_type.clone();
    d.flipped_hand = p.flipped_hand;
    d.flipped_face = p.flipped_face;
    d.is_external = p.is_external;
    d.is_fire_exit = p.is_fire_exit;
    d.fire_rating = p.fire_rating.clone();
    d.acoustic_rating = p.acoustic_rating.clone();
    d.security_rating = p.security_rating.clone();
    d.thermal_transmittance = p.thermal_transmittance;
}

fn apply_preset_to_window(p: &TypePreset, w: &mut Window) {
    w.window_type_name = p.type_name.clone();
    w.width = p.width;
    w.height = p.height;
    w.sill_height = p.sill_height;
    w.base_offset = p.base_offset;
    w.top_offset = p.top_offset;
    w.operation_type = p.operation_type.clone();
    w.is_external = p.is_external;
    w.fire_rating = p.fire_rating.clone();
    w.acoustic_rating = p.acoustic_rating.clone();
    w.thermal_transmittance = p.thermal_transmittance;
    w.solar_heat_gain_coefficient = p.shgc;
}

pub fn apply_preset_to_column(p: &TypePreset, levels: &[(LevelId, f64)], c: &mut Column) {
    c.column_type_name = p.type_name.clone();
    c.width = p.width;
    c.depth = if p.is_round { p.width } else { p.depth };
    c.is_round = p.is_round;
    c.base_offset = p.base_offset;
    c.top_level = resolve_top_level(p, levels, c.base_level);
    c.top_offset = p.top_offset;
    c.unconnected_height = p.unconnected_height;
    c.is_structural = p.is_structural;
    c.is_external = p.is_external;
    c.fire_rating = p.fire_rating.clone();
    c.load_bearing_capacity = p.load_bearing_capacity;
}

pub fn apply_preset_to_footing(p: &TypePreset, levels: &[(LevelId, f64)], f: &mut Footing) {
    f.footing_type_name = p.type_name.clone();
    f.footing_style = p.footing_style;
    f.width = p.width;
    f.length = p.depth;
    f.thickness = p.thickness;
    f.base_offset = p.base_offset;
    f.top_level = resolve_top_level(p, levels, f.base_level);
    f.top_offset = p.top_offset;
    f.is_external = p.is_external;
    f.soil_bearing_capacity = p.soil_bearing_capacity;
    f.concrete_grade = p.concrete_grade.clone();
    f.load_bearing_capacity = p.load_bearing_capacity;
}

pub fn apply_preset_to_beam(p: &TypePreset, levels: &[(LevelId, f64)], b: &mut Beam) {
    b.beam_type_name = p.type_name.clone();
    b.width = p.width;
    b.depth = p.depth;
    b.z_offset = p.base_offset;
    b.top_level = resolve_top_level(p, levels, b.level);
    b.top_offset = p.top_offset;
    b.is_structural = p.is_structural;
    b.is_external = p.is_external;
    b.fire_rating = p.fire_rating.clone();
    b.load_bearing_capacity = p.load_bearing_capacity;
}

pub fn apply_preset_to_slab(p: &TypePreset, levels: &[(LevelId, f64)], s: &mut Slab) {
    s.slab_type_name = p.type_name.clone();
    s.thickness = p.thickness;
    s.assembly_layers = p.layers.clone();
    s.offset = p.base_offset;
    s.top_level = resolve_top_level(p, levels, s.level);
    s.top_offset = p.top_offset;
    s.is_structural = p.is_structural;
    s.is_external = p.is_external;
    s.fire_rating = p.fire_rating.clone();
    s.acoustic_rating = p.acoustic_rating.clone();
    s.thermal_transmittance = p.thermal_transmittance;
    s.load_bearing_capacity = p.load_bearing_capacity;
}

fn apply_preset_to_ceiling(p: &TypePreset, levels: &[(LevelId, f64)], c: &mut Ceiling) {
    c.ceiling_type_name = p.type_name.clone();
    c.thickness = p.thickness;
    c.assembly_layers = p.layers.clone();
    c.height_above_level = p.base_offset;
    c.top_level = resolve_top_level(p, levels, c.level);
    c.top_offset = p.top_offset;
    c.grid_spacing_x = p.grid_spacing_x;
    c.grid_spacing_y = p.grid_spacing_y;
    c.light_reflectance = p.light_reflectance;
    c.is_external = p.is_external;
    c.fire_rating = p.fire_rating.clone();
    c.acoustic_rating = p.acoustic_rating.clone();
}

fn apply_preset_to_stair(p: &TypePreset, levels: &[(LevelId, f64)], s: &mut Stair) {
    s.stair_type_name = p.type_name.clone();
    s.stair_style = p.stair_style;
    s.width = p.width;
    s.riser_count = p.riser_count.max(2);
    s.waist_thickness = p.waist_thickness;
    s.base_offset = p.base_offset;
    s.top_level = resolve_top_level(p, levels, s.base_level);
    s.top_offset = p.top_offset;
    s.height = p.unconnected_height;
    s.is_external = p.is_external;
    s.fire_rating = p.fire_rating.clone();
}

fn apply_preset_to_ramp(p: &TypePreset, levels: &[(LevelId, f64)], r: &mut Ramp) {
    r.ramp_type_name = p.type_name.clone();
    r.ramp_style = p.ramp_style;
    r.width = p.width;
    r.thickness = p.thickness;
    r.base_offset = p.base_offset;
    r.top_level = resolve_top_level(p, levels, r.base_level);
    r.top_offset = p.top_offset;
    r.height = p.unconnected_height;
    r.is_external = p.is_external;
    r.fire_rating = p.fire_rating.clone();
}

fn apply_preset_to_railing(p: &TypePreset, levels: &[(LevelId, f64)], rl: &mut Railing) {
    rl.railing_type_name = p.type_name.clone();
    rl.height = p.unconnected_height;
    rl.baluster_spacing = p.baluster_spacing;
    rl.mid_rail_count = p.mid_rail_count;
    rl.material_name = p.material_name.clone();
    rl.base_offset = p.base_offset;
    rl.top_level = resolve_top_level(p, levels, rl.level);
    rl.top_offset = p.top_offset;
    rl.is_external = p.is_external;
}

fn apply_preset_to_roof(p: &TypePreset, levels: &[(LevelId, f64)], r: &mut Roof) {
    r.roof_type_name = p.type_name.clone();
    r.shape = p.roof_shape;
    r.slope_degrees = p.slope_degrees;
    r.overhang = p.overhang;
    r.thickness = p.thickness;
    r.assembly_layers = p.layers.clone();
    r.offset = p.base_offset;
    r.top_level = resolve_top_level(p, levels, r.level);
    r.top_offset = p.top_offset;
    r.is_structural = p.is_structural;
    r.is_external = p.is_external;
    r.fire_rating = p.fire_rating.clone();
    r.acoustic_rating = p.acoustic_rating.clone();
    r.thermal_transmittance = p.thermal_transmittance;
    r.solar_reflectance_index = p.solar_reflectance_index;
    r.snow_load_capacity = p.snow_load_capacity;
}

fn apply_preset_to_fascia(p: &TypePreset, levels: &[(LevelId, f64)], f: &mut Fascia) {
    f.height = p.height;
    f.thickness = p.thickness;
    f.material_name = p.material_name.clone();
    f.vertical_offset = p.base_offset;
    f.top_level = resolve_top_level(p, levels, f.level);
    f.top_offset = p.top_offset;
}

fn apply_preset_to_gutter(p: &TypePreset, levels: &[(LevelId, f64)], g: &mut Gutter) {
    g.width = p.width;
    g.depth = p.depth;
    g.material_name = p.material_name.clone();
    g.vertical_offset = p.base_offset;
    g.top_level = resolve_top_level(p, levels, g.level);
    g.top_offset = p.top_offset;
}

// =============================================================================
// Floating Studio UI Renderer
// =============================================================================

pub struct StudioOutcome {
    pub activated_tool: Option<Tool>,
    pub applied_to_selection: bool,
    pub status_note: Option<String>,
}

pub fn render_tool_config_studio(
    ctx: &Context,
    studio: &mut ToolConfigStudio,
    doc: &mut Document,
    selected_element: Option<ElementId>,
    theme: &AppTheme,
) -> StudioOutcome {
    let mut outcome = StudioOutcome {
        activated_tool: None,
        applied_to_selection: false,
        status_note: None,
    };

    if !studio.is_open && studio.open_categories.is_empty() {
        return outcome;
    }
    if studio.is_open && studio.open_categories.is_empty() {
        studio.open_categories.insert(studio.active_category);
    }
    if !studio.is_open && !studio.open_categories.is_empty() {
        studio.open_categories.clear();
        return outcome;
    }

    let open_cats: Vec<ToolCategory> = studio.open_categories.iter().copied().collect();
    let mut close_cats = Vec::new();

    for cat in open_cats {
        studio.ensure_valid_selection(cat);

        let screen = ctx.screen_rect();
        let def_w = (screen.width() * 0.88).clamp(960.0, 1260.0);
        let def_h = (screen.height() * 0.90).clamp(740.0, 1000.0);
        let min_h = (screen.height() * 0.75).clamp(620.0, 880.0);
        let max_w = (screen.width() * 0.98).max(960.0);
        let max_h = (screen.height() * 0.98).max(760.0);

        let mut is_cat_open = true;
        EguiWindow::new(format!("⚙ {} Settings & Type Studio", cat.title()))
            .id(egui::Id::new("arch3d_object_settings").with(cat))
            .open(&mut is_cat_open)
            .default_size([def_w, def_h])
            .min_size([760.0, min_h])
            .max_size([max_w, max_h])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                // 1. Dedicated Item Header Banner (Zero Clutter — Dedicated to this BIM Item)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                    draw_icon(ui.painter(), icon_rect, cat.icon_kind(), theme.accent);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(cat.title()).size(13.5).strong().color(theme.text_primary));
                            let disc_badge_color = if cat.is_structural_discipline() {
                                Color32::from_rgb(220, 130, 40)
                            } else {
                                Color32::from_rgb(45, 145, 230)
                            };
                            ui.label(
                                RichText::new(format!(" {} ", cat.discipline_label()))
                                    .size(9.5)
                                    .strong()
                                    .color(Color32::WHITE)
                                    .background_color(disc_badge_color),
                            );
                            ui.label(
                                RichText::new(format!(" [{}] ", cat.pset_name()))
                                    .size(9.5)
                                    .color(theme.text_muted),
                            );
                        });
                        let preset_count = studio.presets.iter().filter(|p| p.category == cat).count();
                        ui.label(
                            RichText::new(format!(
                                "Parametric BIM Object Definition • {} Types configured in project library",
                                preset_count
                            ))
                            .size(10.0)
                            .color(theme.text_muted),
                        );
                    });
                });

                ui.add_space(3.0);
                ui.separator();

                // 2. Main Split Workspace: Left Type Library (Fixed 260px) | Right Preview & Parameters
                let left_w = 260.0f32;
                let _avail_h = (ui.available_height() - 44.0).max(520.0);

                ui.horizontal(|ui| {
                    // --- LEFT COLUMN: TYPE LIBRARY & DUPLICATE TYPE (Fixed 260px) ---
                    ui.vertical(|ui| {
                        ui.set_width(left_w);
                        ui.set_max_width(left_w);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} Library", cat.short_label()))
                                    .strong()
                                    .size(12.5),
                            );
                        });

                        let mut query = studio.search_query_by_cat.get(&cat).cloned().unwrap_or_default();
                        let search_resp = ui.horizontal(|ui| {
                            ui.label("🔍");
                            ui.add(
                                egui::TextEdit::singleline(&mut query)
                                    .desired_width(left_w - 45.0)
                                    .hint_text("Filter presets..."),
                            )
                        });
                        if search_resp.inner.changed() {
                            studio.search_query_by_cat.insert(cat, query.clone());
                        }

                        ui.add_space(3.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("📋 Duplicate").strong().size(11.0))
                                .on_hover_text("Clone selected type to create custom type")
                                .clicked()
                            {
                                if let Some(new_idx) = studio.duplicate_type_for(cat) {
                                    outcome.status_note = Some(format!(
                                        "Duplicated new type: {}",
                                        studio.presets[new_idx].type_name
                                    ));
                                }
                            }
                            if ui
                                .button(RichText::new("➕ New").size(11.0))
                                .on_hover_text("Create a blank custom type")
                                .clicked()
                            {
                                let new_idx = studio.create_blank_type_for(cat);
                                outcome.status_note = Some(format!(
                                    "Created custom type: {}",
                                    studio.presets[new_idx].type_name
                                ));
                            }
                            let current_sel_idx = studio
                                .selected_preset_by_cat
                                .get(&cat)
                                .copied()
                                .unwrap_or(0);

                            let cat_count = studio
                                .presets
                                .iter()
                                .filter(|p| p.category == cat)
                                .count();
                            let can_delete = cat_count > 1
                                && studio
                                    .presets
                                    .get(current_sel_idx)
                                    .map_or(false, |p| !p.is_builtin);

                            if can_delete && ui.small_button("🗑").on_hover_text("Delete custom type").clicked() {
                                studio.presets.remove(current_sel_idx);
                                for val in studio.selected_preset_by_cat.values_mut() {
                                    if *val > current_sel_idx {
                                        *val -= 1;
                                    }
                                }
                                studio.ensure_valid_selection(cat);
                                outcome.status_note = Some("Deleted custom type".to_string());
                            }
                        });

                        ui.separator();

                        let query_lower = query.trim().to_lowercase();
                        let active_idx_now = studio
                            .selected_preset_by_cat
                            .get(&cat)
                            .copied()
                            .unwrap_or(0);

                        let scroll_h = (ui.available_height() - 8.0).max(380.0);
                        ScrollArea::vertical()
                            .id_salt(("studio_preset_list", cat))
                            .min_scrolled_height(380.0)
                            .max_height(scroll_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let matching_indices: Vec<usize> = studio
                                    .presets
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, p)| {
                                        p.category == cat
                                            && (query_lower.is_empty()
                                                || p.type_name.to_lowercase().contains(&query_lower)
                                                || p.description.to_lowercase().contains(&query_lower))
                                    })
                                    .map(|(i, _)| i)
                                    .collect();

                                let item_w = (left_w - 20.0).max(200.0);
                                for idx in matching_indices {
                                    let is_sel = idx == active_idx_now;
                                    let p = &studio.presets[idx];
                                    let card_fill = if is_sel {
                                        theme.accent.gamma_multiply(0.22)
                                    } else {
                                        theme.panel_bg
                                    };
                                    let frame = egui::Frame::NONE
                                        .fill(card_fill)
                                        .stroke(Stroke::new(
                                            if is_sel { 1.5 } else { 0.8 },
                                            if is_sel {
                                                theme.accent
                                            } else {
                                                theme.border_color
                                            },
                                        ))
                                        .corner_radius(4.0)
                                        .inner_margin(egui::Margin::symmetric(6, 5));

                                    let resp = frame
                                        .show(ui, |ui| {
                                            ui.set_width(item_w);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&p.type_name)
                                                        .strong()
                                                        .size(11.0)
                                                        .color(if is_sel {
                                                            theme.accent
                                                        } else {
                                                            theme.text_primary
                                                        }),
                                                );
                                                if !p.is_builtin {
                                                    ui.label(
                                                        RichText::new("CUSTOM")
                                                            .size(8.5)
                                                            .color(Color32::from_rgb(255, 165, 50)),
                                                    );
                                                }
                                            });
                                            ui.label(
                                                RichText::new(p.summary_badge())
                                                    .size(9.5)
                                                    .color(theme.text_muted),
                                            );
                                        })
                                        .response
                                        .interact(Sense::click());

                                    if resp.clicked() {
                                        studio.selected_preset_by_cat.insert(cat, idx);
                                    }
                                    ui.add_space(2.0);
                                }
                            });
                    });

                    ui.separator();

                    // --- RIGHT COLUMN: INTERACTIVE BLUEPRINT PREVIEW & PARAMETER INSPECTOR ---
                    ui.vertical(|ui| {
                        let sel_idx = match studio.selected_preset_by_cat.get(&cat).copied() {
                            Some(i) if i < studio.presets.len() => i,
                            _ => return,
                        };

                        draw_preset_blueprint_preview(ui, &studio.presets[sel_idx], theme);

                        ui.add_space(6.0);

                        let scroll_h = (ui.available_height() - 8.0).max(460.0);
                        ScrollArea::vertical()
                            .id_salt(("studio_param_scroll", cat))
                            .min_scrolled_height(460.0)
                            .max_height(scroll_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let preset = &mut studio.presets[sel_idx];
                                render_preset_editor(ui, preset, theme);
                            });
                    });
                });

                ui.separator();

                // 3. Footer Action Bar
                ui.horizontal(|ui| {
                    let sel_idx = studio
                        .selected_preset_by_cat
                        .get(&cat)
                        .copied()
                        .unwrap_or(0);

                    if let Some(p) = studio.presets.get(sel_idx) {
                        let activate_btn = egui::Button::new(
                            RichText::new(format!("✓ Activate {} Tool", cat.short_label()))
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent);

                        if ui.add(activate_btn).clicked() {
                            outcome.activated_tool = Some(cat.to_tool(p.roof_shape));
                            outcome.status_note = Some(format!(
                                "Active tool: {} ({})",
                                cat.title(),
                                p.type_name
                            ));
                            close_cats.push(cat);
                        }

                        let sel_matches = selected_element
                            .and_then(|id| doc.elements.get(&id))
                            .map_or(false, |el| cat.matches_element(el));

                        let matching_item_ids: Vec<ElementId> = doc
                            .elements
                            .iter()
                            .filter(|(_, el)| cat.matches_element(el))
                            .map(|(id, _)| *id)
                            .collect();
                        let total_placed = matching_item_ids.len();

                        let apply_sel_btn = egui::Button::new(
                            RichText::new("⚡ Apply to Selected Item").strong(),
                        );
                        if ui
                            .add_enabled(sel_matches, apply_sel_btn)
                            .on_hover_text(if sel_matches {
                                "Apply this configured type, compound layers, and dimensions to the selected item"
                            } else {
                                "Select an element of this type in the viewport to apply directly to it"
                            })
                            .clicked()
                        {
                            if let Some(id) = selected_element {
                                doc.push_undo();
                                let lvls = ToolConfigStudio::level_pairs(doc);
                                if let Some(el) = doc.elements.get_mut(&id) {
                                    if apply_preset_to_element(p, &lvls, el) {
                                        outcome.applied_to_selection = true;
                                        outcome.status_note = Some(format!(
                                            "Applied type '{}' to selected {}",
                                            p.type_name,
                                            cat.short_label()
                                        ));
                                    }
                                }
                            }
                        }

                        let apply_all_btn = egui::Button::new(
                            RichText::new(format!("⚡ Apply to All Placed ({})", total_placed)).size(11.0),
                        );
                        if ui
                            .add_enabled(total_placed > 0, apply_all_btn)
                            .on_hover_text(format!(
                                "Batch-apply this configured type and layers to all {} placed {} items in the project",
                                total_placed,
                                cat.short_label()
                            ))
                            .clicked()
                        {
                            doc.push_undo();
                            let lvls = ToolConfigStudio::level_pairs(doc);
                            let mut count = 0;
                            for id in matching_item_ids {
                                if let Some(el) = doc.elements.get_mut(&id) {
                                    if apply_preset_to_element(p, &lvls, el) {
                                        count += 1;
                                    }
                                }
                            }
                            outcome.applied_to_selection = true;
                            outcome.status_note = Some(format!(
                                "Applied type '{}' to all {} placed {} items",
                                p.type_name,
                                count,
                                cat.short_label()
                            ));
                        }

                        ui.label(
                            RichText::new(format!("Active Type: {}", p.type_name))
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_cats.push(cat);
                        }
                    });
                });
            });

        if !is_cat_open {
            close_cats.push(cat);
        }
    }

    for c in close_cats {
        studio.close_category(c);
    }
    studio.is_open = !studio.open_categories.is_empty();

    outcome
}

// =============================================================================
// Procedural Blueprint Diagram (Zero Bitmaps — Pure Mathematical Painter)
// =============================================================================

fn layer_function_color(func: LayerFunction) -> Color32 {
    let [r, g, b] = func.preview_rgb();
    Color32::from_rgb(r, g, b)
}

fn draw_preset_blueprint_preview(ui: &mut Ui, p: &TypePreset, theme: &AppTheme) {
    let desired = Vec2::new(ui.available_width(), 118.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);

    let bg = if theme.mode == crate::theme::ThemeMode::Dark {
        Color32::from_rgb(18, 22, 30)
    } else {
        Color32::from_rgb(242, 245, 250)
    };
    painter.rect_filled(rect, 6.0, bg);
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, theme.border_color),
        StrokeKind::Inside,
    );

    painter.text(
        Pos2::new(rect.min.x + 10.0, rect.min.y + 7.0),
        Align2::LEFT_TOP,
        format!("LIVE ASSEMBLY BLUEPRINT  •  {}", p.type_name),
        FontId::proportional(10.0),
        theme.text_muted,
    );

    let cx = rect.center().x;
    let cy = rect.center().y + 5.0;

    if p.category.has_compound_layers() && !p.layers.is_empty() {
        let is_vertical = matches!(p.category, ToolCategory::WallArch | ToolCategory::WallStruct);
        let total_visual_w: f32 = p
            .layers
            .iter()
            .map(|l| (l.thickness as f32).max(6.0))
            .sum::<f32>()
            .max(10.0);

        if is_vertical {
            let draw_w = (rect.width() * 0.42).clamp(110.0, 240.0);
            let draw_h = 76.0f32;
            let x_start = cx - draw_w * 0.5;
            let y_top = cy - draw_h * 0.5;
            let mut cur_x = x_start;

            painter.text(
                Pos2::new(x_start - 10.0, cy),
                Align2::RIGHT_CENTER,
                "EXTERIOR",
                FontId::proportional(9.5),
                theme.accent,
            );
            painter.text(
                Pos2::new(x_start + draw_w + 10.0, cy),
                Align2::LEFT_CENTER,
                "INTERIOR",
                FontId::proportional(9.5),
                theme.text_muted,
            );

            for layer in &p.layers {
                let vis_t = (layer.thickness as f32).max(6.0);
                let w_px = (vis_t / total_visual_w) * draw_w;
                let l_rect = Rect::from_min_size(Pos2::new(cur_x, y_top), Vec2::new(w_px, draw_h));
                let col = layer_function_color(layer.function);
                painter.rect_filled(l_rect, 0.0, col.gamma_multiply(0.82));
                painter.rect_stroke(
                    l_rect,
                    0.0,
                    Stroke::new(1.0, Color32::from_gray(45)),
                    StrokeKind::Inside,
                );

                if layer.function == LayerFunction::ThermalInsulation && w_px >= 8.0 {
                    let steps = 8;
                    for s in 0..steps {
                        let y0 = y_top + (s as f32 / steps as f32) * draw_h;
                        let y1 = y_top + ((s + 1) as f32 / steps as f32) * draw_h;
                        let xm = if s % 2 == 0 {
                            l_rect.max.x - 2.0
                        } else {
                            l_rect.min.x + 2.0
                        };
                        let x_prev = if s % 2 == 0 {
                            l_rect.min.x + 2.0
                        } else {
                            l_rect.max.x - 2.0
                        };
                        painter.line_segment(
                            [Pos2::new(x_prev, y0), Pos2::new(xm, y1)],
                            Stroke::new(1.0, Color32::from_rgb(130, 90, 15)),
                        );
                    }
                }
                cur_x += w_px;
            }

            let y_dim = y_top + draw_h + 10.0;
            painter.line_segment(
                [Pos2::new(x_start, y_dim), Pos2::new(x_start + draw_w, y_dim)],
                Stroke::new(1.2, theme.accent),
            );
            painter.text(
                Pos2::new(cx, y_dim + 2.0),
                Align2::CENTER_TOP,
                format!(
                    "Total: {:.0} mm  •  U = {:.2} W/m²K  •  {}",
                    p.thickness, p.thermal_transmittance, p.fire_rating
                ),
                FontId::proportional(10.0),
                theme.text_primary,
            );
        } else {
            let draw_w = (rect.width() * 0.56).clamp(160.0, 320.0);
            let draw_h = 60.0f32;
            let x_left = cx - draw_w * 0.5;
            let y_start = cy - draw_h * 0.5 - 3.0;
            let mut cur_y = y_start;

            for layer in &p.layers {
                let vis_t = (layer.thickness as f32).max(6.0);
                let h_px = (vis_t / total_visual_w) * draw_h;
                let l_rect =
                    Rect::from_min_size(Pos2::new(x_left, cur_y), Vec2::new(draw_w, h_px));
                let col = layer_function_color(layer.function);
                painter.rect_filled(l_rect, 0.0, col.gamma_multiply(0.82));
                painter.rect_stroke(
                    l_rect,
                    0.0,
                    Stroke::new(1.0, Color32::from_gray(45)),
                    StrokeKind::Inside,
                );
                cur_y += h_px;
            }

            let extra = if p.category == ToolCategory::Roof && p.roof_shape != RoofShape::Flat {
                format!(" • {:?} {:.0}°", p.roof_shape, p.slope_degrees)
            } else {
                String::new()
            };
            painter.text(
                Pos2::new(cx, y_start + draw_h + 8.0),
                Align2::CENTER_TOP,
                format!(
                    "Deck: {:.0} mm  •  {} Layers  •  U = {:.2} W/m²K{}",
                    p.thickness,
                    p.layers.len(),
                    p.thermal_transmittance,
                    extra
                ),
                FontId::proportional(10.0),
                theme.text_primary,
            );
        }
        return;
    }

    match p.category {
        ToolCategory::Stair => {
            let n = 6usize;
            let step_w = 24.0f32;
            let step_h = 13.0f32;
            let x0 = cx - (n as f32 * step_w) * 0.5 - 45.0;
            let y0 = cy + 36.0;
            let stroke = Stroke::new(1.6, theme.accent);
            let fill = theme.accent.gamma_multiply(0.24);

            for i in 0..n {
                let sx0 = x0 + i as f32 * step_w;
                let sx1 = sx0 + step_w;
                let sy_top = y0 - (i + 1) as f32 * step_h;
                let sy_prev = y0 - i as f32 * step_h;
                match p.stair_style {
                    StairStyle::OpenRiser => {
                        let r = Rect::from_min_max(
                            Pos2::new(sx0, sy_top),
                            Pos2::new(sx1, sy_top + 4.5),
                        );
                        painter.rect_filled(r, 1.0, fill);
                        painter.rect_stroke(r, 1.0, stroke, StrokeKind::Inside);
                    }
                    StairStyle::SolidStepped => {
                        let r = Rect::from_min_max(Pos2::new(sx0, sy_top), Pos2::new(sx1, y0));
                        painter.rect_filled(r, 0.0, fill);
                        painter.line_segment(
                            [Pos2::new(sx0, sy_prev), Pos2::new(sx0, sy_top)],
                            stroke,
                        );
                        painter.line_segment(
                            [Pos2::new(sx0, sy_top), Pos2::new(sx1, sy_top)],
                            stroke,
                        );
                    }
                    StairStyle::Monolithic => {
                        let waist = 14.0f32;
                        let sb0 = (sy_prev + waist).min(y0);
                        let sb1 = (sy_top + step_h + waist).min(y0);
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                Pos2::new(sx0, sy_top),
                                Pos2::new(sx1, sy_top),
                                Pos2::new(sx1, sb1),
                                Pos2::new(sx0, sb0),
                            ],
                            fill,
                            Stroke::NONE,
                        ));
                        painter.line_segment(
                            [Pos2::new(sx0, sy_prev), Pos2::new(sx0, sy_top)],
                            stroke,
                        );
                        painter.line_segment(
                            [Pos2::new(sx0, sy_top), Pos2::new(sx1, sy_top)],
                            stroke,
                        );
                        painter.line_segment(
                            [Pos2::new(sx0, sb0), Pos2::new(sx1, sb1)],
                            stroke,
                        );
                    }
                }
            }

            let r_mm = p.unconnected_height / (p.riser_count.max(1) as f64);
            let g_mm = p.target_going;
            let blondel = 2.0 * r_mm + g_mm;
            let (status_txt, status_col) = if (600.0..=650.0).contains(&blondel) {
                ("OPTIMAL COMFORT", Color32::from_rgb(60, 200, 110))
            } else if (575.0..=675.0).contains(&blondel) {
                ("ACCEPTABLE", Color32::from_rgb(240, 180, 50))
            } else {
                ("CHECK PITCH", Color32::from_rgb(235, 85, 85))
            };

            let badge_pos = Pos2::new(cx + 85.0, cy);
            painter.text(
                Pos2::new(badge_pos.x, badge_pos.y - 18.0),
                Align2::CENTER_CENTER,
                format!("Riser R = {:.1} mm  •  Going G = {:.0} mm", r_mm, g_mm),
                FontId::proportional(10.5),
                theme.text_primary,
            );
            painter.text(
                badge_pos,
                Align2::CENTER_CENTER,
                format!("Blondel 2R + G = {:.0} mm", blondel),
                FontId::proportional(12.0),
                status_col,
            );
            painter.text(
                Pos2::new(badge_pos.x, badge_pos.y + 18.0),
                Align2::CENTER_CENTER,
                format!("{}  (Target 600–650 mm)", status_txt),
                FontId::proportional(9.5),
                status_col,
            );
        }

        ToolCategory::Ramp => {
            let x0 = cx - 120.0;
            let x1 = cx + 30.0;
            let y0 = cy + 34.0;
            let y1 = cy - 18.0;
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.25);

            match p.ramp_style {
                RampStyle::SolidWedge => {
                    painter.add(egui::Shape::convex_polygon(
                        vec![Pos2::new(x0, y0), Pos2::new(x1, y1), Pos2::new(x1, y0)],
                        fill,
                        stroke,
                    ));
                }
                RampStyle::SlopedSlab => {
                    let t_px = 14.0f32;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            Pos2::new(x0, y0 - t_px),
                            Pos2::new(x1, y1 - t_px),
                            Pos2::new(x1, y1),
                            Pos2::new(x0, y0),
                        ],
                        fill,
                        stroke,
                    ));
                }
            }
            painter.text(
                Pos2::new(cx + 115.0, cy - 8.0),
                Align2::CENTER_CENTER,
                format!("Style: {}", p.ramp_style.label()),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 115.0, cy + 12.0),
                Align2::CENTER_CENTER,
                format!("Width {:.0} mm  •  Slab {:.0} mm", p.width, p.thickness),
                FontId::proportional(10.0),
                theme.accent,
            );
        }

        ToolCategory::Railing => {
            let x0 = cx - 110.0;
            let x1 = cx + 110.0;
            let y_bot = cy + 36.0;
            let y_top = cy - 28.0;
            let stroke = Stroke::new(2.0, theme.accent);
            painter.line_segment([Pos2::new(x0, y_top), Pos2::new(x1, y_top)], stroke);
            for m in 0..p.mid_rail_count {
                let frac = (m + 1) as f32 / (p.mid_rail_count + 1) as f32;
                let ym = y_top + frac * (y_bot - y_top);
                painter.line_segment(
                    [Pos2::new(x0, ym), Pos2::new(x1, ym)],
                    Stroke::new(1.0, theme.text_muted),
                );
            }
            let posts = 9;
            for i in 0..=posts {
                let xp = x0 + (i as f32 / posts as f32) * (x1 - x0);
                painter.line_segment(
                    [Pos2::new(xp, y_bot), Pos2::new(xp, y_top)],
                    Stroke::new(1.2, theme.text_primary),
                );
            }
            let safe = p.baluster_spacing <= 105.0;
            painter.text(
                Pos2::new(cx, y_bot + 6.0),
                Align2::CENTER_TOP,
                format!(
                    "H {:.0} mm  •  Balusters @ {:.0} mm  •  {}",
                    p.unconnected_height,
                    p.baluster_spacing,
                    if safe {
                        "100mm Sphere-Safe ✓"
                    } else {
                        "Wide Industrial Spacing"
                    }
                ),
                FontId::proportional(10.0),
                if safe {
                    Color32::from_rgb(60, 200, 110)
                } else {
                    theme.text_muted
                },
            );
        }

        ToolCategory::ColumnArch | ToolCategory::ColumnStruct | ToolCategory::Beam => {
            let max_dim = p.width.max(p.depth).max(100.0) as f32;
            let w_px = ((p.width as f32 / max_dim) * 76.0).clamp(24.0, 90.0);
            let d_px = ((p.depth as f32 / max_dim) * 76.0).clamp(24.0, 90.0);
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.24);
            if p.is_round && p.category != ToolCategory::Beam {
                painter.circle(Pos2::new(cx - 50.0, cy), w_px * 0.5, fill, stroke);
            } else {
                let r = Rect::from_center_size(Pos2::new(cx - 50.0, cy), Vec2::new(w_px, d_px));
                painter.rect_filled(r, 2.0, fill);
                painter.rect_stroke(r, 2.0, stroke, StrokeKind::Inside);
            }
            let area_cm2 = if p.is_round && p.category != ToolCategory::Beam {
                std::f64::consts::PI * (p.width * 0.05).powi(2)
            } else {
                (p.width * p.depth) / 100.0
            };
            painter.text(
                Pos2::new(cx + 65.0, cy - 10.0),
                Align2::CENTER_CENTER,
                format!("Section: {}", p.summary_badge()),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 65.0, cy + 10.0),
                Align2::CENTER_CENTER,
                format!("Area: {:.0} cm²  •  {}", area_cm2, p.fire_rating),
                FontId::proportional(10.0),
                theme.accent,
            );
        }

        ToolCategory::Door | ToolCategory::Window => {
            let w_px = ((p.width as f32 / 2400.0) * 110.0).clamp(38.0, 125.0);
            let h_px = ((p.height as f32 / 2400.0) * 82.0).clamp(34.0, 86.0);
            let r = Rect::from_center_size(Pos2::new(cx - 45.0, cy), Vec2::new(w_px, h_px));
            let fill = if p.category == ToolCategory::Window {
                Color32::from_rgba_unmultiplied(110, 195, 245, 75)
            } else {
                Color32::from_rgba_unmultiplied(180, 130, 85, 85)
            };
            painter.rect_filled(r, 1.0, fill);
            painter.rect_stroke(r, 1.0, Stroke::new(2.0, theme.accent), StrokeKind::Inside);
            if p.operation_type.contains("DOUBLE") {
                painter.line_segment(
                    [
                        Pos2::new(r.center().x, r.min.y),
                        Pos2::new(r.center().x, r.max.y),
                    ],
                    Stroke::new(1.2, theme.accent),
                );
            }
            painter.text(
                Pos2::new(cx + 75.0, cy - 10.0),
                Align2::CENTER_CENTER,
                format!("{:.0} × {:.0} mm", p.width, p.height),
                FontId::proportional(11.5),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 75.0, cy + 10.0),
                Align2::CENTER_CENTER,
                format!("{}  •  U={:.2}", p.operation_type, p.thermal_transmittance),
                FontId::proportional(9.5),
                theme.accent,
            );
        }

        ToolCategory::Footing => {
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.24);
            let pad_w = ((p.width as f32 / 2400.0) * 110.0).clamp(48.0, 120.0);
            let pad_h = ((p.thickness as f32 / 800.0) * 42.0).clamp(18.0, 48.0);
            let bx = cx - 55.0;
            let by = cy + 16.0;
            match p.footing_style {
                FootingStyle::Pad => {
                    let r = Rect::from_center_size(Pos2::new(bx, by), Vec2::new(pad_w, pad_h));
                    painter.rect_filled(r, 1.0, fill);
                    painter.rect_stroke(r, 1.0, stroke, StrokeKind::Inside);
                }
                FootingStyle::Stepped => {
                    let r1 = Rect::from_center_size(Pos2::new(bx, by + pad_h * 0.22), Vec2::new(pad_w, pad_h * 0.56));
                    let r2 = Rect::from_center_size(Pos2::new(bx, by - pad_h * 0.28), Vec2::new(pad_w * 0.60, pad_h * 0.44));
                    painter.rect_filled(r1, 1.0, fill);
                    painter.rect_stroke(r1, 1.0, stroke, StrokeKind::Inside);
                    painter.rect_filled(r2, 1.0, fill);
                    painter.rect_stroke(r2, 1.0, stroke, StrokeKind::Inside);
                }
                FootingStyle::Sloped => {
                    let y_bot = by + pad_h * 0.5;
                    let y_mid = by + pad_h * 0.05;
                    let y_top = by - pad_h * 0.5;
                    let pts = vec![
                        Pos2::new(bx - pad_w * 0.5, y_bot),
                        Pos2::new(bx + pad_w * 0.5, y_bot),
                        Pos2::new(bx + pad_w * 0.5, y_mid),
                        Pos2::new(bx + pad_w * 0.28, y_top),
                        Pos2::new(bx - pad_w * 0.28, y_top),
                        Pos2::new(bx - pad_w * 0.5, y_mid),
                    ];
                    painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
                }
            }
            painter.text(
                Pos2::new(cx + 68.0, cy - 10.0),
                Align2::CENTER_CENTER,
                format!("{:.0} × {:.0} × {:.0} mm", p.width, p.depth, p.thickness),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 68.0, cy + 10.0),
                Align2::CENTER_CENTER,
                format!("q_allow {:.0} kPa  •  {}", p.soil_bearing_capacity, p.concrete_grade),
                FontId::proportional(10.0),
                theme.accent,
            );
        }

        ToolCategory::CurtainWall => {
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = Color32::from_rgba_unmultiplied(100, 180, 240, 65);
            let r = Rect::from_center_size(Pos2::new(cx - 50.0, cy), Vec2::new(90.0, 68.0));
            // Translucent glass fill
            painter.rect_filled(r, 0.0, fill);
            // Outer frame
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
            // Intermediate mullions
            let xm1 = r.min.x + r.width() * 0.333;
            let xm2 = r.min.x + r.width() * 0.666;
            painter.line_segment([Pos2::new(xm1, r.min.y), Pos2::new(xm1, r.max.y)], stroke);
            painter.line_segment([Pos2::new(xm2, r.min.y), Pos2::new(xm2, r.max.y)], stroke);
            // Intermediate transom
            let ym = r.min.y + r.height() * 0.5;
            painter.line_segment([Pos2::new(r.min.x, ym), Pos2::new(r.max.x, ym)], stroke);

            painter.text(
                Pos2::new(cx + 65.0, cy - 14.0),
                Align2::CENTER_CENTER,
                format!("Grid: {:.0} × {:.0} mm", p.grid_spacing_x, p.grid_spacing_y),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 65.0, cy + 2.0),
                Align2::CENTER_CENTER,
                format!("Mullion: {:.0} × {:.0} mm  •  Glass {:.0} mm", p.width, p.depth, p.thickness),
                FontId::proportional(9.5),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(cx + 65.0, cy + 18.0),
                Align2::CENTER_CENTER,
                format!("U = {:.2} W/m²K  •  {}", p.thermal_transmittance, p.fire_rating),
                FontId::proportional(9.5),
                theme.accent,
            );
        }

        ToolCategory::Fascia | ToolCategory::Gutter => {
            let stroke = Stroke::new(2.0, theme.accent);
            let fill = theme.accent.gamma_multiply(0.24);
            if p.category == ToolCategory::Fascia {
                let r = Rect::from_center_size(Pos2::new(cx - 45.0, cy), Vec2::new(16.0, 72.0));
                painter.rect_filled(r, 1.0, fill);
                painter.rect_stroke(r, 1.0, stroke, StrokeKind::Inside);
            } else {
                let r = Rect::from_center_size(Pos2::new(cx - 45.0, cy), Vec2::new(56.0, 40.0));
                painter.line_segment([r.left_top(), r.left_bottom()], stroke);
                painter.line_segment([r.left_bottom(), r.right_bottom()], stroke);
                painter.line_segment([r.right_bottom(), r.right_top()], stroke);
            }
            painter.text(
                Pos2::new(cx + 60.0, cy),
                Align2::CENTER_CENTER,
                format!("{}  •  {}", p.summary_badge(), p.material_name),
                FontId::proportional(10.5),
                theme.text_primary,
            );
        }

        ToolCategory::DimensionLinear => {
            let stroke = Stroke::new(1.4, theme.accent);
            let p1 = Pos2::new(cx - 90.0, cy + 15.0);
            let p2 = Pos2::new(cx + 10.0, cy + 15.0);
            let p1_dim = Pos2::new(cx - 90.0, cy - 15.0);
            let p2_dim = Pos2::new(cx + 10.0, cy - 15.0);

            // Witness lines
            painter.line_segment([p1, Pos2::new(p1_dim.x, p1_dim.y - 6.0)], Stroke::new(1.0, theme.text_muted));
            painter.line_segment([p2, Pos2::new(p2_dim.x, p2_dim.y - 6.0)], Stroke::new(1.0, theme.text_muted));
            // Dimension line
            painter.line_segment([p1_dim, p2_dim], stroke);
            // Tick marks
            match p.tick_mark {
                plinth_model::TickMark::Tick45 => {
                    painter.line_segment([Pos2::new(p1_dim.x - 4.0, p1_dim.y + 4.0), Pos2::new(p1_dim.x + 4.0, p1_dim.y - 4.0)], Stroke::new(2.0, theme.accent));
                    painter.line_segment([Pos2::new(p2_dim.x - 4.0, p2_dim.y + 4.0), Pos2::new(p2_dim.x + 4.0, p2_dim.y - 4.0)], Stroke::new(2.0, theme.accent));
                }
                plinth_model::TickMark::Arrow | plinth_model::TickMark::OpenArrow => {
                    painter.line_segment([p1_dim, Pos2::new(p1_dim.x + 6.0, p1_dim.y - 3.0)], Stroke::new(1.5, theme.accent));
                    painter.line_segment([p1_dim, Pos2::new(p1_dim.x + 6.0, p1_dim.y + 3.0)], Stroke::new(1.5, theme.accent));
                    painter.line_segment([p2_dim, Pos2::new(p2_dim.x - 6.0, p2_dim.y - 3.0)], Stroke::new(1.5, theme.accent));
                    painter.line_segment([p2_dim, Pos2::new(p2_dim.x - 6.0, p2_dim.y + 3.0)], Stroke::new(1.5, theme.accent));
                }
                _ => {
                    painter.circle_filled(p1_dim, 2.5, theme.accent);
                    painter.circle_filled(p2_dim, 2.5, theme.accent);
                }
            }
            // Value
            let val_str = format!("{}{:.0}{}", p.prefix.as_deref().unwrap_or(""), 4500.0, p.suffix.as_deref().unwrap_or(""));
            painter.text(Pos2::new((p1_dim.x + p2_dim.x) * 0.5, p1_dim.y - 9.0), Align2::CENTER_CENTER, val_str, FontId::proportional(11.0), theme.text_primary);
            painter.text(Pos2::new(cx + 80.0, cy), Align2::CENTER_CENTER, format!("Linear Dimension String\nContinuous Chain: {}", if p.continuous_string { "ON" } else { "OFF" }), FontId::proportional(10.0), theme.accent);
        }

        ToolCategory::DimensionCircular => {
            let stroke = Stroke::new(1.4, theme.accent);
            let center = Pos2::new(cx - 50.0, cy);
            painter.circle_stroke(center, 28.0, Stroke::new(1.0, theme.text_muted.gamma_multiply(0.5)));
            let rim = Pos2::new(center.x + 20.0, center.y - 19.5);
            let shoulder = Pos2::new(center.x + 45.0, center.y - 30.0);
            let shelf = Pos2::new(center.x + 85.0, center.y - 30.0);
            painter.line_segment([center, rim], Stroke::new(1.0, theme.text_muted));
            painter.line_segment([rim, shoulder], stroke);
            painter.line_segment([shoulder, shelf], stroke);
            painter.circle_filled(center, 2.0, theme.accent);
            painter.text(Pos2::new(shoulder.x + 20.0, shoulder.y - 8.0), Align2::CENTER_CENTER, if p.is_diameter { "Ø 1,800" } else { "R 900" }, FontId::proportional(11.0), theme.text_primary);
        }

        ToolCategory::DimensionAngular => {
            let apex = Pos2::new(cx - 60.0, cy + 20.0);
            let r1 = Pos2::new(apex.x + 50.0, apex.y);
            let r2 = Pos2::new(apex.x + 35.0, apex.y - 35.0);
            painter.line_segment([apex, r1], Stroke::new(1.2, theme.text_muted));
            painter.line_segment([apex, r2], Stroke::new(1.2, theme.text_muted));
            painter.text(Pos2::new(apex.x + 30.0, apex.y - 12.0), Align2::CENTER_CENTER, "45.0°", FontId::proportional(11.0), theme.accent);
            painter.text(Pos2::new(cx + 70.0, cy), Align2::CENTER_CENTER, "Angular Dimension\nArc Radius: 600mm", FontId::proportional(10.0), theme.text_primary);
        }

        ToolCategory::TextNote => {
            let box_rect = Rect::from_center_size(Pos2::new(cx - 30.0, cy), Vec2::new(140.0, 48.0));
            painter.rect_filled(box_rect, 2.0, theme.accent.gamma_multiply(0.10));
            painter.rect_stroke(box_rect, 2.0, Stroke::new(1.0, theme.accent), StrokeKind::Inside);
            painter.text(Pos2::new(box_rect.min.x + 8.0, cy), Align2::LEFT_CENTER, "Aa  PROJECT NOTE", FontId::proportional(12.0), theme.text_primary);
            painter.text(Pos2::new(cx + 85.0, cy), Align2::CENTER_CENTER, format!("Font: {:.1}mm\nStyle: {:?}", p.font_size, p.font_style), FontId::proportional(10.0), theme.accent);
        }

        ToolCategory::Leader => {
            let stroke = Stroke::new(1.4, theme.accent);
            let target = Pos2::new(cx - 85.0, cy + 18.0);
            let elbow = Pos2::new(cx - 35.0, cy - 10.0);
            let shelf = Pos2::new(cx + 15.0, cy - 10.0);
            painter.line_segment([target, elbow], stroke);
            painter.line_segment([elbow, shelf], stroke);
            // Arrow tip
            painter.line_segment([target, Pos2::new(target.x + 6.0, target.y - 2.0)], Stroke::new(1.6, theme.accent));
            painter.line_segment([target, Pos2::new(target.x + 2.0, target.y - 6.0)], Stroke::new(1.6, theme.accent));
            painter.text(Pos2::new(shelf.x + 6.0, shelf.y), Align2::LEFT_CENTER, "MATERIAL SPEC", FontId::proportional(10.5), theme.text_primary);
        }

        ToolCategory::DetailLine => {
            let p1 = Pos2::new(cx - 90.0, cy);
            let p2 = Pos2::new(cx + 90.0, cy);
            let weight = (p.line_weight * 4.0).clamp(1.0, 6.0);
            if p.line_style.contains("Dashed") || p.line_style.contains("Hidden") {
                let steps = 12;
                for s in 0..steps {
                    if s % 2 == 0 {
                        let x0 = p1.x + (s as f32 / steps as f32) * 180.0;
                        let x1 = p1.x + ((s + 1) as f32 / steps as f32) * 180.0;
                        painter.line_segment([Pos2::new(x0, cy), Pos2::new(x1, cy)], Stroke::new(weight, theme.accent));
                    }
                }
            } else {
                painter.line_segment([p1, p2], Stroke::new(weight, theme.accent));
            }
            painter.text(Pos2::new(cx, cy - 16.0), Align2::CENTER_CENTER, format!("Style: {}  •  Weight: {:.2}mm", p.line_style, p.line_weight), FontId::proportional(10.0), theme.text_primary);
        }

        ToolCategory::Road => {
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.25);
            let r_w = 160.0f32;
            let r_h = 24.0f32;
            let crown_dy = 4.0f32;

            let top_left = Pos2::new(cx - 50.0 - r_w * 0.5, cy - r_h * 0.5 + crown_dy);
            let top_mid = Pos2::new(cx - 50.0, cy - r_h * 0.5);
            let top_right = Pos2::new(cx - 50.0 + r_w * 0.5, cy - r_h * 0.5 + crown_dy);
            let bot_right = Pos2::new(cx - 50.0 + r_w * 0.5, cy + r_h * 0.5);
            let bot_left = Pos2::new(cx - 50.0 - r_w * 0.5, cy + r_h * 0.5);

            painter.add(egui::Shape::convex_polygon(
                vec![top_left, top_mid, top_right, bot_right, bot_left],
                fill,
                stroke,
            ));

            let curb_w = 8.0f32;
            let curb_h = 10.0f32;
            let l_curb = Rect::from_min_max(Pos2::new(top_left.x - curb_w, top_left.y - curb_h), Pos2::new(top_left.x, bot_left.y));
            let r_curb = Rect::from_min_max(Pos2::new(top_right.x, top_right.y - curb_h), Pos2::new(top_right.x + curb_w, bot_right.y));
            painter.rect_filled(l_curb, 1.0, theme.accent.gamma_multiply(0.40));
            painter.rect_filled(r_curb, 1.0, theme.accent.gamma_multiply(0.40));
            painter.rect_stroke(l_curb, 1.0, stroke, StrokeKind::Inside);
            painter.rect_stroke(r_curb, 1.0, stroke, StrokeKind::Inside);

            let yellow = Color32::from_rgb(255, 190, 0);
            painter.line_segment([top_mid, Pos2::new(cx - 50.0, cy + r_h * 0.5)], Stroke::new(1.8, yellow));

            painter.text(
                Pos2::new(cx + 80.0, cy - 14.0),
                Align2::CENTER_CENTER,
                format!("Road: {:.1}m  •  {} Lanes", p.width / 1000.0, p.lanes),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 80.0, cy + 2.0),
                Align2::CENTER_CENTER,
                format!("Lane: {:.1}m  •  Crown: {:.1}%", p.lane_width / 1000.0, p.crown_slope_pct),
                FontId::proportional(9.5),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(cx + 80.0, cy + 18.0),
                Align2::CENTER_CENTER,
                format!("Curb: {:?} ({}×{}mm)", p.road_curb, p.curb_width as i32, p.curb_height as i32),
                FontId::proportional(9.5),
                theme.accent,
            );
        }

        ToolCategory::Sidewalk => {
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.22);
            let sw_w = 140.0f32;
            let sw_h = 28.0f32;
            let curb_drop = 12.0f32;

            let slab = Rect::from_min_max(
                Pos2::new(cx - 50.0 - sw_w * 0.5, cy - sw_h * 0.5),
                Pos2::new(cx - 50.0 + sw_w * 0.5, cy + sw_h * 0.5),
            );
            painter.rect_filled(slab, 1.0, fill);
            painter.rect_stroke(slab, 1.0, stroke, StrokeKind::Inside);

            let bullnose_c = Pos2::new(slab.max.x, slab.min.y + 4.0);
            painter.circle_stroke(bullnose_c, 4.0, Stroke::new(2.2, Color32::from_rgb(0, 200, 255)));
            painter.line_segment([slab.right_bottom(), Pos2::new(slab.max.x, slab.max.y + curb_drop)], Stroke::new(2.0, theme.accent));

            for i in 1..=3 {
                let gx = slab.min.x + (slab.width() / 4.0) * (i as f32);
                painter.line_segment([Pos2::new(gx, slab.min.y), Pos2::new(gx, slab.min.y + 5.0)], Stroke::new(1.5, theme.accent));
            }

            painter.text(
                Pos2::new(cx + 75.0, cy - 14.0),
                Align2::CENTER_CENTER,
                format!("Sidewalk: {:.1}m  •  {}mm Thick", p.width / 1000.0, p.thickness as i32),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 75.0, cy + 2.0),
                Align2::CENTER_CENTER,
                format!("Edge: {:?}  •  Curb: {}mm", p.sidewalk_edge, p.curb_height as i32),
                FontId::proportional(9.5),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(cx + 75.0, cy + 18.0),
                Align2::CENTER_CENTER,
                format!("Grooves: @ {:.1}m  •  Depth {}mm", p.groove_spacing / 1000.0, p.groove_depth as i32),
                FontId::proportional(9.5),
                theme.accent,
            );
        }

        ToolCategory::Pathwalk => {
            let stroke = Stroke::new(1.8, theme.accent);
            let fill = theme.accent.gamma_multiply(0.20);
            let pw_w = 130.0f32;
            let pw_h = 32.0f32;
            let slab = Rect::from_center_size(Pos2::new(cx - 50.0, cy), Vec2::new(pw_w, pw_h));

            painter.rect_filled(slab, 1.0, fill);
            painter.rect_stroke(slab, 1.0, stroke, StrokeKind::Inside);

            let edge_w = 8.0f32;
            let l_edge = Rect::from_min_max(slab.min, Pos2::new(slab.min.x + edge_w, slab.max.y));
            let r_edge = Rect::from_min_max(Pos2::new(slab.max.x - edge_w, slab.min.y), slab.max);
            painter.rect_filled(l_edge, 0.0, theme.accent.gamma_multiply(0.38));
            painter.rect_filled(r_edge, 0.0, theme.accent.gamma_multiply(0.38));
            painter.rect_stroke(l_edge, 0.0, Stroke::new(1.2, theme.accent), StrokeKind::Inside);
            painter.rect_stroke(r_edge, 0.0, Stroke::new(1.2, theme.accent), StrokeKind::Inside);

            let n_pavers = 5;
            let p_w = (slab.width() - edge_w * 2.0) / n_pavers as f32;
            for i in 1..n_pavers {
                let px = slab.min.x + edge_w + i as f32 * p_w;
                painter.line_segment([Pos2::new(px, slab.min.y), Pos2::new(px, slab.max.y)], Stroke::new(1.0, theme.accent.gamma_multiply(0.70)));
            }

            painter.text(
                Pos2::new(cx + 75.0, cy - 14.0),
                Align2::CENTER_CENTER,
                format!("Pathwalk: {:.1}m  •  {:?}", p.width / 1000.0, p.pathwalk_profile),
                FontId::proportional(11.0),
                theme.text_primary,
            );
            painter.text(
                Pos2::new(cx + 75.0, cy + 2.0),
                Align2::CENTER_CENTER,
                format!("Thickness: {}mm  •  Joint: {}mm", p.thickness as i32, p.groove_width as i32),
                FontId::proportional(9.5),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(cx + 75.0, cy + 18.0),
                Align2::CENTER_CENTER,
                format!("Edging: {} ({}mm)", if p.has_edging { "Active" } else { "None" }, p.edging_width as i32),
                FontId::proportional(9.5),
                theme.accent,
            );
        }

        _ => {}
    }
}

// =============================================================================
// Parameter Inspector & Compound Layer Stack Editor
// =============================================================================

fn render_preset_editor(ui: &mut Ui, p: &mut TypePreset, theme: &AppTheme) {
    // 1. Identity Card
    CollapsingHeader::new("1. Type Identity & Classification")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Type Name:");
                ui.text_edit_singleline(&mut p.type_name);
            });
            ui.horizontal(|ui| {
                ui.label("Tag Prefix:");
                ui.add(egui::TextEdit::singleline(&mut p.tag_prefix).desired_width(60.0));
                ui.label("Description:");
                ui.text_edit_singleline(&mut p.description);
            });
        });

    // 2. Compound Assembly Layers (Wall, Floor, Slab, Roof, Ceiling)
    if p.category.has_compound_layers() {
        CollapsingHeader::new("2. Compound Material Layers (Exterior/Top → Interior/Bottom)")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("➕ Add Layer").clicked() {
                        p.layers.push(make_layer(
                            LayerFunction::InteriorFinish,
                            "Gypsum Board 12.5mm",
                            12.5,
                            0.25,
                        ));
                        p.sync_from_layers();
                    }
                    if ui
                        .checkbox(
                            &mut p.auto_sync_layers,
                            "Auto-sync Total Thickness & U-Value from Layers",
                        )
                        .changed()
                    {
                        p.sync_from_layers();
                    }
                });

                let mut move_up: Option<usize> = None;
                let mut move_down: Option<usize> = None;
                let mut remove_idx: Option<usize> = None;
                let mut changed_any = false;
                let layer_len = p.layers.len();

                for (idx, layer) in p.layers.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let swatch_col = layer_function_color(layer.function);
                        let (sw_rect, _) =
                            ui.allocate_exact_size(Vec2::new(8.0, 18.0), Sense::hover());
                        ui.painter().rect_filled(sw_rect, 2.0, swatch_col);

                        if ui.add_enabled(idx > 0, egui::Button::new("↑").small()).clicked() {
                            move_up = Some(idx);
                        }
                        if ui
                            .add_enabled(idx + 1 < layer_len, egui::Button::new("↓").small())
                            .clicked()
                        {
                            move_down = Some(idx);
                        }

                        ComboBox::from_id_salt(("layer_fn", idx))
                            .width(136.0)
                            .selected_text(layer.function.label())
                            .show_ui(ui, |ui| {
                                for f in [
                                    LayerFunction::ExteriorFinish,
                                    LayerFunction::MembraneBarrier,
                                    LayerFunction::AirGap,
                                    LayerFunction::ThermalInsulation,
                                    LayerFunction::Substrate,
                                    LayerFunction::StructuralCore,
                                    LayerFunction::InteriorFinish,
                                ] {
                                    if ui
                                        .selectable_value(&mut layer.function, f, f.label())
                                        .changed()
                                    {
                                        layer.name = f.label().to_string();
                                        changed_any = true;
                                    }
                                }
                            });

                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut layer.material_name)
                                    .desired_width(130.0),
                            )
                            .changed()
                        {
                            changed_any = true;
                        }

                        if ui
                            .add(
                                DragValue::new(&mut layer.thickness)
                                    .speed(2.0)
                                    .range(0.0..=2000.0)
                                    .suffix(" mm"),
                            )
                            .changed()
                        {
                            changed_any = true;
                        }

                        if ui
                            .add(
                                DragValue::new(&mut layer.thermal_conductivity)
                                    .speed(0.02)
                                    .range(0.015..=55.0)
                                    .prefix("λ "),
                            )
                            .on_hover_text("Thermal conductivity λ (W/m·K)")
                            .changed()
                        {
                            changed_any = true;
                        }

                        if layer_len > 1 && ui.small_button("✕").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
                }

                if let Some(i) = move_up {
                    p.layers.swap(i - 1, i);
                    changed_any = true;
                }
                if let Some(i) = move_down {
                    p.layers.swap(i, i + 1);
                    changed_any = true;
                }
                if let Some(i) = remove_idx {
                    p.layers.remove(i);
                    changed_any = true;
                }
                if changed_any {
                    p.sync_from_layers();
                }

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Assembly Total Thickness: {:.1} mm", p.thickness))
                            .strong()
                            .color(theme.accent),
                    );
                    if !p.auto_sync_layers {
                        ui.add(
                            DragValue::new(&mut p.thickness)
                                .speed(5.0)
                                .range(10.0..=3000.0)
                                .suffix(" mm"),
                        );
                    }
                });
            });
    }

    // 3. Category-Specific Parametric Geometry
    CollapsingHeader::new("3. Parametric Geometry & Construction Style")
        .default_open(true)
        .show(ui, |ui| match p.category {
            ToolCategory::WallArch | ToolCategory::WallStruct => {
                ui.label("Wall thickness is driven by the Compound Layer Stack above.");
            }
            ToolCategory::CurtainWall => {
                ui.horizontal(|ui| {
                    ui.label("Mullion Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(5.0)
                            .range(20.0..=300.0)
                            .suffix(" mm"),
                    );
                    ui.label("Mullion Depth:");
                    ui.add(
                        DragValue::new(&mut p.depth)
                            .speed(5.0)
                            .range(40.0..=500.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Glass Panel Thickness:");
                    ui.add(
                        DragValue::new(&mut p.thickness)
                            .speed(1.0)
                            .range(6.0..=80.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Vertical Grid Spacing:");
                    ui.add(
                        DragValue::new(&mut p.grid_spacing_x)
                            .speed(50.0)
                            .range(300.0..=5000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Horizontal Grid Spacing:");
                    ui.add(
                        DragValue::new(&mut p.grid_spacing_y)
                            .speed(50.0)
                            .range(300.0..=5000.0)
                            .suffix(" mm"),
                    );
                });
            }
            ToolCategory::FloorArch | ToolCategory::SlabStruct => {
                ui.label("Slab thickness is driven by the Compound Layer Stack above.");
            }
            ToolCategory::Roof => {
                ui.horizontal(|ui| {
                    ui.label("Roof Shape:");
                    ComboBox::from_id_salt("studio_roof_shape")
                        .selected_text(format!("{:?}", p.roof_shape))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.roof_shape, RoofShape::Hip, "Hip Roof");
                            ui.selectable_value(&mut p.roof_shape, RoofShape::Gable, "Gable Roof");
                            ui.selectable_value(&mut p.roof_shape, RoofShape::Shed, "Shed Roof");
                            ui.selectable_value(&mut p.roof_shape, RoofShape::Flat, "Flat Roof");
                        });
                    if p.roof_shape != RoofShape::Flat {
                        ui.label("Pitch:");
                        ui.add(
                            DragValue::new(&mut p.slope_degrees)
                                .speed(0.5)
                                .range(1.0..=75.0)
                                .suffix("°"),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Eave Overhang:");
                    ui.add(
                        DragValue::new(&mut p.overhang)
                            .speed(10.0)
                            .range(0.0..=3000.0)
                            .suffix(" mm"),
                    );
                });
            }
            ToolCategory::Ceiling => {
                ui.horizontal(|ui| {
                    ui.label("Acoustic Tile Grid X:");
                    ui.add(
                        DragValue::new(&mut p.grid_spacing_x)
                            .speed(50.0)
                            .range(100.0..=3000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Grid Y:");
                    ui.add(
                        DragValue::new(&mut p.grid_spacing_y)
                            .speed(50.0)
                            .range(100.0..=3000.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Light Reflectance (LR):");
                    ui.add(
                        DragValue::new(&mut p.light_reflectance)
                            .speed(0.02)
                            .range(0.1..=1.0),
                    );
                });
            }
            ToolCategory::Door => {
                ui.horizontal(|ui| {
                    ui.label("Clear Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(10.0)
                            .range(400.0..=6000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Clear Height:");
                    ui.add(
                        DragValue::new(&mut p.height)
                            .speed(10.0)
                            .range(1200.0..=5000.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Threshold / Sill:");
                    ui.add(DragValue::new(&mut p.sill_height).speed(5.0).suffix(" mm"));
                    ui.label("Operation:");
                    ComboBox::from_id_salt("studio_door_op")
                        .selected_text(&p.operation_type)
                        .show_ui(ui, |ui| {
                            for op in [
                                "SINGLE_SWING_LEFT",
                                "SINGLE_SWING_RIGHT",
                                "DOUBLE_DOOR_SINGLE_SWING",
                                "SLIDING_TO_LEFT",
                                "FOLDING_TO_LEFT",
                                "REVOLVING",
                            ] {
                                ui.selectable_value(&mut p.operation_type, op.to_string(), op);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.flipped_hand, "Flip Hand");
                    ui.checkbox(&mut p.flipped_face, "Flip Swing Side");
                    ui.checkbox(&mut p.is_fire_exit, "Emergency Egress Exit");
                });
            }
            ToolCategory::Window => {
                ui.horizontal(|ui| {
                    ui.label("Frame Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(10.0)
                            .range(300.0..=8000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Frame Height:");
                    ui.add(
                        DragValue::new(&mut p.height)
                            .speed(10.0)
                            .range(300.0..=6000.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Default Sill Height:");
                    ui.add(DragValue::new(&mut p.sill_height).speed(10.0).suffix(" mm"));
                    ui.label("Operation:");
                    ComboBox::from_id_salt("studio_win_op")
                        .selected_text(&p.operation_type)
                        .show_ui(ui, |ui| {
                            for op in [
                                "SINGLE_PANEL",
                                "DOUBLE_PANEL_VERTICAL",
                                "DOUBLE_PANEL_HORIZONTAL",
                                "TRIPLE_PANEL_VERTICAL",
                                "SIDE_HUNG_RIGHTHAND",
                                "TOP_HUNG",
                            ] {
                                ui.selectable_value(&mut p.operation_type, op.to_string(), op);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Solar Heat Gain (SHGC):");
                    ui.add(DragValue::new(&mut p.shgc).speed(0.02).range(0.05..=0.95));
                });
            }
            ToolCategory::ColumnArch | ToolCategory::ColumnStruct => {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.is_round, "Circular Column Profile");
                    ui.label(if p.is_round { "Diameter:" } else { "Width (b):" });
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(10.0)
                            .range(100.0..=3000.0)
                            .suffix(" mm"),
                    );
                    if !p.is_round {
                        ui.label("Depth (h):");
                        ui.add(
                            DragValue::new(&mut p.depth)
                                .speed(10.0)
                                .range(100.0..=3000.0)
                                .suffix(" mm"),
                        );
                    }
                });
            }
            ToolCategory::Footing => {
                ui.horizontal(|ui| {
                    ui.label("Foundation Geometry:");
                    ComboBox::from_id_salt("studio_footing_style")
                        .selected_text(p.footing_style.label())
                        .show_ui(ui, |ui| {
                            for st in [
                                FootingStyle::Pad,
                                FootingStyle::Stepped,
                                FootingStyle::Sloped,
                            ] {
                                ui.selectable_value(&mut p.footing_style, st, st.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Width (B):");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(25.0)
                            .range(400.0..=6000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Length (L):");
                    ui.add(
                        DragValue::new(&mut p.depth)
                            .speed(25.0)
                            .range(400.0..=6000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Depth (D):");
                    ui.add(
                        DragValue::new(&mut p.thickness)
                            .speed(10.0)
                            .range(150.0..=3000.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Allowable Soil Bearing:");
                    ui.add(
                        DragValue::new(&mut p.soil_bearing_capacity)
                            .speed(5.0)
                            .range(50.0..=2000.0)
                            .suffix(" kPa"),
                    );
                    ui.label("Concrete Grade:");
                    ui.add(egui::TextEdit::singleline(&mut p.concrete_grade).desired_width(80.0));
                });
            }
            ToolCategory::Beam => {
                ui.horizontal(|ui| {
                    ui.label("Web / Flange Width (b):");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(10.0)
                            .range(80.0..=2000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Overall Depth (h):");
                    ui.add(
                        DragValue::new(&mut p.depth)
                            .speed(10.0)
                            .range(100.0..=3000.0)
                            .suffix(" mm"),
                    );
                });
            }
            ToolCategory::Stair => {
                ui.horizontal(|ui| {
                    ui.label("Construction Style:");
                    ComboBox::from_id_salt("studio_stair_style")
                        .selected_text(p.stair_style.label())
                        .show_ui(ui, |ui| {
                            for st in [
                                StairStyle::Monolithic,
                                StairStyle::SolidStepped,
                                StairStyle::OpenRiser,
                            ] {
                                ui.selectable_value(&mut p.stair_style, st, st.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Flight Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(20.0)
                            .range(600.0..=5000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Riser Count:");
                    ui.add(DragValue::new(&mut p.riser_count).range(2..=60));
                });
                ui.horizontal(|ui| {
                    ui.label("Waist / Stringer:");
                    ui.add(
                        DragValue::new(&mut p.waist_thickness)
                            .speed(5.0)
                            .range(60.0..=500.0)
                            .suffix(" mm"),
                    );
                    ui.label("Ref Going (G):");
                    ui.add(
                        DragValue::new(&mut p.target_going)
                            .speed(5.0)
                            .range(200.0..=400.0)
                            .suffix(" mm"),
                    );
                });
            }
            ToolCategory::Ramp => {
                ui.horizontal(|ui| {
                    ui.label("Ramp Construction:");
                    ComboBox::from_id_salt("studio_ramp_style")
                        .selected_text(p.ramp_style.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut p.ramp_style,
                                RampStyle::SlopedSlab,
                                RampStyle::SlopedSlab.label(),
                            );
                            ui.selectable_value(
                                &mut p.ramp_style,
                                RampStyle::SolidWedge,
                                RampStyle::SolidWedge.label(),
                            );
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Clear Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(20.0)
                            .range(600.0..=6000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Slab Thickness:");
                    ui.add(
                        DragValue::new(&mut p.thickness)
                            .speed(10.0)
                            .range(80.0..=1000.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Quick Rise Presets:");
                    if ui.small_button("Threshold 150 mm").clicked() {
                        p.unconnected_height = 150.0;
                    }
                    if ui.small_button("Mezzanine 1500 mm").clicked() {
                        p.unconnected_height = 1500.0;
                    }
                    if ui.small_button("Full Storey 3000 mm").clicked() {
                        p.unconnected_height = 3000.0;
                    }
                });
            }
            ToolCategory::Railing => {
                ui.horizontal(|ui| {
                    ui.label("Baluster Spacing:");
                    ui.add(
                        DragValue::new(&mut p.baluster_spacing)
                            .speed(5.0)
                            .range(50.0..=1000.0)
                            .suffix(" mm"),
                    );
                    ui.label("Horizontal Mid-Rails:");
                    ui.add(DragValue::new(&mut p.mid_rail_count).range(0..=8));
                });
                ui.horizontal(|ui| {
                    ui.label("Material:");
                    ui.text_edit_singleline(&mut p.material_name);
                });
            }
            ToolCategory::Fascia => {
                ui.horizontal(|ui| {
                    ui.label("Fascia Height:");
                    ui.add(
                        DragValue::new(&mut p.height)
                            .speed(5.0)
                            .range(80.0..=800.0)
                            .suffix(" mm"),
                    );
                    ui.label("Board Thickness:");
                    ui.add(
                        DragValue::new(&mut p.thickness)
                            .speed(2.0)
                            .range(10.0..=150.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Material:");
                    ui.text_edit_singleline(&mut p.material_name);
                });
            }
            ToolCategory::Gutter => {
                ui.horizontal(|ui| {
                    ui.label("Trough Width:");
                    ui.add(
                        DragValue::new(&mut p.width)
                            .speed(5.0)
                            .range(60.0..=500.0)
                            .suffix(" mm"),
                    );
                    ui.label("Trough Depth:");
                    ui.add(
                        DragValue::new(&mut p.depth)
                            .speed(5.0)
                            .range(50.0..=400.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Material:");
                    ui.text_edit_singleline(&mut p.material_name);
                });
            }
            ToolCategory::DimensionLinear | ToolCategory::DimensionCircular | ToolCategory::DimensionAngular => {
                ui.horizontal(|ui| {
                    ui.label("Tick Mark Style:");
                    ComboBox::from_id_salt("dim_tick_cfg")
                        .selected_text(match p.tick_mark {
                            plinth_model::TickMark::Tick45 => "45° Architectural Tick",
                            plinth_model::TickMark::Arrow => "Filled Arrowhead",
                            plinth_model::TickMark::OpenArrow => "Open Arrowhead",
                            plinth_model::TickMark::Dot => "Filled Dot",
                            plinth_model::TickMark::None => "None",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Tick45, "45° Architectural Tick");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Arrow, "Filled Arrowhead");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::OpenArrow, "Open Arrowhead");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Dot, "Filled Dot");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::None, "None");
                        });
                    ui.label("Font Size:");
                    ui.add(DragValue::new(&mut p.font_size).speed(0.1).range(1.0..=14.0).suffix(" mm"));
                });
                ui.horizontal(|ui| {
                    ui.label("Font Style:");
                    ComboBox::from_id_salt("dim_font_style")
                        .selected_text(format!("{:?}", p.font_style))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Regular, "Regular");
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Bold, "Bold");
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Italic, "Italic");
                        });
                    ui.label("Default Offset:");
                    ui.add(DragValue::new(&mut p.dim_offset).speed(10.0).range(100.0..=5000.0).suffix(" mm"));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.continuous_string, "Automatic Snapping & Chaining on Dimension Strings");
                    if p.category == ToolCategory::DimensionCircular {
                        ui.checkbox(&mut p.is_diameter, "Display as Diameter (Ø)");
                    }
                });
            }
            ToolCategory::TextNote => {
                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    ui.add(DragValue::new(&mut p.font_size).speed(0.2).range(1.5..=25.0).suffix(" mm"));
                    ui.label("Typography Style:");
                    ComboBox::from_id_salt("text_font_style")
                        .selected_text(format!("{:?}", p.font_style))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Regular, "Regular");
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Bold, "Bold");
                            ui.selectable_value(&mut p.font_style, plinth_model::FontStyle::Italic, "Italic");
                        });
                });
            }
            ToolCategory::Leader => {
                ui.horizontal(|ui| {
                    ui.label("Arrowhead Tick:");
                    ComboBox::from_id_salt("leader_tick_cfg")
                        .selected_text(match p.tick_mark {
                            plinth_model::TickMark::Arrow => "Filled Arrowhead",
                            plinth_model::TickMark::OpenArrow => "Open Arrowhead",
                            plinth_model::TickMark::Tick45 => "45° Architectural Tick",
                            plinth_model::TickMark::Dot => "Filled Dot",
                            _ => "None",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Arrow, "Filled Arrowhead");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::OpenArrow, "Open Arrowhead");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Tick45, "45° Architectural Tick");
                            ui.selectable_value(&mut p.tick_mark, plinth_model::TickMark::Dot, "Filled Dot");
                        });
                    ui.label("Text Size:");
                    ui.add(DragValue::new(&mut p.font_size).speed(0.1).range(1.5..=14.0).suffix(" mm"));
                });
            }
            ToolCategory::DetailLine => {
                ui.horizontal(|ui| {
                    ui.label("Drafting Line Style:");
                    ComboBox::from_id_salt("detail_line_style")
                        .selected_text(&p.line_style)
                        .show_ui(ui, |ui| {
                            for st in ["Thin Line (0.18mm)", "Medium Object (0.35mm)", "Wide Contour (0.70mm)", "Hidden Dashed (0.25mm)", "Overhead Dashed (0.18mm)"] {
                                ui.selectable_value(&mut p.line_style, st.to_string(), st);
                            }
                        });
                    ui.label("Line Weight:");
                    ui.add(DragValue::new(&mut p.line_weight).speed(0.02).range(0.05..=3.0).suffix(" mm"));
                });
            }
            ToolCategory::Road => {
                ui.horizontal(|ui| {
                    ui.label("Road Width:");
                    if ui.add(DragValue::new(&mut p.width).speed(100.0).range(2000.0..=50000.0).suffix(" mm")).changed() {
                        if p.lanes > 0 {
                            p.lane_width = p.width / p.lanes as f64;
                        }
                    }
                    ui.label("Number of Lanes:");
                    if ui.add(DragValue::new(&mut p.lanes).range(1..=12)).changed() {
                        if p.lanes > 0 {
                            p.lane_width = p.width / p.lanes as f64;
                        }
                    }
                    ui.label("Lane Width:");
                    if ui.add(DragValue::new(&mut p.lane_width).speed(50.0).range(1500.0..=6000.0).suffix(" mm")).changed() {
                        p.width = p.lane_width * p.lanes as f64;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Pavement Thickness:");
                    ui.add(DragValue::new(&mut p.thickness).speed(10.0).range(100.0..=1000.0).suffix(" mm"));
                    ui.label("Curb Profile:");
                    ComboBox::from_id_salt("road_curb_cfg")
                        .selected_text(match p.road_curb {
                            plinth_model::RoadCurb::Barrier => "Barrier Curb (Standard)",
                            plinth_model::RoadCurb::Mountable => "Mountable Roll Curb",
                            plinth_model::RoadCurb::None => "No Curb / Flush Shoulder",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.road_curb, plinth_model::RoadCurb::Barrier, "Barrier Curb (Standard)");
                            ui.selectable_value(&mut p.road_curb, plinth_model::RoadCurb::Mountable, "Mountable Roll Curb");
                            ui.selectable_value(&mut p.road_curb, plinth_model::RoadCurb::None, "No Curb / Flush Shoulder");
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Curb Width:");
                    ui.add(DragValue::new(&mut p.curb_width).speed(10.0).range(50.0..=500.0).suffix(" mm"));
                    ui.label("Curb Height:");
                    ui.add(DragValue::new(&mut p.curb_height).speed(10.0).range(50.0..=400.0).suffix(" mm"));
                    ui.label("Crown Cross-Slope:");
                    ui.add(DragValue::new(&mut p.crown_slope_pct).speed(0.1).range(0.0..=10.0).suffix(" %"));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.has_centerline, "Paint Yellow Centerline & Lane Dividers");
                });
            }
            ToolCategory::Sidewalk => {
                ui.horizontal(|ui| {
                    ui.label("Walkway Width:");
                    ui.add(DragValue::new(&mut p.width).speed(50.0).range(600.0..=10000.0).suffix(" mm"));
                    ui.label("Slab Thickness:");
                    ui.add(DragValue::new(&mut p.thickness).speed(5.0).range(50.0..=400.0).suffix(" mm"));
                    ui.label("Curb Height:");
                    ui.add(DragValue::new(&mut p.curb_height).speed(5.0).range(0.0..=300.0).suffix(" mm"));
                });
                ui.horizontal(|ui| {
                    ui.label("Edge Profile:");
                    let current_edge_label = match p.sidewalk_edge {
                        plinth_model::SidewalkEdgeProfile::Rounded { radius } => format!("Bullnose Rounded (R{:.0}mm)", radius),
                        plinth_model::SidewalkEdgeProfile::Square => "Square Edge".to_string(),
                        plinth_model::SidewalkEdgeProfile::Beveled { chamfer } => format!("Beveled Chamfer ({:.0}mm)", chamfer),
                    };
                    ComboBox::from_id_salt("sw_edge_cfg")
                        .selected_text(current_edge_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.sidewalk_edge, plinth_model::SidewalkEdgeProfile::Rounded { radius: 25.0 }, "Bullnose Rounded (R25mm)");
                            ui.selectable_value(&mut p.sidewalk_edge, plinth_model::SidewalkEdgeProfile::Square, "Square Edge");
                            ui.selectable_value(&mut p.sidewalk_edge, plinth_model::SidewalkEdgeProfile::Beveled { chamfer: 20.0 }, "Beveled Chamfer (20mm)");
                        });
                    if let plinth_model::SidewalkEdgeProfile::Rounded { ref mut radius } = p.sidewalk_edge {
                        ui.label("Radius:");
                        ui.add(DragValue::new(radius).speed(1.0).range(5.0..=100.0).suffix(" mm"));
                    } else if let plinth_model::SidewalkEdgeProfile::Beveled { ref mut chamfer } = p.sidewalk_edge {
                        ui.label("Chamfer:");
                        ui.add(DragValue::new(chamfer).speed(1.0).range(5.0..=100.0).suffix(" mm"));
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.has_top_groove, "Top Contraction Joint Grooves");
                    if p.has_top_groove {
                        ui.label("Spacing:");
                        ui.add(DragValue::new(&mut p.groove_spacing).speed(50.0).range(500.0..=5000.0).suffix(" mm"));
                        ui.label("Depth:");
                        ui.add(DragValue::new(&mut p.groove_depth).speed(1.0).range(5.0..=50.0).suffix(" mm"));
                    }
                });
            }
            ToolCategory::Pathwalk => {
                ui.horizontal(|ui| {
                    ui.label("Path Width:");
                    ui.add(DragValue::new(&mut p.width).speed(50.0).range(500.0..=8000.0).suffix(" mm"));
                    ui.label("Bed Thickness:");
                    ui.add(DragValue::new(&mut p.thickness).speed(5.0).range(40.0..=300.0).suffix(" mm"));
                });
                ui.horizontal(|ui| {
                    ui.label("Surface Profile:");
                    ComboBox::from_id_salt("pw_profile_cfg")
                        .selected_text(match p.pathwalk_profile {
                            plinth_model::PathwalkProfile::PaverBlocks => "Interlocking Paver Blocks",
                            plinth_model::PathwalkProfile::Cobblestone => "Natural Cobblestone",
                            plinth_model::PathwalkProfile::Flagstone => "Flagstone Stepping Stones",
                            plinth_model::PathwalkProfile::GravelBed => "Gravel Bed / Pea Stone",
                            plinth_model::PathwalkProfile::TimberBoardwalk => "Timber Boardwalk Planks",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p.pathwalk_profile, plinth_model::PathwalkProfile::PaverBlocks, "Interlocking Paver Blocks");
                            ui.selectable_value(&mut p.pathwalk_profile, plinth_model::PathwalkProfile::Cobblestone, "Natural Cobblestone");
                            ui.selectable_value(&mut p.pathwalk_profile, plinth_model::PathwalkProfile::Flagstone, "Flagstone Stepping Stones");
                            ui.selectable_value(&mut p.pathwalk_profile, plinth_model::PathwalkProfile::GravelBed, "Gravel Bed / Pea Stone");
                            ui.selectable_value(&mut p.pathwalk_profile, plinth_model::PathwalkProfile::TimberBoardwalk, "Timber Boardwalk Planks");
                        });
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.has_edging, "Border Restraint Edging");
                    if p.has_edging {
                        ui.label("Edging Width:");
                        ui.add(DragValue::new(&mut p.edging_width).speed(5.0).range(20.0..=200.0).suffix(" mm"));
                    }
                    ui.label("Joint Width:");
                    ui.add(DragValue::new(&mut p.groove_width).speed(1.0).range(2.0..=30.0).suffix(" mm"));
                });
            }
        });

    // 4. Default Level Hosting & Offsets
    CollapsingHeader::new("4. Default Level Hosting & Offsets")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Default Base Offset:");
                ui.add(DragValue::new(&mut p.base_offset).speed(10.0).suffix(" mm"));
                ui.label("Default Top Offset:");
                ui.add(DragValue::new(&mut p.top_offset).speed(10.0).suffix(" mm"));
            });
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut p.top_to_next_level,
                    "Auto-constrain Top Level to Next Storey Above (otherwise Unconnected)",
                );
            });
            if !p.top_to_next_level
                && matches!(
                    p.category,
                    ToolCategory::WallArch
                        | ToolCategory::WallStruct
                        | ToolCategory::CurtainWall
                        | ToolCategory::ColumnArch
                        | ToolCategory::ColumnStruct
                        | ToolCategory::Stair
                        | ToolCategory::Ramp
                        | ToolCategory::Railing
                )
            {
                ui.horizontal(|ui| {
                    ui.label("Default Unconnected Height:");
                    ui.add(
                        DragValue::new(&mut p.unconnected_height)
                            .speed(50.0)
                            .range(100.0..=30000.0)
                            .suffix(" mm"),
                    );
                });
            }
        });

    // 5. IFC4 Property Set Standards
    CollapsingHeader::new(format!(
        "5. IFC4 Standard Properties ({})",
        p.category.pset_name()
    ))
    .default_open(false)
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut p.is_external, "IsExternal (Building Envelope)");
            ui.checkbox(&mut p.is_structural, "LoadBearing (Structural)");
        });
        ui.horizontal(|ui| {
            ui.label("Fire Rating:");
            ui.add(egui::TextEdit::singleline(&mut p.fire_rating).desired_width(90.0));
            for chip in ["Unrated", "REI 30", "REI 60", "REI 90", "REI 120"] {
                if ui.small_button(chip).clicked() {
                    p.fire_rating = chip.to_string();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Acoustic Rating:");
            ui.add(egui::TextEdit::singleline(&mut p.acoustic_rating).desired_width(90.0));
            ui.label("Thermal U:");
            ui.add(
                DragValue::new(&mut p.thermal_transmittance)
                    .speed(0.01)
                    .suffix(" W/m²K"),
            );
        });
    });
}

// =============================================================================
// Curated Cleanroom BIM Type Library Across All 16 Tool Categories
// =============================================================================

fn build_default_type_library() -> Vec<TypePreset> {
    let mut lib = Vec::new();

    // --- 1. Architectural Walls ---
    let mut w1 = TypePreset::base(
        ToolCategory::WallArch,
        "Generic Masonry Wall 200mm",
        "W",
        "Rendered masonry block wall",
    );
    w1.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Exterior Cement Render", 15.0, 0.80),
        make_layer(LayerFunction::StructuralCore, "Concrete Masonry Unit 170mm", 170.0, 1.10),
        make_layer(LayerFunction::InteriorFinish, "Gypsum Plaster Skim", 15.0, 0.40),
    ];
    lib.push(w1);

    let mut w2 = TypePreset::base(
        ToolCategory::WallArch,
        "Insulated Brick Cavity Wall 310mm",
        "EW",
        "High-performance thermal envelope wall",
    );
    w2.fire_rating = "REI 90".into();
    w2.acoustic_rating = "Rw 54 dB".into();
    w2.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Facing Clay Brick", 102.5, 0.77),
        make_layer(LayerFunction::AirGap, "Ventilated Air Cavity", 25.0, 0.18),
        make_layer(LayerFunction::ThermalInsulation, "Rigid PIR Insulation", 70.0, 0.023),
        make_layer(LayerFunction::MembraneBarrier, "Breather Vapor Membrane", 0.0, 0.20),
        make_layer(LayerFunction::StructuralCore, "Dense Concrete Block", 100.0, 1.13),
        make_layer(LayerFunction::InteriorFinish, "Gypsum Plasterboard", 12.5, 0.25),
    ];
    lib.push(w2);

    let mut w3 = TypePreset::base(
        ToolCategory::WallArch,
        "Interior Acoustic Stud Partition 125mm",
        "IW",
        "Double-boarded steel stud acoustic partition",
    );
    w3.is_external = false;
    w3.is_structural = false;
    w3.fire_rating = "EI 60".into();
    w3.acoustic_rating = "Rw 50 dB".into();
    w3.layers = vec![
        make_layer(LayerFunction::InteriorFinish, "2x Acoustic Gypsum Board", 25.0, 0.25),
        make_layer(LayerFunction::ThermalInsulation, "Mineral Wool + 75mm Stud", 75.0, 0.036),
        make_layer(LayerFunction::InteriorFinish, "2x Acoustic Gypsum Board", 25.0, 0.25),
    ];
    lib.push(w3);

    // --- 1b. Curtain Walls & Storefronts ---
    let mut cw1 = TypePreset::base(
        ToolCategory::CurtainWall,
        "Storefront Standard 1500 × 1200mm",
        "CW",
        "Standard aluminum framed commercial storefront",
    );
    cw1.width = 50.0;
    cw1.depth = 150.0;
    cw1.thickness = 28.0;
    cw1.grid_spacing_x = 1500.0;
    cw1.grid_spacing_y = 1200.0;
    cw1.unconnected_height = 3000.0;
    cw1.thermal_transmittance = 1.35;
    cw1.fire_rating = "1 HR".into();
    cw1.acoustic_rating = "36 dB".into();
    lib.push(cw1);

    let mut cw2 = TypePreset::base(
        ToolCategory::CurtainWall,
        "Glazed Curtain Wall 1200 × 1500mm",
        "CW",
        "Full-height unitized facade system",
    );
    cw2.width = 60.0;
    cw2.depth = 180.0;
    cw2.thickness = 32.0;
    cw2.grid_spacing_x = 1200.0;
    cw2.grid_spacing_y = 1500.0;
    cw2.unconnected_height = 3600.0;
    cw2.thermal_transmittance = 1.15;
    cw2.fire_rating = "2 HR".into();
    cw2.acoustic_rating = "42 dB".into();
    lib.push(cw2);

    let mut cw3 = TypePreset::base(
        ToolCategory::CurtainWall,
        "Structural Glazing 2000 × 1500mm",
        "CW",
        "Minimalist silicone-jointed architectural glazing",
    );
    cw3.width = 40.0;
    cw3.depth = 120.0;
    cw3.thickness = 24.0;
    cw3.grid_spacing_x = 2000.0;
    cw3.grid_spacing_y = 1500.0;
    cw3.unconnected_height = 3000.0;
    cw3.thermal_transmittance = 1.45;
    lib.push(cw3);

    let mut cw4 = TypePreset::base(
        ToolCategory::CurtainWall,
        "Linear Ribbon Glazing 1000 × 2000mm",
        "CW",
        "Horizontal clerestory and ribbon window system",
    );
    cw4.width = 50.0;
    cw4.depth = 140.0;
    cw4.thickness = 28.0;
    cw4.grid_spacing_x = 1000.0;
    cw4.grid_spacing_y = 2000.0;
    cw4.unconnected_height = 2400.0;
    cw4.thermal_transmittance = 1.30;
    lib.push(cw4);

    // --- 2. Doors ---
    let mut d1 = TypePreset::base(
        ToolCategory::Door,
        "Single Flush Timber 900 × 2100mm",
        "D",
        "Standard single leaf interior/entry door",
    );
    d1.width = 900.0;
    d1.height = 2100.0;
    d1.thermal_transmittance = 1.80;
    lib.push(d1);

    let mut d2 = TypePreset::base(
        ToolCategory::Door,
        "Double Glazed Entry 1800 × 2200mm",
        "D",
        "Double-leaf aluminum entrance door",
    );
    d2.width = 1800.0;
    d2.height = 2200.0;
    d2.operation_type = "DOUBLE_DOOR_SINGLE_SWING".into();
    d2.is_fire_exit = true;
    d2.thermal_transmittance = 1.40;
    lib.push(d2);

    let mut d3 = TypePreset::base(
        ToolCategory::Door,
        "Sliding Patio Door 2400 × 2100mm",
        "SD",
        "Wide thermally-broken sliding glass door",
    );
    d3.width = 2400.0;
    d3.height = 2100.0;
    d3.operation_type = "SLIDING_TO_LEFT".into();
    d3.thermal_transmittance = 1.20;
    lib.push(d3);

    // --- 3. Windows ---
    let mut wn1 = TypePreset::base(
        ToolCategory::Window,
        "Double Casement 1200 × 1500mm",
        "WN",
        "Low-E double glazed casement window",
    );
    wn1.width = 1200.0;
    wn1.height = 1500.0;
    wn1.sill_height = 900.0;
    wn1.operation_type = "DOUBLE_PANEL_VERTICAL".into();
    wn1.thermal_transmittance = 1.30;
    lib.push(wn1);

    let mut wn2 = TypePreset::base(
        ToolCategory::Window,
        "Floor-to-Ceiling Ribbon 2400 × 2400mm",
        "CW",
        "Triple-glazed architectural picture window",
    );
    wn2.width = 2400.0;
    wn2.height = 2400.0;
    wn2.sill_height = 150.0;
    wn2.operation_type = "TRIPLE_PANEL_VERTICAL".into();
    wn2.thermal_transmittance = 0.85;
    wn2.shgc = 0.32;
    lib.push(wn2);

    let mut wn3 = TypePreset::base(
        ToolCategory::Window,
        "Clerestory Awning 1500 × 600mm",
        "AW",
        "High-sill daylighting awning window",
    );
    wn3.width = 1500.0;
    wn3.height = 600.0;
    wn3.sill_height = 2100.0;
    wn3.operation_type = "TOP_HUNG".into();
    wn3.thermal_transmittance = 1.35;
    lib.push(wn3);

    // --- 4. Architectural Columns ---
    let mut ca1 = TypePreset::base(
        ToolCategory::ColumnArch,
        "Clad Square Column 400 × 400mm",
        "AC",
        "Architectural finished square column",
    );
    ca1.width = 400.0;
    ca1.depth = 400.0;
    ca1.is_structural = false;
    lib.push(ca1);

    let mut ca2 = TypePreset::base(
        ToolCategory::ColumnArch,
        "Round Feature Column Ø 450mm",
        "AC",
        "Smooth cylindrical feature colonnade column",
    );
    ca2.width = 450.0;
    ca2.depth = 450.0;
    ca2.is_round = true;
    ca2.is_structural = false;
    lib.push(ca2);

    // --- 5. Architectural Floors ---
    let mut fl1 = TypePreset::base(
        ToolCategory::FloorArch,
        "Timber Parquet on Screed & RC 250mm",
        "FL",
        "Acoustic floating timber floor assembly",
    );
    fl1.is_external = false;
    fl1.layers = vec![
        make_layer(LayerFunction::InteriorFinish, "Engineered Oak Parquet", 18.0, 0.16),
        make_layer(LayerFunction::Substrate, "Cement Leveling Screed", 52.0, 1.40),
        make_layer(LayerFunction::ThermalInsulation, "Acoustic Impact Mat", 20.0, 0.038),
        make_layer(LayerFunction::StructuralCore, "Reinforced Concrete Deck", 160.0, 2.30),
    ];
    lib.push(fl1);

    let mut fl2 = TypePreset::base(
        ToolCategory::FloorArch,
        "Porcelain Tile Wet-Room Floor 220mm",
        "FL",
        "Waterproofed ceramic/porcelain floor system",
    );
    fl2.is_external = false;
    fl2.layers = vec![
        make_layer(LayerFunction::InteriorFinish, "Porcelain Tile + Bedding", 20.0, 1.30),
        make_layer(LayerFunction::MembraneBarrier, "Liquid Waterproof Membrane", 0.0, 0.20),
        make_layer(LayerFunction::Substrate, "Sloped Mortar Screed", 50.0, 1.40),
        make_layer(LayerFunction::StructuralCore, "Reinforced Concrete Slab", 150.0, 2.30),
    ];
    lib.push(fl2);

    // --- 6. Ceilings ---
    let mut cl1 = TypePreset::base(
        ToolCategory::Ceiling,
        "Acoustic Mineral Grid 600 × 600mm",
        "CL",
        "Suspended T-bar acoustic ceiling at +2600mm",
    );
    cl1.base_offset = 2600.0;
    cl1.is_external = false;
    cl1.grid_spacing_x = 600.0;
    cl1.grid_spacing_y = 600.0;
    cl1.layers = vec![
        make_layer(LayerFunction::StructuralCore, "Galvanized Suspension Grid", 35.0, 50.0),
        make_layer(LayerFunction::InteriorFinish, "Mineral Fiber Acoustic Tile", 15.0, 0.058),
    ];
    lib.push(cl1);

    let mut cl2 = TypePreset::base(
        ToolCategory::Ceiling,
        "Flush Gypsum Bulkhead Ceiling",
        "CL",
        "Monolithic skimmed gypsum ceiling at +2700mm",
    );
    cl2.base_offset = 2700.0;
    cl2.is_external = false;
    cl2.grid_spacing_x = 1200.0;
    cl2.grid_spacing_y = 1200.0;
    cl2.layers = vec![
        make_layer(LayerFunction::StructuralCore, "Furring Channel Frame", 28.0, 50.0),
        make_layer(LayerFunction::InteriorFinish, "Gypsum Board 12.5mm", 12.5, 0.25),
    ];
    lib.push(cl2);

    // --- 7. Stairs ---
    let mut st1 = TypePreset::base(
        ToolCategory::Stair,
        "Monolithic Cast-in-Place Concrete Stair",
        "ST",
        "Reinforced concrete flight with smooth sloped soffit",
    );
    st1.stair_style = StairStyle::Monolithic;
    st1.width = 1200.0;
    st1.riser_count = 18;
    st1.waist_thickness = 160.0;
    st1.target_going = 280.0;
    st1.unconnected_height = 3000.0;
    lib.push(st1);

    let mut st2 = TypePreset::base(
        ToolCategory::Stair,
        "Open-Riser Architectural Timber/Steel Stair",
        "ST",
        "Floating hardwood treads on concealed stringers",
    );
    st2.stair_style = StairStyle::OpenRiser;
    st2.width = 1100.0;
    st2.riser_count = 17;
    st2.waist_thickness = 160.0;
    st2.target_going = 285.0;
    st2.unconnected_height = 3000.0;
    st2.is_external = false;
    lib.push(st2);

    let mut st3 = TypePreset::base(
        ToolCategory::Stair,
        "Monumental Solid Podium Steps",
        "ST",
        "Full-mass stepped entrance plinth stair",
    );
    st3.stair_style = StairStyle::SolidStepped;
    st3.width = 2400.0;
    st3.riser_count = 8;
    st3.target_going = 320.0;
    st3.unconnected_height = 1200.0;
    lib.push(st3);

    // --- 8. Ramps ---
    let mut rm1 = TypePreset::base(
        ToolCategory::Ramp,
        "Accessible Sloped Slab Ramp (1:12)",
        "RM",
        "Barrier-free inclined concrete slab",
    );
    rm1.ramp_style = RampStyle::SlopedSlab;
    rm1.width = 1500.0;
    rm1.thickness = 200.0;
    rm1.unconnected_height = 500.0;
    lib.push(rm1);

    let mut rm2 = TypePreset::base(
        ToolCategory::Ramp,
        "Solid Earth-Bearing Wedge Ramp",
        "RM",
        "Full-depth solid wedge vehicular/loading ramp",
    );
    rm2.ramp_style = RampStyle::SolidWedge;
    rm2.width = 3000.0;
    rm2.thickness = 250.0;
    rm2.unconnected_height = 900.0;
    lib.push(rm2);

    // --- 9. Railings ---
    let mut rl1 = TypePreset::base(
        ToolCategory::Railing,
        "Architectural Balustrade 1100mm (@100mm)",
        "RL",
        "Code-compliant guardrail with 100mm sphere safety",
    );
    rl1.unconnected_height = 1100.0;
    rl1.baluster_spacing = 100.0;
    rl1.mid_rail_count = 2;
    rl1.material_name = "Brushed Stainless Steel".into();
    lib.push(rl1);

    let mut rl2 = TypePreset::base(
        ToolCategory::Railing,
        "Industrial Pipe Handrail 1100mm",
        "RL",
        "3-rail galvanized safety guardrail",
    );
    rl2.unconnected_height = 1100.0;
    rl2.baluster_spacing = 900.0;
    rl2.mid_rail_count = 3;
    rl2.material_name = "Galvanized Steel".into();
    lib.push(rl2);

    // --- 10. Roofs ---
    let mut rf1 = TypePreset::base(
        ToolCategory::Roof,
        "Warm Pitched Hip Roof 30° (298mm)",
        "RF",
        "Insulated rafter hip roof with standing seam/tile finish",
    );
    rf1.roof_shape = RoofShape::Hip;
    rf1.slope_degrees = 30.0;
    rf1.overhang = 450.0;
    rf1.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Architectural Standing Seam / Tile", 25.0, 0.90),
        make_layer(LayerFunction::MembraneBarrier, "Breather Underlayment", 0.0, 0.20),
        make_layer(LayerFunction::Substrate, "OSB Roof Sheathing 18mm", 18.0, 0.13),
        make_layer(LayerFunction::ThermalInsulation, "Rafter Mineral Insulation", 240.0, 0.035),
        make_layer(LayerFunction::InteriorFinish, "Gypsum Ceiling Liner", 15.0, 0.25),
    ];
    lib.push(rf1);

    let mut rf2 = TypePreset::base(
        ToolCategory::Roof,
        "Gable Timber Truss Roof 35°",
        "RF",
        "Dual-pitched gable roof with generous 600mm eaves",
    );
    rf2.roof_shape = RoofShape::Gable;
    rf2.slope_degrees = 35.0;
    rf2.overhang = 600.0;
    rf2.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Clay Roof Tiles", 35.0, 0.85),
        make_layer(LayerFunction::Substrate, "Treated Battens & Plywood", 25.0, 0.14),
        make_layer(LayerFunction::ThermalInsulation, "Rigid Roof Insulation", 180.0, 0.024),
    ];
    lib.push(rf2);

    let mut rf3 = TypePreset::base(
        ToolCategory::Roof,
        "Inverted Warm Flat Roof 320mm",
        "RF",
        "EPDM/TPO flat concrete roof deck",
    );
    rf3.roof_shape = RoofShape::Flat;
    rf3.slope_degrees = 2.0;
    rf3.overhang = 150.0;
    rf3.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Ballast / Paver Finish", 40.0, 1.20),
        make_layer(LayerFunction::ThermalInsulation, "XPS Rigid Insulation", 100.0, 0.032),
        make_layer(LayerFunction::MembraneBarrier, "EPDM Waterproof Membrane", 0.0, 0.17),
        make_layer(LayerFunction::StructuralCore, "Reinforced Concrete Roof Slab", 180.0, 2.30),
    ];
    lib.push(rf3);

    // --- 11. Fascias ---
    let mut fs1 = TypePreset::base(
        ToolCategory::Fascia,
        "Painted Timber Eave Fascia 220 × 25mm",
        "FS",
        "Standard architectural eave trim board",
    );
    fs1.height = 220.0;
    fs1.thickness = 25.0;
    fs1.material_name = "Primed Cedar / uPVC".into();
    lib.push(fs1);

    // --- 12. Gutters ---
    let mut gt1 = TypePreset::base(
        ToolCategory::Gutter,
        "Box Eave Gutter 150 × 120mm",
        "GT",
        "Architectural powder-coated aluminum rainwater gutter",
    );
    gt1.width = 150.0;
    gt1.depth = 120.0;
    gt1.material_name = "Powder-Coated Aluminum".into();
    lib.push(gt1);

    // --- 13. Structural Columns ---
    let mut cs1 = TypePreset::base(
        ToolCategory::ColumnStruct,
        "RC Structural Column 450 × 450mm",
        "SC",
        "C40/50 reinforced concrete structural column",
    );
    cs1.width = 450.0;
    cs1.depth = 450.0;
    cs1.load_bearing_capacity = 2400.0;
    cs1.fire_rating = "R 120".into();
    lib.push(cs1);

    let mut cs2 = TypePreset::base(
        ToolCategory::ColumnStruct,
        "Heavy Core Column 600 × 600mm",
        "SC",
        "High-load multi-storey RC column",
    );
    cs2.width = 600.0;
    cs2.depth = 600.0;
    cs2.load_bearing_capacity = 4500.0;
    cs2.fire_rating = "R 120".into();
    lib.push(cs2);

    // --- 14. Structural Beams ---
    let mut bm1 = TypePreset::base(
        ToolCategory::Beam,
        "RC Primary Girder 300 × 600mm",
        "BM",
        "Reinforced concrete primary framing beam",
    );
    bm1.width = 300.0;
    bm1.depth = 600.0;
    bm1.load_bearing_capacity = 45.0;
    bm1.fire_rating = "R 90".into();
    lib.push(bm1);

    let mut bm2 = TypePreset::base(
        ToolCategory::Beam,
        "Steel Wide-Flange Girder 250 × 450mm",
        "SB",
        "Structural steel I-section framing beam",
    );
    bm2.width = 250.0;
    bm2.depth = 450.0;
    bm2.load_bearing_capacity = 60.0;
    bm2.fire_rating = "R 60".into();
    lib.push(bm2);

    // --- 15. Structural Walls ---
    let mut sw1 = TypePreset::base(
        ToolCategory::WallStruct,
        "RC Core / Shear Wall 250mm",
        "SW",
        "Reinforced concrete lateral stability core wall",
    );
    sw1.fire_rating = "REI 120".into();
    sw1.layers = vec![make_layer(
        LayerFunction::StructuralCore,
        "C40 Reinforced Concrete Core",
        250.0,
        2.30,
    )];
    lib.push(sw1);

    let mut sw2 = TypePreset::base(
        ToolCategory::WallStruct,
        "Basement Retaining Wall 350mm",
        "RW",
        "Waterproofed earth-retaining structural concrete wall",
    );
    sw2.fire_rating = "REI 120".into();
    sw2.layers = vec![
        make_layer(LayerFunction::ThermalInsulation, "Drainage Board + XPS", 50.0, 0.034),
        make_layer(LayerFunction::MembraneBarrier, "Tanked Bitumen Membrane", 0.0, 0.17),
        make_layer(LayerFunction::StructuralCore, "Waterproof RC Wall 300mm", 300.0, 2.30),
    ];
    lib.push(sw2);

    // --- 16. Structural Slabs ---
    let mut sl1 = TypePreset::base(
        ToolCategory::SlabStruct,
        "Two-Way Structural RC Slab 200mm",
        "SL",
        "Monolithic reinforced concrete structural deck",
    );
    sl1.fire_rating = "REI 90".into();
    sl1.load_bearing_capacity = 5.0;
    sl1.layers = vec![make_layer(
        LayerFunction::StructuralCore,
        "C35/45 Reinforced Concrete",
        200.0,
        2.30,
    )];
    lib.push(sl1);

    let mut sl2 = TypePreset::base(
        ToolCategory::SlabStruct,
        "Insulated Foundation Raft Slab 400mm",
        "FD",
        "Ground-bearing structural raft foundation with underslab XPS",
    );
    sl2.fire_rating = "REI 120".into();
    sl2.load_bearing_capacity = 15.0;
    sl2.layers = vec![
        make_layer(LayerFunction::StructuralCore, "RC Foundation Raft 300mm", 300.0, 2.30),
        make_layer(LayerFunction::MembraneBarrier, "Radon / Damp-Proof Membrane", 0.0, 0.20),
        make_layer(LayerFunction::ThermalInsulation, "High-Load Underslab XPS", 100.0, 0.034),
    ];
    lib.push(sl2);

    // --- 17. Structural Foundation Footings ---
    let mut ft1 = TypePreset::base(
        ToolCategory::Footing,
        "RC Isolated Pad Footing 1400 × 1400 × 450mm",
        "FT",
        "Standard reinforced concrete column pad foundation",
    );
    ft1.footing_style = FootingStyle::Pad;
    ft1.width = 1400.0;
    ft1.depth = 1400.0;
    ft1.thickness = 450.0;
    ft1.soil_bearing_capacity = 200.0;
    ft1.concrete_grade = "C30/37".into();
    ft1.load_bearing_capacity = 900.0;
    lib.push(ft1);

    let mut ft2 = TypePreset::base(
        ToolCategory::Footing,
        "Heavy Pad Footing 1800 × 1800 × 600mm",
        "FT",
        "High-capacity RC pad footing for multi-storey columns",
    );
    ft2.footing_style = FootingStyle::Pad;
    ft2.width = 1800.0;
    ft2.depth = 1800.0;
    ft2.thickness = 600.0;
    ft2.soil_bearing_capacity = 250.0;
    ft2.concrete_grade = "C35/45".into();
    ft2.load_bearing_capacity = 1800.0;
    lib.push(ft2);

    let mut ft3 = TypePreset::base(
        ToolCategory::Footing,
        "Two-Tier Stepped Footing 2000 × 2000 × 700mm",
        "FT",
        "Stepped pedestal & spread concrete foundation",
    );
    ft3.footing_style = FootingStyle::Stepped;
    ft3.width = 2000.0;
    ft3.depth = 2000.0;
    ft3.thickness = 700.0;
    ft3.soil_bearing_capacity = 220.0;
    ft3.concrete_grade = "C35/45".into();
    ft3.load_bearing_capacity = 2200.0;
    lib.push(ft3);

    let mut ft4 = TypePreset::base(
        ToolCategory::Footing,
        "Sloped Bell Footing 1600 × 1600 × 550mm",
        "FT",
        "Material-efficient sloped frustum concrete footing",
    );
    ft4.footing_style = FootingStyle::Sloped;
    ft4.width = 1600.0;
    ft4.depth = 1600.0;
    ft4.thickness = 550.0;
    ft4.soil_bearing_capacity = 200.0;
    ft4.concrete_grade = "C30/37".into();
    ft4.load_bearing_capacity = 1350.0;
    lib.push(ft4);

    // --- 18. Linear Dimensions ---
    let mut d1 = TypePreset::base(
        ToolCategory::DimensionLinear,
        "Architectural Tick 45° (2.5mm)",
        "DIM",
        "Standard architectural linear dimension string with 45° ticks",
    );
    d1.tick_mark = plinth_model::TickMark::Tick45;
    d1.font_size = 2.5;
    d1.dim_offset = 600.0;
    d1.continuous_string = true;
    lib.push(d1);

    let mut d2 = TypePreset::base(
        ToolCategory::DimensionLinear,
        "Engineering Arrow (2.5mm)",
        "DIM",
        "Mechanical and civil dimension line with closed arrowheads",
    );
    d2.tick_mark = plinth_model::TickMark::Arrow;
    d2.font_size = 2.5;
    d2.dim_offset = 600.0;
    d2.continuous_string = true;
    lib.push(d2);

    let mut d3 = TypePreset::base(
        ToolCategory::DimensionLinear,
        "Metric Heavy (3.5mm)",
        "DIM",
        "High-visibility bold dimension line for large scale plans",
    );
    d3.tick_mark = plinth_model::TickMark::Tick45;
    d3.font_size = 3.5;
    d3.font_style = plinth_model::FontStyle::Bold;
    d3.dim_offset = 800.0;
    d3.continuous_string = true;
    lib.push(d3);

    // --- 19. Circular / Radial Dimensions ---
    let mut dc1 = TypePreset::base(
        ToolCategory::DimensionCircular,
        "Radial Dimension (2.5mm)",
        "RAD",
        "Radius callout with R prefix and arrowhead",
    );
    dc1.tick_mark = plinth_model::TickMark::Arrow;
    dc1.font_size = 2.5;
    dc1.is_diameter = false;
    dc1.prefix = Some("R ".to_string());
    lib.push(dc1);

    let mut dc2 = TypePreset::base(
        ToolCategory::DimensionCircular,
        "Diameter Dimension (Ø 2.5mm)",
        "DIA",
        "Diameter callout with Ø symbol",
    );
    dc2.tick_mark = plinth_model::TickMark::Arrow;
    dc2.font_size = 2.5;
    dc2.is_diameter = true;
    dc2.prefix = Some("Ø ".to_string());
    lib.push(dc2);

    // --- 20. Angular Dimensions ---
    let mut da1 = TypePreset::base(
        ToolCategory::DimensionAngular,
        "Angular Dimension (2.5mm)",
        "ANG",
        "Degrees angle dimension arc with dual arrowheads",
    );
    da1.tick_mark = plinth_model::TickMark::Arrow;
    da1.font_size = 2.5;
    da1.dim_offset = 500.0;
    lib.push(da1);

    // --- 21. Text Notes ---
    let mut tn1 = TypePreset::base(
        ToolCategory::TextNote,
        "Standard Note (2.5mm Regular)",
        "TX",
        "General drawing text annotation (Arial / Segoe UI 2.5mm)",
    );
    tn1.font_size = 2.5;
    tn1.font_style = plinth_model::FontStyle::Regular;
    lib.push(tn1);

    let mut tn2 = TypePreset::base(
        ToolCategory::TextNote,
        "Room Title (3.5mm Bold)",
        "TX",
        "Room and space name label typography (3.5mm Bold)",
    );
    tn2.font_size = 3.5;
    tn2.font_style = plinth_model::FontStyle::Bold;
    lib.push(tn2);

    let mut tn3 = TypePreset::base(
        ToolCategory::TextNote,
        "Drawing Header (5.0mm Bold)",
        "TX",
        "Large sheet and view drawing title (5.0mm Bold)",
    );
    tn3.font_size = 5.0;
    tn3.font_style = plinth_model::FontStyle::Bold;
    lib.push(tn3);

    // --- 22. Leaders ---
    let mut ld1 = TypePreset::base(
        ToolCategory::Leader,
        "Arrowhead Leader (2.5mm)",
        "LD",
        "Standard pointer leader with filled arrow and horizontal landing shelf",
    );
    ld1.tick_mark = plinth_model::TickMark::Arrow;
    ld1.font_size = 2.5;
    lib.push(ld1);

    let mut ld2 = TypePreset::base(
        ToolCategory::Leader,
        "Architectural Keynote Leader (Tick 45°)",
        "LD",
        "Specification callout leader with 45° tick anchor",
    );
    ld2.tick_mark = plinth_model::TickMark::Tick45;
    ld2.font_size = 2.5;
    lib.push(ld2);

    // --- 23. Detail Drafting Lines ---
    let mut dl1 = TypePreset::base(
        ToolCategory::DetailLine,
        "Thin Drafting Line (0.18mm)",
        "LN",
        "Fine drafting line for hatching and guide lines",
    );
    dl1.line_weight = 0.18;
    dl1.line_style = "Thin Line (0.18mm)".to_string();
    lib.push(dl1);

    let mut dl2 = TypePreset::base(
        ToolCategory::DetailLine,
        "Medium Object Line (0.35mm)",
        "LN",
        "Standard 2D architectural detail and drafting line",
    );
    dl2.line_weight = 0.35;
    dl2.line_style = "Medium Object (0.35mm)".to_string();
    lib.push(dl2);

    let mut dl3 = TypePreset::base(
        ToolCategory::DetailLine,
        "Wide Contour Line (0.70mm)",
        "LN",
        "Heavy boundary contour and section cut boundary line",
    );
    dl3.line_weight = 0.70;
    dl3.line_style = "Wide Contour (0.70mm)".to_string();
    lib.push(dl3);

    let mut dl4 = TypePreset::base(
        ToolCategory::DetailLine,
        "Hidden Dashed Line (0.25mm)",
        "LN",
        "Dashed drafting line for concealed or overhead elements",
    );
    dl4.line_weight = 0.25;
    dl4.line_style = "Hidden Dashed (0.25mm)".to_string();
    lib.push(dl4);

    // --- 24. Roads ---
    let mut rd1 = TypePreset::base(
        ToolCategory::Road,
        "Two-Lane Arterial Road (7.3m)",
        "RD",
        "Standard municipal two-lane asphalt roadway with barrier curbs",
    );
    rd1.width = 7300.0;
    rd1.lanes = 2;
    rd1.lane_width = 3650.0;
    rd1.thickness = 275.0;
    rd1.road_curb = plinth_model::RoadCurb::Barrier;
    rd1.curb_width = 150.0;
    rd1.curb_height = 150.0;
    rd1.crown_slope_pct = 2.0;
    rd1.has_centerline = true;
    rd1.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Asphalt Wearing Course", 50.0, 0.75),
        make_layer(LayerFunction::StructuralCore, "Dense Bituminous Binder Course", 75.0, 0.85),
        make_layer(LayerFunction::Substrate, "Crushed Stone Aggregate Subbase", 150.0, 1.40),
    ];
    lib.push(rd1);

    let mut rd2 = TypePreset::base(
        ToolCategory::Road,
        "Four-Lane Divided Boulevard (14.6m)",
        "RD",
        "Dual-carriageway 4-lane major arterial road with barrier curbs",
    );
    rd2.width = 14600.0;
    rd2.lanes = 4;
    rd2.lane_width = 3650.0;
    rd2.thickness = 325.0;
    rd2.road_curb = plinth_model::RoadCurb::Barrier;
    rd2.curb_width = 150.0;
    rd2.curb_height = 150.0;
    rd2.crown_slope_pct = 2.0;
    rd2.has_centerline = true;
    rd2.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Stone Mastic Asphalt Wearing Course", 50.0, 0.75),
        make_layer(LayerFunction::StructuralCore, "Heavy Duty Asphalt Base Course", 100.0, 0.85),
        make_layer(LayerFunction::Substrate, "Cement Stabilized Subbase", 175.0, 1.30),
    ];
    lib.push(rd2);

    let mut rd3 = TypePreset::base(
        ToolCategory::Road,
        "Single-Lane Access Road (4.0m)",
        "RD",
        "Residential single-lane access street with mountable roll curbs",
    );
    rd3.width = 4000.0;
    rd3.lanes = 1;
    rd3.lane_width = 4000.0;
    rd3.thickness = 200.0;
    rd3.road_curb = plinth_model::RoadCurb::Mountable;
    rd3.curb_width = 120.0;
    rd3.curb_height = 100.0;
    rd3.crown_slope_pct = 1.5;
    rd3.has_centerline = false;
    rd3.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Asphalt Surface", 50.0, 0.75),
        make_layer(LayerFunction::Substrate, "Compacted Gravel Base", 150.0, 1.40),
    ];
    lib.push(rd3);

    // --- 25. Sidewalks ---
    let mut sw1 = TypePreset::base(
        ToolCategory::Sidewalk,
        "Concrete Walkway with Bullnose Curb (1.8m)",
        "SW",
        "Pedestrian sidewalk with 25mm rounded bullnose curb edge and contraction joints",
    );
    sw1.width = 1800.0;
    sw1.thickness = 150.0;
    sw1.curb_height = 150.0;
    sw1.sidewalk_edge = plinth_model::SidewalkEdgeProfile::Rounded { radius: 25.0 };
    sw1.has_top_groove = true;
    sw1.groove_spacing = 1500.0;
    sw1.groove_depth = 10.0;
    sw1.layers = vec![
        make_layer(LayerFunction::StructuralCore, "Cast-in-Place Concrete Slab", 100.0, 1.30),
        make_layer(LayerFunction::Substrate, "Compacted Sand Bed", 50.0, 1.00),
    ];
    lib.push(sw1);

    let mut sw2 = TypePreset::base(
        ToolCategory::Sidewalk,
        "Commercial Wide Sidewalk with Square Curb (3.0m)",
        "SW",
        "Wide commercial concrete sidewalk with barrier square curb and top grooves",
    );
    sw2.width = 3000.0;
    sw2.thickness = 175.0;
    sw2.curb_height = 150.0;
    sw2.sidewalk_edge = plinth_model::SidewalkEdgeProfile::Square;
    sw2.has_top_groove = true;
    sw2.groove_spacing = 1500.0;
    sw2.groove_depth = 12.0;
    sw2.layers = vec![
        make_layer(LayerFunction::StructuralCore, "Reinforced Concrete Pavement", 125.0, 1.30),
        make_layer(LayerFunction::Substrate, "Crushed Gravel Bedding", 50.0, 1.40),
    ];
    lib.push(sw2);

    let mut sw3 = TypePreset::base(
        ToolCategory::Sidewalk,
        "Flush Garden Walkway with Chamfer Edge (1.2m)",
        "SW",
        "Flush-level landscape pathway with 15mm chamfered bevel edge and contraction grooves",
    );
    sw3.width = 1200.0;
    sw3.thickness = 120.0;
    sw3.curb_height = 0.0;
    sw3.sidewalk_edge = plinth_model::SidewalkEdgeProfile::Beveled { chamfer: 15.0 };
    sw3.has_top_groove = true;
    sw3.groove_spacing = 1200.0;
    sw3.groove_depth = 8.0;
    sw3.layers = vec![
        make_layer(LayerFunction::StructuralCore, "Exposed Aggregate Concrete", 80.0, 1.20),
        make_layer(LayerFunction::Substrate, "Sand Subbase", 40.0, 1.00),
    ];
    lib.push(sw3);

    // --- 26. Pathwalks ---
    let mut pw1 = TypePreset::base(
        ToolCategory::Pathwalk,
        "Interlocking Paver Block Pathwalk (1.5m)",
        "PW",
        "Decorative interlocking concrete paver blocks with sand joints and border edging",
    );
    pw1.width = 1500.0;
    pw1.thickness = 100.0;
    pw1.pathwalk_profile = plinth_model::PathwalkProfile::PaverBlocks;
    pw1.has_edging = true;
    pw1.edging_width = 100.0;
    pw1.groove_width = 8.0;
    pw1.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Interlocking Paver Blocks", 60.0, 1.10),
        make_layer(LayerFunction::Substrate, "Coarse Sand Bedding", 40.0, 1.00),
    ];
    lib.push(pw1);

    let mut pw2 = TypePreset::base(
        ToolCategory::Pathwalk,
        "Garden Cobblestone Promenade (2.0m)",
        "PW",
        "Rustic natural cobblestone pathway with mortar joints and granite border curbs",
    );
    pw2.width = 2000.0;
    pw2.thickness = 140.0;
    pw2.pathwalk_profile = plinth_model::PathwalkProfile::Cobblestone;
    pw2.has_edging = true;
    pw2.edging_width = 150.0;
    pw2.groove_width = 12.0;
    pw2.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Natural Cobblestone Setts", 90.0, 1.60),
        make_layer(LayerFunction::Substrate, "Mortar Bedding & Aggregate", 50.0, 1.20),
    ];
    lib.push(pw2);

    let mut pw3 = TypePreset::base(
        ToolCategory::Pathwalk,
        "Nature Trail Timber Boardwalk (1.2m)",
        "PW",
        "Elevated eco-timber boardwalk planks with non-slip grooved decking",
    );
    pw3.width = 1200.0;
    pw3.thickness = 100.0;
    pw3.pathwalk_profile = plinth_model::PathwalkProfile::TimberBoardwalk;
    pw3.has_edging = false;
    pw3.edging_width = 0.0;
    pw3.groove_width = 6.0;
    pw3.layers = vec![
        make_layer(LayerFunction::ExteriorFinish, "Treated Hardwood Decking Planks", 40.0, 0.13),
        make_layer(LayerFunction::StructuralCore, "Timber Support Joists", 60.0, 0.13),
    ];
    lib.push(pw3);

    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_studio_library_covers_all_18_categories_and_duplicates() {
        let mut studio = ToolConfigStudio::new();
        for cat in ToolCategory::ALL_ARCH
            .iter()
            .chain(ToolCategory::ALL_STRUCT.iter())
            .chain(ToolCategory::ALL_ANNOTATION.iter())
            .chain(ToolCategory::ALL_SITE.iter())
        {
            assert!(
                studio.active_preset(*cat).is_some(),
                "Missing default preset for {:?}",
                cat
            );
        }

        studio.open_for(ToolCategory::WallArch);
        let orig_count = studio.presets.len();
        let dup_idx = studio.duplicate_active_type().expect("should duplicate");
        assert_eq!(studio.presets.len(), orig_count + 1);
        assert!(!studio.presets[dup_idx].is_builtin);
        assert!(studio.presets[dup_idx].thickness > 100.0);
    }

    #[test]
    fn test_individual_object_settings_per_item() {
        let mut studio = ToolConfigStudio::new();

        // 1. All 18 items have distinct icons, discipline tags, and IFC Pset names
        for cat in ToolCategory::ALL_ARCH
            .iter()
            .chain(ToolCategory::ALL_STRUCT.iter())
            .chain(ToolCategory::ALL_ANNOTATION.iter())
            .chain(ToolCategory::ALL_SITE.iter())
        {
            assert!(!cat.title().is_empty());
            assert!(!cat.short_label().is_empty());
            assert!(!cat.discipline_label().is_empty());
            assert!(cat.pset_name().starts_with("Pset_"));
        }

        // 2. Open individual items: WallArch, CurtainWall, and ColumnStruct
        studio.open_for(ToolCategory::WallArch);
        assert!(studio.is_category_open(ToolCategory::WallArch));
        assert_eq!(studio.open_categories.len(), 1);
        assert!(studio.is_open);

        studio.open_for(ToolCategory::CurtainWall);
        assert!(studio.is_category_open(ToolCategory::WallArch));
        assert!(studio.is_category_open(ToolCategory::CurtainWall));
        assert_eq!(studio.open_categories.len(), 2);

        studio.open_for(ToolCategory::ColumnStruct);
        assert_eq!(studio.open_categories.len(), 3);

        // 3. Category-specific duplication and creation
        let count_before = studio.presets.len();
        let new_cw_idx = studio.duplicate_type_for(ToolCategory::CurtainWall).unwrap();
        assert_eq!(studio.presets.len(), count_before + 1);
        assert_eq!(studio.presets[new_cw_idx].category, ToolCategory::CurtainWall);

        let new_beam_idx = studio.create_blank_type_for(ToolCategory::Beam);
        assert_eq!(studio.presets.len(), count_before + 2);
        assert_eq!(studio.presets[new_beam_idx].category, ToolCategory::Beam);

        // 4. Closing an individual item only closes that item
        studio.close_category(ToolCategory::WallArch);
        assert!(!studio.is_category_open(ToolCategory::WallArch));
        assert!(studio.is_category_open(ToolCategory::CurtainWall));
        assert!(studio.is_category_open(ToolCategory::ColumnStruct));
        assert_eq!(studio.open_categories.len(), 2);
        assert!(studio.is_open);

        // 5. Render individual windows via egui
        let mut doc = Document::create_sample_building();
        let theme = AppTheme::dark();
        let ctx = Context::default();

        let _ = ctx.run(Default::default(), |ctx| {
            let outcome = render_tool_config_studio(ctx, &mut studio, &mut doc, None, &theme);
            assert!(outcome.activated_tool.is_none());
        });

        // 6. Test Apply to Items
        let w_id = doc
            .elements
            .iter()
            .find_map(|(id, el)| {
                if matches!(el, Element::Wall(_)) {
                    Some(*id)
                } else {
                    None
                }
            })
            .expect("sample building should have walls");

        studio.open_for(ToolCategory::WallArch);
        let _ = ctx.run(Default::default(), |ctx| {
            let outcome = render_tool_config_studio(ctx, &mut studio, &mut doc, Some(w_id), &theme);
            assert!(outcome.activated_tool.is_none());
        });

        let p_wall = studio.active_preset(ToolCategory::WallArch).unwrap().clone();
        let lvls = ToolConfigStudio::level_pairs(&doc);
        if let Some(el) = doc.elements.get_mut(&w_id) {
            assert!(apply_preset_to_element(&p_wall, &lvls, el));
            if let Element::Wall(w) = el {
                assert_eq!(w.thickness, p_wall.thickness);
            }
        }

        // Close remaining items
        studio.close_category(ToolCategory::WallArch);
        studio.close_category(ToolCategory::CurtainWall);
        studio.close_category(ToolCategory::ColumnStruct);
        assert!(studio.open_categories.is_empty());
        assert!(!studio.is_open);
    }
}
