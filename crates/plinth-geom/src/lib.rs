//! # plinth-geom
//!
//! Small, dependency-light geometry kernel used by every other Plinth crate.
//!
//! Conventions used throughout Plinth:
//! * Units are **millimetres**. The model is always stored at real-world size (1:1).
//! * World axes: +X east, +Y north, +Z up (right-handed).
//! * 2D polygons are counter-clockwise (CCW) unless stated otherwise.
//! * Every solid in the simplified Plinth model is a vertical [`Prism`]
//!   (a 2D profile extruded along Z). This keeps cutting, sectioning and
//!   IFC export exact and simple.

pub mod bounds;
pub mod clip;
pub mod polygon;
pub mod prism;
pub mod vec;
pub mod xform;

pub use bounds::{Aabb2, Aabb3};
pub use prism::{newell_normal, Face, Prism};
pub use vec::{vec2, vec3, Vec2, Vec3};
pub use xform::Xform2;

/// Numeric tolerance (millimetres) for floating point comparisons.
pub const EPS: f64 = 1e-6;

/// Tolerance (millimetres) for "these two points are the same" in modelling operations.
pub const POINT_TOL: f64 = 1.0;
