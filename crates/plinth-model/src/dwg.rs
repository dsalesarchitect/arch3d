//! Native AutoCAD DWG and DXF Importer for Arch3D / Plinth BIM.
//!
//! Powered by `acadrust` (MPL-2.0, pure Rust) to parse binary AutoCAD DWG files
//! (R13 through R2018/AC1032) and ASCII/binary DXF files without external C/C++ dependencies.
//!
//! Standardized on IFC4, 1:1 millimeter real-world scale, Right-Handed +Z Up coordinates.
//! Converts architectural layers directly to native BIM Walls (`Element::Wall`) and
//! Slabs (`Element::Slab`), 2D linework to `Element::DetailLine`, and 3D faces/solids/meshes
//! to `Element::FreeformSolid`. Preserves CAD layers and colors in document materials.

use crate::{
    DetailLine, Document, Element, FreeformSolid, Material, MaterialCategory, MaterialId, Slab,
    Wall,
};
use acadrust::entities::EntityType;
use acadrust::io::dwg::DwgReader;
use acadrust::io::dxf::DxfReader;
use acadrust::CadDocument;
use plinth_geom::{vec2, Face, Vec2, Vec3};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

/// Summary of elements imported from an AutoCAD DWG or DXF drawing.
#[derive(Debug, Clone, Default)]
pub struct DwgImportSummary {
    pub lines_count: usize,
    pub polylines_count: usize,
    pub solids_count: usize,
    pub walls_count: usize,
    pub slabs_count: usize,
    pub materials_count: usize,
    pub layers_count: usize,
    pub model_name: String,
}

/// Imports an AutoCAD DWG or DXF file from a disk path into the BIM document.
pub fn import_dwg_file(doc: &mut Document, path: &Path) -> Result<DwgImportSummary, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read CAD file: {e}"))?;
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CAD Model");
    let is_dxf = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("dxf"))
        .unwrap_or(false);

    import_cad_bytes(doc, &bytes, file_name, is_dxf)
}

/// Imports an AutoCAD DWG or DXF file from memory bytes into the BIM document.
pub fn import_dwg_bytes(
    doc: &mut Document,
    bytes: &[u8],
    file_name: Option<&str>,
) -> Result<DwgImportSummary, String> {
    let name = file_name.unwrap_or("CAD Model");
    let is_dxf = bytes.starts_with(b"0\r\nSECTION")
        || bytes.starts_with(b"0\nSECTION")
        || bytes.starts_with(b"  0\r\nSECTION")
        || bytes.starts_with(b"  0\nSECTION")
        || bytes.starts_with(b"AutoCAD DXF");

    import_cad_bytes(doc, bytes, name, is_dxf)
}

fn import_cad_bytes(
    doc: &mut Document,
    bytes: &[u8],
    model_name: &str,
    hint_dxf: bool,
) -> Result<DwgImportSummary, String> {
    let owned_bytes = bytes.to_vec();
    let cad_doc = if hint_dxf {
        match DxfReader::from_reader(Cursor::new(owned_bytes.clone())).and_then(|r| r.read()) {
            Ok(d) => d,
            Err(e_dxf) => match DwgReader::from_stream(Cursor::new(owned_bytes)).read() {
                Ok(d) => d,
                Err(e_dwg) => {
                    return Err(format!(
                        "Failed to parse CAD file as DXF ({e_dxf}) or DWG ({e_dwg})"
                    ))
                }
            },
        }
    } else {
        match DwgReader::from_stream(Cursor::new(owned_bytes.clone())).read() {
            Ok(d) => d,
            Err(e_dwg) => {
                match DxfReader::from_reader(Cursor::new(owned_bytes)).and_then(|r| r.read()) {
                    Ok(d) => d,
                    Err(e_dxf) => {
                        return Err(format!(
                            "Failed to parse CAD file as DWG ({e_dwg}) or DXF ({e_dxf})"
                        ))
                    }
                }
            }
        }
    };

    convert_cad_document(doc, &cad_doc, model_name)
}

fn convert_cad_document(
    doc: &mut Document,
    cad_doc: &CadDocument,
    model_name: &str,
) -> Result<DwgImportSummary, String> {
    let mut summary = DwgImportSummary {
        model_name: model_name.to_string(),
        ..Default::default()
    };

    let default_level = doc.active_level_id;
    let active_view = doc.active_view_id;

    // 1. Determine insertion scale factor to millimeter scale (1:1 standard)
    // AutoCAD INSUNITS:
    // 0: Unitless, 1: Inches, 2: Feet, 3: Miles, 4: Millimeters, 5: Centimeters, 6: Meters
    let insunits = cad_doc.header.insertion_units;
    let unit_scale: f64 = match insunits {
        1 => 25.4,       // Inches -> mm
        2 => 304.8,      // Feet -> mm
        3 => 1609344.0,  // Miles -> mm
        4 => 1.0,        // Millimeters
        5 => 10.0,       // Centimeters -> mm
        6 => 1000.0,     // Meters -> mm
        _ => {
            // Unitless (0): standard 1:1 millimeter scale for Arch3D BIM
            1.0
        }
    };

    // 2. Extract Layers and create corresponding Materials
    let mut layer_mat_map: HashMap<String, MaterialId> = HashMap::new();
    let mut layer_color_map: HashMap<String, [u8; 3]> = HashMap::new();

    for layer in cad_doc.layers.iter() {
        let layer_name = &layer.name;
        let rgb_u8 = layer.color.rgb().unwrap_or((180, 180, 180));
        let color_f32 = [
            rgb_u8.0 as f32 / 255.0,
            rgb_u8.1 as f32 / 255.0,
            rgb_u8.2 as f32 / 255.0,
            1.0,
        ];

        let mat = Material::new(
            if layer_name.is_empty() {
                "CAD Layer Default".to_string()
            } else {
                layer_name.clone()
            },
            MaterialCategory::Generic,
            color_f32,
        );
        let id = mat.id;
        doc.materials.insert(id, mat);
        let key = layer_name.trim().to_lowercase();
        layer_mat_map.insert(key.clone(), id);
        layer_color_map.insert(key, [rgb_u8.0, rgb_u8.1, rgb_u8.2]);
        summary.layers_count += 1;
        summary.materials_count += 1;
    }

    // Helper to resolve entity layer and color
    let get_layer_info = |layer_name: &str| -> (Option<MaterialId>, [u8; 3]) {
        let key = layer_name.trim().to_lowercase();
        let mat = layer_mat_map.get(&key).copied();
        let col = layer_color_map
            .get(&key)
            .copied()
            .unwrap_or([180, 180, 180]);
        (mat, col)
    };

    // Helper to test if layer is an architectural wall layer
    let is_wall_layer = |layer: &str| -> bool {
        let l = layer.to_uppercase();
        l.contains("WALL")
            || l.contains("PAREDE")
            || l.contains("MURO")
            || l.contains("PAROIR")
            || l.starts_with("A-WALL")
            || l.starts_with("M-WALL")
            || l.starts_with("ARQ-PAR")
    };

    // Helper to test if layer is a slab/floor layer
    let is_slab_layer = |layer: &str| -> bool {
        let l = layer.to_uppercase();
        l.contains("SLAB")
            || l.contains("FLOOR")
            || l.contains("PISO")
            || l.contains("LAJE")
            || l.starts_with("A-FLOR")
            || l.starts_with("A-SLAB")
            || l.starts_with("S-SLAB")
    };

    // 3. Collect all entities from model space, recursively exploding INSERT blocks
    let mut raw_entities: Vec<EntityType> = Vec::new();
    let model_entities: Vec<&EntityType> = cad_doc.model_space_entities().collect();
    let source_iter = if model_entities.is_empty() {
        cad_doc.entities().collect::<Vec<_>>()
    } else {
        model_entities
    };

    for ent in source_iter {
        if let EntityType::Insert(ins) = ent {
            // Recursively explode blocks into world entities
            let mut stack = vec![ins.clone()];
            let mut depth = 0;
            while let Some(current_ins) = stack.pop() {
                depth += 1;
                if depth > 100 {
                    break;
                }
                let exploded = current_ins.explode_from_document(cad_doc);
                for sub_ent in exploded {
                    if let EntityType::Insert(nested_ins) = sub_ent {
                        stack.push(nested_ins);
                    } else {
                        raw_entities.push(sub_ent);
                    }
                }
            }
        } else {
            raw_entities.push(ent.clone());
        }
    }

    // 4. Process CAD Entities into BIM Elements
    let mut mesh_faces: Vec<Face> = Vec::new();

    for ent in &raw_entities {
        let common = ent.common();
        let layer_name = &common.layer;
        let (layer_mat, _layer_col) = get_layer_info(layer_name);

        match ent {
            EntityType::Line(l) => {
                let p0 = Vec3::new(
                    l.start.x * unit_scale,
                    l.start.y * unit_scale,
                    l.start.z * unit_scale,
                );
                let p1 = Vec3::new(
                    l.end.x * unit_scale,
                    l.end.y * unit_scale,
                    l.end.z * unit_scale,
                );
                let len = (p1 - p0).length();

                if is_wall_layer(layer_name) && len > 50.0 {
                    // Convert line to native BIM Wall
                    let mut wall =
                        Wall::new(vec2(p0.x, p0.y), vec2(p1.x, p1.y), default_level);
                    wall.name = format!("CAD Wall ({layer_name})");
                    wall.material_id = layer_mat;
                    wall.unconnected_height = 3000.0;
                    wall.thickness = 200.0;
                    doc.elements.insert(wall.id, Element::Wall(wall));
                    summary.walls_count += 1;
                } else {
                    // Create DetailLine for 2D drafting
                    let detail = DetailLine::new(
                        active_view,
                        vec2(p0.x, p0.y),
                        vec2(p1.x, p1.y),
                        "Medium",
                    );
                    doc.elements.insert(detail.id, Element::DetailLine(detail));
                    summary.lines_count += 1;

                    // If 3D, emit a thin 3D rib so it renders in 3D views as well
                    if (p0.z - p1.z).abs() > 1.0 || p0.z.abs() > 1.0 {
                        let offset = Vec3::new(0.0, 0.0, 10.0);
                        let face = Face::solid(vec![p0, p1, p1 + offset, p0 + offset]);
                        mesh_faces.push(face);
                    }
                }
            }
            EntityType::LwPolyline(p) => {
                let z = p.elevation * unit_scale;
                let mut pts: Vec<Vec2> = Vec::new();

                for i in 0..p.vertices.len() {
                    let v0 = &p.vertices[i];
                    let pt0 = vec2(v0.location.x * unit_scale, v0.location.y * unit_scale);
                    pts.push(pt0);

                    // Interpolate arc bulge if non-zero
                    if v0.bulge.abs() > 1e-6 && (i + 1 < p.vertices.len() || p.is_closed) {
                        let next_idx = (i + 1) % p.vertices.len();
                        let v1 = &p.vertices[next_idx];
                        let pt1 = vec2(v1.location.x * unit_scale, v1.location.y * unit_scale);
                        interpolate_bulge(pt0, pt1, v0.bulge, &mut pts);
                    }
                }

                if pts.len() >= 2 {
                    if p.is_closed && pts.len() >= 3 && is_slab_layer(layer_name) {
                        // Closed polyline on slab layer -> Native BIM Slab!
                        let mut slab = Slab::new(pts.clone(), default_level);
                        slab.name = format!("CAD Slab ({layer_name})");
                        slab.thickness = 200.0;
                        slab.material_id = layer_mat;
                        doc.elements.insert(slab.id, Element::Slab(slab));
                        summary.slabs_count += 1;
                    } else if p.is_closed && pts.len() >= 3 && is_wall_layer(layer_name) {
                        // Closed polyline on wall layer: emit wall segments
                        for i in 0..pts.len() {
                            let p_a = pts[i];
                            let p_b = pts[(i + 1) % pts.len()];
                            if (p_b - p_a).length() > 50.0 {
                                let mut wall = Wall::new(p_a, p_b, default_level);
                                wall.name = format!("CAD Wall ({layer_name})");
                                wall.material_id = layer_mat;
                                doc.elements.insert(wall.id, Element::Wall(wall));
                                summary.walls_count += 1;
                            }
                        }
                    } else {
                        // General polyline -> 2D linework
                        let n = if p.is_closed { pts.len() } else { pts.len() - 1 };
                        for i in 0..n {
                            let p_a = pts[i];
                            let p_b = pts[(i + 1) % pts.len()];
                            let detail = DetailLine::new(active_view, p_a, p_b, "Medium");
                            doc.elements.insert(detail.id, Element::DetailLine(detail));
                        }
                        summary.polylines_count += 1;

                        // 3D face if closed polygon
                        if p.is_closed && pts.len() >= 3 {
                            let face_pts: Vec<Vec3> =
                                pts.iter().map(|p| Vec3::new(p.x, p.y, z)).collect();
                            mesh_faces.push(Face::solid(face_pts));
                        }
                    }
                }
            }
            EntityType::Face3D(f) => {
                let corners = f.corners();
                if corners.len() >= 3 {
                    let face_pts: Vec<Vec3> = corners
                        .iter()
                        .map(|c| Vec3::new(c.x * unit_scale, c.y * unit_scale, c.z * unit_scale))
                        .collect();
                    mesh_faces.push(Face::solid(face_pts));
                    summary.solids_count += 1;
                }
            }
            EntityType::Solid(s) => {
                // AutoCAD Solid corners: first, second, fourth, third (standard DXF vertex ordering)
                let c0 = Vec3::new(
                    s.first_corner.x * unit_scale,
                    s.first_corner.y * unit_scale,
                    s.first_corner.z * unit_scale,
                );
                let c1 = Vec3::new(
                    s.second_corner.x * unit_scale,
                    s.second_corner.y * unit_scale,
                    s.second_corner.z * unit_scale,
                );
                let c2 = Vec3::new(
                    s.third_corner.x * unit_scale,
                    s.third_corner.y * unit_scale,
                    s.third_corner.z * unit_scale,
                );
                let c3 = Vec3::new(
                    s.fourth_corner.x * unit_scale,
                    s.fourth_corner.y * unit_scale,
                    s.fourth_corner.z * unit_scale,
                );

                let face_pts = if (c2 - c3).length() < 1e-4 {
                    vec![c0, c1, c2]
                } else {
                    vec![c0, c1, c3, c2]
                };
                mesh_faces.push(Face::solid(face_pts));
                summary.solids_count += 1;
            }
            EntityType::Mesh(m) => {
                for face in &m.faces {
                    if face.vertices.len() >= 3 {
                        let mut face_pts = Vec::with_capacity(face.vertices.len());
                        for &v_idx in &face.vertices {
                            if let Some(v) = m.vertices.get(v_idx) {
                                face_pts.push(Vec3::new(
                                    v.x * unit_scale,
                                    v.y * unit_scale,
                                    v.z * unit_scale,
                                ));
                            }
                        }
                        if face_pts.len() >= 3 {
                            mesh_faces.push(Face::solid(face_pts));
                        }
                    }
                }
                summary.solids_count += 1;
            }
            EntityType::Circle(c) => {
                let center = vec2(c.center.x * unit_scale, c.center.y * unit_scale);
                let r = c.radius * unit_scale;
                let segments = 24;
                for i in 0..segments {
                    let theta1 = (i as f64 / segments as f64) * std::f64::consts::TAU;
                    let theta2 = ((i + 1) as f64 / segments as f64) * std::f64::consts::TAU;
                    let p0 = center + vec2(r * theta1.cos(), r * theta1.sin());
                    let p1 = center + vec2(r * theta2.cos(), r * theta2.sin());
                    let detail = DetailLine::new(active_view, p0, p1, "Medium");
                    doc.elements.insert(detail.id, Element::DetailLine(detail));
                }
                summary.lines_count += segments;
            }
            EntityType::Arc(a) => {
                let center = vec2(a.center.x * unit_scale, a.center.y * unit_scale);
                let r = a.radius * unit_scale;
                let start = a.start_angle;
                let mut end = a.end_angle;
                if end < start {
                    end += std::f64::consts::TAU;
                }
                let delta = end - start;
                let segments = ((delta / std::f64::consts::TAU) * 24.0).ceil().max(4.0) as usize;
                for i in 0..segments {
                    let theta1 = start + (i as f64 / segments as f64) * delta;
                    let theta2 = start + ((i + 1) as f64 / segments as f64) * delta;
                    let p0 = center + vec2(r * theta1.cos(), r * theta1.sin());
                    let p1 = center + vec2(r * theta2.cos(), r * theta2.sin());
                    let detail = DetailLine::new(active_view, p0, p1, "Medium");
                    doc.elements.insert(detail.id, Element::DetailLine(detail));
                }
                summary.lines_count += segments;
            }
            _ => {
                // Other non-geometric or unhandled entities
            }
        }
    }

    // 5. If 3D mesh faces were gathered, wrap into a FreeformSolid Mesh
    if !mesh_faces.is_empty() {
        let solid_element =
            FreeformSolid::new_mesh(format!("CAD Mesh ({model_name})"), mesh_faces, default_level);
        doc.elements
            .insert(solid_element.id, Element::FreeformSolid(solid_element));
        summary.solids_count += 1;
    }

    Ok(summary)
}

/// Discretizes an AutoCAD polyline bulge arc into intermediate vertices.
/// Bulge = tan(included_angle / 4).
fn interpolate_bulge(p0: Vec2, p1: Vec2, bulge: f64, pts: &mut Vec<Vec2>) {
    let d = (p1 - p0).length();
    if d < 1e-4 {
        return;
    }

    let theta = 4.0 * bulge.atan();
    let num_samples = 6;
    let normal = vec2(-(p1.y - p0.y), p1.x - p0.x) / d;

    let sagitta = (d * 0.5) * (theta * 0.25).tan();

    for step in 1..num_samples {
        let t = step as f64 / num_samples as f64;
        let linear_pt = p0 + (p1 - p0) * t;
        // Parabolic arc displacement approximation for smooth curvature
        let arch_factor = 4.0 * t * (1.0 - t);
        let curved_pt = linear_pt + normal * (sagitta * arch_factor);
        pts.push(curved_pt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_dxf_linework_and_walls() {
        // Minimal standard DXF with a generic line and a WALL layer line
        let dxf_content = "0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n30\n0.0\n11\n500.0\n21\n0.0\n31\n0.0\n\
0\nLINE\n8\nA-WALL\n10\n100.0\n20\n200.0\n30\n0.0\n11\n100.0\n21\n5200.0\n31\n0.0\n\
0\nENDSEC\n0\nEOF\n";

        let mut doc = Document::new_empty();
        let summary = import_dwg_bytes(&mut doc, dxf_content.as_bytes(), Some("test_plan.dxf"))
            .expect("DXF import should succeed");

        assert_eq!(summary.lines_count, 1, "Should have 1 detail line on layer 0");
        assert_eq!(summary.walls_count, 1, "Should convert A-WALL line to BIM Wall");

        // Verify wall in document
        let wall = doc
            .elements
            .values()
            .find_map(|e| match e {
                Element::Wall(w) => Some(w),
                _ => None,
            })
            .expect("Wall should be inserted");
        assert_eq!(wall.start, vec2(100.0, 200.0));
        assert_eq!(wall.end, vec2(100.0, 5200.0));
        assert_eq!(wall.unconnected_height, 3000.0);
    }

    #[test]
    fn test_import_dxf_closed_slab() {
        // Minimal DXF with closed LWPOLYLINE on SLAB layer
        let dxf_content = "0\nSECTION\n2\nENTITIES\n\
0\nLWPOLYLINE\n8\nA-SLAB\n90\n4\n70\n1\n\
10\n0.0\n20\n0.0\n\
10\n4000.0\n20\n0.0\n\
10\n4000.0\n20\n3000.0\n\
10\n0.0\n20\n3000.0\n\
0\nENDSEC\n0\nEOF\n";

        let mut doc = Document::new_empty();
        let summary = import_dwg_bytes(&mut doc, dxf_content.as_bytes(), Some("test_slab.dxf"))
            .expect("DXF import should succeed");

        assert_eq!(summary.slabs_count, 1, "Should convert A-SLAB closed polyline to BIM Slab");

        let slab = doc
            .elements
            .values()
            .find_map(|e| match e {
                Element::Slab(s) => Some(s),
                _ => None,
            })
            .expect("Slab should be inserted");
        assert_eq!(slab.boundary.len(), 4);
    }

    #[test]
    fn test_import_dxf_3d_faces() {
        // Minimal DXF with a 3DFACE
        let dxf_content = "0\nSECTION\n2\nENTITIES\n\
0\n3DFACE\n8\n3D_OBJECTS\n\
10\n0.0\n20\n0.0\n30\n0.0\n\
11\n1000.0\n21\n0.0\n31\n0.0\n\
12\n1000.0\n22\n1000.0\n32\n500.0\n\
13\n0.0\n23\n1000.0\n33\n500.0\n\
0\nENDSEC\n0\nEOF\n";

        let mut doc = Document::new_empty();
        let summary = import_dwg_bytes(&mut doc, dxf_content.as_bytes(), Some("test_face.dxf"))
            .expect("DXF import should succeed");

        assert!(summary.solids_count >= 1, "Should produce solid freeform mesh");

        let mesh = doc
            .elements
            .values()
            .find_map(|e| match e {
                Element::FreeformSolid(f) => Some(f),
                _ => None,
            })
            .expect("FreeformSolid should exist");
        assert!(mesh.is_mesh());
    }

    #[test]
    fn test_import_dxf_circle_and_arc() {
        let dxf_content = "0\nSECTION\n2\nENTITIES\n\
0\nCIRCLE\n8\nCAD_CIRCLES\n10\n500.0\n20\n500.0\n30\n0.0\n40\n250.0\n\
0\nARC\n8\nCAD_ARCS\n10\n1000.0\n20\n1000.0\n30\n0.0\n40\n300.0\n50\n0.0\n51\n90.0\n\
0\nENDSEC\n0\nEOF\n";

        let mut doc = Document::new_empty();
        let summary = import_dwg_bytes(&mut doc, dxf_content.as_bytes(), Some("test_curves.dxf"))
            .expect("DXF import should succeed");

        assert!(summary.lines_count >= 24, "Should discretize circle and arc into detail lines");
    }
}
