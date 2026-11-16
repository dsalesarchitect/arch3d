//! 2D Architectural Annotation Projection & Rendering
//! (Linear Dimensions, Circular Dimensions, Angular Dimensions, Text Notes, Leaders)

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_geom::Vec2;
use plinth_model::{Dimension, DimensionKind, Leader, TextNote, TickMark};

/// Emits an architectural tick mark or arrowhead at `pt` oriented along `dir`.
pub fn emit_tick_mark(
    scene: &mut RenderScene,
    pt: Vec2,
    dir: Vec2,
    tick: TickMark,
    color: [u8; 4],
    scale: f64,
) {
    let s = scale.max(0.2);
    let d = if dir.length() > 1e-4 {
        dir.normalized()
    } else {
        Vec2::new(1.0, 0.0)
    };
    let norm = d.perp();

    match tick {
        TickMark::Tick45 => {
            let v = (d + norm).normalized() * (s * 140.0);
            scene.add(RenderPrimitive::Line {
                start: pt - v,
                end: pt + v,
                color,
                width: 1.6,
                dashed: false,
            });
        }
        TickMark::Arrow => {
            let tip = pt;
            let base = pt + d * (s * 160.0);
            let perp = norm * (s * 55.0);
            scene.add(RenderPrimitive::Polygon {
                points: vec![tip, base + perp, base - perp],
                fill_color: color,
                stroke_color: None,
                stroke_width: 0.0,
            });
        }
        TickMark::Dot => {
            scene.add(RenderPrimitive::Circle {
                center: pt,
                radius: (s * 55.0) as f32,
                fill_color: Some(color),
                stroke_color: color,
                stroke_width: 1.0,
            });
        }
        TickMark::OpenArrow => {
            let base = pt + d * (s * 160.0);
            let perp = norm * (s * 60.0);
            scene.add(RenderPrimitive::Line {
                start: base + perp,
                end: pt,
                color,
                width: 1.4,
                dashed: false,
            });
            scene.add(RenderPrimitive::Line {
                start: base - perp,
                end: pt,
                color,
                width: 1.4,
                dashed: false,
            });
        }
        TickMark::None => {}
    }
}

/// Renders a Dimension (Linear, Circular, or Angular) into the 2D scene scaled to the view drawing scale.
pub fn render_dimension(scene: &mut RenderScene, d: &Dimension, theme: &ColorTheme, view_scale: u32) {
    let scale_ratio = (view_scale.max(1) as f64) / 100.0;
    let color = if d.color != [0, 0, 0, 0] {
        d.color
    } else {
        theme.dim_line
    };
    let base_font_size = if d.font_size > 4.0 {
        d.font_size
    } else {
        12.0
    };
    let font_size = base_font_size * (scale_ratio as f32);

    match d.kind {
        DimensionKind::Linear => {
            let diff = d.p2 - d.p1;
            if diff.length() < 1e-3 {
                return;
            }
            let dir = diff.normalized();
            let norm = dir.perp();
            let p1_dim = d.p1 + norm * d.offset;
            let p2_dim = d.p2 + norm * d.offset;

            // Witness lines from reference geometry to dimension line
            let witness_ext = norm * (150.0 * scale_ratio);
            scene.add(RenderPrimitive::Line {
                start: d.p1,
                end: p1_dim + witness_ext,
                color,
                width: 1.0,
                dashed: false,
            });
            scene.add(RenderPrimitive::Line {
                start: d.p2,
                end: p2_dim + witness_ext,
                color,
                width: 1.0,
                dashed: false,
            });

            // Dimension line
            scene.add(RenderPrimitive::Line {
                start: p1_dim,
                end: p2_dim,
                color,
                width: 1.0,
                dashed: false,
            });

            // Tick marks at p1 and p2
            emit_tick_mark(scene, p1_dim, dir, d.tick_mark, color, scale_ratio);
            emit_tick_mark(scene, p2_dim, -dir, d.tick_mark, color, scale_ratio);

            // Dimension text
            let mid = (p1_dim + p2_dim) * 0.5 + norm * (180.0 * scale_ratio);
            scene.add(RenderPrimitive::Text {
                pos: mid,
                text: d.formatted_text(),
                color,
                size: font_size,
                align_center: true,
            });
        }
        DimensionKind::Circular => {
            let center = d.p1;
            let rim_pt = d.p2;
            let diff = rim_pt - center;
            let r = diff.length();
            if r < 1e-3 {
                return;
            }
            let dir = diff.normalized();
            let end_pt = if d.offset.abs() > 10.0 {
                rim_pt + dir * d.offset
            } else {
                rim_pt
            };

            // Ray line from center to rim/leader end
            scene.add(RenderPrimitive::Line {
                start: center,
                end: end_pt,
                color,
                width: 1.0,
                dashed: false,
            });

            // Center mark cross
            let cm_size = 120.0 * scale_ratio;
            scene.add(RenderPrimitive::Line {
                start: center - Vec2::new(cm_size, 0.0),
                end: center + Vec2::new(cm_size, 0.0),
                color,
                width: 1.0,
                dashed: false,
            });
            scene.add(RenderPrimitive::Line {
                start: center - Vec2::new(0.0, cm_size),
                end: center + Vec2::new(0.0, cm_size),
                color,
                width: 1.0,
                dashed: false,
            });

            // Tick at rim point
            emit_tick_mark(scene, rim_pt, -dir, d.tick_mark, color, scale_ratio);

            // Horizontal landing shelf for dimension text
            let shelf_len = if dir.x >= 0.0 { 400.0 * scale_ratio } else { -400.0 * scale_ratio };
            let shelf_end = end_pt + Vec2::new(shelf_len, 0.0);
            scene.add(RenderPrimitive::Line {
                start: end_pt,
                end: shelf_end,
                color,
                width: 1.0,
                dashed: false,
            });

            // Dimension text above shelf
            let text_pos = (end_pt + shelf_end) * 0.5 + Vec2::new(0.0, 160.0 * scale_ratio);
            scene.add(RenderPrimitive::Text {
                pos: text_pos,
                text: d.formatted_text(),
                color,
                size: font_size,
                align_center: true,
            });
        }
        DimensionKind::Angular => {
            let apex = d.p1;
            let p1 = d.p2;
            let p2 = d.p3.unwrap_or(d.p2);
            let v1 = p1 - apex;
            let v2 = p2 - apex;
            if v1.length() < 1e-3 || v2.length() < 1e-3 {
                return;
            }
            let mut a1 = v1.y.atan2(v1.x);
            let mut a2 = v2.y.atan2(v2.x);

            // Compute CCW sweep
            while a2 < a1 {
                a2 += std::f64::consts::TAU;
            }
            if (a2 - a1) > std::f64::consts::PI {
                // Take shorter interior angle
                std::mem::swap(&mut a1, &mut a2);
                while a2 < a1 {
                    a2 += std::f64::consts::TAU;
                }
            }

            let arc_r = if d.offset > 50.0 { d.offset } else { 800.0 * scale_ratio };

            // Leg reference lines
            let leg1_dir = Vec2::new(a1.cos(), a1.sin());
            let leg2_dir = Vec2::new(a2.cos(), a2.sin());
            let arc_p1 = apex + leg1_dir * arc_r;
            let arc_p2 = apex + leg2_dir * arc_r;

            scene.add(RenderPrimitive::Line {
                start: apex,
                end: arc_p1 + leg1_dir * (120.0 * scale_ratio),
                color,
                width: 0.8,
                dashed: true,
            });
            scene.add(RenderPrimitive::Line {
                start: apex,
                end: arc_p2 + leg2_dir * (120.0 * scale_ratio),
                color,
                width: 0.8,
                dashed: true,
            });

            // Arc line
            scene.add(RenderPrimitive::Arc {
                center: apex,
                radius: arc_r as f32,
                start_angle: a1 as f32,
                end_angle: a2 as f32,
                color,
                width: 1.0,
            });

            // Ticks at arc ends
            emit_tick_mark(scene, arc_p1, leg1_dir.perp(), d.tick_mark, color, scale_ratio);
            emit_tick_mark(scene, arc_p2, -leg2_dir.perp(), d.tick_mark, color, scale_ratio);

            // Text at arc midpoint
            let mid_a = (a1 + a2) * 0.5;
            let mid_dir = Vec2::new(mid_a.cos(), mid_a.sin());
            let mid_pos = apex + mid_dir * (arc_r + 200.0 * scale_ratio);

            scene.add(RenderPrimitive::Text {
                pos: mid_pos,
                text: d.formatted_text(),
                color,
                size: font_size,
                align_center: true,
            });
        }
    }
}

/// Renders a Text Note into the 2D scene scaled to the view drawing scale.
pub fn render_text_note(scene: &mut RenderScene, t: &TextNote, theme: &ColorTheme, view_scale: u32) {
    let scale_ratio = (view_scale.max(1) as f64) / 100.0;
    let color = if t.color != [0, 0, 0, 0] {
        t.color
    } else {
        theme.text_color
    };
    let base_font_size = if t.font_size > 4.0 {
        t.font_size
    } else {
        14.0
    };
    let font_size = base_font_size * (scale_ratio as f32);
    scene.add(RenderPrimitive::Text {
        pos: t.pos,
        text: t.text.clone(),
        color,
        size: font_size,
        align_center: false,
    });
}

/// Renders a Leader note with pointing arrowhead and landing shelf into the 2D scene scaled to the view drawing scale.
pub fn render_leader(scene: &mut RenderScene, ld: &Leader, theme: &ColorTheme, view_scale: u32) {
    let scale_ratio = (view_scale.max(1) as f64) / 100.0;
    let color = if ld.color != [0, 0, 0, 0] {
        ld.color
    } else {
        theme.dim_line
    };
    let base_font_size = if ld.font_size > 4.0 {
        ld.font_size
    } else {
        12.0
    };
    let font_size = base_font_size * (scale_ratio as f32);

    // Pointer line from target to shoulder
    scene.add(RenderPrimitive::Line {
        start: ld.target_pt,
        end: ld.shoulder_pt,
        color,
        width: 1.0,
        dashed: false,
    });

    // Arrowhead / tick at target pointing point
    let pointer_dir = (ld.shoulder_pt - ld.target_pt).normalized();
    emit_tick_mark(scene, ld.target_pt, pointer_dir, ld.tick_mark, color, scale_ratio);

    // Horizontal landing shelf from shoulder to text endpoint
    let shelf_len = if ld.text_pt.x >= ld.shoulder_pt.x { 400.0 * scale_ratio } else { -400.0 * scale_ratio };
    let text_end = ld.shoulder_pt + Vec2::new(shelf_len, 0.0);
    scene.add(RenderPrimitive::Line {
        start: ld.shoulder_pt,
        end: text_end,
        color,
        width: 1.0,
        dashed: false,
    });

    // Text note placed above the landing shelf
    let text_pos = (ld.shoulder_pt + text_end) * 0.5 + Vec2::new(0.0, 160.0 * scale_ratio);
    scene.add(RenderPrimitive::Text {
        pos: text_pos,
        text: ld.text.clone(),
        color,
        size: font_size,
        align_center: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_model::{Dimension, FontStyle, Leader, TextNote, TickMark, ViewId};

    #[test]
    fn test_render_linear_dimension() {
        let mut scene = RenderScene::new();
        let theme = ColorTheme::light();
        let mut dim = Dimension::new_linear(
            ViewId::new(),
            Vec2::new(0.0, 0.0),
            Vec2::new(3000.0, 0.0),
            250.0,
            TickMark::Tick45,
            12.0,
            FontStyle::Regular,
            [0, 0, 0, 255],
        );
        dim.prefix = None;
        dim.suffix = Some(" mm".into());

        render_dimension(&mut scene, &dim, &theme, 100);
        assert!(!scene.primitives.is_empty(), "Scene should contain linear dimension primitives");
        let has_text = scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Text { text, .. } if text.contains("3000")));
        assert!(has_text, "Linear dimension should generate text label with measurement");
    }

    #[test]
    fn test_render_circular_dimension() {
        let mut scene = RenderScene::new();
        let theme = ColorTheme::light();
        let dim = Dimension::new_circular(
            ViewId::new(),
            Vec2::new(1000.0, 1000.0),
            Vec2::new(1500.0, 1000.0),
            300.0,
            false,
            TickMark::Arrow,
            11.0,
            FontStyle::Bold,
            [0, 100, 220, 255],
        );

        render_dimension(&mut scene, &dim, &theme, 100);
        let has_radial_text = scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Text { text, .. } if text.starts_with("R 500")));
        assert!(has_radial_text, "Circular dimension should render 'R 500' radius note");
    }

    #[test]
    fn test_render_angular_dimension() {
        let mut scene = RenderScene::new();
        let theme = ColorTheme::light();
        let dim = Dimension::new_angular(
            ViewId::new(),
            Vec2::new(0.0, 0.0),
            Vec2::new(1000.0, 0.0),
            Vec2::new(0.0, 1000.0),
            600.0,
            TickMark::Dot,
            12.0,
            FontStyle::Regular,
            [220, 40, 40, 255],
        );

        render_dimension(&mut scene, &dim, &theme, 100);
        let has_angle_text = scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Text { text, .. } if text.contains("90")));
        assert!(has_angle_text, "Angular dimension should render 90° text label");
    }

    #[test]
    fn test_render_text_note_and_leader() {
        let mut scene = RenderScene::new();
        let theme = ColorTheme::light();
        let note = TextNote::new(ViewId::new(), Vec2::new(500.0, 500.0), "GLAZED CURTAIN WALL");
        render_text_note(&mut scene, &note, &theme, 100);
        assert!(scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Text { text, .. } if text == "GLAZED CURTAIN WALL")));

        let mut leader_scene = RenderScene::new();
        let leader = Leader::new(
            ViewId::new(),
            Vec2::new(100.0, 100.0),
            Vec2::new(300.0, 300.0),
            Vec2::new(500.0, 300.0),
            "BEAM MK-1",
            TickMark::OpenArrow,
            11.0,
            FontStyle::Bold,
            [0, 0, 0, 255],
        );
        render_leader(&mut leader_scene, &leader, &theme, 100);
        assert!(leader_scene.primitives.iter().any(|p| matches!(p, RenderPrimitive::Text { text, .. } if text == "BEAM MK-1")));
    }
}
