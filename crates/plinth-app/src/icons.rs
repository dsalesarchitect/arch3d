//! Vector-rendered professional architectural toolbar and ribbon icons.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IconKind {
    Wall,
    Column,
    Beam,
    Slab,
    Ceiling,
    Door,
    Window,
    Grid,
    Dimension,
    DetailLine,
    Select,
    Move,
    Rotate,
    Mirror,
    Delete,
    IfcExport,
    IfcImport,
    ThreeD,
    RectSurface,
    CircleSurface,
    PolygonSurface,
    PushPull,
    BoxPrimitive,
    CylinderPrimitive,
    SpherePrimitive,
    Settings,
    Undo,
    Redo,
    Save,
    SaveAs,
    Import,
    ImportSkp,
    ImportDwg,
    Export,
    ExportPdf,
    ExportGlb,
    RenderMuTracer,
    PluginManager,
    BlenderLiveSync,
    NewProject,
    Arch3dApp,
    Roof,
    Fascia,
    Gutter,
    Stair,
    Ramp,
    Railing,
    SingleViewport,
    SplitHorizontal,
    SplitVertical,
    SplitTriple,
    SplitQuad,
    Sheet,
    NewSheet,
    Footing,
    LevelManager,
    GridManager,
    CopyLevel,
    PasteLevel,
    AutoColumnsOnGrids,
    AutoFootingsOnColumns,
    AutoBeamsOnGrids,
    AutoSlabsOnBays,
    CurtainWall,
    OpenProject,
    ZoomExtents,
    Snapping,
    CircularDimension,
    AngularDimension,
    TextAnnotation,
    Leader,
    BuildingPad,
    Toposolid,
    TerrainContours,
    LotBoundary,
    LotBearing,
    Road,
    Sidewalk,
    Pathwalk,
    ProjectTree,
    Blocks,
    SearchFilter,
    FloorPlanView,
    SectionView,
    ElevationView,
}

pub fn draw_icon(painter: &Painter, rect: Rect, kind: IconKind, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    let thin_stroke = Stroke::new(1.0, color);
    let c = rect.center();
    let s = rect.size();
    let (w, h) = (s.x, s.y);

    match kind {
        IconKind::Wall => {
            // Isometric or section wall brickwork pattern
            let r = Rect::from_center_size(c, Vec2::new(w * 0.75, h * 0.6));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            // Internal brick hatch
            painter.line_segment([Pos2::new(r.min.x, c.y), Pos2::new(r.max.x, c.y)], thin_stroke);
            painter.line_segment(
                [
                    Pos2::new(r.min.x + r.width() * 0.33, r.min.y),
                    Pos2::new(r.min.x + r.width() * 0.33, c.y),
                ],
                thin_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(r.min.x + r.width() * 0.66, c.y),
                    Pos2::new(r.min.x + r.width() * 0.66, r.max.y),
                ],
                thin_stroke,
            );
        }
        IconKind::Column => {
            // Square column with centerline cross
            let r = Rect::from_center_size(c, Vec2::new(w * 0.55, h * 0.55));
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.2));
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(r.min.x, r.min.y), Pos2::new(r.max.x, r.max.y)], thin_stroke);
            painter.line_segment([Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.min.y)], thin_stroke);
        }
        IconKind::Beam => {
            // I-beam profile
            let hw = w * 0.3;
            let hh = h * 0.35;
            // Top flange
            painter.line_segment([Pos2::new(c.x - hw, c.y - hh), Pos2::new(c.x + hw, c.y - hh)], stroke);
            // Bottom flange
            painter.line_segment([Pos2::new(c.x - hw, c.y + hh), Pos2::new(c.x + hw, c.y + hh)], stroke);
            // Web
            painter.line_segment([Pos2::new(c.x, c.y - hh), Pos2::new(c.x, c.y + hh)], stroke);
        }
        IconKind::Slab => {
            // Horizontal slab profile with concrete speckle
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.35));
            painter.rect_filled(r, 1.0, color.gamma_multiply(0.2));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
        }
        IconKind::Ceiling => {
            // Suspended ceiling grid
            let r = Rect::from_center_size(c, Vec2::new(w * 0.75, h * 0.45));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], thin_stroke);
            painter.line_segment([Pos2::new(r.min.x, c.y), Pos2::new(r.max.x, c.y)], thin_stroke);
        }
        IconKind::Door => {
            // Door opening with 90° swing arc
            let r = Rect::from_center_size(c, Vec2::new(w * 0.65, h * 0.65));
            // Wall jambs
            painter.line_segment([r.left_bottom(), Pos2::new(r.left_bottom().x, r.left_bottom().y - 3.0)], stroke);
            painter.line_segment([r.right_bottom(), Pos2::new(r.right_bottom().x, r.right_bottom().y - 3.0)], stroke);
            // Door leaf swung 90 deg
            painter.line_segment([r.left_bottom(), r.left_top()], stroke);
            // Swing arc
            let radius = r.width();
            let mut pts = Vec::new();
            for i in 0..=8 {
                let a = std::f32::consts::FRAC_PI_2 * (1.0 - (i as f32) / 8.0);
                pts.push(Pos2::new(r.left_bottom().x + radius * a.cos(), r.left_bottom().y - radius * a.sin()));
            }
            for i in 0..pts.len() - 1 {
                painter.line_segment([pts[i], pts[i + 1]], thin_stroke);
            }
        }
        IconKind::Window => {
            // Window in wall: outer sill + 2 glazing lines
            let r = Rect::from_center_size(c, Vec2::new(w * 0.7, h * 0.55));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], stroke);
            painter.line_segment([Pos2::new(r.min.x - 2.0, r.max.y), Pos2::new(r.max.x + 2.0, r.max.y)], thin_stroke);
        }
        IconKind::CurtainWall => {
            // Glazed curtain wall / storefront grid facade with translucent glass and mullion grid
            let r = Rect::from_center_size(c, Vec2::new(w * 0.76, h * 0.65));
            // Translucent glass fill
            painter.rect_filled(r, 1.0, Color32::from_rgba_unmultiplied(100, 180, 240, 75));
            // Outer frame
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            // Intermediate vertical mullions (dividing into 3 bays)
            let x1 = r.min.x + r.width() * 0.333;
            let x2 = r.min.x + r.width() * 0.666;
            painter.line_segment([Pos2::new(x1, r.min.y), Pos2::new(x1, r.max.y)], stroke);
            painter.line_segment([Pos2::new(x2, r.min.y), Pos2::new(x2, r.max.y)], stroke);
            // Intermediate horizontal transom
            let y_mid = r.min.y + r.height() * 0.45;
            painter.line_segment([Pos2::new(r.min.x, y_mid), Pos2::new(r.max.x, y_mid)], stroke);
            // Base sill / wall cut accent line
            let sill_stroke = Stroke::new(2.0, Color32::from_rgb(255, 120, 0));
            painter.line_segment(
                [Pos2::new(r.min.x - 2.0, r.max.y + 1.5), Pos2::new(r.max.x + 2.0, r.max.y + 1.5)],
                sill_stroke,
            );
        }
        IconKind::Grid => {
            // Grid line with bubble circle "1"
            let r_bub = 5.0;
            let top_bub = Pos2::new(c.x, rect.min.y + r_bub + 1.0);
            painter.circle_stroke(top_bub, r_bub, stroke);
            painter.line_segment([Pos2::new(c.x, top_bub.y + r_bub), Pos2::new(c.x, rect.max.y - 1.0)], thin_stroke);
        }
        IconKind::Dimension => {
            // Dimension line with witness ticks
            let hw = w * 0.35;
            let p1 = Pos2::new(c.x - hw, c.y);
            let p2 = Pos2::new(c.x + hw, c.y);
            painter.line_segment([p1, p2], stroke);
            // Slanted ticks
            painter.line_segment([Pos2::new(p1.x - 3.0, p1.y + 4.0), Pos2::new(p1.x + 3.0, p1.y - 4.0)], stroke);
            painter.line_segment([Pos2::new(p2.x - 3.0, p2.y + 4.0), Pos2::new(p2.x + 3.0, p2.y - 4.0)], stroke);
        }
        IconKind::DetailLine => {
            // Slanted pencil / line
            painter.line_segment([Pos2::new(c.x - w * 0.3, c.y + h * 0.3), Pos2::new(c.x + w * 0.3, c.y - h * 0.3)], stroke);
        }
        IconKind::Select => {
            // Arrow cursor
            let p0 = Pos2::new(c.x - 6.0, c.y - 8.0);
            let p1 = Pos2::new(c.x - 6.0, c.y + 7.0);
            let p2 = Pos2::new(c.x - 1.0, c.y + 3.0);
            let p3 = Pos2::new(c.x + 4.0, c.y + 8.0);
            let p4 = Pos2::new(c.x + 6.0, c.y + 7.0);
            let p5 = Pos2::new(c.x + 1.0, c.y + 2.0);
            let p6 = Pos2::new(c.x + 6.0, c.y + 1.0);
            painter.line_segment([p0, p1], stroke);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p5], stroke);
            painter.line_segment([p5, p6], stroke);
            painter.line_segment([p6, p0], stroke);
        }
        IconKind::Move => {
            // 4-way arrow
            let d = 7.0;
            painter.line_segment([Pos2::new(c.x - d, c.y), Pos2::new(c.x + d, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - d), Pos2::new(c.x, c.y + d)], stroke);
        }
        IconKind::Rotate => {
            // Circular arrow
            let r = 6.0;
            painter.circle_stroke(c, r, stroke);
        }
        IconKind::Mirror => {
            // Dashed centerline + two triangles
            let d = 6.0;
            painter.line_segment([Pos2::new(c.x, c.y - d), Pos2::new(c.x, c.y + d)], thin_stroke);
            painter.line_segment([Pos2::new(c.x - 2.0, c.y - 3.0), Pos2::new(c.x - 6.0, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x + 2.0, c.y - 3.0), Pos2::new(c.x + 6.0, c.y)], stroke);
        }
        IconKind::Delete => {
            // Red / clear cross
            let d = 6.0;
            painter.line_segment([Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)], Stroke::new(2.0, Color32::from_rgb(220, 60, 60)));
            painter.line_segment([Pos2::new(c.x - d, c.y + d), Pos2::new(c.x + d, c.y - d)], Stroke::new(2.0, Color32::from_rgb(220, 60, 60)));
        }
        IconKind::IfcExport | IconKind::IfcImport => {
            // Cube with IFC label or arrow
            let r = Rect::from_center_size(c, Vec2::new(w * 0.7, h * 0.6));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
            painter.text(c, egui::Align2::CENTER_CENTER, "IFC", egui::FontId::proportional(9.0), color);
        }
        IconKind::ThreeD => {
            // Isometric cube
            let s = 6.0;
            let p_top = Pos2::new(c.x, c.y - s * 1.2);
            let p_mid = c;
            let p_bot = Pos2::new(c.x, c.y + s * 1.2);
            let p_l = Pos2::new(c.x - s * 1.2, c.y - s * 0.3);
            let p_r = Pos2::new(c.x + s * 1.2, c.y - s * 0.3);
            let p_bl = Pos2::new(c.x - s * 1.2, c.y + s * 0.7);
            let p_br = Pos2::new(c.x + s * 1.2, c.y + s * 0.7);

            painter.line_segment([p_top, p_r], stroke);
            painter.line_segment([p_r, p_br], stroke);
            painter.line_segment([p_br, p_bot], stroke);
            painter.line_segment([p_bot, p_bl], stroke);
            painter.line_segment([p_bl, p_l], stroke);
            painter.line_segment([p_l, p_top], stroke);
            painter.line_segment([p_l, p_mid], stroke);
            painter.line_segment([p_r, p_mid], stroke);
            painter.line_segment([p_bot, p_mid], stroke);
        }
        IconKind::RectSurface => {
            // Planar rectangle surface with shaded interior
            let r = Rect::from_center_size(c, Vec2::new(w * 0.7, h * 0.5));
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.25));
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
            painter.circle_filled(r.min, 1.5, color);
            painter.circle_filled(r.max, 1.5, color);
        }
        IconKind::CircleSurface => {
            // Planar circle surface with shaded interior
            let rad = w.min(h) * 0.32;
            painter.circle_filled(c, rad, color.gamma_multiply(0.25));
            painter.circle_stroke(c, rad, stroke);
            painter.circle_filled(c, 1.5, color);
        }
        IconKind::PolygonSurface => {
            // 5-sided polygon surface
            let rad = w.min(h) * 0.32;
            let mut pts = Vec::with_capacity(5);
            for i in 0..5 {
                let a = (i as f32 / 5.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                pts.push(Pos2::new(c.x + rad * a.cos(), c.y + rad * a.sin()));
            }
            let n = pts.len();
            for i in 0..n {
                painter.line_segment([pts[i], pts[(i + 1) % n]], stroke);
            }
        }
        IconKind::PushPull => {
            // SketchUp-style Push/Pull icon: 3D prism base with an upward extrusion arrow
            let bx = w * 0.32;
            let by = h * 0.22;
            let base_top = c.y + 1.0;
            let base_bot = c.y + by + 2.0;
            // Base rectangle
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(c.x - bx, base_top), Pos2::new(c.x + bx, base_bot)),
                0.0,
                color.gamma_multiply(0.2),
            );
            painter.rect_stroke(
                Rect::from_min_max(Pos2::new(c.x - bx, base_top), Pos2::new(c.x + bx, base_bot)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            // Upward arrow lifting top surface
            let arrow_tip = Pos2::new(c.x, c.y - h * 0.35);
            let arrow_base = Pos2::new(c.x, base_top);
            let arrow_stroke = Stroke::new(2.2, Color32::from_rgb(255, 120, 0)); // Accent orange
            painter.line_segment([arrow_base, arrow_tip], arrow_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x - 3.5, arrow_tip.y + 4.5)], arrow_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x + 3.5, arrow_tip.y + 4.5)], arrow_stroke);
        }
        IconKind::BoxPrimitive => {
            // 3D Isometric box
            let s = 5.5;
            let top = Pos2::new(c.x, c.y - s * 1.1);
            let mid = c;
            let bot = Pos2::new(c.x, c.y + s * 1.1);
            let l = Pos2::new(c.x - s * 1.1, c.y - s * 0.25);
            let r = Pos2::new(c.x + s * 1.1, c.y - s * 0.25);
            let bl = Pos2::new(c.x - s * 1.1, c.y + s * 0.65);
            let br = Pos2::new(c.x + s * 1.1, c.y + s * 0.65);

            painter.line_segment([top, r], stroke);
            painter.line_segment([r, br], stroke);
            painter.line_segment([br, bot], stroke);
            painter.line_segment([bot, bl], stroke);
            painter.line_segment([bl, l], stroke);
            painter.line_segment([l, top], stroke);
            painter.line_segment([l, mid], thin_stroke);
            painter.line_segment([r, mid], thin_stroke);
            painter.line_segment([bot, mid], thin_stroke);
        }
        IconKind::CylinderPrimitive => {
            // 3D Cylinder
            let cw = w * 0.28;
            let ch = cw * 0.45;
            let top_y = c.y - h * 0.25;
            let bot_y = c.y + h * 0.25;
            let mut top_pts = Vec::with_capacity(17);
            let mut bot_pts = Vec::with_capacity(17);
            for i in 0..=16 {
                let theta = (i as f32 / 16.0) * std::f32::consts::TAU;
                top_pts.push(Pos2::new(c.x + cw * theta.cos(), top_y + ch * theta.sin()));
                bot_pts.push(Pos2::new(c.x + cw * theta.cos(), bot_y + ch * theta.sin()));
            }
            painter.add(egui::Shape::line(top_pts, stroke));
            painter.add(egui::Shape::line(bot_pts, stroke));
            painter.line_segment([Pos2::new(c.x - cw, top_y), Pos2::new(c.x - cw, bot_y)], stroke);
            painter.line_segment([Pos2::new(c.x + cw, top_y), Pos2::new(c.x + cw, bot_y)], stroke);
        }
        IconKind::SpherePrimitive => {
            // 3D Sphere with equator
            let rad = w.min(h) * 0.32;
            painter.circle_stroke(c, rad, stroke);
            let mut eq_pts = Vec::with_capacity(17);
            for i in 0..=16 {
                let theta = (i as f32 / 16.0) * std::f32::consts::TAU;
                eq_pts.push(Pos2::new(c.x + rad * theta.cos(), c.y + rad * 0.35 * theta.sin()));
            }
            painter.add(egui::Shape::line(eq_pts, thin_stroke));
        }
        IconKind::Settings => {
            // Procedural vector gear/cog icon
            let r_outer = w.min(h) * 0.34;
            let r_inner = r_outer * 0.42;

            // Translucent gear disc body
            painter.circle_filled(c, r_outer, color.gamma_multiply(0.2));

            // Central axle hole
            painter.circle_stroke(c, r_inner, stroke);

            // 6 radial gear teeth
            let teeth = 6;
            for i in 0..teeth {
                let theta = (i as f32 / teeth as f32) * std::f32::consts::TAU;
                let (sin, cos) = theta.sin_cos();
                let p1 = Pos2::new(c.x + r_outer * 0.65 * cos, c.y + r_outer * 0.65 * sin);
                let p2 = Pos2::new(c.x + r_outer * 1.30 * cos, c.y + r_outer * 1.30 * sin);
                painter.line_segment([p1, p2], Stroke::new(2.4, color));
            }

            // Outer rim ring
            painter.circle_stroke(c, r_outer, stroke);
        }
        IconKind::Undo => {
            // Procedural vector curved back-arrow
            let tip = Pos2::new(c.x - w * 0.32, c.y - h * 0.05);
            let fin1 = Pos2::new(c.x - w * 0.14, c.y - h * 0.24);
            let fin2 = Pos2::new(c.x - w * 0.14, c.y + h * 0.14);
            painter.line_segment([tip, fin1], stroke);
            painter.line_segment([tip, fin2], stroke);

            // Smooth arching curve
            let p0 = Pos2::new(c.x - w * 0.24, c.y - h * 0.05);
            let p1 = Pos2::new(c.x + w * 0.05, c.y - h * 0.40);
            let p2 = Pos2::new(c.x + w * 0.28, c.y - h * 0.08);
            let p3 = Pos2::new(c.x + w * 0.22, c.y + h * 0.28);

            let mut curve_pts = Vec::with_capacity(11);
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let omt = 1.0 - t;
                let px = omt * omt * omt * p0.x
                    + 3.0 * omt * omt * t * p1.x
                    + 3.0 * omt * t * t * p2.x
                    + t * t * t * p3.x;
                let py = omt * omt * omt * p0.y
                    + 3.0 * omt * omt * t * p1.y
                    + 3.0 * omt * t * t * p2.y
                    + t * t * t * p3.y;
                curve_pts.push(Pos2::new(px, py));
            }
            painter.add(egui::Shape::line(curve_pts, stroke));
        }
        IconKind::Save => {
            // 3.5" Diskette / Storage Drive
            let disk_w = w * 0.65;
            let disk_h = h * 0.65;
            let r = Rect::from_center_size(c, Vec2::new(disk_w, disk_h));

            let chamfer = 4.0f32;
            let pts = vec![
                Pos2::new(r.min.x, r.min.y),
                Pos2::new(r.max.x - chamfer, r.min.y),
                Pos2::new(r.max.x, r.min.y + chamfer),
                Pos2::new(r.max.x, r.max.y),
                Pos2::new(r.min.x, r.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color.gamma_multiply(0.20), stroke));

            // Top metal shutter
            let shutter_w = disk_w * 0.45;
            let shutter_h = disk_h * 0.35;
            let shutter_rect = Rect::from_min_size(
                Pos2::new(c.x - shutter_w * 0.5, r.min.y),
                Vec2::new(shutter_w, shutter_h),
            );
            painter.rect_filled(shutter_rect, 0.0, color.gamma_multiply(0.35));
            painter.rect_stroke(shutter_rect, 0.0, thin_stroke, egui::StrokeKind::Inside);
            // Shutter notch
            painter.line_segment(
                [
                    Pos2::new(shutter_rect.center().x - 1.5, shutter_rect.min.y + 2.0),
                    Pos2::new(shutter_rect.center().x - 1.5, shutter_rect.max.y - 2.0),
                ],
                thin_stroke,
            );

            // Bottom label area
            let label_w = disk_w * 0.65;
            let label_h = disk_h * 0.35;
            let label_rect = Rect::from_min_size(
                Pos2::new(c.x - label_w * 0.5, r.max.y - label_h),
                Vec2::new(label_w, label_h),
            );
            painter.rect_stroke(label_rect, 1.0, thin_stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(label_rect.min.x + 2.0, label_rect.center().y),
                    Pos2::new(label_rect.max.x - 2.0, label_rect.center().y),
                ],
                thin_stroke,
            );
        }
        IconKind::SaveAs => {
            // Save base with orange Action Accent ("Save As" pen)
            let disk_w = w * 0.58;
            let disk_h = h * 0.58;
            let r = Rect::from_min_size(Pos2::new(c.x - disk_w * 0.65, c.y - disk_h * 0.65), Vec2::new(disk_w, disk_h));

            let chamfer = 3.5f32;
            let pts = vec![
                Pos2::new(r.min.x, r.min.y),
                Pos2::new(r.max.x - chamfer, r.min.y),
                Pos2::new(r.max.x, r.min.y + chamfer),
                Pos2::new(r.max.x, r.max.y),
                Pos2::new(r.min.x, r.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color.gamma_multiply(0.20), stroke));

            // Bottom right: Action Accent Pen (orange)
            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(2.0, accent_color);
            let pen_tip = Pos2::new(c.x + w * 0.12, c.y + h * 0.35);
            let pen_top = Pos2::new(c.x + w * 0.38, c.y + h * 0.05);
            painter.line_segment([pen_tip, pen_top], accent_stroke);
            painter.circle_filled(pen_tip, 1.5, accent_color);
            painter.line_segment([
                Pos2::new(pen_top.x - 2.5, pen_top.y + 2.5),
                Pos2::new(pen_top.x + 2.5, pen_top.y - 2.5),
            ], accent_stroke);
        }
        IconKind::Import => {
            // Tray/box base with orange arrow pointing inward (down)
            let tray_w = w * 0.70;
            let tray_y = c.y + h * 0.10;
            let tray_h = h * 0.28;
            let t_min_x = c.x - tray_w * 0.5;
            let t_max_x = c.x + tray_w * 0.5;

            painter.line_segment([Pos2::new(t_min_x, tray_y), Pos2::new(t_min_x, tray_y + tray_h)], stroke);
            painter.line_segment([Pos2::new(t_min_x, tray_y + tray_h), Pos2::new(t_max_x, tray_y + tray_h)], stroke);
            painter.line_segment([Pos2::new(t_max_x, tray_y + tray_h), Pos2::new(t_max_x, tray_y)], stroke);

            // Action Accent Inward Arrow (orange)
            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(2.2, accent_color);
            let arrow_tip = Pos2::new(c.x, c.y + h * 0.18);
            let arrow_top = Pos2::new(c.x, c.y - h * 0.38);
            painter.line_segment([arrow_top, arrow_tip], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x - w * 0.18, c.y + h * 0.02)], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x + w * 0.18, c.y + h * 0.02)], accent_stroke);
        }
        IconKind::ImportSkp => {
            // Isometric SketchUp cube in classic red tone with action accent arrow
            let red_primary = Color32::from_rgb(220, 50, 45);
            let red_left = Color32::from_rgb(180, 40, 35);
            let red_right = Color32::from_rgb(150, 30, 25);
            let red_stroke = Stroke::new(1.8, red_primary);

            let box_c = Pos2::new(c.x, c.y + h * 0.12);
            let rx = w * 0.35;
            let zh = h * 0.32;

            let top_c = Pos2::new(box_c.x, box_c.y - zh);
            let top_left = Pos2::new(box_c.x - rx, box_c.y - zh * 0.65);
            let top_right = Pos2::new(box_c.x + rx, box_c.y - zh * 0.65);
            let top_mid = Pos2::new(box_c.x, box_c.y - zh * 0.30);

            let bot_mid = Pos2::new(box_c.x, box_c.y + zh * 0.40);
            let bot_left = Pos2::new(box_c.x - rx, box_c.y + zh * 0.05);
            let bot_right = Pos2::new(box_c.x + rx, box_c.y + zh * 0.05);

            // 1. Top isometric face
            painter.add(egui::Shape::convex_polygon(
                vec![top_c, top_right, top_mid, top_left],
                red_primary.gamma_multiply(0.35),
                red_stroke,
            ));
            // 2. Left face
            painter.add(egui::Shape::convex_polygon(
                vec![top_left, top_mid, bot_mid, bot_left],
                red_left.gamma_multiply(0.45),
                red_stroke,
            ));
            // 3. Right face
            painter.add(egui::Shape::convex_polygon(
                vec![top_mid, top_right, bot_right, bot_mid],
                red_right.gamma_multiply(0.55),
                red_stroke,
            ));

            // High-Contrast Action Inward Arrow (orange)
            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(2.2, accent_color);
            let arrow_tip = Pos2::new(c.x, top_c.y + 4.0);
            let arrow_top = Pos2::new(c.x, c.y - h * 0.42);
            painter.line_segment([arrow_top, arrow_tip], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x - w * 0.16, arrow_tip.y - h * 0.14)], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x + w * 0.16, arrow_tip.y - h * 0.14)], accent_stroke);

            // Text "SKP" badge inside the front face
            painter.text(
                Pos2::new(c.x, box_c.y + 3.0),
                egui::Align2::CENTER_CENTER,
                "SKP",
                egui::FontId::proportional(8.5),
                Color32::WHITE,
            );
        }
        IconKind::ImportDwg => {
            // Isometric AutoCAD DWG blueprint drawing sheet in classic CAD blue with action accent arrow
            let cad_blue = Color32::from_rgb(0, 102, 204);
            let cad_dark = Color32::from_rgb(0, 70, 150);
            let cad_stroke = Stroke::new(1.8, cad_blue);
            let thin_stroke = Stroke::new(1.0, cad_blue.gamma_multiply(0.40));

            let sheet_c = Pos2::new(c.x, c.y + h * 0.12);
            let rx = w * 0.38;
            let zh = h * 0.34;

            let top_c = Pos2::new(sheet_c.x, sheet_c.y - zh);
            let top_left = Pos2::new(sheet_c.x - rx, sheet_c.y - zh * 0.50);
            let top_right = Pos2::new(sheet_c.x + rx, sheet_c.y - zh * 0.50);
            let top_mid = Pos2::new(sheet_c.x, sheet_c.y);

            let bot_mid = Pos2::new(sheet_c.x, sheet_c.y + zh * 0.40);
            let bot_left = Pos2::new(sheet_c.x - rx, sheet_c.y - zh * 0.10);
            let bot_right = Pos2::new(sheet_c.x + rx, sheet_c.y - zh * 0.10);

            // 1. Isometric blueprint paper polygon
            painter.add(egui::Shape::convex_polygon(
                vec![top_c, top_right, top_mid, top_left],
                cad_blue.gamma_multiply(0.30),
                cad_stroke,
            ));
            // 2. Front edge of sheet (thickness)
            painter.add(egui::Shape::convex_polygon(
                vec![top_left, top_mid, bot_mid, bot_left],
                cad_dark.gamma_multiply(0.50),
                cad_stroke,
            ));
            painter.add(egui::Shape::convex_polygon(
                vec![top_mid, top_right, bot_right, bot_mid],
                cad_dark.gamma_multiply(0.60),
                cad_stroke,
            ));

            // 3. Drafting grid lines on the CAD sheet
            painter.line_segment(
                [
                    Pos2::new(sheet_c.x - rx * 0.5, sheet_c.y - zh * 0.75),
                    Pos2::new(sheet_c.x + rx * 0.5, sheet_c.y - zh * 0.25),
                ],
                thin_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(sheet_c.x - rx * 0.5, sheet_c.y - zh * 0.25),
                    Pos2::new(sheet_c.x + rx * 0.5, sheet_c.y - zh * 0.75),
                ],
                thin_stroke,
            );

            // High-Contrast Action Inward Arrow (orange)
            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(2.2, accent_color);
            let arrow_tip = Pos2::new(c.x, top_c.y + 4.0);
            let arrow_top = Pos2::new(c.x, c.y - h * 0.42);
            painter.line_segment([arrow_top, arrow_tip], accent_stroke);
            painter.line_segment(
                [arrow_tip, Pos2::new(c.x - w * 0.16, arrow_tip.y - h * 0.14)],
                accent_stroke,
            );
            painter.line_segment(
                [arrow_tip, Pos2::new(c.x + w * 0.16, arrow_tip.y - h * 0.14)],
                accent_stroke,
            );

            // Text "DWG" badge inside the front face
            painter.text(
                Pos2::new(c.x, sheet_c.y + zh * 0.18),
                egui::Align2::CENTER_CENTER,
                "DWG",
                egui::FontId::proportional(8.5),
                Color32::WHITE,
            );
        }
        IconKind::Export => {
            // Tray base with orange arrow pointing outward (up)
            let tray_w = w * 0.70;
            let tray_y = c.y + h * 0.10;
            let tray_h = h * 0.28;
            let t_min_x = c.x - tray_w * 0.5;
            let t_max_x = c.x + tray_w * 0.5;

            painter.line_segment([Pos2::new(t_min_x, tray_y), Pos2::new(t_min_x, tray_y + tray_h)], stroke);
            painter.line_segment([Pos2::new(t_min_x, tray_y + tray_h), Pos2::new(t_max_x, tray_y + tray_h)], stroke);
            painter.line_segment([Pos2::new(t_max_x, tray_y + tray_h), Pos2::new(t_max_x, tray_y)], stroke);

            // Action Accent Outward Arrow (orange)
            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(2.2, accent_color);
            let arrow_tip = Pos2::new(c.x, c.y - h * 0.38);
            let arrow_bot = Pos2::new(c.x, c.y + h * 0.12);
            painter.line_segment([arrow_bot, arrow_tip], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x - w * 0.18, c.y - h * 0.22)], accent_stroke);
            painter.line_segment([arrow_tip, Pos2::new(c.x + w * 0.18, c.y - h * 0.22)], accent_stroke);
        }
        IconKind::ExportPdf => {
            // Architectural Drawing Sheet with Dog-Ear Fold and Red "PDF" Badge
            let sheet_w = w * 0.62;
            let sheet_h = h * 0.74;
            let r = Rect::from_center_size(Pos2::new(c.x - w * 0.04, c.y - h * 0.04), Vec2::new(sheet_w, sheet_h));
            let fold = 5.0f32;

            // Sheet Paper Polygon with folded corner
            let paper_pts = vec![
                Pos2::new(r.min.x, r.min.y),
                Pos2::new(r.max.x - fold, r.min.y),
                Pos2::new(r.max.x, r.min.y + fold),
                Pos2::new(r.max.x, r.max.y),
                Pos2::new(r.min.x, r.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                paper_pts,
                color.gamma_multiply(0.12),
                stroke,
            ));

            // Corner fold triangle
            let fold_pts = vec![
                Pos2::new(r.max.x - fold, r.min.y),
                Pos2::new(r.max.x, r.min.y + fold),
                Pos2::new(r.max.x - fold, r.min.y + fold),
            ];
            painter.add(egui::Shape::convex_polygon(
                fold_pts,
                color.gamma_multiply(0.35),
                Stroke::new(1.0, color.gamma_multiply(0.7)),
            ));

            // Architectural drafting lines
            let line_color = color.gamma_multiply(0.38);
            let line_stroke = Stroke::new(1.0, line_color);
            let line_left = r.min.x + sheet_w * 0.16;
            let line_right = r.max.x - sheet_w * 0.20;
            let y1 = r.min.y + sheet_h * 0.30;
            let y2 = r.min.y + sheet_h * 0.46;
            let y3 = r.min.y + sheet_h * 0.62;
            painter.line_segment([Pos2::new(line_left, y1), Pos2::new(line_right, y1)], line_stroke);
            painter.line_segment([Pos2::new(line_left, y2), Pos2::new(line_right * 0.85 + line_left * 0.15, y2)], line_stroke);
            painter.line_segment([Pos2::new(line_left, y3), Pos2::new(line_right * 0.70 + line_left * 0.30, y3)], line_stroke);

            // Red PDF pill badge on the bottom-right
            let badge_w = w * 0.48;
            let badge_h = h * 0.32;
            let badge_center = Pos2::new(c.x + w * 0.20, c.y + h * 0.22);
            let badge_rect = Rect::from_center_size(badge_center, Vec2::new(badge_w, badge_h));
            let badge_color = Color32::from_rgb(220, 48, 48); // Architectural PDF red
            painter.rect_filled(badge_rect, 3.0, badge_color);
            painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0, Color32::from_rgb(255, 90, 90)), egui::StrokeKind::Middle);

            // Bold white text "PDF"
            painter.text(
                badge_center,
                egui::Align2::CENTER_CENTER,
                "PDF",
                egui::FontId::proportional((h * 0.20).max(7.5)),
                Color32::WHITE,
            );
        }
        IconKind::ExportGlb => {
            // Isometric 3D Solid Model with Cyan/Violet Faces and "GLB" Badge
            let cube_center = Pos2::new(c.x - w * 0.08, c.y - h * 0.06);
            let s = (w.min(h) * 0.38).max(6.0);
            let dx = s * 0.866;
            let dy = s * 0.50;

            let top_p = Pos2::new(cube_center.x, cube_center.y - s);
            let right_p = Pos2::new(cube_center.x + dx, cube_center.y - dy);
            let bot_right_p = Pos2::new(cube_center.x + dx, cube_center.y + dy);
            let bot_p = Pos2::new(cube_center.x, cube_center.y + s);
            let bot_left_p = Pos2::new(cube_center.x - dx, cube_center.y + dy);
            let left_p = Pos2::new(cube_center.x - dx, cube_center.y - dy);

            // Top Face (Pastel Sky Cyan)
            painter.add(egui::Shape::convex_polygon(
                vec![top_p, right_p, cube_center, left_p],
                Color32::from_rgb(0, 200, 255).gamma_multiply(0.35),
                Stroke::new(1.2, Color32::from_rgb(0, 200, 255)),
            ));

            // Left Face (Electric Violet)
            painter.add(egui::Shape::convex_polygon(
                vec![left_p, cube_center, bot_p, bot_left_p],
                Color32::from_rgb(217, 70, 239).gamma_multiply(0.28),
                Stroke::new(1.2, Color32::from_rgb(217, 70, 239)),
            ));

            // Right Face (Muted Architectural Slate)
            painter.add(egui::Shape::convex_polygon(
                vec![cube_center, right_p, bot_right_p, bot_p],
                Color32::from_rgb(180, 190, 205).gamma_multiply(0.25),
                Stroke::new(1.2, Color32::from_rgb(180, 190, 205)),
            ));

            // Outer contour
            painter.line_segment([top_p, right_p], Stroke::new(1.5, color));
            painter.line_segment([right_p, bot_right_p], Stroke::new(1.5, color));
            painter.line_segment([bot_right_p, bot_p], Stroke::new(1.5, color));
            painter.line_segment([bot_p, bot_left_p], Stroke::new(1.5, color));
            painter.line_segment([bot_left_p, left_p], Stroke::new(1.5, color));
            painter.line_segment([left_p, top_p], Stroke::new(1.5, color));

            // Purple GLB pill badge on the bottom-right
            let badge_w = w * 0.48;
            let badge_h = h * 0.30;
            let badge_center = Pos2::new(c.x + w * 0.22, c.y + h * 0.24);
            let badge_rect = Rect::from_center_size(badge_center, Vec2::new(badge_w, badge_h));
            let badge_color = Color32::from_rgb(147, 51, 234); // GLB purple
            painter.rect_filled(badge_rect, 3.0, badge_color);
            painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0, Color32::from_rgb(192, 132, 252)), egui::StrokeKind::Middle);

            painter.text(
                badge_center,
                egui::Align2::CENTER_CENTER,
                "GLB",
                egui::FontId::proportional((h * 0.19).max(7.0)),
                Color32::WHITE,
            );
        }
        IconKind::RenderMuTracer => {
            // Glass sphere with physically based ray refraction, reflection and photon spark
            let sphere_r = (w.min(h) * 0.28).max(5.0);
            let sphere_center = Pos2::new(c.x + w * 0.05, c.y + h * 0.05);

            // Translucent glass sphere body
            painter.circle_filled(
                sphere_center,
                sphere_r,
                Color32::from_rgb(0, 200, 255).gamma_multiply(0.25),
            );
            painter.circle_stroke(
                sphere_center,
                sphere_r,
                Stroke::new(1.6, Color32::from_rgb(0, 200, 255)),
            );

            // Specular glass crescent highlight
            let hl_pos = Pos2::new(sphere_center.x - sphere_r * 0.35, sphere_center.y - sphere_r * 0.35);
            painter.circle_filled(hl_pos, sphere_r * 0.25, Color32::from_white_alpha(180));

            // Incident Light Ray (Amber Gold from top-left)
            let ray_start = Pos2::new(c.x - w * 0.42, c.y - h * 0.38);
            let hit_pt = Pos2::new(sphere_center.x - sphere_r * 0.707, sphere_center.y - sphere_r * 0.707);
            painter.line_segment([ray_start, hit_pt], Stroke::new(2.0, Color32::from_rgb(255, 180, 0)));

            // Reflected Ray (Electric Violet bouncing up-right)
            let refl_end = Pos2::new(c.x + w * 0.35, c.y - h * 0.42);
            painter.line_segment([hit_pt, refl_end], Stroke::new(1.6, Color32::from_rgb(217, 70, 239)));

            // Refracted Transmitted Ray (Cyan bending through and out)
            let exit_pt = Pos2::new(sphere_center.x + sphere_r * 0.60, sphere_center.y + sphere_r * 0.80);
            let trans_end = Pos2::new(c.x + w * 0.42, c.y + h * 0.42);
            painter.line_segment([hit_pt, exit_pt], Stroke::new(1.5, Color32::from_rgb(0, 220, 255).gamma_multiply(0.7)));
            painter.line_segment([exit_pt, trans_end], Stroke::new(2.0, Color32::from_rgb(0, 220, 255)));

            // Sparkling Photon Star at Hit Point
            let spark_r = 2.5f32;
            painter.circle_filled(hit_pt, spark_r, Color32::WHITE);
        }
        IconKind::PluginManager => {
            // Modular interlocking puzzle piece / extension plug
            let pw = w * 0.58;
            let ph = h * 0.58;
            let pr = Rect::from_center_size(c, Vec2::new(pw, ph));
            let tab_r = pw * 0.20;

            // Base rounded rect
            painter.rect_filled(pr, 4.0, color.gamma_multiply(0.20));
            painter.rect_stroke(pr, 4.0, stroke, egui::StrokeKind::Middle);

            // Right interlocking tab
            let right_tab_center = Pos2::new(pr.max.x, c.y);
            painter.circle_filled(right_tab_center, tab_r, color.gamma_multiply(0.35));
            painter.circle_stroke(right_tab_center, tab_r, stroke);

            // Top interlocking tab
            let top_tab_center = Pos2::new(c.x, pr.min.y);
            painter.circle_filled(top_tab_center, tab_r, color.gamma_multiply(0.35));
            painter.circle_stroke(top_tab_center, tab_r, stroke);

            // Center neon plus / connection pin
            let accent_color = Color32::from_rgb(217, 70, 239); // Violet
            let s_acc = Stroke::new(2.0, accent_color);
            let cross_len = pw * 0.25;
            painter.line_segment([Pos2::new(c.x - cross_len, c.y), Pos2::new(c.x + cross_len, c.y)], s_acc);
            painter.line_segment([Pos2::new(c.x, c.y - cross_len), Pos2::new(c.x, c.y + cross_len)], s_acc);
        }
        IconKind::BlenderLiveSync => {
            // Authentic Blender vector emblem precisely proportioned from Blender-Emblem.png
            // Features outer circular eye body, 3 radiating arms (top diagonal, middle horizontal, bottom-left diagonal),
            // high-contrast white ring cutout, and solid central pupil.
            let s = (w.min(h) * 0.95).max(16.0);
            let bw = s;
            let bh = s * 0.81; // Exact aspect ratio 2065 / 2550

            let x0 = c.x - bw * 0.5;
            let y0 = c.y - bh * 0.5;

            let hub = Pos2::new(x0 + 0.6597 * bw, y0 + 0.5781 * bh);
            let r_outer = 0.3350 * bw;
            let r_white = 0.2118 * bw;
            let r_pupil = 0.1280 * bw;

            let arm_w = 0.0980 * bw;
            let arm_r = arm_w * 0.5;
            let s_arm = Stroke::new(arm_w, color);

            // 1. Outer circular eye body
            painter.circle_filled(hub, r_outer, color);

            // 2. Three Radiating Arms (Top Diagonal, Middle Horizontal, Bottom-Left Diagonal)
            // Top arm: extends from upper-right of body diagonally up to top-center peak
            let top_tip = Pos2::new(x0 + 0.5648 * bw, y0 + 0.0500 * bh);
            let top_root = Pos2::new(x0 + 0.7200 * bw, y0 + 0.3500 * bh);
            painter.line_segment([top_root, top_tip], s_arm);
            painter.circle_filled(top_tip, arm_r, color);

            // Middle arm: horizontal bar extending straight to the left
            let mid_tip = Pos2::new(x0 + 0.1451 * bw, y0 + 0.2824 * bh);
            let mid_root = Pos2::new(hub.x, mid_tip.y);
            painter.line_segment([mid_root, mid_tip], s_arm);
            painter.circle_filled(mid_tip, arm_r, color);

            // Bottom-left arm: diagonal bar extending down-left to corner
            let bot_tip = Pos2::new(x0 + 0.0550 * bw, y0 + 0.7106 * bh);
            let bot_root = Pos2::new(x0 + 0.4200 * bw, y0 + 0.6500 * bh);
            painter.line_segment([bot_root, bot_tip], s_arm);
            painter.circle_filled(bot_tip, arm_r, color);

            // 3. Crisp white gap ring cutout (authentic eye aperture)
            painter.circle_filled(hub, r_white, Color32::WHITE);

            // 4. Solid Central Iris Pupil
            painter.circle_filled(hub, r_pupil, color);
        }
        IconKind::NewProject => {
            // Document with folded top-right corner and plus sign
            let doc_w = w * 0.55;
            let doc_h = h * 0.68;
            let r = Rect::from_center_size(c, Vec2::new(doc_w, doc_h));
            let fold = 5.0f32;
            let pts = vec![
                Pos2::new(r.min.x, r.min.y),
                Pos2::new(r.max.x - fold, r.min.y),
                Pos2::new(r.max.x, r.min.y + fold),
                Pos2::new(r.max.x, r.max.y),
                Pos2::new(r.min.x, r.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color.gamma_multiply(0.20), stroke));
            painter.line_segment([Pos2::new(r.max.x - fold, r.min.y), Pos2::new(r.max.x - fold, r.min.y + fold)], thin_stroke);
            painter.line_segment([Pos2::new(r.max.x - fold, r.min.y + fold), Pos2::new(r.max.x, r.min.y + fold)], thin_stroke);

            let accent_color = Color32::from_rgb(255, 120, 0);
            let accent_stroke = Stroke::new(1.8, accent_color);
            let pc = Pos2::new(c.x, c.y + 2.0);
            painter.line_segment([Pos2::new(pc.x - 4.0, pc.y), Pos2::new(pc.x + 4.0, pc.y)], accent_stroke);
            painter.line_segment([Pos2::new(pc.x, pc.y - 4.0), Pos2::new(pc.x, pc.y + 4.0)], accent_stroke);
        }
        IconKind::Redo => {
            // Procedural vector curved forward-arrow (exact mirror of Undo)
            let tip = Pos2::new(c.x + w * 0.32, c.y - h * 0.05);
            let fin1 = Pos2::new(c.x + w * 0.14, c.y - h * 0.24);
            let fin2 = Pos2::new(c.x + w * 0.14, c.y + h * 0.14);
            painter.line_segment([tip, fin1], stroke);
            painter.line_segment([tip, fin2], stroke);

            let p0 = Pos2::new(c.x + w * 0.24, c.y - h * 0.05);
            let p1 = Pos2::new(c.x - w * 0.05, c.y - h * 0.40);
            let p2 = Pos2::new(c.x - w * 0.28, c.y - h * 0.08);
            let p3 = Pos2::new(c.x - w * 0.22, c.y + h * 0.28);

            let mut curve_pts = Vec::with_capacity(11);
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let omt = 1.0 - t;
                let px = omt * omt * omt * p0.x
                    + 3.0 * omt * omt * t * p1.x
                    + 3.0 * omt * t * t * p2.x
                    + t * t * t * p3.x;
                let py = omt * omt * omt * p0.y
                    + 3.0 * omt * omt * t * p1.y
                    + 3.0 * omt * t * t * p2.y
                    + t * t * t * p3.y;
                curve_pts.push(Pos2::new(px, py));
            }
            painter.add(egui::Shape::line(curve_pts, stroke));
        }
        IconKind::Arch3dApp => {
            // Dark modern squircle background tile matching user official logo
            let radius = (w.min(h) * 0.24).clamp(3.0, 16.0);
            let bg_color = Color32::from_rgb(13, 14, 18); // Deep obsidian dark charcoal
            let rim_glow = Color32::from_rgb(217, 70, 239); // Neon magenta/violet rim

            painter.rect_filled(rect, radius, bg_color);
            painter.rect_stroke(rect, radius, Stroke::new(1.8, rim_glow), egui::StrokeKind::Inside);

            // Sweeping neon purple accent in bottom-right corner
            let br_pt = rect.right_bottom();
            let sweep_w = w * 0.52;
            let sweep_h = h * 0.38;
            let sweep_pts = vec![
                Pos2::new(br_pt.x - sweep_w, br_pt.y),
                Pos2::new(br_pt.x - sweep_w * 0.25, br_pt.y - sweep_h * 0.75),
                Pos2::new(br_pt.x, br_pt.y - sweep_h),
                br_pt,
            ];
            painter.add(egui::Shape::convex_polygon(
                sweep_pts,
                Color32::from_rgba_premultiplied(192, 38, 211, 230),
                Stroke::NONE,
            ));

            // Central bold white isometric wireframe cube
            let cube_r = (w.min(h) * 0.28).max(5.0);
            let cos30 = 0.8660254f32;
            let sin30 = 0.5000000f32;

            let top_v = Pos2::new(c.x, c.y - cube_r);
            let mid_v = c;
            let bot_v = Pos2::new(c.x, c.y + cube_r);
            let tl_v = Pos2::new(c.x - cube_r * cos30, c.y - cube_r * sin30);
            let bl_v = Pos2::new(c.x - cube_r * cos30, c.y + cube_r * sin30);
            let tr_v = Pos2::new(c.x + cube_r * cos30, c.y - cube_r * sin30);
            let br_v = Pos2::new(c.x + cube_r * cos30, c.y + cube_r * sin30);

            let cube_stroke = Stroke::new((w * 0.08).clamp(1.8, 3.2), Color32::WHITE);

            // Outer hexagon contour: top -> tr -> br -> bot -> bl -> tl -> top
            painter.add(egui::Shape::line(
                vec![top_v, tr_v, br_v, bot_v, bl_v, tl_v, top_v],
                cube_stroke,
            ));

            // Inner Y-spokes: mid to top, mid to bl, mid to br
            painter.line_segment([mid_v, top_v], cube_stroke);
            painter.line_segment([mid_v, bl_v], cube_stroke);
            painter.line_segment([mid_v, br_v], cube_stroke);
        }
        IconKind::Roof => {
            // Pitched architectural gable roof with eave overhang
            let p_peak = Pos2::new(c.x, c.y - h * 0.35);
            let p_left = Pos2::new(c.x - w * 0.40, c.y + h * 0.20);
            let p_right = Pos2::new(c.x + w * 0.40, c.y + h * 0.20);
            let p_ceiling_l = Pos2::new(c.x - w * 0.30, c.y + h * 0.20);
            let p_ceiling_r = Pos2::new(c.x + w * 0.30, c.y + h * 0.20);
            let p_base_l = Pos2::new(c.x - w * 0.30, c.y + h * 0.35);
            let p_base_r = Pos2::new(c.x + w * 0.30, c.y + h * 0.35);

            let roof_poly = vec![p_peak, p_right, p_ceiling_r, p_ceiling_l, p_left];
            painter.add(egui::Shape::convex_polygon(roof_poly, color.gamma_multiply(0.25), stroke));
            painter.line_segment([p_ceiling_l, p_base_l], thin_stroke);
            painter.line_segment([p_ceiling_r, p_base_r], thin_stroke);
        }
        IconKind::Fascia => {
            // Roof overhang with vertical fascia trim board
            let eave_line_l = Pos2::new(c.x - w * 0.35, c.y - h * 0.25);
            let eave_line_r = Pos2::new(c.x + w * 0.15, c.y - h * 0.25);
            painter.line_segment([eave_line_l, eave_line_r], stroke);

            // Vertical Fascia board
            let fascia_r = Rect::from_min_max(
                Pos2::new(c.x + w * 0.10, c.y - h * 0.28),
                Pos2::new(c.x + w * 0.30, c.y + h * 0.30),
            );
            painter.rect_filled(fascia_r, 1.0, color.gamma_multiply(0.3));
            painter.rect_stroke(fascia_r, 1.0, stroke, egui::StrokeKind::Middle);
        }
        IconKind::Gutter => {
            // U-shaped eave gutter channel with falling water droplet
            let p1 = Pos2::new(c.x - w * 0.30, c.y - h * 0.20);
            let p2 = Pos2::new(c.x - w * 0.30, c.y + h * 0.15);
            let p3 = Pos2::new(c.x + w * 0.10, c.y + h * 0.15);
            let p4 = Pos2::new(c.x + w * 0.10, c.y - h * 0.20);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);

            // Water flow indicator
            let down_start = Pos2::new(c.x - w * 0.10, c.y + h * 0.15);
            let down_end = Pos2::new(c.x - w * 0.10, c.y + h * 0.40);
            painter.line_segment([down_start, down_end], Stroke::new(1.4, Color32::from_rgb(0, 180, 255)));
        }
        IconKind::Stair => {
            // Stepped architectural staircase profile with treads and risers
            let p0 = Pos2::new(c.x - w * 0.38, c.y + h * 0.35);
            let p1 = Pos2::new(c.x - w * 0.38, c.y + h * 0.15);
            let p2 = Pos2::new(c.x - w * 0.15, c.y + h * 0.15);
            let p3 = Pos2::new(c.x - w * 0.15, c.y - h * 0.05);
            let p4 = Pos2::new(c.x + w * 0.10, c.y - h * 0.05);
            let p5 = Pos2::new(c.x + w * 0.10, c.y - h * 0.25);
            let p6 = Pos2::new(c.x + w * 0.35, c.y - h * 0.25);
            let p7 = Pos2::new(c.x + w * 0.35, c.y + h * 0.35);

            let step_poly = vec![p0, p1, p2, p3, p4, p5, p6, p7];
            painter.add(egui::Shape::convex_polygon(step_poly, color.gamma_multiply(0.20), stroke));

            // Upward arrow accent
            let arrow_color = Color32::from_rgb(255, 120, 0);
            let arrow_stroke = Stroke::new(1.8, arrow_color);
            let a_start = Pos2::new(c.x - w * 0.20, c.y + h * 0.28);
            let a_end = Pos2::new(c.x + w * 0.20, c.y - h * 0.12);
            painter.line_segment([a_start, a_end], arrow_stroke);
            painter.line_segment([a_end, Pos2::new(a_end.x - 4.0, a_end.y)], arrow_stroke);
            painter.line_segment([a_end, Pos2::new(a_end.x, a_end.y + 4.0)], arrow_stroke);
        }
        IconKind::Ramp => {
            // Sloped ramp plane with bottom/top landings
            let p0 = Pos2::new(c.x - w * 0.40, c.y + h * 0.28);
            let p1 = Pos2::new(c.x - w * 0.25, c.y + h * 0.28);
            let p2 = Pos2::new(c.x + w * 0.25, c.y - h * 0.20);
            let p3 = Pos2::new(c.x + w * 0.40, c.y - h * 0.20);
            let p4 = Pos2::new(c.x + w * 0.40, c.y + h * 0.28);

            let ramp_poly = vec![p0, p1, p2, p3, p4];
            painter.add(egui::Shape::convex_polygon(ramp_poly, color.gamma_multiply(0.20), stroke));

            // Incline slope arrow
            let arrow_color = Color32::from_rgb(255, 120, 0);
            let arrow_stroke = Stroke::new(1.6, arrow_color);
            let a_start = Pos2::new(c.x - w * 0.15, c.y + h * 0.15);
            let a_end = Pos2::new(c.x + w * 0.15, c.y - h * 0.05);
            painter.line_segment([a_start, a_end], arrow_stroke);
            painter.line_segment([a_end, Pos2::new(a_end.x - 4.0, a_end.y)], arrow_stroke);
            painter.line_segment([a_end, Pos2::new(a_end.x, a_end.y + 4.0)], arrow_stroke);
        }
        IconKind::Railing => {
            // Horizontal top handrail
            let top_l = Pos2::new(c.x - w * 0.38, c.y - h * 0.22);
            let top_r = Pos2::new(c.x + w * 0.38, c.y - h * 0.22);
            painter.line_segment([top_l, top_r], Stroke::new(2.2, color));

            // Horizontal bottom rail
            let bot_l = Pos2::new(c.x - w * 0.38, c.y + h * 0.25);
            let bot_r = Pos2::new(c.x + w * 0.38, c.y + h * 0.25);
            painter.line_segment([bot_l, bot_r], stroke);

            // Vertical baluster posts
            let xs = [
                c.x - w * 0.32,
                c.x - w * 0.16,
                c.x,
                c.x + w * 0.16,
                c.x + w * 0.32,
            ];
            for x in xs {
                painter.line_segment(
                    [Pos2::new(x, c.y - h * 0.22), Pos2::new(x, c.y + h * 0.25)],
                    thin_stroke,
                );
            }
        }
        IconKind::SingleViewport => {
            // Single viewport full frame
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.7));
            painter.rect_filled(r, 2.0, color.gamma_multiply(0.2));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
        }
        IconKind::SplitHorizontal => {
            // Two viewports split horizontally
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.7));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(r.min.x, c.y), Pos2::new(r.max.x, c.y)], stroke);
        }
        IconKind::SplitVertical => {
            // Two viewports split vertically
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.7));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], stroke);
        }
        IconKind::SplitTriple => {
            // 1 large left viewport, 2 stacked right viewports
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.7));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y), Pos2::new(r.max.x, c.y)], stroke);
        }
        IconKind::SplitQuad => {
            // 4 quadrant split viewports
            let r = Rect::from_center_size(c, Vec2::new(w * 0.8, h * 0.7));
            painter.rect_stroke(r, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(r.min.x, c.y), Pos2::new(r.max.x, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], stroke);
        }
        IconKind::Sheet => {
            // Architectural sheet drawing with titleblock border and drawing bubble
            let r = Rect::from_center_size(c, Vec2::new(w * 0.85, h * 0.65));
            painter.rect_filled(r, 1.0, color.gamma_multiply(0.15));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            // Title block right column
            painter.line_segment(
                [Pos2::new(r.max.x - w * 0.18, r.min.y), Pos2::new(r.max.x - w * 0.18, r.max.y)],
                thin_stroke,
            );
            // Drawing title bubble
            painter.circle_stroke(Pos2::new(r.min.x + w * 0.20, r.max.y - h * 0.18), w * 0.08, thin_stroke);
        }
        IconKind::NewSheet => {
            // Sheet icon with green/cyan '+' badge
            let r = Rect::from_center_size(c - Vec2::new(2.0, 2.0), Vec2::new(w * 0.7, h * 0.6));
            painter.rect_filled(r, 1.0, color.gamma_multiply(0.15));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            // Badge '+'
            let badge_c = Pos2::new(rect.max.x - 4.0, rect.max.y - 4.0);
            let badge_stroke = Stroke::new(1.8, Color32::from_rgb(0, 200, 100));
            painter.line_segment([Pos2::new(badge_c.x - 4.0, badge_c.y), Pos2::new(badge_c.x + 4.0, badge_c.y)], badge_stroke);
            painter.line_segment([Pos2::new(badge_c.x, badge_c.y - 4.0), Pos2::new(badge_c.x, badge_c.y + 4.0)], badge_stroke);
        }
        IconKind::Footing => {
            // Stepped concrete foundation pad footing + column stub + soil hatch
            let pad = Rect::from_center_size(Pos2::new(c.x, c.y + h * 0.16), Vec2::new(w * 0.78, h * 0.22));
            let ped = Rect::from_center_size(Pos2::new(c.x, c.y - h * 0.02), Vec2::new(w * 0.48, h * 0.14));
            let col = Rect::from_min_max(
                Pos2::new(c.x - w * 0.13, c.y - h * 0.34),
                Pos2::new(c.x + w * 0.13, ped.min.y),
            );
            painter.rect_filled(pad, 0.5, color.gamma_multiply(0.25));
            painter.rect_filled(ped, 0.5, color.gamma_multiply(0.20));
            painter.rect_stroke(pad, 0.5, stroke, egui::StrokeKind::Middle);
            painter.rect_stroke(ped, 0.5, stroke, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(col.min.x, col.min.y), Pos2::new(col.min.x, col.max.y)], thin_stroke);
            painter.line_segment([Pos2::new(col.max.x, col.min.y), Pos2::new(col.max.x, col.max.y)], thin_stroke);
            // Soil bearing hatch ticks underneath pad
            for i in -2..=2 {
                let x0 = c.x + (i as f32) * w * 0.14;
                painter.line_segment(
                    [Pos2::new(x0, pad.max.y + 1.0), Pos2::new(x0 - 3.0, pad.max.y + 4.0)],
                    thin_stroke,
                );
            }
        }
        IconKind::LevelManager => {
            // Three horizontal storey datum lines with elevation triangles & bubbles
            let accent = Stroke::new(1.8, Color32::from_rgb(255, 120, 0));
            for (idx, yf) in [-0.24_f32, 0.02, 0.26].into_iter().enumerate() {
                let y = c.y + h * yf;
                let st = if idx == 1 { accent } else { thin_stroke };
                painter.line_segment([Pos2::new(c.x - w * 0.36, y), Pos2::new(c.x + w * 0.18, y)], st);
                let bc = Pos2::new(c.x + w * 0.28, y);
                painter.circle_filled(bc, w * 0.08, color.gamma_multiply(0.18));
                painter.circle_stroke(bc, w * 0.08, if idx == 1 { accent } else { stroke });
            }
            // Vertical elevation scale spine on left
            painter.line_segment(
                [Pos2::new(c.x - w * 0.30, c.y - h * 0.32), Pos2::new(c.x - w * 0.30, c.y + h * 0.32)],
                stroke,
            );
        }
        IconKind::GridManager => {
            // Orthogonal structural grid lines with top & left datum bubbles
            let r_bub = w * 0.085;
            for xf in [-0.14_f32, 0.20] {
                let x = c.x + w * xf;
                let bc = Pos2::new(x, c.y - h * 0.28);
                painter.circle_stroke(bc, r_bub, stroke);
                painter.line_segment([Pos2::new(x, bc.y + r_bub), Pos2::new(x, c.y + h * 0.34)], thin_stroke);
            }
            for yf in [-0.04_f32, 0.22] {
                let y = c.y + h * yf;
                let bc = Pos2::new(c.x - w * 0.30, y);
                painter.circle_stroke(bc, r_bub, stroke);
                painter.line_segment([Pos2::new(bc.x + r_bub, y), Pos2::new(c.x + w * 0.36, y)], thin_stroke);
            }
            // Orange intersection highlight node
            painter.circle_filled(
                Pos2::new(c.x + w * 0.20, c.y - h * 0.04),
                2.4,
                Color32::from_rgb(255, 120, 0),
            );
        }
        IconKind::CopyLevel => {
            // Active level floor plate highlighted with copy twin badge
            let accent = Color32::from_rgb(255, 120, 0);
            let accent_s = Stroke::new(1.8, accent);
            // Lower & upper muted storey plates
            let r_bot = Rect::from_center_size(Pos2::new(c.x - 2.0, c.y + h * 0.22), Vec2::new(w * 0.62, h * 0.12));
            painter.rect_stroke(r_bot, 1.0, thin_stroke, egui::StrokeKind::Middle);
            // Active highlighted plate
            let r_act = Rect::from_center_size(Pos2::new(c.x - 2.0, c.y), Vec2::new(w * 0.66, h * 0.16));
            painter.rect_filled(r_act, 1.0, accent.gamma_multiply(0.28));
            painter.rect_stroke(r_act, 1.0, accent_s, egui::StrokeKind::Middle);
            // Overlapping clipboard/copy card top-right
            let card = Rect::from_center_size(Pos2::new(c.x + w * 0.20, c.y - h * 0.22), Vec2::new(w * 0.32, h * 0.26));
            painter.rect_filled(card, 1.0, color.gamma_multiply(0.20));
            painter.rect_stroke(card, 1.0, stroke, egui::StrokeKind::Middle);
        }
        IconKind::PasteLevel => {
            // Multi-storey plates with orange upward replication arrow to upper levels
            let accent = Color32::from_rgb(255, 120, 0);
            let arrow_s = Stroke::new(2.0, accent);
            for yf in [-0.24_f32, 0.0, 0.24] {
                let plate = Rect::from_center_size(Pos2::new(c.x - w * 0.08, c.y + h * yf), Vec2::new(w * 0.56, h * 0.12));
                painter.rect_filled(plate, 1.0, color.gamma_multiply(0.20));
                painter.rect_stroke(plate, 1.0, stroke, egui::StrokeKind::Middle);
            }
            let ax = c.x + w * 0.30;
            let ay0 = c.y + h * 0.26;
            let ay1 = c.y - h * 0.26;
            painter.line_segment([Pos2::new(ax, ay0), Pos2::new(ax, ay1)], arrow_s);
            painter.line_segment([Pos2::new(ax, ay1), Pos2::new(ax - 3.5, ay1 + 4.5)], arrow_s);
            painter.line_segment([Pos2::new(ax, ay1), Pos2::new(ax + 3.5, ay1 + 4.5)], arrow_s);
        }
        IconKind::AutoColumnsOnGrids => {
            // 2x2 grid crosshairs with solid structural columns snapped at all 4 intersections
            let accent = Color32::from_rgb(255, 120, 0);
            let accent_s = Stroke::new(1.8, accent);
            let xs = [c.x - w * 0.18, c.x + w * 0.18];
            let ys = [c.y - h * 0.18, c.y + h * 0.18];
            for &x in &xs {
                painter.line_segment([Pos2::new(x, c.y - h * 0.36), Pos2::new(x, c.y + h * 0.36)], thin_stroke);
            }
            for &y in &ys {
                painter.line_segment([Pos2::new(c.x - w * 0.36, y), Pos2::new(c.x + w * 0.36, y)], thin_stroke);
            }
            for (ix, &x) in xs.iter().enumerate() {
                for (iy, &y) in ys.iter().enumerate() {
                    let cr = Rect::from_center_size(Pos2::new(x, y), Vec2::new(w * 0.17, h * 0.17));
                    let is_hi = ix == 1 && iy == 0;
                    painter.rect_filled(cr, 0.5, if is_hi { accent.gamma_multiply(0.35) } else { color.gamma_multiply(0.25) });
                    painter.rect_stroke(cr, 0.5, if is_hi { accent_s } else { stroke }, egui::StrokeKind::Middle);
                }
            }
        }
        IconKind::AutoFootingsOnColumns => {
            // Twin structural columns with automated orange-accented pad footings below
            let accent = Color32::from_rgb(255, 120, 0);
            let accent_s = Stroke::new(1.8, accent);
            for xf in [-0.20_f32, 0.20] {
                let cx_i = c.x + w * xf;
                let col_r = Rect::from_min_max(
                    Pos2::new(cx_i - w * 0.08, c.y - h * 0.30),
                    Pos2::new(cx_i + w * 0.08, c.y + h * 0.08),
                );
                let pad_r = Rect::from_min_max(
                    Pos2::new(cx_i - w * 0.17, c.y + h * 0.08),
                    Pos2::new(cx_i + w * 0.17, c.y + h * 0.26),
                );
                painter.rect_filled(col_r, 0.0, color.gamma_multiply(0.20));
                painter.rect_stroke(col_r, 0.0, stroke, egui::StrokeKind::Middle);
                painter.rect_filled(pad_r, 0.5, accent.gamma_multiply(0.30));
                painter.rect_stroke(pad_r, 0.5, accent_s, egui::StrokeKind::Middle);
            }
        }
        IconKind::AutoBeamsOnGrids => {
            // Twin columns connected overhead by an automated structural beam span
            let accent = Color32::from_rgb(255, 120, 0);
            let accent_s = Stroke::new(2.0, accent);
            let x_l = c.x - w * 0.25;
            let x_r = c.x + w * 0.25;
            for x in [x_l, x_r] {
                let col_r = Rect::from_min_max(
                    Pos2::new(x - w * 0.09, c.y - h * 0.10),
                    Pos2::new(x + w * 0.09, c.y + h * 0.30),
                );
                painter.rect_filled(col_r, 0.0, color.gamma_multiply(0.22));
                painter.rect_stroke(col_r, 0.0, stroke, egui::StrokeKind::Middle);
            }
            let beam_r = Rect::from_min_max(
                Pos2::new(x_l - w * 0.10, c.y - h * 0.28),
                Pos2::new(x_r + w * 0.10, c.y - h * 0.10),
            );
            painter.rect_filled(beam_r, 0.5, accent.gamma_multiply(0.30));
            painter.rect_stroke(beam_r, 0.5, accent_s, egui::StrokeKind::Middle);
        }
        IconKind::AutoSlabsOnBays => {
            // Closed 4-column structural bay with beams and automated structural slab fill
            let accent = Color32::from_rgb(255, 120, 0);
            let accent_s = Stroke::new(1.8, accent);
            let bay = Rect::from_center_size(c, Vec2::new(w * 0.58, h * 0.52));
            painter.rect_filled(bay, 1.0, accent.gamma_multiply(0.28));
            painter.rect_stroke(bay, 1.0, accent_s, egui::StrokeKind::Middle);
            // Diagonal structural span symbol inside bay
            painter.line_segment(
                [Pos2::new(bay.min.x + 4.0, c.y), Pos2::new(bay.max.x - 4.0, c.y)],
                thin_stroke,
            );
            // 4 corner columns
            for p in [bay.left_top(), bay.right_top(), bay.right_bottom(), bay.left_bottom()] {
                let cr = Rect::from_center_size(p, Vec2::new(w * 0.15, h * 0.15));
                painter.rect_filled(cr, 0.5, color);
                painter.rect_stroke(cr, 0.5, stroke, egui::StrokeKind::Middle);
            }
        }
        IconKind::OpenProject => {
            // Architectural project folder opening with paper document emerging
            let f_w = w * 0.72;
            let f_h = h * 0.55;
            let f_r = Rect::from_center_size(Pos2::new(c.x, c.y + 2.0), Vec2::new(f_w, f_h));
            // Back folder tab
            let tab_w = f_w * 0.38;
            let tab_h = 3.5;
            let tab_pts = vec![
                Pos2::new(f_r.min.x, f_r.min.y),
                Pos2::new(f_r.min.x + 2.0, f_r.min.y - tab_h),
                Pos2::new(f_r.min.x + tab_w, f_r.min.y - tab_h),
                Pos2::new(f_r.min.x + tab_w + 3.0, f_r.min.y),
            ];
            painter.add(egui::Shape::convex_polygon(tab_pts, color.gamma_multiply(0.20), stroke));
            // Document sheet emerging from folder
            let doc_w = f_w * 0.55;
            let doc_h = f_h * 0.65;
            let doc_r = Rect::from_center_size(Pos2::new(c.x + 2.0, c.y - 3.0), Vec2::new(doc_w, doc_h));
            painter.rect_filled(doc_r, 1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 180));
            painter.rect_stroke(doc_r, 1.0, thin_stroke, egui::StrokeKind::Middle);
            // Document lines
            painter.line_segment([Pos2::new(doc_r.min.x + 3.0, doc_r.min.y + 3.0), Pos2::new(doc_r.max.x - 3.0, doc_r.min.y + 3.0)], thin_stroke);
            painter.line_segment([Pos2::new(doc_r.min.x + 3.0, doc_r.min.y + 6.0), Pos2::new(doc_r.max.x - 5.0, doc_r.min.y + 6.0)], thin_stroke);
            // Front open angled folder flap
            let flap_pts = vec![
                Pos2::new(f_r.min.x - 1.0, f_r.max.y),
                Pos2::new(f_r.min.x + 3.0, f_r.min.y + 2.0),
                Pos2::new(f_r.max.x + 3.0, f_r.min.y + 2.0),
                Pos2::new(f_r.max.x - 1.0, f_r.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(flap_pts, color.gamma_multiply(0.35), stroke));
        }
        IconKind::ZoomExtents => {
            // 4 corner brackets indicating bounds and 4-way diagonal arrows
            let m = 3.0;
            let bl = 4.5;
            let r = Rect::from_min_max(Pos2::new(rect.min.x + m, rect.min.y + m), Pos2::new(rect.max.x - m, rect.max.y - m));
            // 4 corner brackets
            painter.line_segment([Pos2::new(r.min.x, r.min.y + bl), r.min], stroke);
            painter.line_segment([r.min, Pos2::new(r.min.x + bl, r.min.y)], stroke);

            painter.line_segment([Pos2::new(r.max.x - bl, r.min.y), Pos2::new(r.max.x, r.min.y)], stroke);
            painter.line_segment([Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.min.y + bl)], stroke);

            painter.line_segment([Pos2::new(r.min.x, r.max.y - bl), Pos2::new(r.min.x, r.max.y)], stroke);
            painter.line_segment([Pos2::new(r.min.x, r.max.y), Pos2::new(r.min.x + bl, r.max.y)], stroke);

            painter.line_segment([Pos2::new(r.max.x - bl, r.max.y), r.max], stroke);
            painter.line_segment([r.max, Pos2::new(r.max.x, r.max.y - bl)], stroke);

            // Center magnifying / 3D model icon
            let cube_size = 7.0;
            let cube_r = Rect::from_center_size(c, Vec2::splat(cube_size));
            painter.rect_filled(cube_r, 1.0, color.gamma_multiply(0.25));
            painter.rect_stroke(cube_r, 1.0, stroke, egui::StrokeKind::Middle);

            // 4 diagonal arrows pointing from center outward
            let accent_s = Stroke::new(1.4, Color32::from_rgb(255, 120, 0));
            painter.line_segment([Pos2::new(c.x - 4.5, c.y - 4.5), Pos2::new(r.min.x + 3.0, r.min.y + 3.0)], accent_s);
            painter.line_segment([Pos2::new(c.x + 4.5, c.y - 4.5), Pos2::new(r.max.x - 3.0, r.min.y + 3.0)], accent_s);
            painter.line_segment([Pos2::new(c.x - 4.5, c.y + 4.5), Pos2::new(r.min.x + 3.0, r.max.y - 3.0)], accent_s);
            painter.line_segment([Pos2::new(c.x + 4.5, c.y + 4.5), Pos2::new(r.max.x - 3.0, r.max.y - 3.0)], accent_s);
        }
        IconKind::Snapping => {
            // Magnet CAD snap icon with magnetic poles
            let r = Rect::from_center_size(c, Vec2::new(w * 0.55, h * 0.65));
            let u_stroke = Stroke::new(2.4, color);
            painter.line_segment([Pos2::new(r.min.x, r.min.y), Pos2::new(r.min.x, r.max.y - 3.0)], u_stroke);
            painter.line_segment([Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y - 3.0)], u_stroke);
            painter.line_segment([Pos2::new(r.min.x, r.max.y - 3.0), Pos2::new(r.max.x, r.max.y - 3.0)], u_stroke);
            let tip_s = Stroke::new(2.4, Color32::from_rgb(255, 120, 0));
            painter.line_segment([Pos2::new(r.min.x, r.min.y), Pos2::new(r.min.x, r.min.y + 4.0)], tip_s);
            painter.line_segment([Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.min.y + 4.0)], tip_s);
        }
        IconKind::CircularDimension => {
            // Circle with radial dimension line and arrowhead
            let r = (w.min(h) * 0.35).max(6.0);
            painter.circle_stroke(c, r, thin_stroke);
            // Center cross
            painter.line_segment([Pos2::new(c.x - 2.5, c.y), Pos2::new(c.x + 2.5, c.y)], thin_stroke);
            painter.line_segment([Pos2::new(c.x, c.y - 2.5), Pos2::new(c.x, c.y + 2.5)], thin_stroke);
            // Radial leader ray pointing up-right at 45°
            let p_rim = Pos2::new(c.x + r * 0.707, c.y - r * 0.707);
            let p_ext = Pos2::new(p_rim.x + 5.0, p_rim.y - 5.0);
            let p_shelf = Pos2::new(p_ext.x + 6.0, p_ext.y);
            let accent_s = Stroke::new(1.6, Color32::from_rgb(255, 120, 0));
            painter.line_segment([c, p_ext], stroke);
            painter.line_segment([p_ext, p_shelf], stroke);
            // Arrowhead at rim
            painter.line_segment([p_rim, Pos2::new(p_rim.x - 3.5, p_rim.y)], accent_s);
            painter.line_segment([p_rim, Pos2::new(p_rim.x, p_rim.y + 3.5)], accent_s);
        }
        IconKind::AngularDimension => {
            // Two rays meeting at vertex with curved dimension arc
            let apex = Pos2::new(c.x - w * 0.35, c.y + h * 0.3);
            let p1 = Pos2::new(c.x + w * 0.38, c.y + h * 0.3);
            let p2 = Pos2::new(c.x + w * 0.25, c.y - h * 0.35);
            painter.line_segment([apex, p1], stroke);
            painter.line_segment([apex, p2], stroke);
            // Curved arc
            let arc_r = w * 0.42;
            let n_segs = 8;
            let a1: f32 = 0.0;
            let a2 = (p2.y - apex.y).atan2(p2.x - apex.x);
            let mut pts = Vec::with_capacity(n_segs + 1);
            for i in 0..=n_segs {
                let t = i as f32 / n_segs as f32;
                let ang = a1 + (a2 - a1) * t;
                pts.push(Pos2::new(apex.x + arc_r * ang.cos(), apex.y + arc_r * ang.sin()));
            }
            for i in 0..n_segs {
                painter.line_segment([pts[i], pts[i + 1]], Stroke::new(1.5, Color32::from_rgb(255, 120, 0)));
            }
        }
        IconKind::TextAnnotation => {
            // Crisp architectural 'A' letter with baseline shelf
            let base_y = c.y + h * 0.35;
            let top_y = c.y - h * 0.38;
            let apex = Pos2::new(c.x, top_y);
            let left_foot = Pos2::new(c.x - w * 0.28, base_y);
            let right_foot = Pos2::new(c.x + w * 0.28, base_y);
            let cross_y = c.y + h * 0.05;
            let cross_l = Pos2::new(c.x - w * 0.16, cross_y);
            let cross_r = Pos2::new(c.x + w * 0.16, cross_y);
            let thick_s = Stroke::new(2.2, color);
            painter.line_segment([left_foot, apex], thick_s);
            painter.line_segment([apex, right_foot], thick_s);
            painter.line_segment([cross_l, cross_r], Stroke::new(1.8, Color32::from_rgb(255, 120, 0)));
            // Text underline / baseline
            painter.line_segment([Pos2::new(c.x - w * 0.38, base_y + 3.0), Pos2::new(c.x + w * 0.38, base_y + 3.0)], thin_stroke);
        }
        IconKind::Leader => {
            // Pointer arrow pointing down-left with elbow shoulder and text shelf
            let tip = Pos2::new(c.x - w * 0.35, c.y + h * 0.25);
            let shoulder = Pos2::new(c.x + w * 0.02, c.y - h * 0.15);
            let shelf_end = Pos2::new(c.x + w * 0.40, c.y - h * 0.15);
            painter.line_segment([tip, shoulder], stroke);
            painter.line_segment([shoulder, shelf_end], stroke);
            // Arrowhead at tip
            let arr_s = Stroke::new(1.8, Color32::from_rgb(255, 120, 0));
            painter.line_segment([tip, Pos2::new(tip.x + 5.0, tip.y - 1.0)], arr_s);
            painter.line_segment([tip, Pos2::new(tip.x + 1.0, tip.y - 5.0)], arr_s);
            // Small simulated text lines above shelf
            let text_y = shoulder.y - 4.5;
            painter.line_segment([Pos2::new(shoulder.x + 4.0, text_y), Pos2::new(shelf_end.x - 2.0, text_y)], thin_stroke);
        }
        IconKind::BuildingPad => {
            // Excavated foundation pad with sloped earth berms, concrete footing slab, and downward excavation arrow
            let pad_w = w * 0.55;
            let pad_h = h * 0.22;
            let pad_rect = Rect::from_center_size(Pos2::new(c.x, c.y + h * 0.12), Vec2::new(pad_w, pad_h));
            
            // Concrete foundation pad fill and border
            painter.rect_filled(pad_rect, 0.0, color.gamma_multiply(0.28));
            painter.rect_stroke(pad_rect, 0.0, stroke, egui::StrokeKind::Middle);

            // Sloped excavation pit banks on left and right
            let ground_y = c.y - h * 0.22;
            let left_bank = [
                Pos2::new(c.x - w * 0.44, ground_y),
                Pos2::new(pad_rect.min.x, pad_rect.max.y),
            ];
            let right_bank = [
                Pos2::new(pad_rect.max.x, pad_rect.max.y),
                Pos2::new(c.x + w * 0.44, ground_y),
            ];
            painter.line_segment(left_bank, stroke);
            painter.line_segment(right_bank, stroke);
            // Ground surface tails
            painter.line_segment([Pos2::new(c.x - w * 0.48, ground_y), left_bank[0]], thin_stroke);
            painter.line_segment([right_bank[1], Pos2::new(c.x + w * 0.48, ground_y)], thin_stroke);

            // Downward orange excavation arrow
            let arr_x = c.x;
            let arr_y0 = ground_y + 1.0;
            let arr_y1 = pad_rect.min.y - 2.0;
            let orange = Color32::from_rgb(255, 120, 0);
            let s_arr = Stroke::new(1.8, orange);
            painter.line_segment([Pos2::new(arr_x, arr_y0), Pos2::new(arr_x, arr_y1)], s_arr);
            painter.line_segment([Pos2::new(arr_x - 3.5, arr_y1 - 3.5), Pos2::new(arr_x, arr_y1)], s_arr);
            painter.line_segment([Pos2::new(arr_x + 3.5, arr_y1 - 3.5), Pos2::new(arr_x, arr_y1)], s_arr);
        }
        IconKind::Toposolid => {
            // Volumetric solid 3D terrain prism: undulating green top surface + soil strata hatch layers
            let box_w = w * 0.70;
            let box_h = h * 0.48;
            let top_y = c.y - h * 0.18;
            let bot_y = top_y + box_h;
            let x0 = c.x - box_w * 0.5;
            let x1 = c.x + box_w * 0.5;

            // Soil strata solid prism
            let soil_rect = Rect::from_min_max(Pos2::new(x0, top_y + 4.0), Pos2::new(x1, bot_y));
            let turf_green = Color32::from_rgb(92, 135, 70);
            painter.rect_filled(soil_rect, 0.0, color.gamma_multiply(0.20));
            painter.rect_stroke(soil_rect, 0.0, stroke, egui::StrokeKind::Middle);

            // Soil strata horizontal layer line
            let mid_y = top_y + box_h * 0.55;
            painter.line_segment([Pos2::new(x0, mid_y), Pos2::new(x1, mid_y)], thin_stroke);

            // Undulating top terrain surface profile (hills and valleys)
            let mut top_curve = Vec::new();
            for step in 0..=12 {
                let frac = step as f32 / 12.0;
                let px = x0 + frac * box_w;
                let py = top_y + 4.0 - (frac * std::f32::consts::PI * 2.0).sin() * (h * 0.10);
                top_curve.push(Pos2::new(px, py));
            }
            // Green turf cap strip
            let turf_stroke = Stroke::new(2.2, turf_green);
            for i in 0..top_curve.len() - 1 {
                painter.line_segment([top_curve[i], top_curve[i + 1]], turf_stroke);
            }
        }
        IconKind::TerrainContours => {
            // Topographic contour lines with index elevation curves and mountain peak marker
            let r0 = w * 0.40;
            let r1 = w * 0.28;
            let r2 = w * 0.16;
            let center = Pos2::new(c.x, c.y + 1.0);

            let brown = Color32::from_rgb(140, 100, 65);
            let s_primary = Stroke::new(1.8, brown);
            let s_sec = Stroke::new(1.1, color.gamma_multiply(0.65));

            // Outer primary index contour (irregular wavy ellipse)
            let mut pts_outer = Vec::new();
            for i in 0..=16 {
                let a = i as f32 / 16.0 * std::f32::consts::TAU;
                let rad = r0 + (a * 3.0).sin() * 2.5;
                pts_outer.push(Pos2::new(center.x + a.cos() * rad * 0.95, center.y + a.sin() * rad * 0.75));
            }
            for i in 0..pts_outer.len() - 1 {
                painter.line_segment([pts_outer[i], pts_outer[i + 1]], s_primary);
            }

            // Middle contour
            let mut pts_mid = Vec::new();
            for i in 0..=14 {
                let a = i as f32 / 14.0 * std::f32::consts::TAU;
                let rad = r1 + (a * 2.0).cos() * 2.0;
                pts_mid.push(Pos2::new(center.x + a.cos() * rad * 0.95, center.y + a.sin() * rad * 0.75));
            }
            for i in 0..pts_mid.len() - 1 {
                painter.line_segment([pts_mid[i], pts_mid[i + 1]], s_sec);
            }

            // Inner summit contour
            let mut pts_inner = Vec::new();
            for i in 0..=12 {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                pts_inner.push(Pos2::new(center.x + a.cos() * r2 * 0.95, center.y + a.sin() * r2 * 0.75));
            }
            for i in 0..pts_inner.len() - 1 {
                painter.line_segment([pts_inner[i], pts_inner[i + 1]], s_sec);
            }

            // Summit elevation spot marker (small cross)
            let orange = Color32::from_rgb(255, 120, 0);
            painter.line_segment([Pos2::new(center.x - 2.5, center.y), Pos2::new(center.x + 2.5, center.y)], Stroke::new(1.5, orange));
            painter.line_segment([Pos2::new(center.x, center.y - 2.5), Pos2::new(center.x, center.y + 2.5)], Stroke::new(1.5, orange));
        }
        IconKind::LotBoundary => {
            // Cadastral property line parcel: dashed boundary polygon with 4 corner iron pins/monuments
            let pw = w * 0.65;
            let ph = h * 0.55;
            let p_rect = Rect::from_center_size(c, Vec2::new(pw, ph));

            let pts = [
                Pos2::new(p_rect.min.x + 4.0, p_rect.min.y),
                Pos2::new(p_rect.max.x, p_rect.min.y + 6.0),
                Pos2::new(p_rect.max.x - 6.0, p_rect.max.y),
                Pos2::new(p_rect.min.x, p_rect.max.y - 4.0),
            ];

            // Dashed Cadastral Boundary Lines
            let red_cad = Color32::from_rgb(220, 50, 50);
            let s_prop = Stroke::new(1.8, red_cad);
            for i in 0..4 {
                let p0 = pts[i];
                let p1 = pts[(i + 1) % 4];
                // Long dash - dot - long dash
                let mid = Pos2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
                let q1 = Pos2::new(p0.x * 0.7 + p1.x * 0.3, p0.y * 0.7 + p1.y * 0.3);
                let q2 = Pos2::new(p0.x * 0.3 + p1.x * 0.7, p0.y * 0.3 + p1.y * 0.7);
                painter.line_segment([p0, q1], s_prop);
                painter.circle_filled(mid, 1.2, red_cad);
                painter.line_segment([q2, p1], s_prop);
            }

            // Corner iron pin / monument markers (circles with center cross)
            for p in &pts {
                painter.circle_filled(*p, 2.4, Color32::WHITE);
                painter.circle_stroke(*p, 2.4, Stroke::new(1.2, red_cad));
            }
        }
        IconKind::LotBearing => {
            // Surveyor compass bearing: survey boundary line inclined at angle with quadrant arc & North indicator
            let compass_c = Pos2::new(c.x - w * 0.20, c.y + h * 0.15);
            let survey_end = Pos2::new(c.x + w * 0.38, c.y - h * 0.35);

            // North vertical reference line (dashed)
            let north_top = Pos2::new(compass_c.x, compass_c.y - h * 0.42);
            painter.line_segment([compass_c, north_top], thin_stroke);

            // North Arrowhead (Cyan)
            let cyan = Color32::from_rgb(0, 200, 255);
            let s_n = Stroke::new(1.6, cyan);
            painter.line_segment([Pos2::new(north_top.x - 3.0, north_top.y + 4.0), north_top], s_n);
            painter.line_segment([Pos2::new(north_top.x + 3.0, north_top.y + 4.0), north_top], s_n);

            // Inclined Survey Bearing Line (Red)
            let red_cad = Color32::from_rgb(220, 50, 50);
            painter.line_segment([compass_c, survey_end], Stroke::new(2.0, red_cad));
            painter.circle_filled(compass_c, 2.5, red_cad);
            painter.circle_filled(survey_end, 2.5, red_cad);

            // Quadrant Bearing Arc from North to Survey line
            let arc_r = w * 0.16;
            for i in 0..=6 {
                let frac = i as f32 / 6.0;
                let a = -std::f32::consts::FRAC_PI_2 + frac * 0.95; // From North CW towards survey line
                let px = compass_c.x + a.cos() * arc_r;
                let py = compass_c.y + a.sin() * arc_r;
                if i > 0 {
                    let prev_a = -std::f32::consts::FRAC_PI_2 + (i - 1) as f32 / 6.0 * 0.95;
                    let p0 = Pos2::new(compass_c.x + prev_a.cos() * arc_r, compass_c.y + prev_a.sin() * arc_r);
                    painter.line_segment([p0, Pos2::new(px, py)], thin_stroke);
                }
            }
        }
        IconKind::Road => {
            // Perspective asphalt roadway with curbs, solid white shoulder lines, and dashed yellow centerline
            let road_poly = vec![
                Pos2::new(c.x - w * 0.18, c.y - h * 0.35),
                Pos2::new(c.x + w * 0.18, c.y - h * 0.35),
                Pos2::new(c.x + w * 0.42, c.y + h * 0.38),
                Pos2::new(c.x - w * 0.42, c.y + h * 0.38),
            ];
            // Asphalt fill
            painter.add(egui::Shape::convex_polygon(road_poly.clone(), color.gamma_multiply(0.24), stroke));

            // Outer Concrete Curb Gutters
            let curb_s = Stroke::new(2.2, color);
            painter.line_segment([road_poly[0], road_poly[3]], curb_s);
            painter.line_segment([road_poly[1], road_poly[2]], curb_s);

            // White edge shoulder stripes
            let s_white = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 200));
            let left_sh_top = Pos2::new(c.x - w * 0.14, c.y - h * 0.35);
            let left_sh_bot = Pos2::new(c.x - w * 0.34, c.y + h * 0.38);
            let right_sh_top = Pos2::new(c.x + w * 0.14, c.y - h * 0.35);
            let right_sh_bot = Pos2::new(c.x + w * 0.34, c.y + h * 0.38);
            painter.line_segment([left_sh_top, left_sh_bot], s_white);
            painter.line_segment([right_sh_top, right_sh_bot], s_white);

            // Dashed Yellow Centerline
            let yellow = Color32::from_rgb(255, 190, 0);
            let s_yel = Stroke::new(1.8, yellow);
            for i in 0..3 {
                let t0 = 0.12 + i as f32 * 0.30;
                let t1 = t0 + 0.18;
                let y_top = (c.y - h * 0.35) * (1.0 - t0) + (c.y + h * 0.38) * t0;
                let y_bot = (c.y - h * 0.35) * (1.0 - t1) + (c.y + h * 0.38) * t1;
                painter.line_segment([Pos2::new(c.x, y_top), Pos2::new(c.x, y_bot)], s_yel);
            }
        }
        IconKind::Sidewalk => {
            // Elevated concrete sidewalk slab with rounded bullnose curb edge and transverse contraction joint grooves
            let sw_w = w * 0.72;
            let sw_h = h * 0.44;
            let curb_drop = 5.5;
            let top_rect = Rect::from_min_max(
                Pos2::new(c.x - sw_w * 0.5, c.y - sw_h * 0.5),
                Pos2::new(c.x + sw_w * 0.5, c.y + sw_h * 0.5 - curb_drop),
            );

            // Top concrete surface
            painter.rect_filled(top_rect, 1.0, color.gamma_multiply(0.20));
            painter.rect_stroke(top_rect, 1.0, stroke, egui::StrokeKind::Middle);

            // Vertical curb face at bottom edge
            let curb_pts = vec![
                top_rect.left_bottom(),
                top_rect.right_bottom(),
                Pos2::new(top_rect.max.x, top_rect.max.y + curb_drop),
                Pos2::new(top_rect.min.x, top_rect.max.y + curb_drop),
            ];
            painter.add(egui::Shape::convex_polygon(curb_pts, color.gamma_multiply(0.40), stroke));

            // Rounded Bullnose Edge Highlight
            let bullnose_s = Stroke::new(2.0, Color32::from_rgb(0, 200, 255));
            painter.line_segment([top_rect.left_bottom(), top_rect.right_bottom()], bullnose_s);

            // Transverse Contraction Joint Top Grooves (3 grooves across width)
            let s_groove = Stroke::new(1.0, color.gamma_multiply(0.70));
            for i in 1..=3 {
                let gx = top_rect.min.x + (top_rect.width() / 4.0) * (i as f32);
                painter.line_segment([Pos2::new(gx, top_rect.min.y), Pos2::new(gx, top_rect.max.y)], s_groove);
                // Groove continues down curb face
                painter.line_segment([Pos2::new(gx, top_rect.max.y), Pos2::new(gx, top_rect.max.y + curb_drop)], s_groove);
            }
        }
        IconKind::Pathwalk => {
            // Interlocking stone paver pedestrian path flanked by lateral edging stones
            let pw_w = w * 0.70;
            let pw_h = h * 0.60;
            let r = Rect::from_center_size(c, Vec2::new(pw_w, pw_h));

            // Path base fill
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.18));
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);

            // Lateral concrete/brick border edgings (left and right)
            let edge_w = 4.0;
            let l_edge = Rect::from_min_max(r.min, Pos2::new(r.min.x + edge_w, r.max.y));
            let r_edge = Rect::from_min_max(Pos2::new(r.max.x - edge_w, r.min.y), r.max);
            painter.rect_filled(l_edge, 0.0, color.gamma_multiply(0.35));
            painter.rect_filled(r_edge, 0.0, color.gamma_multiply(0.35));
            painter.rect_stroke(l_edge, 0.0, thin_stroke, egui::StrokeKind::Middle);
            painter.rect_stroke(r_edge, 0.0, thin_stroke, egui::StrokeKind::Middle);

            // Running-bond interlocking paver blocks
            let paver_w = (r.width() - edge_w * 2.0) / 2.0;
            let paver_h = r.height() / 4.0;
            let s_joint = Stroke::new(1.0, color.gamma_multiply(0.65));

            for row in 0..4 {
                let y = r.min.y + row as f32 * paver_h;
                if row > 0 {
                    painter.line_segment([Pos2::new(r.min.x + edge_w, y), Pos2::new(r.max.x - edge_w, y)], s_joint);
                }
                let shift = if row % 2 == 1 { paver_w * 0.5 } else { 0.0 };
                let x = r.min.x + edge_w + shift;
                if x < r.max.x - edge_w {
                    painter.line_segment([Pos2::new(x, y), Pos2::new(x, y + paver_h)], s_joint);
                }
                if row % 2 == 1 && x - paver_w > r.min.x + edge_w {
                    painter.line_segment([Pos2::new(x - paver_w, y), Pos2::new(x - paver_w, y + paver_h)], s_joint);
                }
            }
        }
        IconKind::ProjectTree => {
            // Stylized hierarchical branch tree node structure
            // Root stem on left
            let x_stem = c.x - w * 0.28;
            let y_top = c.y - h * 0.32;
            let y_mid = c.y;
            let y_bot = c.y + h * 0.32;
            let x_leaf = c.x + w * 0.28;

            // Vertical backbone stem
            painter.line_segment([Pos2::new(x_stem, y_top), Pos2::new(x_stem, y_bot)], stroke);

            // Horizontal branches to tree nodes
            painter.line_segment([Pos2::new(x_stem, y_top), Pos2::new(x_leaf, y_top)], thin_stroke);
            painter.line_segment([Pos2::new(x_stem, y_mid), Pos2::new(x_leaf, y_mid)], thin_stroke);
            painter.line_segment([Pos2::new(x_stem, y_bot), Pos2::new(x_leaf, y_bot)], thin_stroke);

            // Circular node pills at branch ends
            let node_r = 2.2;
            let fill_col = color.gamma_multiply(0.3);
            for y_node in [y_top, y_mid, y_bot] {
                painter.circle_filled(Pos2::new(x_leaf, y_node), node_r, fill_col);
                painter.circle_stroke(Pos2::new(x_leaf, y_node), node_r, thin_stroke);
            }
            // Root node anchor
            painter.circle_filled(Pos2::new(x_stem, y_mid), 2.5, color);
        }
        IconKind::Blocks => {
            // Isometric architectural component blocks / family cubes stacked in 3D
            let cube_size = w * 0.28;
            // Lower left block
            let r1 = Rect::from_center_size(Pos2::new(c.x - w * 0.16, c.y + h * 0.14), egui::vec2(cube_size, cube_size));
            painter.rect_filled(r1, 1.5, color.gamma_multiply(0.22));
            painter.rect_stroke(r1, 1.5, stroke, egui::StrokeKind::Middle);

            // Lower right block
            let r2 = Rect::from_center_size(Pos2::new(c.x + w * 0.18, c.y + h * 0.14), egui::vec2(cube_size, cube_size));
            painter.rect_filled(r2, 1.5, color.gamma_multiply(0.22));
            painter.rect_stroke(r2, 1.5, stroke, egui::StrokeKind::Middle);

            // Top centered block (creating stepped block assembly)
            let r3 = Rect::from_center_size(Pos2::new(c.x + w * 0.01, c.y - h * 0.18), egui::vec2(cube_size, cube_size));
            painter.rect_filled(r3, 1.5, color.gamma_multiply(0.35));
            painter.rect_stroke(r3, 1.5, stroke, egui::StrokeKind::Middle);

            // Inner cross accents indicating parametric components
            painter.line_segment([Pos2::new(r3.min.x + 3.0, r3.center().y), Pos2::new(r3.max.x - 3.0, r3.center().y)], thin_stroke);
            painter.line_segment([Pos2::new(r3.center().x, r3.min.y + 3.0), Pos2::new(r3.center().x, r3.max.y - 3.0)], thin_stroke);
        }
        IconKind::SearchFilter => {
            // Sleek vector magnifying glass search lens with diagonal handle
            let lens_c = Pos2::new(c.x - w * 0.10, c.y - h * 0.10);
            let lens_r = w * 0.24;
            painter.circle_stroke(lens_c, lens_r, stroke);
            painter.circle_filled(lens_c, lens_r - 0.5, color.gamma_multiply(0.15));

            // 45-degree angled handle
            let h_start = Pos2::new(lens_c.x + lens_r * 0.707, lens_c.y + lens_r * 0.707);
            let h_end = Pos2::new(c.x + w * 0.35, c.y + h * 0.35);
            painter.line_segment([h_start, h_end], Stroke::new(2.4, color));
        }
        IconKind::FloorPlanView => {
            // Authentic architectural floor plan blueprint symbol:
            // 2D building partition layout with exterior walls, internal room wall, and door opening swing
            let r = Rect::from_center_size(c, egui::vec2(w * 0.76, h * 0.76));
            
            // Exterior boundary walls
            let wall_stroke = Stroke::new(1.8, color);
            painter.rect_stroke(r, 0.0, wall_stroke, egui::StrokeKind::Middle);
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.08));

            // Interior partition wall dividing rooms horizontally
            let y_part = r.min.y + r.height() * 0.52;
            let door_open_x = r.min.x + r.width() * 0.48;
            painter.line_segment([Pos2::new(r.min.x, y_part), Pos2::new(door_open_x, y_part)], wall_stroke);

            // Door opening quarter-circle swing arc + door leaf line
            let door_w = r.width() * 0.30;
            let hinge = Pos2::new(door_open_x, y_part);
            painter.line_segment([hinge, Pos2::new(hinge.x, hinge.y - door_w)], thin_stroke);
            
            // Door swing arc
            let arc_steps = 6;
            for i in 0..arc_steps {
                let a0 = -(std::f32::consts::FRAC_PI_2 * (i as f32) / (arc_steps as f32));
                let a1 = -(std::f32::consts::FRAC_PI_2 * ((i + 1) as f32) / (arc_steps as f32));
                let p0 = Pos2::new(hinge.x + door_w * a0.cos(), hinge.y + door_w * a0.sin());
                let p1 = Pos2::new(hinge.x + door_w * a1.cos(), hinge.y + door_w * a1.sin());
                painter.line_segment([p0, p1], Stroke::new(1.0, color.gamma_multiply(0.65)));
            }
        }
        IconKind::SectionView => {
            // Authentic architectural building section cut symbol:
            // Heavy dashed/solid cut plane line through a building profile with directional view arrow head
            let cut_x = c.x - w * 0.05;
            let y_top = c.y - h * 0.38;
            let y_bot = c.y + h * 0.38;

            // Vertical section cutting plane line
            painter.line_segment([Pos2::new(cut_x, y_top), Pos2::new(cut_x, y_bot)], Stroke::new(2.0, color));

            // Circular section head bubble at top
            let bub_r = w * 0.16;
            let bub_c = Pos2::new(cut_x, y_top);
            painter.circle_filled(bub_c, bub_r, color.gamma_multiply(0.20));
            painter.circle_stroke(bub_c, bub_r, stroke);

            // Directional arrow head pointing right (view direction)
            let arr_tip = Pos2::new(cut_x + w * 0.34, y_top);
            let arr_top = Pos2::new(cut_x + bub_r * 0.7, y_top - h * 0.14);
            let arr_bot = Pos2::new(cut_x + bub_r * 0.7, y_top + h * 0.14);
            painter.line_segment([arr_tip, arr_top], Stroke::new(1.8, color));
            painter.line_segment([arr_tip, arr_bot], Stroke::new(1.8, color));
            painter.line_segment([arr_top, arr_bot], thin_stroke);

            // Cut hash marks at bottom of section line
            painter.line_segment(
                [Pos2::new(cut_x - 3.5, y_bot - 2.0), Pos2::new(cut_x + 3.5, y_bot + 2.0)],
                Stroke::new(1.8, color),
            );
        }
        IconKind::ElevationView => {
            // Architectural building facade elevation symbol:
            // Multi-storey exterior facade with pitched roofline, wall borders, and window openings
            let r = Rect::from_min_max(Pos2::new(c.x - w * 0.36, c.y - h * 0.08), Pos2::new(c.x + w * 0.36, c.y + h * 0.38));
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
            painter.rect_filled(r, 0.0, color.gamma_multiply(0.12));

            // Pitched roof pediment / gable outline
            let ridge = Pos2::new(c.x, c.y - h * 0.38);
            painter.line_segment([ridge, r.left_top()], stroke);
            painter.line_segment([ridge, r.right_top()], stroke);

            // Facade window punctures (2 small windows)
            let win_w = w * 0.16;
            let win_h = h * 0.18;
            let w1 = Rect::from_center_size(Pos2::new(r.min.x + r.width() * 0.30, r.center().y), egui::vec2(win_w, win_h));
            let w2 = Rect::from_center_size(Pos2::new(r.max.x - r.width() * 0.30, r.center().y), egui::vec2(win_w, win_h));
            painter.rect_stroke(w1, 0.0, thin_stroke, egui::StrokeKind::Middle);
            painter.rect_stroke(w2, 0.0, thin_stroke, egui::StrokeKind::Middle);
        }
    }
}

/// Analytical distance from 2D point (px, py) to segment (x1, y1)-(x2, y2).
fn dist_to_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-4 {
        let rx = px - x1;
        let ry = py - y1;
        return (rx * rx + ry * ry).sqrt();
    }
    let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    let rx = px - proj_x;
    let ry = py - proj_y;
    (rx * rx + ry * ry).sqrt()
}

/// Analytical distance from 2D point (px, py) to circular arc.
fn dist_to_arc(px: f32, py: f32, cx: f32, cy: f32, radius: f32, a_start: f32, a_end: f32) -> f32 {
    let dx = px - cx;
    let dy = py - cy;
    let dist_c = (dx * dx + dy * dy).sqrt();
    let mut angle = dy.atan2(dx);
    if angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    let norm_start = if a_start < 0.0 { a_start + std::f32::consts::TAU } else { a_start };
    let norm_end = if a_end < 0.0 { a_end + std::f32::consts::TAU } else { a_end };

    let in_arc = if norm_start <= norm_end {
        angle >= norm_start && angle <= norm_end
    } else {
        angle >= norm_start || angle <= norm_end
    };

    if in_arc {
        (dist_c - radius).abs()
    } else {
        let p1x = cx + radius * a_start.cos();
        let p1y = cy + radius * a_start.sin();
        let p2x = cx + radius * a_end.cos();
        let p2y = cy + radius * a_end.sin();
        let d1 = ((px - p1x) * (px - p1x) + (py - p1y) * (py - p1y)).sqrt();
        let d2 = ((px - p2x) * (px - p2x) + (py - p2y) * (py - p2y)).sqrt();
        d1.min(d2)
    }
}

/// Generates a crisp RGBA pixel buffer of the Arch3D application icon at any specified dimension.
pub fn generate_arch3d_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    let scale = s / 128.0;

    let cx = s * 0.5;
    let cy = s * 0.5;
    let half_w = (s * 0.5) - 4.0 * scale;
    let half_h = (s * 0.5) - 4.0 * scale;
    let corner_r = 24.0 * scale;

    for y in 0..size {
        let py = y as f32 + 0.5;
        for x in 0..size {
            let px = x as f32 + 0.5;
            let idx = ((y * size + x) * 4) as usize;

            // 1. Distance to Squircle Background Tile
            let qx = (px - cx).abs() - (half_w - corner_r);
            let qy = (py - cy).abs() - (half_h - corner_r);
            let ext_x = qx.max(0.0);
            let ext_y = qy.max(0.0);
            let ext_dist = (ext_x * ext_x + ext_y * ext_y).sqrt();
            let int_dist = qx.max(qy).min(0.0);
            let dist_squircle = ext_dist + int_dist - corner_r;

            if dist_squircle > 1.0 {
                continue;
            }

            let tile_alpha = (1.0 - dist_squircle).clamp(0.0, 1.0);

            // Base Background Gradient (#121927 to #0A0E18)
            let grad = (py / s).clamp(0.0, 1.0);
            let mut r = (18.0 * (1.0 - grad) + 10.0 * grad) as f32;
            let mut g = (25.0 * (1.0 - grad) + 14.0 * grad) as f32;
            let mut b = (39.0 * (1.0 - grad) + 24.0 * grad) as f32;

            // Subtle inner rim stroke
            if dist_squircle >= -2.0 * scale {
                let rim_factor = ((dist_squircle + 2.0 * scale) / (2.0 * scale)).clamp(0.0, 1.0);
                r = r * (1.0 - rim_factor * 0.5) + 32.0 * rim_factor * 0.5;
                g = g * (1.0 - rim_factor * 0.5) + 50.0 * rim_factor * 0.5;
                b = b * (1.0 - rim_factor * 0.5) + 76.0 * rim_factor * 0.5;
            }

            // 2. Architectural Blueprint Reticle & Crosshair (Watermark)
            let dist_center = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();
            let reticle_r = 44.0 * scale;
            let circle_dist = (dist_center - reticle_r).abs();
            if circle_dist < 1.0 * scale {
                let a = (1.0 - circle_dist / (1.0 * scale)) * 0.22;
                r = r * (1.0 - a) + 0.0 * a;
                g = g * (1.0 - a) + 180.0 * a;
                b = b * (1.0 - a) + 240.0 * a;
            }
            if dist_center <= reticle_r + 6.0 * scale {
                let cross_x = (py - cy).abs();
                let cross_y = (px - cx).abs();
                if cross_x < 0.8 * scale {
                    let a = (1.0 - cross_x / (0.8 * scale)) * 0.18;
                    r = r * (1.0 - a) + 0.0 * a;
                    g = g * (1.0 - a) + 180.0 * a;
                    b = b * (1.0 - a) + 240.0 * a;
                }
                if cross_y < 0.8 * scale {
                    let a = (1.0 - cross_y / (0.8 * scale)) * 0.18;
                    r = r * (1.0 - a) + 0.0 * a;
                    g = g * (1.0 - a) + 180.0 * a;
                    b = b * (1.0 - a) + 240.0 * a;
                }
            }

            // 3. "ARCH" Strokes (White)
            let mut min_arch_dist = 999.0f32;
            let a1 = dist_to_segment(px, py, 26.0 * scale, 46.0 * scale, 34.0 * scale, 24.0 * scale);
            let a2 = dist_to_segment(px, py, 42.0 * scale, 46.0 * scale, 34.0 * scale, 24.0 * scale);
            let a3 = dist_to_segment(px, py, 29.0 * scale, 38.0 * scale, 39.0 * scale, 38.0 * scale);
            min_arch_dist = min_arch_dist.min(a1).min(a2).min(a3);

            let r1 = dist_to_segment(px, py, 48.0 * scale, 24.0 * scale, 48.0 * scale, 46.0 * scale);
            let r2 = dist_to_segment(px, py, 48.0 * scale, 24.0 * scale, 57.0 * scale, 24.0 * scale);
            let r_arc = dist_to_arc(px, py, 57.0 * scale, 29.0 * scale, 5.0 * scale, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
            let r3 = dist_to_segment(px, py, 48.0 * scale, 34.0 * scale, 57.0 * scale, 34.0 * scale);
            let r4 = dist_to_segment(px, py, 54.0 * scale, 34.0 * scale, 62.0 * scale, 46.0 * scale);
            min_arch_dist = min_arch_dist.min(r1).min(r2).min(r_arc).min(r3).min(r4);

            let c_arc = dist_to_arc(px, py, 75.0 * scale, 35.0 * scale, 9.5 * scale, 0.785, 5.498);
            min_arch_dist = min_arch_dist.min(c_arc);

            let h1 = dist_to_segment(px, py, 88.0 * scale, 24.0 * scale, 88.0 * scale, 46.0 * scale);
            let h2 = dist_to_segment(px, py, 100.0 * scale, 24.0 * scale, 100.0 * scale, 46.0 * scale);
            let h3 = dist_to_segment(px, py, 88.0 * scale, 35.0 * scale, 100.0 * scale, 35.0 * scale);
            min_arch_dist = min_arch_dist.min(h1).min(h2).min(h3);

            let stroke_r_arch = 1.9 * scale;
            if min_arch_dist <= stroke_r_arch + 1.0 {
                let a = ((stroke_r_arch + 1.0 - min_arch_dist) / 1.0).clamp(0.0, 1.0);
                r = r * (1.0 - a) + 255.0 * a;
                g = g * (1.0 - a) + 255.0 * a;
                b = b * (1.0 - a) + 255.0 * a;
            }

            // 4. Accent Divider Rule (Cyan) with terminal notches
            let div_line = dist_to_segment(px, py, 26.0 * scale, 56.0 * scale, 102.0 * scale, 56.0 * scale);
            let tick_l = dist_to_segment(px, py, 26.0 * scale, 54.0 * scale, 26.0 * scale, 58.0 * scale);
            let tick_r = dist_to_segment(px, py, 102.0 * scale, 54.0 * scale, 102.0 * scale, 58.0 * scale);
            let div_dist = div_line.min(tick_l).min(tick_r);
            if div_dist <= 1.0 * scale + 1.0 {
                let a = ((1.0 * scale + 1.0 - div_dist) / 1.0).clamp(0.0, 1.0);
                r = r * (1.0 - a) + 0.0 * a;
                g = g * (1.0 - a) + 190.0 * a;
                b = b * (1.0 - a) + 255.0 * a;
            }

            // 5. "3D" Strokes (Electric Cyan-Blue with Soft Neon Glow)
            let mut min_3d_dist = 999.0f32;
            let d3_1 = dist_to_segment(px, py, 36.0 * scale, 70.0 * scale, 57.0 * scale, 70.0 * scale);
            let d3_2 = dist_to_segment(px, py, 57.0 * scale, 70.0 * scale, 48.0 * scale, 85.0 * scale);
            let d3_3 = dist_to_segment(px, py, 45.0 * scale, 85.0 * scale, 48.0 * scale, 85.0 * scale);
            let d3_arc = dist_to_arc(px, py, 46.0 * scale, 93.0 * scale, 11.5 * scale, -1.309, 2.356);
            min_3d_dist = min_3d_dist.min(d3_1).min(d3_2).min(d3_3).min(d3_arc);

            let dd_stem = dist_to_segment(px, py, 71.0 * scale, 70.0 * scale, 71.0 * scale, 104.0 * scale);
            let dd_top = dist_to_segment(px, py, 71.0 * scale, 70.0 * scale, 80.0 * scale, 70.0 * scale);
            let dd_bot = dist_to_segment(px, py, 71.0 * scale, 104.0 * scale, 80.0 * scale, 104.0 * scale);
            let dd_arc = dist_to_arc(px, py, 80.0 * scale, 87.0 * scale, 17.0 * scale, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
            min_3d_dist = min_3d_dist.min(dd_stem).min(dd_top).min(dd_bot).min(dd_arc);

            let stroke_r_3d = 3.2 * scale;
            let glow_radius = 8.5 * scale;
            if min_3d_dist < glow_radius {
                let glow_factor = ((glow_radius - min_3d_dist) / glow_radius).powi(2) * 0.45;
                r = r * (1.0 - glow_factor) + 0.0 * glow_factor;
                g = g * (1.0 - glow_factor) + 160.0 * glow_factor;
                b = b * (1.0 - glow_factor) + 255.0 * glow_factor;
            }

            if min_3d_dist <= stroke_r_3d + 1.0 {
                let a = ((stroke_r_3d + 1.0 - min_3d_dist) / 1.0).clamp(0.0, 1.0);
                r = r * (1.0 - a) + 0.0 * a;
                g = g * (1.0 - a) + 185.0 * a;
                b = b * (1.0 - a) + 255.0 * a;
            }

            rgba[idx] = r.clamp(0.0, 255.0) as u8;
            rgba[idx + 1] = g.clamp(0.0, 255.0) as u8;
            rgba[idx + 2] = b.clamp(0.0, 255.0) as u8;
            rgba[idx + 3] = (tile_alpha * 255.0).clamp(0.0, 255.0) as u8;
        }
    }

    rgba
}

/// Raw 128x128 32-bit RGBA pixel buffer generated from official Arch3D logo.
const ARCH3D_ICON_BYTES_128: &[u8] = include_bytes!("arch3d_icon_128.bin");

/// Produces the official Arch3D application icon data for eframe viewport window & taskbar.
pub fn arch3d_icon_data() -> egui::viewport::IconData {
    let size = 128u32;
    let rgba = ARCH3D_ICON_BYTES_128.to_vec();
    egui::viewport::IconData {
        rgba,
        width: size,
        height: size,
    }
}

/// Creates a standard binary Windows `.ico` byte buffer for the Arch3D application icon.
pub fn create_arch3d_ico_bytes(size: u32) -> Vec<u8> {
    let rgba = if size == 128 && ARCH3D_ICON_BYTES_128.len() == 128 * 128 * 4 {
        ARCH3D_ICON_BYTES_128.to_vec()
    } else {
        generate_arch3d_icon_rgba(size)
    };
    let mut ico = Vec::with_capacity(6 + 16 + 40 + (size * size * 4) as usize + (size * size / 8) as usize);

    // 1. ICONDIR (6 bytes)
    ico.extend_from_slice(&[0, 0]); // Reserved
    ico.extend_from_slice(&[1, 0]); // Type 1 = ICO
    ico.extend_from_slice(&[1, 0]); // 1 image

    let image_data_size = 40 + (size * size * 4) + (size * size / 8);
    let offset = 6 + 16; // 22 bytes

    // 2. ICONDIRENTRY (16 bytes)
    ico.push(if size >= 256 { 0 } else { size as u8 });
    ico.push(if size >= 256 { 0 } else { size as u8 });
    ico.push(0); // Palette color count (0 for 32-bit)
    ico.push(0); // Reserved
    ico.extend_from_slice(&[1, 0]); // Planes
    ico.extend_from_slice(&[32, 0]); // 32 bpp
    ico.extend_from_slice(&(image_data_size as u32).to_le_bytes());
    ico.extend_from_slice(&(offset as u32).to_le_bytes());

    // 3. BITMAPINFOHEADER (40 bytes)
    ico.extend_from_slice(&40u32.to_le_bytes());
    ico.extend_from_slice(&(size as i32).to_le_bytes());
    ico.extend_from_slice(&((size * 2) as i32).to_le_bytes()); // Height doubled for XOR + AND masks
    ico.extend_from_slice(&1u16.to_le_bytes()); // Planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // BitCount
    ico.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (uncompressed)
    ico.extend_from_slice(&((size * size * 4) as u32).to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes()); // biXPelsPerMeter
    ico.extend_from_slice(&0u32.to_le_bytes()); // biYPelsPerMeter
    ico.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    ico.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    // 4. Pixel data: bottom-to-top rows, BGRA format
    for y in (0..size).rev() {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            ico.push(b);
            ico.push(g);
            ico.push(r);
            ico.push(a);
        }
    }

    // 5. 1-bit AND mask: (size * size / 8) bytes
    let mask_bytes = (size * size / 8) as usize;
    ico.extend(vec![0u8; mask_bytes]);

    ico
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch3d_icon_generation() {
        let icon = arch3d_icon_data();
        assert_eq!(icon.width, 128);
        assert_eq!(icon.height, 128);
        assert_eq!(icon.rgba.len(), 128 * 128 * 4);

        // Verify center pixel is not empty
        let center_idx = (64 * 128 + 64) * 4;
        let a = icon.rgba[center_idx + 3];
        assert!(a > 0, "Icon center must be opaque");
    }

    #[test]
    fn test_arch3d_ico_bytes_generation() {
        let ico = create_arch3d_ico_bytes(64);
        assert!(ico.len() > 22 + 40);
        // Verify ICO header: reserved=0, type=1, count=1
        assert_eq!(&ico[0..6], &[0, 0, 1, 0, 1, 0]);
        // Verify width and height
        assert_eq!(ico[6], 64);
        assert_eq!(ico[7], 64);

        // Generate arch3d.ico in both project root and workspace root
        let _ = std::fs::write("arch3d.ico", &ico);
        let _ = std::fs::write("../arch3d.ico", &ico);
        let _ = std::fs::write("../../arch3d.ico", &ico);
        let _ = std::fs::write("d:/mymusicapp/plinth-bim/arch3d.ico", &ico);
        let _ = std::fs::write("d:/mymusicapp/arch3d.ico", &ico);
    }

    #[test]
    fn test_blender_live_sync_icon_render() {
        let painter = Painter::new(
            egui::Context::default(),
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0)),
        );
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(32.0, 32.0));
        draw_icon(&painter, rect, IconKind::BlenderLiveSync, Color32::from_rgb(118, 118, 118));
    }
}
