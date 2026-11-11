//! # glTF 2.0 Binary (GLB) Exporter for Arch3D
//!
//! Generates standalone, clean-room ISO/IEC 12113 glTF 2.0 Binary (`.glb`) files
//! compatible with modern physically-based path tracers (e.g. MuTracer / WebGPU, Three.js, Blender).
//!
//! Features:
//! - 1:1 Millimeter to Meter scale transformation ($\times 0.001$).
//! - Coordinate remapping from Arch3D ($+Z$ Up) to glTF ($+Y$ Up):
//!   `[X, Y, Z] -> [X, Z, -Y]`, preserving orthonormal basis and CCW face winding.
//! - Accurate outward vertex unit normals for physically based reflections and GGX specular lobes.
//! - Ear-clipping triangulation for arbitrary non-convex faces, courtyards, and window reveals.
//! - OpenPBR / Physical materials with metallic-roughness factors.
//! - Transmission and IOR extensions (`KHR_materials_transmission`, `KHR_materials_ior`,
//!   `KHR_materials_volume`) for photorealistic path-traced architectural glass and curtain walls.
//! - Structured scene graph with element category nodes (Walls, Slabs, Columns, Beams, etc.).

use plinth_geom::{prism::newell_normal, Face, Vec2, Vec3};
use plinth_model::{Document, Element};
use serde_json::{json, Value};

use crate::scene::MaterialPalette;

/// Exports the 3D model in the document as a standard glTF 2.0 Binary (`.glb`) byte stream.
pub fn export_glb(doc: &Document, materials: Option<&MaterialPalette>) -> Result<Vec<u8>, String> {
    let default_palette = MaterialPalette::signature_pastel();
    let mat_pal = materials.unwrap_or(&default_palette);

    // 1. Collect geometry categorized by architectural node
    let categories = [
        "Walls",
        "Slabs",
        "Roofs",
        "Ceilings",
        "Columns",
        "Beams",
        "Doors",
        "Windows",
        "Curtain Walls",
        "Stairs & Ramps",
        "Railings",
        "Footings",
        "Site",
        "Freeform Solids",
    ];

    let mut cat_primitives: Vec<Vec<RawPrimitive>> = (0..categories.len()).map(|_| Vec::new()).collect();

    // Material index constants
    const MAT_WALL: usize = 0;
    const MAT_SLAB: usize = 1;
    const MAT_ROOF: usize = 2;
    const MAT_CEILING: usize = 3;
    const MAT_COLUMN: usize = 4;
    const MAT_BEAM: usize = 5;
    const MAT_DOOR: usize = 6;
    const MAT_WINDOW_GLASS: usize = 7;
    const MAT_CURTAIN_FRAME: usize = 8;
    const MAT_CURTAIN_GLASS: usize = 9;
    const MAT_STAIR_RAMP: usize = 10;
    const MAT_RAILING: usize = 11;
    const MAT_FOOTING: usize = 12;
    const MAT_SITE: usize = 13;
    const MAT_FREEFORM: usize = 14;

    for (id, el) in &doc.elements {
        match el {
            Element::CurtainWall(cw) => {
                let base_z = doc.level_elevation(cw.base_level);
                let top_z = cw.top_level.map(|lvl| doc.level_elevation(lvl));
                let (z0, z1) = cw.z_bounds(base_z, top_z);
                if cw.length() < 10.0 || (z1 - z0) < 10.0 {
                    continue;
                }

                for prism in cw.solids(base_z, top_z) {
                    let is_glass = (prism.z_max - prism.z_min) > cw.mullion_width * 1.5
                        && prism.profile.len() == 4
                        && (prism.profile[1] - prism.profile[0]).length() > cw.mullion_width * 1.5;

                    let mat_idx = if is_glass {
                        MAT_CURTAIN_GLASS
                    } else {
                        MAT_CURTAIN_FRAME
                    };

                    for face in prism.faces() {
                        add_face_to_primitives(&mut cat_primitives[8], &face, mat_idx);
                    }
                }
            }
            Element::Window(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[7], &face, MAT_WINDOW_GLASS);
                }
            }
            Element::Door(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[6], &face, MAT_DOOR);
                }
            }
            Element::Wall(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[0], &face, MAT_WALL);
                }
            }
            Element::Slab(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[1], &face, MAT_SLAB);
                }
            }
            Element::Roof(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[2], &face, MAT_ROOF);
                }
            }
            Element::Ceiling(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[3], &face, MAT_CEILING);
                }
            }
            Element::Column(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[4], &face, MAT_COLUMN);
                }
            }
            Element::Beam(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[5], &face, MAT_BEAM);
                }
            }
            Element::Stair(_) | Element::Ramp(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[9], &face, MAT_STAIR_RAMP);
                }
            }
            Element::Railing(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[10], &face, MAT_RAILING);
                }
            }
            Element::Footing(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[11], &face, MAT_FOOTING);
                }
            }
            Element::BuildingPad(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[11], &face, MAT_FOOTING);
                }
            }
            Element::Toposolid(_) => {
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[1], &face, MAT_SLAB);
                }
            }
            Element::PropertyLine(_) => {
                // 2D cadastral survey boundary - no 3D GLB mesh
            }
            Element::FreeformSolid(f) => {
                if f.group_name.as_deref() == Some("Space")
                    || f.ifc_category.as_deref() == Some("IFCSPACE")
                {
                    continue;
                }
                let (cat_idx, mat_idx) = match f.ifc_category.as_deref().or(f.group_name.as_deref()) {
                    Some("Wall") => (0, MAT_WALL),
                    Some("Floor") | Some("Slab") => (1, MAT_SLAB),
                    Some("Roof") => (2, MAT_ROOF),
                    Some("Ceiling") => (3, MAT_CEILING),
                    Some("Column") => (4, MAT_COLUMN),
                    Some("Beam") => (5, MAT_BEAM),
                    Some("Door") => (6, MAT_DOOR),
                    Some("Window") => (7, MAT_WINDOW_GLASS),
                    Some("CurtainWall") => (8, MAT_CURTAIN_GLASS),
                    Some("Stair") | Some("Ramp") => (9, MAT_STAIR_RAMP),
                    Some("Railing") => (10, MAT_RAILING),
                    Some("Footing") => (11, MAT_FOOTING),
                    Some("Site") => (12, MAT_SITE),
                    _ => (13, MAT_FREEFORM),
                };
                for face in f.faces() {
                    add_face_to_primitives(&mut cat_primitives[cat_idx], &face, mat_idx);
                }
            }
            Element::FreeformSurface(s) => {
                if s.is_hole {
                    continue;
                }
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[13], &face, MAT_FREEFORM);
                }
            }
            _ => {
                // Any other architectural solids
                for face in doc.element_faces(*id) {
                    add_face_to_primitives(&mut cat_primitives[13], &face, MAT_FREEFORM);
                }
            }
        }
    }

    // 2. Build Materials Table (OpenPBR / Physical parameters)
    let to_f32_rgb = |c: [u8; 3]| [c[0] as f64 / 255.0, c[1] as f64 / 255.0, c[2] as f64 / 255.0];

    let materials_json = vec![
        // 0: Wall
        build_pbr_mat("Wall_Birch", to_f32_rgb(mat_pal.wall), 1.0, 0.0, 0.85, false, None),
        // 1: Slab
        build_pbr_mat("Slab_Concrete", to_f32_rgb(mat_pal.slab), 1.0, 0.0, 0.80, false, None),
        // 2: Roof
        build_pbr_mat("Roof_Terracotta", to_f32_rgb(mat_pal.roof), 1.0, 0.05, 0.70, false, None),
        // 3: Ceiling
        build_pbr_mat("Ceiling_Chalk", to_f32_rgb(mat_pal.ceiling), 1.0, 0.0, 0.90, false, None),
        // 4: Column
        build_pbr_mat("Column_MutedSage", to_f32_rgb(mat_pal.column), 1.0, 0.15, 0.60, false, None),
        // 5: Beam
        build_pbr_mat("Beam_MineralGrey", to_f32_rgb(mat_pal.beam), 1.0, 0.35, 0.50, false, None),
        // 6: Door
        build_pbr_mat("Door_BleachedWood", to_f32_rgb(mat_pal.door), 1.0, 0.0, 0.55, false, None),
        // 7: Window Glass (Transmissive PBR glass with IOR 1.52)
        build_pbr_mat("Window_Glass", to_f32_rgb([mat_pal.window_glass[0], mat_pal.window_glass[1], mat_pal.window_glass[2]]), 0.35, 0.0, 0.03, true, Some(GlassParams { transmission: 0.95, ior: 1.52, thickness: 0.015 })),
        // 8: Curtain Frame (Matte metallic aluminum)
        build_pbr_mat("Curtain_Frame", to_f32_rgb(mat_pal.curtain_frame), 1.0, 0.80, 0.30, false, None),
        // 9: Curtain Glass (Transmissive pale sky cyan glass)
        build_pbr_mat("Curtain_Glass", to_f32_rgb([mat_pal.curtain_glass[0], mat_pal.curtain_glass[1], mat_pal.curtain_glass[2]]), 0.30, 0.0, 0.02, true, Some(GlassParams { transmission: 0.96, ior: 1.52, thickness: 0.020 })),
        // 10: Stair & Ramp
        build_pbr_mat("Stair_Ramp", to_f32_rgb(mat_pal.stair_ramp), 1.0, 0.0, 0.75, false, None),
        // 11: Railing
        build_pbr_mat("Railing_Titanium", to_f32_rgb(mat_pal.railing), 1.0, 0.85, 0.25, false, None),
        // 12: Footing
        build_pbr_mat("Footing_Basalt", to_f32_rgb(mat_pal.footing), 1.0, 0.0, 0.90, false, None),
        // 13: Site
        build_pbr_mat("Site_Matcha", to_f32_rgb(mat_pal.site), 1.0, 0.0, 0.85, false, None),
        // 14: Freeform
        build_pbr_mat("Generic_Ivory", to_f32_rgb(mat_pal.freeform), 1.0, 0.0, 0.70, false, None),
    ];

    // 3. Assemble binary buffer, accessors, and buffer views
    let mut bin_data: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<Value> = Vec::new();
    let mut accessors: Vec<Value> = Vec::new();
    let mut meshes: Vec<Value> = Vec::new();
    let mut nodes: Vec<Value> = Vec::new();
    let mut scene_node_indices: Vec<usize> = Vec::new();

    for (cat_idx, name) in categories.iter().enumerate() {
        let prims = &cat_primitives[cat_idx];
        if prims.is_empty() {
            continue;
        }

        // Group primitives in this category by material index
        let mut grouped: std::collections::BTreeMap<usize, RawPrimitive> = std::collections::BTreeMap::new();
        for p in prims {
            let entry = grouped.entry(p.material_idx).or_insert_with(|| RawPrimitive {
                material_idx: p.material_idx,
                positions: Vec::new(),
                normals: Vec::new(),
                indices: Vec::new(),
            });

            let base_v = entry.positions.len() as u32;
            entry.positions.extend_from_slice(&p.positions);
            entry.normals.extend_from_slice(&p.normals);
            for idx in &p.indices {
                entry.indices.push(base_v + idx);
            }
        }

        let mut mesh_primitives_json: Vec<Value> = Vec::new();

        for (_, prim) in grouped {
            if prim.positions.is_empty() || prim.indices.is_empty() {
                continue;
            }

            // A. Position BufferView & Accessor
            let pos_bytes = slice_to_bytes(&prim.positions);
            let pos_offset = bin_data.len();
            bin_data.extend_from_slice(&pos_bytes);
            pad_to_4(&mut bin_data);

            let pos_bv_idx = buffer_views.len();
            buffer_views.push(json!({
                "buffer": 0,
                "byteOffset": pos_offset,
                "byteLength": pos_bytes.len(),
                "target": 34962 // ARRAY_BUFFER
            }));

            let (min_pos, max_pos) = compute_bounds(&prim.positions);
            let pos_acc_idx = accessors.len();
            accessors.push(json!({
                "bufferView": pos_bv_idx,
                "byteOffset": 0,
                "componentType": 5126, // FLOAT
                "count": prim.positions.len(),
                "type": "VEC3",
                "min": min_pos,
                "max": max_pos
            }));

            // B. Normal BufferView & Accessor
            let norm_bytes = slice_to_bytes(&prim.normals);
            let norm_offset = bin_data.len();
            bin_data.extend_from_slice(&norm_bytes);
            pad_to_4(&mut bin_data);

            let norm_bv_idx = buffer_views.len();
            buffer_views.push(json!({
                "buffer": 0,
                "byteOffset": norm_offset,
                "byteLength": norm_bytes.len(),
                "target": 34962 // ARRAY_BUFFER
            }));

            let norm_acc_idx = accessors.len();
            accessors.push(json!({
                "bufferView": norm_bv_idx,
                "byteOffset": 0,
                "componentType": 5126, // FLOAT
                "count": prim.normals.len(),
                "type": "VEC3"
            }));

            // C. Index BufferView & Accessor
            let idx_bytes = slice_u32_to_bytes(&prim.indices);
            let idx_offset = bin_data.len();
            bin_data.extend_from_slice(&idx_bytes);
            pad_to_4(&mut bin_data);

            let idx_bv_idx = buffer_views.len();
            buffer_views.push(json!({
                "buffer": 0,
                "byteOffset": idx_offset,
                "byteLength": idx_bytes.len(),
                "target": 34963 // ELEMENT_ARRAY_BUFFER
            }));

            let idx_acc_idx = accessors.len();
            accessors.push(json!({
                "bufferView": idx_bv_idx,
                "byteOffset": 0,
                "componentType": 5125, // UNSIGNED_INT
                "count": prim.indices.len(),
                "type": "SCALAR"
            }));

            mesh_primitives_json.push(json!({
                "attributes": {
                    "POSITION": pos_acc_idx,
                    "NORMAL": norm_acc_idx
                },
                "indices": idx_acc_idx,
                "material": prim.material_idx
            }));
        }

        if !mesh_primitives_json.is_empty() {
            let mesh_idx = meshes.len();
            meshes.push(json!({
                "name": format!("Mesh_{}", name),
                "primitives": mesh_primitives_json
            }));

            let node_idx = nodes.len();
            nodes.push(json!({
                "name": name,
                "mesh": mesh_idx
            }));
            scene_node_indices.push(node_idx);
        }
    }

    // 4. Build glTF Document JSON
    let gltf_json = json!({
        "asset": {
            "version": "2.0",
            "generator": "Arch3D Clean-Room BIM Modeler"
        },
        "extensionsUsed": [
            "KHR_materials_transmission",
            "KHR_materials_ior",
            "KHR_materials_volume"
        ],
        "scene": 0,
        "scenes": [
            {
                "name": "Arch3D BIM Model",
                "nodes": scene_node_indices
            }
        ],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials_json,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [
            {
                "byteLength": bin_data.len()
            }
        ]
    });

    let mut json_bytes = serde_json::to_vec(&gltf_json)
        .map_err(|e| format!("Failed to serialize glTF JSON: {}", e))?;

    // Pad JSON chunk to 4-byte boundary with spaces (0x20) per glTF spec
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(0x20);
    }

    // Pad BIN chunk to 4-byte boundary with null bytes (0x00) per glTF spec
    while bin_data.len() % 4 != 0 {
        bin_data.push(0x00);
    }

    // 5. Assemble GLB Container
    // Header (12 bytes) + Chunk 0 (JSON: 8 + json_len) + Chunk 1 (BIN: 8 + bin_len)
    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_data.len();

    let mut glb: Vec<u8> = Vec::with_capacity(total_length);

    // GLB Header
    glb.extend_from_slice(b"glTF"); // magic: 0x46546C67
    glb.extend_from_slice(&2u32.to_le_bytes()); // version: 2
    glb.extend_from_slice(&(total_length as u32).to_le_bytes()); // total byte length

    // Chunk 0: JSON Chunk
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"JSON"); // 0x4E4F534A
    glb.extend_from_slice(&json_bytes);

    // Chunk 1: BIN Chunk
    glb.extend_from_slice(&(bin_data.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"BIN\0"); // 0x004E4942
    glb.extend_from_slice(&bin_data);

    Ok(glb)
}

struct GlassParams {
    transmission: f64,
    ior: f64,
    thickness: f64,
}

fn build_pbr_mat(
    name: &str,
    color: [f64; 3],
    alpha: f64,
    metallic: f64,
    roughness: f64,
    is_glass: bool,
    glass: Option<GlassParams>,
) -> Value {
    let mut mat = json!({
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [color[0], color[1], color[2], alpha],
            "metallicFactor": metallic,
            "roughnessFactor": roughness
        },
        "doubleSided": true
    });

    if is_glass {
        mat["alphaMode"] = json!("BLEND");
        if let Some(g) = glass {
            mat["extensions"] = json!({
                "KHR_materials_transmission": {
                    "transmissionFactor": g.transmission
                },
                "KHR_materials_ior": {
                    "ior": g.ior
                },
                "KHR_materials_volume": {
                    "thicknessFactor": g.thickness
                }
            });
        }
    }

    mat
}

struct RawPrimitive {
    material_idx: usize,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn add_face_to_primitives(target: &mut Vec<RawPrimitive>, face: &Face, mat_idx: usize) {
    let n = face.pts.len();
    if n < 3 {
        return;
    }

    // Normal calculation
    let norm_v = if face.normal.length() > 1e-4 {
        face.normal.normalized()
    } else {
        newell_normal(&face.pts)
    };

    // Coordinate & Normal Remapping:
    // Arch3D (+Z Up, mm) -> glTF (+Y Up, meters)
    // [X, Y, Z] -> [X * 0.001, Z * 0.001, -Y * 0.001]
    let map_pos = |p: Vec3| -> [f32; 3] {
        [(p.x * 0.001) as f32, (p.z * 0.001) as f32, (-p.y * 0.001) as f32]
    };
    let map_norm = |n: Vec3| -> [f32; 3] {
        [n.x as f32, n.z as f32, -n.y as f32]
    };

    let gltf_norm = map_norm(norm_v);

    let mut tri_indices: Vec<[usize; 3]> = Vec::new();
    if n == 3 {
        tri_indices.push([0, 1, 2]);
    } else if n == 4 {
        // Test convexity in 2D tangent plane
        let (u, v) = tangent_basis(norm_v);
        let pts_2d: Vec<Vec2> = face
            .pts
            .iter()
            .map(|p| Vec2::new(p.dot(u), p.dot(v)))
            .collect();

        if plinth_geom::polygon::is_convex(&pts_2d) {
            tri_indices.push([0, 1, 2]);
            tri_indices.push([0, 2, 3]);
        } else {
            tri_indices = plinth_geom::polygon::triangulate(&pts_2d);
        }
    } else {
        let (u, v) = tangent_basis(norm_v);
        let pts_2d: Vec<Vec2> = face
            .pts
            .iter()
            .map(|p| Vec2::new(p.dot(u), p.dot(v)))
            .collect();

        let tris = plinth_geom::polygon::triangulate(&pts_2d);
        if !tris.is_empty() {
            tri_indices = tris;
        } else {
            // Robust fan triangulation fallback
            for i in 1..(n - 1) {
                tri_indices.push([0, i, i + 1]);
            }
        }
    }

    if tri_indices.is_empty() {
        return;
    }

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n);

    for p in &face.pts {
        positions.push(map_pos(*p));
        normals.push(gltf_norm);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(tri_indices.len() * 3);
    for t in tri_indices {
        indices.push(t[0] as u32);
        indices.push(t[1] as u32);
        indices.push(t[2] as u32);
    }

    target.push(RawPrimitive {
        material_idx: mat_idx,
        positions,
        normals,
        indices,
    });
}

fn tangent_basis(normal: Vec3) -> (Vec3, Vec3) {
    let u = if normal.z.abs() < 0.9 && (normal.x.abs() > 1e-4 || normal.y.abs() > 1e-4) {
        Vec3::new(-normal.y, normal.x, 0.0).normalized()
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let v = normal.cross(u).normalized();
    (u, v)
}

fn compute_bounds(pts: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    if pts.is_empty() {
        return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    }
    let mut min = pts[0];
    let mut max = pts[0];
    for p in pts {
        for i in 0..3 {
            if p[i] < min[i] {
                min[i] = p[i];
            }
            if p[i] > max[i] {
                max[i] = p[i];
            }
        }
    }
    (min, max)
}

fn pad_to_4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0x00);
    }
}

fn slice_to_bytes(slice: &[[f32; 3]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(slice.len() * 12);
    for p in slice {
        bytes.extend_from_slice(&p[0].to_le_bytes());
        bytes.extend_from_slice(&p[1].to_le_bytes());
        bytes.extend_from_slice(&p[2].to_le_bytes());
    }
    bytes
}

fn slice_u32_to_bytes(slice: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(slice.len() * 4);
    for &idx in slice {
        bytes.extend_from_slice(&idx.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_model::Document;

    #[test]
    fn test_export_sample_building_glb() {
        let doc = Document::create_sample_building();
        let glb = export_glb(&doc, None).expect("GLB export should succeed");

        assert!(glb.len() > 100, "GLB file should have valid length");
        // Verify GLB magic "glTF"
        assert_eq!(&glb[0..4], b"glTF");
        // Verify version 2
        let version = u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]);
        assert_eq!(version, 2);
        // Verify total length matches
        let total_len = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(total_len, glb.len());

        // Verify JSON chunk
        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        assert_eq!(&glb[16..20], b"JSON");
        let json_bytes = &glb[20..(20 + json_len)];
        let parsed: serde_json::Value =
            serde_json::from_slice(json_bytes).expect("JSON chunk must be valid glTF schema");

        assert_eq!(parsed["asset"]["version"], "2.0");
        assert!(parsed["nodes"].as_array().unwrap().len() > 0);
        assert!(parsed["meshes"].as_array().unwrap().len() > 0);
        assert!(parsed["materials"].as_array().unwrap().len() >= 10);
    }
}
