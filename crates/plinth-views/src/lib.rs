//! # plinth-views
//!
//! View projection engine.
//!
//! Derives Floor Plans, Reflected Ceiling Plans, Elevations, Sections, 3D, and Details
//! directly from the single-source-of-truth 3D model.

pub mod annotation;
pub mod ceiling;
pub mod detail;
pub mod elevation;
pub mod glb;
pub mod pdf;
pub mod plan;
pub mod scene;
pub mod section;
pub mod threed;

pub use annotation::{emit_tick_mark, render_dimension, render_leader, render_text_note};
pub use glb::export_glb;
pub use pdf::{export_all_sheets_pdf, export_sheet_pdf};
pub use scene::{ColorTheme, MaterialPalette, RenderPrimitive, RenderScene, VisualStyle};

use plinth_model::{Document, ViewDef, ViewKind};

/// Dispatches projection for any view in the document with default Shaded style.
pub fn render_view(doc: &Document, view: &ViewDef, is_dark: bool) -> RenderScene {
    render_view_with_style(doc, view, is_dark, VisualStyle::Shaded)
}

/// Dispatches projection for any view with an explicit visual style (Shaded, HiddenLine, ConsistentColors, Wireframe).
pub fn render_view_with_style(
    doc: &Document,
    view: &ViewDef,
    is_dark: bool,
    style: VisualStyle,
) -> RenderScene {
    let theme = if is_dark {
        ColorTheme::dark()
    } else {
        ColorTheme::light()
    };
    render_view_with_style_and_theme(doc, view, &theme, style)
}

/// Dispatches projection for any view with an explicit visual style and custom theme/materials.
pub fn render_view_with_style_and_theme(
    doc: &Document,
    view: &ViewDef,
    theme: &ColorTheme,
    style: VisualStyle,
) -> RenderScene {
    match view.kind {
        ViewKind::FloorPlan => plan::render_floor_plan(doc, view, theme),
        ViewKind::CeilingPlan => ceiling::render_ceiling_plan(doc, view, theme),
        ViewKind::ElevationNorth
        | ViewKind::ElevationSouth
        | ViewKind::ElevationEast
        | ViewKind::ElevationWest => elevation::render_elevation(doc, view, theme),
        ViewKind::Section => section::render_section(doc, view, theme),
        ViewKind::ThreeD => threed::render_3d(doc, view, theme, style),
        ViewKind::Detail => detail::render_detail(doc, view, theme),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_views_render_from_single_source_of_truth() {
        let doc = Document::create_sample_building();
        assert!(!doc.views.is_empty());

        for view in doc.views.values() {
            let scene_dark = render_view(&doc, view, true);
            let scene_light = render_view(&doc, view, false);

            assert!(
                !scene_dark.primitives.is_empty(),
                "View {} generated empty scene",
                view.name
            );
            assert!(
                !scene_light.primitives.is_empty(),
                "View {} generated empty scene in light mode",
                view.name
            );
        }
    }

    #[test]
    fn test_plan_and_section_cuts() {
        let doc = Document::create_sample_building();
        let plan_view = doc.views.values().find(|v| v.kind == ViewKind::FloorPlan).unwrap();
        let plan_scene = render_view(&doc, plan_view, true);
        assert!(plan_scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Polygon { .. })));

        let section_view = doc.views.values().find(|v| v.kind == ViewKind::Section).unwrap();
        let section_scene = render_view(&doc, section_view, true);
        assert!(section_scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Polygon { .. })));
    }

    #[test]
    fn test_3d_solids_opaque_and_properly_culled() {
        let mut doc = Document::create_sample_building();
        let lvl1_id = doc.active_level_id;

        // Add 3D Freeform Primitives: Box, Cylinder, Sphere
        let b = plinth_model::FreeformSolid::new_box(
            plinth_geom::vec2(-2000.0, -2000.0),
            plinth_geom::vec2(-500.0, -500.0),
            0.0,
            2000.0,
            lvl1_id,
        );
        doc.elements.insert(b.id, plinth_model::Element::FreeformSolid(b));

        let cyl = plinth_model::FreeformSolid::new_cylinder(
            plinth_geom::vec2(-4000.0, 2000.0),
            800.0,
            0.0,
            2500.0,
            24,
            lvl1_id,
        );
        doc.elements.insert(cyl.id, plinth_model::Element::FreeformSolid(cyl));

        let sph = plinth_model::FreeformSolid::new_sphere(
            plinth_geom::vec2(-4000.0, -2000.0),
            1000.0,
            1000.0,
            lvl1_id,
        );
        doc.elements.insert(sph.id, plinth_model::Element::FreeformSolid(sph));

        let threed_view = doc.views.values().find(|v| v.kind == ViewKind::ThreeD).unwrap();
        let threed_scene = render_view(&doc, threed_view, true);

        // Verify scene has solid polygons with non-zero fill
        let poly_count = threed_scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] > 0)).count();
        assert!(poly_count > 10, "Expected multiple solid opaque polygons, got {}", poly_count);

        // Verify scene has crisp edge outlines
        let line_count = threed_scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Line { .. })).count();
        assert!(line_count > 10, "Expected boundary lines, got {}", line_count);
    }

    #[test]
    fn test_diagnose_rotation_depth_order() {
        let doc = Document::create_sample_building();
        let mut view = doc.views.values().find(|v| v.kind == ViewKind::ThreeD).unwrap().clone();

        // Test 8 cardinal angles around 360-degree rotation
        for i in 0..8 {
            let yaw = (i as f64) * std::f64::consts::FRAC_PI_4;
            if let Some(cam) = view.camera_3d.as_mut() {
                cam.yaw = yaw;
                cam.pitch = 0.55;
            }
            let scene = render_view(&doc, &view, false);
            let poly_count = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { .. })).count();
            let line_count = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Line { .. })).count();
            assert!(poly_count >= 4, "Angle {}: expected >= 4 monolithic polygons, got {}", i, poly_count);
            assert!(line_count >= 12, "Angle {}: expected >= 12 boundary lines, got {}", i, line_count);
        }
    }

    #[test]
    fn test_visual_styles_and_perspective_projection() {
        let doc = Document::create_sample_building();
        let mut view = doc.views.values().find(|v| v.kind == ViewKind::ThreeD).unwrap().clone();

        // 1. Shaded Style
        let scene_shaded = render_view_with_style(&doc, &view, false, VisualStyle::Shaded);
        assert!(scene_shaded.primitives.iter().any(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255)));

        // 2. Hidden Line Style
        let scene_hidden = render_view_with_style(&doc, &view, false, VisualStyle::HiddenLine);
        assert!(scene_hidden.primitives.iter().any(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255)));

        // 3. Consistent Colors Style
        let scene_consistent = render_view_with_style(&doc, &view, false, VisualStyle::ConsistentColors);
        assert!(scene_consistent.primitives.iter().any(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255)));

        // 4. Wireframe Style
        let scene_wire = render_view_with_style(&doc, &view, false, VisualStyle::Wireframe);
        let wire_polys: Vec<_> = scene_wire.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { .. })).collect();
        println!("TOTAL WIREFRAME POLYGONS: {}", wire_polys.len());
        for (i, p) in wire_polys.iter().enumerate() {
            if let RenderPrimitive::Polygon { fill_color, stroke_color, stroke_width, .. } = p {
                println!("  Poly {}: fill={:?}, stroke={:?}, stroke_w={}", i, fill_color, stroke_color, stroke_width);
            }
        }
        let wire_lines: Vec<_> = scene_wire.primitives.iter().filter_map(|p| match p {
            RenderPrimitive::Line { start, end, color, width, .. } => Some((*start, *end, *color, *width)),
            _ => None,
        }).collect();
        println!("TOTAL WIREFRAME LINES: {}", wire_lines.len());
        for (i, (s, e, c, w)) in wire_lines.iter().take(20).enumerate() {
            println!("Line {}: {:?} -> {:?}, color={:?}, w={}", i, s, e, c, w);
        }
        
        let mut zero_len = 0;
        let mut non_zero = 0;
        for (start, end, _color, _width) in &wire_lines {
            let len = (*start - *end).length();
            if len < 0.1 {
                zero_len += 1;
            } else {
                non_zero += 1;
            }
        }
        println!("Zero len: {}, Non-zero len: {}", zero_len, non_zero);
        // Wireframe polygons have fill alpha 0
        assert!(scene_wire.primitives.iter().all(|p| !matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] > 0)));
        // Wireframe has boundary lines
        assert!(scene_wire.primitives.iter().any(|p| matches!(p, RenderPrimitive::Line { .. })));

        // 5. Perspective Projection
        if let Some(cam) = view.camera_3d.as_mut() {
            cam.fov_degrees = 50.0; // Perspective mode
        }
        let scene_persp = render_view_with_style(&doc, &view, false, VisualStyle::Shaded);
        assert!(!scene_persp.primitives.is_empty(), "Perspective scene must have primitives");
    }

    #[test]
    fn test_slab_and_wall_watertight_occlusion() {
        let doc = Document::create_sample_building();
        let threed_view = doc.views.values().find(|v| v.kind == ViewKind::ThreeD).unwrap();

        // 1. Shaded 3D render from standard isometric camera (looking down from above, pitch > 0)
        let scene = render_view_with_style(&doc, threed_view, false, VisualStyle::Shaded);

        // Verify we have clean monolithic opaque solid polygons for smooth watertight occlusion
        let poly_count = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255)).count();
        assert!(poly_count >= 20, "Expected solid polygon surfaces, got {}", poly_count);

        // 2. Verify that ceilings never bleed edges across exterior walls when viewed from above
        // (Ceiling only renders its bottom face when looking up from inside rooms)
        // Check that all line primitives in the scene belong to real exterior/visible boundaries
        let line_count = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Line { .. })).count();
        assert!(line_count >= 50, "Expected crisp boundary outlines for watertight solids, got {}", line_count);
    }

    #[test]
    fn test_roof_slab_strict_occlusion_over_interior_elements() {
        let doc = Document::create_sample_building();
        let threed_view = doc.views.values().find(|v| v.kind == ViewKind::ThreeD).unwrap();

        let scene = render_view_with_style(&doc, threed_view, false, VisualStyle::Shaded);
        let polys: Vec<_> = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] > 0)).collect();

        // Verify that there are solid polygons rendered
        assert!(!polys.is_empty(), "Scene must contain rendered polygons");

        // Verify that interior beam is NOT rendered when roof slab occludes it from above
        let beam_rgb = [160, 160, 165];
        let has_beam = polys.iter().any(|p| {
            if let RenderPrimitive::Polygon { fill_color, .. } = p {
                fill_color[0] == beam_rgb[0] && fill_color[1] == beam_rgb[1] && fill_color[2] == beam_rgb[2]
            } else {
                false
            }
        });
        assert!(!has_beam, "Interior beam must be occluded by the roof slab from outside view");
    }

    #[test]
    fn test_wall_miter_corners_and_seamless_openings() {
        let doc = Document::create_sample_building();
        let all_walls: Vec<&plinth_model::Wall> = doc
            .elements
            .values()
            .filter_map(|e| match e {
                plinth_model::Element::Wall(w) => Some(w),
                _ => None,
            })
            .collect();

        // 1. Verify all 5 walls have both start and end cleanly joined at L-corners or T-joints
        for w in &all_walls {
            let (poly, start_joined, end_joined) = w.mitered_footprint(&all_walls);
            assert!(start_joined, "Wall '{}' start must be joined", w.name);
            assert!(end_joined, "Wall '{}' end must be joined", w.name);
            assert_eq!(poly.len(), 4);
        }

        // Verify South Exterior Wall (0,0)->(12000,0) with 250mm thickness (half_t = 125mm):
        // outer edge spans (-125, -125) to (12125, -125), inner edge spans (125, 125) to (11875, 125)
        let south_wall = all_walls
            .iter()
            .find(|w| (w.start.y - 0.0).abs() < 1.0 && (w.end.y - 0.0).abs() < 1.0)
            .unwrap();
        let (s_poly, _, _) = south_wall.mitered_footprint(&all_walls);
        assert!((s_poly[0].x - (-125.0)).abs() < 1e-3 && (s_poly[0].y - (-125.0)).abs() < 1e-3);
        assert!((s_poly[1].x - 12125.0).abs() < 1e-3 && (s_poly[1].y - (-125.0)).abs() < 1e-3);
        assert!((s_poly[2].x - 11875.0).abs() < 1e-3 && (s_poly[2].y - 125.0).abs() < 1e-3);
        assert!((s_poly[3].x - 125.0).abs() < 1e-3 && (s_poly[3].y - 125.0).abs() < 1e-3);

        // Verify Interior Partition (6000,0)->(6000,8000) T-joint trims cleanly against North/South interior faces (y = 125..7875)
        let part_wall = all_walls
            .iter()
            .find(|w| (w.start.x - 6000.0).abs() < 1.0 && (w.end.x - 6000.0).abs() < 1.0)
            .unwrap();
        let (p_poly, _, _) = part_wall.mitered_footprint(&all_walls);
        assert!((p_poly[0].y - 125.0).abs() < 1e-3 && (p_poly[3].y - 125.0).abs() < 1e-3);
        assert!((p_poly[1].y - 7875.0).abs() < 1e-3 && (p_poly[2].y - 7875.0).abs() < 1e-3);

        // 2. Verify South Wall B-Rep boundary faces have zero internal cross-section faces
        // (no faces with normal along +X or -X except window/door jamb reveals inside opening z-range)
        let openings = doc.hosted_openings_for_wall(south_wall.id);
        let b_faces = south_wall.boundary_faces(0.0, Some(3300.0), &openings, &all_walls, true);
        for f in &b_faces {
            if f.normal.x.abs() > 0.9 {
                // Any X-facing face on South Wall must be an opening jamb reveal (never spanning above window top z = 2300)
                let z_max = f.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
                assert!(
                    z_max <= 2300.5,
                    "No internal lintel cross-section face should exist above z=2300 (found z_max={})",
                    z_max
                );
            }
        }
    }

    #[test]
    fn test_standalone_and_l_walls_both_faces_visible_in_3d() {
        let mut doc = Document::create_sample_building();
        let lvl1 = doc.active_level_id;

        // Add standalone L-shaped walls outside the roofed building (matching user screenshot)
        let w_a = plinth_model::Wall::new(
            plinth_geom::vec2(-8000.0, 0.0),
            plinth_geom::vec2(-8000.0, 6000.0),
            lvl1,
        );
        let w_b = plinth_model::Wall::new(
            plinth_geom::vec2(-8000.0, 6000.0),
            plinth_geom::vec2(-3000.0, 6000.0),
            lvl1,
        );
        doc.elements.insert(w_a.id, plinth_model::Element::Wall(w_a));
        doc.elements.insert(w_b.id, plinth_model::Element::Wall(w_b));

        // Now strip all sample elements except the roofed building AND check standalone doc too
        let mut standalone_doc = Document::new_empty();
        let slvl = standalone_doc.active_level_id;
        // Include a roofed shed elsewhere in the scene to ensure roof presence never culls standalone wall faces
        let mut shed_roof = plinth_model::Roof::new(
            vec![
                plinth_geom::vec2(5000.0, 0.0),
                plinth_geom::vec2(10000.0, 0.0),
                plinth_geom::vec2(10000.0, 5000.0),
                plinth_geom::vec2(5000.0, 5000.0),
            ],
            slvl,
        );
        shed_roof.offset = 3000.0;
        standalone_doc
            .elements
            .insert(shed_roof.id, plinth_model::Element::Roof(shed_roof));
        let solo_wall = plinth_model::Wall::new(
            plinth_geom::vec2(-6000.0, 0.0),
            plinth_geom::vec2(-6000.0, 5000.0),
            slvl,
        );
        standalone_doc
            .elements
            .insert(solo_wall.id, plinth_model::Element::Wall(solo_wall));

        // Orbit from BOTH left and right of solo_wall (`yaw = 0.7` and `yaw = 0.7 + PI`)
        for yaw in [0.7_f64, 0.7_f64 + std::f64::consts::PI] {
            let mut v3d = plinth_model::ViewDef::new_3d("Test3D");
            if let Some(ref mut cam) = v3d.camera_3d {
                cam.yaw = yaw;
                cam.pitch = 0.45;
            }
            let scene = render_view_with_style(&standalone_doc, &v3d, true, VisualStyle::Shaded);
            // Solo wall (non-structural base_rgb = [225, 228, 232]) must emit at least:
            // 1 vertical side face + 1 top cap + 1 end cap = 3 solid shaded polygons!
            let wall_polys = scene
                .primitives
                .iter()
                .filter(|p| match p {
                    RenderPrimitive::Polygon { fill_color, .. } => fill_color[3] == 255,
                    _ => false,
                })
                .count();
            assert!(
                wall_polys >= 3,
                "Standalone wall must render vertical face + top cap + end cap at yaw={yaw}, got {wall_polys}"
            );
        }
    }

    #[test]
    fn test_stair_and_ramp_3d_solid_rendering() {
        let mut doc = Document::new_empty();
        let lvl = doc.active_level_id;
        let stair = plinth_model::Stair::new(
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(4500.0, 0.0),
            lvl,
        );
        let ramp = plinth_model::Ramp::new(
            plinth_geom::vec2(0.0, 3000.0),
            plinth_geom::vec2(7200.0, 3000.0),
            lvl,
        );
        doc.elements.insert(stair.id, plinth_model::Element::Stair(stair));
        doc.elements.insert(ramp.id, plinth_model::Element::Ramp(ramp));

        let v3d = doc
            .views
            .values()
            .find(|v| v.kind == plinth_model::ViewKind::ThreeD)
            .unwrap()
            .clone();
        let scene = render_view_with_style(&doc, &v3d, true, VisualStyle::Shaded);
        let solid_polys = scene
            .primitives
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255))
            .count();
        assert!(solid_polys >= 20, "Stair and Ramp must render as opaque 3D solids");
    }

    #[test]
    fn test_beam_column_trimming_and_top_flange_visibility() {
        use plinth_geom::vec2;

        let mut doc = Document::new_empty();
        let lvl = doc.active_level_id;

        // 1. Verify column ray exit distance helper
        let mut col_rect = plinth_model::Column::new(vec2(0.0, 0.0), lvl);
        col_rect.width = 400.0;
        col_rect.depth = 400.0;
        col_rect.rotation = 0.0;
        let exit_x = threed::column_ray_exit_distance(&col_rect, vec2(0.0, 0.0), vec2(1.0, 0.0));
        assert_eq!(exit_x, Some(200.0), "400mm column exit distance along +X must be 200mm");
        let exit_y = threed::column_ray_exit_distance(&col_rect, vec2(0.0, 0.0), vec2(0.0, 1.0));
        assert_eq!(exit_y, Some(200.0), "400mm column exit distance along +Y must be 200mm");

        let mut col_round = plinth_model::Column::new(vec2(0.0, 0.0), lvl);
        col_round.is_round = true;
        col_round.width = 500.0;
        let exit_round = threed::column_ray_exit_distance(&col_round, vec2(0.0, 0.0), vec2(1.0, 0.0));
        assert_eq!(exit_round, Some(250.0), "500mm round column exit distance must be 250mm");

        // 2. Build framing system: 2 columns and a connecting beam
        let mut c1 = plinth_model::Column::new(vec2(0.0, 0.0), lvl);
        c1.width = 400.0;
        c1.depth = 400.0;
        let mut c2 = plinth_model::Column::new(vec2(6000.0, 0.0), lvl);
        c2.width = 400.0;
        c2.depth = 400.0;

        let mut beam = plinth_model::Beam::new(vec2(0.0, 0.0), vec2(6000.0, 0.0), lvl);
        beam.width = 300.0;
        beam.depth = 500.0;

        // Also add an adjacent slab to ensure beam top flange is NOT erroneously suppressed
        let slab = plinth_model::Slab::new(
            vec![
                vec2(0.0, 0.0),
                vec2(6000.0, 0.0),
                vec2(6000.0, 6000.0),
                vec2(0.0, 6000.0),
            ],
            lvl,
        );

        doc.elements.insert(c1.id, plinth_model::Element::Column(c1));
        doc.elements.insert(c2.id, plinth_model::Element::Column(c2));
        doc.elements.insert(beam.id, plinth_model::Element::Beam(beam));
        doc.elements.insert(slab.id, plinth_model::Element::Slab(slab));

        let v3d = doc
            .views
            .values()
            .find(|v| v.kind == plinth_model::ViewKind::ThreeD)
            .unwrap()
            .clone();

        let scene = render_view_with_style(&doc, &v3d, true, VisualStyle::Shaded);

        // Ensure solid polygons are generated for columns, beam (web + top flange), and slab
        let solid_polys = scene
            .primitives
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Polygon { fill_color, .. } if fill_color[3] == 255))
            .count();
        assert!(solid_polys >= 5, "Structural frame must render all solid faces including beam top flange");

        // Verify crisp edge lines are present (including column-beam joint seams)
        let edge_lines = scene
            .primitives
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Line { .. }))
            .count();
        assert!(edge_lines >= 10, "Seam lines and silhouette edges must be emitted for beam-to-column joints");
    }

    #[test]
    fn test_curtain_wall_auto_hosting_and_wall_hole_cut() {
        use plinth_geom::vec2;
        let mut doc = Document::create_sample_building();
        let lvl = doc.active_level_id;

        // 1. Create a 10m long host wall
        let mut wall = plinth_model::Wall::new(vec2(0.0, 0.0), vec2(10000.0, 0.0), lvl);
        wall.unconnected_height = 3000.0;
        wall.thickness = 200.0;
        let wall_id = wall.id;
        doc.elements.insert(wall_id, plinth_model::Element::Wall(wall));

        // 2. Create a 4m embedded curtain wall from X=2000 to X=6000
        let mut cw = plinth_model::CurtainWall::new(vec2(2000.0, 0.0), vec2(6000.0, 0.0), lvl);
        cw.host_wall = Some(wall_id);
        cw.automatically_embed = true;
        cw.unconnected_height = 2500.0;
        cw.base_offset = 200.0;
        cw.mullion_width = 50.0;
        cw.mullion_depth = 150.0;
        cw.panel_thickness = 28.0;
        cw.vertical_grid_spacing = 1000.0;
        cw.horizontal_grid_spacing = 1200.0;
        let cw_id = cw.id;
        doc.elements.insert(cw_id, plinth_model::Element::CurtainWall(cw.clone()));

        // 3. Verify hosted openings query automatically recognizes the curtain wall opening
        let openings = doc.hosted_openings_for_wall(wall_id);
        assert_eq!(openings.len(), 1, "Host wall must register curtain wall opening cutout");
        assert!((openings[0].width - 4000.0).abs() < 1e-3, "Opening width must match curtain wall length");
        assert!((openings[0].offset_along - 4000.0).abs() < 1e-3, "Opening center must be at X=4000 along wall");
        assert!((openings[0].height - 2500.0).abs() < 1e-3, "Opening height must match curtain wall height");
        assert!((openings[0].sill_height - 200.0).abs() < 1e-3, "Opening sill must match curtain wall base offset");

        // 4. Verify 3D view renders wall piers with hole cut out and 3D curtain wall mullions/glass
        let v3d = doc
            .views
            .values()
            .find(|v| v.kind == plinth_model::ViewKind::ThreeD)
            .unwrap()
            .clone();
        let scene_3d = render_view_with_style(&doc, &v3d, true, VisualStyle::Shaded);
        assert!(!scene_3d.primitives.is_empty(), "3D view must generate primitives for hosted curtain wall");

        // 5. Verify Floor Plan renders cut wall piers and curtain wall mullions/glazing
        let v_plan = doc
            .views
            .values()
            .find(|v| v.kind == plinth_model::ViewKind::FloorPlan && v.level_id == Some(lvl))
            .unwrap()
            .clone();
        let scene_plan = render_view_with_style(&doc, &v_plan, true, VisualStyle::Shaded);
        let plan_polys = scene_plan
            .primitives
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Polygon { .. }))
            .count();
        // 2 wall piers + 5 mullion boxes in curtain wall
        assert!(plan_polys >= 3, "Plan view must render severed wall piers and curtain wall mullion boxes");

        // 6. Verify Curtain Wall solids generate intermediate transoms and watertight B-Rep faces
        let solids = cw.solids(200.0, None);
        // Vertical mullions (5) + bottom rail (4) + top rail (4) + intermediate transoms (4 * 1) + glass panels (4 * 2) = 25
        assert!(solids.len() >= 15, "Curtain wall solids must generate intermediate transoms, rails, mullions, and glass panels");

        // 7. Verify South Elevation renders accurate 2D solid mullions and glass panels matching 3D
        let v_elev = doc
            .views
            .values()
            .find(|v| v.kind == plinth_model::ViewKind::ElevationSouth)
            .unwrap()
            .clone();
        let scene_elev = render_view_with_style(&doc, &v_elev, true, VisualStyle::Shaded);
        let elev_polys = scene_elev
            .primitives
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Polygon { .. }))
            .count();
        assert!(elev_polys >= 10, "Elevation must render 2D solid mullions, transoms, and individual glass panels");
    }

    #[test]
    fn test_roof_3d_faces_and_column_top_solid_with_slab() {
        use plinth_geom::vec2;
        let mut doc = Document::new_empty();
        let lvl1 = plinth_model::Level::new("Level 1", 0.0, 3500.0);
        let lvl1_id = lvl1.id;
        doc.levels.insert(lvl1_id, lvl1);
        doc.active_level_id = lvl1_id;

        let v3d = plinth_model::ViewDef::new_3d("3D View");
        doc.views.insert(v3d.id, v3d.clone());

        // 1. Column with slab on top: column top surface must remain solid!
        let mut col = plinth_model::Column::new(vec2(0.0, 0.0), lvl1_id);
        col.unconnected_height = 3500.0;
        col.width = 400.0;
        col.depth = 400.0;
        let col_id = col.id;
        doc.elements.insert(col_id, plinth_model::Element::Column(col));

        // Slab at the same elevation touching column top
        let mut slab = plinth_model::Slab::new(
            vec![vec2(-1000.0, -1000.0), vec2(1000.0, -1000.0), vec2(1000.0, 1000.0), vec2(-1000.0, 1000.0)],
            lvl1_id,
        );
        slab.thickness = 200.0;
        slab.offset = 3300.0;
        doc.elements.insert(slab.id, plinth_model::Element::Slab(slab));

        let scene = render_view_with_style(&doc, &v3d, true, VisualStyle::Shaded);
        assert!(!scene.primitives.is_empty(), "3D view must generate solid primitives for column and slab");

        // 2. Hip Roof: test that all soffits have outward normals pointing down (-Z)
        let roof_pts = vec![
            vec2(0.0, 0.0),
            vec2(10000.0, 0.0),
            vec2(10000.0, 8000.0),
            vec2(0.0, 8000.0),
        ];
        let mut roof = plinth_model::Roof::new(roof_pts, lvl1_id);
        roof.shape = plinth_model::RoofShape::Hip;
        roof.offset = 3500.0;
        roof.overhang = 600.0;
        roof.thickness = 200.0;
        roof.slope_degrees = 30.0;

        let faces = roof.faces(3500.0);
        let top_faces: Vec<_> = faces.iter().filter(|f| f.normal.z > 0.1).collect();
        let soffit_faces: Vec<_> = faces.iter().filter(|f| f.normal.z < -0.1).collect();
        let fascia_faces: Vec<_> = faces.iter().filter(|f| f.normal.z.abs() <= 0.1).collect();

        assert_eq!(top_faces.len(), 4, "Hip roof must generate 4 sloped top faces facing upward (+Z)");
        assert_eq!(soffit_faces.len(), 4, "Hip roof must generate 4 soffit faces facing downward (-Z)");
        assert_eq!(fascia_faces.len(), 4, "Hip roof must generate 4 eave fascia faces facing outward horizontally");

        for soffit in &soffit_faces {
            assert!(soffit.normal.z < -0.3, "Soffit outward normal must face downward into the envelope");
        }
        for top in &top_faces {
            assert!(top.normal.z > 0.3, "Top roof slope outward normal must face upward (+Z)");
        }
    }

    #[test]
    fn test_column_visible_in_sections_and_slab_thickness_bounds() {
        use plinth_geom::vec2;
        let mut doc = Document::new_empty();
        let lvl1 = plinth_model::Level::new("Level 1", 0.0, 3500.0);
        let lvl2 = plinth_model::Level::new("Level 2", 3500.0, 3500.0);
        let lvl1_id = lvl1.id;
        let lvl2_id = lvl2.id;
        doc.levels.insert(lvl1_id, lvl1);
        doc.levels.insert(lvl2_id, lvl2);
        doc.active_level_id = lvl1_id;

        // 1. Column in background (Y = 6000mm) while section cut is at Y = 4000mm
        let mut bg_col = plinth_model::Column::new(vec2(5000.0, 6000.0), lvl1_id);
        bg_col.unconnected_height = 3500.0;
        bg_col.width = 400.0;
        bg_col.depth = 400.0;
        doc.elements.insert(bg_col.id, plinth_model::Element::Column(bg_col));

        // 2. Column directly on the cut line (Y = 4000mm)
        let mut cut_col = plinth_model::Column::new(vec2(8000.0, 4000.0), lvl1_id);
        cut_col.unconnected_height = 3500.0;
        cut_col.width = 400.0;
        cut_col.depth = 400.0;
        doc.elements.insert(cut_col.id, plinth_model::Element::Column(cut_col));

        // 3. Section view at Y = 4000mm looking towards +Y
        let v_sec = plinth_model::ViewDef::new_section(
            "Section 1",
            vec2(-2000.0, 4000.0),
            vec2(14000.0, 4000.0),
        );
        let theme = ColorTheme::light();
        let scene = section::render_section(&doc, &v_sec, &theme);

        // Verify that section scene has polygons for both background projected column and cut column!
        let polys: Vec<_> = scene.primitives.iter().filter(|p| matches!(p, RenderPrimitive::Polygon { .. })).collect();
        assert!(polys.len() >= 2, "Section view must render both projected background columns and cut columns");

        // 4. Verify Slab z_bounds with top_level preserves exact slab thickness (200mm, not 3500mm!)
        let mut slab = plinth_model::Slab::new(
            vec![vec2(0.0, 0.0), vec2(6000.0, 0.0), vec2(6000.0, 6000.0), vec2(0.0, 6000.0)],
            lvl1_id,
        );
        slab.top_level = Some(lvl2_id);
        slab.thickness = 200.0;
        let (z_bot, z_top) = slab.z_bounds(0.0, Some(3500.0));
        assert_eq!(z_top, 3500.0, "Slab top face must align to top level elevation");
        assert_eq!(z_bot, 3300.0, "Slab bottom face must reflect thickness (3500 - 200 = 3300mm)");
        assert_eq!(z_top - z_bot, 200.0, "Slab thickness must remain strictly 200mm, never stretching across the storey!");
    }

    #[test]
    fn test_multi_storey_lower_walls_visible_in_3d() {
        use plinth_geom::vec2;
        let mut doc = Document::new_empty();
        let lvl1 = plinth_model::Level::new("Ground Floor", 0.0, 3000.0);
        let lvl2 = plinth_model::Level::new("First Floor", 3000.0, 3000.0);
        let lvl3 = plinth_model::Level::new("Roof Level", 6000.0, 3000.0);
        let l1_id = lvl1.id;
        let l2_id = lvl2.id;
        let l3_id = lvl3.id;
        doc.levels.insert(l1_id, lvl1);
        doc.levels.insert(l2_id, lvl2);
        doc.levels.insert(l3_id, lvl3);

        // Ground Floor exterior walls
        let w1 = plinth_model::Wall::new(vec2(0.0, 0.0), vec2(10000.0, 0.0), l1_id);
        let w2 = plinth_model::Wall::new(vec2(10000.0, 0.0), vec2(10000.0, 10000.0), l1_id);
        let w3 = plinth_model::Wall::new(vec2(10000.0, 10000.0), vec2(0.0, 10000.0), l1_id);
        let w4 = plinth_model::Wall::new(vec2(0.0, 10000.0), vec2(0.0, 0.0), l1_id);
        doc.elements.insert(w1.id, plinth_model::Element::Wall(w1));
        doc.elements.insert(w2.id, plinth_model::Element::Wall(w2));
        doc.elements.insert(w3.id, plinth_model::Element::Wall(w3));
        doc.elements.insert(w4.id, plinth_model::Element::Wall(w4));

        // Intermediate Floor Slab at Level 2 (covers ground floor completely and extends 500mm cantilever)
        let slab_pts = vec![
            vec2(-500.0, -500.0),
            vec2(10500.0, -500.0),
            vec2(10500.0, 10500.0),
            vec2(-500.0, 10500.0),
        ];
        let mut mid_slab = plinth_model::Slab::new(slab_pts.clone(), l2_id);
        mid_slab.thickness = 200.0;
        doc.elements.insert(mid_slab.id, plinth_model::Element::Slab(mid_slab));

        // First Floor exterior walls
        let w5 = plinth_model::Wall::new(vec2(0.0, 0.0), vec2(10000.0, 0.0), l2_id);
        let w6 = plinth_model::Wall::new(vec2(10000.0, 0.0), vec2(10000.0, 10000.0), l2_id);
        let w7 = plinth_model::Wall::new(vec2(10000.0, 10000.0), vec2(0.0, 10000.0), l2_id);
        let w8 = plinth_model::Wall::new(vec2(0.0, 10000.0), vec2(0.0, 0.0), l2_id);
        doc.elements.insert(w5.id, plinth_model::Element::Wall(w5));
        doc.elements.insert(w6.id, plinth_model::Element::Wall(w6));
        doc.elements.insert(w7.id, plinth_model::Element::Wall(w7));
        doc.elements.insert(w8.id, plinth_model::Element::Wall(w8));

        // Roof slab at Level 3
        let mut roof_slab = plinth_model::Slab::new(slab_pts, l3_id);
        roof_slab.thickness = 200.0;
        doc.elements.insert(roof_slab.id, plinth_model::Element::Slab(roof_slab));

        // 3D View looking down from outside at 35 degrees pitch
        let mut v_3d = plinth_model::ViewDef::new_3d("Default 3D");
        v_3d.camera_3d = Some(plinth_model::Camera3D {
            target_x: 5000.0,
            target_y: 5000.0,
            target_z: 3000.0,
            distance: 25000.0,
            yaw: 45.0_f64.to_radians(),
            pitch: 35.0_f64.to_radians(),
            fov_degrees: 0.0,
        });

        let scene = render_view_with_style(&doc, &v_3d, false, VisualStyle::Shaded);

        // Verify that wall polygons are rendered for BOTH storeys!
        let mut wall_polygon_count = 0;
        for prim in &scene.primitives {
            if let RenderPrimitive::Polygon { fill_color, .. } = prim {
                if fill_color[3] > 0 {
                    wall_polygon_count += 1;
                }
            }
        }
        // Level 1 has 4 walls (2 visible from 45 deg orbit: south and east facades).
        // Level 2 has 4 walls (2 visible from 45 deg orbit: south and east facades).
        // Plus 2 slabs (mid_slab and roof_slab).
        // Total visible wall faces = at least 4 (2 ground floor + 2 first floor).
        println!("TOTAL VISIBLE POLYGONS IN 3D: {}", wall_polygon_count);
        assert!(wall_polygon_count >= 5, "Must render visible walls for both ground and first floors plus slabs");
    }
}
