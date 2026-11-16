//! Floor Plan projection engine.

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_model::{Document, Element, ViewDef};

pub fn render_floor_plan(doc: &Document, view: &ViewDef, theme: &ColorTheme) -> RenderScene {
    let mut scene = RenderScene::new();

    let level_id = match view.level_id {
        Some(lid) => lid,
        None => return scene,
    };
    let level_z = doc.level_elevation(level_id);
    let cut_z = level_z + view.cut_offset;

    // 0. Render Structural Foundation Footings below/at current floor level
    for el in doc.elements.values() {
        if let Element::Footing(f) = el {
            let (_f_bot, f_top) = f.z_bounds(
                doc.level_elevation(f.base_level),
                f.top_level.map(|l| doc.level_elevation(l)),
            );
            if f_top <= cut_z && f_top >= level_z - 2000.0 {
                let outer = f.footprint();
                if outer.len() >= 3 {
                    scene.add(RenderPrimitive::Polygon {
                        points: outer.clone(),
                        fill_color: theme.slab_fill,
                        stroke_color: None,
                        stroke_width: 0.0,
                    });
                    for i in 0..outer.len() {
                        scene.add(RenderPrimitive::Line {
                            start: outer[i],
                            end: outer[(i + 1) % outer.len()],
                            color: theme.proj_line,
                            width: 1.3,
                            dashed: f_top < level_z - 50.0,
                        });
                    }
                    if f.footing_style != plinth_model::FootingStyle::Pad {
                        let ratio = if f.footing_style == plinth_model::FootingStyle::Stepped {
                            0.60
                        } else {
                            0.55
                        };
                        let inner = f.rect_corners(f.width * ratio, f.length * ratio);
                        for i in 0..4 {
                            scene.add(RenderPrimitive::Line {
                                start: inner[i],
                                end: inner[(i + 1) % 4],
                                color: theme.hidden_line,
                                width: 1.0,
                                dashed: false,
                            });
                            if f.footing_style == plinth_model::FootingStyle::Sloped {
                                scene.add(RenderPrimitive::Line {
                                    start: outer[i],
                                    end: inner[i],
                                    color: theme.hidden_line,
                                    width: 0.9,
                                    dashed: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 0b. Render Toposolid (Terrain footprint, contour lines, spot elevation markers)
    for el in doc.elements.values() {
        if let Element::Toposolid(ts) = el {
            if ts.boundary.len() >= 3 {
                // Subtle translucent terrain tint
                let topo_fill = [205, 222, 195, 120];
                scene.add(RenderPrimitive::Polygon {
                    points: ts.boundary.clone(),
                    fill_color: topo_fill,
                    stroke_color: None,
                    stroke_width: 0.0,
                });
                for i in 0..ts.boundary.len() {
                    scene.add(RenderPrimitive::Line {
                        start: ts.boundary[i],
                        end: ts.boundary[(i + 1) % ts.boundary.len()],
                        color: [90, 125, 70, 255],
                        width: 1.6,
                        dashed: false,
                    });
                }
                // Render computed contour lines
                for contour in ts.compute_contour_lines() {
                    let color = if contour.is_primary {
                        [115, 85, 55, 255] // Bold Primary Index Contour (Brown)
                    } else {
                        [165, 140, 110, 180] // Intermediate Contour
                    };
                    let width = if contour.is_primary { 1.5 } else { 0.9 };
                    for w in contour.points.windows(2) {
                        scene.add(RenderPrimitive::Line {
                            start: w[0],
                            end: w[1],
                            color,
                            width,
                            dashed: false,
                        });
                    }
                    if contour.is_primary && contour.points.len() >= 2 {
                        let mid = (contour.points[0] + contour.points[1]) * 0.5;
                        scene.add(RenderPrimitive::Text {
                            pos: mid,
                            text: format!("{:.0}", contour.elevation),
                            color,
                            size: 9.0,
                            align_center: true,
                        });
                    }
                }
            }
        }
    }

    // 0c. Render Building Pads (Concrete excavation boundary and hatch)
    for el in doc.elements.values() {
        if let Element::BuildingPad(bp) = el {
            if bp.boundary.len() >= 3 {
                let pad_fill = [218, 220, 224, 200];
                scene.add(RenderPrimitive::Polygon {
                    points: bp.boundary.clone(),
                    fill_color: pad_fill,
                    stroke_color: None,
                    stroke_width: 0.0,
                });
                for i in 0..bp.boundary.len() {
                    scene.add(RenderPrimitive::Line {
                        start: bp.boundary[i],
                        end: bp.boundary[(i + 1) % bp.boundary.len()],
                        color: theme.proj_line,
                        width: 1.8,
                        dashed: false,
                    });
                }
            }
        }
    }

    // 0d. Render Property Lines (Official Cadastral Survey Boundary Line & Bearings)
    for el in doc.elements.values() {
        if let Element::PropertyLine(pl) = el {
            let n = pl.boundary.len();
            if n >= 2 {
                let prop_color = [190, 45, 45, 255]; // Cadastral Red
                for i in 0..n {
                    let next = (i + 1) % n;
                    // If open boundary (not closed), don't wrap last to first
                    if next == 0 && (pl.boundary[0] - pl.boundary[n - 1]).length() > 50.0 {
                        continue;
                    }
                    let p0 = pl.boundary[i];
                    let p1 = pl.boundary[next];
                    scene.add(RenderPrimitive::Line {
                        start: p0,
                        end: p1,
                        color: prop_color,
                        width: 2.2,
                        dashed: true,
                    });
                    // Corner iron pin / monument marker
                    scene.add(RenderPrimitive::Circle {
                        center: p0,
                        radius: 3.5,
                        fill_color: Some(prop_color),
                        stroke_color: [255, 255, 255, 255],
                        stroke_width: 1.0,
                    });
                    // Bearing and distance label along boundary segment
                    if i < pl.survey_table.len() {
                        let seg = &pl.survey_table[i];
                        let mid = (p0 + p1) * 0.5;
                        let dir = (p1 - p0).normalized();
                        let norm = plinth_geom::vec2(-dir.y, dir.x);
                        let text_pos = mid + norm * 350.0;
                        scene.add(RenderPrimitive::Text {
                            pos: text_pos,
                            text: format!("{}  {:.2}m", seg.format_bearing(), seg.distance_m),
                            color: prop_color,
                            size: 10.0,
                            align_center: true,
                        });
                    }
                }
            }
        }
    }

    // 0e. Render Roads, Sidewalks, and Pathwalks (Site Infrastructure)
    for el in doc.elements.values() {
        match el {
            Element::Road(rd) => {
                let poly = rd.footprint();
                if poly.len() >= 3 {
                    // Asphalt road surface
                    let asphalt_fill = [50, 52, 58, 255];
                    scene.add(RenderPrimitive::Polygon {
                        points: poly.clone(),
                        fill_color: asphalt_fill,
                        stroke_color: Some([180, 182, 188, 255]),
                        stroke_width: 1.8,
                    });

                    // Yellow painted centerline
                    if rd.has_centerline && rd.path.len() >= 2 {
                        let yellow = [245, 200, 35, 255];
                        for i in 0..rd.path.len() - 1 {
                            scene.add(RenderPrimitive::Line {
                                start: rd.path[i],
                                end: rd.path[i + 1],
                                color: yellow,
                                width: 2.2,
                                dashed: true,
                            });
                        }
                    }
                }
            }
            Element::Sidewalk(sw) => {
                let poly = sw.footprint();
                if poly.len() >= 3 {
                    // Clean concrete sidewalk pavement
                    let concrete_fill = [228, 229, 232, 255];
                    scene.add(RenderPrimitive::Polygon {
                        points: poly.clone(),
                        fill_color: concrete_fill,
                        stroke_color: Some([140, 142, 148, 255]),
                        stroke_width: 1.5,
                    });

                    // Transverse contraction joint grooves across sidewalk width
                    if sw.has_top_groove && sw.path.len() >= 2 {
                        let half_w = sw.width * 0.5;
                        let groove_spacing = sw.groove_spacing.max(800.0);
                        let groove_color = [170, 172, 178, 255];

                        for i in 0..sw.path.len() - 1 {
                            let p0 = sw.path[i];
                            let p1 = sw.path[i + 1];
                            let seg_vec = p1 - p0;
                            let seg_len = seg_vec.length();
                            if seg_len < 10.0 {
                                continue;
                            }
                            let seg_dir = seg_vec / seg_len;
                            let norm = plinth_geom::vec2(-seg_dir.y, seg_dir.x);

                            let steps = (seg_len / groove_spacing).floor() as usize;
                            for s in 1..=steps {
                                let t_dist = (s as f64) * groove_spacing;
                                if t_dist < seg_len {
                                    let pt = p0 + seg_dir * t_dist;
                                    scene.add(RenderPrimitive::Line {
                                        start: pt - norm * half_w,
                                        end: pt + norm * half_w,
                                        color: groove_color,
                                        width: 1.0,
                                        dashed: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Element::Pathwalk(pw) => {
                let poly = pw.footprint();
                if poly.len() >= 3 {
                    let fill_color = match pw.profile {
                        plinth_model::PathwalkProfile::PaverBlocks => [212, 185, 168, 255],
                        plinth_model::PathwalkProfile::Cobblestone => [180, 180, 182, 255],
                        plinth_model::PathwalkProfile::Flagstone => [185, 195, 198, 255],
                        plinth_model::PathwalkProfile::GravelBed => [218, 212, 198, 255],
                        plinth_model::PathwalkProfile::TimberBoardwalk => [198, 165, 128, 255],
                    };
                    scene.add(RenderPrimitive::Polygon {
                        points: poly.clone(),
                        fill_color,
                        stroke_color: Some([130, 125, 120, 255]),
                        stroke_width: 1.4,
                    });

                    // Timber boardwalk planks or paver joints
                    if pw.path.len() >= 2 {
                        let half_w = pw.width * 0.5;
                        let joint_spacing = match pw.profile {
                            plinth_model::PathwalkProfile::TimberBoardwalk => 150.0,
                            plinth_model::PathwalkProfile::PaverBlocks => 200.0,
                            _ => 1000.0,
                        };
                        let joint_color = [110, 105, 100, 200];

                        for i in 0..pw.path.len() - 1 {
                            let p0 = pw.path[i];
                            let p1 = pw.path[i + 1];
                            let seg_vec = p1 - p0;
                            let seg_len = seg_vec.length();
                            if seg_len < 10.0 {
                                continue;
                            }
                            let seg_dir = seg_vec / seg_len;
                            let norm = plinth_geom::vec2(-seg_dir.y, seg_dir.x);

                            let steps = (seg_len / joint_spacing).floor() as usize;
                            for s in 1..=steps {
                                let t_dist = (s as f64) * joint_spacing;
                                if t_dist < seg_len {
                                    let pt = p0 + seg_dir * t_dist;
                                    scene.add(RenderPrimitive::Line {
                                        start: pt - norm * half_w,
                                        end: pt + norm * half_w,
                                        color: joint_color,
                                        width: 0.8,
                                        dashed: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 1. Render Slabs (floors) in projection
    for el in doc.elements.values() {
        if let Element::Slab(s) = el {
            let (_slab_bot, slab_z) = s.z_bounds(
                doc.level_elevation(s.level),
                s.top_level.map(|l| doc.level_elevation(l)),
            );
            // Visible if it's at or slightly below current floor
            if slab_z <= cut_z && slab_z >= level_z - 1000.0 && s.boundary.len() >= 3 {
                scene.add(RenderPrimitive::Polygon {
                    points: s.boundary.clone(),
                    fill_color: theme.slab_fill,
                    stroke_color: Some(theme.proj_line),
                    stroke_width: 1.0,
                });
            }
        }
    }

    // 1b. Render Freeform Planar Surfaces & Solids
    for el in doc.elements.values() {
        match el {
            Element::FreeformSurface(surf) => {
                if surf.boundary.len() >= 3 {
                    scene.add(RenderPrimitive::Polygon {
                        points: surf.boundary.clone(),
                        fill_color: [210, 235, 255, 170],
                        stroke_color: Some([0, 120, 215, 255]),
                        stroke_width: 1.5,
                    });
                }
            }
            Element::FreeformSolid(solid) => {
                if solid.boundary.len() >= 3 {
                    if cut_z >= solid.z_min && cut_z <= solid.z_max {
                        scene.add(RenderPrimitive::Polygon {
                            points: solid.boundary.clone(),
                            fill_color: theme.wall_fill,
                            stroke_color: Some(theme.cut_line),
                            stroke_width: 2.0,
                        });
                    } else if solid.z_max <= cut_z && solid.z_max >= level_z - 1000.0 {
                        scene.add(RenderPrimitive::Polygon {
                            points: solid.boundary.clone(),
                            fill_color: theme.slab_fill,
                            stroke_color: Some(theme.proj_line),
                            stroke_width: 1.0,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // 2. Render Walls and their Hosted Doors/Windows
    let all_walls: Vec<&plinth_model::Wall> = doc
        .elements
        .values()
        .filter_map(|e| match e {
            Element::Wall(w) => Some(w),
            _ => None,
        })
        .collect();

    for el in doc.elements.values() {
        if let Element::Wall(w) = el {
            let (base_z, top_z) = w.z_bounds(
                doc.level_elevation(w.base_level),
                w.top_level.map(|tl| doc.level_elevation(tl)),
            );

            // Does wall intersect cut plane?
            if cut_z >= base_z && cut_z <= top_z {
                let dir = w.direction();
                let norm = w.normal();
                let half_thick = w.thickness * 0.5;
                let wall_len = w.length();

                let (miter_poly, start_joined, end_joined) = w.mitered_footprint(&all_walls);
                let p_start_left = miter_poly[0];
                let p_end_left = miter_poly[1];
                let p_end_right = miter_poly[2];
                let p_start_right = miter_poly[3];

                let pt_left = |d: f64| {
                    if d <= 0.5 {
                        p_start_left
                    } else if d >= wall_len - 0.5 {
                        p_end_left
                    } else {
                        w.start + dir * d - norm * half_thick
                    }
                };
                let pt_right = |d: f64| {
                    if d <= 0.5 {
                        p_start_right
                    } else if d >= wall_len - 0.5 {
                        p_end_right
                    } else {
                        w.start + dir * d + norm * half_thick
                    }
                };

                // Collect plan-cut openings and cross-joint spans
                let mut cut_spans: Vec<(f64, f64)> = Vec::new();
                for op in doc.hosted_openings_for_wall(w.id) {
                    let op_z0 = base_z + op.sill_height;
                    let op_z1 = op_z0 + op.height;
                    if cut_z >= op_z0 && cut_z <= op_z1 {
                        let s = (op.offset_along - op.width * 0.5).max(0.0);
                        let e = (op.offset_along + op.width * 0.5).min(wall_len);
                        if e > s + 1.0 {
                            cut_spans.push((s, e));
                        }
                    }
                }
                for (cs, ce) in w.cross_joint_spans(&all_walls) {
                    if ce > cs + 1.0 {
                        cut_spans.push((cs, ce));
                    }
                }
                cut_spans.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                let mut emit_plan_span = |d0: f64, d1: f64| {
                    if d1 <= d0 + 1.0 {
                        return;
                    }
                    let pl0 = pt_left(d0);
                    let pl1 = pt_left(d1);
                    let pr1 = pt_right(d1);
                    let pr0 = pt_right(d0);
                    scene.add(RenderPrimitive::Polygon {
                        points: vec![pl0, pl1, pr1, pr0],
                        fill_color: theme.wall_fill,
                        stroke_color: None,
                        stroke_width: 0.0,
                    });
                    // Outer left and right wall lines
                    scene.add(RenderPrimitive::Line {
                        start: pl0,
                        end: pl1,
                        color: theme.cut_line,
                        width: 2.0,
                        dashed: false,
                    });
                    scene.add(RenderPrimitive::Line {
                        start: pr1,
                        end: pr0,
                        color: theme.cut_line,
                        width: 2.0,
                        dashed: false,
                    });
                    // End caps (only drawn when unjoined or at opening jambs, never across mitered/T-joined wall joints)
                    if d0 > 0.5 || !start_joined {
                        scene.add(RenderPrimitive::Line {
                            start: pr0,
                            end: pl0,
                            color: theme.cut_line,
                            width: 2.0,
                            dashed: false,
                        });
                    }
                    if d1 < wall_len - 0.5 || !end_joined {
                        scene.add(RenderPrimitive::Line {
                            start: pl1,
                            end: pr1,
                            color: theme.cut_line,
                            width: 2.0,
                            dashed: false,
                        });
                    }
                };

                if cut_spans.is_empty() {
                    emit_plan_span(0.0, wall_len);
                } else {
                    let mut curr_dist = 0.0;
                    for (op_start, op_end) in cut_spans {
                        let s = op_start.max(curr_dist);
                        let e = op_end.max(s);
                        if s > curr_dist + 1.0 {
                            emit_plan_span(curr_dist, s);
                        }
                        curr_dist = e;
                    }
                    if wall_len > curr_dist + 1.0 {
                        emit_plan_span(curr_dist, wall_len);
                    }
                }
            }
        }
    }

    // 3. Render Hosted Doors (Panel + Swing Arc)
    for el in doc.elements.values() {
        if let Element::Door(d) = el {
            if let Some(Element::Wall(w)) = doc.elements.get(&d.host_wall) {
                let wall_z0 = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|tl| doc.level_elevation(tl)),
                ).0;
                let d_base_z = d.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                let (d_z0, d_z1) = d.z_bounds(d_base_z, d.top_level.map(|l| doc.level_elevation(l)));
                if cut_z >= d_z0 && cut_z <= d_z1 {
                    let dir = w.direction();
                    let norm = w.normal();
                    let hinge = w.start + dir * (d.offset_along_wall - d.width * 0.5);
                    let strike = w.start + dir * (d.offset_along_wall + d.width * 0.5);

                    // Door jamb lines
                    scene.add(RenderPrimitive::Line {
                        start: hinge - norm * (w.thickness * 0.5),
                        end: hinge + norm * (w.thickness * 0.5),
                        color: theme.cut_line,
                        width: 1.5,
                        dashed: false,
                    });
                    scene.add(RenderPrimitive::Line {
                        start: strike - norm * (w.thickness * 0.5),
                        end: strike + norm * (w.thickness * 0.5),
                        color: theme.cut_line,
                        width: 1.5,
                        dashed: false,
                    });

                    // Door panel swung 90 degrees
                    let swing_dir = if d.flipped_face { -norm } else { norm };
                    let open_panel_end = hinge + swing_dir * d.width;
                    scene.add(RenderPrimitive::Line {
                        start: hinge,
                        end: open_panel_end,
                        color: theme.cut_line,
                        width: 1.5,
                        dashed: false,
                    });

                    // Swing arc (quarter circle)
                    let start_ang = (strike - hinge).angle() as f32;
                    let end_ang = (open_panel_end - hinge).angle() as f32;
                    let (a0, a1) = if end_ang >= start_ang { (start_ang, end_ang) } else { (end_ang, start_ang) };
                    scene.add(RenderPrimitive::Arc {
                        center: hinge,
                        radius: d.width as f32,
                        start_angle: a0,
                        end_angle: a1,
                        color: theme.proj_line,
                        width: 1.0,
                    });
                }
            }
        }
    }

    // 4. Render Hosted Windows (Sill lines + Glazing)
    for el in doc.elements.values() {
        if let Element::Window(win) = el {
            if let Some(Element::Wall(w)) = doc.elements.get(&win.host_wall) {
                let wall_z0 = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|tl| doc.level_elevation(tl)),
                ).0;
                let w_base_z = win.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                let (win_bottom, win_top) = win.z_bounds(w_base_z, win.top_level.map(|l| doc.level_elevation(l)));
                if cut_z >= win_bottom && cut_z <= win_top {
                    let dir = w.direction();
                    let norm = w.normal();
                    let half_thick = w.thickness * 0.5;

                    let p0 = w.start + dir * (win.offset_along_wall - win.width * 0.5);
                    let p1 = w.start + dir * (win.offset_along_wall + win.width * 0.5);

                    // Wall jambs
                    scene.add(RenderPrimitive::Line {
                        start: p0 - norm * half_thick,
                        end: p0 + norm * half_thick,
                        color: theme.cut_line,
                        width: 1.5,
                        dashed: false,
                    });
                    scene.add(RenderPrimitive::Line {
                        start: p1 - norm * half_thick,
                        end: p1 + norm * half_thick,
                        color: theme.cut_line,
                        width: 1.5,
                        dashed: false,
                    });

                    // Outer & inner sill lines
                    scene.add(RenderPrimitive::Line {
                        start: p0 - norm * half_thick,
                        end: p1 - norm * half_thick,
                        color: theme.proj_line,
                        width: 1.0,
                        dashed: false,
                    });
                    scene.add(RenderPrimitive::Line {
                        start: p0 + norm * half_thick,
                        end: p1 + norm * half_thick,
                        color: theme.proj_line,
                        width: 1.0,
                        dashed: false,
                    });

                    // Glass center line
                    scene.add(RenderPrimitive::Line {
                        start: p0,
                        end: p1,
                        color: theme.glass_stroke,
                        width: 1.8,
                        dashed: false,
                    });
                }
            }
        }
    }

    // 4b. Render Curtain Walls (Mullions + Double Glazing)
    for el in doc.elements.values() {
        if let Element::CurtainWall(cw) = el {
            let (cw_z0, cw_z1) = cw.z_bounds(
                doc.level_elevation(cw.base_level),
                cw.top_level.map(|tl| doc.level_elevation(tl)),
            );
            if cut_z >= cw_z0 && cut_z <= cw_z1 {
                let len = cw.length();
                if len >= 10.0 {
                    let dir = cw.direction();
                    let norm = cw.normal();
                    let half_d = cw.mullion_depth * 0.5;
                    let half_w = (cw.mullion_width * 0.5).min(len * 0.2);
                    let half_glass = (cw.panel_thickness * 0.5).min(half_d * 0.4);

                    let n_v = (len / cw.vertical_grid_spacing.max(200.0)).round().max(1.0) as usize;
                    let bay_w = len / (n_v as f64);

                    // Glass lines along the curtain wall length
                    scene.add(RenderPrimitive::Line {
                        start: cw.start - norm * half_glass,
                        end: cw.end - norm * half_glass,
                        color: theme.glass_stroke,
                        width: 1.5,
                        dashed: false,
                    });
                    scene.add(RenderPrimitive::Line {
                        start: cw.start + norm * half_glass,
                        end: cw.end + norm * half_glass,
                        color: theme.glass_stroke,
                        width: 1.5,
                        dashed: false,
                    });

                    // Mullion box profiles in plan cut
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
                        let m_poly = vec![
                            p0 - norm * half_d,
                            p1 - norm * half_d,
                            p1 + norm * half_d,
                            p0 + norm * half_d,
                        ];
                        scene.add(RenderPrimitive::Polygon {
                            points: m_poly,
                            fill_color: [60, 65, 75, 255],
                            stroke_color: Some(theme.cut_line),
                            stroke_width: 1.2,
                        });
                    }
                }
            }
        }
    }

    // 5. Render Columns
    for el in doc.elements.values() {
        if let Element::Column(c) = el {
            let (base_z, top_z) = c.z_bounds(
                doc.level_elevation(c.base_level),
                c.top_level.map(|tl| doc.level_elevation(tl)),
            );
            if cut_z >= base_z && cut_z <= top_z {
                let foot = c.footprint();
                if foot.len() >= 3 {
                    scene.add(RenderPrimitive::Polygon {
                        points: foot,
                        fill_color: theme.col_fill,
                        stroke_color: Some(theme.cut_line),
                        stroke_width: 2.0,
                    });
                }
            }
        }
    }

    // 6. Render Structural Beams (dashed overhead lines)
    for el in doc.elements.values() {
        if let Element::Beam(b) = el {
            let (_beam_bot, beam_top) = b.z_bounds(
                doc.level_elevation(b.level),
                b.top_level.map(|tl| doc.level_elevation(tl)),
            );
            if (beam_top - cut_z).abs() < 4000.0 {
                let norm = (b.end - b.start).normalized().perp() * (b.width * 0.5);
                scene.add(RenderPrimitive::Line {
                    start: b.start - norm,
                    end: b.end - norm,
                    color: theme.hidden_line,
                    width: 1.0,
                    dashed: true,
                });
                scene.add(RenderPrimitive::Line {
                    start: b.start + norm,
                    end: b.end + norm,
                    color: theme.hidden_line,
                    width: 1.0,
                    dashed: true,
                });
            }
        }
    }

    // 7. Render Grids
    for grid in doc.grids.values() {
        scene.add(RenderPrimitive::Line {
            start: grid.start,
            end: grid.end,
            color: theme.grid_line,
            width: 1.0,
            dashed: true,
        });

        // Bubble at start
        let radius = 400.0;
        let dir = (grid.start - grid.end).normalized();
        let center = grid.start + dir * radius;
        scene.add(RenderPrimitive::Circle {
            center,
            radius: radius as f32,
            fill_color: Some(theme.background),
            stroke_color: theme.grid_line,
            stroke_width: 1.2,
        });
        scene.add(RenderPrimitive::Text {
            pos: center,
            text: grid.name.clone(),
            color: theme.text_color,
            size: 14.0,
            align_center: true,
        });
    }

    // 8. Render Dimensions and Annotations
    for el in doc.elements.values() {
        match el {
            Element::Dimension(d) => {
                crate::annotation::render_dimension(&mut scene, d, theme, view.scale);
            }
            Element::TextNote(t) => {
                crate::annotation::render_text_note(&mut scene, t, theme, view.scale);
            }
            Element::Leader(ld) => {
                crate::annotation::render_leader(&mut scene, ld, theme, view.scale);
            }
            _ => {}
        }
    }

    // 9. Stairs, Ramps, Railings, Roofs, Fascias, and Gutters
    for el in doc.elements.values() {
        match el {
            Element::Stair(s) => {
                let (sz0, sz1) = s.z_bounds(
                    doc.level_elevation(s.base_level),
                    s.top_level.map(|l| doc.level_elevation(l)),
                );
                if sz1 < level_z - 500.0 || sz0 > cut_z + 3000.0 {
                    continue;
                }
                let n = s.riser_count.max(2);
                let dir = s.direction();
                let norm = s.normal() * (s.width * 0.5);
                let run_len = s.length();
                let step_len = run_len / (n as f64);

                let p_start_l = s.start - norm;
                let p_start_r = s.start + norm;
                let p_end_l = s.end - norm;
                let p_end_r = s.end + norm;

                // Subtle stair plan fill
                scene.add(RenderPrimitive::Polygon {
                    points: vec![p_start_l, p_end_l, p_end_r, p_start_r],
                    fill_color: theme.slab_fill,
                    stroke_color: None,
                    stroke_width: 0.0,
                });

                // Outer boundary
                scene.add(RenderPrimitive::Line { start: p_start_l, end: p_start_r, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_start_r, end: p_end_r, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_end_r, end: p_end_l, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_end_l, end: p_start_l, color: theme.cut_line, width: 1.5, dashed: false });

                // Tread lines (solid below cut plane, dashed above cut plane)
                let step_h = (sz1 - sz0) / (n as f64);
                for i in 1..n {
                    let step_c = s.start + dir * (i as f64 * step_len);
                    let above_cut = (sz0 + (i as f64) * step_h) > cut_z;
                    scene.add(RenderPrimitive::Line {
                        start: step_c - norm,
                        end: step_c + norm,
                        color: if above_cut { theme.hidden_line } else { theme.proj_line },
                        width: 1.0,
                        dashed: above_cut,
                    });
                }

                // Up arrow + "UP" label
                let arrow_start = s.start + dir * (run_len * 0.15);
                let arrow_end = s.start + dir * (run_len * 0.85);
                scene.add(RenderPrimitive::Line { start: arrow_start, end: arrow_end, color: theme.dim_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line {
                    start: arrow_end,
                    end: arrow_end - dir * 200.0 + norm * 0.25,
                    color: theme.dim_line,
                    width: 1.5,
                    dashed: false,
                });
                scene.add(RenderPrimitive::Line {
                    start: arrow_end,
                    end: arrow_end - dir * 200.0 - norm * 0.25,
                    color: theme.dim_line,
                    width: 1.5,
                    dashed: false,
                });
                scene.add(RenderPrimitive::Text {
                    pos: s.start + dir * (run_len * 0.10),
                    text: "UP".into(),
                    color: theme.text_color,
                    size: 11.0,
                    align_center: true,
                });
            }
            Element::Ramp(r) => {
                let (rz0, rz1) = r.z_bounds(
                    doc.level_elevation(r.base_level),
                    r.top_level.map(|l| doc.level_elevation(l)),
                );
                if rz1 < level_z - 500.0 || rz0 > cut_z + 3000.0 {
                    continue;
                }
                let dir = r.direction();
                let norm = r.normal() * (r.width * 0.5);
                let run_len = r.length();
                let p_start_l = r.start - norm;
                let p_start_r = r.start + norm;
                let p_end_l = r.end - norm;
                let p_end_r = r.end + norm;

                scene.add(RenderPrimitive::Polygon {
                    points: vec![p_start_l, p_end_l, p_end_r, p_start_r],
                    fill_color: theme.slab_fill,
                    stroke_color: None,
                    stroke_width: 0.0,
                });

                scene.add(RenderPrimitive::Line { start: p_start_l, end: p_start_r, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_start_r, end: p_end_r, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_end_r, end: p_end_l, color: theme.cut_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line { start: p_end_l, end: p_start_l, color: theme.cut_line, width: 1.5, dashed: false });

                // Slope arrow + ratio label
                let arrow_start = r.start + dir * (run_len * 0.15);
                let arrow_end = r.start + dir * (run_len * 0.85);
                scene.add(RenderPrimitive::Line { start: arrow_start, end: arrow_end, color: theme.dim_line, width: 1.5, dashed: false });
                scene.add(RenderPrimitive::Line {
                    start: arrow_end,
                    end: arrow_end - dir * 200.0 + norm * 0.25,
                    color: theme.dim_line,
                    width: 1.5,
                    dashed: false,
                });
                scene.add(RenderPrimitive::Line {
                    start: arrow_end,
                    end: arrow_end - dir * 200.0 - norm * 0.25,
                    color: theme.dim_line,
                    width: 1.5,
                    dashed: false,
                });
                let ratio = r.slope_ratio(
                    doc.level_elevation(r.base_level),
                    r.top_level.map(|l| doc.level_elevation(l)),
                );
                scene.add(RenderPrimitive::Text {
                    pos: r.start + dir * (run_len * 0.5) + r.normal() * 140.0,
                    text: format!("UP 1:{:.1}", ratio),
                    color: theme.text_color,
                    size: 11.0,
                    align_center: true,
                });
            }
            Element::Railing(rl) => {
                for i in 0..rl.path.len().saturating_sub(1) {
                    let p1 = rl.path[i];
                    let p2 = rl.path[i + 1];
                    scene.add(RenderPrimitive::Line { start: p1, end: p2, color: theme.cut_line, width: 2.0, dashed: false });
                }
            }
            _ => {}
        }
    }

    // 10. Detail Lines
    for el in doc.elements.values() {
        if let Element::DetailLine(l) = el {
            scene.add(RenderPrimitive::Line {
                start: l.start,
                end: l.end,
                color: theme.proj_line,
                width: 1.5,
                dashed: l.style == "Hidden",
            });
        }
    }

    scene
}
