//! Physical and visual materials.

use crate::id::MaterialId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialCategory {
    Concrete,
    Steel,
    Timber,
    Masonry,
    Glass,
    Drywall,
    Insulation,
    Generic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub category: MaterialCategory,
    /// RGBA colour normalized to [0.0 .. 1.0].
    pub color: [f32; 4],
    /// Cut fill pattern density or roughness.
    pub roughness: f32,
    /// Transparency for glass (0.0 opaque, 1.0 clear).
    pub transparency: f32,
    /// Embedded texture filename or path, if textured.
    #[serde(default)]
    pub texture_name: Option<String>,
    /// Embedded raw texture image bytes (PNG or JPEG), if available.
    #[serde(default)]
    pub texture_bytes: Option<Vec<u8>>,
}

impl Material {
    pub fn new(name: impl Into<String>, category: MaterialCategory, color: [f32; 4]) -> Self {
        Self {
            id: MaterialId::new(),
            name: name.into(),
            category,
            color,
            roughness: 0.5,
            transparency: if category == MaterialCategory::Glass { 0.6 } else { 0.0 },
            texture_name: None,
            texture_bytes: None,
        }
    }

    pub fn default_concrete() -> Self {
        Self::new("Cast-in-Place Concrete", MaterialCategory::Concrete, [0.72, 0.72, 0.72, 1.0])
    }

    pub fn default_masonry() -> Self {
        Self::new("Masonry Brick", MaterialCategory::Masonry, [0.76, 0.42, 0.32, 1.0])
    }

    pub fn default_steel() -> Self {
        Self::new("Structural Steel S355", MaterialCategory::Steel, [0.35, 0.42, 0.52, 1.0])
    }

    pub fn default_glass() -> Self {
        Self {
            id: MaterialId::new(),
            name: "Glazing Clear".into(),
            category: MaterialCategory::Glass,
            color: [0.65, 0.85, 0.95, 0.4],
            roughness: 0.1,
            transparency: 0.7,
            texture_name: None,
            texture_bytes: None,
        }
    }

    pub fn default_drywall() -> Self {
        Self::new("Gypsum Wall Board", MaterialCategory::Drywall, [0.92, 0.92, 0.90, 1.0])
    }

    pub fn default_timber() -> Self {
        Self::new("Timber Softwood", MaterialCategory::Timber, [0.82, 0.68, 0.48, 1.0])
    }
}
