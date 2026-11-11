//! Reflected Ceiling Plan (RCP) projection engine.

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_geom::{Aabb2, Vec2};
use plinth_model::{Document, Element, ViewDef};

pub fn render_ceiling_plan(doc: &Document, view: &ViewDef, theme: &ColorTheme) -> RenderScene {
    let mut scene = RenderScene::new();

    let level_id = match view.level_id {
        Some(lid) => lid,
        None => return scene,
    };
    let level_z = doc.level_elevation(level_id);
    let cut_z = level_z + view.cut_offset;

    // 1. Render Ceiling grids and outlines
    for el in doc.elements.values() {
        if let Element::Ceiling(c) = el {
            if c.level == level_id && c.boundary.len() >= 3 {
                // Ceiling boundary fill
                scene.add(RenderPrimitive::Polygon {
                    points: c.boundary.clone(),
                    fill_color: theme.slab_fill,
                    stroke_color: Some(theme.proj_line),
                    stroke_width: 1.2,
                });

                // Generate ceiling tile grid (e.g. 600x600 mm)
                let bbox = Aabb2::from_points(c.boundary.clone());
                let gx = if c.grid_spacing_x > 100.0 { c.grid_spacing_x } else { 600.0 };
                let gy = if c.grid_spacing_y > 100.0 { c.grid_spacing_y } else { 600.0 };

                let mut x = bbox.min.x + (gx - (bbox.min.x % gx));
                while x < bbox.max.x {
                    scene.add(RenderPrimitive::Line {
                        start: Vec2::new(x, bbox.min.y),
                        end: Vec2::new(x, bbox.max.y),
                        color: theme.hidden_line,
                        width: 0.8,
                        dashed: false,
                    });
                    x += gx;
                }

                let mut y = bbox.min.y + (gy - (bbox.min.y % gy));
                while y < bbox.max.y {
                    scene.add(RenderPrimitive::Line {
                        start: Vec2::new(bbox.min.x, y),
                        end: Vec2::new(bbox.max.x, y),
                        color: theme.hidden_line,
                        width: 0.8,
                        dashed: false,
                    });
                    y += gy;
                }
            }
        }
    }

    // 2. Render Walls cut at ceiling level
    for el in doc.elements.values() {
        if let Element::Wall(w) = el {
            let (base_z, top_z) = w.z_bounds(
                doc.level_elevation(w.base_level),
                w.top_level.map(|tl| doc.level_elevation(tl)),
            );
            if cut_z >= base_z && cut_z <= top_z {
                scene.add(RenderPrimitive::Polygon {
                    points: w.footprint(),
                    fill_color: theme.wall_fill,
                    stroke_color: Some(theme.cut_line),
                    stroke_width: 2.0,
                });
            }
        }
    }

    // 3. Render Columns
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

    // 4. Render Grids
    for grid in doc.grids.values() {
        scene.add(RenderPrimitive::Line {
            start: grid.start,
            end: grid.end,
            color: theme.grid_line,
            width: 1.0,
            dashed: true,
        });
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

    // 5. Annotations (Dimensions, Text Notes, Leaders)
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

    scene
}
