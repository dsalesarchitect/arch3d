//! Elevation views projection engine (North, South, East, West).

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_geom::{vec2, Vec2, Vec3};
use plinth_model::{Document, Element, ViewDef, ViewKind};

pub fn render_elevation(doc: &Document, view: &ViewDef, theme: &ColorTheme) -> RenderScene {
    let mut scene = RenderScene::new();

    // Map 3D point to 2D elevation coordinate (u, z)
    // u: horizontal coordinate on screen
    // z: vertical elevation coordinate (Z axis mm)
    let project_pt = |p: Vec3| -> Vec2 {
        match view.kind {
            ViewKind::ElevationSouth => vec2(p.x, p.z),
            ViewKind::ElevationNorth => vec2(-p.x, p.z),
            ViewKind::ElevationEast => vec2(-p.y, p.z),
            ViewKind::ElevationWest => vec2(p.y, p.z),
            _ => vec2(p.x, p.z),
        }
    };

    // View direction vector (towards building)
    let view_dir = match view.kind {
        ViewKind::ElevationSouth => Vec3::new(0.0, 1.0, 0.0),
        ViewKind::ElevationNorth => Vec3::new(0.0, -1.0, 0.0),
        ViewKind::ElevationEast => Vec3::new(-1.0, 0.0, 0.0),
        ViewKind::ElevationWest => Vec3::new(1.0, 0.0, 0.0),
        _ => Vec3::new(0.0, 1.0, 0.0),
    };

    // 1. Gather all element faces and project visible faces
    let mut faces_to_draw = Vec::new();

    for &id in doc.elements.keys() {
        for face in doc.element_faces(id) {
            // Face is visible if facing towards the camera (normal dot view_dir < 0)
            let facing = face.normal.dot(view_dir);
            if facing < -0.01 {
                let depth = face.centroid().dot(view_dir);
                faces_to_draw.push((depth, face));
            }
        }
    }

    // Sort back-to-front (painters algorithm: furthest faces drawn first)
    faces_to_draw.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_depth, face) in faces_to_draw {
        let poly_2d: Vec<Vec2> = face.pts.iter().map(|&p| project_pt(p)).collect();
        let n = poly_2d.len();
        if n >= 3 {
            scene.add(RenderPrimitive::Polygon {
                points: poly_2d.clone(),
                fill_color: theme.wall_fill,
                stroke_color: None,
                stroke_width: 0.0,
            });
            for i in 0..n {
                if face.hard.get(i).copied().unwrap_or(true) {
                    scene.add(RenderPrimitive::Line {
                        start: poly_2d[i],
                        end: poly_2d[(i + 1) % n],
                        color: theme.proj_line,
                        width: 1.2,
                        dashed: false,
                    });
                }
            }
        }
    }

    // 2. Project Doors and Windows on exterior walls
    for el in doc.elements.values() {
        match el {
            Element::Door(d) => {
                if let Some(Element::Wall(w)) = doc.elements.get(&d.host_wall) {
                    let wall_facing = w.normal().extend(0.0).dot(view_dir);
                    if wall_facing.abs() > 0.3 {
                        let wall_z0 = w.z_bounds(
                            doc.level_elevation(w.base_level),
                            w.top_level.map(|tl| doc.level_elevation(tl)),
                        ).0;
                        let d_base_z = d.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                        let (z0, z1) = d.z_bounds(d_base_z, d.top_level.map(|l| doc.level_elevation(l)));
                        let dir = w.direction();
                        let p_center = w.start + dir * d.offset_along_wall;
                        let half_w = d.width * 0.5;
                        let p0 = p_center - dir * half_w;
                        let p1 = p_center + dir * half_w;

                        let pts = vec![
                            project_pt(p0.extend(z0)),
                            project_pt(p1.extend(z0)),
                            project_pt(p1.extend(z1)),
                            project_pt(p0.extend(z1)),
                        ];
                        scene.add(RenderPrimitive::Polygon {
                            points: pts,
                            fill_color: [80, 50, 30, 255],
                            stroke_color: Some(theme.cut_line),
                            stroke_width: 1.5,
                        });
                    }
                }
            }
            Element::Window(win) => {
                if let Some(Element::Wall(w)) = doc.elements.get(&win.host_wall) {
                    let wall_facing = w.normal().extend(0.0).dot(view_dir);
                    if wall_facing.abs() > 0.3 {
                        let wall_z0 = w.z_bounds(
                            doc.level_elevation(w.base_level),
                            w.top_level.map(|tl| doc.level_elevation(tl)),
                        ).0;
                        let w_base_z = win.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                        let (z0, z1) = win.z_bounds(w_base_z, win.top_level.map(|l| doc.level_elevation(l)));
                        let dir = w.direction();
                        let p_center = w.start + dir * win.offset_along_wall;
                        let half_w = win.width * 0.5;
                        let p0 = p_center - dir * half_w;
                        let p1 = p_center + dir * half_w;

                        let pts = vec![
                            project_pt(p0.extend(z0)),
                            project_pt(p1.extend(z0)),
                            project_pt(p1.extend(z1)),
                            project_pt(p0.extend(z1)),
                        ];
                        scene.add(RenderPrimitive::Polygon {
                            points: pts,
                            fill_color: theme.glass_fill,
                            stroke_color: Some(theme.glass_stroke),
                            stroke_width: 1.5,
                        });
                    }
                }
            }
            Element::CurtainWall(cw) => {
                let cw_facing = cw.normal().extend(0.0).dot(view_dir);
                if cw_facing.abs() > 0.05 {
                    let (z0, z1) = cw.z_bounds(
                        doc.level_elevation(cw.base_level),
                        cw.top_level.map(|tl| doc.level_elevation(tl)),
                    );
                    let len = cw.length();
                    let h = z1 - z0;
                    if len > 10.0 && h > 10.0 {
                        let dir = cw.direction();
                        let half_w = (cw.mullion_width * 0.5).min(len * 0.2);
                        let n_v = (len / cw.vertical_grid_spacing.max(200.0)).round().max(1.0) as usize;
                        let bay_w = len / (n_v as f64);
                        let n_h = (h / cw.horizontal_grid_spacing.max(200.0)).round().max(1.0) as usize;
                        let tier_h = h / (n_h as f64);

                        let mullion_fill = [65u8, 70, 80, 255];
                        let mullion_stroke = [45u8, 50, 60, 255];

                        // 1. Individual Glazed Glass Panels inside each grid cell
                        for i in 0..n_v {
                            let bay_start = if i == 0 { half_w * 2.0 } else { i as f64 * bay_w + half_w };
                            let bay_end = if i + 1 == n_v { len - half_w * 2.0 } else { (i + 1) as f64 * bay_w - half_w };
                            if bay_end <= bay_start + 1.0 {
                                continue;
                            }
                            let bp0 = cw.start + dir * bay_start;
                            let bp1 = cw.start + dir * bay_end;

                            for j in 0..n_h {
                                let gz0 = if j == 0 { z0 + half_w * 2.0 } else { z0 + j as f64 * tier_h + half_w };
                                let gz1 = if j + 1 == n_h { z1 - half_w * 2.0 } else { z0 + (j + 1) as f64 * tier_h - half_w };
                                if gz1 > gz0 + 1.0 {
                                    scene.add(RenderPrimitive::Polygon {
                                        points: vec![
                                            project_pt(bp0.extend(gz0)),
                                            project_pt(bp1.extend(gz0)),
                                            project_pt(bp1.extend(gz1)),
                                            project_pt(bp0.extend(gz1)),
                                        ],
                                        fill_color: theme.glass_fill,
                                        stroke_color: Some(theme.glass_stroke),
                                        stroke_width: 1.0,
                                    });
                                }
                            }
                        }

                        // 2. Solid Vertical Mullion Profiles (outer boundary jambs at i=0, i=n_v and intermediate mullions)
                        for i in 0..=n_v {
                            let cx = i as f64 * bay_w;
                            let (d0, d1) = if i == 0 {
                                (0.0, half_w * 2.0)
                            } else if i == n_v {
                                (len - half_w * 2.0, len)
                            } else {
                                (cx - half_w, cx + half_w)
                            };
                            let mp0 = cw.start + dir * d0;
                            let mp1 = cw.start + dir * d1;
                            scene.add(RenderPrimitive::Polygon {
                                points: vec![
                                    project_pt(mp0.extend(z0)),
                                    project_pt(mp1.extend(z0)),
                                    project_pt(mp1.extend(z1)),
                                    project_pt(mp0.extend(z1)),
                                ],
                                fill_color: mullion_fill,
                                stroke_color: Some(mullion_stroke),
                                stroke_width: 1.2,
                            });
                        }

                        // 3. Solid Horizontal Transoms (bottom sill, top head, and intermediate transoms)
                        for i in 0..n_v {
                            let bay_start = if i == 0 { half_w * 2.0 } else { i as f64 * bay_w + half_w };
                            let bay_end = if i + 1 == n_v { len - half_w * 2.0 } else { (i + 1) as f64 * bay_w - half_w };
                            if bay_end <= bay_start + 1.0 {
                                continue;
                            }
                            let bp0 = cw.start + dir * bay_start;
                            let bp1 = cw.start + dir * bay_end;

                            // Bottom sill rail
                            scene.add(RenderPrimitive::Polygon {
                                points: vec![
                                    project_pt(bp0.extend(z0)),
                                    project_pt(bp1.extend(z0)),
                                    project_pt(bp1.extend((z0 + half_w * 2.0).min(z1))),
                                    project_pt(bp0.extend((z0 + half_w * 2.0).min(z1))),
                                ],
                                fill_color: mullion_fill,
                                stroke_color: Some(mullion_stroke),
                                stroke_width: 1.2,
                            });

                            // Top head rail
                            scene.add(RenderPrimitive::Polygon {
                                points: vec![
                                    project_pt(bp0.extend((z1 - half_w * 2.0).max(z0))),
                                    project_pt(bp1.extend((z1 - half_w * 2.0).max(z0))),
                                    project_pt(bp1.extend(z1)),
                                    project_pt(bp0.extend(z1)),
                                ],
                                fill_color: mullion_fill,
                                stroke_color: Some(mullion_stroke),
                                stroke_width: 1.2,
                            });

                            // Intermediate horizontal transoms
                            for j in 1..n_h {
                                let tz = z0 + j as f64 * tier_h;
                                let tz0 = (tz - half_w).max(z0);
                                let tz1 = (tz + half_w).min(z1);
                                if tz1 > tz0 + 1.0 {
                                    scene.add(RenderPrimitive::Polygon {
                                        points: vec![
                                            project_pt(bp0.extend(tz0)),
                                            project_pt(bp1.extend(tz0)),
                                            project_pt(bp1.extend(tz1)),
                                            project_pt(bp0.extend(tz1)),
                                        ],
                                        fill_color: mullion_fill,
                                        stroke_color: Some(mullion_stroke),
                                        stroke_width: 1.2,
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

    // 3. Level Datum lines and Bubbles
    let span_x = if scene.bounds.is_valid() {
        (scene.bounds.min.x - 2000.0, scene.bounds.max.x + 2000.0)
    } else {
        (-5000.0, 15000.0)
    };

    for level in doc.levels.values() {
        let z = level.elevation;
        let p_start = vec2(span_x.0, z);
        let p_end = vec2(span_x.1, z);

        scene.add(RenderPrimitive::Line {
            start: p_start,
            end: p_end,
            color: theme.level_line,
            width: 1.0,
            dashed: true,
        });

        // Bubble at right end
        let bubble_center = vec2(span_x.1 + 400.0, z);
        scene.add(RenderPrimitive::Circle {
            center: bubble_center,
            radius: 350.0,
            fill_color: Some(theme.background),
            stroke_color: theme.level_line,
            stroke_width: 1.2,
        });
        scene.add(RenderPrimitive::Text {
            pos: vec2(bubble_center.x + 500.0, z + 80.0),
            text: format!("{}  {:.0} mm", level.name, level.elevation),
            color: theme.text_color,
            size: 13.0,
            align_center: false,
        });
    }

    // 4. Heavy Ground Line at Z = 0
    scene.add(RenderPrimitive::Line {
        start: vec2(span_x.0 - 1000.0, 0.0),
        end: vec2(span_x.1 + 1000.0, 0.0),
        color: theme.cut_line,
        width: 2.5,
        dashed: false,
    });

    // 5. Annotations (Dimensions, Text Notes, Leaders)
    for el in doc.elements.values() {
        match el {
            Element::Dimension(d) if d.view_id == view.id => {
                crate::annotation::render_dimension(&mut scene, d, theme, view.scale);
            }
            Element::TextNote(t) if t.view_id == view.id => {
                crate::annotation::render_text_note(&mut scene, t, theme, view.scale);
            }
            Element::Leader(ld) if ld.view_id == view.id => {
                crate::annotation::render_leader(&mut scene, ld, theme, view.scale);
            }
            _ => {}
        }
    }

    scene
}
