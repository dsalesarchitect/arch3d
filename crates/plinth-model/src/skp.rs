//! Native SketchUp (.skp) Model Importer for Arch3D / Plinth BIM.
//!
//! Uses `openskp` to read both modern VFF containers (SketchUp 2021-2026) and
//! legacy MFC CArchive streams (SketchUp 2013-2020) without proprietary SDK dependencies.
//!
//! Converts geometry directly to millimeter-scale 1:1 `FreeformSolid` mesh elements,
//! extracting materials, embedded textures, scene graph component transformations,
//! and clean outward-oriented watertight faces without exploding artifacts.

use crate::{Document, Element, FreeformSolid, Material, MaterialCategory, MaterialId};
use plinth_geom::{vec2, vec3, Face, Vec2, Vec3};
use std::collections::HashMap;
use std::path::Path;

/// Summary of elements imported from a SketchUp (.skp) file.
#[derive(Debug, Clone)]
pub struct SkpImportSummary {
    pub solids_count: usize,
    pub faces_count: usize,
    pub materials_count: usize,
    pub layers_count: usize,
    pub model_name: String,
}

/// Imports a SketchUp (`.skp`) file from a disk path into the BIM document.
pub fn import_skp_file(doc: &mut Document, path: &Path) -> Result<SkpImportSummary, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read SKP file: {e}"))?;
    let mut summary = import_skp_bytes(doc, &bytes)?;
    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
        summary.model_name = file_name.to_string();
    }
    Ok(summary)
}

/// Imports a SketchUp (`.skp`) model from memory bytes into the BIM document.
pub fn import_skp_bytes(doc: &mut Document, bytes: &[u8]) -> Result<SkpImportSummary, String> {
    let skp_model = openskp::Model::parse(bytes)
        .map_err(|e| format!("OpenSKP parse error: {e:?}"))?;

    let default_level = doc.active_level_id;

    // 1. Extract Materials & Embedded Textures
    // Map openskp material slot index to plinth-model MaterialId
    let mut mat_map: HashMap<usize, MaterialId> = HashMap::new();
    let mut materials_count = 0;

    for (idx, mat) in skp_model.materials.iter().enumerate() {
        let (name, rgba, texture_name, image_bytes, opacity) = match mat {
            openskp::Material::Solid { name, rgba, opacity, .. } => {
                (name.clone(), *rgba, None, None, *opacity)
            }
            openskp::Material::Textured {
                name,
                avg_rgba,
                texture,
                image_bytes,
                opacity,
                ..
            } => {
                let rgba = avg_rgba.unwrap_or([200, 200, 200, 255]);
                (name.clone(), rgba, texture.clone(), image_bytes.clone(), *opacity)
            }
        };

        let color_f32 = [
            rgba[0] as f32 / 255.0,
            rgba[1] as f32 / 255.0,
            rgba[2] as f32 / 255.0,
            rgba[3] as f32 / 255.0,
        ];

        let mut doc_mat = Material::new(
            if name.is_empty() { format!("SketchUp Material {idx}") } else { name },
            MaterialCategory::Generic,
            color_f32,
        );
        doc_mat.transparency = (1.0 - opacity as f32).clamp(0.0, 1.0);
        doc_mat.texture_name = texture_name;
        doc_mat.texture_bytes = image_bytes;

        let id = doc_mat.id;
        doc.materials.insert(id, doc_mat);
        mat_map.insert(idx, id);
        materials_count += 1;
    }

    // Helper to resolve material for a face
    let resolve_face_material = |front_slot: Option<u16>, back_slot: Option<u16>| -> (Option<MaterialId>, bool) {
        // If front material exists, use it
        if let Some(slot) = front_slot {
            if let Some(&(_, mat_idx)) = skp_model.material_links.iter().find(|(s, _)| *s == slot) {
                if let Some(&id) = mat_map.get(&mat_idx) {
                    return (Some(id), false);
                }
            }
        }
        // If only back material exists (common in casual SketchUp models with flipped normals),
        // use back material and signal to flip winding!
        if let Some(slot) = back_slot {
            if let Some(&(_, mat_idx)) = skp_model.material_links.iter().find(|(s, _)| *s == slot) {
                if let Some(&id) = mat_map.get(&mat_idx) {
                    return (Some(id), true);
                }
            }
        }
        (None, false)
    };

    let mut total_solids = 0;
    let mut total_faces = 0;

    // Helper to transform a vertex (in meters) to world coordinates (in millimetres) with sanity checks
    let transform_point = |p: [f64; 3], world: &[f64; 16]| -> Option<Vec3> {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            return None;
        }
        let x = world[0] * p[0] + world[1] * p[1] + world[2] * p[2] + world[3];
        let y = world[4] * p[0] + world[5] * p[1] + world[6] * p[2] + world[7];
        let z = world[8] * p[0] + world[9] * p[1] + world[10] * p[2] + world[11];
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return None;
        }
        let x_mm = x * 1000.0;
        let y_mm = y * 1000.0;
        let z_mm = z * 1000.0;
        // Ignore extreme outliers (> 1,000,000 m = 1,000,000,000 mm)
        if x_mm.abs() > 1e9 || y_mm.abs() > 1e9 || z_mm.abs() > 1e9 {
            return None;
        }
        Some(vec3(x_mm, y_mm, z_mm))
    };

    // Helper to convert an OpenSKP mesh into Arch3D Face objects grouped by material
    let process_mesh = |mesh: &openskp::Mesh, world: &[f64; 16]| -> Vec<(Option<MaterialId>, Vec<Face>)> {
        let mut grouped_faces: HashMap<Option<MaterialId>, Vec<Face>> = HashMap::new();

        // Build edge hardness lookup table: (min(v0, v1), max(v0, v1)) -> is_hard
        // Smooth and soft edges in SketchUp are internal mesh contours that should NOT be drawn as hard lines.
        let mut edge_hard_map: HashMap<(u32, u32), bool> = HashMap::with_capacity(mesh.edges.len());
        for edge in &mesh.edges {
            let key = (edge.v0.min(edge.v1), edge.v0.max(edge.v1));
            let is_hard = !edge.soft && !edge.smooth && !edge.hidden;
            edge_hard_map.insert(key, is_hard);
        }

        let check_edge_hard = |a: u32, b: u32| -> bool {
            let key = (a.min(b), a.max(b));
            edge_hard_map.get(&key).copied().unwrap_or(false)
        };

        let vert_count = mesh.vertices.len() as u32;

        for face in &mesh.faces {
            if face.hidden {
                continue;
            }

            // Validate all vertex indices in face.outer are strictly within bounds
            if face.outer.iter().any(|&vi| vi >= vert_count) {
                continue;
            }

            let (mat_id, flip_normal) = resolve_face_material(face.front_material, face.back_material);

            // Extract outer ring 3D coordinates
            let mut outer_vids: Vec<u32> = Vec::with_capacity(face.outer.len());
            let mut outer_pts: Vec<Vec3> = Vec::with_capacity(face.outer.len());

            for &vi in &face.outer {
                if let Some(&p) = mesh.vertices.get(vi as usize) {
                    if let Some(tp) = transform_point(p, world) {
                        outer_vids.push(vi);
                        outer_pts.push(tp);
                    } else {
                        outer_pts.clear();
                        break;
                    }
                } else {
                    outer_pts.clear();
                    break;
                }
            }

            if outer_pts.len() < 3 {
                continue;
            }

            // Deduplicate consecutive identical vertices (distance < 0.1 mm)
            let mut dedup_vids = Vec::with_capacity(outer_vids.len());
            let mut dedup_pts = Vec::with_capacity(outer_pts.len());
            let n = outer_pts.len();
            for i in 0..n {
                let p = outer_pts[i];
                let next_p = outer_pts[(i + 1) % n];
                if p.distance(next_p) > 0.1 {
                    dedup_vids.push(outer_vids[i]);
                    dedup_pts.push(p);
                }
            }

            if dedup_pts.len() < 3 {
                continue;
            }

            // Normal in world space from true Newell normal
            let mut world_normal = plinth_geom::prism::newell_normal(&dedup_pts);
            if world_normal.length() < 1e-4 {
                continue; // Degenerate normal
            }

            if flip_normal {
                world_normal = -world_normal;
                dedup_vids.reverse();
                dedup_pts.reverse();
            }

            // Robust Triangulation:
            // 1. Single Triangles (len == 3)
            if dedup_pts.len() == 3 {
                let h0 = check_edge_hard(dedup_vids[0], dedup_vids[1]);
                let h1 = check_edge_hard(dedup_vids[1], dedup_vids[2]);
                let h2 = check_edge_hard(dedup_vids[2], dedup_vids[0]);
                let f = Face::new(dedup_pts, vec![h0, h1, h2]);
                grouped_faces.entry(mat_id).or_default().push(f);
            }
            // 2. Quads (len == 4) - split into 2 triangles along the shortest diagonal to avoid non-planar distortions
            else if dedup_pts.len() == 4 && face.holes.is_empty() {
                let d02 = dedup_pts[0].distance(dedup_pts[2]);
                let d13 = dedup_pts[1].distance(dedup_pts[3]);
                if d02 <= d13 {
                    // Split diagonal (0, 2)
                    let h01 = check_edge_hard(dedup_vids[0], dedup_vids[1]);
                    let h12 = check_edge_hard(dedup_vids[1], dedup_vids[2]);
                    let f1 = Face::new(
                        vec![dedup_pts[0], dedup_pts[1], dedup_pts[2]],
                        vec![h01, h12, false], // internal diagonal is soft
                    );
                    let h23 = check_edge_hard(dedup_vids[2], dedup_vids[3]);
                    let h30 = check_edge_hard(dedup_vids[3], dedup_vids[0]);
                    let f2 = Face::new(
                        vec![dedup_pts[0], dedup_pts[2], dedup_pts[3]],
                        vec![false, h23, h30], // internal diagonal is soft
                    );
                    grouped_faces.entry(mat_id).or_default().push(f1);
                    grouped_faces.entry(mat_id).or_default().push(f2);
                } else {
                    // Split diagonal (1, 3)
                    let h12 = check_edge_hard(dedup_vids[1], dedup_vids[2]);
                    let h23 = check_edge_hard(dedup_vids[2], dedup_vids[3]);
                    let f1 = Face::new(
                        vec![dedup_pts[1], dedup_pts[2], dedup_pts[3]],
                        vec![h12, h23, false],
                    );
                    let h30 = check_edge_hard(dedup_vids[3], dedup_vids[0]);
                    let h01 = check_edge_hard(dedup_vids[0], dedup_vids[1]);
                    let f2 = Face::new(
                        vec![dedup_pts[1], dedup_pts[3], dedup_pts[0]],
                        vec![false, h30, h01],
                    );
                    grouped_faces.entry(mat_id).or_default().push(f1);
                    grouped_faces.entry(mat_id).or_default().push(f2);
                }
            }
            // 3. General N-Gons - project into exact orthonormal tangent frame to prevent foreshortening/distortion
            else {
                let n_vec = world_normal;
                let (u_axis, v_axis) = if n_vec.z.abs() < 0.9 {
                    let u = n_vec.cross(Vec3::new(0.0, 0.0, 1.0)).normalized();
                    let v = n_vec.cross(u).normalized();
                    (u, v)
                } else {
                    let u = n_vec.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
                    let v = n_vec.cross(u).normalized();
                    (u, v)
                };

                let p0 = dedup_pts[0];
                let pts_2d: Vec<Vec2> = dedup_pts
                    .iter()
                    .map(|&p| vec2((p - p0).dot(u_axis), (p - p0).dot(v_axis)))
                    .collect();

                let tris = plinth_geom::polygon::triangulate(&pts_2d);
                let m = dedup_pts.len();

                for tri in tris {
                    let i0 = tri[0];
                    let i1 = tri[1];
                    let i2 = tri[2];

                    if i0 >= m || i1 >= m || i2 >= m {
                        continue;
                    }

                    // Check if edge is part of the original perimeter contour
                    let is_perimeter = |ia: usize, ib: usize| -> bool {
                        (ia + 1) % m == ib || (ib + 1) % m == ia
                    };

                    let h0 = if is_perimeter(i0, i1) {
                        check_edge_hard(dedup_vids[i0], dedup_vids[i1])
                    } else {
                        false
                    };
                    let h1 = if is_perimeter(i1, i2) {
                        check_edge_hard(dedup_vids[i1], dedup_vids[i2])
                    } else {
                        false
                    };
                    let h2 = if is_perimeter(i2, i0) {
                        check_edge_hard(dedup_vids[i2], dedup_vids[i0])
                    } else {
                        false
                    };

                    let tri_pts = vec![dedup_pts[i0], dedup_pts[i1], dedup_pts[i2]];
                    let f = Face::new(tri_pts, vec![h0, h1, h2]);
                    grouped_faces.entry(mat_id).or_default().push(f);
                }
            }
        }

        grouped_faces.into_iter().collect()
    };

    // 2. Process Root Geometry (loose geometry in model at identity)
    const IDENTITY_16: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    if let Some(root_idx) = skp_model.geometry.iter().position(|r| r.def_index.is_none()) {
        let run = &skp_model.geometry[root_idx];
        if !run.mesh.faces.is_empty() {
            let groups = process_mesh(&run.mesh, &IDENTITY_16);
            for (mat_id, faces) in groups {
                if !faces.is_empty() {
                    let face_count = faces.len();
                    let mut solid = FreeformSolid::new_mesh("Model Geometry", faces, default_level);
                    solid.material_id = mat_id;
                    doc.elements.insert(solid.id, Element::FreeformSolid(solid));
                    total_solids += 1;
                    total_faces += face_count;
                }
            }
        }
    }

    // 3. Process Component & Group Instances via Scene Graph
    let scene_nodes = skp_model.scene();

    // OpenSKP WYSIWYG rule: if node.hidden is true, prune the entire subtree!
    fn visit_node<F>(node: &openskp::Node, path: &str, f: &mut F)
    where
        F: FnMut(&openskp::Node, &str),
    {
        if node.hidden {
            return;
        }
        let current_name = node
            .definition
            .as_deref()
            .unwrap_or(if node.is_group == Some(true) { "Group" } else { "Component" });
        let full_path = if path.is_empty() {
            current_name.to_string()
        } else {
            format!("{path} / {current_name}")
        };
        f(node, &full_path);
        for child in &node.children {
            visit_node(child, &full_path, f);
        }
    }

    for root_node in &scene_nodes {
        visit_node(root_node, "", &mut |node, full_path| {
            if let Some(ri) = node.run {
                if let Some(run) = skp_model.geometry.get(ri) {
                    if !run.mesh.faces.is_empty() {
                        let groups = process_mesh(&run.mesh, &node.world);
                        let is_group = node.is_group == Some(true);
                        let default_name = if is_group { "Group" } else { "Component" };
                        let node_name = node.definition.clone().unwrap_or_else(|| default_name.into());

                        for (mat_id, faces) in groups {
                            if !faces.is_empty() {
                                let face_count = faces.len();
                                let mut solid = FreeformSolid::new_mesh(node_name.clone(), faces, default_level);
                                solid.group_name = Some(full_path.to_string());
                                solid.is_group = is_group;

                                // Override with node inherited material if face has no specific paint
                                solid.material_id = mat_id.or_else(|| {
                                    if node.material != 0 {
                                        skp_model
                                            .material_links
                                            .iter()
                                            .find(|(s, _)| *s == node.material)
                                            .and_then(|&(_, mi)| mat_map.get(&mi).copied())
                                    } else {
                                        None
                                    }
                                });

                                doc.elements.insert(solid.id, Element::FreeformSolid(solid));
                                total_solids += 1;
                                total_faces += face_count;
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(SkpImportSummary {
        solids_count: total_solids,
        faces_count: total_faces,
        materials_count,
        layers_count: skp_model.layers.len(),
        model_name: "SketchUp Model".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_skp_invalid_bytes_fails_gracefully() {
        let mut doc = Document::new_empty();
        let invalid = b"NOT_A_SKETCHUP_FILE_HEADER";
        let res = import_skp_bytes(&mut doc, invalid);
        assert!(res.is_err());
    }

    #[test]
    fn test_material_with_texture_metadata() {
        let mut mat = Material::new("Wood Oak Texture", MaterialCategory::Generic, [0.8, 0.6, 0.4, 1.0]);
        mat.texture_name = Some("wood_grain.jpg".into());
        mat.texture_bytes = Some(vec![0xFF, 0xD8, 0xFF, 0xE0]); // JPEG magic header
        assert_eq!(mat.texture_name.as_deref(), Some("wood_grain.jpg"));
        assert!(mat.texture_bytes.is_some());
    }

    #[test]
    fn test_freeform_solid_group_hierarchy() {
        let f = Face::solid(vec![
            vec3(0.0, 0.0, 0.0),
            vec3(100.0, 0.0, 0.0),
            vec3(100.0, 100.0, 0.0),
        ]);
        let mut solid = FreeformSolid::new_mesh("Armchair Seat", vec![f], crate::LevelId::new());
        solid.group_name = Some("Furniture / Armchair / Seat".into());
        solid.is_group = true;
        assert_eq!(solid.name, "Armchair Seat");
        assert_eq!(solid.group_name.as_deref(), Some("Furniture / Armchair / Seat"));
        assert!(solid.is_group);
    }
}
