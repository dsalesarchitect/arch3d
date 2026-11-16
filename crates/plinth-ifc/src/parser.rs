//! Robust, Standards-Compliant IFC STEP Parser (ISO 10303-21).
//!
//! Provides comprehensive support for all IFC schema versions:
//! - IFC2X3 (TC1)
//! - IFC4 (Add1, Add2, Add2_TC1)
//! - IFC4X3 (IFC4.3_ADD2)
//! - IFC2X / IFC2X2 (Legacy)
//!
//! Supports automatic unit scaling (metres, millimetres, feet, inches),
//! hierarchical local placement graphs, swept area solids, arbitrary profiles,
//! Brep meshes, triangulated face sets, spatial containment, and wall hosting.

use crate::step::{parse_step_file, StepEntity, StepFile, StepValue, TransformFrame3D};
use plinth_geom::{vec2, Face, Prism, Vec2, Vec3};
use plinth_model::{
    Beam, Camera3D, Ceiling, Column, Document, Door, Element, ElementId, FreeformSolid, Level,
    LevelId, Roof, RoofShape, Slab, ViewDef, ViewKind, Wall, Window,
};
use std::collections::BTreeMap;

/// Result of parsing an IFC file.
pub fn parse_ifc(text: &str) -> Result<Document, String> {
    let step = parse_step_file(text)?;

    // 1. Detect Length Units & Conversion Factor to Millimetres
    let scale_factor = detect_length_scale(&step);

    let mut doc = Document::new_empty();

    // 2. Parse Building Storeys (Levels)
    let (storey_map, default_level_id) = parse_levels(&step, scale_factor, &mut doc);

    // 3. Parse Spatial Structure Containment (Elements -> Storey)
    let elem_to_storey = parse_spatial_containment(&step);

    // 4. Parse Opening Elements and Hosting Relationships
    // IfcRelVoidsElement: Wall -> Opening
    // IfcRelFillsElement: Opening -> Door/Window
    let (opening_to_wall, door_window_to_wall) = parse_opening_relationships(&step);

    // 5. Parse All Building Elements
    let mut imported_elements: Vec<(ElementId, Element, usize)> = Vec::new();
    let mut wall_candidates: BTreeMap<usize, (ElementId, Vec2, Vec2, f64, f64, LevelId)> = BTreeMap::new();

    // Process all entities in the file
    for ent in step.entities.values() {
        let el_id = ent.id;
        let name = ent.name.as_str();

        // Determine which level this element belongs to
        let level_id = elem_to_storey
            .get(&el_id)
            .and_then(|s_id| storey_map.get(s_id))
            .copied()
            .unwrap_or(default_level_id);

        let elem_name = ent
            .get_str(2)
            .unwrap_or_else(|| ent.name.as_str())
            .to_string();

        // World coordinate placement frame
        let placement_id = ent.get_ref(5);
        let world_frame = step.resolve_world_placement(placement_id, scale_factor);

        match name {
            // ---------------------------------------------------------------
            // Walls: IFCWALL, IFCWALLSTANDARDCASE, IFCWALLELEMENTEDCASE
            // ---------------------------------------------------------------
            "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCWALLELEMENTEDCASE" => {
                if let Some((start, end, thickness, height)) = extract_wall_geometry(&step, ent, &world_frame, scale_factor) {
                    let mut wall = Wall::new(start, end, level_id);
                    wall.name = elem_name;
                    wall.thickness = thickness.max(50.0);
                    wall.unconnected_height = height.max(500.0);
                    let id = wall.id;
                    wall_candidates.insert(el_id, (id, start, end, thickness, height, level_id));
                    imported_elements.push((id, Element::Wall(wall), el_id));
                } else {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Wall".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                        let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("Wall".into()));
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            // ---------------------------------------------------------------
            // Roofs: IFCROOF
            // ---------------------------------------------------------------
            "IFCROOF" => {
                let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                if faces.len() >= 4 {
                    let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                    solid.group_name = Some("Roof".into());
                    imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    if boundary.len() >= 3 {
                        let mut roof = Roof::new(boundary.clone(), level_id);
                        roof.name = elem_name;
                        roof.shape = match ent.get_str(8).unwrap_or("").to_uppercase().as_str() {
                            s if s.contains("GABLE") => RoofShape::Gable,
                            s if s.contains("SHED") => RoofShape::Shed,
                            s if s.contains("FLAT") => RoofShape::Flat,
                            _ => RoofShape::Hip,
                        };
                        roof.thickness = 150.0;
                        let level_elev = doc.level_elevation(level_id);
                        roof.offset = z_min - level_elev;
                        let height_diff = (z_max - z_min).abs();
                        if height_diff > 100.0 && boundary.len() >= 4 {
                            let min_x = boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                            let max_x = boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                            let min_y = boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                            let max_y = boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                            let half_span = ((max_x - min_x).min(max_y - min_y) * 0.5).max(1000.0);
                            roof.slope_degrees = (height_diff / half_span).atan().to_degrees().clamp(10.0, 60.0);
                        }
                        imported_elements.push((roof.id, Element::Roof(roof), el_id));
                    }
                }
            }

            // ---------------------------------------------------------------
            // Slabs: IFCSLAB
            // ---------------------------------------------------------------
            "IFCSLAB" => {
                let elem_name_lower = elem_name.to_lowercase();
                let predefined_type = ent.get_str(8).unwrap_or("").to_uppercase();

                // 1. Check if this is a ROOF exported as IFCSLAB (Revit standard export)
                if predefined_type.contains("ROOF") || elem_name_lower.contains("roof") {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Roof".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                        continue;
                    }
                }

                // 2. Check if this is SITE / TOPOGRAPHY / TOPOSOLID exported as IFCSLAB
                if elem_name_lower.contains("site") || elem_name_lower.contains("topo") || elem_name_lower.contains("ground") || elem_name_lower.contains("road") {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 3 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Site".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                        continue;
                    }
                }

                // 3. Check if this is an ISOLATED FOOTING exported as IFCSLAB
                if elem_name_lower.contains("isolated footing") || (elem_name_lower.contains("footing") && !elem_name_lower.contains("slab")) {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Footing".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                        continue;
                    }
                }

                // 4. Standard Floor Slab
                if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    if boundary.len() >= 3 {
                        let raw_thick = (z_max - z_min).abs();
                        let thickness = if raw_thick > 20.0 && raw_thick <= 2000.0 { raw_thick } else { 150.0 };
                        let mut slab = Slab::new(boundary, level_id);
                        slab.name = elem_name;
                        slab.thickness = thickness;
                        let level_elev = doc.level_elevation(level_id);
                        slab.offset = z_max - level_elev;
                        imported_elements.push((slab.id, Element::Slab(slab), el_id));
                    }
                } else {
                    let all_faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if all_faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, all_faces, level_id);
                        solid.group_name = Some("Floor".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            // ---------------------------------------------------------------
            // Columns: IFCCOLUMN, IFCCOLUMNELEMENTEDCASE
            // ---------------------------------------------------------------
            "IFCCOLUMN" | "IFCCOLUMNELEMENTEDCASE" => {
                if let Some((center, width, depth, height, z_min)) = extract_column_geometry(&step, ent, &world_frame, scale_factor) {
                    let mut col = Column::new(center, level_id);
                    col.name = elem_name;
                    col.width = width.max(100.0);
                    col.depth = depth.max(100.0);
                    col.unconnected_height = height.max(500.0);
                    let level_elev = doc.level_elevation(level_id);
                    col.base_offset = z_min - level_elev;
                    imported_elements.push((col.id, Element::Column(col), el_id));
                } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("Column".into()));
                    imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                }
            }

            // ---------------------------------------------------------------
            // Beams: IFCBEAM, IFCBEAMELEMENTEDCASE, IFCMEMBER
            // ---------------------------------------------------------------
            "IFCBEAM" | "IFCBEAMELEMENTEDCASE" | "IFCMEMBER" => {
                if let Some((start, end, width, depth, _z_min, z_max)) = extract_beam_geometry(&step, ent, &world_frame, scale_factor) {
                    let mut beam = Beam::new(start, end, level_id);
                    beam.name = elem_name;
                    beam.width = width.max(50.0);
                    beam.depth = depth.max(100.0);
                    let level_elev = doc.level_elevation(level_id);
                    beam.z_offset = z_max - level_elev;
                    imported_elements.push((beam.id, Element::Beam(beam), el_id));
                } else {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Beam".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                        let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("Beam".into()));
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            // ---------------------------------------------------------------
            // Doors: IFCDOOR, IFCDOORSTANDARDCASE
            // ---------------------------------------------------------------
            "IFCDOOR" | "IFCDOORSTANDARDCASE" => {
                let overall_h = ent.get_f64(8).or_else(|| ent.get_f64(10)).map(|h| h * scale_factor).unwrap_or(2100.0);
                let overall_w = ent.get_f64(9).map(|w| w * scale_factor).unwrap_or(900.0);
                let center_2d = vec2(world_frame.origin.x, world_frame.origin.y);

                // Find host wall
                let host_wall_id = find_host_wall(
                    el_id,
                    center_2d,
                    &door_window_to_wall,
                    &opening_to_wall,
                    &wall_candidates,
                );

                if let Some((hw_id, corner_along, hw_level_id, hw_len)) = host_wall_id {
                    let center_offset = (corner_along + overall_w * 0.5).min(hw_len);
                    let mut door = Door::new(hw_id, center_offset);
                    door.name = elem_name;
                    door.width = overall_w.max(500.0);
                    door.height = overall_h.max(1200.0);
                    door.base_level = Some(hw_level_id);
                    let wall_elev = doc.level_elevation(hw_level_id);
                    door.sill_height = (world_frame.origin.z - wall_elev).max(0.0);
                    imported_elements.push((door.id, Element::Door(door), el_id));
                } else {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Door".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                        if boundary.len() >= 3 {
                            let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("Door".into()));
                            imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                        }
                    }
                }
            }

            // ---------------------------------------------------------------
            // Windows: IFCWINDOW, IFCWINDOWSTANDARDCASE
            // ---------------------------------------------------------------
            "IFCWINDOW" | "IFCWINDOWSTANDARDCASE" => {
                let overall_h = ent.get_f64(8).or_else(|| ent.get_f64(10)).map(|h| h * scale_factor).unwrap_or(1500.0);
                let overall_w = ent.get_f64(9).map(|w| w * scale_factor).unwrap_or(1200.0);
                let center_2d = vec2(world_frame.origin.x, world_frame.origin.y);

                let host_wall_id = find_host_wall(
                    el_id,
                    center_2d,
                    &door_window_to_wall,
                    &opening_to_wall,
                    &wall_candidates,
                );

                if let Some((hw_id, corner_along, hw_level_id, hw_len)) = host_wall_id {
                    let center_offset = (corner_along + overall_w * 0.5).min(hw_len);
                    let mut win = Window::new(hw_id, center_offset);
                    win.name = elem_name;
                    win.width = overall_w.max(400.0);
                    win.height = overall_h.max(400.0);
                    win.base_level = Some(hw_level_id);
                    let wall_elev = doc.level_elevation(hw_level_id);
                    win.sill_height = (world_frame.origin.z - wall_elev).max(0.0);
                    imported_elements.push((win.id, Element::Window(win), el_id));
                } else {
                    let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                        solid.group_name = Some("Window".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                        if boundary.len() >= 3 {
                            let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("Window".into()));
                            imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                        }
                    }
                }
            }

            // ---------------------------------------------------------------
            // Ceilings & Coverings: IFCCEILING, IFCCOVERING
            // ---------------------------------------------------------------
            "IFCCEILING" | "IFCCOVERING" => {
                if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    if boundary.len() >= 3 {
                        let mut ceil = Ceiling::new(boundary, level_id);
                        ceil.name = elem_name;
                        let level_elev = doc.level_elevation(level_id);
                        ceil.height_above_level = (z_min - level_elev).max(0.0);
                        ceil.thickness = (z_max - z_min).abs().clamp(10.0, 100.0);
                        imported_elements.push((ceil.id, Element::Ceiling(ceil), el_id));
                    }
                } else {
                    let all_faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                    if all_faces.len() >= 4 {
                        let mut solid = FreeformSolid::new_mesh(&elem_name, all_faces, level_id);
                        solid.group_name = Some("Ceiling".into());
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            // ---------------------------------------------------------------
            // General Architectural / Structural / MEP / Infrastructure Elements
            // (Stairs, Railings, Ramps, Footings, Curtain Walls, MEP Systems, etc.)
            // ---------------------------------------------------------------
            "IFCBUILDINGELEMENTPROXY"
            | "IFCSTAIR"
            | "IFCSTAIRFLIGHT"
            | "IFCRAMP"
            | "IFCRAMPFLIGHT"
            | "IFCRAILING"
            | "IFCFOOTING"
            | "IFCPLATE"
            | "IFCCURTAINWALL"
            | "IFCFURNISHINGELEMENT"
            | "IFCFURNITURE"
            | "IFCSANITARYTERMINAL"
            | "IFCFLOWTERMINAL"
            | "IFCFLOWSEGMENT"
            | "IFCDUCTSEGMENT"
            | "IFCPIPESEGMENT"
            | "IFCFLOWFITTING"
            | "IFCDUCTFITTING"
            | "IFCPIPEFITTING"
            | "IFCFLOWCONTROLLER"
            | "IFCDAMPER"
            | "IFCVALVE"
            | "IFCFLOWMOVINGDEVICE"
            | "IFCFAN"
            | "IFCPUMP"
            | "IFCENERGYCONVERSIONDEVICE"
            | "IFCBOILER"
            | "IFCCHILLER"
            | "IFCCOIL"
            | "IFCFLOWSTORAGEDEVICE"
            | "IFCTANK"
            | "IFCFLOWTREATMENTDEVICE"
            | "IFCFILTER"
            | "IFCDISTRIBUTIONELEMENT"
            | "IFCDISTRIBUTIONFLOWELEMENT"
            | "IFCELECTRICALELEMENT"
            | "IFCELECTRICAPPLIANCE"
            | "IFCSPACE"
            | "IFCSITE"
            | "IFCBRIDGE"
            | "IFCBRIDGEPART"
            | "IFCROAD"
            | "IFCPAVEMENT"
            | "IFCRAILWAY"
            | "IFCTRACKELEMENT"
            | "IFCEARTHWORKSFILL"
            | "IFCEARTHWORKSCUT" => {
                let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                let elem_name_lower = elem_name.to_lowercase();
                let group_str = match name {
                    "IFCSTAIR" | "IFCSTAIRFLIGHT" => "Stair",
                    "IFCRAILING" => "Railing",
                    "IFCRAMP" | "IFCRAMPFLIGHT" => "Ramp",
                    "IFCFOOTING" => "Footing",
                    "IFCCURTAINWALL" => "Curtain Wall",
                    "IFCPLATE" => {
                        if elem_name_lower.contains("glass") || elem_name_lower.contains("glaz") || elem_name_lower.contains("panel") {
                            "Curtain Wall Panel"
                        } else {
                            "Plate"
                        }
                    }
                    "IFCFURNISHINGELEMENT" | "IFCFURNITURE" => "Furniture",
                    "IFCSANITARYTERMINAL" => "Plumbing Fixture",
                    "IFCFLOWTERMINAL" => {
                        if elem_name_lower.contains("light") || elem_name_lower.contains("lamp") {
                            "Lighting"
                        } else if elem_name_lower.contains("diffuser") || elem_name_lower.contains("grille") {
                            "HVAC Terminal"
                        } else {
                            "MEP Terminal"
                        }
                    }
                    "IFCFLOWSEGMENT" | "IFCDUCTSEGMENT" | "IFCPIPESEGMENT" => {
                        if elem_name_lower.contains("pipe") {
                            "Pipe"
                        } else {
                            "Duct"
                        }
                    }
                    "IFCFLOWFITTING" | "IFCDUCTFITTING" | "IFCPIPEFITTING" => "MEP Fitting",
                    "IFCFLOWCONTROLLER" | "IFCDAMPER" | "IFCVALVE" => "MEP Controller",
                    "IFCFLOWMOVINGDEVICE" | "IFCFAN" | "IFCPUMP" => "MEP Equipment",
                    "IFCENERGYCONVERSIONDEVICE" | "IFCBOILER" | "IFCCHILLER" | "IFCCOIL" => "HVAC Equipment",
                    "IFCFLOWSTORAGEDEVICE" | "IFCTANK" => "Storage Equipment",
                    "IFCFLOWTREATMENTDEVICE" | "IFCFILTER" => "Treatment Device",
                    "IFCDISTRIBUTIONELEMENT" | "IFCDISTRIBUTIONFLOWELEMENT" => "Distribution System",
                    "IFCELECTRICALELEMENT" | "IFCELECTRICAPPLIANCE" => "Electrical",
                    "IFCSPACE" => "Space",
                    "IFCSITE" => "Site",
                    "IFCBRIDGE" | "IFCBRIDGEPART" => "Bridge",
                    "IFCROAD" | "IFCPAVEMENT" => "Road",
                    "IFCRAILWAY" | "IFCTRACKELEMENT" => "Railway",
                    "IFCEARTHWORKSFILL" | "IFCEARTHWORKSCUT" => "Earthworks",
                    _ => {
                        if elem_name_lower.contains("stair") {
                            "Stair"
                        } else if elem_name_lower.contains("railing") {
                            "Railing"
                        } else if elem_name_lower.contains("duct") {
                            "Duct"
                        } else if elem_name_lower.contains("pipe") {
                            "Pipe"
                        } else if elem_name_lower.contains("curtain") {
                            "Curtain Wall"
                        } else {
                            "IFC Element"
                        }
                    }
                };

                if faces.len() >= 4 {
                    let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                    solid.group_name = Some(group_str.into());
                    imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    if boundary.len() >= 3 {
                        let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some(group_str.into()));
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            // Opening elements represent voids in host walls for doors and windows.
            // They are processed during relational hosting and must not be imported
            // as visible solid geometry.
            "IFCOPENINGELEMENT" => {}

            // Universal catch-all for any other IFC element containing 3D geometry
            _ if name.starts_with("IFC") => {
                let faces = extract_all_element_faces(&step, ent, &world_frame, scale_factor);
                if faces.len() >= 4 {
                    let mut solid = FreeformSolid::new_mesh(&elem_name, faces, level_id);
                    solid.group_name = Some("IFC Element".into());
                    imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                } else if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(&step, ent, &world_frame, scale_factor) {
                    if boundary.len() >= 3 {
                        let solid = FreeformSolid::new_custom_extrusion(elem_name, boundary, z_min, z_max, level_id, Some("IFC Element".into()));
                        imported_elements.push((solid.id, Element::FreeformSolid(solid), el_id));
                    }
                }
            }

            _ => {}
        }
    }

    if imported_elements.is_empty() {
        return Err("No supported 3D architectural or structural elements could be extracted from IFC file".into());
    }

    // Insert all elements into the document
    for (id, elem, _) in imported_elements {
        doc.elements.insert(id, elem);
    }

    // 6. Recalibrate Default Camera and Standard Views
    setup_views_for_imported_model(&mut doc);

    Ok(doc)
}

/// Backwards compatibility alias for `parse_ifc`.
pub fn parse_ifc4(text: &str) -> Result<Document, String> {
    parse_ifc(text)
}

// ---------------------------------------------------------------------------
// Unit Scaling & Detection
// ---------------------------------------------------------------------------

/// Detects the length unit scale factor required to convert IFC measurements into millimetres.
fn detect_length_scale(step: &StepFile) -> f64 {
    // 1. Check IFCUNITASSIGNMENT
    for ent in step.iter_by_name("IFCUNITASSIGNMENT") {
        if let Some(units_list) = ent.get_list(0) {
            for unit_val in units_list {
                if let Some(unit_id) = unit_val.as_ref() {
                    if let Some(unit_ent) = step.get(unit_id) {
                        if unit_ent.name == "IFCSIUNIT" {
                            let is_length = unit_ent.get_str(1).map_or(false, |s| s.contains("LENGTHUNIT"));
                            if is_length {
                                let prefix = unit_ent.get_str(2).unwrap_or("");
                                if prefix.contains("MILLI") {
                                    return 1.0; // Already in millimetres
                                } else if prefix.contains("CENTI") {
                                    return 10.0; // Centimetres to mm
                                } else {
                                    return 1000.0; // Metres to mm
                                }
                            }
                        } else if unit_ent.name == "IFCCONVERSIONBASEDUNIT" {
                            let name = unit_ent.get_str(2).unwrap_or("").to_uppercase();
                            if name.contains("FOOT") || name.contains("FEET") {
                                return 304.8; // Feet to mm
                            } else if name.contains("INCH") {
                                return 25.4; // Inches to mm
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback Heuristic Inspection of Coordinates:
    // If maximum coordinate values are < 120.0, the model is almost certainly in metres.
    let mut max_coord = 0.0f64;
    let mut sample_count = 0;
    for ent in step.iter_by_name("IFCCARTESIANPOINT") {
        if let Some(list) = ent.get_list(0) {
            for val in list {
                if let Some(c) = val.as_f64() {
                    max_coord = max_coord.max(c.abs());
                    sample_count += 1;
                    if sample_count > 100 {
                        break;
                    }
                }
            }
        }
        if sample_count > 100 {
            break;
        }
    }

    if max_coord > 0.0 && max_coord < 150.0 {
        1000.0 // Metres to mm
    } else {
        1.0 // Default to millimetres
    }
}

// ---------------------------------------------------------------------------
// Storey / Level Parsing
// ---------------------------------------------------------------------------

fn parse_levels(
    step: &StepFile,
    scale: f64,
    doc: &mut Document,
) -> (BTreeMap<usize, LevelId>, LevelId) {
    let mut storey_map = BTreeMap::new();
    let mut raw_storeys: Vec<(usize, String, f64)> = Vec::new();

    for ent in step.iter_by_name("IFCBUILDINGSTOREY") {
        let name = ent.get_str(2).unwrap_or("Storey").to_string();
        // Elevation can be in arg 8 or 9
        let elev = ent
            .get_f64(8)
            .or_else(|| ent.get_f64(9))
            .map(|e| e * scale)
            .or_else(|| {
                // Check placement
                ent.get_ref(5)
                    .map(|p_id| step.resolve_world_placement(Some(p_id), scale).origin.z)
            })
            .unwrap_or(0.0);

        raw_storeys.push((ent.id, name, elev));
    }

    // Sort storeys by elevation ascending
    raw_storeys.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    if !raw_storeys.is_empty() {
        doc.levels.clear();
        for i in 0..raw_storeys.len() {
            let (ent_id, name, elev) = &raw_storeys[i];
            let height = if i + 1 < raw_storeys.len() {
                (raw_storeys[i + 1].2 - elev).max(2000.0)
            } else {
                3500.0
            };
            let lvl = Level::new(name, *elev, height);
            storey_map.insert(*ent_id, lvl.id);
            doc.levels.insert(lvl.id, lvl);
        }
        let first_id = *doc.levels.keys().next().unwrap();
        doc.active_level_id = first_id;
        (storey_map, first_id)
    } else {
        // Fallback default level
        let default_lvl = Level::new("Ground Floor", 0.0, 3500.0);
        let id = default_lvl.id;
        doc.levels.insert(id, default_lvl);
        doc.active_level_id = id;
        (storey_map, id)
    }
}

// ---------------------------------------------------------------------------
// Spatial Structure & Relationship Parsing
// ---------------------------------------------------------------------------

fn parse_spatial_containment(step: &StepFile) -> BTreeMap<usize, usize> {
    let mut map = BTreeMap::new();
    for ent in step.iter_by_name("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
        // arg 4: list of element references, arg 5: spatial container reference
        if let (Some(elems), Some(container_id)) = (ent.get_list(4), ent.get_ref(5)) {
            for el_val in elems {
                if let Some(el_id) = el_val.as_ref() {
                    map.insert(el_id, container_id);
                }
            }
        }
    }
    map
}

fn parse_opening_relationships(
    step: &StepFile,
) -> (BTreeMap<usize, usize>, BTreeMap<usize, usize>) {
    let mut opening_to_wall = BTreeMap::new();
    let mut door_win_to_wall = BTreeMap::new();

    // IfcRelVoidsElement: Wall -> Opening
    for ent in step.iter_by_name("IFCRELVOIDSELEMENT") {
        if let (Some(wall_id), Some(opening_id)) = (ent.get_ref(4), ent.get_ref(5)) {
            opening_to_wall.insert(opening_id, wall_id);
        }
    }

    // IfcRelFillsElement: Opening -> Door/Window
    for ent in step.iter_by_name("IFCRELFILLSELEMENT") {
        if let (Some(opening_id), Some(door_win_id)) = (ent.get_ref(4), ent.get_ref(5)) {
            if let Some(wall_id) = opening_to_wall.get(&opening_id) {
                door_win_to_wall.insert(door_win_id, *wall_id);
            }
        }
    }

    (opening_to_wall, door_win_to_wall)
}

// ---------------------------------------------------------------------------
// Geometry Extraction Helpers
// ---------------------------------------------------------------------------

/// Attempts to extract an architectural wall's start, end, thickness, and height.
fn extract_wall_geometry(
    step: &StepFile,
    ent: &StepEntity,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec2, Vec2, f64, f64)> {
    let rep_id = ent.get_ref(6)?;
    let prod_shape = step.get(rep_id)?;
    if prod_shape.name != "IFCPRODUCTDEFINITIONSHAPE" {
        return None;
    }

    let reps = prod_shape.get_list(2)?;
    let mut axis_pts: Option<(Vec2, Vec2)> = None;
    let mut solid_dim: Option<(f64, f64, Vec<Vec2>)> = None;

    for r_val in reps {
        if let Some(r_id) = r_val.as_ref() {
            if let Some(shape_rep) = step.get(r_id) {
                if shape_rep.name == "IFCSHAPEREPRESENTATION" {
                    let rep_type = shape_rep.get_str(2).unwrap_or("");
                    if let Some(items) = shape_rep.get_list(3) {
                        // 1. Axis Representation (Centerline)
                        if rep_type.eq_ignore_ascii_case("Curve2D") || rep_type.eq_ignore_ascii_case("Axis") {
                            for item_val in items {
                                if let Some(item_id) = item_val.as_ref() {
                                    if let Some(curve) = step.get(item_id) {
                                        if curve.name == "IFCPOLYLINE" {
                                            if let Some(pts_list) = curve.get_list(0) {
                                                if pts_list.len() >= 2 {
                                                    if let (Some(p1_id), Some(p2_id)) = (pts_list[0].as_ref(), pts_list[1].as_ref()) {
                                                        if let (Some(p1), Some(p2)) = (step.resolve_point_3d(p1_id, scale), step.resolve_point_3d(p2_id, scale)) {
                                                            let wp1 = world_frame.transform_point(p1);
                                                            let wp2 = world_frame.transform_point(p2);
                                                            axis_pts = Some((vec2(wp1.x, wp1.y), vec2(wp2.x, wp2.y)));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 2. SweptSolid / SolidModel / Brep Representation
                        if rep_type.eq_ignore_ascii_case("SweptSolid")
                            || rep_type.eq_ignore_ascii_case("SolidModel")
                            || rep_type.eq_ignore_ascii_case("Brep")
                            || rep_type.is_empty()
                        {
                            for item_val in items {
                                if let Some(item_id) = item_val.as_ref() {
                                    if let Some((boundary, height, thick, _, _)) = extract_extrusion_info(step, item_id, world_frame, scale) {
                                        solid_dim = Some((height, thick, boundary));
                                    } else if let Some((boundary, z_min, z_max)) = extract_brep_geometry(step, item_id, world_frame, scale) {
                                        let height = (z_max - z_min).abs();
                                        let thick = if let Some((ap1, ap2)) = axis_pts {
                                            let axis_len = ap1.distance(ap2);
                                            if axis_len > 1.0 {
                                                let norm = (ap2 - ap1).normalized().perp();
                                                let dists: Vec<f64> = boundary.iter().map(|pt| (*pt - ap1).dot(norm)).collect();
                                                let max_d = dists.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                                let min_d = dists.iter().cloned().fold(f64::INFINITY, f64::min);
                                                (max_d - min_d).abs().clamp(50.0, 600.0)
                                            } else {
                                                150.0
                                            }
                                        } else {
                                            150.0
                                        };
                                        solid_dim = Some((height, thick, boundary));
                                    }
                                }
                            }
                        }
                    }
                } else if shape_rep.name == "IFCEXTRUDEDAREASOLID" || shape_rep.name == "IFCBOOLEANCLIPPINGRESULT" || shape_rep.name == "IFCBOOLEANRESULT" {
                    // Direct solid reference resilience
                    if let Some((boundary, height, thick, _, _)) = extract_extrusion_info(step, r_id, world_frame, scale) {
                        solid_dim = Some((height, thick, boundary));
                    }
                } else if shape_rep.name == "IFCFACETEDBREP" || shape_rep.name == "IFCSHELLBASEDSURFACEMODEL" {
                    if let Some((boundary, z_min, z_max)) = extract_brep_geometry(step, r_id, world_frame, scale) {
                        let height = (z_max - z_min).abs();
                        solid_dim = Some((height, 150.0, boundary));
                    }
                }
            }
        }
    }

    if let Some((p1, p2)) = axis_pts {
        let (height, thick) = if let Some((h, t, _)) = solid_dim {
            (h.max(500.0), t.clamp(50.0, 600.0))
        } else {
            (3000.0, 150.0)
        };
        return Some((p1, p2, thick, height));
    }

    if let Some((height, _thick, boundary)) = solid_dim {
        if boundary.len() == 4 {
            // Find two long sides
            let mut edges = Vec::new();
            for i in 0..4 {
                let a = boundary[i];
                let b = boundary[(i + 1) % 4];
                edges.push((a, b, a.distance(b)));
            }
            edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            // Midpoints of short sides give start and end
            let start = (edges[2].0 + edges[2].1) * 0.5;
            let end = (edges[3].0 + edges[3].1) * 0.5;
            let thickness = edges[2].2.min(edges[3].2);
            return Some((start, end, thickness, height));
        }
    }

    None
}

/// Extracts column geometry: (center, width, depth, height, z_min).
fn extract_column_geometry(
    step: &StepFile,
    ent: &StepEntity,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec2, f64, f64, f64, f64)> {
    let (boundary, z_min, z_max) = extract_general_solid_geometry(step, ent, world_frame, scale)?;
    if boundary.is_empty() {
        return None;
    }
    let height = (z_max - z_min).abs();
    let min_x = boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    let center = vec2((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let width = (max_x - min_x).max(100.0);
    let depth = (max_y - min_y).max(100.0);
    Some((center, width, depth, height, z_min))
}

/// Extracts beam geometry: (start, end, width, depth, z_min, z_max).
fn extract_beam_geometry(
    step: &StepFile,
    ent: &StepEntity,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec2, Vec2, f64, f64, f64, f64)> {
    let rep_id = ent.get_ref(6)?;
    let prod_shape = step.get(rep_id)?;
    if prod_shape.name != "IFCPRODUCTDEFINITIONSHAPE" {
        return None;
    }
    let reps = prod_shape.get_list(2)?;

    let mut axis_start_end: Option<(Vec2, Vec2, f64, f64)> = None;
    let mut profile_dims: Option<(f64, f64)> = None;
    let mut swept_solid_info: Option<(Vec2, Vec2, f64, f64, f64, f64)> = None;

    for r_val in reps {
        if let Some(r_id) = r_val.as_ref() {
            if let Some(shape_rep) = step.get(r_id) {
                if shape_rep.name == "IFCSHAPEREPRESENTATION" {
                    let rep_type = shape_rep.get_str(2).unwrap_or("");
                    if let Some(items) = shape_rep.get_list(3) {
                        // 1. Centerline Axis Representation
                        if rep_type.eq_ignore_ascii_case("Curve2D")
                            || rep_type.eq_ignore_ascii_case("Curve3D")
                            || rep_type.eq_ignore_ascii_case("Axis")
                        {
                            for item_val in items {
                                if let Some(item_id) = item_val.as_ref() {
                                    if let Some(curve) = step.get(item_id) {
                                        if curve.name == "IFCPOLYLINE" {
                                            if let Some(pts_list) = curve.get_list(0) {
                                                if pts_list.len() >= 2 {
                                                    if let (Some(p1_id), Some(p2_id)) = (pts_list[0].as_ref(), pts_list[1].as_ref()) {
                                                        if let (Some(p1), Some(p2)) = (
                                                            step.resolve_point_3d(p1_id, scale),
                                                            step.resolve_point_3d(p2_id, scale),
                                                        ) {
                                                            let wp1 = world_frame.transform_point(p1);
                                                            let wp2 = world_frame.transform_point(p2);
                                                            let s = vec2(wp1.x, wp1.y);
                                                            let e = vec2(wp2.x, wp2.y);
                                                            if s.distance(e) > 50.0 {
                                                                let z_min = wp1.z.min(wp2.z);
                                                                let z_max = wp1.z.max(wp2.z);
                                                                axis_start_end = Some((s, e, z_min, z_max));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 2. SweptSolid / Body Representation
                        if rep_type.eq_ignore_ascii_case("SweptSolid")
                            || rep_type.eq_ignore_ascii_case("SolidModel")
                            || rep_type.eq_ignore_ascii_case("Body")
                            || rep_type.is_empty()
                        {
                            for item_val in items {
                                if let Some(item_id) = item_val.as_ref() {
                                    if let Some(solid_ent) = step.get(item_id) {
                                        if solid_ent.name == "IFCEXTRUDEDAREASOLID" {
                                            if let (Some(prof_id), Some(pos_id), Some(dir_id), Some(len)) = (
                                                solid_ent.get_ref(0),
                                                solid_ent.get_ref(1),
                                                solid_ent.get_ref(2),
                                                solid_ent.get_f64(3),
                                            ) {
                                                let beam_len = len * scale;
                                                let mut profile_pts: Vec<Vec2> = Vec::new();
                                                let mut raw_x_dim = 200.0 * scale;
                                                let mut raw_y_dim = 300.0 * scale;

                                                if let Some(prof_ent) = step.get(prof_id) {
                                                    if prof_ent.name == "IFCRECTANGLEPROFILEDEF" {
                                                        let d1 = prof_ent.get_f64(3).unwrap_or(200.0) * scale;
                                                        let d2 = prof_ent.get_f64(4).unwrap_or(300.0) * scale;
                                                        raw_x_dim = d1;
                                                        raw_y_dim = d2;
                                                        let hx = d1 * 0.5;
                                                        let hy = d2 * 0.5;
                                                        let mut pts = vec![
                                                            vec2(-hx, -hy),
                                                            vec2(hx, -hy),
                                                            vec2(hx, hy),
                                                            vec2(-hx, hy),
                                                        ];
                                                        if let Some(pos_2d_id) = prof_ent.get_ref(2) {
                                                            if let Some((origin, u_x, u_y)) = step.resolve_axis2_placement_2d(pos_2d_id, scale) {
                                                                for p in &mut pts {
                                                                    *p = origin + u_x * p.x + u_y * p.y;
                                                                }
                                                            }
                                                        }
                                                        profile_pts = pts;
                                                        profile_dims = Some((d1.min(d2), d1.max(d2)));
                                                    } else if let Some(pts) = extract_profile_points(step, prof_id, scale) {
                                                        let min_x = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                                                        let max_x = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                                                        let min_y = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                                                        let max_y = pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                                                        let w = (max_x - min_x).abs();
                                                        let d = (max_y - min_y).abs();
                                                        profile_dims = Some((w.min(d), w.max(d)));
                                                        profile_pts = pts;
                                                    }
                                                }

                                                if profile_pts.is_empty() {
                                                    let hx = raw_x_dim * 0.5;
                                                    let hy = raw_y_dim * 0.5;
                                                    profile_pts = vec![
                                                        vec2(-hx, -hy),
                                                        vec2(hx, -hy),
                                                        vec2(hx, hy),
                                                        vec2(-hx, hy),
                                                    ];
                                                }

                                                let extr_frame = step
                                                    .resolve_axis2_placement_3d(pos_id, scale)
                                                    .unwrap_or_else(TransformFrame3D::identity);
                                                let extr_dir = step
                                                    .resolve_direction_3d(dir_id)
                                                    .unwrap_or(Vec3::Z);
                                                let combined = world_frame.combine(&extr_frame);

                                                let mut world_pts = Vec::new();
                                                for &p in &profile_pts {
                                                    let p0 = Vec3::new(p.x, p.y, 0.0);
                                                    let p1 = p0 + extr_dir * beam_len;
                                                    world_pts.push(combined.transform_point(p0));
                                                    world_pts.push(combined.transform_point(p1));
                                                }

                                                let solid_z_min = world_pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
                                                let solid_z_max = world_pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
                                                let solid_depth = (solid_z_max - solid_z_min).abs();

                                                let extr_world_vec = combined.transform_vector(extr_dir * beam_len);
                                                let extr_span_2d = vec2(extr_world_vec.x, extr_world_vec.y);

                                                if let Some((axis_s, axis_e, _, _)) = axis_start_end {
                                                    let dir = (axis_e - axis_s).normalized();
                                                    let norm = dir.perp();
                                                    let d_norm_min = world_pts.iter().map(|w| (vec2(w.x, w.y) - axis_s).dot(norm)).fold(f64::INFINITY, f64::min);
                                                    let d_norm_max = world_pts.iter().map(|w| (vec2(w.x, w.y) - axis_s).dot(norm)).fold(f64::NEG_INFINITY, f64::max);
                                                    let width = (d_norm_max - d_norm_min).abs().max(50.0);
                                                    let depth = solid_depth.max(50.0);
                                                    swept_solid_info = Some((axis_s, axis_e, width, depth, solid_z_min, solid_z_max));
                                                } else if extr_span_2d.length() > 50.0 {
                                                    let p_start_3d = combined.transform_point(Vec3::ZERO);
                                                    let s_2d = vec2(p_start_3d.x, p_start_3d.y);
                                                    let e_2d = s_2d + extr_span_2d;
                                                    let dir = extr_span_2d.normalized();
                                                    let norm = dir.perp();
                                                    let d_norm_min = world_pts.iter().map(|w| (vec2(w.x, w.y) - s_2d).dot(norm)).fold(f64::INFINITY, f64::min);
                                                    let d_norm_max = world_pts.iter().map(|w| (vec2(w.x, w.y) - s_2d).dot(norm)).fold(f64::NEG_INFINITY, f64::max);
                                                    let width = (d_norm_max - d_norm_min).abs().max(50.0);
                                                    let depth = solid_depth.max(50.0);
                                                    swept_solid_info = Some((s_2d, e_2d, width, depth, solid_z_min, solid_z_max));
                                                } else {
                                                    // Vertical extrusion from horizontal 2D footprint (Style 2)
                                                    let pts_2d: Vec<Vec2> = world_pts.iter().take(profile_pts.len()).map(|w| vec2(w.x, w.y)).collect();
                                                    if pts_2d.len() >= 4 {
                                                        let mut edges = Vec::new();
                                                        for i in 0..pts_2d.len() {
                                                            let a = pts_2d[i];
                                                            let b = pts_2d[(i + 1) % pts_2d.len()];
                                                            edges.push((a, b, a.distance(b)));
                                                        }
                                                        edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
                                                        let s_2d = (edges[0].0 + edges[1].0) * 0.5;
                                                        let e_2d = (edges[0].1 + edges[1].1) * 0.5;
                                                        let width = (edges[0].0.distance(edges[1].0)).max(50.0);
                                                        let depth = solid_depth.max(50.0);
                                                        swept_solid_info = Some((s_2d, e_2d, width, depth, solid_z_min, solid_z_max));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Priority 1: High-precision swept solid info
    if let Some(info) = swept_solid_info {
        return Some(info);
    }

    // Priority 2: Axis polyline fallback if swept solid was missing
    if let Some((start, end, _z_min, z_max)) = axis_start_end {
        let (width, depth) = profile_dims.unwrap_or((200.0, 300.0));
        let top_z = z_max;
        let bot_z = top_z - depth;
        return Some((start, end, width, depth, bot_z, top_z));
    }

    if let Some((boundary, z_min, z_max)) = extract_general_solid_geometry(step, ent, world_frame, scale) {
        if boundary.len() >= 4 {
            let depth = (z_max - z_min).abs();
            let mut edges = Vec::new();
            for i in 0..boundary.len() {
                let a = boundary[i];
                let b = boundary[(i + 1) % boundary.len()];
                edges.push((a, b, a.distance(b)));
            }
            edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            let start = (edges[0].0 + edges[1].0) * 0.5;
            let end = (edges[0].1 + edges[1].1) * 0.5;
            if start.distance(end) > 50.0 {
                let width = (edges[0].0.distance(edges[1].0)).max(100.0);
                return Some((start, end, width, depth, z_min, z_max));
            }
        }
    }

    None
}

/// General fallback solid geometry extractor: returns (boundary_polygon, z_min, z_max) in world coordinates.
fn extract_general_solid_geometry(
    step: &StepFile,
    ent: &StepEntity,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec<Vec2>, f64, f64)> {
    let rep_id = ent.get_ref(6)?;
    let prod_shape = step.get(rep_id)?;
    if prod_shape.name != "IFCPRODUCTDEFINITIONSHAPE" {
        return None;
    }

    let reps = prod_shape.get_list(2)?;
    for r_val in reps {
        if let Some(r_id) = r_val.as_ref() {
            if let Some(shape_rep) = step.get(r_id) {
                if shape_rep.name == "IFCSHAPEREPRESENTATION" {
                    if let Some(geom) = extract_shape_rep_geometry(step, r_id, world_frame, scale, 4) {
                        return Some(geom);
                    }
                } else {
                    // Direct solid reference resilience
                    if let Some((boundary, _, _, z_min, z_max)) = extract_extrusion_info(step, r_id, world_frame, scale) {
                        return Some((boundary, z_min, z_max));
                    }
                    if let Some((boundary, z_min, z_max)) = extract_brep_geometry(step, r_id, world_frame, scale) {
                        return Some((boundary, z_min, z_max));
                    }
                    if let Some(geom) = extract_triangulated_face_set_geometry(step, r_id, world_frame, scale) {
                        return Some(geom);
                    }
                }
            }
        }
    }

    None
}

/// Recursively evaluates items in an `IFCSHAPEREPRESENTATION`.
fn extract_shape_rep_geometry(
    step: &StepFile,
    rep_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
    depth: usize,
) -> Option<(Vec<Vec2>, f64, f64)> {
    let rep_ent = step.get(rep_id)?;
    let items = rep_ent.get_list(3)?;

    for item_val in items {
        if let Some(item_id) = item_val.as_ref() {
            if let Some(item_ent) = step.get(item_id) {
                // 1. Extruded Area Solid or Boolean Operation
                if item_ent.name == "IFCEXTRUDEDAREASOLID"
                    || item_ent.name == "IFCBOOLEANCLIPPINGRESULT"
                    || item_ent.name == "IFCBOOLEANRESULT"
                {
                    if let Some((boundary, _, _, z_min, z_max)) = extract_extrusion_info(step, item_id, world_frame, scale) {
                        return Some((boundary, z_min, z_max));
                    }
                }
                // 2. Faceted Brep or Shell
                if item_ent.name == "IFCFACETEDBREP"
                    || item_ent.name == "IFCSHELLBASEDSURFACEMODEL"
                    || item_ent.name == "IFCFACEBASEDSURFACEMODEL"
                {
                    if let Some(geom) = extract_brep_geometry(step, item_id, world_frame, scale) {
                        return Some(geom);
                    }
                }
                // 3. Triangulated / Polygonal Face Set (IFC4 / IFC4X3)
                if item_ent.name == "IFCTRIANGULATEDFACESET" || item_ent.name == "IFCPOLYGONALFACESET" {
                    if let Some(geom) = extract_triangulated_face_set_geometry(step, item_id, world_frame, scale) {
                        return Some(geom);
                    }
                }
                // 4. Mapped Item (Family instances, components)
                if item_ent.name == "IFCMAPPEDITEM" {
                    if let Some(geom) = extract_mapped_item_geometry(step, item_id, world_frame, scale, depth) {
                        return Some(geom);
                    }
                }
            }
        }
    }
    None
}

/// Resolves `IFCMAPPEDITEM` referencing an `IFCREPRESENTATIONMAP`.
fn extract_mapped_item_geometry(
    step: &StepFile,
    item_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
    depth: usize,
) -> Option<(Vec<Vec2>, f64, f64)> {
    if depth == 0 {
        return None;
    }
    let ent = step.get(item_id)?;
    if ent.name != "IFCMAPPEDITEM" {
        return None;
    }

    let source_id = ent.get_ref(0)?;
    let target_id = ent.get_ref(1)?;

    let rep_map = step.get(source_id)?;
    if rep_map.name != "IFCREPRESENTATIONMAP" {
        return None;
    }

    let map_origin_id = rep_map.get_ref(0);
    let mapped_rep_id = rep_map.get_ref(1)?;

    let target_frame = step
        .resolve_cartesian_transformation_3d(target_id, scale)
        .unwrap_or_else(TransformFrame3D::identity);

    let origin_frame = map_origin_id
        .and_then(|id| step.resolve_axis2_placement_3d(id, scale))
        .unwrap_or_else(TransformFrame3D::identity);

    let combined_frame = world_frame.combine(&target_frame).combine(&origin_frame);
    extract_shape_rep_geometry(step, mapped_rep_id, &combined_frame, scale, depth - 1)
}

/// Extracts vertices and bounding range from `IFCTRIANGULATEDFACESET` or `IFCPOLYGONALFACESET`.
fn extract_triangulated_face_set_geometry(
    step: &StepFile,
    ent_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec<Vec2>, f64, f64)> {
    let ent = step.get(ent_id)?;
    if ent.name != "IFCTRIANGULATEDFACESET" && ent.name != "IFCPOLYGONALFACESET" {
        return None;
    }
    let coords_id = ent.get_ref(0)?;
    let coords_ent = step.get(coords_id)?;
    if coords_ent.name != "IFCCARTESIANPOINTLIST3D" {
        return None;
    }
    let points_list = coords_ent.get_list(0)?;
    let mut world_pts = Vec::new();
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;

    for pt_val in points_list {
        if let StepValue::List(xyz) = pt_val {
            if xyz.len() >= 3 {
                let x = xyz[0].as_f64().unwrap_or(0.0) * scale;
                let y = xyz[1].as_f64().unwrap_or(0.0) * scale;
                let z = xyz[2].as_f64().unwrap_or(0.0) * scale;
                let wp = world_frame.transform_point(Vec3::new(x, y, z));
                z_min = z_min.min(wp.z);
                z_max = z_max.max(wp.z);
                world_pts.push(vec2(wp.x, wp.y));
            }
        }
    }

    if world_pts.len() >= 3 && z_min.is_finite() && z_max.is_finite() {
        let min_x = world_pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = world_pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = world_pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = world_pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let footprint = vec![
            vec2(min_x, min_y),
            vec2(max_x, min_y),
            vec2(max_x, max_y),
            vec2(min_x, max_y),
        ];
        Some((footprint, z_min, z_max))
    } else {
        None
    }
}

/// Resolves an `IFCEXTRUDEDAREASOLID` or `IFCBOOLEANCLIPPINGRESULT`.
/// Returns (world_boundary, depth, thickness, z_min, z_max).
fn extract_extrusion_info(
    step: &StepFile,
    solid_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec<Vec2>, f64, f64, f64, f64)> {
    let ent = step.get(solid_id)?;

    // Handle Boolean clipping result: first operand contains the base solid
    if ent.name == "IFCBOOLEANCLIPPINGRESULT" || ent.name == "IFCBOOLEANRESULT" {
        if let Some(first_op_id) = ent.get_ref(2) {
            return extract_extrusion_info(step, first_op_id, world_frame, scale);
        }
    }

    if ent.name != "IFCEXTRUDEDAREASOLID" {
        return None;
    }

    let profile_id = ent.get_ref(0)?;
    let pos_id = ent.get_ref(1);
    let depth = ent.get_f64(3).unwrap_or(3000.0) * scale;

    let extr_frame = pos_id
        .and_then(|id| step.resolve_axis2_placement_3d(id, scale))
        .unwrap_or_else(TransformFrame3D::identity);

    let combined_frame = world_frame.combine(&extr_frame);

    // Extract 2D profile points
    let local_profile = extract_profile_points(step, profile_id, scale)?;
    if local_profile.len() < 3 {
        return None;
    }

    // Transform profile points to world coordinates
    let world_boundary: Vec<Vec2> = local_profile
        .iter()
        .map(|p| {
            let p3 = combined_frame.transform_point(Vec3::new(p.x, p.y, 0.0));
            vec2(p3.x, p3.y)
        })
        .collect();

    let thickness = if world_boundary.len() >= 4 {
        world_boundary[0].distance(world_boundary[1]).min(world_boundary[1].distance(world_boundary[2]))
    } else {
        200.0
    };

    let z_min = combined_frame.origin.z;
    let z_max = z_min + depth;

    Some((world_boundary, depth, thickness, z_min, z_max))
}

/// Extracts 2D polygon points from an `IFCPROFILEDEF`.
fn extract_profile_points(step: &StepFile, profile_id: usize, scale: f64) -> Option<Vec<Vec2>> {
    let ent = step.get(profile_id)?;
    match ent.name.as_str() {
        "IFCARBITRARYCLOSEDPROFILEDEF" | "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
            let curve_id = ent.get_ref(2)?;
            extract_curve_points(step, curve_id, scale)
        }
        "IFCRECTANGLEPROFILEDEF" | "IFCRECTANGLEHOLLOWPROFILEDEF" => {
            let x_dim = ent.get_f64(3).unwrap_or(500.0) * scale;
            let y_dim = ent.get_f64(4).unwrap_or(500.0) * scale;
            let hw = x_dim * 0.5;
            let hl = y_dim * 0.5;
            Some(vec![
                vec2(-hw, -hl),
                vec2(hw, -hl),
                vec2(hw, hl),
                vec2(-hw, hl),
            ])
        }
        "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
            let rad = ent.get_f64(3).unwrap_or(200.0) * scale;
            let segs = 24;
            let mut pts = Vec::with_capacity(segs);
            for i in 0..segs {
                let a = (i as f64 / segs as f64) * std::f64::consts::TAU;
                pts.push(vec2(rad * a.cos(), rad * a.sin()));
            }
            Some(pts)
        }
        "IFCISHAPEPROFILEDEF" => {
            let w = ent.get_f64(3).unwrap_or(200.0) * scale;
            let d = ent.get_f64(4).unwrap_or(300.0) * scale;
            let web_t = ent.get_f64(5).unwrap_or(10.0) * scale;
            let fl_t = ent.get_f64(6).unwrap_or(15.0) * scale;

            let hw = w * 0.5;
            let hd = d * 0.5;
            let hwt = web_t * 0.5;

            Some(vec![
                vec2(-hw, -hd),
                vec2(hw, -hd),
                vec2(hw, -hd + fl_t),
                vec2(hwt, -hd + fl_t),
                vec2(hwt, hd - fl_t),
                vec2(hw, hd - fl_t),
                vec2(hw, hd),
                vec2(-hw, hd),
                vec2(-hw, hd - fl_t),
                vec2(-hwt, hd - fl_t),
                vec2(-hwt, -hd + fl_t),
                vec2(-hw, -hd + fl_t),
            ])
        }
        "IFCLSHAPEPROFILEDEF" => {
            let h = ent.get_f64(3).unwrap_or(100.0) * scale;
            let w = ent.get_f64(4).unwrap_or(100.0) * scale;
            let t = ent.get_f64(5).unwrap_or(10.0) * scale;
            let hw = w * 0.5;
            let hh = h * 0.5;
            Some(vec![
                vec2(-hw, -hh),
                vec2(hw, -hh),
                vec2(hw, -hh + t),
                vec2(-hw + t, -hh + t),
                vec2(-hw + t, hh),
                vec2(-hw, hh),
            ])
        }
        "IFCCSHAPEPROFILEDEF" | "IFCUSHAPEPROFILEDEF" => {
            let h = ent.get_f64(3).unwrap_or(150.0) * scale;
            let w = ent.get_f64(4).unwrap_or(75.0) * scale;
            let tw = ent.get_f64(5).unwrap_or(8.0) * scale;
            let tf = ent.get_f64(6).unwrap_or(10.0) * scale;
            let hw = w * 0.5;
            let hh = h * 0.5;
            Some(vec![
                vec2(-hw, -hh),
                vec2(hw, -hh),
                vec2(hw, -hh + tf),
                vec2(-hw + tw, -hh + tf),
                vec2(-hw + tw, hh - tf),
                vec2(hw, hh - tf),
                vec2(hw, hh),
                vec2(-hw, hh),
            ])
        }
        "IFCTSHAPEPROFILEDEF" => {
            let h = ent.get_f64(3).unwrap_or(150.0) * scale;
            let w = ent.get_f64(4).unwrap_or(150.0) * scale;
            let tw = ent.get_f64(5).unwrap_or(10.0) * scale;
            let tf = ent.get_f64(6).unwrap_or(12.0) * scale;
            let hw = w * 0.5;
            let hh = h * 0.5;
            let htw = tw * 0.5;
            Some(vec![
                vec2(-hw, hh),
                vec2(hw, hh),
                vec2(hw, hh - tf),
                vec2(htw, hh - tf),
                vec2(htw, -hh),
                vec2(-htw, -hh),
                vec2(-htw, hh - tf),
                vec2(-hw, hh - tf),
            ])
        }
        _ => None,
    }
}

/// Extracts points from `IFCPOLYLINE`, `IFCPOLYLOOP`, or `IFCCOMPOSITECURVE`.
fn extract_curve_points(step: &StepFile, curve_id: usize, scale: f64) -> Option<Vec<Vec2>> {
    let ent = step.get(curve_id)?;
    if ent.name == "IFCPOLYLINE" || ent.name == "IFCPOLYLOOP" {
        let pts_list = ent.get_list(0)?;
        let mut pts = Vec::with_capacity(pts_list.len());
        for p_val in pts_list {
            if let Some(p_id) = p_val.as_ref() {
                if let Some(p) = step.resolve_point_2d(p_id, scale) {
                    pts.push(p);
                }
            }
        }
        // Deduplicate consecutive identical points
        pts.dedup_by(|a, b| a.distance(*b) < 1e-4);
        // Strip closing point if identical to first point
        if pts.len() >= 2 {
            if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                if first.distance(*last) < 1e-4 {
                    pts.pop();
                }
            }
        }
        if pts.len() >= 3 {
            return Some(pts);
        }
    } else if ent.name == "IFCTRIMMEDCURVE" {
        if let Some(basis_id) = ent.get_ref(0) {
            if let Some(basis) = step.get(basis_id) {
                if basis.name == "IFCCIRCLE" {
                    let rad = basis.get_f64(1).unwrap_or(200.0) * scale;
                    let (cx, cy) = if let Some(pos_id) = basis.get_ref(0) {
                        step.resolve_point_2d(pos_id, scale).map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0))
                    } else {
                        (0.0, 0.0)
                    };
                    let segs = 16;
                    let pts: Vec<Vec2> = (0..=segs).map(|i| {
                        let a = (i as f64 / segs as f64) * std::f64::consts::TAU;
                        vec2(cx + rad * a.cos(), cy + rad * a.sin())
                    }).collect();
                    return Some(pts);
                } else if basis.name == "IFCLINE" {
                    if let Some(p_id) = basis.get_ref(0) {
                        if let Some(p0) = step.resolve_point_2d(p_id, scale) {
                            if let Some(t_id) = ent.get_ref(2).or_else(|| ent.get_ref(1)) {
                                if let Some(p1) = step.resolve_point_2d(t_id, scale) {
                                    return Some(vec![p0, p1]);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if ent.name == "IFCCOMPOSITECURVE" {
        if let Some(segments) = ent.get_list(0) {
            let mut all_pts = Vec::new();
            for seg_val in segments {
                if let Some(seg_id) = seg_val.as_ref() {
                    if let Some(seg_ent) = step.get(seg_id) {
                        let curve_id = seg_ent.get_ref(1).or_else(|| seg_ent.get_ref(2));
                        if let Some(cid) = curve_id {
                            if let Some(pts) = extract_curve_points(step, cid, scale) {
                                all_pts.extend(pts);
                            }
                        }
                    }
                }
            }
            if all_pts.len() >= 3 {
                return Some(all_pts);
            }
        }
    }
    None
}

/// Extracts geometry from `IFCFACETEDBREP` or `IFCSHELLBASEDSURFACEMODEL`.
fn extract_brep_geometry(
    step: &StepFile,
    brep_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<(Vec<Vec2>, f64, f64)> {
    let ent = step.get(brep_id)?;
    if ent.name != "IFCFACETEDBREP" && ent.name != "IFCSHELLBASEDSURFACEMODEL" {
        return None;
    }

    let shell_id = ent.get_ref(0)?;
    let shell = step.get(shell_id)?;
    let faces_list = shell.get_list(0)?;

    let mut all_world_pts = Vec::new();
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;

    for f_val in faces_list {
        if let Some(f_id) = f_val.as_ref() {
            if let Some(face) = step.get(f_id) {
                if let Some(bounds_list) = face.get_list(0) {
                    for b_val in bounds_list {
                        if let Some(b_id) = b_val.as_ref() {
                            if let Some(bound) = step.get(b_id) {
                                if let Some(loop_id) = bound.get_ref(0) {
                                    if let Some(loop_ent) = step.get(loop_id) {
                                        if let Some(pts_list) = loop_ent.get_list(0) {
                                            for p_val in pts_list {
                                                if let Some(p_id) = p_val.as_ref() {
                                                    if let Some(p) = step.resolve_point_3d(p_id, scale) {
                                                        let wp = world_frame.transform_point(p);
                                                        z_min = z_min.min(wp.z);
                                                        z_max = z_max.max(wp.z);
                                                        all_world_pts.push(vec2(wp.x, wp.y));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if all_world_pts.len() >= 3 && z_min.is_finite() && z_max.is_finite() {
        // Compute 2D convex hull / bounding footprint
        let min_x = all_world_pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = all_world_pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = all_world_pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = all_world_pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let footprint = vec![
            vec2(min_x, min_y),
            vec2(max_x, min_y),
            vec2(max_x, max_y),
            vec2(min_x, max_y),
        ];
        Some((footprint, z_min, z_max))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 3D Mesh Face Extraction (Multi-Item Breps, Triangulated & Polygonal Face Sets)
// ---------------------------------------------------------------------------

/// Recursively extracts all true 3D solid boundary faces from an element's shape representation.
fn extract_all_element_faces(
    step: &StepFile,
    ent: &StepEntity,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Vec<Face> {
    let mut all_faces = Vec::new();
    let Some(rep_id) = ent.get_ref(6) else {
        return all_faces;
    };
    let Some(prod_shape) = step.get(rep_id) else {
        return all_faces;
    };
    if prod_shape.name != "IFCPRODUCTDEFINITIONSHAPE" {
        return all_faces;
    }
    let Some(reps) = prod_shape.get_list(2) else {
        return all_faces;
    };

    for r_val in reps {
        if let Some(r_id) = r_val.as_ref() {
            all_faces.extend(extract_shape_rep_all_faces(step, r_id, world_frame, scale, 4));
        }
    }

    mark_coplanar_edges_soft(&mut all_faces);
    all_faces
}

fn extract_shape_rep_all_faces(
    step: &StepFile,
    rep_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
    depth: usize,
) -> Vec<Face> {
    let mut faces = Vec::new();
    let Some(rep_ent) = step.get(rep_id) else {
        return faces;
    };

    if rep_ent.name == "IFCSHAPEREPRESENTATION" {
        if let Some(items) = rep_ent.get_list(3) {
            for item_val in items {
                if let Some(item_id) = item_val.as_ref() {
                    faces.extend(extract_item_faces(step, item_id, world_frame, scale, depth));
                }
            }
        }
    } else {
        faces.extend(extract_item_faces(step, rep_id, world_frame, scale, depth));
    }

    faces
}

/// Extracts true 3D solid faces (bottom cap, top cap, side quads) for an `IFCEXTRUDEDAREASOLID`.
fn extract_extrusion_3d_faces(
    step: &StepFile,
    solid_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Option<Vec<Face>> {
    let ent = step.get(solid_id)?;
    if ent.name == "IFCBOOLEANCLIPPINGRESULT" || ent.name == "IFCBOOLEANRESULT" {
        if let Some(first_op_id) = ent.get_ref(2) {
            return extract_extrusion_3d_faces(step, first_op_id, world_frame, scale);
        }
    }
    if ent.name != "IFCEXTRUDEDAREASOLID" {
        return None;
    }

    let profile_id = ent.get_ref(0)?;
    let pos_id = ent.get_ref(1);
    let extr_dir_id = ent.get_ref(2);
    let depth = ent.get_f64(3).unwrap_or(3000.0) * scale;

    let extr_frame = pos_id
        .and_then(|id| step.resolve_axis2_placement_3d(id, scale))
        .unwrap_or_else(TransformFrame3D::identity);
    let extr_dir = extr_dir_id
        .and_then(|id| step.resolve_direction_3d(id))
        .unwrap_or(Vec3::Z);

    let combined_frame = world_frame.combine(&extr_frame);
    let local_profile = extract_profile_points(step, profile_id, scale)?;
    let n = local_profile.len();
    if n < 3 {
        return None;
    }

    let mut bot_pts = Vec::with_capacity(n);
    let mut top_pts = Vec::with_capacity(n);
    let extr_vec = extr_dir * depth;

    for p in &local_profile {
        let b = combined_frame.transform_point(Vec3::new(p.x, p.y, 0.0));
        let t = combined_frame.transform_point(Vec3::new(p.x, p.y, 0.0) + extr_vec);
        bot_pts.push(b);
        top_pts.push(t);
    }

    let mut faces = Vec::with_capacity(n + 2);

    // Bottom face (reversed winding for outward normal)
    let mut bot_rev = bot_pts.clone();
    bot_rev.reverse();
    faces.push(Face::new(bot_rev, vec![true; n]));

    // Top face
    faces.push(Face::new(top_pts.clone(), vec![true; n]));

    // Side faces (each side is a quad: b[i], b[next], t[next], t[i])
    for i in 0..n {
        let next = (i + 1) % n;
        let quad = vec![bot_pts[i], bot_pts[next], top_pts[next], top_pts[i]];
        faces.push(Face::new(quad, vec![true; 4]));
    }

    Some(faces)
}

fn extract_item_faces(
    step: &StepFile,
    item_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
    depth: usize,
) -> Vec<Face> {
    let mut faces = Vec::new();
    let Some(item_ent) = step.get(item_id) else {
        return faces;
    };

    match item_ent.name.as_str() {
        "IFCFACETEDBREP" | "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
            faces.extend(extract_brep_faces(step, item_id, world_frame, scale));
        }
        "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => {
            faces.extend(extract_triangulated_face_set_faces(step, item_id, world_frame, scale));
        }
        "IFCSWEPTDISKSOLID" => {
            faces.extend(extract_swept_disk_solid_faces(step, item_id, world_frame, scale));
        }
        "IFCEXTRUDEDAREASOLID" => {
            if let Some(extr_faces) = extract_extrusion_3d_faces(step, item_id, world_frame, scale) {
                faces.extend(extr_faces);
            } else if let Some((boundary, _, _, z_min, z_max)) = extract_extrusion_info(step, item_id, world_frame, scale) {
                faces.extend(Prism::new(boundary, z_min, z_max).faces());
            }
        }
        "IFCBOOLEANCLIPPINGRESULT" | "IFCBOOLEANRESULT" => {
            if let Some(first_op_id) = item_ent.get_ref(2) {
                faces.extend(extract_item_faces(step, first_op_id, world_frame, scale, depth));
            } else if let Some(extr_faces) = extract_extrusion_3d_faces(step, item_id, world_frame, scale) {
                faces.extend(extr_faces);
            } else if let Some((boundary, _, _, z_min, z_max)) = extract_extrusion_info(step, item_id, world_frame, scale) {
                faces.extend(Prism::new(boundary, z_min, z_max).faces());
            }
        }
        "IFCMAPPEDITEM" => {
            if depth > 0 {
                if let (Some(source_id), Some(target_id)) = (item_ent.get_ref(0), item_ent.get_ref(1)) {
                    if let Some(rep_map) = step.get(source_id) {
                        if rep_map.name == "IFCREPRESENTATIONMAP" {
                            let map_origin_id = rep_map.get_ref(0);
                            if let Some(mapped_rep_id) = rep_map.get_ref(1) {
                                let target_frame = step
                                    .resolve_cartesian_transformation_3d(target_id, scale)
                                    .unwrap_or_else(TransformFrame3D::identity);
                                let origin_frame = map_origin_id
                                    .and_then(|id| step.resolve_axis2_placement_3d(id, scale))
                                    .unwrap_or_else(TransformFrame3D::identity);
                                let combined_frame = world_frame.combine(&target_frame).combine(&origin_frame);
                                faces.extend(extract_shape_rep_all_faces(step, mapped_rep_id, &combined_frame, scale, depth - 1));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    faces
}

fn extract_brep_faces(
    step: &StepFile,
    brep_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Vec<Face> {
    let mut faces = Vec::new();
    let Some(ent) = step.get(brep_id) else {
        return faces;
    };

    let shell_ids: Vec<usize> = match ent.name.as_str() {
        "IFCFACETEDBREP" => ent.get_ref(0).into_iter().collect(),
        "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
            ent.get_list(0)
                .map(|list| list.iter().filter_map(|v| v.as_ref()).collect())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    };

    for shell_id in shell_ids {
        if let Some(shell) = step.get(shell_id) {
            if let Some(faces_list) = shell.get_list(0) {
                for f_val in faces_list {
                    if let Some(f_id) = f_val.as_ref() {
                        if let Some(face_ent) = step.get(f_id) {
                            if let Some(bounds_list) = face_ent.get_list(0) {
                                for b_val in bounds_list {
                                    if let Some(b_id) = b_val.as_ref() {
                                        if let Some(bound) = step.get(b_id) {
                                            if let Some(loop_id) = bound.get_ref(0) {
                                                if let Some(loop_ent) = step.get(loop_id) {
                                                    if let Some(pts_list) = loop_ent.get_list(0) {
                                                        let mut pts = Vec::with_capacity(pts_list.len());
                                                        for p_val in pts_list {
                                                            if let Some(p_id) = p_val.as_ref() {
                                                                if let Some(p) = step.resolve_point_3d(p_id, scale) {
                                                                    pts.push(world_frame.transform_point(p));
                                                                }
                                                            }
                                                        }
                                                        if pts.len() >= 3 {
                                                            let n = pts.len();
                                                            faces.push(Face::new(pts, vec![true; n]));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    mark_coplanar_edges_soft(&mut faces);
    faces
}

fn extract_triangulated_face_set_faces(
    step: &StepFile,
    ent_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Vec<Face> {
    let mut faces = Vec::new();
    let Some(ent) = step.get(ent_id) else {
        return faces;
    };
    if ent.name != "IFCTRIANGULATEDFACESET" && ent.name != "IFCPOLYGONALFACESET" {
        return faces;
    }
    let Some(coords_id) = ent.get_ref(0) else {
        return faces;
    };
    let Some(coords_ent) = step.get(coords_id) else {
        return faces;
    };
    if coords_ent.name != "IFCCARTESIANPOINTLIST3D" {
        return faces;
    }
    let Some(points_list) = coords_ent.get_list(0) else {
        return faces;
    };

    let mut world_pts = Vec::with_capacity(points_list.len());
    for pt_val in points_list {
        if let StepValue::List(xyz) = pt_val {
            if xyz.len() >= 3 {
                let x = xyz[0].as_f64().unwrap_or(0.0) * scale;
                let y = xyz[1].as_f64().unwrap_or(0.0) * scale;
                let z = xyz[2].as_f64().unwrap_or(0.0) * scale;
                world_pts.push(world_frame.transform_point(Vec3::new(x, y, z)));
            }
        }
    }

    let coord_index_list = ent.get_list(1).or_else(|| ent.get_list(2)).or_else(|| ent.get_list(3));
    if let Some(idx_list) = coord_index_list {
        for f_val in idx_list {
            if let StepValue::List(indices) = f_val {
                let mut tri_pts = Vec::with_capacity(indices.len());
                for idx_val in indices {
                    if let Some(idx) = idx_val.as_usize() {
                        if idx > 0 {
                            let i = idx - 1;
                            if i < world_pts.len() {
                                tri_pts.push(world_pts[i]);
                            }
                        }
                    }
                }
                if tri_pts.len() >= 3 {
                    let n = tri_pts.len();
                    faces.push(Face::new(tri_pts, vec![true; n]));
                }
            }
        }
    }

    mark_coplanar_edges_soft(&mut faces);
    faces
}

/// Marks internal coplanar diagonal chords as soft (`hard = false`) so triangulated meshes
/// (e.g. flat roofs, slabs, walls) render as clean monolithic surfaces without spiderweb lines.
fn mark_coplanar_edges_soft(faces: &mut [Face]) {
    use std::collections::HashMap;

    // Quantize 3D coordinates to 0.1mm grid
    fn quantize(p: Vec3) -> (i64, i64, i64) {
        ((p.x * 10.0).round() as i64, (p.y * 10.0).round() as i64, (p.z * 10.0).round() as i64)
    }

    type EdgeKey = ((i64, i64, i64), (i64, i64, i64));
    let mut edge_map: HashMap<EdgeKey, Vec<(usize, usize, Vec3)>> = HashMap::new();

    for (f_idx, face) in faces.iter().enumerate() {
        let n = face.pts.len();
        if n < 3 {
            continue;
        }
        let norm = face.normal;
        for i in 0..n {
            let next = (i + 1) % n;
            let q0 = quantize(face.pts[i]);
            let q1 = quantize(face.pts[next]);
            if q0 == q1 {
                continue;
            }
            let key = if q0 < q1 { (q0, q1) } else { (q1, q0) };
            edge_map.entry(key).or_default().push((f_idx, i, norm));
        }
    }

    for occurrences in edge_map.values() {
        if occurrences.len() == 2 {
            let (f0, e0, n0) = occurrences[0];
            let (f1, e1, n1) = occurrences[1];
            // If the two adjacent faces are coplanar (angle < 10 degrees, dot > 0.985):
            if n0.dot(n1) > 0.985 {
                if let Some(face0) = faces.get_mut(f0) {
                    if let Some(h) = face0.hard.get_mut(e0) {
                        *h = false;
                    }
                }
                if let Some(face1) = faces.get_mut(f1) {
                    if let Some(h) = face1.hard.get_mut(e1) {
                        *h = false;
                    }
                }
            }
        }
    }
}

fn extract_3d_curve_points(
    step: &StepFile,
    curve_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Vec<Vec3> {
    let mut pts = Vec::new();
    let Some(ent) = step.get(curve_id) else {
        return pts;
    };

    match ent.name.as_str() {
        "IFCPOLYLINE" | "IFCPOLYLOOP" => {
            if let Some(pts_list) = ent.get_list(0) {
                for p_val in pts_list {
                    if let Some(p_id) = p_val.as_ref() {
                        if let Some(p) = step.resolve_point_3d(p_id, scale) {
                            pts.push(world_frame.transform_point(p));
                        }
                    }
                }
            }
        }
        "IFCTRIMMEDCURVE" => {
            if let Some(basis_id) = ent.get_ref(0) {
                if let Some(basis) = step.get(basis_id) {
                    if basis.name == "IFCLINE" {
                        if let Some(p_id) = basis.get_ref(0) {
                            if let Some(p) = step.resolve_point_3d(p_id, scale) {
                                pts.push(world_frame.transform_point(p));
                            }
                        }
                        if let Some(t2) = ent.get_ref(2).or_else(|| ent.get_ref(1)) {
                            if let Some(p) = step.resolve_point_3d(t2, scale) {
                                pts.push(world_frame.transform_point(p));
                            }
                        }
                    } else if basis.name == "IFCCIRCLE" {
                        let rad = basis.get_f64(1).unwrap_or(100.0) * scale;
                        let pos_id = basis.get_ref(0);
                        let circle_frame = pos_id
                            .and_then(|id| step.resolve_axis2_placement_3d(id, scale))
                            .unwrap_or_else(TransformFrame3D::identity);
                        let combined = world_frame.combine(&circle_frame);
                        let segs = 16;
                        for i in 0..=segs {
                            let theta = (i as f64 / segs as f64) * std::f64::consts::TAU;
                            let p = combined.transform_point(Vec3::new(rad * theta.cos(), rad * theta.sin(), 0.0));
                            pts.push(p);
                        }
                    }
                }
            }
        }
        "IFCCOMPOSITECURVE" => {
            if let Some(segments) = ent.get_list(0) {
                for seg_val in segments {
                    if let Some(seg_id) = seg_val.as_ref() {
                        if let Some(seg_ent) = step.get(seg_id) {
                            let cid = seg_ent.get_ref(1).or_else(|| seg_ent.get_ref(2));
                            if let Some(cid) = cid {
                                pts.extend(extract_3d_curve_points(step, cid, world_frame, scale));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    pts.dedup_by(|a, b| (*a - *b).length() < 1.0);
    pts
}

/// Generates watertight 8-sided solid cylindrical tubes for `IFCSWEPTDISKSOLID`
fn extract_swept_disk_solid_faces(
    step: &StepFile,
    solid_id: usize,
    world_frame: &TransformFrame3D,
    scale: f64,
) -> Vec<Face> {
    let mut faces = Vec::new();
    let Some(ent) = step.get(solid_id) else {
        return faces;
    };
    if ent.name != "IFCSWEPTDISKSOLID" {
        return faces;
    }

    let Some(directrix_id) = ent.get_ref(0) else {
        return faces;
    };
    let radius = ent.get_f64(1).unwrap_or(25.0) * scale;
    let curve_pts = extract_3d_curve_points(step, directrix_id, world_frame, scale);
    if curve_pts.len() < 2 {
        return faces;
    }

    let sides = 8;
    for pair in curve_pts.windows(2) {
        let p0 = pair[0];
        let p1 = pair[1];
        let dir = p1 - p0;
        let len = dir.length();
        if len < 1.0 {
            continue;
        }
        let u_z = dir / len;
        let ref_v = if u_z.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
        let u_x = u_z.cross(ref_v).normalized();
        let u_y = u_z.cross(u_x).normalized();

        let ring0: Vec<Vec3> = (0..sides)
            .map(|k| {
                let a = (k as f64 / sides as f64) * std::f64::consts::TAU;
                p0 + u_x * (radius * a.cos()) + u_y * (radius * a.sin())
            })
            .collect();
        let ring1: Vec<Vec3> = (0..sides)
            .map(|k| {
                let a = (k as f64 / sides as f64) * std::f64::consts::TAU;
                p1 + u_x * (radius * a.cos()) + u_y * (radius * a.sin())
            })
            .collect();

        // Side quads
        for i in 0..sides {
            let next = (i + 1) % sides;
            let quad = vec![ring0[i], ring0[next], ring1[next], ring1[i]];
            faces.push(Face::new(quad, vec![true, true, true, true]));
        }
        // Start cap (reversed)
        let mut start_cap = ring0.clone();
        start_cap.reverse();
        faces.push(Face::new(start_cap, vec![true; sides]));
        // End cap
        faces.push(Face::new(ring1, vec![true; sides]));
    }

    mark_coplanar_edges_soft(&mut faces);
    faces
}

// ---------------------------------------------------------------------------
// Wall Hosting Linker
// ---------------------------------------------------------------------------

fn find_host_wall(
    elem_id: usize,
    elem_pos: Vec2,
    door_win_to_wall: &BTreeMap<usize, usize>,
    opening_to_wall: &BTreeMap<usize, usize>,
    wall_candidates: &BTreeMap<usize, (ElementId, Vec2, Vec2, f64, f64, LevelId)>,
) -> Option<(ElementId, f64, LevelId, f64)> {
    // 1. Direct explicit relational link
    if let Some(w_step_id) = door_win_to_wall.get(&elem_id).or_else(|| opening_to_wall.get(&elem_id)) {
        if let Some((w_id, start, end, _, _, lvl_id)) = wall_candidates.get(w_step_id) {
            let wall_len = start.distance(*end);
            let offset = (elem_pos - *start).dot((*end - *start).normalized()).max(0.0);
            return Some((*w_id, offset, *lvl_id, wall_len));
        }
    }

    // 2. Proximity fallback: search for closest wall within 400 mm
    let mut closest_wall: Option<(ElementId, f64, LevelId, f64, f64)> = None;
    for (w_id, start, end, thick, _, lvl_id) in wall_candidates.values() {
        let (proj, dist) = plinth_geom::polygon::closest_point_on_segment(elem_pos, *start, *end);
        let offset = proj.distance(*start);
        let wall_len = start.distance(*end);
        if dist <= thick * 0.75 + 200.0 {
            if closest_wall.as_ref().map_or(true, |(_, _, _, _, d)| dist < *d) {
                closest_wall = Some((*w_id, offset, *lvl_id, wall_len, dist));
            }
        }
    }

    closest_wall.map(|(w_id, off, lvl_id, wall_len, _)| (w_id, off, lvl_id, wall_len))
}

// ---------------------------------------------------------------------------
// View & Camera Setup for Imported Models
// ---------------------------------------------------------------------------

fn setup_views_for_imported_model(doc: &mut Document) {
    // 1. Calculate full 3D bounding box of imported elements
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;

    for el in doc.elements.values() {
        match el {
            Element::Wall(w) => {
                for p in w.footprint() {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let base_z = doc.level_elevation(w.base_level);
                let (z0, z1) = w.z_bounds(base_z, w.top_level.map(|l| doc.level_elevation(l)));
                min_z = min_z.min(z0);
                max_z = max_z.max(z1);
            }
            Element::Column(c) => {
                for p in c.footprint() {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let base_z = doc.level_elevation(c.base_level);
                let (z0, z1) = c.z_bounds(base_z, c.top_level.map(|l| doc.level_elevation(l)));
                min_z = min_z.min(z0);
                max_z = max_z.max(z1);
            }
            Element::Slab(s) => {
                for p in &s.boundary {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let z = doc.level_elevation(s.level);
                min_z = min_z.min(z - s.thickness);
                max_z = max_z.max(z);
            }
            Element::FreeformSolid(f) => {
                for p in &f.boundary {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                min_z = min_z.min(f.z_min);
                max_z = max_z.max(f.z_max);
            }
            _ => {}
        }
    }

    if !min_x.is_finite() {
        min_x = 0.0;
        max_x = 10000.0;
        min_y = 0.0;
        max_y = 8000.0;
        min_z = 0.0;
        max_z = 3500.0;
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let span_x = (max_x - min_x).max(1000.0);
    let span_y = (max_y - min_y).max(1000.0);
    let span_z = (max_z - min_z).max(1000.0);
    let diameter = (span_x * span_x + span_y * span_y + span_z * span_z).sqrt();
    let cam_distance = (diameter * 1.6).clamp(8000.0, 150000.0);

    // Update 3D View Camera
    let v_3d_id = if let Some(v_3d) = doc.views.values_mut().find(|v| v.kind == ViewKind::ThreeD) {
        let mut cam = Camera3D::default();
        cam.target_x = center_x;
        cam.target_y = center_y;
        cam.target_z = center_z;
        cam.distance = cam_distance;
        cam.yaw = -std::f64::consts::FRAC_PI_4;
        cam.pitch = 0.55;
        v_3d.camera_3d = Some(cam);
        v_3d.id
    } else {
        let mut v_3d = ViewDef::new_3d("{3D}");
        let mut cam = Camera3D::default();
        cam.target_x = center_x;
        cam.target_y = center_y;
        cam.target_z = center_z;
        cam.distance = cam_distance;
        cam.yaw = -std::f64::consts::FRAC_PI_4;
        cam.pitch = 0.55;
        v_3d.camera_3d = Some(cam);
        let id = v_3d.id;
        doc.views.insert(id, v_3d);
        id
    };
    doc.active_view_id = v_3d_id;

    // Ensure standard floor plans and elevations exist for imported levels
    if !doc.views.values().any(|v| v.kind == ViewKind::ElevationSouth) {
        let south = ViewDef::new_elevation("Elevation: South", ViewKind::ElevationSouth);
        doc.views.insert(south.id, south);
    }
    if !doc.views.values().any(|v| v.kind == ViewKind::ElevationNorth) {
        let north = ViewDef::new_elevation("Elevation: North", ViewKind::ElevationNorth);
        doc.views.insert(north.id, north);
    }
    if !doc.views.values().any(|v| v.kind == ViewKind::ElevationEast) {
        let east = ViewDef::new_elevation("Elevation: East", ViewKind::ElevationEast);
        doc.views.insert(east.id, east);
    }
    if !doc.views.values().any(|v| v.kind == ViewKind::ElevationWest) {
        let west = ViewDef::new_elevation("Elevation: West", ViewKind::ElevationWest);
        doc.views.insert(west.id, west);
    }

    for lvl in doc.levels.values() {
        let plan_name = format!("Floor Plan: {}", lvl.name);
        if !doc.views.values().any(|v| v.name == plan_name) {
            let plan = ViewDef::new_floor_plan(&plan_name, lvl.id);
            doc.views.insert(plan.id, plan);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_sample_building_export_and_import() {
        let sample_doc = Document::create_sample_building();
        let step_text = crate::export_ifc4(&sample_doc);

        let parsed_doc = parse_ifc(&step_text).expect("Should parse exported IFC4 file");
        assert!(!parsed_doc.elements.is_empty(), "Parsed elements should not be empty");
        assert!(!parsed_doc.levels.is_empty(), "Parsed levels should not be empty");

        // Verify elements are real (not hardcoded dummy geometry)
        let wall_count = parsed_doc.elements.values().filter(|e| matches!(e, Element::Wall(_))).count();
        let column_count = parsed_doc.elements.values().filter(|e| matches!(e, Element::Column(_))).count();
        let slab_count = parsed_doc.elements.values().filter(|e| matches!(e, Element::Slab(_))).count();

        assert!(wall_count >= 4, "Should have parsed exterior walls (got {wall_count})");
        assert!(column_count >= 1, "Should have parsed structural columns (got {column_count})");
        assert!(slab_count >= 1, "Should have parsed foundation/roof slabs (got {slab_count})");
    }

    #[test]
    fn test_parse_ifc2x3_compatibility() {
        let ifc2x3_sample = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');
FILE_NAME('sample_2x3.ifc','2026-10-31T00:00:00',('Architect'),('Revit'),'Autodesk Revit','Revit','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCDIRECTION((0.,0.,1.));
#3=IFCDIRECTION((1.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#1,#2,#3);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCUNITASSIGNMENT((#5));
#7=IFCBUILDINGSTOREY('2q_zB3v798BfHqG3d7l$4k',$,'Level 1',$,$,#4,$,$,.ELEMENT.,0.);
#8=IFCLOCALPLACEMENT($,#4);
#9=IFCCARTESIANPOINT((0.,0.));
#10=IFCCARTESIANPOINT((6.,0.));
#11=IFCCARTESIANPOINT((6.,0.2));
#12=IFCCARTESIANPOINT((0.,0.2));
#13=IFCPOLYLOOP((#9,#10,#11,#12));
#14=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#13);
#15=IFCEXTRUDEDAREASOLID(#14,#4,#2,3.);
#16=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#15));
#17=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#18=IFCWALLSTANDARDCASE('1tWf4$Ocf7qwbK84Rk$7_Y',$,'Exterior Brick Wall',$,$,#8,#17,$);
#19=IFCRELCONTAINEDINSPATIALSTRUCTURE('0$rVbA93f8ePsm41W$q81a',$,'Containment',$,(#18),#7);
ENDSEC;
END-10303-21;
";
        let parsed = parse_ifc(ifc2x3_sample).expect("Should parse IFC2X3 file");
        assert_eq!(parsed.levels.len(), 1);
        assert_eq!(parsed.elements.len(), 1);

        let elem = parsed.elements.values().next().unwrap();
        match elem {
            Element::Wall(w) => {
                assert_eq!(w.name, "Exterior Brick Wall");
                // Unit conversion from metres: length should be ~6000 mm, thickness ~200 mm, height ~3000 mm
                assert!((w.length() - 6000.0).abs() < 50.0, "Length expected ~6000mm, got {}", w.length());
                assert!((w.thickness - 200.0).abs() < 10.0, "Thickness expected ~200mm, got {}", w.thickness);
                assert!((w.unconnected_height - 3000.0).abs() < 10.0, "Height expected ~3000mm, got {}", w.unconnected_height);
            }
            other => panic!("Expected Wall, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ifc4x3_and_multiline_comments() {
        let ifc4x3_sample = "\
ISO-10303-21;
HEADER;
/* Multi-line header comment explaining schema */
FILE_DESCRIPTION(('ViewDefinition [Infrastructure_V1.0]'),'2;1');
FILE_NAME('bridge_pier.ifc','2026-10-31T00:00:00',('Engineer'),('Arch3D'),'Arch3D Engine','Arch3D','');
FILE_SCHEMA(('IFC4X3'));
ENDSEC;
DATA;
/* Base placement and units in millimetres */
#1 = IFCCARTESIANPOINT((0.0, 0.0, 0.0));
#2 = IFCDIRECTION((0.0, 0.0, 1.0));
#3 = IFCDIRECTION((1.0, 0.0, 0.0));
#4 = IFCAXIS2PLACEMENT3D(#1, #2, #3);
#5 = IFCSIUNIT(*, .LENGTHUNIT., .MILLI., .METRE.);
#6 = IFCUNITASSIGNMENT((#5));

#7 = IFCBUILDINGSTOREY('3aB9$1234567890abcdefg', $, 'Foundation Level', $, $, #4, $, $, .ELEMENT., 0.0);

/* Structural Column with Rectangle Profile */
#10 = IFCRECTANGLEPROFILEDEF(.AREA., 'Pier Column Profile', $, 600.0, 600.0);
#11 = IFCEXTRUDEDAREASOLID(#10, #4, #2, 4500.0);
#12 = IFCSHAPEREPRESENTATION($, 'Body', 'SweptSolid', (#11));
#13 = IFCPRODUCTDEFINITIONSHAPE($, $, (#12));
#14 = IFCCOLUMN('1234567890abcdefghijkl', $, 'Pier Column C1', $, $, #4, #13, $);

/* Structural Slab Foundation */
#20 = IFCRECTANGLEPROFILEDEF(.AREA., 'Footing Slab Profile', $, 4000.0, 4000.0);
#21 = IFCEXTRUDEDAREASOLID(#20, #4, #2, 800.0);
#22 = IFCSHAPEREPRESENTATION($, 'Body', 'SweptSolid', (#21));
#23 = IFCPRODUCTDEFINITIONSHAPE($, $, (#22));
#24 = IFCSLAB('abcdefghijklmnopqrstuv', $, 'Footing Slab F1', $, $, #4, #23, $, .BASESLAB.);

#30 = IFCRELCONTAINEDINSPATIALSTRUCTURE('containment_rel_guid_12', $, 'SpatialStructure', $, (#14, #24), #7);
ENDSEC;
END-10303-21;
";
        let parsed = parse_ifc(ifc4x3_sample).expect("Should parse IFC4X3 file");
        assert_eq!(parsed.levels.len(), 1);
        assert_eq!(parsed.elements.len(), 2);

        let col = parsed.elements.values().find(|e| matches!(e, Element::Column(_))).expect("Column should exist");
        if let Element::Column(c) = col {
            assert_eq!(c.name, "Pier Column C1");
            assert!((c.width - 600.0).abs() < 5.0);
            assert!((c.depth - 600.0).abs() < 5.0);
            assert!((c.unconnected_height - 4500.0).abs() < 5.0);
        }

        let slab = parsed.elements.values().find(|e| matches!(e, Element::Slab(_))).expect("Slab should exist");
        if let Element::Slab(s) = slab {
            assert_eq!(s.name, "Footing Slab F1");
            assert!((s.thickness - 800.0).abs() < 5.0);
        }
    }

    #[test]
    fn test_parse_imperial_feet_units() {
        let imperial_sample = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');
FILE_NAME('imperial.ifc','2026-10-31T00:00:00',('Architect'),('Arch3D'),'Arch3D','Arch3D','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCCARTESIANPOINT((0.,0.,0.));
#2 = IFCDIRECTION((0.,0.,1.));
#3 = IFCDIRECTION((1.,0.,0.));
#4 = IFCAXIS2PLACEMENT3D(#1,#2,#3);
#5 = IFCCONVERSIONBASEDUNIT(*, .LENGTHUNIT., 'FOOT', #1);
#6 = IFCUNITASSIGNMENT((#5));

#7 = IFCBUILDINGSTOREY('storey_guid_1234567890', $, 'First Floor', $, $, #4, $, $, .ELEMENT., 0.0);

/* Wall 20 feet long by 10 feet high */
#10 = IFCCARTESIANPOINT((0., 0.));
#11 = IFCCARTESIANPOINT((20., 0.));
#12 = IFCCARTESIANPOINT((20., 0.656));
#13 = IFCCARTESIANPOINT((0., 0.656));
#14 = IFCPOLYLOOP((#10,#11,#12,#13));
#15 = IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#14);
#16 = IFCEXTRUDEDAREASOLID(#15,#4,#2,10.);
#17 = IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#16));
#18 = IFCPRODUCTDEFINITIONSHAPE($,$,(#17));
#19 = IFCWALL('wall_guid_12345678901234',$,'Imperial 20ft Wall',$,$,#4,#18,$);
ENDSEC;
END-10303-21;
";
        let parsed = parse_ifc(imperial_sample).expect("Should parse Imperial IFC file");
        let wall_elem = parsed.elements.values().next().expect("Wall should exist");
        if let Element::Wall(w) = wall_elem {
            assert_eq!(w.name, "Imperial 20ft Wall");
            // 20 feet = 6096 mm, 10 feet = 3048 mm
            assert!((w.length() - 6096.0).abs() < 100.0, "Expected length ~6096mm, got {}", w.length());
            assert!((w.unconnected_height - 3048.0).abs() < 50.0, "Expected height ~3048mm, got {}", w.unconnected_height);
        } else {
            panic!("Expected Wall element");
        }
    }

    #[test]
    fn test_sgd_bodo_inspect() {
        let path = std::path::Path::new(r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\SGD_BODO_Arch-1.ifc");
        if !path.exists() {
            println!("SGD_BODO_Arch-1.ifc does not exist");
            return;
        }
        let content = std::fs::read_to_string(path).expect("Failed to read");
        let doc = parse_ifc(&content).expect("Failed to parse");
        println!("DOC ELEMENTS COUNT: {}", doc.elements.len());
        println!("LEVELS:");
        for (_lid, lvl) in &doc.levels {
            println!("  Level: id={:?}, name='{}', elev={:.1}", lvl.id, lvl.name, lvl.elevation);
        }
        for (id, el) in &doc.elements {
            match el {
                Element::FreeformSolid(f) => {
                    let min_x = f.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                    let max_x = f.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                    let min_y = f.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                    let max_y = f.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                    let bb_dx = max_x - min_x;
                    let bb_dy = max_y - min_y;
                    let bb_dz = f.z_max - f.z_min;
                    if bb_dx > 40000.0 || bb_dy > 40000.0 {
                        println!("LARGE FREEFORM: id={:?}, name='{}', group={:?}, ifc_cat={:?}, bb=({:.1}x{:.1}x{:.1})",
                            id, f.name, f.group_name, f.ifc_category, bb_dx, bb_dy, bb_dz);
                    }
                }
                Element::Slab(s) => {
                    let min_x = s.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                    let max_x = s.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                    let min_y = s.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                    let max_y = s.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                    let dx = max_x - min_x;
                    let dy = max_y - min_y;
                    if dx > 40000.0 || dy > 40000.0 {
                        println!("LARGE SLAB: id={:?}, name='{}', level={:?}, thick={:.1}, offset={:.1}, bb=({:.1}x{:.1}), min=({:.1},{:.1}), max=({:.1},{:.1})",
                            id, s.name, s.level, s.thickness, s.offset, dx, dy, min_x, min_y, max_x, max_y);
                    }
                }
                _ => {}
            }
        }
        // Count walls per level
        let mut walls_per_level = std::collections::BTreeMap::new();
        for el in doc.elements.values() {
            if let Element::Wall(w) = el {
                let lvl_name = doc.levels.get(&w.base_level).map(|l| l.name.as_str()).unwrap_or("Unknown");
                *walls_per_level.entry(lvl_name.to_string()).or_insert(0) += 1;
            }
        }
        assert!(doc.elements.len() > 2000);
    }

    #[test]
    fn test_parse_triangulated_face_set() {
        let ifc_mesh_sample = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [DesignTransferView_V1.0]'),'2;1');
FILE_NAME('mesh_building.ifc','2026-10-31T00:00:00',('Architect'),('Arch3D'),'Arch3D','Arch3D','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCCARTESIANPOINT((0.,0.,0.));
#2 = IFCDIRECTION((0.,0.,1.));
#3 = IFCDIRECTION((1.,0.,0.));
#4 = IFCAXIS2PLACEMENT3D(#1,#2,#3);
#5 = IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#6 = IFCUNITASSIGNMENT((#5));

#7 = IFCBUILDINGSTOREY('storey_guid_mesh_01', $, 'Mezzanine', $, $, #4, $, $, .ELEMENT., 0.0);

/* Mesh vertices in millimetres */
#10 = IFCCARTESIANPOINTLIST3D(((0.,0.,0.), (2500.,0.,0.), (2500.,1800.,0.), (0.,1800.,0.), (0.,0.,1200.), (2500.,0.,1200.), (2500.,1800.,1200.), (0.,1800.,1200.)));
#11 = IFCTRIANGULATEDFACESET(#10, $, .T., ((1,2,3), (1,3,4), (5,7,6), (5,8,7)), $);
#12 = IFCSHAPEREPRESENTATION($, 'Body', 'Tessellation', (#11));
#13 = IFCPRODUCTDEFINITIONSHAPE($, $, (#12));
#14 = IFCBUILDINGELEMENTPROXY('mesh_proxy_guid_01', $, 'Equipment Platform', $, $, #4, #13, $, .NOTDEFINED.);

#20 = IFCRELCONTAINEDINSPATIALSTRUCTURE('rel_spatial_mesh_01', $, 'SpatialStructure', $, (#14), #7);
ENDSEC;
END-10303-21;
";
        let parsed = parse_ifc(ifc_mesh_sample).expect("Should parse IFC4 TriangulatedFaceSet");
        assert_eq!(parsed.levels.len(), 1);
        assert_eq!(parsed.elements.len(), 1);

        let solid_elem = parsed.elements.values().next().expect("Solid element should exist");
        if let Element::FreeformSolid(s) = solid_elem {
            assert_eq!(s.name, "Equipment Platform");
            assert_eq!(s.z_min, 0.0);
            assert_eq!(s.z_max, 1200.0);
            assert_eq!(s.boundary.len(), 4);
        } else {
            panic!("Expected FreeformSolid for TriangulatedFaceSet mesh");
        }
    }

    #[test]
    fn test_parse_mapped_item_family_instance() {
        let ifc_mapped_sample = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');
FILE_NAME('mapped_door.ifc','2026-10-31T00:00:00',('Architect'),('Arch3D'),'Arch3D','Arch3D','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1 = IFCCARTESIANPOINT((0.,0.,0.));
#2 = IFCDIRECTION((0.,0.,1.));
#3 = IFCDIRECTION((1.,0.,0.));
#4 = IFCAXIS2PLACEMENT3D(#1,#2,#3);
#5 = IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#6 = IFCUNITASSIGNMENT((#5));

#7 = IFCBUILDINGSTOREY('storey_guid_mapped_01', $, 'Ground', $, $, #4, $, $, .ELEMENT., 0.0);

/* Master Door Geometry in RepresentationMap */
#10 = IFCRECTANGLEPROFILEDEF(.AREA., 'Standard Door Leaf Profile', $, 900.0, 60.0);
#11 = IFCEXTRUDEDAREASOLID(#10, #4, #2, 2100.0);
#12 = IFCSHAPEREPRESENTATION($, 'Body', 'SweptSolid', (#11));
#13 = IFCREPRESENTATIONMAP(#4, #12);

/* Instance 1: Mapped with translation offset */
#20 = IFCCARTESIANPOINT((1500., 200., 0.));
#21 = IFCCARTESIANTRANSFORMATIONOPERATOR3D(#3, $, #20, 1.0, #2);
#22 = IFCMAPPEDITEM(#13, #21);
#23 = IFCSHAPEREPRESENTATION($, 'Body', 'MappedRepresentation', (#22));
#24 = IFCPRODUCTDEFINITIONSHAPE($, $, (#23));
#25 = IFCDOOR('door_inst_01', $, 'Entrance Door 900x2100', $, $, #4, #24, $, 2100., 900.);

#30 = IFCRELCONTAINEDINSPATIALSTRUCTURE('rel_spatial_mapped_01', $, 'SpatialStructure', $, (#25), #7);
ENDSEC;
END-10303-21;
";
        let parsed = parse_ifc(ifc_mapped_sample).expect("Should parse MappedItem geometry");
        assert_eq!(parsed.levels.len(), 1);
        assert_eq!(parsed.elements.len(), 1);

        let door_elem = parsed.elements.values().next().expect("Door element should exist");
        match door_elem {
            Element::Door(d) => {
                assert_eq!(d.name, "Entrance Door 900x2100");
                assert_eq!(d.width, 900.0);
                assert_eq!(d.height, 2100.0);
            }
            Element::FreeformSolid(s) => {
                assert_eq!(s.name, "Entrance Door 900x2100");
                assert_eq!(s.z_max, 2100.0);
            }
            other => panic!("Expected Door or FreeformSolid, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_user_school_building() {
        let path = std::path::Path::new(r"c:\Users\redwood-ph-pc\Desktop\OSA files\TEST BIMCRAFT IMPORT\2 Storey School Building.ifc");
        if path.exists() {
            let content = std::fs::read_to_string(path).expect("Failed to read test file");
            let result = parse_ifc(&content);
            match result {
                Ok(doc) => {
                    println!("Successfully parsed user school building! Elements: {}, Levels: {}", doc.elements.len(), doc.levels.len());
                    for (lvl_id, lvl) in &doc.levels {
                        println!("Level: name='{}', elev={:.1}, id={:?}", lvl.name, lvl.elevation, lvl_id);
                    }
                    for el in doc.elements.values() {
                        match el {
                            Element::Door(d) => {
                                println!("DOOR: name='{}', host_wall={:?}, offset={:.1}, w={:.1}, h={:.1}, sill={:.1}",
                                    d.name, d.host_wall, d.offset_along_wall, d.width, d.height, d.sill_height);
                            }
                            Element::Window(w) => {
                                println!("WINDOW: name='{}', host_wall={:?}, offset={:.1}, w={:.1}, h={:.1}, sill={:.1}",
                                    w.name, w.host_wall, w.offset_along_wall, w.width, w.height, w.sill_height);
                            }
                            Element::Column(c) => {
                                println!("COLUMN: name='{}', loc=({:.1},{:.1}), w={:.1}, d={:.1}, base_off={:.1}, h={:.1}",
                                    c.name, c.location.x, c.location.y, c.width, c.depth, c.base_offset, c.unconnected_height);
                            }
                            Element::Slab(s) => {
                                println!("SLAB: name='{}', thick={:.1}, offset={:.1}, bounds_count={}",
                                    s.name, s.thickness, s.offset, s.boundary.len());
                            }
                            Element::Beam(b) => {
                                println!("BEAM: name='{}', start=({:.1},{:.1}), end=({:.1},{:.1}), w={:.1}, d={:.1}, z_off={:.1}",
                                    b.name, b.start.x, b.start.y, b.end.x, b.end.y, b.width, b.depth, b.z_offset);
                            }
                            Element::Roof(r) => {
                                println!("ROOF: name='{}', shape={:?}, bounds={}",
                                    r.name, r.shape, r.boundary.len());
                            }
                            Element::Ceiling(c) => {
                                println!("CEILING: name='{}', height_above_level={:.1}", c.name, c.height_above_level);
                            }
                            Element::FreeformSolid(f) => {
                                println!("FREEFORM_SOLID: name='{}', group={:?}, faces={}",
                                    f.name, f.group_name, f.faces().len());
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to parse: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_parse_revit_residential_model() {
        let path = std::path::Path::new(r"c:\Users\redwood-ph-pc\Desktop\OSA files\TEST BIMCRAFT IMPORT\3 Bedroom Residential + Loft-IFC.ifc");
        if path.exists() {
            let content = std::fs::read_to_string(path).expect("Failed to read test file");
            let result = parse_ifc(&content);
            match result {
                Ok(doc) => {
                    println!("=== REVIT RESIDENTIAL MODEL PARSE SUCCESS ===");
                    println!("Total Elements: {}, Total Levels: {}", doc.elements.len(), doc.levels.len());
                    for (lvl_id, lvl) in &doc.levels {
                        println!("Level: name='{}', elev={:.1}, id={:?}", lvl.name, lvl.elevation, lvl_id);
                    }
                    let mut counts = std::collections::BTreeMap::new();
                    for el in doc.elements.values() {
                        let k = match el {
                            Element::Wall(_) => "Wall",
                            Element::Beam(_) => "Beam",
                            Element::Column(_) => "Column",
                            Element::Slab(_) => "Slab",
                            Element::Roof(_) => "Roof",
                            Element::Door(_) => "Door",
                            Element::Window(_) => "Window",
                            Element::Ceiling(_) => "Ceiling",
                            Element::FreeformSolid(f) => {
                                if let Some(g) = &f.group_name {
                                    println!("Freeform group: '{}', name: '{}', faces: {}", g, f.name, f.faces().len());
                                }
                                "FreeformSolid"
                            }
                            _ => "Other",
                        };
                        *counts.entry(k).or_insert(0) += 1;
                    }
                    for (k, v) in counts {
                        println!("Imported {}: {}", k, v);
                    }
                    for el in doc.elements.values() {
                        match el {
                            Element::Beam(b) => {
                                println!("BEAM: name='{}', start=({:.1},{:.1}), end=({:.1},{:.1}), w={:.1}, d={:.1}, z_off={:.1}",
                                    b.name, b.start.x, b.start.y, b.end.x, b.end.y, b.width, b.depth, b.z_offset);
                            }
                            Element::Roof(r) => {
                                println!("ROOF: name='{}', shape={:?}, bounds={}, slope={:.1}, off={:.1}",
                                    r.name, r.shape, r.boundary.len(), r.slope_degrees, r.offset);
                            }
                            Element::Door(d) => {
                                println!("DOOR: name='{}', host_wall={:?}, offset={:.1}, w={:.1}, h={:.1}, sill={:.1}",
                                    d.name, d.host_wall, d.offset_along_wall, d.width, d.height, d.sill_height);
                            }
                            Element::Window(w) => {
                                println!("WINDOW: name='{}', host_wall={:?}, offset={:.1}, w={:.1}, h={:.1}, sill={:.1}",
                                    w.name, w.host_wall, w.offset_along_wall, w.width, w.height, w.sill_height);
                            }
                            Element::Column(c) => {
                                println!("COLUMN: name='{}', loc=({:.1},{:.1}), w={:.1}, d={:.1}, base_off={:.1}, h={:.1}",
                                    c.name, c.location.x, c.location.y, c.width, c.depth, c.base_offset, c.unconnected_height);
                            }
                            Element::Slab(s) => {
                                println!("SLAB: name='{}', thick={:.1}, offset={:.1}, bounds={}",
                                    s.name, s.thickness, s.offset, s.boundary.len());
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to parse: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_parse_benchmark_ifc_files() {
        let benchmarks = [
            ("KIT_Institute_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\KIT_Institute_Arch.ifc"),
            ("KIT_Smiley-West_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\KIT_Smiley-West_Arch.ifc"),
            ("Academic_Autodesk-BasicSampleProject_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\Academic_Autodesk-BasicSampleProject_Arch.ifc"),
            ("NVW_DCR-LOD100_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\NVW_DCR-LOD100_Arch.ifc"),
            ("NVW_DCR-LOD200_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\NVW_DCR-LOD200_Arch.ifc"),
            ("NBU_MedicalClinic_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\NBU_MedicalClinic_Arch.ifc"),
            ("SGD_BODO_Arch-1.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\SGD_BODO_Arch-1.ifc"),
            ("LTU_A-House_K-modell.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\LTU_A-House_K-modell.ifc"),
            ("Academic_Autodesk-AdvancedSampleProject_Arch.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\Academic_Autodesk-AdvancedSampleProject_Arch.ifc"),
            ("LTU_A-House_Air.ifc", r"C:\Users\redwood-ph-pc\Downloads\extracted_ifc_benchmarks\LTU_A-House_Air.ifc"),
        ];

        for (name, path_str) in benchmarks {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                continue;
            }
            println!("\n==========================================");
            println!("TESTING BENCHMARK: {}", name);
            let content = std::fs::read_to_string(path).expect("Failed to read benchmark file");
            let step = crate::step::parse_step_file(&content).expect("Failed to parse STEP");
            let mut product_classes_total = std::collections::BTreeMap::new();
            for ent in step.entities.values() {
                if ent.name.starts_with("IFC") && (ent.name.contains("WALL") || ent.name.contains("DOOR") || ent.name.contains("WINDOW")
                    || ent.name.contains("SLAB") || ent.name.contains("BEAM") || ent.name.contains("COLUMN") || ent.name.contains("ROOF")
                    || ent.name.contains("STAIR") || ent.name.contains("RAILING") || ent.name.contains("PLATE") || ent.name.contains("MEMBER")
                    || ent.name.contains("CURTAIN") || ent.name.contains("COVERING") || ent.name.contains("FLOW") || ent.name.contains("DUCT")
                    || ent.name.contains("PIPE") || ent.name.contains("PROXY") || ent.name.contains("BUILDINGELEMENT") || ent.name.contains("FURNITURE")) {
                    *product_classes_total.entry(ent.name.clone()).or_insert(0) += 1;
                }
            }
            match parse_ifc(&content) {
                Ok(doc) => {
                    let total_products: usize = product_classes_total.values().sum();
                    println!("SUCCESS! Imported {} elements (out of {} candidate physical product instances), {} levels, {} views",
                        doc.elements.len(), total_products, doc.levels.len(), doc.views.len());
                    let mut counts = std::collections::BTreeMap::new();
                    let mut freeform_groups = std::collections::BTreeMap::new();
                    for el in doc.elements.values() {
                        let k = match el {
                            Element::Wall(_) => "Wall",
                            Element::Slab(_) => "Slab",
                            Element::Column(_) => "Column",
                            Element::Beam(_) => "Beam",
                            Element::Roof(_) => "Roof",
                            Element::Door(_) => "Door",
                            Element::Window(_) => "Window",
                            Element::Ceiling(_) => "Ceiling",
                            Element::FreeformSolid(f) => {
                                let grp = f.group_name.as_deref().unwrap_or("Unassigned");
                                *freeform_groups.entry(grp.to_string()).or_insert(0) += 1;
                                "FreeformSolid"
                            }
                            _ => "Other",
                        };
                        *counts.entry(k).or_insert(0) += 1;
                    }
                    for (k, v) in counts {
                        println!("  {}: {}", k, v);
                    }
                    println!("  Breakdown of FreeformSolid groups:");
                    let mut skipped_products = std::collections::BTreeMap::new();
                    let mut _type_entities = 0;
                    for ent in step.entities.values() {
                        if ent.name.ends_with("TYPE") || ent.name.starts_with("IFCREL") {
                            _type_entities += 1;
                            continue;
                        }
                        if ent.name == "IFCOPENINGELEMENT" {
                            continue;
                        }
                        // Check if ent has representation (arg 6)
                        if let Some(rep_id) = ent.get_ref(6) {
                            if let Some(rep_ent) = step.get(rep_id) {
                                if rep_ent.name == "IFCPRODUCTDEFINITIONSHAPE" {
                                    // Check if this product was imported
                                    *skipped_products.entry(ent.name.clone()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                    println!("  Physical products with ProductDefinitionShape total: {}", skipped_products.values().sum::<usize>());
                    for (k, v) in skipped_products {
                        println!("    {}: {}", k, v);
                    }
                }
                Err(err) => {
                    println!("ERROR parsing {}: {}", name, err);
                }
            }
        }
    }
}
