//! Architectural and structural datum grid lines.

use crate::id::GridId;
use plinth_geom::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridLine {
    pub id: GridId,
    /// Grid bubble tag, e.g. "1", "2", "A", "B".
    pub name: String,
    /// Start point in plan (world mm).
    pub start: Vec2,
    /// End point in plan (world mm).
    pub end: Vec2,
    /// Bubble display radius in canvas points or mm.
    pub bubble_radius: f64,
}

impl GridLine {
    pub fn new(name: impl Into<String>, start: Vec2, end: Vec2) -> Self {
        Self {
            id: GridId::new(),
            name: name.into(),
            start,
            end,
            bubble_radius: 500.0,
        }
    }
}
