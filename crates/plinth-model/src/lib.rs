//! # plinth-model
//!
//! Single-source-of-truth BIM data model.
//!
//! Elements are defined with real-world dimensions in millimetres (1:1 standard scale).
//! Dynamic view cuts (plan, elevation, section, ceiling, 3D) project directly from
//! this unified model.

pub mod document;
pub mod dwg;
pub mod element;
pub mod grid;
pub mod id;
pub mod level;
pub mod material;
pub mod skp;
pub mod view_def;

pub use document::{Document, PaperSize, PlacedViewport, ProjectInfo, Sheet};
pub use dwg::{import_dwg_bytes, import_dwg_file, DwgImportSummary};
pub use element::{
    AssemblyLayer, Beam, BearingQuadrant, BuildingPad, Ceiling, Column, ContourLine, CurtainWall, DetailLine, Dimension, DimensionKind, Door, Element, Fascia, Footing,
    FootingStyle, FontStyle, FreeformSolid, FreeformSurface, Gutter, LayerFunction, Leader, Pathwalk, PathwalkProfile, PrimitiveKind, PropertyLine, PropertyLineCreationMode, Railing,
    Ramp, RampStyle, Road, RoadCurb, Roof, RoofShape, Sidewalk, SidewalkEdgeProfile, Slab, Stair, StairStyle, SurveySegment, TextNote, TickMark, Toposolid, ToposolidPoint, Wall, WallOpening, Window,
};
pub use grid::GridLine;
pub use id::{ElementId, GridId, LevelId, MaterialId, SheetId, ViewId, ViewportId};
pub use level::Level;
pub use material::{Material, MaterialCategory};
pub use skp::{import_skp_bytes, import_skp_file, SkpImportSummary};
pub use view_def::{Camera3D, ViewDef, ViewKind};
