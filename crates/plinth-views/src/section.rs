//! Building Section cutting and projection engine.

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_geom::{polygon, vec2, Vec2};
use plinth_model::{Document, Element, ViewDef};

pub fn render_section(doc: &Document, view: &ViewDef, theme: &ColorTheme) -> RenderScene {
    let mut scene = RenderScene::new();

    let (s1, s2) = match view.section_line {
        Some(line) => line,
        None => return scene,
    };

    let line_vec = s2 - s1;
    let line_len = line_vec.length();
    if line_len < 1.0 {
        return scene;
    }
    let u_dir = line_vec / line_len; // Unit vector along section cut
    // Viewing direction normal pointing beyond the cut line into the building interior
    let v_dir = vec2(-u_dir.y, u_dir.x);
    let max_depth = 30_000.0; // 30m view depth

    // 1. Gather Background Projected Elements (Columns, Beams, Footings, Walls beyond cut plane)
    let mut proj_elements: Vec<(f64, Vec<Vec2>, [u8; 4], [u8; 4], f32)> = Vec::new();

    for el in doc.elements.values() {
        match el {
            Element::Column(c) => {
                let fp = c.footprint();
                // If cut directly by the section line, it will be handled in cut solids
                let is_cut = !polygon::line_polygon_intervals(s1, u_dir, &fp).is_empty();
                if !is_cut {
                    let d = (c.location - s1).dot(v_dir);
                    if d > 0.0 && d < max_depth {
                        let ts: Vec<f64> = fp.iter().map(|&p| (p - s1).dot(u_dir)).collect();
                        let t_min = ts.iter().copied().fold(f64::INFINITY, f64::min);
                        let t_max = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        let base_z = doc.level_elevation(c.base_level);
                        let top_z = c.top_level.map(|l| doc.level_elevation(l));
                        let (z0, z1) = c.z_bounds(base_z, top_z);
                        if z1 > z0 + 1.0 && t_max > t_min + 1.0 {
                            let pts = vec![
                                vec2(t_min, z0),
                                vec2(t_max, z0),
                                vec2(t_max, z1),
                                vec2(t_min, z1),
                            ];
                            proj_elements.push((d, pts, theme.col_fill, theme.proj_line, 1.0));
                        }
                    }
                }
            }
            Element::Beam(b) => {
                let fp = b.footprint();
                let is_cut = !polygon::line_polygon_intervals(s1, u_dir, &fp).is_empty();
                if !is_cut {
                    let mid = (b.start + b.end) * 0.5;
                    let d = (mid - s1).dot(v_dir);
                    if d > 0.0 && d < max_depth {
                        let t0 = (b.start - s1).dot(u_dir);
                        let t1 = (b.end - s1).dot(u_dir);
                        let t_min = t0.min(t1);
                        let t_max = t0.max(t1);
                        let base_z = doc.level_elevation(b.level);
                        let top_z = b.top_level.map(|l| doc.level_elevation(l));
                        let (z0, z1) = b.z_bounds(base_z, top_z);
                        if z1 > z0 + 1.0 && t_max > t_min + 1.0 {
                            let pts = vec![
                                vec2(t_min, z0),
                                vec2(t_max, z0),
                                vec2(t_max, z1),
                                vec2(t_min, z1),
                            ];
                            proj_elements.push((d, pts, theme.wall_fill, theme.proj_line, 1.0));
                        }
                    }
                }
            }
            Element::Footing(f) => {
                let fp = f.footprint();
                let is_cut = !polygon::line_polygon_intervals(s1, u_dir, &fp).is_empty();
                if !is_cut {
                    let d = (f.location - s1).dot(v_dir);
                    if d > 0.0 && d < max_depth {
                        let ts: Vec<f64> = fp.iter().map(|&p| (p - s1).dot(u_dir)).collect();
                        let t_min = ts.iter().copied().fold(f64::INFINITY, f64::min);
                        let t_max = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        let base_z = doc.level_elevation(f.base_level);
                        let top_z = f.top_level.map(|l| doc.level_elevation(l));
                        let (z0, z1) = f.z_bounds(base_z, top_z);
                        if z1 > z0 + 1.0 && t_max > t_min + 1.0 {
                            let pts = vec![
                                vec2(t_min, z0),
                                vec2(t_max, z0),
                                vec2(t_max, z1),
                                vec2(t_min, z1),
                            ];
                            proj_elements.push((d, pts, theme.col_fill, theme.proj_line, 1.0));
                        }
                    }
                }
            }
            Element::Wall(w) => {
                let fp = w.footprint();
                let is_cut = !polygon::line_polygon_intervals(s1, u_dir, &fp).is_empty();
                if !is_cut {
                    let mid = (w.start + w.end) * 0.5;
                    let d = (mid - s1).dot(v_dir);
                    if d > 0.0 && d < max_depth {
                        let t0 = (w.start - s1).dot(u_dir);
                        let t1 = (w.end - s1).dot(u_dir);
                        let t_min = t0.min(t1);
                        let t_max = t0.max(t1);
                        let base_z = doc.level_elevation(w.base_level);
                        let top_z = w.top_level.map(|l| doc.level_elevation(l));
                        let (z0, z1) = w.z_bounds(base_z, top_z);
                        if z1 > z0 + 1.0 && t_max > t_min + 1.0 {
                            let pts = vec![
                                vec2(t_min, z0),
                                vec2(t_max, z0),
                                vec2(t_max, z1),
                                vec2(t_min, z1),
                            ];
                            proj_elements.push((d, pts, theme.wall_fill, theme.proj_line, 1.0));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Sort projected elements back-to-front (largest d drawn first)
    proj_elements.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_d, pts, fill, stroke, stroke_w) in proj_elements {
        scene.add(RenderPrimitive::Polygon {
            points: pts,
            fill_color: fill,
            stroke_color: Some(stroke),
            stroke_width: stroke_w,
        });
    }

    // 2. Cut through all model solids (directly on the cut plane d = 0)
    for &id in doc.elements.keys() {
        let solids = doc.element_solids(id);
        let el = doc.elements.get(&id).unwrap();

        for prism in solids {
            // Find intersection intervals of the infinite line (s1 + t * u_dir) with prism profile
            let intervals = polygon::line_polygon_intervals(s1, u_dir, &prism.profile);

            for (t0, t1) in intervals {
                // Ensure interval is within our section segment or extended segment
                let t_start = t0.max(-1000.0);
                let t_end = t1.min(line_len + 1000.0);
                if t_end - t_start > 1.0 {
                    let (z0, z1) = (prism.z_min, prism.z_max);
                    let pts = vec![
                        vec2(t_start, z0),
                        vec2(t_end, z0),
                        vec2(t_end, z1),
                        vec2(t_start, z1),
                    ];

                    let fill = if el.is_structural() {
                        theme.col_fill
                    } else {
                        theme.wall_fill
                    };

                    scene.add(RenderPrimitive::Polygon {
                        points: pts,
                        fill_color: fill,
                        stroke_color: Some(theme.cut_line),
                        stroke_width: 2.0,
                    });
                }
            }
        }
    }

    // 3. Level Datum lines across section width
    let span_u = (-2000.0, line_len + 2000.0);
    for level in doc.levels.values() {
        let z = level.elevation;
        let p_start = vec2(span_u.0, z);
        let p_end = vec2(span_u.1, z);

        scene.add(RenderPrimitive::Line {
            start: p_start,
            end: p_end,
            color: theme.level_line,
            width: 1.0,
            dashed: true,
        });

        // Bubble at right end
        let bubble_center = vec2(span_u.1 + 400.0, z);
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

    // 4. Ground Line at Z = 0
    scene.add(RenderPrimitive::Line {
        start: vec2(span_u.0 - 1000.0, 0.0),
        end: vec2(span_u.1 + 1000.0, 0.0),
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
