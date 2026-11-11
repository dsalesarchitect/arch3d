//! Interactive 3D axonometric and perspective projection engine with Visual Styles.

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene, VisualStyle};
use plinth_geom::{vec2, Face, Prism, Vec2, Vec3};
use plinth_model::{Document, Element, ViewDef};

pub fn render_3d(
    doc: &Document,
    view: &ViewDef,
    theme: &ColorTheme,
    visual_style: VisualStyle,
) -> RenderScene {
    let mut scene = RenderScene::new();

    let cam = view.camera_3d.unwrap_or_default();
    let target = Vec3::new(cam.target_x, cam.target_y, cam.target_z);

    // Directional sunlight vector (normalized): from south-east and above
    let light_dir = Vec3::new(0.5, -0.6, 0.8).normalized();

    // Camera view transform:
    // 1. Shift by target
    // 2. Yaw (rotation around Z)
    // 3. Pitch (rotation around X)
    let (sin_y, cos_y) = cam.yaw.sin_cos();
    let (sin_p, cos_p) = cam.pitch.sin_cos();

    let transform_to_view = |p: Vec3| -> Vec3 {
        let d = p - target;
        // Yaw
        let x1 = d.x * cos_y - d.y * sin_y;
        let y1 = d.x * sin_y + d.y * cos_y;
        let z1 = d.z;
        // Pitch
        let x2 = x1;
        let y2 = y1 * cos_p - z1 * sin_p;
        let z2 = y1 * sin_p + z1 * cos_p;
        Vec3::new(x2, y2, z2)
    };

    // Camera forward vector in world space:
    let cam_forward = Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);

    // Perspective projection setup
    let is_persp = cam.fov_degrees > 1.0;
    let cam_dist = cam.distance.max(1000.0);

    let project_to_screen = |p: Vec3| -> Vec2 {
        let v = transform_to_view(p);
        if is_persp {
            let dist = (cam_dist + v.y).max(100.0);
            let factor = cam_dist / dist;
            vec2(v.x * factor, v.z * factor)
        } else {
            vec2(v.x, v.z)
        }
    };

    // Gather all faces with their depth: (depth, 2D vertices, fill color, hard edge flags, z_min, z_max)
    let mut faces_with_depth: Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)> = Vec::new();

    let has_roof_slab = doc.elements.values().any(|e| match e {
        Element::Slab(s) => {
            let (_zb, zt) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            zt >= 2500.0
        }
        Element::Roof(_) => true,
        _ => false,
    });
    let has_ground_slab = doc.elements.values().any(|e| match e {
        Element::Slab(s) => {
            let (zb, zt) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            zb <= 500.0 && zt <= 1000.0
        }
        _ => false,
    });
    // In solid visual styles, when viewing from above outside (sin_p > 0.05) with a roof slab,
    // or from below outside (sin_p < -0.05) with a ground slab, interior elements enclosed inside
    // a covered building envelope are occluded. Standalone walls/elements outside remain 100% visible!
    let occlude_from_above = has_roof_slab && visual_style != VisualStyle::Wireframe && sin_p > 0.05;
    let occlude_from_below = has_ground_slab && visual_style != VisualStyle::Wireframe && sin_p < -0.05;

    for &id in doc.elements.keys() {
        let el = doc.elements.get(&id).unwrap();

        // Architectural material base color from active pastel palette
        let m = &theme.materials;
        let base_rgb = match el {
            Element::Wall(w) => {
                if w.is_structural {
                    [
                        (m.wall[0] as u16 * 9 / 10).min(255) as u8,
                        (m.wall[1] as u16 * 9 / 10).min(255) as u8,
                        (m.wall[2] as u16 * 9 / 10).min(255) as u8,
                    ]
                } else {
                    m.wall
                }
            }
            Element::Column(c) => {
                if c.is_structural {
                    [
                        (m.column[0] as u16 * 9 / 10).min(255) as u8,
                        (m.column[1] as u16 * 9 / 10).min(255) as u8,
                        (m.column[2] as u16 * 9 / 10).min(255) as u8,
                    ]
                } else {
                    m.column
                }
            }
            Element::Beam(_) => m.beam,
            Element::Slab(s) => {
                if s.is_structural {
                    [
                        (m.slab[0] as u16 * 95 / 100).min(255) as u8,
                        (m.slab[1] as u16 * 95 / 100).min(255) as u8,
                        (m.slab[2] as u16 * 95 / 100).min(255) as u8,
                    ]
                } else {
                    m.slab
                }
            }
            Element::Roof(_) => m.roof,
            Element::Fascia(_) => m.wall,
            Element::Gutter(_) => m.railing,
            Element::Ceiling(_) => m.ceiling,
            Element::Stair(_) | Element::Ramp(_) => m.stair_ramp,
            Element::Railing(_) => m.railing,
            Element::Footing(_) => m.footing,
            Element::CurtainWall(_) => m.curtain_frame,
            Element::FreeformSolid(f) => {
                if let Some(mat_id) = f.material_id {
                    if let Some(mat) = doc.materials.get(&mat_id) {
                        [
                            (mat.color[0] * 255.0).clamp(0.0, 255.0) as u8,
                            (mat.color[1] * 255.0).clamp(0.0, 255.0) as u8,
                            (mat.color[2] * 255.0).clamp(0.0, 255.0) as u8,
                        ]
                    } else if f.is_sphere() {
                        [225u8, 180, 80]
                    } else {
                        m.freeform
                    }
                } else if let Some(g) = f.group_name.as_deref().or(f.ifc_category.as_deref()) {
                    match g.to_lowercase().as_str() {
                        "roof" => m.roof,
                        "site" | "topography" => m.site,
                        "footing" | "foundation" => m.footing,
                        "beam" => m.beam,
                        "column" => m.column,
                        "wall" => m.wall,
                        "window" => [m.window_glass[0], m.window_glass[1], m.window_glass[2]],
                        "door" => m.door,
                        "stair" | "ramp" => m.stair_ramp,
                        "railing" => m.railing,
                        "ceiling" => m.ceiling,
                        "slab" | "floor" => m.slab,
                        "curtain wall" | "curtainwall" | "curtain wall panel" => [m.window_glass[0], m.window_glass[1], m.window_glass[2]],
                        "furniture" => [190, 160, 130], // warm natural wood
                        "plumbing fixture" | "sanitary" => [240, 245, 250], // porcelain ceramic white
                        "lighting" => [255, 240, 160], // soft luminaire gold
                        "duct" | "hvac" | "hvac terminal" | "ventilation" => [195, 205, 215], // galvanized sheet metal
                        "pipe" | "plumbing" => [90, 140, 190], // technical piping blue
                        "mep equipment" | "hvac equipment" | "equipment" | "storage equipment" | "treatment device" => [125, 135, 145], // machinery steel
                        "mep fitting" | "mep controller" | "distribution system" => [160, 170, 180],
                        "plate" => [175, 180, 185],
                        _ => m.freeform,
                    }
                } else if f.is_sphere() {
                    [225u8, 180, 80] // Warm architectural ochre for sphere
                } else {
                    m.freeform
                }
            }
            Element::Window(_) => [m.window_glass[0], m.window_glass[1], m.window_glass[2]],
            Element::Door(_) => m.door,
            _ => m.freeform,
        };

        // Render 2D Freeform Planar Surfaces (like SketchUp faces before extrusion)
        if let Element::FreeformSurface(surf) = el {
            if surf.boundary.len() >= 3 {
                let pts_3d: Vec<Vec3> = surf
                    .boundary
                    .iter()
                    .map(|p| Vec3::new(p.x, p.y, surf.elevation))
                    .collect();
                let face = Face::solid(pts_3d);
                if face.normal.dot(cam_forward) < -1e-4 {
                    render_cap_face(
                        &face,
                        &project_to_screen,
                        &transform_to_view,
                        light_dir,
                        [170, 215, 255],
                        visual_style,
                        theme,
                        false,
                        &mut faces_with_depth,
                    );
                } else {
                    // Double-sided surface
                    let rev_face = Face::solid(face.pts.into_iter().rev().collect());
                    if rev_face.normal.dot(cam_forward) < -1e-4 {
                        render_cap_face(
                            &rev_face,
                            &project_to_screen,
                            &transform_to_view,
                            light_dir,
                            [170, 215, 255],
                            visual_style,
                            theme,
                            false,
                            &mut faces_with_depth,
                        );
                    }
                }
            }
            continue;
        }

        // Render 3D Sphere Primitive or Freeform Solid
        if let Element::FreeformSolid(f) = el {
            // Invisible spatial zones (IfcSpace, IfcSite) or analytical elements must not be rendered as opaque solid obstacles
            if f.group_name.as_deref() == Some("Space")
                || f.ifc_category.as_deref() == Some("IFCSPACE")
                || f.group_name.as_deref() == Some("Site")
                || f.ifc_category.as_deref() == Some("IFCSITE")
            {
                continue;
            }
            if f.is_sphere() {
                if let (Some(radius), Some(center)) = (f.sphere_radius(), f.sphere_center()) {
                    let center_view = transform_to_view(center);

                    // 1. Shaded 3D sphere facet quads/triangles
                    for face in f.faces() {
                        if face.normal.dot(cam_forward) >= -1e-4 && visual_style != VisualStyle::Wireframe {
                            continue;
                        }

                        let f_center_view = transform_to_view(face.centroid());
                        let depth = f_center_view.y;

                        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();

                        if pts_2d.len() >= 3 {
                            let fill_color = match visual_style {
                                VisualStyle::Shaded => {
                                    let diffuse = face.normal.dot(light_dir).max(0.0) as f32;
                                    let ambient = 0.40f32;
                                    let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                                    let shade = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                                    [shade(base_rgb[0]), shade(base_rgb[1]), shade(base_rgb[2]), 255]
                                }
                                VisualStyle::HiddenLine => theme.background,
                                VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
                                VisualStyle::Wireframe => [0, 0, 0, 0],
                            };
                            let n = face.pts.len();
                            let hard = if visual_style == VisualStyle::Wireframe {
                                vec![true; n]
                            } else {
                                vec![false; n]
                            };
                            faces_with_depth.push((depth, pts_2d, fill_color, hard, face.centroid().z - 1.0, face.centroid().z + 1.0));
                        }
                    }

                    // 2. Equatorial crisp silhouette outline circle
                    let segs = 36;
                    let silhouette_pts: Vec<Vec2> = (0..segs)
                        .map(|k| {
                            let a = (k as f64 / segs as f64) * std::f64::consts::TAU;
                            vec2(center_view.x + radius * a.cos(), center_view.z + radius * a.sin())
                        })
                        .collect();
                    faces_with_depth.push((
                        center_view.y - 1.0,
                        silhouette_pts,
                        [0, 0, 0, 0],
                        vec![true; segs],
                        center.z - radius,
                        center.z + radius,
                    ));
                    continue;
                }
            } else if f.is_mesh() {
                for face in f.faces() {
                    if face.normal.dot(cam_forward) >= -1e-4 && visual_style != VisualStyle::Wireframe {
                        continue;
                    }

                    let f_center_view = transform_to_view(face.centroid());
                    let depth = f_center_view.y;

                    let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();

                    if pts_2d.len() >= 3 {
                        let fill_color = match visual_style {
                            VisualStyle::Shaded => {
                                let diffuse = face.normal.dot(light_dir).max(0.0) as f32;
                                let ambient = 0.40f32;
                                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                                let shade = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                                [shade(base_rgb[0]), shade(base_rgb[1]), shade(base_rgb[2]), 255]
                            }
                            VisualStyle::HiddenLine => theme.background,
                            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
                            VisualStyle::Wireframe => [0, 0, 0, 0],
                        };
                        // For Site topography or earthwork meshes, mark internal edges as soft so terrain does not produce ray/spiderweb lines
                        let hard_flags = if f.group_name.as_deref() == Some("Site") {
                            vec![false; pts_2d.len()]
                        } else {
                            face.hard.clone()
                        };
                        faces_with_depth.push((depth, pts_2d, fill_color, hard_flags, f.z_min, f.z_max));
                    }
                }
                continue;
            }
        }

        // Render Ceiling:
        // Ceiling is an interior horizontal finish mounted inside rooms.
        // It should ONLY emit its downward-facing bottom cap face (normal (0,0,-1))
        // when viewed looking up from below. It should NEVER emit side faces that cut through exterior walls!
        if let Element::Ceiling(c) = el {
            let base_z = doc.level_elevation(c.level);
            let top_z = c.top_level.map(|lvl| doc.level_elevation(lvl));
            let (ceil_z, ceil_top_z) = c.z_bounds(base_z, top_z);
            if !c.boundary.is_empty() {
                let centroid_2d = c.boundary.iter().fold(Vec2::ZERO, |a, &b| a + b) / (c.boundary.len() as f64);
                if is_enclosed_interior(doc, centroid_2d, ceil_z, ceil_top_z, occlude_from_above, occlude_from_below) {
                    continue;
                }
            }
            let normal = Vec3::new(0.0, 0.0, -1.0);
            if normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                let pts: Vec<Vec3> = c.boundary.iter().rev().map(|p| Vec3::new(p.x, p.y, ceil_z)).collect();
                let face = Face::solid(pts);
                render_cap_face(
                    &face,
                    &project_to_screen,
                    &transform_to_view,
                    light_dir,
                    base_rgb,
                    visual_style,
                    theme,
                    false,
                    &mut faces_with_depth,
                );
            }
            continue;
        }

        // Render Walls with exact B-Rep facade & recessed opening pockets (zero internal faces or seam lines)
        if let Element::Wall(w) = el {
            render_wall_solid(
                doc,
                w,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                occlude_from_above,
                occlude_from_below,
                &mut faces_with_depth,
            );
            continue;
        }

        // Hosted Doors and Windows are rendered directly inside their host wall's recessed opening pocket
        if let Element::Door(d) = el {
            if doc.elements.contains_key(&d.host_wall) {
                continue;
            }
        }
        if let Element::Window(win) = el {
            if doc.elements.contains_key(&win.host_wall) {
                continue;
            }
        }

        // Render Structural / Architectural Columns with watertight cap & wall-embed sorting
        if let Element::Column(c) = el {
            render_column_solid(
                doc,
                c,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Structural Beams with trimmed end-caps & soffit sorting
        if let Element::Beam(b) = el {
            let base_z = doc.level_elevation(b.level);
            let top_z = b.top_level.map(|lvl| doc.level_elevation(lvl));
            let (z0, z1) = b.z_bounds(base_z, top_z);
            let beam_mid = (b.start + b.end) * 0.5;
            if is_enclosed_interior(doc, beam_mid, z0, z1, occlude_from_above, occlude_from_below) {
                continue;
            }
            render_beam_solid(
                doc,
                b,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Curtain Walls with 3D mullions & glazed panels
        if let Element::CurtainWall(cw) = el {
            let base_z = doc.level_elevation(cw.base_level);
            let top_z = cw.top_level.map(|lvl| doc.level_elevation(lvl));
            render_curtain_wall_solid(
                cw,
                base_z,
                top_z,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Pitched & Flat Roofs (Hip, Gable, Shed, Flat)
        if let Element::Roof(r) = el {
            let base_z = doc.level_elevation(r.level);
            let top_z = r.top_level.map(|lvl| doc.level_elevation(lvl));
            render_roof_solid(
                r,
                base_z,
                top_z,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Stairs as watertight 3D solids (Monolithic, SolidStepped, OpenRiser)
        if let Element::Stair(s) = el {
            let base_z = doc.level_elevation(s.base_level);
            let top_z = s.top_level.map(|lvl| doc.level_elevation(lvl));
            let (z0, z1) = s.z_bounds(base_z, top_z);
            let stair_mid = (s.start + s.end) * 0.5;
            if is_enclosed_interior(doc, stair_mid, z0, z1, occlude_from_above, occlude_from_below) {
                continue;
            }
            render_stair_solid(
                s,
                base_z,
                top_z,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Ramps as smooth watertight 3D inclined solids (SlopedSlab, SolidWedge)
        if let Element::Ramp(r) = el {
            let base_z = doc.level_elevation(r.base_level);
            let top_z = r.top_level.map(|lvl| doc.level_elevation(lvl));
            let (z0, z1) = r.z_bounds(base_z, top_z);
            let ramp_mid = (r.start + r.end) * 0.5;
            if is_enclosed_interior(doc, ramp_mid, z0, z1, occlude_from_above, occlude_from_below) {
                continue;
            }
            render_ramp_solid(
                r,
                base_z,
                top_z,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Structural Foundation Footings (Pad, Stepped, Sloped) as opaque 3D solids
        if let Element::Footing(f) = el {
            let base_z = doc.level_elevation(f.base_level);
            let top_z = f.top_level.map(|lvl| doc.level_elevation(lvl));
            render_footing_solid(
                f,
                base_z,
                top_z,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Toposolids as 3D terrain volumetric solids
        if let Element::Toposolid(ts) = el {
            let base_z = doc.level_elevation(ts.level);
            let topo_rgb = [92, 132, 70]; // Earth / Turf Grass
            render_face_list_solid(
                &ts.faces(base_z),
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                topo_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Render Building Pads as solid concrete excavation pads
        if let Element::BuildingPad(bp) = el {
            let base_z = doc.level_elevation(bp.level);
            let pad_rgb = [170, 172, 178]; // Concrete excavation pad
            render_face_list_solid(
                &bp.faces(base_z),
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                pad_rgb,
                visual_style,
                theme,
                &mut faces_with_depth,
            );
            continue;
        }

        // Architectural Cap Visibility for remaining elements:
        // Slabs, Fascias, Gutters, Railings, and Freeform solids show top/bottom caps cleanly.
        let (render_top, render_bot) = match el {
            Element::Slab(s) => {
                let (s_bot, s_top) = s.z_bounds(
                    doc.level_elevation(s.level),
                    s.top_level.map(|l| doc.level_elevation(l)),
                );
                let centroid_2d = if s.boundary.is_empty() {
                    Vec2::ZERO
                } else {
                    s.boundary.iter().fold(Vec2::ZERO, |a, &b| a + b) / (s.boundary.len() as f64)
                };
                let is_ground_covered = s_top <= 1000.0
                    && occlude_from_above
                    && is_point_covered_above(doc, centroid_2d, s_top + 500.0);
                let is_roof_sealed_below = s_bot >= 2000.0
                    && occlude_from_below
                    && is_point_sealed_below(doc, centroid_2d, 500.0);
                (!is_ground_covered, !is_roof_sealed_below)
            }
            Element::Roof(_) => (true, !occlude_from_below),
            Element::Fascia(_) | Element::Gutter(_) | Element::Railing(_) => (true, true),
            Element::Beam(_) => (true, true),
            Element::Column(c) => {
                let base_z = doc.level_elevation(c.base_level);
                let top_z = c.top_level.map(|lvl| doc.level_elevation(lvl));
                let (_z0, z1) = c.z_bounds(base_z, top_z);
                let capped = c.top_level.is_some() || z1 >= 3300.0;
                (!capped, false)
            }
            Element::FreeformSolid(f) => {
                if f.group_name.as_deref() == Some("Space") || f.ifc_category.as_deref() == Some("IFCSPACE") {
                    (false, false)
                } else {
                    (true, true)
                }
            }
            Element::Window(_) | Element::Door(_) | Element::CurtainWall(_) => (false, false),
            _ => (true, true),
        };

        // Render solid prisms
        let is_roof_cap = match el {
            Element::Roof(_) => true,
            Element::Slab(s) => {
                let name_l = s.name.to_lowercase();
                let type_l = s.slab_type_name.to_lowercase();
                type_l.contains("roof") || name_l.contains("roof") || name_l.contains("dach")
            }
            _ => false,
        };
        let solids = doc.element_solids(id);
        for prism in solids {
            render_prism_solid(
                &prism,
                &project_to_screen,
                &transform_to_view,
                cam_forward,
                light_dir,
                base_rgb,
                render_top,
                render_bot,
                visual_style,
                theme,
                is_roof_cap,
                &mut faces_with_depth,
            );
        }
    }

    // Sort back-to-front (Painter's algorithm):
    // Depth Y in camera space (larger depth Y is drawn first, smaller depth Y draws on top)
    faces_with_depth.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
    });

    for (_depth, pts, fill, hard_flags, _z_min, _z_max) in faces_with_depth {
        let n = pts.len();
        if n < 3 {
            continue;
        }

        // 1. Solid opaque polygon fill
        if fill[3] > 0 {
            scene.add(RenderPrimitive::Polygon {
                points: pts.clone(),
                fill_color: fill,
                stroke_color: None,
                stroke_width: 0.0,
            });
        }

        // 2. Crisp edge outlines (only for hard boundary edges and silhouettes; internal diagonal chords remain hidden)
        for i in 0..n {
            if hard_flags.get(i).copied().unwrap_or(true) {
                scene.add(RenderPrimitive::Line {
                    start: pts[i],
                    end: pts[(i + 1) % n],
                    color: theme.cut_line,
                    width: 1.0,
                    dashed: false,
                });
            }
        }
    }

    scene
}

/// Renders a single solid prism with exact backface culling, lighting, silhouette extraction,
/// and bounded face subdivision for watertight Painter's depth sorting.
fn render_prism_solid(
    prism: &Prism,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    render_top: bool,
    render_bot: bool,
    visual_style: VisualStyle,
    theme: &ColorTheme,
    is_roof_cap: bool,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let n = prism.profile.len();
    if n < 3 || prism.height() < 1e-4 {
        return;
    }

    let all_faces = prism.faces();
    let total_faces = all_faces.len();
    if total_faces < 2 {
        return;
    }
    let num_side = total_faces - 2;

    // Precalculate visibility of all side facets using their true outward Newell normals
    let mut side_vis = Vec::with_capacity(num_side);
    for i in 0..num_side {
        let vis = all_faces[i].normal.dot(cam_forward) < -1e-4;
        side_vis.push(vis);
    }

    // Process side faces as clean, seamless quads
    for i in 0..num_side {
        if !side_vis[i] && visual_style != VisualStyle::Wireframe {
            continue; // Backface culled
        }

        let face = &all_faces[i];
        let is_silhouette_b = if num_side > 1 { !side_vis[(i + 1) % num_side] } else { true };
        let is_silhouette_a = if num_side > 1 { !side_vis[(i + num_side - 1) % num_side] } else { true };

        let base_hard = vec![
            true,
            face.hard.get(1).copied().unwrap_or(true) || is_silhouette_b,
            true,
            face.hard.get(3).copied().unwrap_or(true) || is_silhouette_a,
        ];

        let fill_color = match visual_style {
            VisualStyle::Shaded => {
                let diffuse = face.normal.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let shade = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [shade(base_rgb[0]), shade(base_rgb[1]), shade(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        };

        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        let center_view = transform_to_view(face.centroid());
        // Elevated/roof slab perimeter eave faces (z_min >= 2500) and ground foundation slab perimeter faces (z_max <= 100)
        // sit at the top/bottom extremities of the building and must draw in front of the vertical wall facades
        // where they meet, without ever drawing over their own visible top/bottom cap!
        let depth = if z_min >= 2500.0 && cam_forward.z < 0.0 {
            let min_corner_y = face
                .pts
                .iter()
                .map(|&p| transform_to_view(p).y)
                .fold(f64::INFINITY, f64::min);
            min_corner_y - 0.5
        } else if z_max <= 100.0 && cam_forward.z > 0.0 {
            let min_corner_y = face
                .pts
                .iter()
                .map(|&p| transform_to_view(p).y)
                .fold(f64::INFINITY, f64::min);
            min_corner_y - 0.5
        } else {
            center_view.y
        };
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();

        if pts_2d.len() >= 3 {
            faces_with_depth.push((depth, pts_2d, fill_color, base_hard, z_min, z_max));
        }
    }

    // Top cap (index num_side in all_faces)
    if render_top {
        if let Some(top_face) = all_faces.get(num_side) {
            if top_face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                render_cap_face(
                    top_face,
                    project_to_screen,
                    transform_to_view,
                    light_dir,
                    base_rgb,
                    visual_style,
                    theme,
                    is_roof_cap,
                    faces_with_depth,
                );
            }
        }
    }

    // Bottom cap (index num_side + 1 in all_faces)
    if render_bot {
        if let Some(bot_face) = all_faces.get(num_side + 1) {
            if bot_face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                render_cap_face(
                    bot_face,
                    project_to_screen,
                    transform_to_view,
                    light_dir,
                    base_rgb,
                    visual_style,
                    theme,
                    is_roof_cap,
                    faces_with_depth,
                );
            }
        }
    }
}

/// Renders a horizontal cap face (e.g. slab, ceiling, floor) with visual style support.
fn render_cap_face(
    face: &Face,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    is_roof_cap: bool,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    if face.pts.len() < 3 {
        return;
    }

    let fill_color = match visual_style {
        VisualStyle::Shaded => {
            let diffuse = face.normal.dot(light_dir).max(0.0) as f32;
            let ambient = 0.40f32;
            let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
            let shade = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
            [shade(base_rgb[0]), shade(base_rgb[1]), shade(base_rgb[2]), 255]
        }
        VisualStyle::HiddenLine => theme.background,
        VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
        VisualStyle::Wireframe => [0, 0, 0, 0],
    };

    let centroid = face.centroid();
    let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
    let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
    let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
    let hard_edges = face.hard.clone();

    // Render horizontal slab/roof/floor caps as a single monolithic polygon (zero internal grid seams!).
    // - Roof top caps (is_roof_cap, z >= 2500, normal.z > 0.5): anchor at camera-closest corner (min_corner_y - 1.0)
    //   so roof caps cleanly cover supporting walls.
    // - Ground floor top caps (z < 500, normal.z > 0.5): anchor at camera-furthest corner (max_corner_y + 1.0)
    //   so walls and columns standing on the floor always draw in front.
    // - Foundation bottom cap (z < 500, normal.z < -0.5): anchor at min_corner_y - 1.0 (watertight from below).
    // - Intermediate floor slabs / structural bay slabs: evaluate depth at centroid so they NEVER jump in front of beams or columns!
    let min_corner_y = face
        .pts
        .iter()
        .map(|&p| transform_to_view(p).y)
        .fold(f64::INFINITY, f64::min);
    let max_corner_y = face
        .pts
        .iter()
        .map(|&p| transform_to_view(p).y)
        .fold(f64::NEG_INFINITY, f64::max);

    let depth = if is_roof_cap && centroid.z >= 2500.0 && face.normal.z > 0.5 {
        min_corner_y - 1.0
    } else if centroid.z < 500.0 && face.normal.z > 0.5 {
        max_corner_y + 1.0
    } else if centroid.z < 500.0 && face.normal.z < -0.5 {
        min_corner_y - 1.0
    } else if is_roof_cap && centroid.z >= 2500.0 && face.normal.z < -0.5 {
        max_corner_y + 1.0
    } else {
        transform_to_view(centroid).y
    };

    faces_with_depth.push((depth, pts_2d, fill_color, hard_edges, z_min, z_max));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CutoutKind {
    Door,
    Window,
    CrossJoint,
    CurtainWall,
}

struct WallCutout {
    start: f64,
    end: f64,
    z_bot: f64,
    z_top: f64,
    kind: CutoutKind,
}

/// Exact 2D polygon + Z-elevation check: returns `true` if `pt` lies inside an elevated
/// topmost `Slab` or `Roof` envelope whose bottom soffit sits at or above `elem_z_top - 300mm`.
fn is_point_covered_above(doc: &Document, pt: Vec2, elem_z_top: f64) -> bool {
    doc.elements.values().any(|other| match other {
        Element::Slab(s) => {
            let (s_bot, s_top) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            if s_top < 1500.0 || s_bot < elem_z_top - 300.0 || !plinth_geom::polygon::point_in_polygon(pt, &s.boundary) {
                return false;
            }
            // A slab only occludes from above if it is the topmost slab/roof covering pt
            // (never an intermediate floor slab in a multi-storey building!)
            let has_higher_slab = doc.elements.values().any(|higher| match higher {
                Element::Slab(hs) if hs.id != s.id => {
                    let (_hb, ht) = hs.z_bounds(
                        doc.level_elevation(hs.level),
                        hs.top_level.map(|l| doc.level_elevation(l)),
                    );
                    ht > s_top + 100.0 && plinth_geom::polygon::point_in_polygon(pt, &hs.boundary)
                }
                Element::Roof(hr) => {
                    let re = doc.level_elevation(hr.level) + hr.offset;
                    re > s_top + 100.0
                }
                _ => false,
            });
            !has_higher_slab
        }
        Element::Roof(r) => {
            if r.boundary.len() < 3 {
                return false;
            }
            let r_eave = doc.level_elevation(r.level) + r.offset;
            if r_eave < elem_z_top - 450.0 {
                return false;
            }
            let oh = r.overhang.max(0.0);
            let min_x = r.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min) - oh;
            let max_x = r.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) + oh;
            let min_y = r.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - oh;
            let max_y = r.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) + oh;
            pt.x >= min_x && pt.x <= max_x && pt.y >= min_y && pt.y <= max_y
        }
        _ => false,
    })
}

/// Exact 2D polygon + Z-elevation check: returns `true` if `pt` lies inside a ground/foundation
/// `Slab` sitting at or below `elem_z_bot + 300mm`.
fn is_point_sealed_below(doc: &Document, pt: Vec2, elem_z_bot: f64) -> bool {
    doc.elements.values().any(|other| match other {
        Element::Slab(s) => {
            let (s_bot, s_top) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            s_top <= elem_z_bot + 300.0
                && s_bot <= elem_z_bot + 50.0
                && plinth_geom::polygon::point_in_polygon(pt, &s.boundary)
        }
        _ => false,
    })
}

/// Checks whether a point `(pt, z_target)` is directly capped/touched by a `Slab` or `Roof` at `z_target`.
fn is_point_touching_slab_or_roof_at_z(doc: &Document, pt: Vec2, z_target: f64) -> bool {
    doc.elements.values().any(|other| match other {
        Element::Slab(s) => {
            let (s_bot, s_top) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            z_target >= s_bot - 100.0
                && z_target <= s_top + 100.0
                && plinth_geom::polygon::point_in_polygon(pt, &s.boundary)
        }
        Element::Roof(r) => {
            if r.boundary.len() < 3 {
                return false;
            }
            let r_eave = doc.level_elevation(r.level) + r.offset;
            let _r_bot = r_eave - r.thickness.max(20.0);
            let oh = r.overhang.max(0.0);
            let min_x = r.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min) - oh;
            let max_x = r.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) + oh;
            let min_y = r.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - oh;
            let max_y = r.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) + oh;
            z_target <= r_eave + 150.0
                && z_target >= r_eave - 1200.0
                && pt.x >= min_x
                && pt.x <= max_x
                && pt.y >= min_y
                && pt.y <= max_y
        }
        _ => false,
    })
}

/// Checks whether `pt` is enclosed on all 4 cardinal sides (+X, -X, +Y, -Y) by walls overlapping `[z0, z1]`,
/// ensuring open structural frames (columns + beams + slabs with no perimeter walls) remain 100% visible in 3D.
fn is_surrounded_by_walls(doc: &Document, pt: Vec2, z0: f64, z1: f64) -> bool {
    let (mut hit_px, mut hit_nx, mut hit_py, mut hit_ny) = (false, false, false, false);
    for el in doc.elements.values() {
        if let Element::Wall(w) = el {
            let (wz0, wz1) = w.z_bounds(
                doc.level_elevation(w.base_level),
                w.top_level.map(|l| doc.level_elevation(l)),
            );
            if wz1 <= z0 + 100.0 || wz0 >= z1 - 100.0 {
                continue;
            }
            let min_x = w.start.x.min(w.end.x);
            let max_x = w.start.x.max(w.end.x);
            let min_y = w.start.y.min(w.end.y);
            let max_y = w.start.y.max(w.end.y);

            // Wall must be within local room proximity (10m) to enclose pt
            let d_max = 10000.0;
            if min_x > pt.x + d_max || max_x < pt.x - d_max || min_y > pt.y + d_max || max_y < pt.y - d_max {
                continue;
            }

            if pt.y >= min_y - 250.0 && pt.y <= max_y + 250.0 {
                if max_x >= pt.x + 150.0 {
                    hit_px = true;
                }
                if min_x <= pt.x - 150.0 {
                    hit_nx = true;
                }
            }
            if pt.x >= min_x - 250.0 && pt.x <= max_x + 250.0 {
                if max_y >= pt.y + 150.0 {
                    hit_py = true;
                }
                if min_y <= pt.y - 150.0 {
                    hit_ny = true;
                }
            }
            if hit_px && hit_nx && hit_py && hit_ny {
                return true;
            }
        }
    }
    false
}

/// Returns `true` ONLY if `pt` and its `±500mm` neighborhood lie strictly inside a wall-enclosed,
/// covered building envelope (so perimeter walls, open structural frames, and standalone courtyard
/// elements are NEVER culled).
fn is_enclosed_interior(
    doc: &Document,
    pt: Vec2,
    z0: f64,
    z1: f64,
    occlude_from_above: bool,
    occlude_from_below: bool,
) -> bool {
    if (!occlude_from_above && !occlude_from_below) || !is_surrounded_by_walls(doc, pt, z0, z1) {
        return false;
    }
    let probes = [
        pt,
        pt + vec2(500.0, 0.0),
        pt - vec2(500.0, 0.0),
        pt + vec2(0.0, 500.0),
        pt - vec2(0.0, 500.0),
    ];
    if occlude_from_above && probes.iter().all(|&p| is_point_covered_above(doc, p, z1)) {
        return true;
    }
    if occlude_from_below && probes.iter().all(|&p| is_point_sealed_below(doc, p, z0)) {
        return true;
    }
    false
}

/// Renders an architectural wall with seamless L-corners, T-joints, Cross-joints,
/// monolithic coplanar facades (zero vertical split seams above/below openings),
/// and 3D recessed window/door pockets with exact pocket culling.
fn render_wall_solid(
    doc: &Document,
    w: &plinth_model::Wall,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    occlude_from_above: bool,
    occlude_from_below: bool,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let base_z = doc.level_elevation(w.base_level);
    let top_z = w.top_level.map(|lvl| doc.level_elevation(lvl));
    let (z0, z1) = w.z_bounds(base_z, top_z);
    let wall_len = w.length();
    if wall_len < 1.0 || z1 - z0 < 1.0 {
        return;
    }

    let dir = w.direction();
    let norm = w.normal();
    let half_t = w.thickness * 0.5;
    let z_mid = (z0 + z1) * 0.5;
    let wall_mid = (w.start + w.end) * 0.5;

    let all_walls: Vec<&plinth_model::Wall> = doc
        .elements
        .values()
        .filter_map(|e| match e {
            Element::Wall(other) => Some(other),
            _ => None,
        })
        .collect();

    let (poly, start_joined, end_joined) = w.mitered_footprint(&all_walls);
    let p_start_left = poly[0];
    let p_end_left = poly[1];
    let p_end_right = poly[2];
    let p_start_right = poly[3];

    // Check if THIS wall's top at `wall_mid` is actually covered by a slab or roof
    let covered = is_point_touching_slab_or_roof_at_z(doc, wall_mid, z1);

    // Gather hosted doors, windows, and cross-joint intersections
    let mut cutouts = Vec::new();
    for el in doc.elements.values() {
        match el {
            Element::Door(d) if d.host_wall == w.id => {
                let s = (d.offset_along_wall - d.width * 0.5).max(0.0);
                let e = (d.offset_along_wall + d.width * 0.5).min(wall_len);
                if e > s + 1.0 {
                    let d_base_z = d.base_level.map(|l| doc.level_elevation(l)).unwrap_or(z0);
                    let d_top_z = d.top_level.map(|l| doc.level_elevation(l));
                    let (raw_zb, raw_zt) = d.z_bounds(d_base_z, d_top_z);
                    let zb = raw_zb.clamp(z0, z1);
                    let zt = raw_zt.clamp(z0, z1);
                    cutouts.push(WallCutout {
                        start: s,
                        end: e,
                        z_bot: zb,
                        z_top: zt,
                        kind: CutoutKind::Door,
                    });
                }
            }
            Element::Window(win) if win.host_wall == w.id => {
                let s = (win.offset_along_wall - win.width * 0.5).max(0.0);
                let e = (win.offset_along_wall + win.width * 0.5).min(wall_len);
                if e > s + 1.0 {
                    let w_base_z = win.base_level.map(|l| doc.level_elevation(l)).unwrap_or(z0);
                    let w_top_z = win.top_level.map(|l| doc.level_elevation(l));
                    let (raw_zb, raw_zt) = win.z_bounds(w_base_z, w_top_z);
                    let zb = raw_zb.clamp(z0, z1);
                    let zt = raw_zt.clamp(z0, z1);
                    cutouts.push(WallCutout {
                        start: s,
                        end: e,
                        z_bot: zb,
                        z_top: zt,
                        kind: CutoutKind::Window,
                    });
                }
            }
            Element::CurtainWall(cw) if cw.automatically_embed => {
                let is_hosted = cw.host_wall == Some(w.id);
                let is_collinear = if cw.host_wall.is_none() {
                    let cw_dir = cw.direction();
                    cw_dir.dot(dir).abs() > 0.98 && (cw.start - w.start).dot(norm).abs() < 50.0
                } else {
                    false
                };
                if is_hosted || is_collinear {
                    let s_dist = (cw.start - w.start).dot(dir);
                    let e_dist = (cw.end - w.start).dot(dir);
                    let s = s_dist.min(e_dist).max(0.0);
                    let e = s_dist.max(e_dist).min(wall_len);
                    if e > s + 1.0 {
                        let cw_base_z = doc.level_elevation(cw.base_level);
                        let cw_top_z = cw.top_level.map(|l| doc.level_elevation(l));
                        let (raw_zb, raw_zt) = cw.z_bounds(cw_base_z, cw_top_z);
                        let zb = raw_zb.clamp(z0, z1);
                        let zt = raw_zt.clamp(z0, z1);
                        if zt > zb + 1.0 {
                            cutouts.push(WallCutout {
                                start: s,
                                end: e,
                                z_bot: zb,
                                z_top: zt,
                                kind: CutoutKind::CurtainWall,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (cs, ce) in w.cross_joint_spans(&all_walls) {
        if ce > cs + 1.0 {
            cutouts.push(WallCutout {
                start: cs,
                end: ce,
                z_bot: z0,
                z_top: z1,
                kind: CutoutKind::CrossJoint,
            });
        }
    }
    cutouts.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    let pt_left = |d: f64| -> Vec2 {
        if d <= 0.5 {
            p_start_left
        } else if d >= wall_len - 0.5 {
            p_end_left
        } else {
            w.start + dir * d - norm * half_t
        }
    };
    let pt_right = |d: f64| -> Vec2 {
        if d <= 0.5 {
            p_start_right
        } else if d >= wall_len - 0.5 {
            p_end_right
        } else {
            w.start + dir * d + norm * half_t
        }
    };

    let shade_face = |n_vec: Vec3, rgb: [u8; 3]| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let s = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [s(rgb[0]), s(rgb[1]), s(rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [rgb[0], rgb[1], rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    let push_quad = |faces: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
                     pts_3d: [Vec3; 4],
                     depth: f64,
                     fill: [u8; 4],
                     hard: [bool; 4]| {
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        let hard_vec = hard.to_vec();
        let z_min = pts_3d.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = pts_3d.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces.push((depth, pts_2d, fill, hard_vec, z_min, z_max));
    };

    // Helper to emit a pier span [d0, d1] as a single monolithic quad (zero anti-aliasing seam lines!)
    let emit_pier_span = |faces: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
                          d0: f64,
                          d1: f64,
                          side: f64,
                          side_fill: [u8; 4]| {
        let span_len = d1 - d0;
        if span_len <= 1.0 {
            return;
        }
        let is_start = d0 <= 0.5;
        let is_end = d1 >= wall_len - 0.5;
        let p0 = if side < 0.0 { pt_left(d0) } else { pt_right(d0) };
        let p1 = if side < 0.0 { pt_left(d1) } else { pt_right(d1) };
        let mid_pt = (p0 + p1) * 0.5;
        let depth = transform_to_view(mid_pt.extend(z_mid)).y;
        push_quad(
            faces,
            [p0.extend(z0), p1.extend(z0), p1.extend(z1), p0.extend(z1)],
            depth,
            side_fill,
            [true, is_end, true, is_start],
        );
    };

    let panel_half_t = 22.0_f64.min(half_t * 0.35);

    // 1. Render visible side(s) of the wall (-norm and +norm).
    // A side face is culled as an enclosed indoor face ONLY when this wall sits on the perimeter of a
    // roofed/sealed building (so `probe_this` lies inside the roof/slab envelope while `probe_opp` is outdoors).
    // Standalone walls, L-walls, courtyard walls, and unroofed room walls render BOTH faces 100% visibly!
    for side in [-1.0_f64, 1.0_f64] {
        let side_normal_2d = norm * side;
        let probe_this = wall_mid + side_normal_2d * (half_t + 600.0);
        let probe_opp = wall_mid - side_normal_2d * (half_t + 600.0);
        let is_sealed_indoor_above = occlude_from_above
            && is_point_covered_above(doc, probe_this, z1)
            && !is_point_covered_above(doc, probe_opp, z1);
        let is_sealed_indoor_below = occlude_from_below
            && is_point_sealed_below(doc, probe_this, z0)
            && !is_point_sealed_below(doc, probe_opp, z0);
        if is_sealed_indoor_above || is_sealed_indoor_below {
            continue;
        }
        let side_normal = Vec3::new(norm.x * side, norm.y * side, 0.0);
        let side_vis = side_normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !side_vis {
            continue;
        }

        let side_fill = shade_face(side_normal, base_rgb);
        let mut curr_dist = 0.0;

        for (idx, c) in cutouts.iter().enumerate() {
            let prev_start = curr_dist;
            let op_start = c.start.max(curr_dist);
            let op_end = c.end.max(op_start);
            let next_end = if idx + 1 < cutouts.len() {
                cutouts[idx + 1].start.max(op_end).min(wall_len)
            } else {
                wall_len
            };

            if op_start > curr_dist + 1.0 {
                emit_pier_span(faces_with_depth, curr_dist, op_start, side, side_fill);
            }

            if op_end > op_start + 1.0 {
                let p_out0 = if side < 0.0 { pt_left(op_start) } else { pt_right(op_start) };
                let p_out1 = if side < 0.0 { pt_left(op_end) } else { pt_right(op_end) };
                let mid_out = (p_out0 + p_out1) * 0.5;
                let facade_depth = transform_to_view(mid_out.extend(z_mid)).y;

                let p_prev0 = if side < 0.0 { pt_left(prev_start) } else { pt_right(prev_start) };
                let p_next1 = if side < 0.0 { pt_left(next_end) } else { pt_right(next_end) };
                let prev_pier_depth = if op_start > prev_start + 1.0 {
                    transform_to_view(((p_prev0 + p_out0) * 0.5).extend(z_mid)).y
                } else {
                    facade_depth
                };
                let next_pier_depth = if next_end > op_end + 1.0 {
                    transform_to_view(((p_out1 + p_next1) * 0.5).extend(z_mid)).y
                } else {
                    facade_depth
                };
                let closer_pier_depth = prev_pier_depth.min(next_pier_depth);
                let base_pocket_depth = facade_depth.max(closer_pier_depth + 0.4);
                let panel_depth = base_pocket_depth - 0.1;
                let reveal_depth = base_pocket_depth - 0.2;
                let sill_lintel_depth = closer_pier_depth.min(facade_depth) - 0.1;
                let cutout_frame_depth = closer_pier_depth.min(facade_depth) - 0.2;

                let is_start = op_start <= 0.5;
                let is_end = op_end >= wall_len - 0.5;
                // Slightly widen sill and lintel fills by 8mm into adjacent piers (outside Wireframe) to seal coplanar anti-aliasing seams without touching the opening hole
                let s_d0 = if is_start || visual_style == VisualStyle::Wireframe {
                    op_start
                } else {
                    (op_start - 8.0).max(0.0)
                };
                let s_d1 = if is_end || visual_style == VisualStyle::Wireframe {
                    op_end
                } else {
                    (op_end + 8.0).min(wall_len)
                };
                let s_out0 = if side < 0.0 { pt_left(s_d0) } else { pt_right(s_d0) };
                let s_out1 = if side < 0.0 { pt_left(s_d1) } else { pt_right(s_d1) };

                // Sill facade quad (coplanar with wall face: no vertical seam lines at op_start/op_end; top sill edge is drawn by Cutout Frame)
                if c.z_bot > z0 + 1.0 {
                    push_quad(
                        faces_with_depth,
                        [
                            s_out0.extend(z0),
                            s_out1.extend(z0),
                            s_out1.extend(c.z_bot),
                            s_out0.extend(c.z_bot),
                        ],
                        sill_lintel_depth,
                        side_fill,
                        [true, is_end, false, is_start],
                    );
                }

                // Lintel facade quad (coplanar with wall face: no vertical seam lines at op_start/op_end; bottom head edge is drawn by Cutout Frame)
                if z1 > c.z_top + 1.0 {
                    push_quad(
                        faces_with_depth,
                        [
                            s_out0.extend(c.z_top),
                            s_out1.extend(c.z_top),
                            s_out1.extend(z1),
                            s_out0.extend(z1),
                        ],
                        sill_lintel_depth,
                        side_fill,
                        [false, is_end, true, is_start],
                    );
                }

                // Recessed Door/Window Pocket on this visible wall face
                if c.kind != CutoutKind::CrossJoint && c.kind != CutoutKind::CurtainWall {
                    let p_in0 = w.start + dir * op_start + norm * (side * panel_half_t);
                    let p_in1 = w.start + dir * op_end + norm * (side * panel_half_t);

                    // 1. Recessed Door or Window Panel (skip in Wireframe so it doesn't draw a 3rd inner floating frame)
                    if visual_style != VisualStyle::Wireframe {
                        let panel_rgb = match c.kind {
                            CutoutKind::Door => [185u8, 150, 120],
                            CutoutKind::Window => [165u8, 205, 235],
                            CutoutKind::CrossJoint | CutoutKind::CurtainWall => base_rgb,
                        };
                        let panel_fill = shade_face(side_normal, panel_rgb);
                        push_quad(
                            faces_with_depth,
                            [
                                p_in0.extend(c.z_bot),
                                p_in1.extend(c.z_bot),
                                p_in1.extend(c.z_top),
                                p_in0.extend(c.z_top),
                            ],
                            panel_depth,
                            panel_fill,
                            [true, true, true, true],
                        );

                        // 2. Visible Jamb Reveals (between outer wall face and recessed panel)
                        let jamb_l_norm = Vec3::new(dir.x, dir.y, 0.0);
                        if jamb_l_norm.dot(cam_forward) < -1e-4 {
                            let jamb_fill = shade_face(jamb_l_norm, base_rgb);
                            push_quad(
                                faces_with_depth,
                                [
                                    p_out0.extend(c.z_bot),
                                    p_in0.extend(c.z_bot),
                                    p_in0.extend(c.z_top),
                                    p_out0.extend(c.z_top),
                                ],
                                reveal_depth,
                                jamb_fill,
                                [true, true, true, true],
                            );
                        }

                        let jamb_r_norm = Vec3::new(-dir.x, -dir.y, 0.0);
                        if jamb_r_norm.dot(cam_forward) < -1e-4 {
                            let jamb_fill = shade_face(jamb_r_norm, base_rgb);
                            push_quad(
                                faces_with_depth,
                                [
                                    p_in1.extend(c.z_bot),
                                    p_out1.extend(c.z_bot),
                                    p_out1.extend(c.z_top),
                                    p_in1.extend(c.z_top),
                                ],
                                reveal_depth,
                                jamb_fill,
                                [true, true, true, true],
                            );
                        }

                        // 3. Visible Sill Top Reveal (at c.z_bot)
                        if c.z_bot > z0 + 1.0 && Vec3::Z.dot(cam_forward) < -1e-4 {
                            let sill_fill = shade_face(Vec3::Z, base_rgb);
                            push_quad(
                                faces_with_depth,
                                [
                                    p_out0.extend(c.z_bot),
                                    p_out1.extend(c.z_bot),
                                    p_in1.extend(c.z_bot),
                                    p_in0.extend(c.z_bot),
                                ],
                                reveal_depth,
                                sill_fill,
                                [true, true, true, true],
                            );
                        }

                        // 4. Visible Lintel Soffit Reveal (at c.z_top)
                        if z1 > c.z_top + 1.0 && (-Vec3::Z).dot(cam_forward) < -1e-4 {
                            let soffit_fill = shade_face(-Vec3::Z, base_rgb);
                            push_quad(
                                faces_with_depth,
                                [
                                    p_in0.extend(c.z_top),
                                    p_in1.extend(c.z_top),
                                    p_out1.extend(c.z_top),
                                    p_out0.extend(c.z_top),
                                ],
                                reveal_depth,
                                soffit_fill,
                                [true, true, true, true],
                            );
                        }
                    } else if side < 0.0 {
                        // In Wireframe mode, emit clean through-wall opening reveal connecting -norm and +norm faces once
                        let pl0 = pt_left(op_start);
                        let pl1 = pt_left(op_end);
                        let pr0 = pt_right(op_start);
                        let pr1 = pt_right(op_end);
                        // Left & Right Jamb thickness lines at sill and head
                        push_quad(
                            faces_with_depth,
                            [
                                pl0.extend(c.z_bot),
                                pr0.extend(c.z_bot),
                                pr0.extend(c.z_top),
                                pl0.extend(c.z_top),
                            ],
                            reveal_depth,
                            [0, 0, 0, 0],
                            [c.z_bot > z0 + 1.0, false, z1 > c.z_top + 1.0, false],
                        );
                        push_quad(
                            faces_with_depth,
                            [
                                pl1.extend(c.z_bot),
                                pr1.extend(c.z_bot),
                                pr1.extend(c.z_top),
                                pl1.extend(c.z_top),
                            ],
                            reveal_depth,
                            [0, 0, 0, 0],
                            [c.z_bot > z0 + 1.0, false, z1 > c.z_top + 1.0, false],
                        );
                    }

                    // 5. Crisp Outer Opening Cutout Frame on Wall Facade (Sill, Right Jamb, Lintel Head, Left Jamb)
                    push_quad(
                        faces_with_depth,
                        [
                            p_out0.extend(c.z_bot),
                            p_out1.extend(c.z_bot),
                            p_out1.extend(c.z_top),
                            p_out0.extend(c.z_top),
                        ],
                        cutout_frame_depth,
                        [0, 0, 0, 0],
                        [c.z_bot > z0 + 1.0, true, z1 > c.z_top + 1.0, true],
                    );
                }
            }

            curr_dist = op_end;
        }

        if wall_len > curr_dist + 1.0 {
            emit_pier_span(faces_with_depth, curr_dist, wall_len, side, side_fill);
        }
    }

    // 2. Unjoined Wall End-Caps (suppressed when joined at L-corners or T-joints)
    if !start_joined {
        let start_norm = Vec3::new(-dir.x, -dir.y, 0.0);
        if start_norm.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
            let fill = shade_face(start_norm, base_rgb);
            let mid = (p_start_left + p_start_right) * 0.5;
            let depth = transform_to_view(mid.extend(z_mid)).y;
            push_quad(
                faces_with_depth,
                [
                    p_start_right.extend(z0),
                    p_start_left.extend(z0),
                    p_start_left.extend(z1),
                    p_start_right.extend(z1),
                ],
                depth,
                fill,
                [true, true, true, true],
            );
        }
    }
    if !end_joined {
        let end_norm = Vec3::new(dir.x, dir.y, 0.0);
        if end_norm.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
            let fill = shade_face(end_norm, base_rgb);
            let mid = (p_end_left + p_end_right) * 0.5;
            let depth = transform_to_view(mid.extend(z_mid)).y;
            push_quad(
                faces_with_depth,
                [
                    p_end_left.extend(z0),
                    p_end_right.extend(z0),
                    p_end_right.extend(z1),
                    p_end_left.extend(z1),
                ],
                depth,
                fill,
                [true, true, true, true],
            );
        }
    }

    // 3. Continuous Wall Top Cap at z1 (when not covered by a roof slab)
    if !covered && (Vec3::Z.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe) {
        let top_fill = shade_face(Vec3::Z, base_rgb);
        let front_side = if Vec3::new(-norm.x, -norm.y, 0.0).dot(cam_forward) < 0.0 {
            -1.0
        } else {
            1.0
        };

        let emit_top_span = |faces: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>, d0: f64, d1: f64| {
            let span_len = d1 - d0;
            if span_len <= 1.0 {
                return;
            }
            let pl0 = pt_left(d0);
            let pl1 = pt_left(d1);
            let pr1 = pt_right(d1);
            let pr0 = pt_right(d0);
            let front_mid = if front_side < 0.0 {
                (pl0 + pl1) * 0.5
            } else {
                (pr0 + pr1) * 0.5
            };
            // Evaluating top cap depth at the front-most edge at z1 guarantees it always draws on top of the wall facade (at z_mid)
            let depth = transform_to_view(front_mid.extend(z1)).y - 0.5;
            let hard_start = d0 <= 0.5 && !start_joined;
            let hard_end = d1 >= wall_len - 0.5 && !end_joined;
            push_quad(
                faces,
                [pl0.extend(z1), pl1.extend(z1), pr1.extend(z1), pr0.extend(z1)],
                depth,
                top_fill,
                [true, hard_end, true, hard_start],
            );
        };

        let mut curr_top = 0.0;
        for c in &cutouts {
            // Only full-height cutouts ( reaching z1, like CrossJoints) interrupt the top cap at z1
            if c.z_top >= z1 - 1.0 {
                let cs = c.start.max(curr_top);
                let ce = c.end.max(cs);
                if cs > curr_top + 1.0 {
                    emit_top_span(faces_with_depth, curr_top, cs);
                }
                curr_top = ce;
            }
        }
        if wall_len > curr_top + 1.0 {
            emit_top_span(faces_with_depth, curr_top, wall_len);
        }
    }
}

/// Renders a Structural or Architectural Column with watertight B-Rep culling,
/// exact top/bottom cap depth anchoring, and clean protrusion in front of embedded walls.
fn render_column_solid(
    doc: &Document,
    c: &plinth_model::Column,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let base_z = doc.level_elevation(c.base_level);
    let top_z = c.top_level.map(|lvl| doc.level_elevation(lvl));
    let (z0, z1) = c.z_bounds(base_z, top_z);
    if z1 - z0 < 1.0 {
        return;
    }

    let profile = c.footprint();
    let n = profile.len();
    if n < 3 {
        return;
    }

    let z_mid = (z0 + z1) * 0.5;

    let bot_covered = doc.elements.values().any(|other| match other {
        Element::Slab(s) => {
            let (s_bot, s_top) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            z0 >= s_bot - 100.0
                && z0 <= s_top + 100.0
                && plinth_geom::polygon::point_in_polygon(c.location, &s.boundary)
        }
        _ => false,
    });

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let s = |ch: u8| -> u8 { ((ch as f32) * intensity).min(255.0) as u8 };
                [s(base_rgb[0]), s(base_rgb[1]), s(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    // Precalculate outward normals and visibility for each vertical facet
    let mut side_normals = Vec::with_capacity(n);
    let mut side_vis = Vec::with_capacity(n);
    for i in 0..n {
        let p0 = profile[i];
        let p1 = profile[(i + 1) % n];
        let edge_dir = (p1 - p0).normalized();
        // Profile is CCW, so outward 2D normal is -edge_dir.perp() = (edge_dir.y, -edge_dir.x)
        let out_2d = Vec2::new(edge_dir.y, -edge_dir.x);
        let out_3d = Vec3::new(out_2d.x, out_2d.y, 0.0);
        side_normals.push(out_3d);
        side_vis.push(out_3d.dot(cam_forward) < -1e-4);
    }

    // 1. Vertical Side Facets
    // Anchor side depth at the camera-closest edge at z_mid (-1.0 bias) so columns embedded in walls
    // always project cleanly in front of the host wall without being sliced by long wall spans.
    for i in 0..n {
        if !side_vis[i] && visual_style != VisualStyle::Wireframe {
            continue;
        }
        let p0 = profile[i];
        let p1 = profile[(i + 1) % n];
        let pts_3d = [
            p0.extend(z0),
            p1.extend(z0),
            p1.extend(z1),
            p0.extend(z1),
        ];
        let y0 = transform_to_view(p0.extend(z_mid)).y;
        let y1 = transform_to_view(p1.extend(z_mid)).y;
        let depth = y0.min(y1) - 1.0;

        let is_silhouette_next = !side_vis[(i + 1) % n];
        let is_silhouette_prev = !side_vis[(i + n - 1) % n];
        let hard = if c.is_round && visual_style != VisualStyle::Wireframe {
            vec![true, is_silhouette_next, true, is_silhouette_prev]
        } else {
            vec![true, true, true, true]
        };

        let fill = shade_face(side_normals[i]);
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        faces_with_depth.push((depth, pts_2d, fill, hard, z0, z1));
    }

    // 2. Column Top Cap at z1 (always solid when viewed from above, never empty or transparent!)
    if Vec3::Z.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
        let pts_3d: Vec<Vec3> = profile.iter().map(|&p| p.extend(z1)).collect();
        let min_y = pts_3d
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        // Evaluate top cap depth at its camera-closest point at z1 (-1.5 bias) so it always caps the vertical column sides cleanly!
        let depth = min_y - 1.5;
        let fill = shade_face(Vec3::Z);
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        let hard = vec![true; n];
        faces_with_depth.push((depth, pts_2d, fill, hard, z1, z1));
    }

    // 3. Column Bottom Cap at z0 (when freestanding and viewed from below)
    if !bot_covered && ((-Vec3::Z).dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe) {
        let pts_3d: Vec<Vec3> = profile.iter().rev().map(|&p| p.extend(z0)).collect();
        let min_y = pts_3d
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        let depth = min_y - 1.5;
        let fill = shade_face(-Vec3::Z);
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        let hard = vec![true; n];
        faces_with_depth.push((depth, pts_2d, fill, hard, z0, z0));
    }
}

/// Computes the positive exit distance along `ray_dir` from `origin` through a Column footprint.
pub(crate) fn column_ray_exit_distance(
    c: &plinth_model::Column,
    origin: Vec2,
    ray_dir: Vec2,
) -> Option<f64> {
    if c.is_round {
        let r = c.width * 0.5;
        let v = origin - c.location;
        let b = v.dot(ray_dir);
        let c_quad = v.length_sq() - r * r;
        let disc = b * b - c_quad;
        if disc < 0.0 {
            return None;
        }
        let t = -b + disc.sqrt();
        if t > 0.0 {
            Some(t)
        } else {
            None
        }
    } else {
        let ux = Vec2::from_angle(c.rotation);
        let uy = ux.perp();
        let hx = c.width * 0.5;
        let hy = c.depth * 0.5;
        let v = origin - c.location;
        let px = v.dot(ux);
        let py = v.dot(uy);
        let dx = ray_dir.dot(ux);
        let dy = ray_dir.dot(uy);

        let (t_min_x, t_max_x) = if dx.abs() > 1e-6 {
            let t1 = (-hx - px) / dx;
            let t2 = (hx - px) / dx;
            (t1.min(t2), t1.max(t2))
        } else {
            if px < -hx - 25.0 || px > hx + 25.0 {
                return None;
            }
            (-f64::INFINITY, f64::INFINITY)
        };

        let (t_min_y, t_max_y) = if dy.abs() > 1e-6 {
            let t1 = (-hy - py) / dy;
            let t2 = (hy - py) / dy;
            (t1.min(t2), t1.max(t2))
        } else {
            if py < -hy - 25.0 || py > hy + 25.0 {
                return None;
            }
            (-f64::INFINITY, f64::INFINITY)
        };

        let t_enter = t_min_x.max(t_min_y);
        let t_exit = t_max_x.min(t_max_y);

        if t_enter > t_exit {
            return None;
        }

        if t_exit > 0.0 {
            Some(t_exit)
        } else {
            None
        }
    }
}

/// Renders a Structural Beam with trimmed end-caps, exact soffit/top-flange depth sorting,
/// and clean joint integration with supporting columns, walls, and connecting beams.
fn render_beam_solid(
    doc: &Document,
    b: &plinth_model::Beam,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let beam_len = b.start.distance(b.end);
    if beam_len < 1.0 || b.depth < 1.0 || b.width < 1.0 {
        return;
    }

    let base_z = doc.level_elevation(b.level);
    let top_z = b.top_level.map(|lvl| doc.level_elevation(lvl));
    let (z_bot, z_top) = b.z_bounds(base_z, top_z);
    if z_top - z_bot < 1.0 {
        return;
    }
    let z_mid = (z_bot + z_top) * 0.5;

    let dir = (b.end - b.start).normalized();
    let norm = dir.perp();
    let half_w = b.width * 0.5;

    // Trim beam start/end against supporting columns, walls, or framing beams so beams stop
    // cleanly at column/wall boundaries and never penetrate interior spaces or overlap.
    let mut d_start = 0.0_f64;
    let mut d_end = beam_len;
    let mut start_joined = false;
    let mut end_joined = false;

    for other in doc.elements.values() {
        match other {
            Element::Column(c) => {
                let (c_z0, c_z1) = c.z_bounds(
                    doc.level_elevation(c.base_level),
                    c.top_level.map(|lvl| doc.level_elevation(lvl)),
                );
                // Vertical elevation check: column must overlap beam vertical span
                if z_bot < c_z1 - 10.0 && z_top > c_z0 + 10.0 {
                    let c_diag = (c.width * 0.5).hypot(c.depth * 0.5);
                    if b.start.distance(c.location) <= c_diag + 25.0 {
                        start_joined = true;
                        if let Some(t_exit) = column_ray_exit_distance(c, b.start, dir) {
                            if t_exit > 0.0 && t_exit < beam_len * 0.48 {
                                d_start = d_start.max(t_exit);
                            }
                        }
                    }
                    if b.end.distance(c.location) <= c_diag + 25.0 {
                        end_joined = true;
                        if let Some(t_exit) = column_ray_exit_distance(c, b.end, -dir) {
                            if t_exit > 0.0 && t_exit < beam_len * 0.48 {
                                d_end = d_end.min(beam_len - t_exit);
                            }
                        }
                    }
                }
            }
            Element::Wall(w) => {
                let (w_z0, w_z1) = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|l| doc.level_elevation(l)),
                );
                if z_bot < w_z1 - 10.0 && z_top > w_z0 + 10.0 {
                    let w_dir = w.direction();
                    if dir.dot(w_dir).abs() < 0.90 {
                        let w_len = w.length();
                        let w_half_t = w.thickness * 0.5;
                        let w_norm = w.normal();
                        let denom_s = dir.dot(w_norm).abs();
                        let denom_e = (-dir).dot(w_norm).abs();

                        // Check if b.start is inside or on this wall
                        let v_s = b.start - w.start;
                        let along_s = v_s.dot(w_dir);
                        let perp_s = v_s.dot(w_norm).abs();
                        if along_s >= -25.0 && along_s <= w_len + 25.0 && perp_s <= w_half_t + 25.0 {
                            start_joined = true;
                            if denom_s > 0.15 {
                                let t = (w_half_t - v_s.dot(w_norm) * dir.dot(w_norm).signum()) / denom_s;
                                if t > 0.0 && t < beam_len * 0.48 {
                                    d_start = d_start.max(t);
                                }
                            }
                        }
                        // Check if b.end is inside or on this wall
                        let v_e = b.end - w.start;
                        let along_e = v_e.dot(w_dir);
                        let perp_e = v_e.dot(w_norm).abs();
                        if along_e >= -25.0 && along_e <= w_len + 25.0 && perp_e <= w_half_t + 25.0 {
                            end_joined = true;
                            if denom_e > 0.15 {
                                let t = (w_half_t - v_e.dot(w_norm) * (-dir).dot(w_norm).signum()) / denom_e;
                                if t > 0.0 && t < beam_len * 0.48 {
                                    d_end = d_end.min(beam_len - t);
                                }
                            }
                        }
                    }
                }
            }
            Element::Beam(other_b) => {
                if other_b.id != b.id {
                    let (other_z0, other_z1) = other_b.z_bounds(
                        doc.level_elevation(other_b.level),
                        other_b.top_level.map(|lvl| doc.level_elevation(lvl)),
                    );
                    if z_bot < other_z1 - 10.0 && z_top > other_z0 + 10.0 {
                        let other_len = other_b.start.distance(other_b.end);
                        if other_len > 1.0 {
                            let other_dir = (other_b.end - other_b.start).normalized();
                            if dir.dot(other_dir).abs() < 0.90 {
                                let other_norm = other_dir.perp();
                                let other_half_w = other_b.width * 0.5;
                                let denom_s = dir.dot(other_norm).abs();
                                let denom_e = (-dir).dot(other_norm).abs();

                                let v_s = b.start - other_b.start;
                                let along_s = v_s.dot(other_dir);
                                let perp_s = v_s.dot(other_norm).abs();
                                if along_s >= -25.0 && along_s <= other_len + 25.0 && perp_s <= other_half_w + 25.0 {
                                    start_joined = true;
                                    if denom_s > 0.15 {
                                        let t = (other_half_w - v_s.dot(other_norm) * dir.dot(other_norm).signum()) / denom_s;
                                        if t > 0.0 && t < beam_len * 0.48 {
                                            d_start = d_start.max(t);
                                        }
                                    }
                                }

                                let v_e = b.end - other_b.start;
                                let along_e = v_e.dot(other_dir);
                                let perp_e = v_e.dot(other_norm).abs();
                                if along_e >= -25.0 && along_e <= other_len + 25.0 && perp_e <= other_half_w + 25.0 {
                                    end_joined = true;
                                    if denom_e > 0.15 {
                                        let t = (other_half_w - v_e.dot(other_norm) * (-dir).dot(other_norm).signum()) / denom_e;
                                        if t > 0.0 && t < beam_len * 0.48 {
                                            d_end = d_end.min(beam_len - t);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if d_end <= d_start + 1.0 {
        return;
    }

    let p0 = b.start + dir * d_start;
    let p1 = b.end - dir * (beam_len - d_end);

    let p0_left = p0 - norm * half_w;
    let p1_left = p1 - norm * half_w;
    let p1_right = p1 + norm * half_w;
    let p0_right = p0 + norm * half_w;

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let s = |ch: u8| -> u8 { ((ch as f32) * intensity).min(255.0) as u8 };
                [s(base_rgb[0]), s(base_rgb[1]), s(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    let push_quad = |faces: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
                     pts_3d: [Vec3; 4],
                     depth: f64,
                     fill: [u8; 4],
                     hard: [bool; 4]| {
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        let z_min = pts_3d.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = pts_3d.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces.push((depth, pts_2d, fill, hard.to_vec(), z_min, z_max));
    };

    let _mid_2d = (p0 + p1) * 0.5;
    let _base_depth = transform_to_view(_mid_2d.extend(z_mid)).y;

    // 1. Left & Right Vertical Web Faces (-norm and +norm)
    for side in [-1.0_f64, 1.0_f64] {
        let side_normal = Vec3::new(norm.x * side, norm.y * side, 0.0);
        if side_normal.dot(cam_forward) >= -1e-4 && visual_style != VisualStyle::Wireframe {
            continue;
        }
        let (pa, pb) = if side < 0.0 {
            (p0_left, p1_left)
        } else {
            (p1_right, p0_right)
        };
        let web_pts = [pa.extend(z_bot), pb.extend(z_bot), pb.extend(z_top), pa.extend(z_top)];
        let side_depth = web_pts
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        let fill = shade_face(side_normal);
        // All 4 edges are hard, ensuring vertical seam joint lines at supporting column faces are 100% visible!
        push_quad(
            faces_with_depth,
            web_pts,
            side_depth,
            fill,
            [true, true, true, true],
        );
    }

    // 2. Unjoined Beam End-Caps
    if !start_joined {
        let start_norm = Vec3::new(-dir.x, -dir.y, 0.0);
        if start_norm.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
            let fill = shade_face(start_norm);
            let end_pts = [
                p0_right.extend(z_bot),
                p0_left.extend(z_bot),
                p0_left.extend(z_top),
                p0_right.extend(z_top),
            ];
            let depth = end_pts
                .iter()
                .map(|&p| transform_to_view(p).y)
                .fold(f64::INFINITY, f64::min)
                - 0.5;
            push_quad(
                faces_with_depth,
                end_pts,
                depth,
                fill,
                [true, true, true, true],
            );
        }
    }
    if !end_joined {
        let end_norm = Vec3::new(dir.x, dir.y, 0.0);
        if end_norm.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
            let fill = shade_face(end_norm);
            let end_pts = [
                p1_left.extend(z_bot),
                p1_right.extend(z_bot),
                p1_right.extend(z_top),
                p1_left.extend(z_top),
            ];
            let depth = end_pts
                .iter()
                .map(|&p| transform_to_view(p).y)
                .fold(f64::INFINITY, f64::min)
                - 0.5;
            push_quad(
                faces_with_depth,
                end_pts,
                depth,
                fill,
                [true, true, true, true],
            );
        }
    }

    // 3. Beam Top Flange at z_top (always rendered solid when camera looks down from above)
    if Vec3::Z.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
        let fill = shade_face(Vec3::Z);
        let top_pts = [
            p0_left.extend(z_top),
            p1_left.extend(z_top),
            p1_right.extend(z_top),
            p0_right.extend(z_top),
        ];
        // Evaluate top flange depth at its camera-closest corner (-1.0 bias) so it always draws cleanly on top of the beam web!
        let min_corner_y = top_pts
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        let depth = min_corner_y - 1.0;
        push_quad(
            faces_with_depth,
            top_pts,
            depth,
            fill,
            [true, true, true, true],
        );
    }

    // 4. Beam Bottom Soffit at z_bot (when camera looks up from below)
    if (-Vec3::Z).dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
        let fill = shade_face(-Vec3::Z);
        let bot_pts = [
            p0_right.extend(z_bot),
            p1_right.extend(z_bot),
            p1_left.extend(z_bot),
            p0_left.extend(z_bot),
        ];
        // Evaluate bottom soffit depth at its camera-closest corner (-1.0 bias) so it always draws cleanly on top of the beam web!
        let min_corner_y = bot_pts
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        let depth = min_corner_y - 1.0;
        push_quad(
            faces_with_depth,
            bot_pts,
            depth,
            fill,
            [true, true, true, true],
        );
    }
}

/// Renders a Curtain Wall / Storefront assembly with 3D solid mullions, transoms, and glazed glass panels.
fn render_curtain_wall_solid(
    cw: &plinth_model::CurtainWall,
    base_z: f64,
    top_z: Option<f64>,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let (z0, z1) = cw.z_bounds(base_z, top_z);
    let len = cw.length();
    let h = z1 - z0;
    if len < 10.0 || h < 10.0 {
        return;
    }

    let dir = cw.direction();
    let norm = cw.normal();
    let half_d = cw.mullion_depth * 0.5;
    let half_w = (cw.mullion_width * 0.5).min(len * 0.2);

    let mullion_rgb = [55u8, 60, 68];
    let glass_rgb = match visual_style {
        VisualStyle::Shaded => [145u8, 205, 235],
        VisualStyle::ConsistentColors => [165u8, 215, 240],
        VisualStyle::HiddenLine => [theme.background[0], theme.background[1], theme.background[2]],
        VisualStyle::Wireframe => [0, 0, 0],
    };
    let glass_alpha = match visual_style {
        VisualStyle::Shaded | VisualStyle::ConsistentColors => 210u8,
        VisualStyle::HiddenLine => 255u8,
        VisualStyle::Wireframe => 0u8,
    };

    let shade_face = |n_vec: Vec3, rgb: [u8; 3], alpha: u8| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.45f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let s = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [s(rgb[0]), s(rgb[1]), s(rgb[2]), alpha]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [rgb[0], rgb[1], rgb[2], alpha],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    let mut push_quad = |pts_3d: [Vec3; 4], n_vec: Vec3, rgb: [u8; 3], alpha: u8, hard: [bool; 4], bias: f64| {
        let pts_2d: Vec<Vec2> = pts_3d.iter().map(|&p| project_to_screen(p)).collect();
        let min_y = pts_3d.iter().map(|&p| transform_to_view(p).y).fold(f64::INFINITY, f64::min);
        let depth = min_y + bias;
        let fill = shade_face(n_vec, rgb, alpha);
        let z_min = pts_3d.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = pts_3d.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((depth, pts_2d, fill, hard.to_vec(), z_min, z_max));
    };

    let n_v = (len / cw.vertical_grid_spacing.max(200.0)).round().max(1.0) as usize;
    let bay_w = len / (n_v as f64);
    let n_h = (h / cw.horizontal_grid_spacing.max(200.0)).round().max(1.0) as usize;
    let tier_h = h / (n_h as f64);
    let half_t_glass = cw.panel_thickness * 0.5;

    // Macro to emit a 100% watertight 6-sided solid mullion / transom box
    macro_rules! emit_mullion_box {
        ($p_start:expr, $p_end:expr, $mz0:expr, $mz1:expr) => {{
            let p_start: Vec2 = $p_start;
            let p_end: Vec2 = $p_end;
            let mz0: f64 = $mz0;
            let mz1: f64 = $mz1;
            let p0_l = p_start - norm * half_d;
            let p1_l = p_end - norm * half_d;
            let p1_r = p_end + norm * half_d;
            let p0_r = p_start + norm * half_d;

            // 1. Front face (+norm)
            let n_front = Vec3::new(norm.x, norm.y, 0.0);
            if n_front.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p1_r.extend(mz0), p0_r.extend(mz0), p0_r.extend(mz1), p1_r.extend(mz1)], n_front, mullion_rgb, 255, [true, true, true, true], -0.2);
            }
            // 2. Back face (-norm)
            let n_back = Vec3::new(-norm.x, -norm.y, 0.0);
            if n_back.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p0_l.extend(mz0), p1_l.extend(mz0), p1_l.extend(mz1), p0_l.extend(mz1)], n_back, mullion_rgb, 255, [true, true, true, true], -0.2);
            }
            // 3. Top cap (+Z) - emitted for every mullion and horizontal transom when looking down from above!
            if Vec3::Z.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p0_l.extend(mz1), p1_l.extend(mz1), p1_r.extend(mz1), p0_r.extend(mz1)], Vec3::Z, mullion_rgb, 255, [true, true, true, true], -0.3);
            }
            // 4. Bottom cap (-Z) - emitted for every mullion and horizontal transom when looking up from below!
            if (-Vec3::Z).dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p0_r.extend(mz0), p1_r.extend(mz0), p1_l.extend(mz0), p0_l.extend(mz0)], -Vec3::Z, mullion_rgb, 255, [true, true, true, true], -0.3);
            }
            // 5. Left end face (-dir) - emitted when camera faces -dir
            let n_left = Vec3::new(-dir.x, -dir.y, 0.0);
            if n_left.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p0_l.extend(mz0), p0_r.extend(mz0), p0_r.extend(mz1), p0_l.extend(mz1)], n_left, mullion_rgb, 255, [true, true, true, true], -0.25);
            }
            // 6. Right end face (+dir) - emitted when camera faces +dir
            let n_right = Vec3::new(dir.x, dir.y, 0.0);
            if n_right.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([p1_r.extend(mz0), p1_l.extend(mz0), p1_l.extend(mz1), p1_r.extend(mz1)], n_right, mullion_rgb, 255, [true, true, true, true], -0.25);
            }
        }};
    }

    // 1. Vertical mullions
    for i in 0..=n_v {
        let cx = i as f64 * bay_w;
        let (d0, d1) = if i == 0 {
            (0.0, half_w * 2.0)
        } else if i == n_v {
            (len - half_w * 2.0, len)
        } else {
            (cx - half_w, cx + half_w)
        };
        let p0 = cw.start + dir * d0;
        let p1 = cw.start + dir * d1;
        emit_mullion_box!(p0, p1, z0, z1);
    }

    // 2. Horizontal transoms and glass panels in bays
    for i in 0..n_v {
        let bay_start = if i == 0 { half_w * 2.0 } else { i as f64 * bay_w + half_w };
        let bay_end = if i + 1 == n_v { len - half_w * 2.0 } else { (i + 1) as f64 * bay_w - half_w };
        if bay_end <= bay_start + 1.0 {
            continue;
        }

        let bp0 = cw.start + dir * bay_start;
        let bp1 = cw.start + dir * bay_end;

        // Bottom sill rail
        emit_mullion_box!(bp0, bp1, z0, (z0 + half_w * 2.0).min(z1));
        // Top head rail
        emit_mullion_box!(bp0, bp1, (z1 - half_w * 2.0).max(z0), z1);

        // Intermediate horizontal transoms
        for j in 1..n_h {
            let tz = z0 + j as f64 * tier_h;
            let tz0 = (tz - half_w).max(z0);
            let tz1 = (tz + half_w).min(z1);
            emit_mullion_box!(bp0, bp1, tz0, tz1);
        }

        // Glazed glass panels for each tier
        let gp0_l = bp0 - norm * half_t_glass;
        let gp1_l = bp1 - norm * half_t_glass;
        let gp1_r = bp1 + norm * half_t_glass;
        let gp0_r = bp0 + norm * half_t_glass;

        for j in 0..n_h {
            let gz0 = if j == 0 { z0 + half_w * 2.0 } else { z0 + j as f64 * tier_h + half_w };
            let gz1 = if j + 1 == n_h { z1 - half_w * 2.0 } else { z0 + (j + 1) as f64 * tier_h - half_w };
            if gz1 <= gz0 + 1.0 {
                continue;
            }

            // Front glass face (+norm)
            let n_front = Vec3::new(norm.x, norm.y, 0.0);
            if n_front.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([gp1_r.extend(gz0), gp0_r.extend(gz0), gp0_r.extend(gz1), gp1_r.extend(gz1)], n_front, glass_rgb, glass_alpha, [true, true, true, true], 0.1);
            }
            // Back glass face (-norm)
            let n_back = Vec3::new(-norm.x, -norm.y, 0.0);
            if n_back.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe {
                push_quad([gp0_l.extend(gz0), gp1_l.extend(gz0), gp1_l.extend(gz1), gp0_l.extend(gz1)], n_back, glass_rgb, glass_alpha, [true, true, true, true], 0.1);
            }
        }
    }
}

/// Renders pitched and flat roofs (Hip, Gable, Shed, Flat) as watertight 3D solids.
fn render_roof_solid(
    r: &plinth_model::Roof,
    base_z: f64,
    top_z: Option<f64>,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let faces = r.faces_with_bounds(base_z, top_z);
    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let s = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [s(base_rgb[0]), s(base_rgb[1]), s(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    for face in faces {
        let is_vis = face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !is_vis {
            continue;
        }
        let fill = shade_face(face.normal);
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
        let min_corner_y = face
            .pts
            .iter()
            .map(|&p| transform_to_view(p).y)
            .fold(f64::INFINITY, f64::min);
        let depth = if cam_forward.z < 0.0 && face.normal.z > 0.0 {
            // Viewing from above: sloped roof top faces must draw on top of walls and interior underneath
            min_corner_y - 2.0
        } else if cam_forward.z > 0.0 && face.normal.z < 0.0 {
            // Viewing from below: soffits are in front of attic/interior
            min_corner_y - 2.0
        } else {
            // Eave fascias and vertical faces
            min_corner_y - 1.0
        };
        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((depth, pts_2d, fill, face.hard, z_min, z_max));
    }
}

/// Renders Stairs (`Monolithic`, `SolidStepped`, `OpenRiser`) as watertight 3D solids
/// with exact Painter depth ordering between treads, risers, sloped soffit, and seam-free side cheeks.
fn render_stair_solid(
    s: &plinth_model::Stair,
    base_z: f64,
    top_z: Option<f64>,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let faces = s.faces(base_z, top_z);
    if faces.is_empty() {
        return;
    }

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let ch = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [ch(base_rgb[0]), ch(base_rgb[1]), ch(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    let dir = s.direction();
    let norm = s.normal();
    let n_dir = Vec3::new(dir.x, dir.y, 0.0);
    let n_norm = Vec3::new(norm.x, norm.y, 0.0);

    // Camera-closest Y of the entire stair volume (used to anchor outer supporting hull planes
    // so the near side cheek, soffit, toe base, and back face cleanly bound internal step geometry)
    let min_stair_y = faces
        .iter()
        .flat_map(|f| f.pts.iter())
        .map(|&p| transform_to_view(p).y)
        .fold(f64::INFINITY, f64::min);

    let is_open_riser = s.stair_style == plinth_model::StairStyle::OpenRiser;

    for face in faces {
        let is_vis = face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !is_vis {
            continue;
        }
        let fill = shade_face(face.normal);
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
        let centroid_y = transform_to_view(face.centroid()).y;

        let depth = if is_open_riser || visual_style == VisualStyle::Wireframe {
            centroid_y
        } else {
            let dot_side = face.normal.dot(n_norm).abs();
            let dot_back = face.normal.dot(n_dir);
            let is_side_cheek = dot_side > 0.9;
            let is_soffit_or_toe = face.normal.z < -0.15;
            let is_back_face = dot_back > 0.9;

            if is_side_cheek {
                // Visible near side cheek lies on a global supporting plane W = ±half_w in front of all steps
                min_stair_y - 1.0
            } else if is_soffit_or_toe || is_back_face {
                // Visible soffit, toe seat, and top back face lie on supporting hull planes of the stair
                min_stair_y - 0.5
            } else {
                // Treads and Risers sort naturally back-to-front by exact 3D centroid Y
                centroid_y
            }
        };

        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((depth, pts_2d, fill, face.hard, z_min, z_max));
    }
}

/// Renders Structural Foundation Footings (`Pad`, `Stepped`, `Sloped`) as opaque convex 3D solids.
fn render_footing_solid(
    f: &plinth_model::Footing,
    base_z: f64,
    top_z: Option<f64>,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let faces = f.faces(base_z, top_z);
    if faces.is_empty() {
        return;
    }

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let ch = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [ch(base_rgb[0]), ch(base_rgb[1]), ch(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    for face in faces {
        let is_vis = face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !is_vis {
            continue;
        }
        let fill = shade_face(face.normal);
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
        // Slight back-bias (+0.5) on footing top cap so a column resting on top of the footing draws cleanly without z-fighting
        let centroid_y = transform_to_view(face.centroid()).y;
        let depth = if face.normal.z > 0.8 {
            centroid_y + 0.5
        } else {
            centroid_y
        };
        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((depth, pts_2d, fill, face.hard, z_min, z_max));
    }
}

/// Renders Ramps (`SlopedSlab`, `SolidWedge`) as smooth, watertight 3D inclined convex solids.
fn render_ramp_solid(
    r: &plinth_model::Ramp,
    base_z: f64,
    top_z: Option<f64>,
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    let faces = r.faces(base_z, top_z);
    if faces.is_empty() {
        return;
    }

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let ch = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [ch(base_rgb[0]), ch(base_rgb[1]), ch(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    for face in faces {
        let is_vis = face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !is_vis {
            continue;
        }
        let fill = shade_face(face.normal);
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
        let depth = transform_to_view(face.centroid()).y;
        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((depth, pts_2d, fill, face.hard, z_min, z_max));
    }
}

/// Renders a list of 3D solid faces (such as Toposolids or Building Pads).
fn render_face_list_solid(
    faces: &[plinth_geom::Face],
    project_to_screen: &impl Fn(Vec3) -> Vec2,
    transform_to_view: &impl Fn(Vec3) -> Vec3,
    cam_forward: Vec3,
    light_dir: Vec3,
    base_rgb: [u8; 3],
    visual_style: VisualStyle,
    theme: &ColorTheme,
    faces_with_depth: &mut Vec<(f64, Vec<Vec2>, [u8; 4], Vec<bool>, f64, f64)>,
) {
    if faces.is_empty() {
        return;
    }

    let shade_face = |n_vec: Vec3| -> [u8; 4] {
        match visual_style {
            VisualStyle::Shaded => {
                let diffuse = n_vec.dot(light_dir).max(0.0) as f32;
                let ambient = 0.40f32;
                let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.25, 1.0);
                let ch = |c: u8| -> u8 { ((c as f32) * intensity).min(255.0) as u8 };
                [ch(base_rgb[0]), ch(base_rgb[1]), ch(base_rgb[2]), 255]
            }
            VisualStyle::HiddenLine => theme.background,
            VisualStyle::ConsistentColors => [base_rgb[0], base_rgb[1], base_rgb[2], 255],
            VisualStyle::Wireframe => [0, 0, 0, 0],
        }
    };

    for face in faces {
        let is_vis = face.normal.dot(cam_forward) < -1e-4 || visual_style == VisualStyle::Wireframe;
        if !is_vis {
            continue;
        }
        let fill = shade_face(face.normal);
        let pts_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_to_screen(p)).collect();
        let centroid_y = transform_to_view(face.centroid()).y;
        let z_min = face.pts.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
        let z_max = face.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        faces_with_depth.push((centroid_y, pts_2d, fill, face.hard.clone(), z_min, z_max));
    }
}
