//! Intermediate 2D vector primitives produced by the projection engine.

use plinth_geom::{Aabb2, Vec2};
use serde::{Deserialize, Serialize};

/// Visual display style for 3D and 2D views (inspired by Revit standards).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VisualStyle {
    /// Directional sunlight shading with crisp boundary edge outlines.
    #[default]
    Shaded,
    /// Clean monochrome architectural style with opaque solid fills and crisp dark outlines.
    HiddenLine,
    /// Uniform material tint colors without directional sunlight variation.
    ConsistentColors,
    /// Transparent fills showing internal structural framing and wireframe outlines.
    Wireframe,
}

impl VisualStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::Shaded => "Shaded",
            Self::HiddenLine => "Hidden Line",
            Self::ConsistentColors => "Consistent Colors",
            Self::Wireframe => "Wireframe",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Shaded => "☀️",
            Self::HiddenLine => "📐",
            Self::ConsistentColors => "🎨",
            Self::Wireframe => "🕸️",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderPrimitive {
    Line {
        start: Vec2,
        end: Vec2,
        color: [u8; 4],
        width: f32,
        dashed: bool,
    },
    Polygon {
        points: Vec<Vec2>,
        fill_color: [u8; 4],
        stroke_color: Option<[u8; 4]>,
        stroke_width: f32,
    },
    Arc {
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: [u8; 4],
        width: f32,
    },
    Circle {
        center: Vec2,
        radius: f32,
        fill_color: Option<[u8; 4]>,
        stroke_color: [u8; 4],
        stroke_width: f32,
    },
    Text {
        pos: Vec2,
        text: String,
        color: [u8; 4],
        size: f32,
        align_center: bool,
    },
}

#[derive(Clone, Debug, Default)]
pub struct RenderScene {
    pub primitives: Vec<RenderPrimitive>,
    pub bounds: Aabb2,
}

impl RenderScene {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            bounds: Aabb2::EMPTY,
        }
    }

    pub fn add(&mut self, prim: RenderPrimitive) {
        match &prim {
            RenderPrimitive::Line { start, end, .. } => {
                self.bounds.include(*start);
                self.bounds.include(*end);
            }
            RenderPrimitive::Polygon { points, .. } => {
                for p in points {
                    self.bounds.include(*p);
                }
            }
            RenderPrimitive::Circle { center, radius, .. } | RenderPrimitive::Arc { center, radius, .. } => {
                let r = *radius as f64;
                self.bounds.include(*center - Vec2::new(r, r));
                self.bounds.include(*center + Vec2::new(r, r));
            }
            RenderPrimitive::Text { pos, .. } => {
                self.bounds.include(*pos);
            }
        }
        self.primitives.push(prim);
    }
}

/// Arch3D signature pastel palette for architectural materials.
/// Clean, distinct, professional aesthetic designed for modern BIM authoring.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialPalette {
    pub wall: [u8; 3],          // Linen Birch Pastel [238, 234, 226]
    pub slab: [u8; 3],          // Pale Sand Limestone [224, 226, 220]
    pub roof: [u8; 3],          // Dusty Terracotta Rose [214, 182, 168]
    pub ceiling: [u8; 3],       // Soft Chalk White [248, 248, 250]
    pub column: [u8; 3],        // Muted Slate Sage [198, 206, 208]
    pub beam: [u8; 3],          // Warm Mineral Grey [206, 210, 214]
    pub door: [u8; 3],          // Bleached Honey Wood [212, 184, 150]
    pub window_glass: [u8; 4],  // Glacier Cyan Translucent [175, 215, 225, 140]
    pub window_frame: [u8; 3],  // Warm Anthracite Slate [85, 92, 102]
    pub stair_ramp: [u8; 3],    // Travertine Oatmeal [228, 224, 216]
    pub railing: [u8; 3],       // Titanium Graphite [108, 116, 128]
    pub curtain_frame: [u8; 3], // Matte Aluminum [95, 105, 118]
    pub curtain_glass: [u8; 4], // Pale Sky Cyan [168, 212, 224, 130]
    pub footing: [u8; 3],       // Basalt Sand [192, 190, 185]
    pub site: [u8; 3],          // Matcha Pastel [195, 208, 185]
    pub freeform: [u8; 3],      // Signature Pastel Ivory [232, 228, 220]
}

impl MaterialPalette {
    pub fn signature_pastel() -> Self {
        Self {
            wall: [238, 234, 226],
            slab: [224, 226, 220],
            roof: [214, 182, 168],
            ceiling: [248, 248, 250],
            column: [198, 206, 208],
            beam: [206, 210, 214],
            door: [212, 184, 150],
            window_glass: [175, 215, 225, 140],
            window_frame: [85, 92, 102],
            stair_ramp: [228, 224, 216],
            railing: [108, 116, 128],
            curtain_frame: [95, 105, 118],
            curtain_glass: [168, 212, 224, 130],
            footing: [192, 190, 185],
            site: [195, 208, 185],
            freeform: [232, 228, 220],
        }
    }

    pub fn architectural_neutral() -> Self {
        Self {
            wall: [230, 230, 232],
            slab: [215, 216, 220],
            roof: [180, 185, 195],
            ceiling: [245, 245, 248],
            column: [190, 195, 202],
            beam: [185, 190, 198],
            door: [195, 170, 145],
            window_glass: [160, 205, 225, 140],
            window_frame: [70, 75, 85],
            stair_ramp: [215, 218, 222],
            railing: [100, 105, 115],
            curtain_frame: [80, 85, 95],
            curtain_glass: [160, 205, 225, 130],
            footing: [175, 178, 182],
            site: [185, 195, 180],
            freeform: [225, 225, 228],
        }
    }

    pub fn scandinavian_earth() -> Self {
        Self {
            wall: [242, 238, 230],
            slab: [218, 214, 204],
            roof: [188, 155, 138],
            ceiling: [250, 248, 244],
            column: [195, 198, 192],
            beam: [190, 192, 186],
            door: [205, 165, 125],
            window_glass: [170, 210, 215, 140],
            window_frame: [75, 78, 72],
            stair_ramp: [225, 220, 210],
            railing: [95, 100, 95],
            curtain_frame: [85, 88, 85],
            curtain_glass: [165, 205, 210, 130],
            footing: [180, 178, 170],
            site: [188, 200, 175],
            freeform: [235, 230, 220],
        }
    }

    pub fn modern_blueprint() -> Self {
        Self {
            wall: [220, 228, 240],
            slab: [205, 215, 230],
            roof: [165, 185, 215],
            ceiling: [240, 245, 252],
            column: [180, 195, 215],
            beam: [175, 190, 210],
            door: [200, 185, 170],
            window_glass: [150, 200, 240, 140],
            window_frame: [60, 75, 100],
            stair_ramp: [210, 220, 235],
            railing: [90, 105, 130],
            curtain_frame: [70, 85, 110],
            curtain_glass: [150, 195, 235, 130],
            footing: [170, 180, 195],
            site: [180, 205, 195],
            freeform: [215, 225, 238],
        }
    }
}

impl Default for MaterialPalette {
    fn default() -> Self {
        Self::signature_pastel()
    }
}

/// Theme palette for architectural line weights, fills, and materials.
#[derive(Clone, Copy, Debug)]
pub struct ColorTheme {
    pub background: [u8; 4],
    pub cut_line: [u8; 4],
    pub proj_line: [u8; 4],
    pub hidden_line: [u8; 4],
    pub wall_fill: [u8; 4],
    pub col_fill: [u8; 4],
    pub slab_fill: [u8; 4],
    pub glass_stroke: [u8; 4],
    pub glass_fill: [u8; 4],
    pub grid_line: [u8; 4],
    pub text_color: [u8; 4],
    pub level_line: [u8; 4],
    pub dim_line: [u8; 4],
    pub materials: MaterialPalette,
}

impl ColorTheme {
    pub fn dark() -> Self {
        Self {
            background: [26, 28, 32, 255],
            cut_line: [240, 240, 245, 255],
            proj_line: [170, 175, 185, 255],
            hidden_line: [120, 125, 135, 200],
            wall_fill: [60, 64, 72, 255],
            col_fill: [75, 80, 90, 255],
            slab_fill: [40, 44, 52, 255],
            glass_stroke: [100, 180, 240, 255],
            glass_fill: [80, 160, 220, 80],
            grid_line: [110, 120, 140, 255],
            text_color: [225, 230, 240, 255],
            level_line: [90, 150, 220, 255],
            dim_line: [130, 170, 220, 255],
            materials: MaterialPalette::signature_pastel(),
        }
    }

    pub fn light() -> Self {
        Self {
            background: [255, 255, 255, 255],
            cut_line: [20, 20, 25, 255],
            proj_line: [70, 75, 85, 255],
            hidden_line: [140, 145, 155, 220],
            wall_fill: [215, 218, 222, 255],
            col_fill: [180, 185, 195, 255],
            slab_fill: [242, 244, 247, 255],
            glass_stroke: [30, 110, 180, 255],
            glass_fill: [160, 210, 255, 100],
            grid_line: [100, 110, 130, 255],
            text_color: [25, 30, 40, 255],
            level_line: [30, 90, 170, 255],
            dim_line: [40, 80, 150, 255],
            materials: MaterialPalette::signature_pastel(),
        }
    }
}
