//! Modeling and Modify tool state machine.

use plinth_geom::Vec2;
use plinth_model::ElementId;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    DrawWall { start: Option<Vec2>, is_structural: bool },
    DrawCurtainWall { start: Option<Vec2> },
    DrawColumn { is_structural: bool },
    DrawFooting,
    DrawBeam { start: Option<Vec2> },
    DrawSlab { points: Vec<Vec2>, is_structural: bool },
    DrawRoof { shape: plinth_model::RoofShape, points: Vec<Vec2> },
    DrawFascia { points: Vec<Vec2> },
    DrawGutter { points: Vec<Vec2> },
    DrawCeiling { points: Vec<Vec2> },
    DrawStair { start: Option<Vec2> },
    DrawRamp { start: Option<Vec2> },
    DrawRailing { points: Vec<Vec2> },
    PlaceDoor,
    PlaceWindow,
    DrawGrid { start: Option<Vec2> },
    DrawDimension {
        start: Option<Vec2>,
        p2: Option<Vec2>,
        locked_offset: Option<f64>,
    },
    DrawCircularDimension { center: Option<Vec2> },
    DrawAngularDimension { apex: Option<Vec2>, first_ray: Option<Vec2> },
    DrawTextAnnotation { pos: Option<Vec2> },
    DrawLeader { target_pt: Option<Vec2>, shoulder_pt: Option<Vec2> },
    DrawDetailLine { start: Option<Vec2> },
    Move { from: Option<Vec2> },
    Rotate { center: Option<Vec2>, start_angle: Option<f64> },
    Mirror { axis_p1: Option<Vec2> },
    // Site & Cadastral Tools
    DrawBuildingPad { points: Vec<Vec2> },
    DrawToposolid { points: Vec<Vec2> },
    DrawPropertyLine { points: Vec<Vec2> },
    DrawRoad { points: Vec<Vec2> },
    DrawSidewalk { points: Vec<Vec2> },
    DrawPathwalk { points: Vec<Vec2> },
    // Freeform Modeling & SketchUp-style Push/Pull
    DrawRectSurface { corner1: Option<Vec2> },
    DrawCircleSurface { center: Option<Vec2> },
    DrawPolygonSurface { points: Vec<Vec2> },
    PushPull { target_id: Option<ElementId>, drag_start: Option<Vec2>, current_height: f64 },
    DrawBoxPrimitive { corner1: Option<Vec2> },
    DrawCylinderPrimitive { center: Option<Vec2> },
    DrawSpherePrimitive { center: Option<Vec2> },
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select (Modify)",
            Tool::DrawWall { is_structural, .. } => {
                if *is_structural {
                    "Structural Wall"
                } else {
                    "Architectural Wall"
                }
            }
            Tool::DrawCurtainWall { .. } => "Curtain Wall & Storefront",
            Tool::DrawColumn { is_structural } => {
                if *is_structural {
                    "Structural Column"
                } else {
                    "Architectural Column"
                }
            }
            Tool::DrawFooting => "Foundation Footing",
            Tool::DrawBeam { .. } => "Structural Beam",
            Tool::DrawSlab { is_structural, .. } => {
                if *is_structural {
                    "Structural Floor"
                } else {
                    "Floor Slab"
                }
            }
            Tool::DrawRoof { shape, .. } => match shape {
                plinth_model::RoofShape::Hip => "Hip Roof",
                plinth_model::RoofShape::Gable => "Gable Roof",
                plinth_model::RoofShape::Shed => "Shed Roof",
                plinth_model::RoofShape::Flat => "Flat Roof",
            },
            Tool::DrawFascia { .. } => "Fascia",
            Tool::DrawGutter { .. } => "Gutter",
            Tool::DrawCeiling { .. } => "Ceiling",
            Tool::DrawStair { .. } => "Monolithic Stair",
            Tool::DrawRamp { .. } => "Access Ramp",
            Tool::DrawRailing { .. } => "Pipe Railing",
            Tool::PlaceDoor => "Place Door",
            Tool::PlaceWindow => "Place Window",
            Tool::DrawGrid { .. } => "Datum Grid Line",
            Tool::DrawDimension { .. } => "Linear Dimension",
            Tool::DrawCircularDimension { .. } => "Circular / Radial Dimension",
            Tool::DrawAngularDimension { .. } => "Angular Dimension",
            Tool::DrawTextAnnotation { .. } => "Text Annotation",
            Tool::DrawLeader { .. } => "Leader Annotation",
            Tool::DrawDetailLine { .. } => "Detail Line",
            Tool::DrawBuildingPad { .. } => "Building Pad (Excavation)",
            Tool::DrawToposolid { .. } => "Toposolid (Terrain)",
            Tool::DrawPropertyLine { .. } => "Property Line Boundary (Cadastral Lot)",
            Tool::DrawRoad { .. } => "Road Corridor (Site)",
            Tool::DrawSidewalk { .. } => "Sidewalk Walkway (Site)",
            Tool::DrawPathwalk { .. } => "Pathwalk Paver Trail (Site)",
            Tool::Move { .. } => "Move",
            Tool::Rotate { .. } => "Rotate",
            Tool::Mirror { .. } => "Mirror",
            Tool::DrawRectSurface { .. } => "Rectangle Surface (2D)",
            Tool::DrawCircleSurface { .. } => "Circle Surface (2D)",
            Tool::DrawPolygonSurface { .. } => "Polygon Surface (2D)",
            Tool::PushPull { .. } => "Push / Pull (Extrude)",
            Tool::DrawBoxPrimitive { .. } => "Box Primitive (3D)",
            Tool::DrawCylinderPrimitive { .. } => "Cylinder Primitive (3D)",
            Tool::DrawSpherePrimitive { .. } => "Sphere Primitive (3D)",
        }
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            Tool::Select => "Click to select element. Drag canvas with Middle Click to pan. Scroll to zoom.",
            Tool::DrawWall { start: None, .. } => "Click to place wall START point (or Esc to cancel).",
            Tool::DrawWall { start: Some(_), .. } => "Click to place wall END point (or Esc to cancel).",
            Tool::DrawCurtainWall { start: None } => "Click start point for curtain wall (draw along or inside a wall to automatically host and cut hole).",
            Tool::DrawCurtainWall { start: Some(_) } => "Click end point to place curtain wall (or Esc to cancel).",
            Tool::DrawColumn { .. } => "Click to place column center in plan.",
            Tool::DrawFooting => "Click in plan (or on a column/grid intersection) to place a structural footing.",
            Tool::DrawBeam { start: None, .. } => "Click to place beam START point.",
            Tool::DrawBeam { start: Some(_), .. } => "Click to place beam END point.",
            Tool::DrawSlab { points, .. } => {
                if points.is_empty() {
                    "Click first boundary vertex for floor slab."
                } else {
                    "Click next vertex, or double-click / click first point to finish slab."
                }
            }
            Tool::DrawRoof { points, .. } => {
                if points.is_empty() {
                    "Click first boundary vertex for roof footprint."
                } else {
                    "Click next vertex, or click initial point to finish roof boundary."
                }
            }
            Tool::DrawFascia { points } => {
                if points.is_empty() {
                    "Click start point for eave fascia board."
                } else {
                    "Click next vertex along eave line (or Esc to commit)."
                }
            }
            Tool::DrawGutter { points } => {
                if points.is_empty() {
                    "Click start point for rainwater gutter."
                } else {
                    "Click next point along roof eave (or Esc to commit)."
                }
            }
            Tool::DrawCeiling { points, .. } => {
                if points.is_empty() {
                    "Click first boundary vertex for ceiling."
                } else {
                    "Click next vertex, or double-click to finish ceiling boundary."
                }
            }
            Tool::DrawStair { start: None } => "Click start point for bottom of stair.",
            Tool::DrawStair { start: Some(_) } => "Click end point for top of stair (or Esc to cancel).",
            Tool::DrawRamp { start: None } => "Click start point for bottom of ramp.",
            Tool::DrawRamp { start: Some(_) } => "Click end point for top of ramp (or Esc to cancel).",
            Tool::DrawRailing { points } => {
                if points.is_empty() {
                    "Click start point for railing path."
                } else {
                    "Click next point along railing path (or double-click / Esc to finish)."
                }
            }
            Tool::PlaceDoor => "Hover and click on any wall to insert a hosted door.",
            Tool::PlaceWindow => "Hover and click on any wall to insert a hosted window.",
            Tool::DrawGrid { start: None } => "Click start point of datum grid line.",
            Tool::DrawGrid { start: Some(_) } => "Click end point of datum grid line.",
            Tool::DrawDimension { start: None, .. } => "Click first reference point for dimension.",
            Tool::DrawDimension { start: Some(_), p2: None, locked_offset: None } => "Click second point to measure.",
            Tool::DrawDimension { start: Some(_), p2: Some(_), .. } => "Move mouse to stretch dimension line, then click to place.",
            Tool::DrawDimension { start: Some(_), p2: None, locked_offset: Some(_) } => "Click next point to continue chained dimension string (Esc to end chain).",
            Tool::DrawCircularDimension { center: None } => "Click center of arc, circle, or column for circular dimension.",
            Tool::DrawCircularDimension { center: Some(_) } => "Click point on rim/circumference to measure radius or diameter.",
            Tool::DrawAngularDimension { apex: None, .. } => "Click vertex/apex of angle.",
            Tool::DrawAngularDimension { apex: Some(_), first_ray: None } => "Click first leg reference point.",
            Tool::DrawAngularDimension { apex: Some(_), first_ray: Some(_) } => "Click second leg point to place angular dimension.",
            Tool::DrawTextAnnotation { pos: None } => "Click position in view to place text annotation note.",
            Tool::DrawTextAnnotation { pos: Some(_) } => "Type text or press Enter to commit.",
            Tool::DrawLeader { target_pt: None, .. } => "Click target point for leader arrowhead.",
            Tool::DrawLeader { target_pt: Some(_), shoulder_pt: None } => "Click shoulder/elbow point for leader line.",
            Tool::DrawLeader { target_pt: Some(_), shoulder_pt: Some(_) } => "Click horizontal landing shelf endpoint.",
            Tool::DrawDetailLine { start: None } => "Click start point of detail line.",
            Tool::DrawDetailLine { start: Some(_) } => "Click end point of detail line.",
            Tool::DrawBuildingPad { points } => {
                if points.is_empty() {
                    "Click first boundary vertex for building excavation pad."
                } else {
                    "Click next vertex, or double-click / press Enter to finish pad boundary."
                }
            }
            Tool::DrawToposolid { points } => {
                if points.is_empty() {
                    "Click first boundary vertex for toposolid terrain."
                } else {
                    "Click next vertex, or double-click / press Enter to finish terrain boundary."
                }
            }
            Tool::DrawPropertyLine { points } => {
                if points.is_empty() {
                    "Click first corner monument for property line boundary."
                } else {
                    "Click next parcel corner, or double-click / press Enter to close lot boundary."
                }
            }
            Tool::DrawRoad { points } => {
                if points.is_empty() {
                    "Click start point of road centerline alignment (double-click ribbon tool to configure profile & lanes)."
                } else {
                    "Click next alignment waypoint, or double-click / press Enter to create road corridor."
                }
            }
            Tool::DrawSidewalk { points } => {
                if points.is_empty() {
                    "Click start point of sidewalk path (double-click ribbon tool to configure curb profile & grooves)."
                } else {
                    "Click next waypoint, or double-click / press Enter to create sidewalk."
                }
            }
            Tool::DrawPathwalk { points } => {
                if points.is_empty() {
                    "Click start point of pathwalk trail (double-click ribbon tool to configure paver pattern & edging)."
                } else {
                    "Click next waypoint, or double-click / press Enter to create pathwalk."
                }
            }
            Tool::Move { from: None } => "Click base point for move.",
            Tool::Move { from: Some(_) } => "Click destination point to finish move.",
            Tool::Rotate { center: None, .. } => "Click center of rotation.",
            Tool::Rotate { center: Some(_), start_angle: None } => "Click reference angle.",
            Tool::Rotate { center: Some(_), start_angle: Some(_) } => "Click target angle to rotate.",
            Tool::Mirror { axis_p1: None } => "Click first point of mirror axis.",
            Tool::Mirror { axis_p1: Some(_) } => "Click second point of mirror axis to reflect.",
            Tool::DrawRectSurface { corner1: None } => "Click first corner of 2D rectangular surface.",
            Tool::DrawRectSurface { corner1: Some(_) } => "Click opposite corner to complete surface (auto-activates Push/Pull).",
            Tool::DrawCircleSurface { center: None } => "Click center point for circular planar surface.",
            Tool::DrawCircleSurface { center: Some(_) } => "Click perimeter point for circle radius.",
            Tool::DrawPolygonSurface { points } => {
                if points.is_empty() {
                    "Click first point of polygon surface."
                } else {
                    "Click next point, or click initial point to close surface."
                }
            }
            Tool::PushPull { target_id: None, .. } => "Click on any 2D surface or solid face to begin Push / Pull.",
            Tool::PushPull { target_id: Some(_), .. } => "Drag mouse or click to commit extrusion height (or Esc to cancel).",
            Tool::DrawBoxPrimitive { corner1: None } => "Click first corner of 3D box base.",
            Tool::DrawBoxPrimitive { corner1: Some(_) } => "Click opposite corner to place 3D box primitive.",
            Tool::DrawCylinderPrimitive { center: None } => "Click center point of 3D cylinder base.",
            Tool::DrawCylinderPrimitive { center: Some(_) } => "Click radius point to place 3D cylinder primitive.",
            Tool::DrawSpherePrimitive { center: None } => "Click center point of 3D sphere.",
            Tool::DrawSpherePrimitive { center: Some(_) } => "Click radius point to place 3D sphere primitive.",
        }
    }
}
