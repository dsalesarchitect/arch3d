//! Building storeys / vertical datum levels.

use crate::id::LevelId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub id: LevelId,
    pub name: String,
    /// Elevation in world coordinates along the Z-axis in millimetres (1:1 scale).
    pub elevation: f64,
    /// Default story height in millimetres (e.g. 3000.0).
    pub story_height: f64,
    /// Whether this level designates a building storey in IFC export.
    pub is_building_storey: bool,
}

impl Level {
    pub fn new(name: impl Into<String>, elevation: f64, story_height: f64) -> Self {
        Self {
            id: LevelId::new(),
            name: name.into(),
            elevation,
            story_height,
            is_building_storey: true,
        }
    }
}
