//! Viewport canvas: 2D plan / elevation / section and 3D interactive graphics, pan/zoom, grid snapping, and tool interactions.

use crate::theme::AppTheme;
use crate::tool::Tool;
use crate::tool_config::ToolConfigStudio;
use egui::{Color32, Key, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use plinth_geom::{Xform2, Vec2 as ModelVec2};
use plinth_model::{
    Beam, BuildingPad, Ceiling, Column, CurtainWall, DetailLine, Dimension, Document, Door, Element, ElementId, Footing,
    Pathwalk, PropertyLine, Railing, Ramp, Road, Sidewalk, Slab, Stair, Toposolid, ViewKind, Wall, Window,
};
use plinth_views::{RenderPrimitive, VisualStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportLayout {
    Single,
    SplitHorizontal,
    SplitVertical,
    SplitTriple,
    SplitQuad,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SheetVpDragMode {
    Move,
    Resize {
        left: bool,
        bottom: bool,
        right: bool,
        top: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaneState {
    pub view_id: Option<plinth_model::ViewId>,
    pub pan: Vec2,
    pub zoom: f32,
    pub visual_style: VisualStyle,
    pub style_menu_open: bool,
    pub view_picker_open: bool,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            view_id: None,
            pan: Vec2::ZERO,
            zoom: 0.05,
            visual_style: VisualStyle::Shaded,
            style_menu_open: false,
            view_picker_open: false,
        }
    }
}

pub use crate::settings::SnapSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapKind {
    Endpoint,
    Midpoint,
    Center,
    Intersection,
    Perpendicular,
    PolarRay,
    OrthoRay,
    Grid,
    Edge,
    WallCenterline,
}

#[derive(Clone, Debug)]
pub struct ActiveSnapVisual {
    pub kind: SnapKind,
    pub screen_pos: Pos2,
    pub world_pos: ModelVec2,
    pub label: String,
    pub ray_start: Option<Pos2>,
    pub ray_end: Option<Pos2>,
    pub ray_angle_deg: Option<f64>,
    pub ray_dist_mm: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorkplaneMode {
    XY,
    XZ,
    YZ,
    Surface {
        origin: plinth_geom::Vec3,
        normal: plinth_geom::Vec3,
        u_axis: plinth_geom::Vec3,
        v_axis: plinth_geom::Vec3,
        host_element: Option<plinth_model::ElementId>,
    },
}

impl Default for WorkplaneMode {
    fn default() -> Self {
        Self::XY
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionBox {
    pub start: Pos2,
    pub current: Pos2,
}

impl SelectionBox {
    /// Right-to-Left drag is Crossing Selection (green, dashed border).
    /// Left-to-Right drag is Window Selection (blue, solid border).
    pub fn is_crossing(&self) -> bool {
        self.current.x < self.start.x
    }

    pub fn rect(&self) -> Rect {
        Rect::from_two_pos(self.start, self.current)
    }
}

pub struct CanvasState {
    pub pan: Vec2,
    pub zoom: f32, // screen pixels per world mm
    pub sheet_pan: Vec2,
    pub sheet_zoom: f32, // screen pixels per paper mm (0.0 triggers initial auto-fit)
    pub snapped_world_pos: ModelVec2,
    pub visual_style: VisualStyle,
    pub style_menu_open: bool,
    pub layout: ViewportLayout,
    pub split_x: f32,
    pub split_y: f32,
    pub is_dragging_split: Option<SplitAxis>,
    pub selected_placed_viewport: Option<plinth_model::ViewportId>,
    pub dragging_sheet_viewport: Option<(plinth_model::ViewportId, SheetVpDragMode)>,

    // Snapping & Drafting Precision
    pub snap_settings: SnapSettings,
    pub active_snap: Option<ActiveSnapVisual>,
    pub snapped_3d_pos: Option<plinth_geom::Vec3>,
    pub move_base_3d: Option<plinth_geom::Vec3>,

    // Multi-viewport independent state
    pub active_pane_idx: usize,
    pub panes: [PaneState; 4],
    pub gpu_renderer: Option<std::sync::Arc<crate::gpu_viewport::GpuViewportRenderer>>,

    // Arch3D Signature Pastel Materials & Freeform Workplane
    pub materials: plinth_views::MaterialPalette,
    pub workplane: WorkplaneMode,
    pub picking_surface_workplane: bool,
    pub surface_action_popup: Option<plinth_model::ElementId>,

    // Window and Crossing Drag Selection Box & Multi-Selection
    pub selection_box: Option<SelectionBox>,
    pub selected_elements: Vec<plinth_model::ElementId>,
    pub active_annotation_grip: Option<(plinth_model::ElementId, usize)>,

    // Active annotation drafting style settings
    pub annotation_settings: crate::settings::AnnotationSettings,

    // High-Performance GPU Scene Cache (Guarantees lag-free 120+ FPS on 10,000+ element models)
    pub cached_gpu_scene: Option<crate::gpu_viewport::GpuScene>,
    pub cached_gpu_elements_count: usize,
    pub cached_gpu_doc_undo_count: usize,
    pub cached_gpu_visual_style: VisualStyle,
    pub cached_gpu_is_dark: bool,
    pub cached_gpu_selection: Vec<plinth_model::ElementId>,
}

impl CanvasState {
    pub fn invalidate_gpu_cache(&mut self) {
        self.cached_gpu_scene = None;
        self.cached_gpu_elements_count = 0;
    }
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 0.05, // e.g. 50 pixels per 1000 mm (1 meter)
            sheet_pan: Vec2::ZERO,
            sheet_zoom: 0.0, // auto-fits active sheet on first view
            snapped_world_pos: ModelVec2::ZERO,
            visual_style: VisualStyle::Shaded,
            style_menu_open: false,
            layout: ViewportLayout::Single,
            split_x: 0.5,
            split_y: 0.5,
            is_dragging_split: None,
            selected_placed_viewport: None,
            dragging_sheet_viewport: None,
            snap_settings: SnapSettings::default(),
            active_snap: None,
            snapped_3d_pos: None,
            move_base_3d: None,
            active_pane_idx: 0,
            gpu_renderer: None,
            panes: [
                PaneState {
                    view_id: None,
                    pan: Vec2::ZERO,
                    zoom: 0.05,
                    visual_style: VisualStyle::HiddenLine,
                    style_menu_open: false,
                    view_picker_open: false,
                },
                PaneState {
                    view_id: None,
                    pan: Vec2::ZERO,
                    zoom: 0.04,
                    visual_style: VisualStyle::Shaded,
                    style_menu_open: false,
                    view_picker_open: false,
                },
                PaneState {
                    view_id: None,
                    pan: Vec2::ZERO,
                    zoom: 0.05,
                    visual_style: VisualStyle::ConsistentColors,
                    style_menu_open: false,
                    view_picker_open: false,
                },
                PaneState {
                    view_id: None,
                    pan: Vec2::ZERO,
                    zoom: 0.05,
                    visual_style: VisualStyle::HiddenLine,
                    style_menu_open: false,
                    view_picker_open: false,
                },
            ],
            materials: plinth_views::MaterialPalette::signature_pastel(),
            workplane: WorkplaneMode::XY,
            picking_surface_workplane: false,
            surface_action_popup: None,
            active_annotation_grip: None,
            selection_box: None,
            selected_elements: Vec::new(),
            annotation_settings: crate::settings::AnnotationSettings::default(),
            cached_gpu_scene: None,
            cached_gpu_elements_count: 0,
            cached_gpu_doc_undo_count: 0,
            cached_gpu_visual_style: VisualStyle::Shaded,
            cached_gpu_is_dark: false,
            cached_gpu_selection: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ViewportSlot {
    pane_idx: usize,
    rect: Rect,
}

fn compute_viewport_slots(rect: Rect, layout: ViewportLayout, split_x: f32, split_y: f32) -> Vec<ViewportSlot> {
    let sx = split_x.clamp(0.15, 0.85);
    let sy = split_y.clamp(0.15, 0.85);
    let x_s = rect.min.x + rect.width() * sx;
    let y_s = rect.min.y + rect.height() * sy;
    let gap = 2.0;

    match layout {
        ViewportLayout::Single => vec![
            ViewportSlot { pane_idx: 0, rect },
        ],
        ViewportLayout::SplitHorizontal => vec![
            ViewportSlot { pane_idx: 0, rect: Rect::from_min_max(rect.min, Pos2::new(rect.max.x, y_s - gap)) },
            ViewportSlot { pane_idx: 1, rect: Rect::from_min_max(Pos2::new(rect.min.x, y_s + gap), rect.max) },
        ],
        ViewportLayout::SplitVertical => vec![
            ViewportSlot { pane_idx: 0, rect: Rect::from_min_max(rect.min, Pos2::new(x_s - gap, rect.max.y)) },
            ViewportSlot { pane_idx: 1, rect: Rect::from_min_max(Pos2::new(x_s + gap, rect.min.y), rect.max) },
        ],
        ViewportLayout::SplitTriple => vec![
            ViewportSlot { pane_idx: 0, rect: Rect::from_min_max(rect.min, Pos2::new(x_s - gap, rect.max.y)) },
            ViewportSlot { pane_idx: 1, rect: Rect::from_min_max(Pos2::new(x_s + gap, rect.min.y), Pos2::new(rect.max.x, y_s - gap)) },
            ViewportSlot { pane_idx: 2, rect: Rect::from_min_max(Pos2::new(x_s + gap, y_s + gap), rect.max) },
        ],
        ViewportLayout::SplitQuad => vec![
            ViewportSlot { pane_idx: 0, rect: Rect::from_min_max(rect.min, Pos2::new(x_s - gap, y_s - gap)) },
            ViewportSlot { pane_idx: 1, rect: Rect::from_min_max(Pos2::new(x_s + gap, rect.min.y), Pos2::new(rect.max.x, y_s - gap)) },
            ViewportSlot { pane_idx: 2, rect: Rect::from_min_max(Pos2::new(rect.min.x, y_s + gap), Pos2::new(x_s - gap, rect.max.y)) },
            ViewportSlot { pane_idx: 3, rect: Rect::from_min_max(Pos2::new(x_s + gap, y_s + gap), rect.max) },
        ],
    }
}

fn resolve_pane_view(
    doc: &Document,
    pane_idx: usize,
    current_id: Option<plinth_model::ViewId>,
) -> plinth_model::ViewId {
    if let Some(id) = current_id {
        if doc.views.contains_key(&id) {
            return id;
        }
    }

    match pane_idx {
        0 => {
            if doc.views.contains_key(&doc.active_view_id) {
                doc.active_view_id
            } else if let Some(v) = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::FloorPlan) {
                v.id
            } else {
                doc.views.keys().next().copied().unwrap_or(doc.active_view_id)
            }
        }
        1 => {
            // Prefer 3D View
            if let Some(v) = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ThreeD) {
                v.id
            } else if let Some(v) = doc.views.values().find(|v| v.id != doc.active_view_id) {
                v.id
            } else {
                doc.views.keys().next().copied().unwrap_or(doc.active_view_id)
            }
        }
        2 => {
            // Prefer Elevation South or any elevation
            if let Some(v) = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ElevationSouth) {
                v.id
            } else if let Some(v) = doc.views.values().find(|v| v.kind.is_elevation_or_section()) {
                v.id
            } else {
                doc.views.keys().next().copied().unwrap_or(doc.active_view_id)
            }
        }
        3 => {
            // Prefer Section or another elevation
            if let Some(v) = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::Section) {
                v.id
            } else if let Some(v) = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ElevationNorth || v.kind == plinth_model::ViewKind::ElevationWest) {
                v.id
            } else {
                doc.views.keys().next().copied().unwrap_or(doc.active_view_id)
            }
        }
        _ => doc.views.keys().next().copied().unwrap_or(doc.active_view_id),
    }
}

pub fn render_canvas(
    ui: &mut Ui,
    doc: &mut Document,
    canvas_state: &mut CanvasState,
    active_tool: &mut Tool,
    selected_element: &mut Option<ElementId>,
    tool_studio: &ToolConfigStudio,
    theme: &AppTheme,
) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // If active sheet is set, render the Sheet Layout Canvas
    if let Some(active_sheet_id) = doc.active_sheet_id {
        render_sheet_layout_canvas(ui, rect, &response, &painter, doc, active_sheet_id, canvas_state, theme);
        return;
    }

    let slots = compute_viewport_slots(rect, canvas_state.layout, canvas_state.split_x, canvas_state.split_y);
    let total_panes = slots.len();

    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()).or(i.pointer.latest_pos()));
    let mouse_pos = pointer_pos.unwrap_or(Pos2::ZERO);
    let is_mmb = ui.input(|i| i.pointer.button_down(PointerButton::Middle));
    let is_rmb = ui.input(|i| i.pointer.button_down(PointerButton::Secondary));
    let is_dragging_split = canvas_state.is_dragging_split.is_some();
    let primary_clicked = response.clicked_by(PointerButton::Primary)
        || (ui.input(|i| i.pointer.primary_clicked()) && !is_dragging_split && !is_mmb && !is_rmb);
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    let drag_delta = response.drag_delta();

    // 1. Resolve views for all slots
    for slot in &slots {
        let vid = resolve_pane_view(doc, slot.pane_idx, canvas_state.panes[slot.pane_idx].view_id);
        canvas_state.panes[slot.pane_idx].view_id = Some(vid);
    }

    // 2. Hovered slot detection & pane activation
    let hovered_slot_idx = slots.iter().position(|s| s.rect.contains(mouse_pos));
    let is_dragging_split = canvas_state.is_dragging_split.is_some();

    if !is_dragging_split && primary_clicked {
        if let Some(h_idx) = hovered_slot_idx {
            let p_idx = slots[h_idx].pane_idx;
            canvas_state.active_pane_idx = p_idx;
            if let Some(vid) = canvas_state.panes[p_idx].view_id {
                doc.active_view_id = vid;
            }
        }
    }

    if canvas_state.active_pane_idx >= 4 {
        canvas_state.active_pane_idx = 0;
    }

    // 3. Pan, Zoom & 3D Orbit inside hovered viewport
    if !is_dragging_split {
        if let Some(h_idx) = hovered_slot_idx {
            let p_idx = slots[h_idx].pane_idx;
            let s_rect = slots[h_idx].rect;

            if scroll.abs() > 0.001 {
                let factor = ((scroll * 0.0025).exp()).clamp(0.88, 1.14);
                let new_zoom = (canvas_state.panes[p_idx].zoom * factor).clamp(0.0005, 20.0);
                let center = s_rect.center();
                let screen_offset = mouse_pos - center - canvas_state.panes[p_idx].pan;
                canvas_state.panes[p_idx].pan -= screen_offset * (factor - 1.0);
                canvas_state.panes[p_idx].zoom = new_zoom;
            }

            if is_mmb {
                canvas_state.panes[p_idx].pan += drag_delta;
            }

            if is_rmb {
                if let Some(vid) = canvas_state.panes[p_idx].view_id {
                    if let Some(v_mut) = doc.views.get_mut(&vid) {
                        if v_mut.kind.is_3d() {
                            if let Some(cam) = &mut v_mut.camera_3d {
                                cam.yaw += (drag_delta.x as f64) * 0.008;
                                cam.pitch = (cam.pitch - (drag_delta.y as f64) * 0.008).clamp(-1.52, 1.52);
                            }
                        }
                    }
                }
            }
        }
    }

    // Sync legacy CanvasState fields for active pane
    let active_pane = canvas_state.active_pane_idx.min(3);
    canvas_state.pan = canvas_state.panes[active_pane].pan;
    canvas_state.zoom = canvas_state.panes[active_pane].zoom;
    canvas_state.visual_style = canvas_state.panes[active_pane].visual_style;

    // 4. Render each viewport slot
    for slot in &slots {
        let is_active = slot.pane_idx == canvas_state.active_pane_idx;
        render_single_viewport(
            ui,
            &painter,
            slot.rect,
            slot.pane_idx,
            is_active,
            total_panes,
            doc,
            canvas_state,
            active_tool,
            selected_element,
            tool_studio,
            theme,
            primary_clicked,
        );
    }

    // 5. Escape Key cancels active operation
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        match active_tool {
            Tool::DrawWall { start, .. } if start.is_some() => {
                *start = None;
            }
            Tool::DrawCurtainWall { start, .. } if start.is_some() => {
                *start = None;
            }
            Tool::DrawDetailLine { start } if start.is_some() => {
                *start = None;
            }
            Tool::DrawDimension { start, p2, locked_offset } => {
                if p2.is_some() {
                    *p2 = None;
                } else if locked_offset.is_some() {
                    *locked_offset = None;
                    *start = None;
                } else if start.is_some() {
                    *start = None;
                } else {
                    *active_tool = Tool::Select;
                    canvas_state.selected_elements.clear();
                    *selected_element = None;
                }
            }
            _ => {
                *active_tool = Tool::Select;
                canvas_state.selected_elements.clear();
                *selected_element = None;
            }
        }
    }

    // 6. Viewport Splitting Separators & Handle Grips (Interactive Resizing)
    if total_panes > 1 {
        render_split_viewport_overlays(
            &painter,
            ui,
            rect,
            canvas_state,
            theme,
            doc,
        );
    }
}

/// Renders a single dedicated viewport pane (Floor Plan, 3D View, Elevation, Section, etc.)
#[allow(clippy::too_many_arguments)]
fn render_single_viewport(
    ui: &mut Ui,
    painter: &egui::Painter,
    vp_rect: Rect,
    pane_idx: usize,
    is_active: bool,
    total_panes: usize,
    doc: &mut Document,
    canvas_state: &mut CanvasState,
    active_tool: &mut Tool,
    selected_element: &mut Option<ElementId>,
    tool_studio: &ToolConfigStudio,
    theme: &AppTheme,
    primary_clicked: bool,
) {
    let view_id = match canvas_state.panes[pane_idx].view_id {
        Some(v) => v,
        None => return,
    };
    let active_view = match doc.views.get(&view_id).cloned() {
        Some(v) => v,
        None => return,
    };

    let active_elev = doc.level_elevation(doc.active_level_id);
    let pane_visual_style = canvas_state.panes[pane_idx].visual_style;
    let (_view_level_z, cut_z) = if let Some(lid) = active_view.level_id {
        let lz = doc.level_elevation(lid);
        (lz, lz + active_view.cut_offset)
    } else {
        (0.0, 1200.0)
    };

    // Scissor clipped painter strictly bounded to this viewport pane
    let vp_painter = painter.with_clip_rect(vp_rect);
    vp_painter.rect_filled(vp_rect, 0.0, theme.canvas_bg);

    let center = vp_rect.center();
    let pan = canvas_state.panes[pane_idx].pan;
    let zoom = canvas_state.panes[pane_idx].zoom;

    let to_screen = |wp: ModelVec2| -> Pos2 {
        Pos2::new(
            center.x + pan.x + (wp.x as f32) * zoom,
            center.y + pan.y - (wp.y as f32) * zoom,
        )
    };

    let to_world_2d = |sp: Pos2| -> ModelVec2 {
        ModelVec2::new(
            ((sp.x - center.x - pan.x) / zoom) as f64,
            -((sp.y - center.y - pan.y) / zoom) as f64,
        )
    };

    let is_3d = active_view.kind.is_3d();
    let cam_3d = active_view.camera_3d.unwrap_or_default();
    let target_3d = plinth_geom::Vec3::new(cam_3d.target_x, cam_3d.target_y, cam_3d.target_z);
    let (sin_y, cos_y) = cam_3d.yaw.sin_cos();
    let (sin_p, cos_p) = cam_3d.pitch.sin_cos();
    let is_persp = cam_3d.fov_degrees > 1.0;
    let cam_dist = cam_3d.distance.max(1000.0);

    let project_3d = |p: plinth_geom::Vec3| -> Pos2 {
        let d = p - target_3d;
        let x1 = d.x * cos_y - d.y * sin_y;
        let y1 = d.x * sin_y + d.y * cos_y;
        let z1 = d.z;
        let x2 = x1;
        let y2 = y1 * cos_p - z1 * sin_p;
        let z2 = y1 * sin_p + z1 * cos_p;
        let v2 = if is_persp {
            let dist = (cam_dist + y2).max(100.0);
            let factor = cam_dist / dist;
            plinth_geom::vec2(x2 * factor, z2 * factor)
        } else {
            plinth_geom::vec2(x2, z2)
        };
        to_screen(v2)
    };

    let is_free_modeling_tool = matches!(
        active_tool,
        Tool::DrawRectSurface { .. }
            | Tool::DrawCircleSurface { .. }
            | Tool::DrawPolygonSurface { .. }
            | Tool::DrawBoxPrimitive { .. }
            | Tool::DrawCylinderPrimitive { .. }
            | Tool::DrawSpherePrimitive { .. }
            | Tool::PushPull { .. }
    );

    // Freeform workplane (Surface, XZ, YZ) ONLY affects Free Modeling tools in 3D views.
    // All architectural & structural tools, and all 2D views (Floor Plans, Ceiling Plans, Elevations, Sections)
    // strictly use standard project coordinates (XY level plane).
    let active_workplane = if is_3d && is_free_modeling_tool {
        canvas_state.workplane
    } else {
        WorkplaneMode::XY
    };

    let workplane_to_3d = |uv: ModelVec2, work_z: f64| -> plinth_geom::Vec3 {
        match active_workplane {
            WorkplaneMode::XY => plinth_geom::Vec3::new(uv.x, uv.y, work_z),
            WorkplaneMode::XZ => plinth_geom::Vec3::new(uv.x, target_3d.y, uv.y),
            WorkplaneMode::YZ => plinth_geom::Vec3::new(target_3d.x, uv.x, uv.y),
            WorkplaneMode::Surface { origin, u_axis, v_axis, .. } => {
                origin + u_axis * uv.x + v_axis * uv.y
            }
        }
    };

    let project_point = |p: ModelVec2, z: f64| -> Pos2 {
        if is_3d {
            project_3d(workplane_to_3d(p, z))
        } else {
            to_screen(p)
        }
    };

    let unproject_ray = |sp: Pos2| -> (plinth_geom::Vec3, plinth_geom::Vec3) {
        let v_x = ((sp.x - center.x - pan.x) / zoom) as f64;
        let v_z = -((sp.y - center.y - pan.y) / zoom) as f64;
        let cam_forward = plinth_geom::Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);
        if is_persp {
            let eye_world = target_3d - cam_forward * cam_dist;
            let dir_view = plinth_geom::Vec3::new(v_x, cam_dist, v_z).normalized();
            let x1 = dir_view.x;
            let y1 = dir_view.y * cos_p + dir_view.z * sin_p;
            let z1 = -dir_view.y * sin_p + dir_view.z * cos_p;
            let dx = x1 * cos_y + y1 * sin_y;
            let dy = -x1 * sin_y + y1 * cos_y;
            let ray_dir = plinth_geom::Vec3::new(dx, dy, z1).normalized();
            (eye_world, ray_dir)
        } else {
            let x1 = v_x;
            let y1 = v_z * sin_p;
            let z1 = v_z * cos_p;
            let dx = x1 * cos_y + y1 * sin_y;
            let dy = -x1 * sin_y + y1 * cos_y;
            let p_world = target_3d + plinth_geom::Vec3::new(dx, dy, z1);
            (p_world - cam_forward * 100_000.0, cam_forward)
        }
    };

    let unproject_screen_to_3d = |sp: Pos2, work_z: f64| -> plinth_geom::Vec3 {
        if !is_3d {
            let p2 = to_world_2d(sp);
            return plinth_geom::Vec3::new(p2.x, p2.y, work_z);
        }

        let (p0, norm) = match active_workplane {
            WorkplaneMode::XY => (plinth_geom::Vec3::new(0.0, 0.0, work_z), plinth_geom::Vec3::Z),
            WorkplaneMode::XZ => (plinth_geom::Vec3::new(0.0, target_3d.y, 0.0), plinth_geom::Vec3::Y),
            WorkplaneMode::YZ => (plinth_geom::Vec3::new(target_3d.x, 0.0, 0.0), plinth_geom::Vec3::X),
            WorkplaneMode::Surface { origin, normal, .. } => (origin, normal),
        };

        let (ray_origin, ray_dir) = unproject_ray(sp);
        let denom = ray_dir.dot(norm);
        if denom.abs() > 1e-6 {
            let t = (p0 - ray_origin).dot(norm) / denom;
            ray_origin + ray_dir * t
        } else {
            target_3d
        }
    };

    let unproject_screen_to_workplane = |sp: Pos2, work_z: f64| -> ModelVec2 {
        if !is_3d {
            return to_world_2d(sp);
        }
        let p3 = unproject_screen_to_3d(sp, work_z);
        match active_workplane {
            WorkplaneMode::XY => plinth_geom::vec2(p3.x, p3.y),
            WorkplaneMode::XZ => plinth_geom::vec2(p3.x, p3.z),
            WorkplaneMode::YZ => plinth_geom::vec2(p3.y, p3.z),
            WorkplaneMode::Surface { origin, u_axis, v_axis, .. } => {
                let rel = p3 - origin;
                plinth_geom::vec2(rel.dot(u_axis), rel.dot(v_axis))
            }
        }
    };

    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()).or(i.pointer.latest_pos()));
    let is_mouse_in_vp = pointer_pos.map(|p| vp_rect.contains(p)).unwrap_or(false);

    let tool_base_pt: Option<ModelVec2> = match active_tool {
        Tool::DrawWall { start: Some(p), .. } => Some(*p),
        Tool::DrawCurtainWall { start: Some(p), .. } => Some(*p),
        Tool::DrawDetailLine { start: Some(p) } => Some(*p),
        Tool::DrawBeam { start: Some(p), .. } => Some(*p),
        Tool::DrawDimension { p2: Some(p), .. } => Some(*p),
        Tool::DrawDimension { start: Some(p), .. } => Some(*p),
        Tool::DrawCircularDimension { center: Some(p) } => Some(*p),
        Tool::DrawAngularDimension { apex: Some(p), .. } => Some(*p),
        Tool::DrawLeader { target_pt: Some(p), shoulder_pt: None } => Some(*p),
        Tool::DrawLeader { shoulder_pt: Some(p), .. } => Some(*p),
        Tool::Move { from: Some(p) } => Some(*p),
        Tool::Rotate { center: Some(p), .. } => Some(*p),
        Tool::Mirror { axis_p1: Some(p) } => Some(*p),
        Tool::DrawSlab { points, .. } if !points.is_empty() => points.last().copied(),
        Tool::DrawPolygonSurface { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawRoof { points, .. } if !points.is_empty() => points.last().copied(),
        Tool::DrawBuildingPad { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawToposolid { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawPropertyLine { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawRoad { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawSidewalk { points } if !points.is_empty() => points.last().copied(),
        Tool::DrawPathwalk { points } if !points.is_empty() => points.last().copied(),
        _ => None,
    };

    // Calculate snapped world coordinate (active pane receives cursor tracking)
    if is_active && is_mouse_in_vp {
        if let Some(hover_pos) = pointer_pos {
            let raw_world = unproject_screen_to_workplane(hover_pos, active_elev);
            let shift_held = ui.input(|i| i.modifiers.shift);
            let snap_cfg = &canvas_state.snap_settings;
            let mut snapped = raw_world;
            let mut active_snap_opt: Option<ActiveSnapVisual> = None;

            let key_x = ui.input(|i| i.key_down(egui::Key::X));
            let key_y = ui.input(|i| i.key_down(egui::Key::Y));
            let key_z = ui.input(|i| i.key_down(egui::Key::Z));

            // View-aware and 3D-aware projection helpers
            let project_to_view = |p: plinth_geom::Vec3| -> (ModelVec2, Pos2) {
                match active_view.kind {
                    plinth_model::ViewKind::FloorPlan | plinth_model::ViewKind::CeilingPlan => {
                        let v2 = plinth_geom::vec2(p.x, p.y);
                        (v2, to_screen(v2))
                    }
                    plinth_model::ViewKind::ElevationSouth => {
                        let v2 = plinth_geom::vec2(p.x, p.z);
                        (v2, to_screen(v2))
                    }
                    plinth_model::ViewKind::ElevationNorth => {
                        let v2 = plinth_geom::vec2(-p.x, p.z);
                        (v2, to_screen(v2))
                    }
                    plinth_model::ViewKind::ElevationEast => {
                        let v2 = plinth_geom::vec2(-p.y, p.z);
                        (v2, to_screen(v2))
                    }
                    plinth_model::ViewKind::ElevationWest => {
                        let v2 = plinth_geom::vec2(p.y, p.z);
                        (v2, to_screen(v2))
                    }
                    plinth_model::ViewKind::Section => {
                        let (s1, s2) = active_view.section_line.unwrap_or((ModelVec2::ZERO, ModelVec2::new(10000.0, 0.0)));
                        let diff = s2 - s1;
                        let len = diff.length().max(1.0);
                        let u_dir = diff / len;
                        let t = (p.x - s1.x) * u_dir.x + (p.y - s1.y) * u_dir.y;
                        let v2 = plinth_geom::vec2(t, p.z);
                        (v2, to_screen(v2))
                    }
                    _ => {
                        let sp = project_3d(p);
                        let v2 = unproject_screen_to_workplane(sp, p.z);
                        (v2, sp)
                    }
                }
            };

            let view_to_3d = |v: ModelVec2| -> plinth_geom::Vec3 {
                match active_view.kind {
                    plinth_model::ViewKind::FloorPlan | plinth_model::ViewKind::CeilingPlan => {
                        plinth_geom::Vec3::new(v.x, v.y, active_elev)
                    }
                    plinth_model::ViewKind::ElevationSouth => {
                        plinth_geom::Vec3::new(v.x, 0.0, v.y)
                    }
                    plinth_model::ViewKind::ElevationNorth => {
                        plinth_geom::Vec3::new(-v.x, 0.0, v.y)
                    }
                    plinth_model::ViewKind::ElevationEast => {
                        plinth_geom::Vec3::new(0.0, -v.x, v.y)
                    }
                    plinth_model::ViewKind::ElevationWest => {
                        plinth_geom::Vec3::new(0.0, v.x, v.y)
                    }
                    plinth_model::ViewKind::Section => {
                        let (s1, s2) = active_view.section_line.unwrap_or((ModelVec2::ZERO, ModelVec2::new(10000.0, 0.0)));
                        let diff = s2 - s1;
                        let len = diff.length().max(1.0);
                        let u_dir = diff / len;
                        plinth_geom::Vec3::new(s1.x + v.x * u_dir.x, s1.y + v.x * u_dir.y, v.y)
                    }
                    _ => {
                        unproject_screen_to_3d(hover_pos, active_elev)
                    }
                }
            };

            if snap_cfg.enabled {
                #[derive(Clone)]
                struct SnapCandidate3D {
                    world_3d: plinth_geom::Vec3,
                    kind: SnapKind,
                    label: &'static str,
                }
                let mut candidates: Vec<SnapCandidate3D> = Vec::new();
                let mut edge_segments: Vec<(plinth_geom::Vec3, plinth_geom::Vec3, SnapKind, &'static str)> = Vec::new();

                let get_z_bounds = |el: &Element| -> (f64, f64) {
                    match el {
                        Element::Wall(w) => w.z_bounds(doc.level_elevation(w.base_level), w.top_level.map(|l| doc.level_elevation(l))),
                        Element::Column(c) => c.z_bounds(doc.level_elevation(c.base_level), c.top_level.map(|l| doc.level_elevation(l))),
                        Element::Beam(b) => b.z_bounds(doc.level_elevation(b.level), b.top_level.map(|l| doc.level_elevation(l))),
                        Element::Slab(s) => s.z_bounds(doc.level_elevation(s.level), s.top_level.map(|l| doc.level_elevation(l))),
                        Element::Roof(r) => {
                            let lz = doc.level_elevation(r.level);
                            (lz + r.offset, lz + r.offset + 2500.0)
                        }
                        Element::Ceiling(c) => c.z_bounds(doc.level_elevation(c.level), c.top_level.map(|l| doc.level_elevation(l))),
                        Element::Footing(f) => f.z_bounds(doc.level_elevation(f.base_level), f.top_level.map(|l| doc.level_elevation(l))),
                        Element::CurtainWall(cw) => cw.z_bounds(doc.level_elevation(cw.base_level), cw.top_level.map(|l| doc.level_elevation(l))),
                        Element::Stair(s) => s.z_bounds(doc.level_elevation(s.base_level), s.top_level.map(|l| doc.level_elevation(l))),
                        Element::Ramp(r) => r.z_bounds(doc.level_elevation(r.base_level), r.top_level.map(|l| doc.level_elevation(l))),
                        Element::Railing(rl) => rl.z_bounds(doc.level_elevation(rl.level), rl.top_level.map(|l| doc.level_elevation(l))),
                        Element::FreeformSolid(fs) => (fs.z_min, fs.z_max),
                        Element::FreeformSurface(fs) => (fs.elevation, fs.elevation),
                        _ => (active_elev, active_elev),
                    }
                };

                for el in doc.elements.values() {
                    match el {
                        Element::Wall(w) => {
                            let (z0, z1) = get_z_bounds(el);
                            let sb = plinth_geom::Vec3::new(w.start.x, w.start.y, z0);
                            let st = plinth_geom::Vec3::new(w.start.x, w.start.y, z1);
                            let eb = plinth_geom::Vec3::new(w.end.x, w.end.y, z0);
                            let et = plinth_geom::Vec3::new(w.end.x, w.end.y, z1);

                            edge_segments.push((sb, eb, SnapKind::WallCenterline, "Centerline"));
                            edge_segments.push((st, et, SnapKind::WallCenterline, "Centerline"));
                            edge_segments.push((sb, st, SnapKind::Edge, "Wall Jamb"));
                            edge_segments.push((eb, et, SnapKind::Edge, "Wall Jamb"));

                            if snap_cfg.snap_to_endpoints {
                                candidates.push(SnapCandidate3D { world_3d: sb, kind: SnapKind::Endpoint, label: "Wall Base" });
                                candidates.push(SnapCandidate3D { world_3d: st, kind: SnapKind::Endpoint, label: "Wall Top" });
                                candidates.push(SnapCandidate3D { world_3d: eb, kind: SnapKind::Endpoint, label: "Wall Base" });
                                candidates.push(SnapCandidate3D { world_3d: et, kind: SnapKind::Endpoint, label: "Wall Top" });
                            }
                            if snap_cfg.snap_to_midpoints {
                                candidates.push(SnapCandidate3D { world_3d: (sb + eb) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                candidates.push(SnapCandidate3D { world_3d: (st + et) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                            }
                            let fp = w.footprint();
                            for i in 0..4 {
                                let p0 = fp[i];
                                let p1 = fp[(i + 1) % 4];
                                let v0_b = plinth_geom::Vec3::new(p0.x, p0.y, z0);
                                let v0_t = plinth_geom::Vec3::new(p0.x, p0.y, z1);
                                let v1_b = plinth_geom::Vec3::new(p1.x, p1.y, z0);
                                let v1_t = plinth_geom::Vec3::new(p1.x, p1.y, z1);

                                edge_segments.push((v0_b, v1_b, SnapKind::Edge, "Wall Footprint"));
                                edge_segments.push((v0_t, v1_t, SnapKind::Edge, "Wall Top Edge"));
                                edge_segments.push((v0_b, v0_t, SnapKind::Edge, "Wall Corner"));

                                candidates.push(SnapCandidate3D { world_3d: v0_b, kind: SnapKind::Endpoint, label: "Vertex" });
                                candidates.push(SnapCandidate3D { world_3d: v0_t, kind: SnapKind::Endpoint, label: "Vertex" });
                                candidates.push(SnapCandidate3D { world_3d: (v0_b + v1_b) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                candidates.push(SnapCandidate3D { world_3d: (v0_t + v1_t) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                            }
                        }
                        Element::Beam(b) => {
                            let (z0, z1) = get_z_bounds(el);
                            let sb = plinth_geom::Vec3::new(b.start.x, b.start.y, z0);
                            let st = plinth_geom::Vec3::new(b.start.x, b.start.y, z1);
                            let eb = plinth_geom::Vec3::new(b.end.x, b.end.y, z0);
                            let et = plinth_geom::Vec3::new(b.end.x, b.end.y, z1);

                            edge_segments.push((st, et, SnapKind::Edge, "Beam Top Centerline"));
                            edge_segments.push((sb, eb, SnapKind::Edge, "Beam Soffit Centerline"));
                            if snap_cfg.snap_to_endpoints {
                                candidates.push(SnapCandidate3D { world_3d: st, kind: SnapKind::Endpoint, label: "Beam Top" });
                                candidates.push(SnapCandidate3D { world_3d: sb, kind: SnapKind::Endpoint, label: "Beam Soffit" });
                                candidates.push(SnapCandidate3D { world_3d: et, kind: SnapKind::Endpoint, label: "Beam Top" });
                                candidates.push(SnapCandidate3D { world_3d: eb, kind: SnapKind::Endpoint, label: "Beam Soffit" });
                            }
                            if snap_cfg.snap_to_midpoints {
                                candidates.push(SnapCandidate3D { world_3d: (st + et) * 0.5, kind: SnapKind::Midpoint, label: "Beam Midpoint" });
                            }
                            let b_len = (b.end - b.start).length();
                            if b_len > 1.0 {
                                let b_dir = (b.end - b.start) / b_len;
                                let b_norm = plinth_geom::vec2(-b_dir.y, b_dir.x);
                                let hw = b.width * 0.5;
                                let corners = [b.start - b_norm * hw, b.start + b_norm * hw, b.end + b_norm * hw, b.end - b_norm * hw];
                                for i in 0..4 {
                                    let c0 = corners[i];
                                    let c1 = corners[(i + 1) % 4];
                                    let c0_t = plinth_geom::Vec3::new(c0.x, c0.y, z1);
                                    let c1_t = plinth_geom::Vec3::new(c1.x, c1.y, z1);
                                    let c0_b = plinth_geom::Vec3::new(c0.x, c0.y, z0);
                                    let c1_b = plinth_geom::Vec3::new(c1.x, c1.y, z0);

                                    edge_segments.push((c0_t, c1_t, SnapKind::Edge, "Beam Edge"));
                                    edge_segments.push((c0_b, c1_b, SnapKind::Edge, "Beam Soffit Edge"));
                                    edge_segments.push((c0_b, c0_t, SnapKind::Edge, "Beam Corner"));

                                    candidates.push(SnapCandidate3D { world_3d: c0_t, kind: SnapKind::Endpoint, label: "Beam Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: c0_b, kind: SnapKind::Endpoint, label: "Beam Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: (c0_t + c1_t) * 0.5, kind: SnapKind::Midpoint, label: "Beam Midpoint" });
                                }
                            }
                        }
                        Element::Column(c) => {
                            let (z0, z1) = get_z_bounds(el);
                            let c_base = plinth_geom::Vec3::new(c.location.x, c.location.y, z0);
                            let c_top = plinth_geom::Vec3::new(c.location.x, c.location.y, z1);

                            if snap_cfg.snap_to_centers {
                                candidates.push(SnapCandidate3D { world_3d: c_base, kind: SnapKind::Center, label: "Column Center" });
                                candidates.push(SnapCandidate3D { world_3d: c_top, kind: SnapKind::Center, label: "Column Top Center" });
                            }
                            edge_segments.push((c_base, c_top, SnapKind::Edge, "Column Centerline"));

                            let (sin, cos) = (c.rotation.sin(), c.rotation.cos());
                            let u = plinth_geom::vec2(cos, sin);
                            let v = plinth_geom::vec2(-sin, cos);
                            let hw = c.width * 0.5;
                            let hd = c.depth * 0.5;
                            let corners = [c.location - u * hw - v * hd, c.location + u * hw - v * hd, c.location + u * hw + v * hd, c.location - u * hw + v * hd];
                            for i in 0..4 {
                                let c0 = corners[i];
                                let c1 = corners[(i + 1) % 4];
                                let c0_b = plinth_geom::Vec3::new(c0.x, c0.y, z0);
                                let c0_t = plinth_geom::Vec3::new(c0.x, c0.y, z1);
                                let c1_b = plinth_geom::Vec3::new(c1.x, c1.y, z0);
                                let c1_t = plinth_geom::Vec3::new(c1.x, c1.y, z1);

                                edge_segments.push((c0_b, c1_b, SnapKind::Edge, "Column Base Edge"));
                                edge_segments.push((c0_t, c1_t, SnapKind::Edge, "Column Top Edge"));
                                edge_segments.push((c0_b, c0_t, SnapKind::Edge, "Column Corner"));

                                candidates.push(SnapCandidate3D { world_3d: c0_b, kind: SnapKind::Endpoint, label: "Column Vertex" });
                                candidates.push(SnapCandidate3D { world_3d: c0_t, kind: SnapKind::Endpoint, label: "Column Vertex" });
                                candidates.push(SnapCandidate3D { world_3d: (c0_b + c1_b) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                candidates.push(SnapCandidate3D { world_3d: (c0_t + c1_t) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                            }
                        }
                        Element::Footing(f) => {
                            let (z0, z1) = get_z_bounds(el);
                            let f_base = plinth_geom::Vec3::new(f.location.x, f.location.y, z0);
                            let f_top = plinth_geom::Vec3::new(f.location.x, f.location.y, z1);

                            if snap_cfg.snap_to_centers {
                                candidates.push(SnapCandidate3D { world_3d: f_base, kind: SnapKind::Center, label: "Footing Center" });
                                candidates.push(SnapCandidate3D { world_3d: f_top, kind: SnapKind::Center, label: "Footing Top Center" });
                            }
                            let (sin, cos) = (f.rotation.sin(), f.rotation.cos());
                            let u = plinth_geom::vec2(cos, sin);
                            let v = plinth_geom::vec2(-sin, cos);
                            let hw = f.width * 0.5;
                            let hd = f.length * 0.5;
                            let corners = [f.location - u * hw - v * hd, f.location + u * hw - v * hd, f.location + u * hw + v * hd, f.location - u * hw + v * hd];
                            for i in 0..4 {
                                let c0 = corners[i];
                                let c1 = corners[(i + 1) % 4];
                                let c0_b = plinth_geom::Vec3::new(c0.x, c0.y, z0);
                                let c0_t = plinth_geom::Vec3::new(c0.x, c0.y, z1);
                                let c1_b = plinth_geom::Vec3::new(c1.x, c1.y, z0);
                                let c1_t = plinth_geom::Vec3::new(c1.x, c1.y, z1);

                                edge_segments.push((c0_b, c1_b, SnapKind::Edge, "Footing Edge"));
                                edge_segments.push((c0_t, c1_t, SnapKind::Edge, "Footing Top Edge"));
                                edge_segments.push((c0_b, c0_t, SnapKind::Edge, "Footing Corner"));

                                candidates.push(SnapCandidate3D { world_3d: c0_b, kind: SnapKind::Endpoint, label: "Footing Vertex" });
                                candidates.push(SnapCandidate3D { world_3d: c0_t, kind: SnapKind::Endpoint, label: "Footing Vertex" });
                            }
                        }
                        Element::CurtainWall(cw) => {
                            let (z0, z1) = get_z_bounds(el);
                            let sb = plinth_geom::Vec3::new(cw.start.x, cw.start.y, z0);
                            let st = plinth_geom::Vec3::new(cw.start.x, cw.start.y, z1);
                            let eb = plinth_geom::Vec3::new(cw.end.x, cw.end.y, z0);
                            let et = plinth_geom::Vec3::new(cw.end.x, cw.end.y, z1);

                            edge_segments.push((sb, eb, SnapKind::WallCenterline, "Curtain Wall Sill"));
                            edge_segments.push((st, et, SnapKind::WallCenterline, "Curtain Wall Head"));
                            edge_segments.push((sb, st, SnapKind::Edge, "Curtain Wall Jamb"));
                            edge_segments.push((eb, et, SnapKind::Edge, "Curtain Wall Jamb"));

                            candidates.push(SnapCandidate3D { world_3d: sb, kind: SnapKind::Endpoint, label: "CW Base" });
                            candidates.push(SnapCandidate3D { world_3d: st, kind: SnapKind::Endpoint, label: "CW Head" });
                            candidates.push(SnapCandidate3D { world_3d: eb, kind: SnapKind::Endpoint, label: "CW Base" });
                            candidates.push(SnapCandidate3D { world_3d: et, kind: SnapKind::Endpoint, label: "CW Head" });
                        }
                        Element::Slab(s) => {
                            let (z0, z1) = get_z_bounds(el);
                            let n = s.boundary.len();
                            if n > 1 {
                                for i in 0..n {
                                    let p_curr = s.boundary[i];
                                    let p_next = s.boundary[(i + 1) % n];
                                    let v0_t = plinth_geom::Vec3::new(p_curr.x, p_curr.y, z1);
                                    let v1_t = plinth_geom::Vec3::new(p_next.x, p_next.y, z1);
                                    let v0_b = plinth_geom::Vec3::new(p_curr.x, p_curr.y, z0);
                                    let v1_b = plinth_geom::Vec3::new(p_next.x, p_next.y, z0);

                                    edge_segments.push((v0_t, v1_t, SnapKind::Edge, "Slab Top Edge"));
                                    edge_segments.push((v0_b, v1_b, SnapKind::Edge, "Slab Bottom Edge"));
                                    edge_segments.push((v0_b, v0_t, SnapKind::Edge, "Slab Rim Edge"));

                                    candidates.push(SnapCandidate3D { world_3d: v0_t, kind: SnapKind::Endpoint, label: "Slab Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: v0_b, kind: SnapKind::Endpoint, label: "Slab Bottom Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: (v0_t + v1_t) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                }
                            }
                        }
                        Element::Roof(r) => {
                            let (z0, z1) = get_z_bounds(el);
                            let n = r.boundary.len();
                            if n > 1 {
                                for i in 0..n {
                                    let p_curr = r.boundary[i];
                                    let p_next = r.boundary[(i + 1) % n];
                                    let v0_e = plinth_geom::Vec3::new(p_curr.x, p_curr.y, z0);
                                    let v1_e = plinth_geom::Vec3::new(p_next.x, p_next.y, z0);
                                    let v0_r = plinth_geom::Vec3::new(p_curr.x, p_curr.y, z1);
                                    let v1_r = plinth_geom::Vec3::new(p_next.x, p_next.y, z1);

                                    edge_segments.push((v0_e, v1_e, SnapKind::Edge, "Roof Eave"));
                                    edge_segments.push((v0_r, v1_r, SnapKind::Edge, "Roof Ridge"));
                                    candidates.push(SnapCandidate3D { world_3d: v0_e, kind: SnapKind::Endpoint, label: "Roof Eave Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: (v0_e + v1_e) * 0.5, kind: SnapKind::Midpoint, label: "Eave Midpoint" });
                                    candidates.push(SnapCandidate3D { world_3d: v0_r, kind: SnapKind::Endpoint, label: "Roof Ridge Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: v1_r, kind: SnapKind::Endpoint, label: "Roof Ridge Vertex" });
                                }
                            }
                        }
                        Element::Ceiling(c) => {
                            let (_z0, z1) = get_z_bounds(el);
                            let n = c.boundary.len();
                            if n > 1 {
                                for i in 0..n {
                                    let p_curr = c.boundary[i];
                                    let p_next = c.boundary[(i + 1) % n];
                                    let v0 = plinth_geom::Vec3::new(p_curr.x, p_curr.y, z1);
                                    let v1 = plinth_geom::Vec3::new(p_next.x, p_next.y, z1);

                                    edge_segments.push((v0, v1, SnapKind::Edge, "Ceiling Edge"));
                                    candidates.push(SnapCandidate3D { world_3d: v0, kind: SnapKind::Endpoint, label: "Ceiling Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: (v0 + v1) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                }
                            }
                        }
                        Element::Stair(s) => {
                            let (z0, z1) = get_z_bounds(el);
                            let sb = plinth_geom::Vec3::new(s.start.x, s.start.y, z0);
                            let et = plinth_geom::Vec3::new(s.end.x, s.end.y, z1);
                            edge_segments.push((sb, et, SnapKind::Edge, "Stair Slope"));
                            candidates.push(SnapCandidate3D { world_3d: sb, kind: SnapKind::Endpoint, label: "Stair Start" });
                            candidates.push(SnapCandidate3D { world_3d: et, kind: SnapKind::Endpoint, label: "Stair Landing" });
                        }
                        Element::Ramp(r) => {
                            let (z0, z1) = get_z_bounds(el);
                            let sb = plinth_geom::Vec3::new(r.start.x, r.start.y, z0);
                            let et = plinth_geom::Vec3::new(r.end.x, r.end.y, z1);
                            edge_segments.push((sb, et, SnapKind::Edge, "Ramp Slope"));
                            candidates.push(SnapCandidate3D { world_3d: sb, kind: SnapKind::Endpoint, label: "Ramp Start" });
                            candidates.push(SnapCandidate3D { world_3d: et, kind: SnapKind::Endpoint, label: "Ramp Landing" });
                        }
                        Element::Railing(rl) => {
                            let (z0, z1) = get_z_bounds(el);
                            let n = rl.path.len();
                            if n > 1 {
                                for i in 0..(n - 1) {
                                    let p0 = rl.path[i];
                                    let p1 = rl.path[i + 1];
                                    let v0_t = plinth_geom::Vec3::new(p0.x, p0.y, z1);
                                    let v1_t = plinth_geom::Vec3::new(p1.x, p1.y, z1);
                                    let v0_b = plinth_geom::Vec3::new(p0.x, p0.y, z0);

                                    edge_segments.push((v0_t, v1_t, SnapKind::Edge, "Railing Handrail"));
                                    candidates.push(SnapCandidate3D { world_3d: v0_t, kind: SnapKind::Endpoint, label: "Railing Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: v0_b, kind: SnapKind::Endpoint, label: "Railing Post Base" });
                                }
                            }
                        }
                        Element::BuildingPad(bp) => {
                            let z = bp.elevation_offset;
                            let n = bp.boundary.len();
                            if n > 1 {
                                for i in 0..n {
                                    let p0 = bp.boundary[i];
                                    let p1 = bp.boundary[(i + 1) % n];
                                    let v0 = plinth_geom::Vec3::new(p0.x, p0.y, z);
                                    let v1 = plinth_geom::Vec3::new(p1.x, p1.y, z);
                                    edge_segments.push((v0, v1, SnapKind::Edge, "Building Pad Edge"));
                                    candidates.push(SnapCandidate3D { world_3d: v0, kind: SnapKind::Endpoint, label: "Pad Vertex" });
                                    candidates.push(SnapCandidate3D { world_3d: (v0 + v1) * 0.5, kind: SnapKind::Midpoint, label: "Midpoint" });
                                }
                            }
                        }
                        Element::Toposolid(ts) => {
                            for ep in &ts.elevation_points {
                                candidates.push(SnapCandidate3D { world_3d: plinth_geom::Vec3::new(ep.x, ep.y, ep.z), kind: SnapKind::Endpoint, label: "Contour Point" });
                            }
                        }
                        Element::Road(rd) => {
                            let z = active_elev;
                            let n = rd.path.len();
                            if n > 1 {
                                for i in 0..(n - 1) {
                                    let v0 = plinth_geom::Vec3::new(rd.path[i].x, rd.path[i].y, z);
                                    let v1 = plinth_geom::Vec3::new(rd.path[i + 1].x, rd.path[i + 1].y, z);
                                    edge_segments.push((v0, v1, SnapKind::Edge, "Road Centerline"));
                                    candidates.push(SnapCandidate3D { world_3d: v0, kind: SnapKind::Endpoint, label: "Road Node" });
                                }
                            }
                        }
                        Element::Sidewalk(sw) => {
                            let z = active_elev;
                            let n = sw.path.len();
                            if n > 1 {
                                for i in 0..(n - 1) {
                                    let v0 = plinth_geom::Vec3::new(sw.path[i].x, sw.path[i].y, z);
                                    let v1 = plinth_geom::Vec3::new(sw.path[i + 1].x, sw.path[i + 1].y, z);
                                    edge_segments.push((v0, v1, SnapKind::Edge, "Sidewalk Edge"));
                                    candidates.push(SnapCandidate3D { world_3d: v0, kind: SnapKind::Endpoint, label: "Sidewalk Node" });
                                }
                            }
                        }
                        Element::Pathwalk(pw) => {
                            let z = active_elev;
                            let n = pw.path.len();
                            if n > 1 {
                                for i in 0..(n - 1) {
                                    let v0 = plinth_geom::Vec3::new(pw.path[i].x, pw.path[i].y, z);
                                    let v1 = plinth_geom::Vec3::new(pw.path[i + 1].x, pw.path[i + 1].y, z);
                                    edge_segments.push((v0, v1, SnapKind::Edge, "Pathwalk Edge"));
                                    candidates.push(SnapCandidate3D { world_3d: v0, kind: SnapKind::Endpoint, label: "Pathwalk Node" });
                                }
                            }
                        }
                        Element::FreeformSolid(fs) => {
                            if let plinth_model::PrimitiveKind::Mesh { ref faces } = fs.primitive {
                                for f in faces {
                                    let m = f.pts.len();
                                    for i in 0..m {
                                        let p0 = f.pts[i];
                                        let p1 = f.pts[(i + 1) % m];
                                        candidates.push(SnapCandidate3D { world_3d: p0, kind: SnapKind::Endpoint, label: "Mesh Vertex" });
                                        edge_segments.push((p0, p1, SnapKind::Edge, "Mesh Edge"));
                                    }
                                }
                            }
                        }
                        Element::FreeformSurface(fs) => {
                            if let Some(ref b3) = fs.boundary_3d {
                                let m = b3.len();
                                for i in 0..m {
                                    let p0 = b3[i];
                                    let p1 = b3[(i + 1) % m];
                                    candidates.push(SnapCandidate3D { world_3d: p0, kind: SnapKind::Endpoint, label: "Surface Vertex" });
                                    edge_segments.push((p0, p1, SnapKind::Edge, "Surface Edge"));
                                }
                            }
                        }
                        Element::Door(d) => {
                            if let Some(Element::Wall(hw)) = doc.elements.get(&d.host_wall) {
                                let wall_len = hw.length();
                                if wall_len > 1.0 {
                                    let wall_z0 = hw.z_bounds(doc.level_elevation(hw.base_level), hw.top_level.map(|l| doc.level_elevation(l))).0;
                                    let wall_dir = (hw.end - hw.start) / wall_len;
                                    let half_w = d.width * 0.5;
                                    let center_2d = hw.start + wall_dir * d.offset_along_wall;
                                    let jamb0_2d = center_2d - wall_dir * half_w;
                                    let jamb1_2d = center_2d + wall_dir * half_w;
                                    let z_sill = wall_z0 + d.sill_height;
                                    let z_head = z_sill + d.height;

                                    let j0_s = plinth_geom::Vec3::new(jamb0_2d.x, jamb0_2d.y, z_sill);
                                    let j0_h = plinth_geom::Vec3::new(jamb0_2d.x, jamb0_2d.y, z_head);
                                    let j1_s = plinth_geom::Vec3::new(jamb1_2d.x, jamb1_2d.y, z_sill);
                                    let j1_h = plinth_geom::Vec3::new(jamb1_2d.x, jamb1_2d.y, z_head);

                                    edge_segments.push((j0_s, j1_s, SnapKind::Edge, "Door Threshold"));
                                    edge_segments.push((j0_h, j1_h, SnapKind::Edge, "Door Lintel"));
                                    edge_segments.push((j0_s, j0_h, SnapKind::Edge, "Door Jamb"));
                                    edge_segments.push((j1_s, j1_h, SnapKind::Edge, "Door Jamb"));

                                    candidates.push(SnapCandidate3D { world_3d: j0_s, kind: SnapKind::Endpoint, label: "Door Jamb Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j0_h, kind: SnapKind::Endpoint, label: "Door Head Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j1_s, kind: SnapKind::Endpoint, label: "Door Jamb Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j1_h, kind: SnapKind::Endpoint, label: "Door Head Corner" });
                                }
                            }
                        }
                        Element::Window(w) => {
                            if let Some(Element::Wall(hw)) = doc.elements.get(&w.host_wall) {
                                let wall_len = hw.length();
                                if wall_len > 1.0 {
                                    let wall_z0 = hw.z_bounds(doc.level_elevation(hw.base_level), hw.top_level.map(|l| doc.level_elevation(l))).0;
                                    let wall_dir = (hw.end - hw.start) / wall_len;
                                    let half_w = w.width * 0.5;
                                    let center_2d = hw.start + wall_dir * w.offset_along_wall;
                                    let jamb0_2d = center_2d - wall_dir * half_w;
                                    let jamb1_2d = center_2d + wall_dir * half_w;
                                    let z_sill = wall_z0 + w.sill_height;
                                    let z_head = z_sill + w.height;

                                    let j0_s = plinth_geom::Vec3::new(jamb0_2d.x, jamb0_2d.y, z_sill);
                                    let j0_h = plinth_geom::Vec3::new(jamb0_2d.x, jamb0_2d.y, z_head);
                                    let j1_s = plinth_geom::Vec3::new(jamb1_2d.x, jamb1_2d.y, z_sill);
                                    let j1_h = plinth_geom::Vec3::new(jamb1_2d.x, jamb1_2d.y, z_head);

                                    edge_segments.push((j0_s, j1_s, SnapKind::Edge, "Window Sill"));
                                    edge_segments.push((j0_h, j1_h, SnapKind::Edge, "Window Head"));
                                    edge_segments.push((j0_s, j0_h, SnapKind::Edge, "Window Jamb"));
                                    edge_segments.push((j1_s, j1_h, SnapKind::Edge, "Window Jamb"));

                                    candidates.push(SnapCandidate3D { world_3d: j0_s, kind: SnapKind::Endpoint, label: "Window Sill Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j0_h, kind: SnapKind::Endpoint, label: "Window Head Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j1_s, kind: SnapKind::Endpoint, label: "Window Sill Corner" });
                                    candidates.push(SnapCandidate3D { world_3d: j1_h, kind: SnapKind::Endpoint, label: "Window Head Corner" });
                                }
                            }
                        }
                        Element::DetailLine(l) => {
                            let p0 = plinth_geom::Vec3::new(l.start.x, l.start.y, active_elev);
                            let p1 = plinth_geom::Vec3::new(l.end.x, l.end.y, active_elev);
                            edge_segments.push((p0, p1, SnapKind::Edge, "Detail Line"));
                            candidates.push(SnapCandidate3D { world_3d: p0, kind: SnapKind::Endpoint, label: "Line Endpoint" });
                            candidates.push(SnapCandidate3D { world_3d: p1, kind: SnapKind::Endpoint, label: "Line Endpoint" });
                            candidates.push(SnapCandidate3D { world_3d: (p0 + p1) * 0.5, kind: SnapKind::Midpoint, label: "Line Midpoint" });
                        }
                        Element::Dimension(d) => {
                            let p1_3d = plinth_geom::Vec3::new(d.p1.x, d.p1.y, active_elev);
                            let p2_3d = plinth_geom::Vec3::new(d.p2.x, d.p2.y, active_elev);
                            candidates.push(SnapCandidate3D { world_3d: p1_3d, kind: SnapKind::Endpoint, label: "Dim Witness" });
                            candidates.push(SnapCandidate3D { world_3d: p2_3d, kind: SnapKind::Endpoint, label: "Dim Witness" });
                        }
                        Element::Leader(ld) => {
                            let pt_tgt = plinth_geom::Vec3::new(ld.target_pt.x, ld.target_pt.y, active_elev);
                            let pt_sh = plinth_geom::Vec3::new(ld.shoulder_pt.x, ld.shoulder_pt.y, active_elev);
                            candidates.push(SnapCandidate3D { world_3d: pt_tgt, kind: SnapKind::Endpoint, label: "Leader Arrow" });
                            candidates.push(SnapCandidate3D { world_3d: pt_sh, kind: SnapKind::Endpoint, label: "Leader Shoulder" });
                        }
                        _ => {}
                    }
                }

                // Level Datums & Grids
                for lvl in doc.levels.values() {
                    let lvl_z = lvl.elevation;
                    if active_view.kind.is_elevation_or_section() {
                        let p0 = plinth_geom::Vec3::new(-100_000.0, 0.0, lvl_z);
                        let p1 = plinth_geom::Vec3::new(100_000.0, 0.0, lvl_z);
                        edge_segments.push((p0, p1, SnapKind::Edge, "Level Datum"));
                    }
                }

                for g in doc.grids.values() {
                    let g0 = plinth_geom::Vec3::new(g.start.x, g.start.y, active_elev);
                    let g1 = plinth_geom::Vec3::new(g.end.x, g.end.y, active_elev);
                    edge_segments.push((g0, g1, SnapKind::WallCenterline, "Grid Line"));
                    candidates.push(SnapCandidate3D { world_3d: g0, kind: SnapKind::Endpoint, label: "Grid End" });
                    candidates.push(SnapCandidate3D { world_3d: g1, kind: SnapKind::Endpoint, label: "Grid End" });
                }

                if snap_cfg.snap_to_intersections {
                    let grids: Vec<(ModelVec2, ModelVec2)> = doc.grids.values().map(|g| (g.start, g.end)).collect();
                    for i in 0..grids.len() {
                        let r = grids[i].1 - grids[i].0;
                        for j in (i + 1)..grids.len() {
                            let s = grids[j].1 - grids[j].0;
                            if let Some((t, u)) = plinth_geom::polygon::line_intersection(grids[i].0, r, grids[j].0, s) {
                                if (-1e-4..=1.0001).contains(&t) && (-1e-4..=1.0001).contains(&u) {
                                    let ip = grids[i].0 + r * t;
                                    candidates.push(SnapCandidate3D { world_3d: plinth_geom::Vec3::new(ip.x, ip.y, active_elev), kind: SnapKind::Intersection, label: "Grid Intersection" });
                                }
                            }
                        }
                    }
                }

                // 1. Magnetic Vertex Snapping
                let mut best_snap: Option<(f32, SnapCandidate3D, Pos2, ModelVec2)> = None;
                for c in &candidates {
                    let (view_pt, sp) = project_to_view(c.world_3d);
                    let d = sp.distance(hover_pos);
                    if d <= snap_cfg.snap_radius_px {
                        if best_snap.as_ref().map_or(true, |(best_d, ..)| d < *best_d) {
                            best_snap = Some((d, c.clone(), sp, view_pt));
                        }
                    }
                }

                if let Some((_, c, sp, v_pt)) = best_snap {
                    snapped = v_pt;
                    canvas_state.snapped_3d_pos = Some(c.world_3d);
                    active_snap_opt = Some(ActiveSnapVisual {
                        kind: c.kind,
                        screen_pos: sp,
                        world_pos: v_pt,
                        label: c.label.to_string(),
                        ray_start: None,
                        ray_end: None,
                        ray_angle_deg: None,
                        ray_dist_mm: None,
                    });
                } else {
                    // 2. Magnetic Edge Snapping
                    let mut best_edge_snap: Option<(f32, plinth_geom::Vec3, Pos2, ModelVec2, SnapKind, &'static str)> = None;
                    if snap_cfg.snap_to_edges || snap_cfg.snap_to_centerlines {
                        for &(p1_3d, p2_3d, kind, label) in &edge_segments {
                            let (_, sp1) = project_to_view(p1_3d);
                            let (_, sp2) = project_to_view(p2_3d);
                            let seg = sp2 - sp1;
                            let seg_len_sq = seg.length_sq();
                            if seg_len_sq > 1e-4 {
                                let t = (((hover_pos.x - sp1.x) * seg.x + (hover_pos.y - sp1.y) * seg.y) / seg_len_sq).clamp(0.0, 1.0);
                                let proj_sp = sp1 + seg * t;
                                let d = proj_sp.distance(hover_pos);
                                if d <= snap_cfg.snap_radius_px {
                                    if best_edge_snap.as_ref().map_or(true, |(best_d, ..)| d < *best_d) {
                                        let proj_3d = p1_3d + (p2_3d - p1_3d) * (t as f64);
                                        let (proj_view, _) = project_to_view(proj_3d);
                                        best_edge_snap = Some((d, proj_3d, proj_sp, proj_view, kind, label));
                                    }
                                }
                            }
                        }
                    }

                    if let Some((_, proj_3d, sp, v_pt, kind, label)) = best_edge_snap {
                        snapped = v_pt;
                        canvas_state.snapped_3d_pos = Some(proj_3d);
                        active_snap_opt = Some(ActiveSnapVisual {
                            kind,
                            screen_pos: sp,
                            world_pos: v_pt,
                            label: label.to_string(),
                            ray_start: None,
                            ray_end: None,
                            ray_angle_deg: None,
                            ray_dist_mm: None,
                        });
                    } else if let Some(base_p) = tool_base_pt {
                        // 3. Drafting Constraints (Ortho mode or Polar Tracking)
                        let is_ortho = snap_cfg.ortho_mode ^ shift_held;
                        if is_ortho {
                            let dx = (raw_world.x - base_p.x).abs();
                            let dy = (raw_world.y - base_p.y).abs();
                            let ortho_pt = if dx >= dy {
                                ModelVec2::new(raw_world.x, base_p.y)
                            } else {
                                ModelVec2::new(base_p.x, raw_world.y)
                            };
                            snapped = ortho_pt;
                            canvas_state.snapped_3d_pos = Some(view_to_3d(ortho_pt));
                            let sp = project_point(ortho_pt, active_elev);
                            let base_sp = project_point(base_p, active_elev);
                            let dist_mm = base_p.distance(ortho_pt);
                            let angle_deg = if dx >= dy {
                                if raw_world.x >= base_p.x { 0.0 } else { 180.0 }
                            } else {
                                if raw_world.y >= base_p.y { 90.0 } else { 270.0 }
                            };
                            active_snap_opt = Some(ActiveSnapVisual {
                                kind: SnapKind::OrthoRay,
                                screen_pos: sp,
                                world_pos: ortho_pt,
                                label: format!("[ORTHO]  {:.0} mm", dist_mm),
                                ray_start: Some(base_sp),
                                ray_end: Some(sp),
                                ray_angle_deg: Some(angle_deg),
                                ray_dist_mm: Some(dist_mm),
                            });
                        } else if snap_cfg.polar_tracking {
                            let v = raw_world - base_p;
                            let dist = v.length();
                            if dist > 35.0 {
                                let inc_rad = snap_cfg.polar_increment_deg.to_radians();
                                let mut angle = v.y.atan2(v.x);
                                if angle < 0.0 {
                                    angle += std::f64::consts::TAU;
                                }
                                let k = (angle / inc_rad).round();
                                let target_angle = k * inc_rad;
                                let angle_diff = (angle - target_angle).abs();
                                let ray_dir = ModelVec2::new(target_angle.cos(), target_angle.sin());
                                let along = v.dot(ray_dir);
                                let perp = (v - ray_dir * along).length();
                                let perp_px = (perp as f32) * zoom;

                                if perp_px <= 14.0 || angle_diff.to_degrees() <= 3.5 {
                                    let polar_pt = base_p + ray_dir * along.max(0.0);
                                    snapped = polar_pt;
                                    canvas_state.snapped_3d_pos = Some(view_to_3d(polar_pt));
                                    let sp = project_point(polar_pt, active_elev);
                                    let base_sp = project_point(base_p, active_elev);
                                    let ray_far_sp = project_point(base_p + ray_dir * 100000.0, active_elev);
                                    let deg = (target_angle.to_degrees().rem_euclid(360.0) * 10.0).round() / 10.0;
                                    active_snap_opt = Some(ActiveSnapVisual {
                                        kind: SnapKind::PolarRay,
                                        screen_pos: sp,
                                        world_pos: polar_pt,
                                        label: format!("∠ {:.1}°   {:.0} mm", deg, along),
                                        ray_start: Some(base_sp),
                                        ray_end: Some(ray_far_sp),
                                        ray_angle_deg: Some(deg),
                                        ray_dist_mm: Some(along),
                                    });
                                }
                            }
                        }
                    }

                    // 4. Grid Snapping fallback
                    if active_snap_opt.is_none() && snap_cfg.snap_to_grid {
                        let snap_interval = if zoom < 0.02 { 500.0 } else { snap_cfg.grid_spacing.max(10.0) };
                        snapped = ModelVec2::new(
                            (raw_world.x / snap_interval).round() * snap_interval,
                            (raw_world.y / snap_interval).round() * snap_interval,
                        );
                        canvas_state.snapped_3d_pos = Some(view_to_3d(snapped));
                    }
                }
            }

            // Fallback unprojected 3D position if no magnetic snap occurred
            if canvas_state.snapped_3d_pos.is_none() {
                canvas_state.snapped_3d_pos = Some(view_to_3d(snapped));
            }

            // Axis constraint filtering during Move
            if matches!(active_tool, Tool::Move { from: Some(_) }) {
                if let (Some(b3), Some(mut cur3)) = (canvas_state.move_base_3d, canvas_state.snapped_3d_pos) {
                    if key_x {
                        cur3 = plinth_geom::Vec3::new(cur3.x, b3.y, b3.z);
                    } else if key_y {
                        cur3 = plinth_geom::Vec3::new(b3.x, cur3.y, b3.z);
                    } else if key_z {
                        cur3 = plinth_geom::Vec3::new(b3.x, b3.y, cur3.z);
                    } else if shift_held {
                        let d = cur3 - b3;
                        if d.x.abs() >= d.y.abs() && d.x.abs() >= d.z.abs() {
                            cur3 = plinth_geom::Vec3::new(cur3.x, b3.y, b3.z);
                        } else if d.y.abs() >= d.x.abs() && d.y.abs() >= d.z.abs() {
                            cur3 = plinth_geom::Vec3::new(b3.x, cur3.y, b3.z);
                        } else {
                            cur3 = plinth_geom::Vec3::new(b3.x, b3.y, cur3.z);
                        }
                    }
                    canvas_state.snapped_3d_pos = Some(cur3);
                    let (v_pt, sp) = project_to_view(cur3);
                    snapped = v_pt;
                    if let Some(ref mut sn) = active_snap_opt {
                        sn.world_pos = v_pt;
                        sn.screen_pos = sp;
                    }
                }
            }

            canvas_state.snapped_world_pos = snapped;
            canvas_state.active_snap = active_snap_opt;
        }
    }

    let mouse_pos = canvas_state.snapped_world_pos;
    let mouse_screen = project_point(mouse_pos, active_elev);

    // 2. Render Single-Source-of-Truth Projection Scene
    let is_dark = theme.mode == crate::theme::ThemeMode::Dark;
    let mut rendered_gpu = false;

    let sel_ids: Vec<plinth_model::ElementId> = if !canvas_state.selected_elements.is_empty() {
        canvas_state.selected_elements.clone()
    } else if let Some(sel_id) = *selected_element {
        vec![sel_id]
    } else {
        Vec::new()
    };

    if active_view.kind == plinth_model::ViewKind::ThreeD {
        if let Some(ref gpu) = canvas_state.gpu_renderer {
            let doc_elem_count = doc.elements.len();
            let doc_undo_count = doc.undo_count();
            let is_cache_valid = canvas_state.cached_gpu_scene.is_some()
                && canvas_state.cached_gpu_elements_count == doc_elem_count
                && canvas_state.cached_gpu_doc_undo_count == doc_undo_count
                && canvas_state.cached_gpu_visual_style == pane_visual_style
                && canvas_state.cached_gpu_is_dark == is_dark
                && canvas_state.cached_gpu_selection == sel_ids;

            if !is_cache_valid {
                let scene = crate::gpu_viewport::build_gpu_scene(doc, pane_visual_style, is_dark, &canvas_state.materials, &sel_ids);
                canvas_state.cached_gpu_scene = Some(scene);
                canvas_state.cached_gpu_elements_count = doc_elem_count;
                canvas_state.cached_gpu_doc_undo_count = doc_undo_count;
                canvas_state.cached_gpu_visual_style = pane_visual_style;
                canvas_state.cached_gpu_is_dark = is_dark;
                canvas_state.cached_gpu_selection = sel_ids.clone();
            }

            if let Some(ref gpu_scene) = canvas_state.cached_gpu_scene {
                if !gpu_scene.is_empty() {
                    let cb = gpu.render_viewport(
                        vp_rect,
                        &active_view.camera_3d.unwrap_or_default(),
                        zoom,
                        pan,
                        gpu_scene,
                        pane_visual_style,
                        is_dark,
                    );
                    vp_painter.add(cb);
                    rendered_gpu = true;
                }
            }
        }
    }

    if !rendered_gpu {
        let mut color_theme = if is_dark {
            plinth_views::ColorTheme::dark()
        } else {
            plinth_views::ColorTheme::light()
        };
        color_theme.materials = canvas_state.materials;
        let scene = plinth_views::render_view_with_style_and_theme(doc, &active_view, &color_theme, pane_visual_style);

        for prim in &scene.primitives {
            match prim {
                RenderPrimitive::Line { start, end, color, width, dashed } => {
                    let p1 = to_screen(*start);
                    let p2 = to_screen(*end);
                    let stroke_color = Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                    let stroke = Stroke::new(*width, stroke_color);

                    if *dashed {
                        // Draw dashed segments safely clipped
                        let dir = p2 - p1;
                        let len = dir.length();
                        if len > 1.0 {
                            let u = dir / len;
                            let dash_len = 8.0f32;
                            let gap_len = 5.0f32;
                            let mut curr = 0.0f32;
                            while curr < len {
                                let end_seg = (curr + dash_len).min(len);
                                draw_line_segment_safe(&vp_painter, p1 + u * curr, p1 + u * end_seg, stroke);
                                curr += dash_len + gap_len;
                            }
                        }
                    } else {
                        draw_line_segment_safe(&vp_painter, p1, p2, stroke);
                    }
                }
                RenderPrimitive::Polygon { points, fill_color, stroke_color, stroke_width } => {
                    if points.len() >= 3 {
                        let screen_pts: Vec<Pos2> = points.iter().map(|&p| to_screen(p)).collect();
                        let fill = Color32::from_rgba_premultiplied(fill_color[0], fill_color[1], fill_color[2], fill_color[3]);
                        let stroke = stroke_color.map(|c| {
                            Stroke::new(*stroke_width, Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
                        }).unwrap_or(Stroke::NONE);

                        draw_polygon_safe(&vp_painter, &screen_pts, fill, stroke);
                    }
                }
                RenderPrimitive::Arc { center: arc_c, radius, start_angle, end_angle, color, width } => {
                    let center_sp = to_screen(*arc_c);
                    let r_px = radius * zoom;
                    let stroke = Stroke::new(*width, Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]));

                    let segs = 16;
                    let mut pts = Vec::with_capacity(segs + 1);
                    for i in 0..=segs {
                        let t = (i as f32) / (segs as f32);
                        let a = start_angle + (end_angle - start_angle) * t;
                        // Note: Y is flipped on screen
                        pts.push(Pos2::new(center_sp.x + r_px * a.cos(), center_sp.y - r_px * a.sin()));
                    }
                    for i in 0..pts.len() - 1 {
                        vp_painter.line_segment([pts[i], pts[i + 1]], stroke);
                    }
                }
                RenderPrimitive::Circle { center: circ_c, radius, fill_color, stroke_color, stroke_width } => {
                    let c_sp = to_screen(*circ_c);
                    let r_px = radius * zoom;
                    let stroke = Stroke::new(*stroke_width, Color32::from_rgba_premultiplied(stroke_color[0], stroke_color[1], stroke_color[2], stroke_color[3]));
                    if let Some(f) = fill_color {
                        let fill = Color32::from_rgba_premultiplied(f[0], f[1], f[2], f[3]);
                        vp_painter.circle_filled(c_sp, r_px, fill);
                    }
                    vp_painter.circle_stroke(c_sp, r_px, stroke);
                }
                RenderPrimitive::Text { pos, text, color, size, align_center } => {
                    let p_sp = to_screen(*pos);
                    let text_color = Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                    let align = if *align_center {
                        egui::Align2::CENTER_CENTER
                    } else {
                        egui::Align2::LEFT_CENTER
                    };
                    let display_size = (*size * (zoom / 0.05).clamp(0.5, 2.0)).clamp(8.0, 48.0);
                    vp_painter.text(p_sp, align, text, egui::FontId::proportional(display_size), text_color);
                }
            }
        }
    }

    // 3. Selection Highlight (renders for 2D Plan, Elevation, Section views, or CPU fallback)
    for sel_id in &sel_ids {
        if let Some(el) = doc.elements.get(sel_id) {
            let sel_stroke = Stroke::new(2.5, theme.selection_color);
            let sel_fill = Color32::from_rgba_premultiplied(50, 150, 255, 40);
            if active_view.kind.is_plan_or_rcp() {
                // 2D Plan Selection Highlight
                match el {
                    Element::Wall(w) => {
                        let p1 = to_screen(w.start);
                        let p2 = to_screen(w.end);
                        vp_painter.line_segment([p1, p2], sel_stroke);
                        let foot = w.footprint();
                        let pts: Vec<Pos2> = foot.iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::Column(c) => {
                        let foot = c.footprint();
                        let pts: Vec<Pos2> = foot.iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::CurtainWall(cw) => {
                        let p1 = to_screen(cw.start);
                        let p2 = to_screen(cw.end);
                        vp_painter.line_segment([p1, p2], sel_stroke);
                        let foot = cw.footprint();
                        let pts: Vec<Pos2> = foot.iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::Door(door) => {
                        if let Some(Element::Wall(hw)) = doc.elements.get(&door.host_wall) {
                            let center = hw.start + hw.direction() * door.offset_along_wall;
                            let norm = hw.normal();
                            let dir = hw.direction();
                            let p0 = center - dir * (door.width * 0.5) - norm * (hw.thickness * 0.5);
                            let p1 = center + dir * (door.width * 0.5) - norm * (hw.thickness * 0.5);
                            let p2 = center + dir * (door.width * 0.5) + norm * (hw.thickness * 0.5);
                            let p3 = center - dir * (door.width * 0.5) + norm * (hw.thickness * 0.5);
                            let pts = vec![to_screen(p0), to_screen(p1), to_screen(p2), to_screen(p3)];
                            vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                        }
                    }
                    Element::Window(win) => {
                        if let Some(Element::Wall(hw)) = doc.elements.get(&win.host_wall) {
                            let center = hw.start + hw.direction() * win.offset_along_wall;
                            let norm = hw.normal();
                            let dir = hw.direction();
                            let p0 = center - dir * (win.width * 0.5) - norm * (hw.thickness * 0.5);
                            let p1 = center + dir * (win.width * 0.5) - norm * (hw.thickness * 0.5);
                            let p2 = center + dir * (win.width * 0.5) + norm * (hw.thickness * 0.5);
                            let p3 = center - dir * (win.width * 0.5) + norm * (hw.thickness * 0.5);
                            let pts = vec![to_screen(p0), to_screen(p1), to_screen(p2), to_screen(p3)];
                            vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                        }
                    }
                    Element::Footing(f) => {
                        let foot = f.footprint();
                        let pts: Vec<Pos2> = foot.iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::Slab(s) => {
                        let pts: Vec<Pos2> = s.boundary.iter().map(|&p| to_screen(p)).collect();
                        if pts.len() >= 3 {
                            draw_polygon_safe(&vp_painter, &pts, Color32::from_rgba_premultiplied(50, 150, 255, 30), sel_stroke);
                        }
                    }
                    Element::Roof(r) => {
                        let pts: Vec<Pos2> = r.boundary.iter().map(|&p| to_screen(p)).collect();
                        if pts.len() >= 3 {
                            draw_polygon_safe(&vp_painter, &pts, Color32::from_rgba_premultiplied(50, 150, 255, 30), sel_stroke);
                        }
                    }
                    Element::Ceiling(c) => {
                        let pts: Vec<Pos2> = c.boundary.iter().map(|&p| to_screen(p)).collect();
                        if pts.len() >= 3 {
                            draw_polygon_safe(&vp_painter, &pts, Color32::from_rgba_premultiplied(50, 150, 255, 30), sel_stroke);
                        }
                    }
                    Element::Stair(s) => {
                        let pts: Vec<Pos2> = s.footprint().iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::Ramp(r) => {
                        let pts: Vec<Pos2> = r.footprint().iter().map(|&p| to_screen(p)).collect();
                        vp_painter.add(egui::Shape::convex_polygon(pts, sel_fill, sel_stroke));
                    }
                    Element::Beam(b) => {
                        vp_painter.line_segment([to_screen(b.start), to_screen(b.end)], sel_stroke);
                    }
                    Element::Railing(rl) => {
                        for pair in rl.path.windows(2) {
                            vp_painter.line_segment([to_screen(pair[0]), to_screen(pair[1])], sel_stroke);
                        }
                    }
                    Element::Fascia(f) => {
                        for pair in f.path.windows(2) {
                            vp_painter.line_segment([to_screen(pair[0]), to_screen(pair[1])], sel_stroke);
                        }
                    }
                    Element::Gutter(g) => {
                        for pair in g.path.windows(2) {
                            vp_painter.line_segment([to_screen(pair[0]), to_screen(pair[1])], sel_stroke);
                        }
                    }
                    Element::FreeformSurface(s) => {
                        let pts: Vec<Pos2> = s.boundary.iter().map(|&p| to_screen(p)).collect();
                        if pts.len() >= 3 {
                            draw_polygon_safe(&vp_painter, &pts, Color32::from_rgba_premultiplied(0, 180, 255, 60), sel_stroke);
                        }
                    }
                    Element::FreeformSolid(f) => {
                        let pts: Vec<Pos2> = f.boundary.iter().map(|&p| to_screen(p)).collect();
                        if pts.len() >= 3 {
                            draw_polygon_safe(&vp_painter, &pts, sel_fill, sel_stroke);
                        }
                    }
                    Element::Dimension(d) => {
                        let diff = d.p2 - d.p1;
                        let len = diff.length();
                        let (p1_dim, p2_dim, mid_dim) = if len > 1e-3 {
                            let dir = diff.normalized();
                            let norm = plinth_geom::vec2(-dir.y, dir.x);
                            let p1d = d.p1 + norm * d.offset;
                            let p2d = d.p2 + norm * d.offset;
                            (p1d, p2d, (p1d + p2d) * 0.5)
                        } else {
                            (d.p1, d.p2, (d.p1 + d.p2) * 0.5)
                        };
                        vp_painter.line_segment([to_screen(d.p1), to_screen(p1_dim)], sel_stroke);
                        vp_painter.line_segment([to_screen(d.p2), to_screen(p2_dim)], sel_stroke);
                        vp_painter.line_segment([to_screen(p1_dim), to_screen(p2_dim)], sel_stroke);

                        // Interactive CAD Grip Handles
                        let grips = [to_screen(d.p1), to_screen(d.p2), to_screen(mid_dim)];
                        for (g_idx, &gp) in grips.iter().enumerate() {
                            let grip_rect = Rect::from_center_size(gp, Vec2::splat(9.0));
                            let is_active = canvas_state.active_annotation_grip == Some((*sel_id, g_idx));
                            let grip_col = if is_active { theme.accent } else { Color32::from_rgb(0, 205, 255) };
                            vp_painter.rect_filled(grip_rect, 1.0, grip_col);
                            vp_painter.rect_stroke(grip_rect, 1.0, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Outside);
                        }
                    }
                    Element::TextNote(t) => {
                        let sp = to_screen(t.pos);
                        let r = Rect::from_center_size(sp, Vec2::new(60.0, 24.0));
                        vp_painter.rect_stroke(r, 2.0, sel_stroke, egui::StrokeKind::Middle);
                        let grip_rect = Rect::from_center_size(sp, Vec2::splat(9.0));
                        let is_active = canvas_state.active_annotation_grip == Some((*sel_id, 0));
                        let grip_col = if is_active { theme.accent } else { Color32::from_rgb(0, 205, 255) };
                        vp_painter.rect_filled(grip_rect, 1.0, grip_col);
                        vp_painter.rect_stroke(grip_rect, 1.0, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Outside);
                    }
                    Element::Leader(l) => {
                        vp_painter.line_segment([to_screen(l.target_pt), to_screen(l.shoulder_pt)], sel_stroke);
                        vp_painter.line_segment([to_screen(l.shoulder_pt), to_screen(l.text_pt)], sel_stroke);
                        let grips = [to_screen(l.target_pt), to_screen(l.shoulder_pt), to_screen(l.text_pt)];
                        for (g_idx, &gp) in grips.iter().enumerate() {
                            let grip_rect = Rect::from_center_size(gp, Vec2::splat(9.0));
                            let is_active = canvas_state.active_annotation_grip == Some((*sel_id, g_idx));
                            let grip_col = if is_active { theme.accent } else { Color32::from_rgb(0, 205, 255) };
                            vp_painter.rect_filled(grip_rect, 1.0, grip_col);
                            vp_painter.rect_stroke(grip_rect, 1.0, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Outside);
                        }
                    }
                    Element::DetailLine(l) => {
                        vp_painter.line_segment([to_screen(l.start), to_screen(l.end)], sel_stroke);
                        let mid = (l.start + l.end) * 0.5;
                        let grips = [to_screen(l.start), to_screen(l.end), to_screen(mid)];
                        for (g_idx, &gp) in grips.iter().enumerate() {
                            let grip_rect = Rect::from_center_size(gp, Vec2::splat(9.0));
                            let is_active = canvas_state.active_annotation_grip == Some((*sel_id, g_idx));
                            let grip_col = if is_active { theme.accent } else { Color32::from_rgb(0, 205, 255) };
                            vp_painter.rect_filled(grip_rect, 1.0, grip_col);
                            vp_painter.rect_stroke(grip_rect, 1.0, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Outside);
                        }
                    }
                    _ => {}
                }
            } else if active_view.kind.is_elevation() {
                // Elevation Selection Highlight (u horizontal, z vertical)
                let project_elev = |p: plinth_geom::Vec3| -> plinth_geom::Vec2 {
                    match active_view.kind {
                        ViewKind::ElevationSouth => plinth_geom::vec2(p.x, p.z),
                        ViewKind::ElevationNorth => plinth_geom::vec2(-p.x, p.z),
                        ViewKind::ElevationEast => plinth_geom::vec2(-p.y, p.z),
                        ViewKind::ElevationWest => plinth_geom::vec2(p.y, p.z),
                        _ => plinth_geom::vec2(p.x, p.z),
                    }
                };
                let draw_box = |u0: f64, u1: f64, z0: f64, z1: f64| {
                    let pts = vec![
                        to_screen(plinth_geom::vec2(u0, z0)),
                        to_screen(plinth_geom::vec2(u1, z0)),
                        to_screen(plinth_geom::vec2(u1, z1)),
                        to_screen(plinth_geom::vec2(u0, z1)),
                    ];
                    draw_polygon_safe(&vp_painter, &pts, sel_fill, sel_stroke);
                };
                match el {
                    Element::Slab(s) => {
                        let (z_bot, z_top) = s.z_bounds(
                            doc.level_elevation(s.level),
                            s.top_level.map(|l| doc.level_elevation(l)),
                        );
                        let us: Vec<f64> = s.boundary.iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                        if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                            draw_box(u_min, u_max, z_bot, z_top);
                        }
                    }
                    Element::Column(c) => {
                        let (z0, z1) = c.z_bounds(
                            doc.level_elevation(c.base_level),
                            c.top_level.map(|l| doc.level_elevation(l)),
                        );
                        let us: Vec<f64> = c.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                        if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                            draw_box(u_min, u_max, z0, z1);
                        }
                    }
                    Element::Beam(b) => {
                        let (z0, z1) = b.z_bounds(
                            doc.level_elevation(b.level),
                            b.top_level.map(|l| doc.level_elevation(l)),
                        );
                        let u0 = project_elev(b.start.extend(0.0)).x;
                        let u1 = project_elev(b.end.extend(0.0)).x;
                        draw_box(u0.min(u1), u0.max(u1), z0, z1);
                    }
                    Element::Wall(w) => {
                        let (z0, z1) = w.z_bounds(
                            doc.level_elevation(w.base_level),
                            w.top_level.map(|l| doc.level_elevation(l)),
                        );
                        let u0 = project_elev(w.start.extend(0.0)).x;
                        let u1 = project_elev(w.end.extend(0.0)).x;
                        draw_box(u0.min(u1), u0.max(u1), z0, z1);
                    }
                    Element::Footing(f) => {
                        let (z0, z1) = f.z_bounds(
                            doc.level_elevation(f.base_level),
                            f.top_level.map(|l| doc.level_elevation(l)),
                        );
                        let us: Vec<f64> = f.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                        if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                            draw_box(u_min, u_max, z0, z1);
                        }
                    }
                    _ => {}
                }
            } else if active_view.kind.is_section() {
                // Section Selection Highlight (t along section cut line, z vertical)
                if let Some((s1, s2)) = active_view.section_line {
                    let line_vec = s2 - s1;
                    let u_dir = if line_vec.length() > 1.0 { line_vec.normalized() } else { plinth_geom::vec2(1.0, 0.0) };
                    let draw_box = |t0: f64, t1: f64, z0: f64, z1: f64| {
                        let pts = vec![
                            to_screen(plinth_geom::vec2(t0, z0)),
                            to_screen(plinth_geom::vec2(t1, z0)),
                            to_screen(plinth_geom::vec2(t1, z1)),
                            to_screen(plinth_geom::vec2(t0, z1)),
                        ];
                        draw_polygon_safe(&vp_painter, &pts, sel_fill, sel_stroke);
                    };
                    match el {
                        Element::Slab(s) => {
                            let (z_bot, z_top) = s.z_bounds(
                                doc.level_elevation(s.level),
                                s.top_level.map(|l| doc.level_elevation(l)),
                            );
                            let ts: Vec<f64> = s.boundary.iter().map(|&p| (p - s1).dot(u_dir)).collect();
                            if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                draw_box(t_min, t_max, z_bot, z_top);
                            }
                        }
                        Element::Column(c) => {
                            let (z0, z1) = c.z_bounds(
                                doc.level_elevation(c.base_level),
                                c.top_level.map(|l| doc.level_elevation(l)),
                            );
                            let ts: Vec<f64> = c.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                            if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                draw_box(t_min, t_max, z0, z1);
                            }
                        }
                        Element::Beam(b) => {
                            let (z0, z1) = b.z_bounds(
                                doc.level_elevation(b.level),
                                b.top_level.map(|l| doc.level_elevation(l)),
                            );
                            let t0 = (b.start - s1).dot(u_dir);
                            let t1 = (b.end - s1).dot(u_dir);
                            draw_box(t0.min(t1), t0.max(t1), z0, z1);
                        }
                        Element::Wall(w) => {
                            let (z0, z1) = w.z_bounds(
                                doc.level_elevation(w.base_level),
                                w.top_level.map(|l| doc.level_elevation(l)),
                            );
                            let t0 = (w.start - s1).dot(u_dir);
                            let t1 = (w.end - s1).dot(u_dir);
                            draw_box(t0.min(t1), t0.max(t1), z0, z1);
                        }
                        Element::Footing(f) => {
                            let (z0, z1) = f.z_bounds(
                                doc.level_elevation(f.base_level),
                                f.top_level.map(|l| doc.level_elevation(l)),
                            );
                            let ts: Vec<f64> = f.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                            if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                draw_box(t_min, t_max, z0, z1);
                            }
                        }
                        _ => {}
                    }
                }
            } else if !rendered_gpu {
                // 3D View Selection Highlight: Only used in software CPU fallback if GPU viewport is unavailable.
                // Strict camera frustum clipping and boundary check completely prevents ray shooting and screen-splitting polygons.
                let cam_forward = plinth_geom::Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);
                for face in doc.element_faces(*sel_id) {
                    if face.normal.dot(cam_forward) >= -1e-4 && pane_visual_style != VisualStyle::Wireframe {
                        continue;
                    }
                    let mut all_valid = true;
                    if is_persp {
                        for p in &face.pts {
                            let d = *p - target_3d;
                            let y1 = d.x * sin_y + d.y * cos_y;
                            let z1 = d.z;
                            let y2 = y1 * cos_p - z1 * sin_p;
                            if (cam_dist + y2) <= 100.0 {
                                all_valid = false;
                                break;
                            }
                        }
                    }
                    if !all_valid {
                        continue;
                    }
                    let pts: Vec<Pos2> = face.pts.iter().map(|&p| project_3d(p)).collect();
                    let n = pts.len();
                    if pts.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                        continue;
                    }
                    let clip_bounds = vp_rect.expand(200.0);
                    let min_x = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                    let max_x = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                    let min_y = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                    let max_y = pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                    if max_x < clip_bounds.min.x || min_x > clip_bounds.max.x || max_y < clip_bounds.min.y || min_y > clip_bounds.max.y {
                        continue;
                    }
                    if n >= 3 && pane_visual_style != VisualStyle::Wireframe {
                        draw_polygon_safe(&vp_painter, &pts, sel_fill, Stroke::NONE);
                    }
                    for i in 0..n {
                        if face.hard.get(i).copied().unwrap_or(true) {
                            draw_line_segment_safe(&vp_painter, pts[i], pts[(i + 1) % n], sel_stroke);
                        }
                    }
                }
            }
        }
    }

    // 4. Interactive Tool Previews and Snapping Cursor (only shown when this pane is active and hovered)
    if is_active && is_mouse_in_vp {
        // 4. Interactive Tool Previews and Snapping Cursor (only shown when this pane is active and hovered)
        if let Some(ref snap) = canvas_state.active_snap {
            let snap_cyan = Color32::from_rgb(0, 220, 255);
            let snap_stroke = Stroke::new(1.8, snap_cyan);
            let p = snap.screen_pos;

            match snap.kind {
                SnapKind::Endpoint => {
                    // Square glyph
                    let r = Rect::from_center_size(p, Vec2::splat(9.0));
                    vp_painter.rect_stroke(r, 0.5, snap_stroke, egui::StrokeKind::Middle);
                }
                SnapKind::Midpoint => {
                    // Triangle glyph
                    let s = 5.5;
                    let tri = vec![
                        Pos2::new(p.x, p.y - s),
                        Pos2::new(p.x - s * 1.15, p.y + s * 0.8),
                        Pos2::new(p.x + s * 1.15, p.y + s * 0.8),
                    ];
                    vp_painter.add(egui::Shape::convex_polygon(tri, Color32::TRANSPARENT, snap_stroke));
                }
                SnapKind::Center => {
                    // Circle glyph
                    vp_painter.circle_stroke(p, 5.0, snap_stroke);
                }
                SnapKind::Intersection => {
                    // Cross 'X' glyph
                    let s = 4.5;
                    vp_painter.line_segment([Pos2::new(p.x - s, p.y - s), Pos2::new(p.x + s, p.y + s)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x - s, p.y + s), Pos2::new(p.x + s, p.y - s)], snap_stroke);
                }
                SnapKind::Perpendicular => {
                    // Right-angle glyph
                    let s = 6.0;
                    vp_painter.line_segment([Pos2::new(p.x, p.y - s), Pos2::new(p.x, p.y)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x, p.y), Pos2::new(p.x + s, p.y)], snap_stroke);
                }
                SnapKind::PolarRay => {
                    // Dashed polar alignment ray
                    if let (Some(r_start), Some(r_end)) = (snap.ray_start, snap.ray_end) {
                        let polar_color = Color32::from_rgb(0, 230, 118);
                        let ray_dir = (r_end - r_start).normalized();
                        let total_len = (r_end - r_start).length();
                        let mut curr = 0.0;
                        while curr < total_len.min(2500.0) {
                            let p1 = r_start + ray_dir * curr;
                            let p2 = r_start + ray_dir * (curr + 6.0).min(total_len);
                            vp_painter.line_segment([p1, p2], Stroke::new(1.2, polar_color));
                            curr += 10.0;
                        }
                        vp_painter.circle_stroke(p, 4.0, Stroke::new(1.5, polar_color));
                    }
                }
                SnapKind::OrthoRay => {
                    // Dashed ortho alignment ray
                    if let (Some(r_start), Some(r_end)) = (snap.ray_start, snap.ray_end) {
                        let ortho_color = Color32::from_rgb(255, 152, 0);
                        let ray_dir = (r_end - r_start).normalized();
                        let total_len = (r_end - r_start).length();
                        let mut curr = 0.0;
                        while curr < total_len.min(2500.0) {
                            let p1 = r_start + ray_dir * curr;
                            let p2 = r_start + ray_dir * (curr + 6.0).min(total_len);
                            vp_painter.line_segment([p1, p2], Stroke::new(1.2, ortho_color));
                            curr += 10.0;
                        }
                        let r = Rect::from_center_size(p, Vec2::splat(7.0));
                        vp_painter.rect_stroke(r, 0.5, Stroke::new(1.5, ortho_color), egui::StrokeKind::Middle);
                    }
                }
                SnapKind::Edge => {
                    // Hourglass glyph
                    let s = 5.0;
                    vp_painter.line_segment([Pos2::new(p.x - s, p.y - s), Pos2::new(p.x + s, p.y - s)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x + s, p.y - s), Pos2::new(p.x - s, p.y + s)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x - s, p.y + s), Pos2::new(p.x + s, p.y + s)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x + s, p.y + s), Pos2::new(p.x - s, p.y - s)], snap_stroke);
                }
                SnapKind::WallCenterline => {
                    // Centerline CL glyph
                    let s = 6.0;
                    vp_painter.line_segment([Pos2::new(p.x - s, p.y), Pos2::new(p.x + s, p.y)], snap_stroke);
                    vp_painter.line_segment([Pos2::new(p.x, p.y - s), Pos2::new(p.x, p.y + s)], snap_stroke);
                    vp_painter.circle_stroke(p, 4.0, snap_stroke);
                }
                _ => {}
            }

            // Snap badge near cursor
            let badge_pos = Pos2::new(p.x + 12.0, p.y - 14.0);
            let galley = vp_painter.layout_no_wrap(snap.label.clone(), egui::FontId::proportional(11.0), Color32::WHITE);
            let badge_rect = Rect::from_min_size(badge_pos, galley.size() + Vec2::new(8.0, 4.0));
            vp_painter.rect_filled(badge_rect, 3.0, Color32::from_rgba_unmultiplied(20, 24, 30, 220));
            vp_painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0, snap_cyan), egui::StrokeKind::Middle);
            vp_painter.text(badge_pos + Vec2::new(4.0, 2.0), egui::Align2::LEFT_TOP, &snap.label, egui::FontId::proportional(11.0), Color32::WHITE);
        }

        // Crosshair snap cursor
        vp_painter.line_segment([Pos2::new(mouse_screen.x - 7.0, mouse_screen.y), Pos2::new(mouse_screen.x + 7.0, mouse_screen.y)], Stroke::new(1.0, theme.accent));
        vp_painter.line_segment([Pos2::new(mouse_screen.x, mouse_screen.y - 7.0), Pos2::new(mouse_screen.x, mouse_screen.y + 7.0)], Stroke::new(1.0, theme.accent));

        let preview_stroke = Stroke::new(1.5, theme.accent);
        let preview_fill = Color32::from_rgba_premultiplied(0, 122, 217, 60);
        let shift_held = ui.input(|i| i.modifiers.shift);

        match active_tool {
            Tool::DrawWall { start: Some(p1), is_structural } => {
                let p2 = mouse_pos;
                let dist = p1.distance(p2);
            if dist > 1.0 {
                let wall_h = tool_studio.preview_wall_height(*is_structural);
                let wall_t = tool_studio.preview_wall_thickness(*is_structural);
                let dir = (p2 - *p1).normalized();
                let norm = dir.perp() * (wall_t * 0.5);

                let b0 = *p1 - norm;
                let b1 = p2 - norm;
                let b2 = p2 + norm;
                let b3 = *p1 + norm;

                let sp_b0 = project_point(b0, active_elev);
                let sp_b1 = project_point(b1, active_elev);
                let sp_b2 = project_point(b2, active_elev);
                let sp_b3 = project_point(b3, active_elev);

                if active_view.kind.is_3d() {
                    let top_z = active_elev + wall_h;
                    let sp_t0 = project_point(b0, top_z);
                    let sp_t1 = project_point(b1, top_z);
                    let sp_t2 = project_point(b2, top_z);
                    let sp_t3 = project_point(b3, top_z);

                    let side_fill = Color32::from_rgba_unmultiplied(0, 180, 255, 65);
                    let top_fill = Color32::from_rgba_unmultiplied(0, 210, 255, 100);
                    let rim_stroke = Stroke::new(1.8, Color32::from_rgb(0, 215, 255));

                    // 4 Side quads
                    painter.add(egui::Shape::convex_polygon(vec![sp_b0, sp_b1, sp_t1, sp_t0], side_fill, rim_stroke));
                    painter.add(egui::Shape::convex_polygon(vec![sp_b1, sp_b2, sp_t2, sp_t1], side_fill, rim_stroke));
                    painter.add(egui::Shape::convex_polygon(vec![sp_b2, sp_b3, sp_t3, sp_t2], side_fill, rim_stroke));
                    painter.add(egui::Shape::convex_polygon(vec![sp_b3, sp_b0, sp_t0, sp_t3], side_fill, rim_stroke));
                    // Top cap
                    painter.add(egui::Shape::convex_polygon(vec![sp_t0, sp_t1, sp_t2, sp_t3], top_fill, rim_stroke));

                    // Floating 3D Badge
                    let mid_top = Pos2::new((sp_t0.x + sp_t2.x) * 0.5, (sp_t0.y + sp_t2.y) * 0.5 - 20.0);
                    let badge_text = format!("L: {:.0} mm  •  H: {:.0} mm", dist, wall_h);
                    let badge_rect = Rect::from_center_size(mid_top, Vec2::new(150.0, 24.0));
                    painter.rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(15, 25, 35, 220));
                    painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, Color32::from_rgb(0, 210, 255)), egui::StrokeKind::Inside);
                    painter.text(mid_top, egui::Align2::CENTER_CENTER, badge_text, egui::FontId::proportional(11.5), Color32::WHITE);
                } else {
                    let fill = Color32::from_rgba_unmultiplied(0, 150, 255, 70);
                    let stroke = Stroke::new(1.8, Color32::from_rgb(0, 180, 255));
                    painter.add(egui::Shape::convex_polygon(vec![sp_b0, sp_b1, sp_b2, sp_b3], fill, stroke));

                    let mid_sp = Pos2::new((sp_b0.x + sp_b2.x) * 0.5, (sp_b0.y + sp_b2.y) * 0.5 - 18.0);
                    let badge_text = format!("{:.0} mm (T: {:.0})", dist, wall_t);
                    let badge_rect = Rect::from_center_size(mid_sp, Vec2::new(120.0, 22.0));
                    painter.rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(20, 25, 35, 220));
                    painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, Color32::from_rgb(0, 180, 255)), egui::StrokeKind::Inside);
                    painter.text(mid_sp, egui::Align2::CENTER_CENTER, badge_text, egui::FontId::proportional(11.0), Color32::WHITE);
                }
            }
        }
        Tool::DrawCurtainWall { start: Some(p1) } => {
            let p2 = mouse_pos;
            let dist = p1.distance(p2);
            if dist > 1.0 {
                // Check if near an existing wall
                let mut near_wall = false;
                for el in doc.elements.values() {
                    if let Element::Wall(w) = el {
                        let (proj1, _) = plinth_geom::polygon::closest_point_on_segment(*p1, w.start, w.end);
                        let (proj2, _) = plinth_geom::polygon::closest_point_on_segment(p2, w.start, w.end);
                        if proj1.distance(*p1) < 400.0 && proj2.distance(p2) < 400.0 {
                            near_wall = true;
                            break;
                        }
                    }
                }

                let dir = (p2 - *p1).normalized();
                let norm = dir.perp() * 75.0; // half 150mm mullion depth
                let b0 = *p1 - norm;
                let b1 = p2 - norm;
                let b2 = p2 + norm;
                let b3 = *p1 + norm;

                let sp_b0 = project_point(b0, active_elev);
                let sp_b1 = project_point(b1, active_elev);
                let sp_b2 = project_point(b2, active_elev);
                let sp_b3 = project_point(b3, active_elev);

                let fill = Color32::from_rgba_unmultiplied(80, 180, 240, 80);
                let stroke = Stroke::new(2.0, if near_wall { Color32::from_rgb(255, 120, 0) } else { Color32::from_rgb(80, 190, 255) });
                painter.add(egui::Shape::convex_polygon(vec![sp_b0, sp_b1, sp_b2, sp_b3], fill, stroke));

                let mid_sp = Pos2::new((sp_b0.x + sp_b2.x) * 0.5, (sp_b0.y + sp_b2.y) * 0.5 - 20.0);
                let badge_text = if near_wall {
                    format!("Curtain Wall: {:.0} mm [Auto-Embed Wall Hole]", dist)
                } else {
                    format!("Curtain Wall: {:.0} mm", dist)
                };
                let badge_rect = Rect::from_center_size(mid_sp, Vec2::new(if near_wall { 270.0 } else { 150.0 }, 22.0));
                painter.rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(20, 28, 38, 230));
                painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, stroke.color), egui::StrokeKind::Inside);
                painter.text(mid_sp, egui::Align2::CENTER_CENTER, badge_text, egui::FontId::proportional(11.0), Color32::WHITE);
            }
        }
        Tool::DrawRoof { points, .. } => {
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([project_point(points[i], active_elev), project_point(points[i + 1], active_elev)], Stroke::new(2.0, Color32::from_rgb(230, 120, 40)));
            }
            if let Some(last) = points.last() {
                painter.line_segment([project_point(*last, active_elev), mouse_screen], Stroke::new(1.5, Color32::from_rgb(230, 120, 40)));
            }
            if points.len() >= 2 {
                let mut poly_pts: Vec<Pos2> = points.iter().map(|&p| project_point(p, active_elev)).collect();
                poly_pts.push(mouse_screen);
                painter.add(egui::Shape::convex_polygon(poly_pts, Color32::from_rgba_unmultiplied(230, 120, 40, 50), Stroke::NONE));
            }
        }
        Tool::DrawFascia { points } => {
            let color = Color32::from_rgb(200, 210, 230);
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([project_point(points[i], active_elev), project_point(points[i + 1], active_elev)], Stroke::new(2.5, color));
            }
            if let Some(last) = points.last() {
                painter.line_segment([project_point(*last, active_elev), mouse_screen], Stroke::new(1.8, color));
            }
        }
        Tool::DrawGutter { points } => {
            let color = Color32::from_rgb(100, 180, 220);
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([project_point(points[i], active_elev), project_point(points[i + 1], active_elev)], Stroke::new(2.5, color));
            }
            if let Some(last) = points.last() {
                painter.line_segment([project_point(*last, active_elev), mouse_screen], Stroke::new(1.8, color));
            }
        }
        Tool::DrawColumn { is_structural } => {
            let mut col = Column::new(mouse_pos, doc.active_level_id);
            tool_studio.configure_new_column(doc, &mut col, *is_structural);
            let pts: Vec<Pos2> = col.footprint().iter().map(|&p| project_point(p, active_elev)).collect();
            painter.add(egui::Shape::convex_polygon(pts, preview_fill, preview_stroke));
        }
        Tool::DrawFooting => {
            let mut ft = Footing::new(mouse_pos, doc.active_level_id);
            tool_studio.configure_new_footing(doc, &mut ft);
            for el in doc.elements.values() {
                if let Element::Column(c) = el {
                    if c.location.distance(mouse_pos) < 350.0 {
                        ft.location = c.location;
                        ft.rotation = c.rotation;
                        ft.width = ft.width.max(c.width + 400.0);
                        ft.length = ft.length.max(c.depth + 400.0);
                        break;
                    }
                }
            }
            let foot = ft.footprint();
            if active_view.kind.is_3d() {
                let z_top = active_elev;
                let z_bot = active_elev - ft.thickness;
                let sp_b: Vec<Pos2> = foot.iter().map(|&p| project_point(p, z_bot)).collect();
                let sp_t: Vec<Pos2> = foot.iter().map(|&p| project_point(p, z_top)).collect();
                let side_fill = Color32::from_rgba_unmultiplied(255, 130, 30, 65);
                let top_fill = Color32::from_rgba_unmultiplied(255, 160, 50, 95);
                let rim_stroke = Stroke::new(1.8, Color32::from_rgb(255, 135, 20));
                for i in 0..4 {
                    let j = (i + 1) % 4;
                    painter.add(egui::Shape::convex_polygon(
                        vec![sp_b[i], sp_b[j], sp_t[j], sp_t[i]],
                        side_fill,
                        rim_stroke,
                    ));
                }
                painter.add(egui::Shape::convex_polygon(sp_t, top_fill, rim_stroke));
            } else {
                let pts: Vec<Pos2> = foot.iter().map(|&p| project_point(p, active_elev)).collect();
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(255, 135, 30, 55),
                    Stroke::new(1.8, Color32::from_rgb(255, 125, 15)),
                ));
            }
        }
        Tool::DrawBeam { start: Some(p1) } => {
            painter.line_segment([project_point(*p1, active_elev), mouse_screen], preview_stroke);
        }
        Tool::DrawSlab { points, .. }
        | Tool::DrawCeiling { points, .. }
        | Tool::DrawRailing { points }
        | Tool::DrawBuildingPad { points }
        | Tool::DrawToposolid { points }
        | Tool::DrawPropertyLine { points }
        | Tool::DrawRoad { points }
        | Tool::DrawSidewalk { points }
        | Tool::DrawPathwalk { points } => {
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([project_point(points[i], active_elev), project_point(points[i + 1], active_elev)], preview_stroke);
            }
            if let Some(last) = points.last() {
                painter.line_segment([project_point(*last, active_elev), mouse_screen], preview_stroke);
            }
        }
        Tool::DrawStair { start: Some(p1) } | Tool::DrawRamp { start: Some(p1) } => {
            let p2 = mouse_pos;
            let dir = (p2 - *p1).normalized();
            let norm = dir.perp() * 500.0;
            let poly = vec![
                project_point(*p1 - norm, active_elev),
                project_point(*p1 + norm, active_elev),
                project_point(p2 + norm, active_elev),
                project_point(p2 - norm, active_elev),
            ];
            painter.add(egui::Shape::convex_polygon(poly, preview_fill, preview_stroke));
            painter.line_segment([project_point(*p1, active_elev), mouse_screen], preview_stroke);
        }
        Tool::DrawGrid { start: Some(p1) } | Tool::DrawDetailLine { start: Some(p1) } => {
            painter.line_segment([project_point(*p1, active_elev), mouse_screen], preview_stroke);
        }
        Tool::DrawDimension { start: Some(p1), p2, locked_offset } => {
            if let Some(second_pt) = p2 {
                // STRETCH MODE: p1 and p2 chosen, cursor dynamically stretches dimension string line!
                let diff = *second_pt - *p1;
                if diff.length() > 1.0 {
                    let dir = diff.normalized();
                    let norm = plinth_geom::vec2(-dir.y, dir.x);
                    let mid = (*p1 + *second_pt) * 0.5;
                    let mut offset = (mouse_pos - mid).dot(norm);
                    if offset.abs() < 10.0 {
                        offset = if offset >= 0.0 { 250.0 } else { -250.0 };
                    }

                    // Collinear alignment snap check
                    for el in doc.elements.values() {
                        if let Element::Dimension(d_other) = el {
                            if d_other.kind == plinth_model::DimensionKind::Linear {
                                let o_diff = d_other.p2 - d_other.p1;
                                if o_diff.length() > 10.0 {
                                    let o_dir = o_diff.normalized();
                                    if (dir.dot(o_dir)).abs() > 0.95 {
                                        let o_norm = plinth_geom::vec2(-o_dir.y, o_dir.x);
                                        let o_p_dim = d_other.p1 + o_norm * d_other.offset;
                                        let target_offset = (o_p_dim - *p1).dot(norm);
                                        if (target_offset - offset).abs() < 300.0 {
                                            offset = target_offset;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let p1_dim = *p1 + norm * offset;
                    let p2_dim = *second_pt + norm * offset;
                    let ext = norm * (if offset >= 0.0 { 150.0 } else { -150.0 });

                    let sp1 = project_point(*p1, active_elev);
                    let sp2 = project_point(*second_pt, active_elev);
                    let sp1_ext = project_point(p1_dim + ext, active_elev);
                    let sp2_ext = project_point(p2_dim + ext, active_elev);
                    let sp1_dim = project_point(p1_dim, active_elev);
                    let sp2_dim = project_point(p2_dim, active_elev);

                    let dim_stroke = Stroke::new(1.4, theme.accent);
                    painter.line_segment([sp1, sp1_ext], dim_stroke);
                    painter.line_segment([sp2, sp2_ext], dim_stroke);
                    painter.line_segment([sp1_dim, sp2_dim], Stroke::new(1.8, theme.accent));

                    // Dynamic tick marks on stretched ends
                    let tick_v = (dir + norm).normalized() * 7.0;
                    let sv1 = Vec2::new(tick_v.x as f32, -tick_v.y as f32);
                    painter.line_segment([sp1_dim - sv1, sp1_dim + sv1], Stroke::new(1.8, theme.accent));
                    painter.line_segment([sp2_dim - sv1, sp2_dim + sv1], Stroke::new(1.8, theme.accent));

                    let mid_dim_sp = sp1_dim + (sp2_dim - sp1_dim) * 0.5;
                    painter.text(
                        mid_dim_sp + Vec2::new(0.0, -12.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.0} mm (Stretch: {:.0} mm)", diff.length(), offset),
                        egui::FontId::proportional(12.0),
                        theme.accent,
                    );
                }
            } else if let Some(off) = locked_offset {
                // CHAINED MODE: offset is locked, stretching to next witness point!
                let diff = mouse_pos - *p1;
                if diff.length() > 1.0 {
                    let dir = diff.normalized();
                    let norm = plinth_geom::vec2(-dir.y, dir.x);
                    let p1_dim = *p1 + norm * *off;
                    let p2_dim = mouse_pos + norm * *off;
                    let ext = norm * (if *off >= 0.0 { 150.0 } else { -150.0 });

                    let sp1 = project_point(*p1, active_elev);
                    let sp2 = project_point(mouse_pos, active_elev);
                    let sp1_ext = project_point(p1_dim + ext, active_elev);
                    let sp2_ext = project_point(p2_dim + ext, active_elev);
                    let sp1_dim = project_point(p1_dim, active_elev);
                    let sp2_dim = project_point(p2_dim, active_elev);

                    painter.line_segment([sp1, sp1_ext], Stroke::new(1.2, theme.accent));
                    painter.line_segment([sp2, sp2_ext], Stroke::new(1.2, theme.accent));
                    painter.line_segment([sp1_dim, sp2_dim], Stroke::new(1.6, theme.accent));
                    let mid_dim_sp = sp1_dim + (sp2_dim - sp1_dim) * 0.5;
                    painter.text(
                        mid_dim_sp + Vec2::new(0.0, -12.0),
                        egui::Align2::CENTER_CENTER,
                        format!("🔗 {:.0} mm", diff.length()),
                        egui::FontId::proportional(11.5),
                        theme.accent,
                    );
                }
            } else {
                // FIRST SEGMENT: P1 chosen, dragging to P2
                let sp1 = project_point(*p1, active_elev);
                painter.line_segment([sp1, mouse_screen], preview_stroke);
                let dist = p1.distance(mouse_pos);
                painter.text(
                    mouse_screen + Vec2::new(12.0, -12.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("📏 {:.0} mm", dist),
                    egui::FontId::proportional(11.0),
                    theme.accent,
                );
            }
        }
        Tool::DrawCircularDimension { center: Some(c) } => {
            let sc = project_point(*c, active_elev);
            let r_px = sc.distance(mouse_screen);
            painter.circle_stroke(sc, r_px, preview_stroke);
            painter.line_segment([sc, mouse_screen], preview_stroke);
            let rad_mm = c.distance(mouse_pos);
            painter.text(mouse_screen + Vec2::new(10.0, -10.0), egui::Align2::LEFT_BOTTOM, format!("R {:.1} mm", rad_mm), egui::FontId::proportional(11.0), theme.accent);
        }
        Tool::DrawAngularDimension { apex: Some(a), first_ray: None } => {
            let sa = project_point(*a, active_elev);
            painter.line_segment([sa, mouse_screen], preview_stroke);
        }
        Tool::DrawAngularDimension { apex: Some(a), first_ray: Some(r1) } => {
            let sa = project_point(*a, active_elev);
            let sr1 = project_point(*r1, active_elev);
            painter.line_segment([sa, sr1], preview_stroke);
            painter.line_segment([sa, mouse_screen], preview_stroke);
            let v1 = *r1 - *a;
            let v2 = mouse_pos - *a;
            let l1 = v1.length();
            let l2 = v2.length();
            if l1 > 1.0 && l2 > 1.0 {
                let cos_theta = ((v1.x * v2.x + v1.y * v2.y) / (l1 * l2)).clamp(-1.0, 1.0);
                let deg = cos_theta.acos().to_degrees();
                painter.text(mouse_screen + Vec2::new(10.0, -10.0), egui::Align2::LEFT_BOTTOM, format!("{:.1}°", deg), egui::FontId::proportional(11.0), theme.accent);
            }
        }
        Tool::DrawTextAnnotation { .. } => {
            painter.text(mouse_screen, egui::Align2::LEFT_TOP, format!("📝 {}", canvas_state.annotation_settings.default_text), egui::FontId::proportional(canvas_state.annotation_settings.font_size), theme.accent);
        }
        Tool::DrawLeader { target_pt: Some(t), shoulder_pt: None } => {
            let st = project_point(*t, active_elev);
            painter.line_segment([st, mouse_screen], preview_stroke);
        }
        Tool::DrawLeader { target_pt: Some(t), shoulder_pt: Some(s) } => {
            let st = project_point(*t, active_elev);
            let ss = project_point(*s, active_elev);
            painter.line_segment([st, ss], preview_stroke);
            painter.line_segment([ss, mouse_screen], preview_stroke);
            painter.text(mouse_screen + Vec2::new(5.0, -10.0), egui::Align2::LEFT_BOTTOM, format!("⮤ {}", canvas_state.annotation_settings.default_text), egui::FontId::proportional(canvas_state.annotation_settings.font_size), theme.accent);
        }
        Tool::Move { from: Some(base) } => {
            let base_screen = project_point(*base, active_elev);
            let dist_mm = base.distance(mouse_pos);
            let delta_3d = if let (Some(b3), Some(t3)) = (canvas_state.move_base_3d, canvas_state.snapped_3d_pos) {
                t3 - b3
            } else {
                plinth_geom::Vec3::new(mouse_pos.x - base.x, mouse_pos.y - base.y, 0.0)
            };
            let stroke_color = if delta_3d.x.abs() > 0.0 && delta_3d.y.abs() < 1e-2 && delta_3d.z.abs() < 1e-2 {
                Color32::from_rgb(235, 60, 60) // Red X
            } else if delta_3d.y.abs() > 0.0 && delta_3d.x.abs() < 1e-2 && delta_3d.z.abs() < 1e-2 {
                Color32::from_rgb(60, 200, 60) // Green Y
            } else if delta_3d.z.abs() > 0.0 && delta_3d.x.abs() < 1e-2 && delta_3d.y.abs() < 1e-2 {
                Color32::from_rgb(60, 140, 240) // Blue Z
            } else {
                theme.accent
            };
            painter.line_segment([base_screen, mouse_screen], Stroke::new(2.0, stroke_color));
            painter.circle_filled(base_screen, 4.0, stroke_color);
            painter.circle_stroke(base_screen, 7.0, Stroke::new(1.5, stroke_color));
            painter.circle_filled(mouse_screen, 4.0, stroke_color);

            let hud_text = format!(
                "Move: ΔX: {:+.0} mm  ΔY: {:+.0} mm  ΔZ: {:+.0} mm  ({:.0} mm)  [X, Y, Z, Shift=Ortho]",
                delta_3d.x, delta_3d.y, delta_3d.z, dist_mm
            );
            painter.text(mouse_screen + Vec2::new(12.0, -14.0), egui::Align2::LEFT_BOTTOM, hud_text, egui::FontId::proportional(11.0), stroke_color);
        }
        Tool::Rotate { center: Some(c), start_angle } => {
            let c_screen = project_point(*c, active_elev);
            painter.circle_filled(c_screen, 4.0, theme.accent);
            painter.circle_stroke(c_screen, 8.0, Stroke::new(1.8, theme.accent));
            match start_angle {
                None => {
                    painter.line_segment([c_screen, mouse_screen], Stroke::new(1.5, theme.accent));
                    painter.text(mouse_screen + Vec2::new(12.0, -14.0), egui::Align2::LEFT_BOTTOM, "Rotate: Pick Reference Start Ray", egui::FontId::proportional(11.0), theme.accent);
                }
                Some(a1) => {
                    let ray_len = 80.0_f32;
                    let p1_ray = c_screen + Vec2::new((*a1 as f32).cos(), -(*a1 as f32).sin()) * ray_len;
                    painter.line_segment([c_screen, p1_ray], Stroke::new(1.5, Color32::from_rgb(180, 180, 180)));
                    painter.line_segment([c_screen, mouse_screen], Stroke::new(2.0, theme.accent));
                    let mut a2 = (mouse_pos - *c).angle();
                    if shift_held {
                        let step = 15.0_f64.to_radians();
                        a2 = (a2 / step).round() * step;
                    }
                    let deg = (a2 - *a1).to_degrees();
                    let hud_text = format!("Rotate: {:+.1}°  [Shift=15° Increments]", deg);
                    painter.text(mouse_screen + Vec2::new(12.0, -14.0), egui::Align2::LEFT_BOTTOM, hud_text, egui::FontId::proportional(11.0), theme.accent);
                }
            }
        }
        Tool::Mirror { axis_p1: Some(p1) } => {
            let sp1 = project_point(*p1, active_elev);
            let mut p2 = mouse_pos;
            if shift_held {
                let dx = (p2.x - p1.x).abs();
                let dy = (p2.y - p1.y).abs();
                if dx >= dy { p2.y = p1.y; } else { p2.x = p1.x; }
            }
            let sp2 = project_point(p2, active_elev);
            let dist_mm = p1.distance(p2);
            let axis_stroke = Stroke::new(2.0, Color32::from_rgb(255, 120, 0));
            painter.line_segment([sp1, sp2], axis_stroke);
            painter.circle_filled(sp1, 4.0, Color32::from_rgb(255, 120, 0));
            painter.circle_filled(sp2, 4.0, Color32::from_rgb(255, 120, 0));
            let hud_text = format!("Mirror Axis: {:.0} mm  [Shift=Ortho Axis]", dist_mm);
            painter.text(sp2 + Vec2::new(12.0, -14.0), egui::Align2::LEFT_BOTTOM, hud_text, egui::FontId::proportional(11.0), Color32::from_rgb(255, 120, 0));
        }
        Tool::DrawRectSurface { corner1: Some(p1) } | Tool::DrawBoxPrimitive { corner1: Some(p1) } => {
            let p2 = mouse_pos;
            let min_x = p1.x.min(p2.x);
            let max_x = p1.x.max(p2.x);
            let min_y = p1.y.min(p2.y);
            let max_y = p1.y.max(p2.y);
            let corners = [
                ModelVec2::new(min_x, min_y),
                ModelVec2::new(max_x, min_y),
                ModelVec2::new(max_x, max_y),
                ModelVec2::new(min_x, max_y),
            ];
            let pts_screen: Vec<Pos2> = corners.iter().map(|&p| project_point(p, active_elev)).collect();
            painter.add(egui::Shape::convex_polygon(pts_screen, Color32::from_rgba_premultiplied(0, 150, 255, 60), preview_stroke));
            let w = (p1.x - p2.x).abs();
            let l = (p1.y - p2.y).abs();
            let center_uv = ModelVec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
            let center_sp = project_point(center_uv, active_elev);
            painter.text(center_sp, egui::Align2::CENTER_CENTER, format!("{:.0} × {:.0} mm", w, l), egui::FontId::proportional(11.0), theme.accent);
        }
        Tool::DrawCircleSurface { center: Some(c) } | Tool::DrawCylinderPrimitive { center: Some(c) } | Tool::DrawSpherePrimitive { center: Some(c) } => {
            let rad_world = c.distance(mouse_pos);
            let sc = project_point(*c, active_elev);
            let segs = 32;
            let pts_screen: Vec<Pos2> = (0..segs).map(|i| {
                let theta = (i as f64 / segs as f64) * std::f64::consts::TAU;
                let pt = ModelVec2::new(c.x + rad_world * theta.cos(), c.y + rad_world * theta.sin());
                project_point(pt, active_elev)
            }).collect();
            painter.add(egui::Shape::convex_polygon(pts_screen, Color32::from_rgba_premultiplied(0, 150, 255, 60), preview_stroke));
            painter.line_segment([sc, mouse_screen], preview_stroke);
            painter.text(mouse_screen + Vec2::new(10.0, -10.0), egui::Align2::LEFT_CENTER, format!("R: {:.0} mm", rad_world), egui::FontId::proportional(11.0), theme.accent);
        }
        Tool::DrawPolygonSurface { points } => {
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([project_point(points[i], active_elev), project_point(points[i + 1], active_elev)], preview_stroke);
            }
            if let Some(last) = points.last() {
                painter.line_segment([project_point(*last, active_elev), mouse_screen], preview_stroke);
            }
        }
        Tool::PushPull { target_id, drag_start, current_height } => {
            if let Some(t_id) = target_id {
                let h = if let Some(ds) = drag_start {
                    ((mouse_pos.y - ds.y).abs() * 2.0).clamp(100.0, 50000.0)
                } else {
                    *current_height
                };
                if let Some(el) = doc.elements.get(t_id) {
                    let (boundary, base_elev) = match el {
                        Element::FreeformSurface(s) => (&s.boundary, s.elevation),
                        Element::FreeformSolid(f) => (&f.boundary, f.z_min),
                        _ => (&Vec::new(), active_elev),
                    };
                    let n = boundary.len();
                    if n >= 3 {
                        let orange_fill = Color32::from_rgba_premultiplied(255, 140, 0, 70);
                        let orange_stroke = Stroke::new(1.8, Color32::from_rgb(255, 120, 0));
                        // 1. Draw 3D side quads
                        for i in 0..n {
                            let p0 = boundary[i];
                            let p1 = boundary[(i + 1) % n];
                            let quad = vec![
                                project_point(p0, base_elev),
                                project_point(p1, base_elev),
                                project_point(p1, base_elev + h),
                                project_point(p0, base_elev + h),
                            ];
                            painter.add(egui::Shape::convex_polygon(quad, orange_fill, orange_stroke));
                        }
                        // 2. Draw 3D top cap
                        let top_pts: Vec<Pos2> = boundary.iter().map(|&p| project_point(p, base_elev + h)).collect();
                        painter.add(egui::Shape::convex_polygon(top_pts, Color32::from_rgba_premultiplied(255, 180, 50, 90), orange_stroke));
                        let badge_pos = project_point(boundary[0], base_elev + h) + Vec2::new(0.0, -20.0);
                        painter.text(badge_pos, egui::Align2::CENTER_CENTER, format!("Push/Pull Height: {:.0} mm (Click to commit)", h), egui::FontId::proportional(12.0), Color32::from_rgb(255, 120, 0));
                    }
                }
            } else {
                // Highlight surface under cursor
                for (_id, el) in &doc.elements {
                    if let Element::FreeformSurface(s) = el {
                        let hit = if is_3d {
                            let scr_pts: Vec<Pos2> = s.boundary.iter().map(|&p| project_point(p, s.elevation)).collect();
                            if let Some(hp) = ui.input(|i| i.pointer.hover_pos()) {
                                is_point_in_quad(hp, &scr_pts)
                            } else {
                                false
                            }
                        } else {
                            plinth_geom::polygon::point_in_polygon(mouse_pos, &s.boundary)
                        };
                        if hit {
                            let pts: Vec<Pos2> = s.boundary.iter().map(|&p| project_point(p, s.elevation)).collect();
                            painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgba_premultiplied(0, 200, 255, 90), Stroke::new(2.0, Color32::from_rgb(0, 180, 255))));
                            painter.text(mouse_screen + Vec2::new(12.0, -12.0), egui::Align2::LEFT_CENTER, "Click to Push/Pull", egui::FontId::proportional(11.0), theme.accent);
                            break;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    }

    // 4. Render Floating ViewCube, Visual Style Selector & Header Badge
    let mut floating_consumed = false;

    if active_view.kind.is_3d() {
        if let Some(v_mut) = doc.views.get_mut(&view_id) {
            if let Some(cam) = &mut v_mut.camera_3d {
                if render_viewcube(painter, ui, vp_rect, cam, theme) {
                    floating_consumed = true;
                }
            }
        }
    }

    if render_visual_style_selector(
        painter,
        ui,
        vp_rect,
        pane_idx,
        &mut canvas_state.panes[pane_idx].visual_style,
        &mut canvas_state.panes[pane_idx].style_menu_open,
        theme,
    ) {
        floating_consumed = true;
    }

    if render_viewport_header_badge(
        painter,
        ui,
        vp_rect,
        pane_idx,
        &active_view,
        is_active,
        total_panes,
        doc,
        canvas_state,
        theme,
    ) {
        floating_consumed = true;
    }

    // Workplane Mini Pie / Pill Tool HUD (in 3D when creating 2D profiles or primitives)
    if is_active && active_view.kind.is_3d() {
        let is_drawing_surf = matches!(
            active_tool,
            Tool::DrawRectSurface { .. }
                | Tool::DrawCircleSurface { .. }
                | Tool::DrawPolygonSurface { .. }
                | Tool::DrawBoxPrimitive { .. }
                | Tool::DrawCylinderPrimitive { .. }
                | Tool::DrawSpherePrimitive { .. }
        );
        if is_drawing_surf {
            if render_workplane_mini_pie_hud(painter, ui, vp_rect, canvas_state, theme) {
                floating_consumed = true;
            }
        }

        // Workflow Surface Action Popup (Extrude vs Hole)
        if render_surface_action_popup(
            painter,
            ui,
            vp_rect,
            doc,
            selected_element,
            canvas_state,
            theme,
        ) {
            floating_consumed = true;
        }
        // Surface Workplane Picking Hover Highlight
        if canvas_state.picking_surface_workplane && is_mouse_in_vp {
            if let Some(sp) = pointer_pos {
                let (ray_origin, ray_dir) = unproject_ray(sp);
                if let Some((id, face, p_hit)) = raycast_closest_face(doc, ray_origin, ray_dir) {
                    let poly_screen: Vec<Pos2> = face.pts.iter().map(|&p| project_3d(p)).collect();
                    if poly_screen.len() >= 3 && !poly_screen.iter().any(|p| p.x.abs() > 50000.0 || p.y.abs() > 50000.0) {
                        draw_polygon_safe(
                            &vp_painter,
                            &poly_screen,
                            Color32::from_rgba_unmultiplied(255, 140, 0, 45),
                            Stroke::NONE,
                        );
                        for i in 0..poly_screen.len() {
                            vp_painter.line_segment(
                                [poly_screen[i], poly_screen[(i + 1) % poly_screen.len()]],
                                Stroke::new(2.5, Color32::from_rgb(255, 140, 0)),
                            );
                        }
                    }
                    let hit_sp = project_3d(p_hit);
                    vp_painter.circle_filled(hit_sp, 4.0, Color32::from_rgb(255, 140, 0));
                    let elem_kind = match doc.elements.get(&id) {
                        Some(Element::Wall(_)) => "Wall",
                        Some(Element::Slab(_)) => "Slab",
                        Some(Element::Roof(_)) => "Roof",
                        Some(Element::Column(_)) => "Column",
                        Some(Element::Beam(_)) => "Beam",
                        Some(Element::FreeformSolid(_)) => "3D Solid",
                        _ => "Surface",
                    };
                    vp_painter.text(
                        hit_sp + Vec2::new(14.0, -14.0),
                        egui::Align2::LEFT_CENTER,
                        format!("🎯 Click to host workplane on {elem_kind} ({id})"),
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(255, 140, 0),
                    );
                }
            }
        }
    }

    if total_panes > 1 {
        let border_stroke = if is_active {
            Stroke::new(2.0, theme.accent)
        } else {
            Stroke::new(1.0, theme.border_color)
        };
        painter.rect_stroke(vp_rect, 0.0, border_stroke, egui::StrokeKind::Inside);
    }

    // Handle Window / Crossing Drag Selection for Tool::Select across all views (Plan, Elevation, Section, 3D)
    let is_primary_down = ui.input(|i| i.pointer.primary_down());
    let is_primary_released = ui.input(|i| i.pointer.primary_released());
    let press_origin = ui.input(|i| i.pointer.press_origin());
    let mut box_selected_this_frame = false;

    // CAD Grip Handle Detection & Dragging for selected annotations
    let is_primary_pressed = ui.input(|i| i.pointer.primary_pressed());
    if is_active && *active_tool == Tool::Select && !floating_consumed {
        if is_primary_pressed {
            if let Some(sel_id) = *selected_element {
                if let Some(el) = doc.elements.get(&sel_id) {
                    let mut hit_grip = None;
                    match el {
                        Element::Dimension(d) => {
                            let diff = d.p2 - d.p1;
                            let mid_dim = if diff.length() > 1e-3 {
                                let dir = diff.normalized();
                                let norm = plinth_geom::vec2(-dir.y, dir.x);
                                (d.p1 + d.p2) * 0.5 + norm * d.offset
                            } else {
                                (d.p1 + d.p2) * 0.5
                            };
                            let grips = [to_screen(d.p1), to_screen(d.p2), to_screen(mid_dim)];
                            for (idx, &gp) in grips.iter().enumerate() {
                                if gp.distance(mouse_screen) <= 12.0 {
                                    hit_grip = Some(idx);
                                    break;
                                }
                            }
                        }
                        Element::TextNote(t) => {
                            let gp = to_screen(t.pos);
                            if gp.distance(mouse_screen) <= 12.0 {
                                hit_grip = Some(0);
                            }
                        }
                        Element::Leader(l) => {
                            let grips = [to_screen(l.target_pt), to_screen(l.shoulder_pt), to_screen(l.text_pt)];
                            for (idx, &gp) in grips.iter().enumerate() {
                                if gp.distance(mouse_screen) <= 12.0 {
                                    hit_grip = Some(idx);
                                    break;
                                }
                            }
                        }
                        Element::DetailLine(l) => {
                            let mid = (l.start + l.end) * 0.5;
                            let grips = [to_screen(l.start), to_screen(l.end), to_screen(mid)];
                            for (idx, &gp) in grips.iter().enumerate() {
                                if gp.distance(mouse_screen) <= 12.0 {
                                    hit_grip = Some(idx);
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                    if let Some(g_idx) = hit_grip {
                        doc.push_undo();
                        canvas_state.active_annotation_grip = Some((sel_id, g_idx));
                    }
                }
            }
        }

        // Active Grip Dragging Update
        if let Some((grip_elem_id, grip_idx)) = canvas_state.active_annotation_grip {
            box_selected_this_frame = true;
            if is_primary_down {
                if let Some(el) = doc.elements.get_mut(&grip_elem_id) {
                    match el {
                        Element::Dimension(d) => {
                            match grip_idx {
                                0 => d.p1 = mouse_pos,
                                1 => d.p2 = mouse_pos,
                                2 => {
                                    let diff = d.p2 - d.p1;
                                    if diff.length() > 1e-3 {
                                        let dir = diff.normalized();
                                        let norm = plinth_geom::vec2(-dir.y, dir.x);
                                        d.offset = (mouse_pos - (d.p1 + d.p2) * 0.5).dot(norm);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Element::TextNote(t) => {
                            if grip_idx == 0 {
                                t.pos = mouse_pos;
                            }
                        }
                        Element::Leader(l) => {
                            match grip_idx {
                                0 => l.target_pt = mouse_pos,
                                1 => l.shoulder_pt = mouse_pos,
                                2 => l.text_pt = mouse_pos,
                                _ => {}
                            }
                        }
                        Element::DetailLine(l) => {
                            match grip_idx {
                                0 => l.start = mouse_pos,
                                1 => l.end = mouse_pos,
                                2 => {
                                    let mid = (l.start + l.end) * 0.5;
                                    let delta = mouse_pos - mid;
                                    l.start += delta;
                                    l.end += delta;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            if is_primary_released {
                canvas_state.active_annotation_grip = None;
            }
        }
    }

    if is_active && *active_tool == Tool::Select && !floating_consumed && !canvas_state.picking_surface_workplane {
        if is_primary_down && canvas_state.active_annotation_grip.is_none() {
            if let Some(origin) = press_origin {
                if vp_rect.contains(origin) && origin.distance(mouse_screen) >= 4.0 {
                    canvas_state.selection_box = Some(SelectionBox {
                        start: origin,
                        current: mouse_screen,
                    });
                }
            }
        }

        if is_primary_released {
            if let Some(sel_box) = canvas_state.selection_box.take() {
                if sel_box.start.distance(sel_box.current) >= 4.0 {
                    box_selected_this_frame = true;
                    let is_crossing = sel_box.is_crossing();
                    let sel_rect = sel_box.rect();
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

                    let mut matched_ids = Vec::new();
                    for (&id, el) in &doc.elements {
                        if let Some(geom) = get_element_projected_screen_pts(
                            id,
                            el,
                            &active_view,
                            doc,
                            &to_screen,
                            &project_3d,
                            cut_z,
                        ) {
                            if test_element_marquee(&geom, sel_rect, is_crossing) {
                                matched_ids.push(id);
                            }
                        }
                    }

                    if is_shift || is_ctrl {
                        for id in matched_ids {
                            if !canvas_state.selected_elements.contains(&id) {
                                canvas_state.selected_elements.push(id);
                            }
                        }
                    } else {
                        canvas_state.selected_elements = matched_ids;
                    }
                    *selected_element = canvas_state.selected_elements.first().copied();
                }
            }
        }
    }

    // 5. Click & Tool Execution Handling (only in active pane when clicked)
    if is_active && !floating_consumed && primary_clicked && is_mouse_in_vp && !box_selected_this_frame {
        // Handle picking a 3D face to set surface workplane
        if canvas_state.picking_surface_workplane {
            if let Some(click_pos) = pointer_pos {
                let (ray_origin, ray_dir) = unproject_ray(click_pos);
                if let Some((id, face, p_hit)) = raycast_closest_face(doc, ray_origin, ray_dir) {
                    let origin = p_hit;
                    let normal = face.normal;
                    let cam_right = plinth_geom::Vec3::new(cos_y, -sin_y, 0.0);
                    let cam_up = plinth_geom::Vec3::new(sin_p * sin_y, sin_p * cos_y, cos_p);
                    let u_raw = cam_right - normal * cam_right.dot(normal);
                    let u_axis = if u_raw.dot(u_raw) > 1e-6 {
                        u_raw.normalized()
                    } else {
                        let v_raw = cam_up - normal * cam_up.dot(normal);
                        if v_raw.dot(v_raw) > 1e-6 {
                            normal.cross(v_raw.normalized()).normalized()
                        } else {
                            plinth_geom::Vec3::X
                        }
                    };
                    let mut v_axis = normal.cross(u_axis).normalized();
                    if v_axis.dot(cam_up) < 0.0 {
                        v_axis = -v_axis;
                    }
                    canvas_state.workplane = WorkplaneMode::Surface {
                        origin,
                        normal,
                        u_axis,
                        v_axis,
                        host_element: Some(id),
                    };
                    canvas_state.picking_surface_workplane = false;
                }
            }
            return;
        }

        if *active_tool != Tool::Select {
            doc.push_undo();
        }
        let active_level = doc.active_level_id;

        match active_tool {
            Tool::Select => {
                if active_view.kind.is_3d() {
                    // True 3D Ray-Polygon Intersection Picking Engine
                    let cam = active_view.camera_3d.unwrap_or_default();
                    let target = plinth_geom::Vec3::new(cam.target_x, cam.target_y, cam.target_z);
                    let (sin_y, cos_y) = cam.yaw.sin_cos();
                    let (sin_p, cos_p) = cam.pitch.sin_cos();
                    let cam_forward = plinth_geom::Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);
                    let is_persp = cam.fov_degrees > 1.0;
                    let cam_dist = cam.distance.max(1000.0);

                    let click_screen = pointer_pos.unwrap_or(mouse_screen);
                    let click_v = plinth_geom::vec2(click_screen.x as f64, click_screen.y as f64);
                    let v_x = ((click_screen.x - center.x - pan.x) / zoom) as f64;
                    let v_z = -((click_screen.y - center.y - pan.y) / zoom) as f64;

                    // Unrotate screen ray to 3D world space:
                    let (ray_origin, ray_dir) = if !is_persp {
                        let x1 = v_x;
                        let y1 = v_z * sin_p;
                        let z1 = v_z * cos_p;
                        let dx = x1 * cos_y + y1 * sin_y;
                        let dy = -x1 * sin_y + y1 * cos_y;
                        let pt_on_ray = target + plinth_geom::Vec3::new(dx, dy, z1);
                        (pt_on_ray - cam_forward * cam_dist, cam_forward)
                    } else {
                        let origin = target - cam_forward * cam_dist;
                        let dir_view = plinth_geom::Vec3::new(v_x, cam_dist, v_z).normalized();
                        let x1 = dir_view.x;
                        let y1 = dir_view.y * cos_p + dir_view.z * sin_p;
                        let z1 = -dir_view.y * sin_p + dir_view.z * cos_p;
                        let dx = x1 * cos_y + y1 * sin_y;
                        let dy = -x1 * sin_y + y1 * cos_y;
                        let dir = plinth_geom::Vec3::new(dx, dy, z1).normalized();
                        (origin, dir)
                    };

                    let mut best_id = None;
                    let mut min_t = f64::INFINITY;

                    // Pass 1: Direct 3D Surface Hits (highest priority, no edge bleeding)
                    for (&id, el) in &doc.elements {
                        for face in doc.element_faces(id) {
                            let denom = face.normal.dot(ray_dir);
                            if denom >= -1e-5 {
                                continue; // Culled: facing away from ray
                            }
                            if face.pts.len() < 3 {
                                continue;
                            }
                            let t = face.normal.dot(face.pts[0] - ray_origin) / denom;
                            if t <= 0.0 {
                                continue;
                            }
                            let bias = match el {
                                Element::Door(_) | Element::Window(_) => -30.0,
                                _ => 0.0,
                            };
                            let eff_t = t + bias;
                            if eff_t >= min_t {
                                continue;
                            }

                            let p_hit = ray_origin + ray_dir * t;
                            let nx = face.normal.x.abs();
                            let ny = face.normal.y.abs();
                            let nz = face.normal.z.abs();

                            let (p_2d, poly_2d) = if nz >= nx && nz >= ny {
                                (
                                    plinth_geom::vec2(p_hit.x, p_hit.y),
                                    face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.y)).collect::<Vec<_>>(),
                                )
                            } else if nx >= ny {
                                (
                                    plinth_geom::vec2(p_hit.y, p_hit.z),
                                    face.pts.iter().map(|p| plinth_geom::vec2(p.y, p.z)).collect::<Vec<_>>(),
                                )
                            } else {
                                (
                                    plinth_geom::vec2(p_hit.x, p_hit.z),
                                    face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.z)).collect::<Vec<_>>(),
                                )
                            };

                            let is_hit = plinth_geom::polygon::point_in_polygon(p_2d, &poly_2d)
                                || plinth_geom::polygon::distance_to_polyline(p_2d, &poly_2d, true) <= 25.0;

                            if is_hit {
                                min_t = eff_t;
                                best_id = Some(id);
                            }
                        }
                    }

                    // Pass 2: Near-Edge Fallback (only if no direct surface hit, e.g. thin frames, mullions, wires)
                    if best_id.is_none() {
                        let mut min_edge_score = f64::INFINITY;
                        for (&id, _el) in &doc.elements {
                            for face in doc.element_faces(id) {
                                if face.normal.dot(ray_dir) >= -1e-5 {
                                    continue;
                                }
                                let denom = face.normal.dot(ray_dir);
                                let face_t = if denom.abs() > 1e-6 {
                                    face.normal.dot(face.pts[0] - ray_origin) / denom
                                } else {
                                    (face.pts[0] - ray_origin).length()
                                };
                                if face_t <= 0.0 {
                                    continue;
                                }
                                let screen_pts: Vec<plinth_geom::Vec2> = face
                                    .pts
                                    .iter()
                                    .map(|&p| {
                                        let sp = project_3d(p);
                                        plinth_geom::vec2(sp.x as f64, sp.y as f64)
                                    })
                                    .collect();
                                let n_pts = screen_pts.len();
                                if n_pts >= 3 {
                                    for i in 0..n_pts {
                                        let p0 = screen_pts[i];
                                        let p1 = screen_pts[(i + 1) % n_pts];
                                        let (closest_pt, _) = plinth_geom::polygon::closest_point_on_segment(click_v, p0, p1);
                                        let d = click_v.distance(closest_pt);
                                        if d <= 8.0 {
                                            let edge_score = face_t + d * 50.0;
                                            if edge_score < min_edge_score {
                                                min_edge_score = edge_score;
                                                best_id = Some(id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let is_shift = ui.input(|i| i.modifiers.shift);
                    let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    if is_shift || is_ctrl {
                        if let Some(id) = best_id {
                            if canvas_state.selected_elements.contains(&id) {
                                canvas_state.selected_elements.retain(|&x| x != id);
                                *selected_element = canvas_state.selected_elements.first().copied();
                            } else {
                                canvas_state.selected_elements.push(id);
                                *selected_element = Some(id);
                            }
                        }
                    } else {
                        *selected_element = best_id;
                        canvas_state.selected_elements = best_id.map(|id| vec![id]).unwrap_or_default();
                    }
                } else if active_view.kind.is_elevation() {
                    // Elevation element selection: mouse_pos is (u, z)
                    let project_elev = |p: plinth_geom::Vec3| -> plinth_geom::Vec2 {
                        match active_view.kind {
                            ViewKind::ElevationSouth => plinth_geom::vec2(p.x, p.z),
                            ViewKind::ElevationNorth => plinth_geom::vec2(-p.x, p.z),
                            ViewKind::ElevationEast => plinth_geom::vec2(-p.y, p.z),
                            ViewKind::ElevationWest => plinth_geom::vec2(p.y, p.z),
                            _ => plinth_geom::vec2(p.x, p.z),
                        }
                    };
                    let dist_to_box = |p: plinth_geom::Vec2, u0: f64, u1: f64, z0: f64, z1: f64| -> f64 {
                        let dx = if p.x < u0 { u0 - p.x } else if p.x > u1 { p.x - u1 } else { 0.0 };
                        let dz = if p.y < z0 { z0 - p.y } else if p.y > z1 { p.y - z1 } else { 0.0 };
                        dx.hypot(dz)
                    };
                    let mut best_id = None;
                    let mut best_score = 400.0;
                    for (id, el) in &doc.elements {
                        let (d, priority) = match el {
                            Element::Column(c) => {
                                let (z0, z1) = c.z_bounds(
                                    doc.level_elevation(c.base_level),
                                    c.top_level.map(|l| doc.level_elevation(l)),
                                );
                                let us: Vec<f64> = c.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                                if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                                    (dist_to_box(mouse_pos, u_min, u_max, z0, z1), 1.0)
                                } else {
                                    (f64::INFINITY, 1.0)
                                }
                            }
                            Element::Footing(f) => {
                                let (z0, z1) = f.z_bounds(
                                    doc.level_elevation(f.base_level),
                                    f.top_level.map(|l| doc.level_elevation(l)),
                                );
                                let us: Vec<f64> = f.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                                if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                                    (dist_to_box(mouse_pos, u_min, u_max, z0, z1), 2.0)
                                } else {
                                    (f64::INFINITY, 2.0)
                                }
                            }
                            Element::Beam(b) => {
                                let (z0, z1) = b.z_bounds(
                                    doc.level_elevation(b.level),
                                    b.top_level.map(|l| doc.level_elevation(l)),
                                );
                                let u0 = project_elev(b.start.extend(0.0)).x;
                                let u1 = project_elev(b.end.extend(0.0)).x;
                                (dist_to_box(mouse_pos, u0.min(u1), u0.max(u1), z0, z1), 3.0)
                            }
                            Element::Wall(w) => {
                                let (z0, z1) = w.z_bounds(
                                    doc.level_elevation(w.base_level),
                                    w.top_level.map(|l| doc.level_elevation(l)),
                                );
                                let u0 = project_elev(w.start.extend(0.0)).x;
                                let u1 = project_elev(w.end.extend(0.0)).x;
                                (dist_to_box(mouse_pos, u0.min(u1), u0.max(u1), z0, z1), 4.0)
                            }
                            Element::Slab(s) => {
                                let (z_bot, z_top) = s.z_bounds(
                                    doc.level_elevation(s.level),
                                    s.top_level.map(|l| doc.level_elevation(l)),
                                );
                                let us: Vec<f64> = s.boundary.iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                                if let (Some(&u_min), Some(&u_max)) = (us.iter().min_by(|a, b| a.total_cmp(b)), us.iter().max_by(|a, b| a.total_cmp(b))) {
                                    (dist_to_box(mouse_pos, u_min, u_max, z_bot, z_top), 5.0)
                                } else {
                                    (f64::INFINITY, 5.0)
                                }
                            }
                            _ => (f64::INFINITY, 10.0),
                        };
                        let score = if d <= 15.0 { priority } else { d + priority * 10.0 };
                        if score < best_score {
                            best_score = score;
                            best_id = Some(*id);
                        }
                    }
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    if is_shift || is_ctrl {
                        if let Some(id) = best_id {
                            if canvas_state.selected_elements.contains(&id) {
                                canvas_state.selected_elements.retain(|&x| x != id);
                                *selected_element = canvas_state.selected_elements.first().copied();
                            } else {
                                canvas_state.selected_elements.push(id);
                                *selected_element = Some(id);
                            }
                        }
                    } else {
                        *selected_element = best_id;
                        canvas_state.selected_elements = best_id.map(|id| vec![id]).unwrap_or_default();
                    }
                } else if active_view.kind.is_section() {
                    // Section element selection: mouse_pos is (t, z)
                    if let Some((s1, s2)) = active_view.section_line {
                        let line_vec = s2 - s1;
                        let u_dir = if line_vec.length() > 1.0 { line_vec.normalized() } else { plinth_geom::vec2(1.0, 0.0) };
                        let dist_to_box = |p: plinth_geom::Vec2, t0: f64, t1: f64, z0: f64, z1: f64| -> f64 {
                            let dt = if p.x < t0 { t0 - p.x } else if p.x > t1 { p.x - t1 } else { 0.0 };
                            let dz = if p.y < z0 { z0 - p.y } else if p.y > z1 { p.y - z1 } else { 0.0 };
                            dt.hypot(dz)
                        };
                        let mut best_id = None;
                        let mut best_score = 400.0;
                        for (id, el) in &doc.elements {
                            let (d, priority) = match el {
                                Element::Column(c) => {
                                    let (z0, z1) = c.z_bounds(
                                        doc.level_elevation(c.base_level),
                                        c.top_level.map(|l| doc.level_elevation(l)),
                                    );
                                    let ts: Vec<f64> = c.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                                    if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                        (dist_to_box(mouse_pos, t_min, t_max, z0, z1), 1.0)
                                    } else {
                                        (f64::INFINITY, 1.0)
                                    }
                                }
                                Element::Footing(f) => {
                                    let (z0, z1) = f.z_bounds(
                                        doc.level_elevation(f.base_level),
                                        f.top_level.map(|l| doc.level_elevation(l)),
                                    );
                                    let ts: Vec<f64> = f.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                                    if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                        (dist_to_box(mouse_pos, t_min, t_max, z0, z1), 2.0)
                                    } else {
                                        (f64::INFINITY, 2.0)
                                    }
                                }
                                Element::Beam(b) => {
                                    let (z0, z1) = b.z_bounds(
                                        doc.level_elevation(b.level),
                                        b.top_level.map(|l| doc.level_elevation(l)),
                                    );
                                    let t0 = (b.start - s1).dot(u_dir);
                                    let t1 = (b.end - s1).dot(u_dir);
                                    (dist_to_box(mouse_pos, t0.min(t1), t0.max(t1), z0, z1), 3.0)
                                }
                                Element::Wall(w) => {
                                    let (z0, z1) = w.z_bounds(
                                        doc.level_elevation(w.base_level),
                                        w.top_level.map(|l| doc.level_elevation(l)),
                                    );
                                    let t0 = (w.start - s1).dot(u_dir);
                                    let t1 = (w.end - s1).dot(u_dir);
                                    (dist_to_box(mouse_pos, t0.min(t1), t0.max(t1), z0, z1), 4.0)
                                }
                                Element::Slab(s) => {
                                    let (z_bot, z_top) = s.z_bounds(
                                        doc.level_elevation(s.level),
                                        s.top_level.map(|l| doc.level_elevation(l)),
                                    );
                                    let ts: Vec<f64> = s.boundary.iter().map(|&p| (p - s1).dot(u_dir)).collect();
                                    if let (Some(&t_min), Some(&t_max)) = (ts.iter().min_by(|a, b| a.total_cmp(b)), ts.iter().max_by(|a, b| a.total_cmp(b))) {
                                        (dist_to_box(mouse_pos, t_min, t_max, z_bot, z_top), 5.0)
                                    } else {
                                        (f64::INFINITY, 5.0)
                                    }
                                }
                                _ => (f64::INFINITY, 10.0),
                            };
                            let score = if d <= 15.0 { priority } else { d + priority * 10.0 };
                            if score < best_score {
                                best_score = score;
                                best_id = Some(*id);
                            }
                        }
                        let is_shift = ui.input(|i| i.modifiers.shift);
                    let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    if is_shift || is_ctrl {
                        if let Some(id) = best_id {
                            if canvas_state.selected_elements.contains(&id) {
                                canvas_state.selected_elements.retain(|&x| x != id);
                                *selected_element = canvas_state.selected_elements.first().copied();
                            } else {
                                canvas_state.selected_elements.push(id);
                                *selected_element = Some(id);
                            }
                        }
                    } else {
                        *selected_element = best_id;
                        canvas_state.selected_elements = best_id.map(|id| vec![id]).unwrap_or_default();
                    }
                    }
                } else {
                    // 2D Floor Plan nearest element selection
                    let (view_level_z, cut_z) = if let Some(lid) = active_view.level_id {
                        let lz = doc.level_elevation(lid);
                        (lz, lz + active_view.cut_offset)
                    } else {
                        (0.0, 1200.0)
                    };

                    let mut best_id = None;
                    let mut best_dist = 280.0; // threshold in mm
                    for (id, el) in &doc.elements {
                        let d = match el {
                            Element::Wall(w) => {
                                let (base_z, top_z) = w.z_bounds(
                                    doc.level_elevation(w.base_level),
                                    w.top_level.map(|tl| doc.level_elevation(tl)),
                                );
                                if cut_z >= base_z && cut_z <= top_z + 100.0 {
                                    let center_dist = plinth_geom::polygon::distance_to_segment(mouse_pos, w.start, w.end);
                                    let half_thick = w.thickness * 0.5;
                                    if center_dist <= half_thick + 40.0 {
                                        (center_dist - half_thick).max(0.0)
                                    } else {
                                        center_dist
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::CurtainWall(cw) => {
                                let (base_z, top_z) = cw.z_bounds(
                                    doc.level_elevation(cw.base_level),
                                    cw.top_level.map(|tl| doc.level_elevation(tl)),
                                );
                                if cut_z >= base_z && cut_z <= top_z + 100.0 {
                                    let center_dist = plinth_geom::polygon::distance_to_segment(mouse_pos, cw.start, cw.end);
                                    let half_thick = cw.mullion_depth * 0.5;
                                    if center_dist <= half_thick + 40.0 {
                                        (center_dist - half_thick).max(0.0)
                                    } else {
                                        center_dist
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Door(door) => {
                                if let Some(Element::Wall(hw)) = doc.elements.get(&door.host_wall) {
                                    let wall_z0 = hw.z_bounds(
                                        doc.level_elevation(hw.base_level),
                                        hw.top_level.map(|tl| doc.level_elevation(tl)),
                                    ).0;
                                    let d_base_z = door.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                                    let (d_z0, d_z1) = door.z_bounds(d_base_z, door.top_level.map(|l| doc.level_elevation(l)));
                                    if cut_z >= d_z0 && cut_z <= d_z1 {
                                        let dir = hw.direction();
                                        let hinge = hw.start + dir * (door.offset_along_wall - door.width * 0.5);
                                        let center = hw.start + dir * door.offset_along_wall;
                                        let strike = hw.start + dir * (door.offset_along_wall + door.width * 0.5);
                                        let d_center = center.distance(mouse_pos);
                                        let d_hinge = hinge.distance(mouse_pos);
                                        let d_strike = strike.distance(mouse_pos);
                                        if d_center <= door.width * 0.6 || d_hinge <= door.width * 0.95 || d_strike <= 60.0 {
                                            0.0
                                        } else {
                                            (d_center - door.width * 0.5).max(0.0)
                                        }
                                    } else {
                                        f64::INFINITY
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Window(win) => {
                                if let Some(Element::Wall(hw)) = doc.elements.get(&win.host_wall) {
                                    let wall_z0 = hw.z_bounds(
                                        doc.level_elevation(hw.base_level),
                                        hw.top_level.map(|tl| doc.level_elevation(tl)),
                                    ).0;
                                    let w_base_z = win.base_level.map(|l| doc.level_elevation(l)).unwrap_or(wall_z0);
                                    let (win_bottom, win_top) = win.z_bounds(w_base_z, win.top_level.map(|l| doc.level_elevation(l)));
                                    if cut_z >= win_bottom && cut_z <= win_top {
                                        let center = hw.start + hw.direction() * win.offset_along_wall;
                                        let dist = center.distance(mouse_pos);
                                        if dist <= win.width * 0.6 {
                                            0.0
                                        } else {
                                            (dist - win.width * 0.5).max(0.0)
                                        }
                                    } else {
                                        f64::INFINITY
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Column(c) => {
                                let (c_base_z, c_top_z) = c.z_bounds(
                                    doc.level_elevation(c.base_level),
                                    c.top_level.map(|tl| doc.level_elevation(tl)),
                                );
                                if cut_z >= c_base_z && cut_z <= c_top_z + 100.0 {
                                    let dist = c.location.distance(mouse_pos);
                                    let radius = c.width.max(c.depth) * 0.5;
                                    if dist <= radius + 40.0 {
                                        0.0
                                    } else {
                                        dist - radius
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Footing(f) => {
                                let (_f_bot, f_top) = f.z_bounds(
                                    doc.level_elevation(f.base_level),
                                    f.top_level.map(|l| doc.level_elevation(l)),
                                );
                                if f_top <= cut_z && f_top >= view_level_z - 2000.0 {
                                    let fp = f.footprint();
                                    if plinth_geom::polygon::point_in_polygon(mouse_pos, &fp) {
                                        5.0
                                    } else {
                                        plinth_geom::polygon::distance_to_polyline(mouse_pos, &fp, true)
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Beam(b) => {
                                let (b_base_z, b_top_z) = b.z_bounds(
                                    doc.level_elevation(b.level),
                                    b.top_level.map(|tl| doc.level_elevation(tl)),
                                );
                                if b_top_z >= view_level_z - 100.0 && b_base_z <= cut_z + 2000.0 {
                                    let d = plinth_geom::polygon::distance_to_segment(mouse_pos, b.start, b.end);
                                    let half_w = b.width * 0.5;
                                    if d <= half_w + 30.0 {
                                        (d - half_w).max(0.0)
                                    } else {
                                        d
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Stair(s) => {
                                let (s_bot, s_top) = s.z_bounds(
                                    doc.level_elevation(s.base_level),
                                    s.top_level.map(|l| doc.level_elevation(l)),
                                );
                                if s_top >= view_level_z - 100.0 && s_bot <= cut_z {
                                    let fp = s.footprint();
                                    if plinth_geom::polygon::point_in_polygon(mouse_pos, &fp) {
                                        0.0
                                    } else {
                                        plinth_geom::polygon::distance_to_polyline(mouse_pos, &fp, true)
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Ramp(r) => {
                                let (r_bot, r_top) = r.z_bounds(
                                    doc.level_elevation(r.base_level),
                                    r.top_level.map(|l| doc.level_elevation(l)),
                                );
                                if r_top >= view_level_z - 100.0 && r_bot <= cut_z {
                                    let fp = r.footprint();
                                    if plinth_geom::polygon::point_in_polygon(mouse_pos, &fp) {
                                        0.0
                                    } else {
                                        plinth_geom::polygon::distance_to_polyline(mouse_pos, &fp, true)
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Railing(rl) => plinth_geom::polygon::distance_to_polyline(mouse_pos, &rl.path, false),
                            Element::Fascia(f) => {
                                let f_z = doc.level_elevation(f.level);
                                if (f_z - cut_z).abs() <= 1500.0 {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &f.path, false)
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Gutter(g) => {
                                let g_z = doc.level_elevation(g.level);
                                if (g_z - cut_z).abs() <= 1500.0 {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &g.path, false)
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Ceiling(c) => {
                                if active_view.kind == ViewKind::CeilingPlan {
                                    let edge_d = plinth_geom::polygon::distance_to_polyline(mouse_pos, &c.boundary, true);
                                    if edge_d <= 80.0 {
                                        edge_d
                                    } else if plinth_geom::polygon::point_in_polygon(mouse_pos, &c.boundary) {
                                        250.0
                                    } else {
                                        f64::INFINITY
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Roof(r) => {
                                let r_lvl_z = doc.level_elevation(r.level);
                                let r_base_z = r_lvl_z + r.offset;
                                if r_base_z > cut_z + 200.0 {
                                    f64::INFINITY
                                } else {
                                    let edge_d = plinth_geom::polygon::distance_to_polyline(mouse_pos, &r.boundary, true);
                                    if edge_d <= 80.0 {
                                        edge_d
                                    } else if plinth_geom::polygon::point_in_polygon(mouse_pos, &r.boundary) {
                                        260.0
                                    } else {
                                        f64::INFINITY
                                    }
                                }
                            }
                            Element::Slab(s) => {
                                let (_slab_bot, slab_top) = s.z_bounds(
                                    doc.level_elevation(s.level),
                                    s.top_level.map(|l| doc.level_elevation(l)),
                                );
                                // In plan view, only slabs that are at or below the cut plane are visible.
                                // Slabs above the cut plane (e.g. upper storey floor slabs, roof decks) MUST NEVER BE SELECTABLE!
                                if slab_top > cut_z || slab_top < view_level_z - 1500.0 {
                                    f64::INFINITY
                                } else {
                                    let edge_d = plinth_geom::polygon::distance_to_polyline(mouse_pos, &s.boundary, true);
                                    if edge_d <= 80.0 {
                                        edge_d
                                    } else if plinth_geom::polygon::point_in_polygon(mouse_pos, &s.boundary) {
                                        // Background priority penalty (250 mm) so that walls, doors, windows, columns, etc.
                                        // are always prioritized over the floor slab.
                                        250.0
                                    } else {
                                        f64::INFINITY
                                    }
                                }
                            }
                            Element::FreeformSurface(s) => {
                                let s_z = s.elevation;
                                if (s_z - view_level_z).abs() <= 3500.0 || (s_z <= cut_z && s_z >= view_level_z - 500.0) {
                                    if plinth_geom::polygon::point_in_polygon(mouse_pos, &s.boundary) {
                                        5.0
                                    } else {
                                        plinth_geom::polygon::distance_to_polyline(mouse_pos, &s.boundary, true)
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::FreeformSolid(f) => {
                                if (cut_z >= f.z_min && cut_z <= f.z_max) || (f.z_max <= cut_z && f.z_max >= view_level_z - 1000.0) {
                                    if plinth_geom::polygon::point_in_polygon(mouse_pos, &f.boundary) {
                                        5.0
                                    } else {
                                        plinth_geom::polygon::distance_to_polyline(mouse_pos, &f.boundary, true)
                                    }
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Dimension(dim) => {
                                match dim.kind {
                                    plinth_model::DimensionKind::Linear => {
                                        let diff = dim.p2 - dim.p1;
                                        let len = diff.length();
                                        if len > 1e-3 {
                                            let dir = diff.normalized();
                                            let norm = plinth_geom::vec2(-dir.y, dir.x);
                                            let p1d = dim.p1 + norm * dim.offset;
                                            let p2d = dim.p2 + norm * dim.offset;
                                            let d_line = plinth_geom::polygon::distance_to_segment(mouse_pos, p1d, p2d);
                                            let d_w1 = plinth_geom::polygon::distance_to_segment(mouse_pos, dim.p1, p1d);
                                            let d_w2 = plinth_geom::polygon::distance_to_segment(mouse_pos, dim.p2, p2d);
                                            d_line.min(d_w1).min(d_w2)
                                        } else {
                                            f64::INFINITY
                                        }
                                    }
                                    plinth_model::DimensionKind::Circular => {
                                        let d_c = mouse_pos.distance(dim.p1);
                                        let r = dim.p1.distance(dim.p2);
                                        let d_ring = (d_c - r).abs();
                                        let d_line = plinth_geom::polygon::distance_to_segment(mouse_pos, dim.p1, dim.p2);
                                        d_ring.min(d_line)
                                    }
                                    plinth_model::DimensionKind::Angular => {
                                        plinth_geom::polygon::distance_to_segment(mouse_pos, dim.p1, dim.p2)
                                    }
                                }
                            }
                            Element::DetailLine(dl) => {
                                plinth_geom::polygon::distance_to_segment(mouse_pos, dl.start, dl.end)
                            }
                            Element::TextNote(t) => {
                                let d = mouse_pos.distance(t.pos);
                                if d <= 300.0 {
                                    d * 0.1
                                } else {
                                    f64::INFINITY
                                }
                            }
                            Element::Leader(l) => {
                                let d1 = plinth_geom::polygon::distance_to_segment(mouse_pos, l.target_pt, l.shoulder_pt);
                                let d2 = plinth_geom::polygon::distance_to_segment(mouse_pos, l.shoulder_pt, l.text_pt);
                                d1.min(d2)
                            }
                            Element::BuildingPad(bp) => {
                                if plinth_geom::polygon::point_in_polygon(mouse_pos, &bp.boundary) {
                                    10.0
                                } else {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &bp.boundary, true)
                                }
                            }
                            Element::Toposolid(ts) => {
                                if plinth_geom::polygon::point_in_polygon(mouse_pos, &ts.boundary) {
                                    15.0
                                } else {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &ts.boundary, true)
                                }
                            }
                            Element::PropertyLine(pl) => {
                                plinth_geom::polygon::distance_to_polyline(mouse_pos, &pl.boundary, true)
                            }
                            Element::Road(rd) => {
                                let poly = rd.footprint();
                                if plinth_geom::polygon::point_in_polygon(mouse_pos, &poly) {
                                    12.0
                                } else {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &poly, true)
                                }
                            }
                            Element::Sidewalk(sw) => {
                                let poly = sw.footprint();
                                if plinth_geom::polygon::point_in_polygon(mouse_pos, &poly) {
                                    12.0
                                } else {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &poly, true)
                                }
                            }
                            Element::Pathwalk(pw) => {
                                let poly = pw.footprint();
                                if plinth_geom::polygon::point_in_polygon(mouse_pos, &poly) {
                                    12.0
                                } else {
                                    plinth_geom::polygon::distance_to_polyline(mouse_pos, &poly, true)
                                }
                            }
                        };
                        if d < best_dist {
                            best_dist = d;
                            best_id = Some(*id);
                        }
                    }
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    if is_shift || is_ctrl {
                        if let Some(id) = best_id {
                            if canvas_state.selected_elements.contains(&id) {
                                canvas_state.selected_elements.retain(|&x| x != id);
                                *selected_element = canvas_state.selected_elements.first().copied();
                            } else {
                                canvas_state.selected_elements.push(id);
                                *selected_element = Some(id);
                            }
                        }
                    } else {
                        *selected_element = best_id;
                        canvas_state.selected_elements = best_id.map(|id| vec![id]).unwrap_or_default();
                    }
                }
            }

            Tool::DrawWall { start, is_structural } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        if p1.distance(mouse_pos) > 10.0 {
                            doc.push_undo();
                            let mut w = Wall::new(*p1, mouse_pos, active_level);
                            tool_studio.configure_new_wall(doc, &mut w, *is_structural);
                            w.is_structural = *is_structural;
                            let id = w.id;
                            doc.elements.insert(id, Element::Wall(w));
                            *selected_element = Some(id);

                            if canvas_state.snap_settings.wall_continuous_chain {
                                *start = Some(mouse_pos);
                            } else {
                                *start = None;
                            }
                        } else {
                            *start = None;
                        }
                    }
                }
            }

            Tool::DrawCurtainWall { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        let p2 = mouse_pos;
                        if p1.distance(p2) > 20.0 {
                            doc.push_undo();
                            // Check if drawn along an existing wall to automatically host and embed
                            let mut detected_host = None;
                            let mut min_avg_dist = 400.0;
                            for (id, el) in &doc.elements {
                                if let Element::Wall(w) = el {
                                    let (proj1, _t1) = plinth_geom::polygon::closest_point_on_segment(*p1, w.start, w.end);
                                    let (proj2, _t2) = plinth_geom::polygon::closest_point_on_segment(p2, w.start, w.end);
                                    let d1 = proj1.distance(*p1);
                                    let d2 = proj2.distance(p2);
                                    let avg_d = (d1 + d2) * 0.5;
                                    if d1 < 400.0 && d2 < 400.0 && proj1.distance(proj2) > 20.0 && avg_d < min_avg_dist {
                                        min_avg_dist = avg_d;
                                        detected_host = Some((*id, proj1, proj2, w.base_level, w.top_level));
                                    }
                                }
                            }

                            let (cw_p1, cw_p2, host_id, base_lvl, top_lvl) = if let Some((h_id, proj1, proj2, bl, tl)) = detected_host {
                                (proj1, proj2, Some(h_id), bl, tl)
                            } else {
                                (*p1, p2, None, active_level, None)
                            };

                            let mut cw = CurtainWall::new(cw_p1, cw_p2, base_lvl);
                            if let Some(tl) = top_lvl {
                                cw.top_level = Some(tl);
                            }
                            cw.host_wall = host_id;
                            cw.automatically_embed = host_id.is_some();
                            tool_studio.configure_new_curtain_wall(doc, &mut cw);
                            if host_id.is_some() {
                                cw.host_wall = host_id;
                                cw.automatically_embed = true;
                            }
                            let id = cw.id;
                            doc.elements.insert(id, Element::CurtainWall(cw));
                            *selected_element = Some(id);

                            if canvas_state.snap_settings.wall_continuous_chain {
                                *start = Some(mouse_pos);
                            } else {
                                *start = None;
                            }
                        } else {
                            *start = None;
                        }
                    }
                }
            }

            Tool::DrawColumn { is_structural } => {
                doc.push_undo();
                let mut col = Column::new(mouse_pos, active_level);
                tool_studio.configure_new_column(doc, &mut col, *is_structural);
                col.is_structural = *is_structural;
                let id = col.id;
                doc.elements.insert(id, Element::Column(col));
                *selected_element = Some(id);
            }

            Tool::DrawFooting => {
                let mut ft = Footing::new(mouse_pos, active_level);
                tool_studio.configure_new_footing(doc, &mut ft);
                let host_col = doc.elements.values().find_map(|el| match el {
                    Element::Column(c) if c.location.distance(mouse_pos) < 350.0 => {
                        Some((c.id, c.location, c.rotation, c.base_level, c.base_offset, c.width, c.depth))
                    }
                    _ => None,
                });
                if let Some((cid, loc, rot, c_lvl, c_off, cw, cd)) = host_col {
                    ft.host_column = Some(cid);
                    ft.location = loc;
                    ft.rotation = rot;
                    ft.base_level = c_lvl;
                    ft.base_offset = c_off;
                    ft.width = ft.width.max(cw + 400.0);
                    ft.length = ft.length.max(cd + 400.0);
                }
                let id = ft.id;
                doc.elements.insert(id, Element::Footing(ft));
                *selected_element = Some(id);
            }

            Tool::DrawBeam { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        if p1.distance(mouse_pos) > 10.0 {
                            doc.push_undo();
                            let mut beam = Beam::new(*p1, mouse_pos, active_level);
                            tool_studio.configure_new_beam(doc, &mut beam);
                            let id = beam.id;
                            doc.elements.insert(id, Element::Beam(beam));
                            *selected_element = Some(id);
                        }
                        *start = None;
                    }
                }
            }

            Tool::DrawSlab { points, is_structural } => {
                if points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0 {
                    // Close polygon
                    if points.len() >= 3 {
                        doc.push_undo();
                        let mut slab = Slab::new(points.clone(), active_level);
                        tool_studio.configure_new_slab(doc, &mut slab, *is_structural);
                        slab.is_structural = *is_structural;
                        let id = slab.id;
                        doc.elements.insert(id, Element::Slab(slab));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawCeiling { points } => {
                if points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0 {
                    if points.len() >= 3 {
                        doc.push_undo();
                        let mut ceiling = Ceiling::new(points.clone(), active_level);
                        tool_studio.configure_new_ceiling(doc, &mut ceiling);
                        let id = ceiling.id;
                        doc.elements.insert(id, Element::Ceiling(ceiling));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawBuildingPad { points } => {
                let should_close = (points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0)
                    || (points.len() >= 3 && ui.input(|i| i.key_pressed(Key::Enter)));
                if should_close {
                    if points.len() >= 3 {
                        doc.push_undo();
                        let pad = BuildingPad::new(points.clone(), active_level);
                        let id = pad.id;
                        doc.elements.insert(id, Element::BuildingPad(pad));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawToposolid { points } => {
                let should_close = (points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0)
                    || (points.len() >= 3 && ui.input(|i| i.key_pressed(Key::Enter)));
                if should_close {
                    if points.len() >= 3 {
                        doc.push_undo();
                        let topo = Toposolid::new(points.clone(), active_level).with_depth(1000.0);
                        let id = topo.id;
                        doc.elements.insert(id, Element::Toposolid(topo));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawPropertyLine { points } => {
                let should_close = (points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0)
                    || (points.len() >= 3 && ui.input(|i| i.key_pressed(Key::Enter)));
                if should_close {
                    if points.len() >= 3 {
                        doc.push_undo();
                        let pl = PropertyLine::from_sketch(points.clone(), active_level);
                        let id = pl.id;
                        doc.elements.insert(id, Element::PropertyLine(pl));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawRoad { points } => {
                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                let double = points.len() >= 2 && mouse_pos.distance(*points.last().unwrap()) < 100.0;
                if enter || double {
                    if points.len() >= 2 {
                        doc.push_undo();
                        let mut rd = Road::new(points.clone(), active_level);
                        tool_studio.configure_new_road(&mut rd);
                        let id = rd.id;
                        doc.elements.insert(id, Element::Road(rd));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawSidewalk { points } => {
                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                let double = points.len() >= 2 && mouse_pos.distance(*points.last().unwrap()) < 100.0;
                if enter || double {
                    if points.len() >= 2 {
                        doc.push_undo();
                        let mut sw = Sidewalk::new(points.clone(), active_level);
                        tool_studio.configure_new_sidewalk(&mut sw);
                        let id = sw.id;
                        doc.elements.insert(id, Element::Sidewalk(sw));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawPathwalk { points } => {
                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                let double = points.len() >= 2 && mouse_pos.distance(*points.last().unwrap()) < 100.0;
                if enter || double {
                    if points.len() >= 2 {
                        doc.push_undo();
                        let mut pw = Pathwalk::new(points.clone(), active_level);
                        tool_studio.configure_new_pathwalk(&mut pw);
                        let id = pw.id;
                        doc.elements.insert(id, Element::Pathwalk(pw));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawStair { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        if p1.distance(mouse_pos) > 50.0 {
                            doc.push_undo();
                            let mut stair = Stair::new(*p1, mouse_pos, active_level);
                            tool_studio.configure_new_stair(doc, &mut stair);
                            let id = stair.id;
                            doc.elements.insert(id, Element::Stair(stair));
                            *selected_element = Some(id);
                        }
                        *start = None;
                    }
                }
            }

            Tool::DrawRamp { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        if p1.distance(mouse_pos) > 50.0 {
                            doc.push_undo();
                            let mut ramp = Ramp::new(*p1, mouse_pos, active_level);
                            tool_studio.configure_new_ramp(doc, &mut ramp);
                            let id = ramp.id;
                            doc.elements.insert(id, Element::Ramp(ramp));
                            *selected_element = Some(id);
                        }
                        *start = None;
                    }
                }
            }

            Tool::DrawRailing { points } => {
                let should_finish = (points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0)
                    || (points.len() >= 2 && ui.input(|i| i.key_pressed(Key::Enter)));
                if should_finish {
                    if points.len() >= 2 {
                        doc.push_undo();
                        let mut railing = Railing::new(points.clone(), active_level);
                        tool_studio.configure_new_railing(doc, &mut railing);
                        let id = railing.id;
                        doc.elements.insert(id, Element::Railing(railing));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::PlaceDoor => {
                // Find closest wall
                let mut closest_wall = None;
                let mut min_dist = 500.0;
                for (id, el) in &doc.elements {
                    if let Element::Wall(w) = el {
                        let (proj, t) = plinth_geom::polygon::closest_point_on_segment(mouse_pos, w.start, w.end);
                        let d = proj.distance(mouse_pos);
                        if d < min_dist {
                            min_dist = d;
                            closest_wall = Some((*id, t * w.length()));
                        }
                    }
                }
                if let Some((w_id, offset_along)) = closest_wall {
                    doc.push_undo();
                    let mut door = Door::new(w_id, offset_along);
                    tool_studio.configure_new_door(&mut door);
                    let id = door.id;
                    doc.elements.insert(id, Element::Door(door));
                    *selected_element = Some(id);
                }
            }

            Tool::PlaceWindow => {
                let mut closest_wall = None;
                let mut min_dist = 500.0;
                for (id, el) in &doc.elements {
                    if let Element::Wall(w) = el {
                        let (proj, t) = plinth_geom::polygon::closest_point_on_segment(mouse_pos, w.start, w.end);
                        let d = proj.distance(mouse_pos);
                        if d < min_dist {
                            min_dist = d;
                            closest_wall = Some((*id, t * w.length()));
                        }
                    }
                }
                if let Some((w_id, offset_along)) = closest_wall {
                    doc.push_undo();
                    let mut win = Window::new(w_id, offset_along);
                    tool_studio.configure_new_window(&mut win);
                    let id = win.id;
                    doc.elements.insert(id, Element::Window(win));
                    *selected_element = Some(id);
                }
            }

            Tool::DrawGrid { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        let tag = format!("{}", doc.grids.len() + 1);
                        let grid = plinth_model::GridLine::new(tag, *p1, mouse_pos);
                        doc.grids.insert(grid.id, grid);
                        *start = None;
                    }
                }
            }

            Tool::DrawDimension { start, p2, locked_offset } => {
                match (*start, *p2) {
                    (None, _) => {
                        *start = Some(mouse_pos);
                        *p2 = None;
                    }
                    (Some(p1), None) => {
                        if mouse_pos.distance(p1) > 10.0 {
                            if let Some(off) = *locked_offset {
                                // Continuous chained placement along existing locked offset!
                                doc.push_undo();
                                let mut dim = Dimension::new(doc.active_view_id, p1, mouse_pos, off);
                                dim.tick_mark = canvas_state.annotation_settings.tick_mark;
                                dim.font_size = canvas_state.annotation_settings.font_size;
                                dim.font_style = canvas_state.annotation_settings.font_style;
                                dim.color = canvas_state.annotation_settings.color;
                                tool_studio.configure_new_dimension(&mut dim);
                                dim.offset = off;
                                let id = dim.id;
                                doc.elements.insert(id, Element::Dimension(dim));
                                *selected_element = Some(id);
                                *start = Some(mouse_pos);
                            } else {
                                // Witness point 2 chosen: enter dynamic stretch mode!
                                *p2 = Some(mouse_pos);
                            }
                        }
                    }
                    (Some(p1), Some(second_pt)) => {
                        let diff = second_pt - p1;
                        if diff.length() > 10.0 {
                            let dir = diff.normalized();
                            let norm = plinth_geom::vec2(-dir.y, dir.x);
                            let mid = (p1 + second_pt) * 0.5;
                            let mut offset = (mouse_pos - mid).dot(norm);
                            if offset.abs() < 10.0 {
                                offset = if offset >= 0.0 { 250.0 } else { -250.0 };
                            }

                            // Collinear snapping to parallel dimension strings
                            for el in doc.elements.values() {
                                if let Element::Dimension(d_other) = el {
                                    if d_other.kind == plinth_model::DimensionKind::Linear {
                                        let o_diff = d_other.p2 - d_other.p1;
                                        if o_diff.length() > 10.0 {
                                            let o_dir = o_diff.normalized();
                                            if (dir.dot(o_dir)).abs() > 0.95 {
                                                let o_norm = plinth_geom::vec2(-o_dir.y, o_dir.x);
                                                let o_p_dim = d_other.p1 + o_norm * d_other.offset;
                                                let target_offset = (o_p_dim - p1).dot(norm);
                                                if (target_offset - offset).abs() < 400.0 {
                                                    offset = target_offset;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            doc.push_undo();
                            let mut dim = Dimension::new(doc.active_view_id, p1, second_pt, offset);
                            dim.tick_mark = canvas_state.annotation_settings.tick_mark;
                            dim.font_size = canvas_state.annotation_settings.font_size;
                            dim.font_style = canvas_state.annotation_settings.font_style;
                            dim.color = canvas_state.annotation_settings.color;
                            tool_studio.configure_new_dimension(&mut dim);
                            dim.offset = offset;
                            let id = dim.id;
                            doc.elements.insert(id, Element::Dimension(dim));
                            *selected_element = Some(id);

                            // Continuous chaining: lock this offset and advance start to second point
                            *start = Some(second_pt);
                            *p2 = None;
                            *locked_offset = Some(offset);
                        }
                    }
                }
            }

            Tool::DrawCircularDimension { center } => {
                match center {
                    None => {
                        *center = Some(mouse_pos);
                    }
                    Some(c) => {
                        let mut dim = Dimension::new_circular(
                            doc.active_view_id,
                            *c,
                            mouse_pos,
                            250.0,
                            false,
                            canvas_state.annotation_settings.tick_mark,
                            canvas_state.annotation_settings.font_size,
                            canvas_state.annotation_settings.font_style,
                            canvas_state.annotation_settings.color,
                        );
                        tool_studio.configure_new_dimension(&mut dim);
                        let id = dim.id;
                        doc.elements.insert(id, Element::Dimension(dim));
                        *selected_element = Some(id);
                        *center = None;
                    }
                }
            }

            Tool::DrawAngularDimension { apex, first_ray } => {
                match (*apex, *first_ray) {
                    (None, _) => {
                        *apex = Some(mouse_pos);
                    }
                    (Some(_), None) => {
                        *first_ray = Some(mouse_pos);
                    }
                    (Some(a), Some(r1)) => {
                        let mut dim = Dimension::new_angular(
                            doc.active_view_id,
                            a,
                            r1,
                            mouse_pos,
                            500.0,
                            canvas_state.annotation_settings.tick_mark,
                            canvas_state.annotation_settings.font_size,
                            canvas_state.annotation_settings.font_style,
                            canvas_state.annotation_settings.color,
                        );
                        tool_studio.configure_new_dimension(&mut dim);
                        let id = dim.id;
                        doc.elements.insert(id, Element::Dimension(dim));
                        *selected_element = Some(id);
                        *apex = None;
                        *first_ray = None;
                    }
                }
            }

            Tool::DrawTextAnnotation { pos } => {
                let p = pos.unwrap_or(mouse_pos);
                let mut note = plinth_model::TextNote::new(doc.active_view_id, p, &canvas_state.annotation_settings.default_text);
                note.font_size = canvas_state.annotation_settings.font_size;
                note.font_style = canvas_state.annotation_settings.font_style;
                note.color = canvas_state.annotation_settings.color;
                tool_studio.configure_new_text_note(&mut note);
                let id = note.id;
                doc.elements.insert(id, Element::TextNote(note));
                *selected_element = Some(id);
                *pos = None;
            }

            Tool::DrawLeader { target_pt, shoulder_pt } => {
                match (*target_pt, *shoulder_pt) {
                    (None, _) => {
                        *target_pt = Some(mouse_pos);
                    }
                    (Some(_), None) => {
                        *shoulder_pt = Some(mouse_pos);
                    }
                    (Some(t), Some(s)) => {
                        let mut ldr = plinth_model::Leader::new(
                            doc.active_view_id,
                            t,
                            s,
                            mouse_pos,
                            &canvas_state.annotation_settings.default_text,
                            canvas_state.annotation_settings.tick_mark,
                            canvas_state.annotation_settings.font_size,
                            canvas_state.annotation_settings.font_style,
                            canvas_state.annotation_settings.color,
                        );
                        tool_studio.configure_new_leader(&mut ldr);
                        let id = ldr.id;
                        doc.elements.insert(id, Element::Leader(ldr));
                        *selected_element = Some(id);
                        *target_pt = None;
                        *shoulder_pt = None;
                    }
                }
            }

            Tool::DrawDetailLine { start } => {
                match start {
                    None => {
                        *start = Some(mouse_pos);
                    }
                    Some(p1) => {
                        if p1.distance(mouse_pos) > 5.0 {
                            doc.push_undo();
                            let mut line = DetailLine::new(doc.active_view_id, *p1, mouse_pos, "Medium");
                            tool_studio.configure_new_detail_line(&mut line);
                            let id = line.id;
                            doc.elements.insert(id, Element::DetailLine(line));
                            *selected_element = Some(id);

                            if canvas_state.snap_settings.wall_continuous_chain {
                                *start = Some(mouse_pos);
                            } else {
                                *start = None;
                            }
                        } else {
                            *start = None;
                        }
                    }
                }
            }

            Tool::Move { from } => {
                match from {
                    None => {
                        *from = Some(mouse_pos);
                        canvas_state.move_base_3d = canvas_state.snapped_3d_pos;
                    }
                    Some(p1) => {
                        let target_ids: Vec<plinth_model::ElementId> = if !canvas_state.selected_elements.is_empty() {
                            canvas_state.selected_elements.clone()
                        } else if let Some(sel_id) = *selected_element {
                            vec![sel_id]
                        } else {
                            Vec::new()
                        };

                        if !target_ids.is_empty() {
                            doc.push_undo();
                            let delta_3d = if let (Some(base3), Some(target3)) = (canvas_state.move_base_3d, canvas_state.snapped_3d_pos) {
                                let mut d = target3 - base3;
                                let key_x = ui.input(|i| i.key_down(egui::Key::X));
                                let key_y = ui.input(|i| i.key_down(egui::Key::Y));
                                let key_z = ui.input(|i| i.key_down(egui::Key::Z));
                                let shift_held = ui.input(|i| i.modifiers.shift);
                                if key_x {
                                    d = plinth_geom::Vec3::new(d.x, 0.0, 0.0);
                                } else if key_y {
                                    d = plinth_geom::Vec3::new(0.0, d.y, 0.0);
                                } else if key_z {
                                    d = plinth_geom::Vec3::new(0.0, 0.0, d.z);
                                } else if shift_held {
                                    if d.x.abs() >= d.y.abs() && d.x.abs() >= d.z.abs() {
                                        d = plinth_geom::Vec3::new(d.x, 0.0, 0.0);
                                    } else if d.y.abs() >= d.x.abs() && d.y.abs() >= d.z.abs() {
                                        d = plinth_geom::Vec3::new(0.0, d.y, 0.0);
                                    } else {
                                        d = plinth_geom::Vec3::new(0.0, 0.0, d.z);
                                    }
                                }
                                d
                            } else {
                                let d2 = mouse_pos - *p1;
                                match active_view.kind {
                                    plinth_model::ViewKind::FloorPlan | plinth_model::ViewKind::CeilingPlan => plinth_geom::Vec3::new(d2.x, d2.y, 0.0),
                                    plinth_model::ViewKind::ElevationSouth => plinth_geom::Vec3::new(d2.x, 0.0, d2.y),
                                    plinth_model::ViewKind::ElevationNorth => plinth_geom::Vec3::new(-d2.x, 0.0, d2.y),
                                    plinth_model::ViewKind::ElevationEast => plinth_geom::Vec3::new(0.0, -d2.x, d2.y),
                                    plinth_model::ViewKind::ElevationWest => plinth_geom::Vec3::new(0.0, d2.x, d2.y),
                                    plinth_model::ViewKind::Section => {
                                        let (s1, s2) = active_view.section_line.unwrap_or((ModelVec2::ZERO, ModelVec2::new(10000.0, 0.0)));
                                        let diff = s2 - s1;
                                        let len = diff.length().max(1.0);
                                        let u_dir = diff / len;
                                        plinth_geom::Vec3::new(d2.x * u_dir.x, d2.x * u_dir.y, d2.y)
                                    }
                                    _ => plinth_geom::Vec3::new(d2.x, d2.y, 0.0),
                                }
                            };

                            for sel_id in target_ids {
                                if let Some(el) = doc.elements.get_mut(&sel_id) {
                                    el.translate_3d(delta_3d);
                                }
                            }
                            canvas_state.invalidate_gpu_cache();
                        }
                        *from = None;
                        canvas_state.move_base_3d = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::Rotate { center, start_angle } => {
                match (*center, *start_angle) {
                    (None, _) => {
                        *center = Some(mouse_pos);
                    }
                    (Some(c), None) => {
                        *start_angle = Some((mouse_pos - c).angle());
                    }
                    (Some(c), Some(a1)) => {
                        let shift_held = ui.input(|i| i.modifiers.shift);
                        let mut a2 = (mouse_pos - c).angle();
                        if shift_held {
                            let step = 15.0_f64.to_radians();
                            a2 = (a2 / step).round() * step;
                        }
                        let d_angle = a2 - a1;
                        let target_ids: Vec<plinth_model::ElementId> = if !canvas_state.selected_elements.is_empty() {
                            canvas_state.selected_elements.clone()
                        } else if let Some(sel_id) = *selected_element {
                            vec![sel_id]
                        } else {
                            Vec::new()
                        };

                        if !target_ids.is_empty() && d_angle.abs() > 1e-4 {
                            doc.push_undo();
                            for sel_id in target_ids {
                                if let Some(el) = doc.elements.get_mut(&sel_id) {
                                    el.transform2d(&Xform2::rotation_about(c, d_angle));
                                }
                            }
                            canvas_state.invalidate_gpu_cache();
                        }
                        *center = None;
                        *start_angle = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::Mirror { axis_p1 } => {
                match axis_p1 {
                    None => {
                        *axis_p1 = Some(mouse_pos);
                    }
                    Some(p1) => {
                        let shift_held = ui.input(|i| i.modifiers.shift);
                        let mut p2 = mouse_pos;
                        if shift_held {
                            let dx = (p2.x - p1.x).abs();
                            let dy = (p2.y - p1.y).abs();
                            if dx >= dy {
                                p2.y = p1.y;
                            } else {
                                p2.x = p1.x;
                            }
                        }
                        let target_ids: Vec<plinth_model::ElementId> = if !canvas_state.selected_elements.is_empty() {
                            canvas_state.selected_elements.clone()
                        } else if let Some(sel_id) = *selected_element {
                            vec![sel_id]
                        } else {
                            Vec::new()
                        };

                        if !target_ids.is_empty() && p1.distance(p2) > 1.0 {
                            doc.push_undo();
                            for sel_id in target_ids {
                                if let Some(el) = doc.elements.get_mut(&sel_id) {
                                    el.transform2d(&Xform2::mirror_about(*p1, p2));
                                }
                            }
                            canvas_state.invalidate_gpu_cache();
                        }
                        *axis_p1 = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawRectSurface { corner1 } => {
                match corner1 {
                    None => {
                        *corner1 = Some(mouse_pos);
                    }
                    Some(p1) => {
                        let p2 = mouse_pos;
                        if p1.distance(p2) > 10.0 {
                            let elev = doc.level_elevation(active_level);
                            let min_u = p1.x.min(p2.x);
                            let max_u = p1.x.max(p2.x);
                            let min_v = p1.y.min(p2.y);
                            let max_v = p1.y.max(p2.y);
                            let corners_uv = vec![
                                ModelVec2::new(min_u, min_v),
                                ModelVec2::new(max_u, min_v),
                                ModelVec2::new(max_u, max_v),
                                ModelVec2::new(min_u, max_v),
                            ];
                            let corners_3d: Vec<plinth_geom::Vec3> = corners_uv.iter().map(|&uv| workplane_to_3d(uv, elev)).collect();
                            let mut surf = plinth_model::FreeformSurface::new_polygon(corners_uv, elev, active_level);
                            surf.boundary_3d = Some(corners_3d);
                            if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                                surf.host_wall = host_element;
                            }
                            let id = surf.id;
                            doc.elements.insert(id, Element::FreeformSurface(surf));
                            *selected_element = Some(id);
                            canvas_state.surface_action_popup = Some(id);
                        }
                        *corner1 = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawCircleSurface { center } => {
                match center {
                    None => {
                        *center = Some(mouse_pos);
                    }
                    Some(c) => {
                        let rad = c.distance(mouse_pos);
                        if rad > 10.0 {
                            let elev = doc.level_elevation(active_level);
                            let segs = 24;
                            let mut boundary_uv = Vec::with_capacity(segs);
                            for i in 0..segs {
                                let theta = (i as f64 / segs as f64) * std::f64::consts::TAU;
                                boundary_uv.push(ModelVec2::new(c.x + rad * theta.cos(), c.y + rad * theta.sin()));
                            }
                            let boundary_3d: Vec<plinth_geom::Vec3> = boundary_uv.iter().map(|&uv| workplane_to_3d(uv, elev)).collect();
                            let mut surf = plinth_model::FreeformSurface::new_circle(*c, rad, segs, elev, active_level);
                            surf.boundary_3d = Some(boundary_3d);
                            if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                                surf.host_wall = host_element;
                            }
                            let id = surf.id;
                            doc.elements.insert(id, Element::FreeformSurface(surf));
                            *selected_element = Some(id);
                            canvas_state.surface_action_popup = Some(id);
                        }
                        *center = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawPolygonSurface { points } => {
                let should_close = points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0;
                if should_close {
                    if points.len() >= 3 {
                        let pts = points.clone();
                        let elev = doc.level_elevation(active_level);
                        let pts_3d: Vec<plinth_geom::Vec3> = pts.iter().map(|&uv| workplane_to_3d(uv, elev)).collect();
                        let mut surf = plinth_model::FreeformSurface::new_polygon(pts, elev, active_level);
                        surf.boundary_3d = Some(pts_3d);
                        if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                            surf.host_wall = host_element;
                        }
                        let id = surf.id;
                        doc.elements.insert(id, Element::FreeformSurface(surf));
                        *selected_element = Some(id);
                        canvas_state.surface_action_popup = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::PushPull { target_id, drag_start, current_height } => {
                match *target_id {
                    None => {
                        let mut found = None;
                        for (id, el) in &doc.elements {
                            match el {
                                Element::FreeformSurface(s) => {
                                    let hit = if is_3d {
                                        let scr_pts: Vec<Pos2> = s.boundary.iter().map(|&p| project_point(p, s.elevation)).collect();
                                        if let Some(hp) = ui.input(|i| i.pointer.hover_pos()) {
                                            is_point_in_quad(hp, &scr_pts)
                                        } else {
                                            false
                                        }
                                    } else {
                                        plinth_geom::polygon::point_in_polygon(mouse_pos, &s.boundary)
                                    };
                                    if hit {
                                        found = Some(*id);
                                        break;
                                    }
                                }
                                Element::FreeformSolid(f) => {
                                    let hit = if is_3d {
                                        let scr_pts: Vec<Pos2> = f.boundary.iter().map(|&p| project_point(p, f.z_max)).collect();
                                        if let Some(hp) = ui.input(|i| i.pointer.hover_pos()) {
                                            is_point_in_quad(hp, &scr_pts)
                                        } else {
                                            false
                                        }
                                    } else {
                                        plinth_geom::polygon::point_in_polygon(mouse_pos, &f.boundary)
                                    };
                                    if hit {
                                        found = Some(*id);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(id) = found {
                            *target_id = Some(id);
                            *drag_start = Some(mouse_pos);
                            *selected_element = Some(id);
                        }
                    }
                    Some(id) => {
                        let h = if let Some(ds) = drag_start {
                            ((mouse_pos.y - ds.y).abs() * 2.0).clamp(100.0, 50000.0)
                        } else {
                            *current_height
                        };
                        if let Some(el) = doc.elements.remove(&id) {
                            match el {
                                Element::FreeformSurface(s) => {
                                    let solid = s.extrude(h);
                                    let new_id = solid.id;
                                    doc.elements.insert(new_id, Element::FreeformSolid(solid));
                                    *selected_element = Some(new_id);
                                }
                                Element::FreeformSolid(mut f) => {
                                    f.push_pull(h);
                                    let new_id = f.id;
                                    doc.elements.insert(new_id, Element::FreeformSolid(f));
                                    *selected_element = Some(new_id);
                                }
                                other => {
                                    doc.elements.insert(id, other);
                                }
                            }
                        }
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawBoxPrimitive { corner1 } => {
                match corner1 {
                    None => {
                        *corner1 = Some(mouse_pos);
                    }
                    Some(p1) => {
                        let p2 = mouse_pos;
                        if p1.distance(p2) > 10.0 {
                            let z0 = doc.level_elevation(active_level);
                            let z1 = z0 + 3000.0;
                            let mut b = plinth_model::FreeformSolid::new_box(*p1, p2, z0, z1, active_level);
                            if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                                b.host_wall = host_element;
                            }
                            let id = b.id;
                            doc.elements.insert(id, Element::FreeformSolid(b));
                            *selected_element = Some(id);
                        }
                        *corner1 = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawCylinderPrimitive { center } => {
                match center {
                    None => {
                        *center = Some(mouse_pos);
                    }
                    Some(c) => {
                        let rad = c.distance(mouse_pos);
                        if rad > 10.0 {
                            let z0 = doc.level_elevation(active_level);
                            let z1 = z0 + 3000.0;
                            let mut cyl = plinth_model::FreeformSolid::new_cylinder(*c, rad, z0, z1, 24, active_level);
                            if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                                cyl.host_wall = host_element;
                            }
                            let id = cyl.id;
                            doc.elements.insert(id, Element::FreeformSolid(cyl));
                            *selected_element = Some(id);
                        }
                        *center = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawSpherePrimitive { center } => {
                match center {
                    None => {
                        *center = Some(mouse_pos);
                    }
                    Some(c) => {
                        let rad = c.distance(mouse_pos);
                        if rad > 10.0 {
                            let z0 = doc.level_elevation(active_level) + rad;
                            let mut sph = plinth_model::FreeformSolid::new_sphere(*c, rad, z0, active_level);
                            if let WorkplaneMode::Surface { host_element, .. } = active_workplane {
                                sph.host_wall = host_element;
                            }
                            let id = sph.id;
                            doc.elements.insert(id, Element::FreeformSolid(sph));
                            *selected_element = Some(id);
                        }
                        *center = None;
                        *active_tool = Tool::Select;
                    }
                }
            }

            Tool::DrawRoof { shape, points } => {
                let should_close = points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0;
                if should_close || (points.len() >= 3 && ui.input(|i| i.key_pressed(Key::Enter))) {
                    if points.len() >= 3 {
                        doc.push_undo();
                        let mut roof = plinth_model::Roof::new(points.clone(), active_level);
                        tool_studio.configure_new_roof(doc, &mut roof);
                        roof.shape = *shape;
                        let id = roof.id;
                        doc.elements.insert(id, Element::Roof(roof));
                        *selected_element = Some(id);
                    }
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawFascia { points } => {
                let should_close = points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0;
                if should_close || (points.len() >= 2 && ui.input(|i| i.key_pressed(Key::Enter))) {
                    doc.push_undo();
                    let mut pts = points.clone();
                    if should_close {
                        pts.push(points[0]);
                    }
                    let mut fascia = plinth_model::Fascia::new(pts, active_level);
                    tool_studio.configure_new_fascia(doc, &mut fascia);
                    let id = fascia.id;
                    doc.elements.insert(id, Element::Fascia(fascia));
                    *selected_element = Some(id);
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }

            Tool::DrawGutter { points } => {
                let should_close = points.len() >= 2 && mouse_pos.distance(points[0]) < 300.0;
                if should_close || (points.len() >= 2 && ui.input(|i| i.key_pressed(Key::Enter))) {
                    doc.push_undo();
                    let mut gutter = plinth_model::Gutter::new(points.clone(), active_level);
                    tool_studio.configure_new_gutter(doc, &mut gutter);
                    let id = gutter.id;
                    doc.elements.insert(id, Element::Gutter(gutter));
                    *selected_element = Some(id);
                    points.clear();
                } else {
                    points.push(mouse_pos);
                }
            }
        }
    }

    // 6. Draw Live Window / Crossing Selection Marquee Overlay
    if is_active {
        if let Some(sel_box) = canvas_state.selection_box {
            let sel_rect = sel_box.rect();
            let is_crossing = sel_box.is_crossing();
            if is_crossing {
                // Crossing Selection (Right to Left): Translucent CAD green fill + dashed green outline
                let fill = Color32::from_rgba_premultiplied(35, 180, 85, 45);
                let border_color = Color32::from_rgb(45, 200, 95);
                vp_painter.rect_filled(sel_rect, 0.0, fill);
                draw_dashed_rect(&vp_painter, sel_rect, Stroke::new(1.5, border_color), 6.0, 4.0);
            } else {
                // Window Selection (Left to Right): Translucent CAD blue fill + solid blue outline
                let fill = Color32::from_rgba_premultiplied(30, 120, 240, 45);
                let border_color = Color32::from_rgb(40, 130, 250);
                vp_painter.rect_filled(sel_rect, 0.0, fill);
                vp_painter.rect_stroke(sel_rect, 0.0, Stroke::new(1.5, border_color), egui::StrokeKind::Inside);
            }
        }
    }
}

fn render_workplane_mini_pie_hud(
    painter: &egui::Painter,
    ui: &mut Ui,
    vp_rect: Rect,
    canvas_state: &mut CanvasState,
    theme: &AppTheme,
) -> bool {
    let hud_size = Vec2::new(325.0, 32.0);
    let hud_pos = Pos2::new(vp_rect.center().x - hud_size.x * 0.5, vp_rect.min.y + 10.0);
    let hud_rect = Rect::from_min_size(hud_pos, hud_size);

    // Pill background
    painter.rect_filled(hud_rect, 16.0, Color32::from_rgba_premultiplied(22, 24, 28, 235));
    painter.rect_stroke(hud_rect, 16.0, Stroke::new(1.2, theme.border_color), egui::StrokeKind::Inside);

    painter.text(
        Pos2::new(hud_rect.min.x + 10.0, hud_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "📐 Plane:",
        egui::FontId::proportional(11.0),
        theme.text_muted,
    );

    let buttons = [
        ("⬒ XY Floor", WorkplaneMode::XY),
        ("⬓ XZ Front", WorkplaneMode::XZ),
        ("⬔ YZ Side", WorkplaneMode::YZ),
    ];

    let mut x_cursor = hud_rect.min.x + 64.0;
    let mut consumed = false;

    for (label, mode) in buttons {
        let is_active = canvas_state.workplane == mode && !canvas_state.picking_surface_workplane;
        let btn_size = Vec2::new(58.0, 22.0);
        let btn_rect = Rect::from_min_size(Pos2::new(x_cursor, hud_rect.center().y - 11.0), btn_size);
        let resp = ui.allocate_rect(btn_rect, egui::Sense::click());

        if resp.clicked() {
            canvas_state.workplane = mode;
            canvas_state.picking_surface_workplane = false;
            consumed = true;
        }

        let bg = if is_active {
            theme.accent
        } else if resp.hovered() {
            theme.hover_highlight
        } else {
            Color32::TRANSPARENT
        };
        if is_active || resp.hovered() {
            painter.rect_filled(btn_rect, 6.0, bg);
        }
        let fg = if is_active { Color32::WHITE } else { theme.text_primary };
        painter.text(btn_rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(10.5), fg);
        x_cursor += btn_size.x + 4.0;
    }

    let pick_active = canvas_state.picking_surface_workplane || matches!(canvas_state.workplane, WorkplaneMode::Surface { .. });
    let pick_size = Vec2::new(66.0, 22.0);
    let pick_rect = Rect::from_min_size(Pos2::new(x_cursor, hud_rect.center().y - 11.0), pick_size);
    let pick_resp = ui.allocate_rect(pick_rect, egui::Sense::click());

    if pick_resp.clicked() {
        canvas_state.picking_surface_workplane = !canvas_state.picking_surface_workplane;
        consumed = true;
    }

    let pick_bg = if pick_active {
        Color32::from_rgb(255, 120, 0)
    } else if pick_resp.hovered() {
        theme.hover_highlight
    } else {
        Color32::TRANSPARENT
    };
    if pick_active || pick_resp.hovered() {
        painter.rect_filled(pick_rect, 6.0, pick_bg);
    }
    let pick_fg = if pick_active { Color32::WHITE } else { theme.text_primary };
    let pick_label = if canvas_state.picking_surface_workplane { "Click Face..." } else { "🎯 Pick Face" };
    painter.text(pick_rect.center(), egui::Align2::CENTER_CENTER, pick_label, egui::FontId::proportional(10.0), pick_fg);

    consumed
}

fn render_surface_action_popup(
    painter: &egui::Painter,
    ui: &mut Ui,
    vp_rect: Rect,
    doc: &mut Document,
    selected_element: &mut Option<ElementId>,
    canvas_state: &mut CanvasState,
    theme: &AppTheme,
) -> bool {
    let target_id = match canvas_state.surface_action_popup.or(*selected_element) {
        Some(id) => id,
        None => return false,
    };

    let is_surf = match doc.elements.get(&target_id) {
        Some(Element::FreeformSurface(_)) => true,
        _ => false,
    };
    if !is_surf {
        canvas_state.surface_action_popup = None;
        return false;
    }

    let popup_size = Vec2::new(268.0, 42.0);
    let popup_pos = Pos2::new(
        (vp_rect.center().x - popup_size.x * 0.5).clamp(vp_rect.min.x + 10.0, vp_rect.max.x - popup_size.x - 10.0),
        vp_rect.max.y - 60.0,
    );
    let popup_rect = Rect::from_min_size(popup_pos, popup_size);

    painter.rect_filled(popup_rect, 8.0, Color32::from_rgba_premultiplied(20, 22, 26, 245));
    painter.rect_stroke(popup_rect, 8.0, Stroke::new(1.4, theme.accent), egui::StrokeKind::Inside);

    let mut consumed = false;

    // 1. Extrude Button
    let ext_rect = Rect::from_min_size(Pos2::new(popup_rect.min.x + 8.0, popup_rect.min.y + 8.0), Vec2::new(110.0, 26.0));
    let ext_resp = ui.allocate_rect(ext_rect, egui::Sense::click());
    if ext_resp.clicked() {
        if let Some(el) = doc.elements.remove(&target_id) {
            if let Element::FreeformSurface(s) = el {
                let solid = s.extrude(2800.0);
                let new_id = solid.id;
                doc.elements.insert(new_id, Element::FreeformSolid(solid));
                *selected_element = Some(new_id);
            }
        }
        canvas_state.surface_action_popup = None;
        return true;
    }
    let ext_bg = if ext_resp.hovered() { Color32::from_rgb(255, 140, 20) } else { Color32::from_rgb(255, 120, 0) };
    painter.rect_filled(ext_rect, 5.0, ext_bg);
    painter.text(ext_rect.center(), egui::Align2::CENTER_CENTER, "⬆ Extrude Solid", egui::FontId::proportional(11.0), Color32::WHITE);

    // 2. Create Hole Button
    let hole_rect = Rect::from_min_size(Pos2::new(popup_rect.min.x + 124.0, popup_rect.min.y + 8.0), Vec2::new(106.0, 26.0));
    let hole_resp = ui.allocate_rect(hole_rect, egui::Sense::click());
    if hole_resp.clicked() {
        if let Some(Element::FreeformSurface(s)) = doc.elements.get_mut(&target_id) {
            s.is_hole = !s.is_hole;
            if s.is_hole && s.ifc_category.is_none() {
                s.ifc_category = Some("Opening".into());
            }
        }
        canvas_state.surface_action_popup = None;
        return true;
    }
    let is_currently_hole = doc.elements.get(&target_id).map_or(false, |el| match el { Element::FreeformSurface(s) => s.is_hole, _ => false });
    let hole_bg = if is_currently_hole {
        Color32::from_rgb(200, 60, 40)
    } else if hole_resp.hovered() {
        theme.hover_highlight
    } else {
        Color32::from_gray(50)
    };
    painter.rect_filled(hole_rect, 5.0, hole_bg);
    let hole_text = if is_currently_hole { "🕳 Cutout (On)" } else { "🕳 Create Hole" };
    painter.text(hole_rect.center(), egui::Align2::CENTER_CENTER, hole_text, egui::FontId::proportional(11.0), Color32::WHITE);

    // 3. Dismiss 'X'
    let x_rect = Rect::from_min_size(Pos2::new(popup_rect.max.x - 24.0, popup_rect.min.y + 9.0), Vec2::new(18.0, 24.0));
    let x_resp = ui.allocate_rect(x_rect, egui::Sense::click());
    if x_resp.clicked() {
        canvas_state.surface_action_popup = None;
        consumed = true;
    }
    painter.text(x_rect.center(), egui::Align2::CENTER_CENTER, "✕", egui::FontId::proportional(11.0), theme.text_muted);

    consumed
}

/// Floating ViewCube in the top-right corner of the drawing canvas.
fn render_viewcube(
    painter: &egui::Painter,
    ui: &Ui,
    rect: Rect,
    cam: &mut plinth_model::Camera3D,
    theme: &AppTheme,
) -> bool {
    let cube_w = 96.0f32;
    let cube_h = 104.0f32;
    let cube_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - cube_w - 14.0, rect.min.y + 14.0),
        egui::vec2(cube_w, cube_h),
    );

    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let consumed = cube_rect.contains(mouse_pos);

    // Card background & border
    let bg_color = Color32::from_rgba_premultiplied(
        theme.panel_bg.r(),
        theme.panel_bg.g(),
        theme.panel_bg.b(),
        240,
    );
    painter.rect_filled(cube_rect, 6.0, bg_color);
    painter.rect_stroke(cube_rect, 6.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);

    // Header: "VIEW CUBE" Title
    painter.text(
        Pos2::new(cube_rect.min.x + 8.0, cube_rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        "VIEW CUBE",
        egui::FontId::proportional(9.0),
        theme.text_muted,
    );

    // Mini 3D Cube Graphic (Rotating live in sync with camera)
    let center = Pos2::new(cube_rect.center().x, cube_rect.min.y + 43.0);
    let half_s = 21.0f64;
    let (sin_y, cos_y) = cam.yaw.sin_cos();
    let (sin_p, cos_p) = cam.pitch.sin_cos();
    let cam_forward = plinth_geom::Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);

    let tx = |x: f64, y: f64, z: f64| -> Pos2 {
        let x1 = x * cos_y - y * sin_y;
        let y1 = x * sin_y + y * cos_y;
        let x2 = x1;
        let _y2 = y1 * cos_p - z * sin_p;
        let z2 = y1 * sin_p + z * cos_p;
        // Inverted Y on screen
        Pos2::new(center.x + x2 as f32, center.y - z2 as f32)
    };

    // 6 faces of the unit cube: (corners, normal, label, target_yaw, target_pitch, fill)
    let faces = [
        (
            vec![tx(-half_s, -half_s, half_s), tx(half_s, -half_s, half_s), tx(half_s, half_s, half_s), tx(-half_s, half_s, half_s)],
            plinth_geom::Vec3::Z,
            "TOP",
            0.0,
            1.50,
            Color32::from_rgb(215, 220, 230),
        ),
        (
            vec![tx(-half_s, -half_s, -half_s), tx(half_s, -half_s, -half_s), tx(half_s, -half_s, half_s), tx(-half_s, -half_s, half_s)],
            plinth_geom::Vec3::new(0.0, -1.0, 0.0),
            "FRONT",
            0.0,
            0.0,
            Color32::from_rgb(180, 185, 195),
        ),
        (
            vec![tx(half_s, -half_s, -half_s), tx(half_s, half_s, -half_s), tx(half_s, half_s, half_s), tx(half_s, -half_s, half_s)],
            plinth_geom::Vec3::X,
            "RIGHT",
            std::f64::consts::FRAC_PI_2,
            0.0,
            Color32::from_rgb(160, 165, 175),
        ),
        (
            vec![tx(half_s, half_s, -half_s), tx(-half_s, half_s, -half_s), tx(-half_s, half_s, half_s), tx(half_s, half_s, half_s)],
            plinth_geom::Vec3::new(0.0, 1.0, 0.0),
            "BACK",
            std::f64::consts::PI,
            0.0,
            Color32::from_rgb(170, 175, 185),
        ),
        (
            vec![tx(-half_s, half_s, -half_s), tx(-half_s, -half_s, -half_s), tx(-half_s, -half_s, half_s), tx(-half_s, half_s, half_s)],
            plinth_geom::Vec3::new(-1.0, 0.0, 0.0),
            "LEFT",
            3.0 * std::f64::consts::FRAC_PI_2,
            0.0,
            Color32::from_rgb(190, 195, 205),
        ),
        (
            vec![tx(-half_s, half_s, -half_s), tx(half_s, half_s, -half_s), tx(half_s, -half_s, -half_s), tx(-half_s, -half_s, -half_s)],
            plinth_geom::Vec3::new(0.0, 0.0, -1.0),
            "BOTTOM",
            0.0,
            -1.50,
            Color32::from_rgb(150, 155, 165),
        ),
    ];

    for (pts, normal, label, ty, tp, fill) in faces {
        if normal.dot(cam_forward) < -1e-4 {
            let is_hover = is_point_in_quad(mouse_pos, &pts);
            let face_fill = if is_hover {
                theme.accent.gamma_multiply(0.35)
            } else {
                fill
            };
            let stroke = Stroke::new(
                if is_hover { 1.5 } else { 1.0 },
                if is_hover { theme.accent } else { Color32::from_rgb(50, 55, 65) },
            );
            painter.add(egui::Shape::convex_polygon(pts.clone(), face_fill, stroke));

            let face_center = Pos2::new(
                pts.iter().map(|p| p.x).sum::<f32>() / pts.len() as f32,
                pts.iter().map(|p| p.y).sum::<f32>() / pts.len() as f32,
            );

            painter.text(
                face_center,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(8.5),
                if is_hover { theme.accent } else { Color32::from_rgb(30, 30, 35) },
            );

            if is_hover && clicked {
                cam.yaw = ty;
                cam.pitch = tp;
            }
        }
    }

    // Simplified Action Buttons: [ ⌂ ISO ] and [ 📐 Ortho / 👁 Persp ]
    let btn_y = cube_rect.min.y + 76.0;
    let btn_h = 20.0f32;
    let btn_w = 40.0f32;
    let btn_gap = 4.0f32;

    // 1. Isometric Reset Button [ ⌂ ISO ]
    let iso_rect = Rect::from_min_size(
        Pos2::new(cube_rect.min.x + 6.0, btn_y),
        egui::vec2(btn_w, btn_h),
    );
    let is_iso = (cam.pitch - 0.55).abs() < 0.1 && (cam.yaw - (-std::f64::consts::FRAC_PI_4)).abs() < 0.1;
    let iso_hover = iso_rect.contains(mouse_pos);
    painter.rect_filled(
        iso_rect,
        3.0,
        if is_iso || iso_hover { theme.hover_highlight } else { theme.panel_bg },
    );
    painter.rect_stroke(iso_rect, 3.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);
    painter.text(
        iso_rect.center(),
        egui::Align2::CENTER_CENTER,
        "⌂ ISO",
        egui::FontId::proportional(9.5),
        if is_iso || iso_hover { theme.accent } else { theme.text_primary },
    );
    if iso_hover && clicked {
        cam.yaw = -std::f64::consts::FRAC_PI_4;
        cam.pitch = 0.55;
        cam.distance = 25000.0;
    }

    // 2. Perspective vs Orthographic Toggle Button [ 📐 / 👁 ]
    let persp_rect = Rect::from_min_size(
        Pos2::new(cube_rect.min.x + 6.0 + btn_w + btn_gap, btn_y),
        egui::vec2(btn_w, btn_h),
    );
    let is_persp = cam.fov_degrees > 1.0;
    let persp_hover = persp_rect.contains(mouse_pos);
    painter.rect_filled(
        persp_rect,
        3.0,
        if persp_hover { theme.hover_highlight } else { theme.panel_bg },
    );
    painter.rect_stroke(persp_rect, 3.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);
    painter.text(
        persp_rect.center(),
        egui::Align2::CENTER_CENTER,
        if is_persp { "👁 Persp" } else { "📐 Ortho" },
        egui::FontId::proportional(9.5),
        if is_persp { theme.accent } else { theme.text_primary },
    );
    if persp_hover && clicked {
        cam.fov_degrees = if is_persp { 0.0 } else { 50.0 };
    }

    consumed
}

/// Renders an architectural dashed rectangle outline for Crossing Selection.
pub fn draw_dashed_rect(
    painter: &egui::Painter,
    rect: Rect,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let draw_dashed_segment = |p0: Pos2, p1: Pos2| {
        let dir = p1 - p0;
        let len = dir.length();
        if len <= 0.001 {
            return;
        }
        let u = dir / len;
        let step = dash_len + gap_len;
        let mut d = 0.0;
        while d < len {
            let start = p0 + u * d;
            let end = p0 + u * (d + dash_len).min(len);
            painter.line_segment([start, end], stroke);
            d += step;
        }
    };
    let lt = rect.left_top();
    let rt = rect.right_top();
    let rb = rect.right_bottom();
    let lb = rect.left_bottom();
    draw_dashed_segment(lt, rt);
    draw_dashed_segment(rt, rb);
    draw_dashed_segment(rb, lb);
    draw_dashed_segment(lb, lt);
}

/// Tests whether two 2D line segments (p1..p2) and (p3..p4) intersect.
pub fn segments_intersect(p1: Pos2, p2: Pos2, p3: Pos2, p4: Pos2) -> bool {
    fn ccw(a: Pos2, b: Pos2, c: Pos2) -> f32 {
        (c.y - a.y) * (b.x - a.x) - (b.y - a.y) * (c.x - a.x)
    }
    let d1 = ccw(p3, p4, p1);
    let d2 = ccw(p3, p4, p2);
    let d3 = ccw(p1, p2, p3);
    let d4 = ccw(p1, p2, p4);

    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }

    fn on_segment(p: Pos2, a: Pos2, b: Pos2) -> bool {
        p.x >= a.x.min(b.x) - 1e-3
            && p.x <= a.x.max(b.x) + 1e-3
            && p.y >= a.y.min(b.y) - 1e-3
            && p.y <= a.y.max(b.y) + 1e-3
    }

    if d1.abs() < 1e-4 && on_segment(p1, p3, p4) {
        return true;
    }
    if d2.abs() < 1e-4 && on_segment(p2, p3, p4) {
        return true;
    }
    if d3.abs() < 1e-4 && on_segment(p3, p1, p2) {
        return true;
    }
    if d4.abs() < 1e-4 && on_segment(p4, p1, p2) {
        return true;
    }

    false
}

/// Tests whether a 2D point lies inside a screen polygon.
pub fn point_in_screen_poly(pt: Pos2, poly: &[Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        let denom = pj.y - pi.y;
        if ((pi.y > pt.y) != (pj.y > pt.y)) && denom.abs() > 1e-5 && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / denom + pi.x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Geometric projection of a BIM/CAD element onto the active viewport screen space.
#[derive(Clone, Debug, Default)]
pub struct ProjectedElementGeometry {
    pub vertices: Vec<Pos2>,
    pub edges: Vec<(Pos2, Pos2)>,
    pub polygons: Vec<Vec<Pos2>>,
}

impl ProjectedElementGeometry {
    pub fn from_polygon(pts: Vec<Pos2>) -> Self {
        let n = pts.len();
        let mut edges = Vec::new();
        if n >= 2 {
            for i in 0..n {
                let next = (i + 1) % n;
                edges.push((pts[i], pts[next]));
            }
        }
        let polygons = if n >= 3 { vec![pts.clone()] } else { Vec::new() };
        Self {
            vertices: pts,
            edges,
            polygons,
        }
    }

    pub fn from_segment(p1: Pos2, p2: Pos2) -> Self {
        Self {
            vertices: vec![p1, p2],
            edges: vec![(p1, p2)],
            polygons: Vec::new(),
        }
    }

    pub fn from_box(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2) -> Self {
        Self::from_polygon(vec![p0, p1, p2, p3])
    }
}

/// Evaluates whether an element's projected screen geometry satisfies Window or Crossing selection.
pub fn test_element_marquee(
    geom: &ProjectedElementGeometry,
    sel_rect: Rect,
    is_crossing: bool,
) -> bool {
    if geom.vertices.is_empty() {
        return false;
    }
    if !is_crossing {
        // Window Selection: Every single vertex must be strictly inside the rectangle
        geom.vertices.iter().all(|p| sel_rect.contains(*p))
    } else {
        // Crossing Selection: Inside OR Intersecting
        // 1. Any element vertex is inside sel_rect
        if geom.vertices.iter().any(|p| sel_rect.contains(*p)) {
            return true;
        }
        // 2. Selection marquee is completely inside an element polygon
        let center = sel_rect.center();
        for poly in &geom.polygons {
            if point_in_screen_poly(center, poly) {
                return true;
            }
        }
        // 3. Any element edge intersects any of the 4 borders of sel_rect
        let rect_edges = [
            (sel_rect.left_top(), sel_rect.right_top()),
            (sel_rect.right_top(), sel_rect.right_bottom()),
            (sel_rect.right_bottom(), sel_rect.left_bottom()),
            (sel_rect.left_bottom(), sel_rect.left_top()),
        ];
        for &(e1, e2) in &geom.edges {
            for &(r1, r2) in &rect_edges {
                if segments_intersect(e1, e2, r1, r2) {
                    return true;
                }
            }
        }
        false
    }
}

/// Projects any element (in 3D, Elevation, Section, or Plan) into screen geometry for marquee selection.
pub fn get_element_projected_screen_pts<FScreen, F3D>(
    id: plinth_model::ElementId,
    el: &plinth_model::Element,
    active_view: &plinth_model::ViewDef,
    doc: &plinth_model::Document,
    to_screen: &FScreen,
    project_3d: &F3D,
    cut_z: f64,
) -> Option<ProjectedElementGeometry>
where
    FScreen: Fn(plinth_geom::Vec2) -> Pos2,
    F3D: Fn(plinth_geom::Vec3) -> Pos2,
{
    if active_view.kind.is_3d() {
        // 3D View Projection
        let faces = doc.element_faces(id);
        if !faces.is_empty() {
            let mut vertices = Vec::new();
            let mut edges = Vec::new();
            let mut polygons = Vec::new();
            for f in &faces {
                let poly: Vec<Pos2> = f.pts.iter().map(|&p| project_3d(p)).collect();
                let n = poly.len();
                for i in 0..n {
                    edges.push((poly[i], poly[(i + 1) % n]));
                }
                for &pt in &poly {
                    vertices.push(pt);
                }
                if n >= 3 {
                    polygons.push(poly);
                }
            }
            Some(ProjectedElementGeometry {
                vertices,
                edges,
                polygons,
            })
        } else if let plinth_model::Element::DetailLine(dl) = el {
            let p1 = project_3d(plinth_geom::Vec3::new(dl.start.x, dl.start.y, 0.0));
            let p2 = project_3d(plinth_geom::Vec3::new(dl.end.x, dl.end.y, 0.0));
            Some(ProjectedElementGeometry::from_segment(p1, p2))
        } else {
            None
        }
    } else if active_view.kind.is_elevation() {
        // Elevation Projection (u horizontal, z vertical)
        let project_elev = |p: plinth_geom::Vec3| -> plinth_geom::Vec2 {
            match active_view.kind {
                plinth_model::ViewKind::ElevationSouth => plinth_geom::vec2(p.x, p.z),
                plinth_model::ViewKind::ElevationNorth => plinth_geom::vec2(-p.x, p.z),
                plinth_model::ViewKind::ElevationEast => plinth_geom::vec2(-p.y, p.z),
                plinth_model::ViewKind::ElevationWest => plinth_geom::vec2(p.y, p.z),
                _ => plinth_geom::vec2(p.x, p.z),
            }
        };

        let bounds: Option<(f64, f64, f64, f64)> = match el {
            plinth_model::Element::Wall(w) => {
                let (z0, z1) = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|l| doc.level_elevation(l)),
                );
                let u0 = project_elev(w.start.extend(0.0)).x;
                let u1 = project_elev(w.end.extend(0.0)).x;
                Some((u0.min(u1), u0.max(u1), z0, z1))
            }
            plinth_model::Element::CurtainWall(cw) => {
                let (z0, z1) = cw.z_bounds(
                    doc.level_elevation(cw.base_level),
                    cw.top_level.map(|l| doc.level_elevation(l)),
                );
                let u0 = project_elev(cw.start.extend(0.0)).x;
                let u1 = project_elev(cw.end.extend(0.0)).x;
                Some((u0.min(u1), u0.max(u1), z0, z1))
            }
            plinth_model::Element::Column(c) => {
                let (z0, z1) = c.z_bounds(
                    doc.level_elevation(c.base_level),
                    c.top_level.map(|l| doc.level_elevation(l)),
                );
                let us: Vec<f64> = c.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                let u_min = us.iter().copied().fold(f64::INFINITY, f64::min);
                let u_max = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((u_min, u_max, z0, z1))
            }
            plinth_model::Element::Beam(b) => {
                let (z0, z1) = b.z_bounds(
                    doc.level_elevation(b.level),
                    b.top_level.map(|l| doc.level_elevation(l)),
                );
                let u0 = project_elev(b.start.extend(0.0)).x;
                let u1 = project_elev(b.end.extend(0.0)).x;
                Some((u0.min(u1), u0.max(u1), z0, z1))
            }
            plinth_model::Element::Slab(s) => {
                let (z_bot, z_top) = s.z_bounds(
                    doc.level_elevation(s.level),
                    s.top_level.map(|l| doc.level_elevation(l)),
                );
                let us: Vec<f64> = s.boundary.iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                let u_min = us.iter().copied().fold(f64::INFINITY, f64::min);
                let u_max = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((u_min, u_max, z_bot, z_top))
            }
            plinth_model::Element::Footing(f) => {
                let (z0, z1) = f.z_bounds(
                    doc.level_elevation(f.base_level),
                    f.top_level.map(|l| doc.level_elevation(l)),
                );
                let us: Vec<f64> = f.footprint().iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                let u_min = us.iter().copied().fold(f64::INFINITY, f64::min);
                let u_max = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((u_min, u_max, z0, z1))
            }
            plinth_model::Element::Roof(r) => {
                let lz = doc.level_elevation(r.level);
                let us: Vec<f64> = r.boundary.iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                let u_min = us.iter().copied().fold(f64::INFINITY, f64::min);
                let u_max = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((u_min, u_max, lz + r.offset, lz + r.offset + 2500.0))
            }
            plinth_model::Element::Ceiling(c) => {
                let lz = doc.level_elevation(c.level);
                let us: Vec<f64> = c.boundary.iter().map(|&p| project_elev(p.extend(0.0)).x).collect();
                let u_min = us.iter().copied().fold(f64::INFINITY, f64::min);
                let u_max = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((u_min, u_max, lz + c.height_above_level, lz + c.height_above_level + c.thickness))
            }
            plinth_model::Element::Door(door) => {
                if let Some(plinth_model::Element::Wall(hw)) = doc.elements.get(&door.host_wall) {
                    let center_3d = hw.start + hw.direction() * door.offset_along_wall;
                    let u_c = project_elev(center_3d.extend(0.0)).x;
                    let hw_z0 = hw.z_bounds(doc.level_elevation(hw.base_level), hw.top_level.map(|l| doc.level_elevation(l))).0;
                    let d_z0 = door.base_level.map(|l| doc.level_elevation(l)).unwrap_or(hw_z0);
                    let (z0, z1) = door.z_bounds(d_z0, door.top_level.map(|l| doc.level_elevation(l)));
                    Some((u_c - door.width * 0.5, u_c + door.width * 0.5, z0, z1))
                } else {
                    None
                }
            }
            plinth_model::Element::Window(win) => {
                if let Some(plinth_model::Element::Wall(hw)) = doc.elements.get(&win.host_wall) {
                    let center_3d = hw.start + hw.direction() * win.offset_along_wall;
                    let u_c = project_elev(center_3d.extend(0.0)).x;
                    let hw_z0 = hw.z_bounds(doc.level_elevation(hw.base_level), hw.top_level.map(|l| doc.level_elevation(l))).0;
                    let w_z0 = win.base_level.map(|l| doc.level_elevation(l)).unwrap_or(hw_z0);
                    let (z0, z1) = win.z_bounds(w_z0, win.top_level.map(|l| doc.level_elevation(l)));
                    Some((u_c - win.width * 0.5, u_c + win.width * 0.5, z0, z1))
                } else {
                    None
                }
            }
            _ => None,
        };

        bounds.map(|(u0, u1, z0, z1)| {
            ProjectedElementGeometry::from_box(
                to_screen(plinth_geom::vec2(u0, z0)),
                to_screen(plinth_geom::vec2(u1, z0)),
                to_screen(plinth_geom::vec2(u1, z1)),
                to_screen(plinth_geom::vec2(u0, z1)),
            )
        })
    } else if active_view.kind.is_section() {
        // Section Projection (t along cut line, z vertical)
        let (s1, s2) = active_view.section_line.unwrap_or((plinth_geom::Vec2::ZERO, plinth_geom::vec2(10000.0, 0.0)));
        let line_vec = s2 - s1;
        let u_dir = if line_vec.length() > 1.0 { line_vec.normalized() } else { plinth_geom::vec2(1.0, 0.0) };

        let bounds: Option<(f64, f64, f64, f64)> = match el {
            plinth_model::Element::Wall(w) => {
                let (z0, z1) = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|l| doc.level_elevation(l)),
                );
                let t0 = (w.start - s1).dot(u_dir);
                let t1 = (w.end - s1).dot(u_dir);
                Some((t0.min(t1), t0.max(t1), z0, z1))
            }
            plinth_model::Element::CurtainWall(cw) => {
                let (z0, z1) = cw.z_bounds(
                    doc.level_elevation(cw.base_level),
                    cw.top_level.map(|l| doc.level_elevation(l)),
                );
                let t0 = (cw.start - s1).dot(u_dir);
                let t1 = (cw.end - s1).dot(u_dir);
                Some((t0.min(t1), t0.max(t1), z0, z1))
            }
            plinth_model::Element::Column(c) => {
                let (z0, z1) = c.z_bounds(
                    doc.level_elevation(c.base_level),
                    c.top_level.map(|l| doc.level_elevation(l)),
                );
                let ts: Vec<f64> = c.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                let t_min = ts.iter().copied().fold(f64::INFINITY, f64::min);
                let t_max = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((t_min, t_max, z0, z1))
            }
            plinth_model::Element::Beam(b) => {
                let (z0, z1) = b.z_bounds(
                    doc.level_elevation(b.level),
                    b.top_level.map(|l| doc.level_elevation(l)),
                );
                let t0 = (b.start - s1).dot(u_dir);
                let t1 = (b.end - s1).dot(u_dir);
                Some((t0.min(t1), t0.max(t1), z0, z1))
            }
            plinth_model::Element::Slab(s) => {
                let (z_bot, z_top) = s.z_bounds(
                    doc.level_elevation(s.level),
                    s.top_level.map(|l| doc.level_elevation(l)),
                );
                let ts: Vec<f64> = s.boundary.iter().map(|&p| (p - s1).dot(u_dir)).collect();
                let t_min = ts.iter().copied().fold(f64::INFINITY, f64::min);
                let t_max = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((t_min, t_max, z_bot, z_top))
            }
            plinth_model::Element::Footing(f) => {
                let (z0, z1) = f.z_bounds(
                    doc.level_elevation(f.base_level),
                    f.top_level.map(|l| doc.level_elevation(l)),
                );
                let ts: Vec<f64> = f.footprint().iter().map(|&p| (p - s1).dot(u_dir)).collect();
                let t_min = ts.iter().copied().fold(f64::INFINITY, f64::min);
                let t_max = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some((t_min, t_max, z0, z1))
            }
            _ => None,
        };

        bounds.map(|(t0, t1, z0, z1)| {
            ProjectedElementGeometry::from_box(
                to_screen(plinth_geom::vec2(t0, z0)),
                to_screen(plinth_geom::vec2(t1, z0)),
                to_screen(plinth_geom::vec2(t1, z1)),
                to_screen(plinth_geom::vec2(t0, z1)),
            )
        })
    } else {
        // 2D Plan Views (Floor Plans & Reflected Ceiling Plans)
        match el {
            plinth_model::Element::Wall(w) => {
                let (base_z, top_z) = w.z_bounds(
                    doc.level_elevation(w.base_level),
                    w.top_level.map(|l| doc.level_elevation(l)),
                );
                if cut_z >= base_z && cut_z <= top_z + 100.0 {
                    let pts: Vec<Pos2> = w.footprint().iter().map(|&p| to_screen(p)).collect();
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::CurtainWall(cw) => {
                let (base_z, top_z) = cw.z_bounds(
                    doc.level_elevation(cw.base_level),
                    cw.top_level.map(|l| doc.level_elevation(l)),
                );
                if cut_z >= base_z && cut_z <= top_z + 100.0 {
                    let pts: Vec<Pos2> = cw.footprint().iter().map(|&p| to_screen(p)).collect();
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::Column(c) => {
                let (base_z, top_z) = c.z_bounds(
                    doc.level_elevation(c.base_level),
                    c.top_level.map(|l| doc.level_elevation(l)),
                );
                if cut_z >= base_z && cut_z <= top_z + 100.0 {
                    let pts: Vec<Pos2> = c.footprint().iter().map(|&p| to_screen(p)).collect();
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::Beam(b) => {
                let (z0, z1) = b.z_bounds(
                    doc.level_elevation(b.level),
                    b.top_level.map(|l| doc.level_elevation(l)),
                );
                if cut_z >= z0 - 400.0 && cut_z <= z1 + 400.0 {
                    Some(ProjectedElementGeometry::from_segment(
                        to_screen(b.start),
                        to_screen(b.end),
                    ))
                } else {
                    None
                }
            }
            plinth_model::Element::Door(door) => {
                if let Some(plinth_model::Element::Wall(hw)) = doc.elements.get(&door.host_wall) {
                    let center = hw.start + hw.direction() * door.offset_along_wall;
                    let norm = hw.normal();
                    let dir = hw.direction();
                    let half_w = door.width * 0.5;
                    let half_t = hw.thickness * 0.5;
                    let pts = vec![
                        to_screen(center - dir * half_w - norm * half_t),
                        to_screen(center + dir * half_w - norm * half_t),
                        to_screen(center + dir * half_w + norm * half_t),
                        to_screen(center - dir * half_w + norm * half_t),
                    ];
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::Window(win) => {
                if let Some(plinth_model::Element::Wall(hw)) = doc.elements.get(&win.host_wall) {
                    let center = hw.start + hw.direction() * win.offset_along_wall;
                    let norm = hw.normal();
                    let dir = hw.direction();
                    let half_w = win.width * 0.5;
                    let half_t = hw.thickness * 0.5;
                    let pts = vec![
                        to_screen(center - dir * half_w - norm * half_t),
                        to_screen(center + dir * half_w - norm * half_t),
                        to_screen(center + dir * half_w + norm * half_t),
                        to_screen(center - dir * half_w + norm * half_t),
                    ];
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::Slab(s) => {
                let (_z_bot, z_top) = s.z_bounds(
                    doc.level_elevation(s.level),
                    s.top_level.map(|l| doc.level_elevation(l)),
                );
                // In Plan view, upper storeys' deck/ceiling slabs above cut plane never match
                if z_top <= cut_z + 50.0 {
                    let pts: Vec<Pos2> = s.boundary.iter().map(|&p| to_screen(p)).collect();
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::Roof(r) => {
                let pts: Vec<Pos2> = r.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Ceiling(c) => {
                let pts: Vec<Pos2> = c.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Stair(s) => {
                let pts: Vec<Pos2> = s.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Ramp(r) => {
                let pts: Vec<Pos2> = r.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Footing(f) => {
                let pts: Vec<Pos2> = f.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::DetailLine(dl) => {
                Some(ProjectedElementGeometry::from_segment(
                    to_screen(dl.start),
                    to_screen(dl.end),
                ))
            }
            plinth_model::Element::Dimension(d) => {
                let diff = d.p2 - d.p1;
                let len = diff.length();
                if len > 1e-3 {
                    let dir = diff.normalized();
                    let norm = plinth_geom::vec2(-dir.y, dir.x);
                    let p1d = d.p1 + norm * d.offset;
                    let p2d = d.p2 + norm * d.offset;
                    let pts = vec![
                        to_screen(d.p1),
                        to_screen(p1d),
                        to_screen(p2d),
                        to_screen(d.p2),
                    ];
                    Some(ProjectedElementGeometry::from_polygon(pts))
                } else {
                    None
                }
            }
            plinth_model::Element::TextNote(t) => {
                let sp = to_screen(t.pos);
                let hw = 30.0;
                let hh = 12.0;
                let pts = vec![
                    Pos2::new(sp.x - hw, sp.y - hh),
                    Pos2::new(sp.x + hw, sp.y - hh),
                    Pos2::new(sp.x + hw, sp.y + hh),
                    Pos2::new(sp.x - hw, sp.y + hh),
                ];
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Leader(l) => {
                let pts = vec![
                    to_screen(l.target_pt),
                    to_screen(l.shoulder_pt),
                    to_screen(l.text_pt),
                ];
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::BuildingPad(bp) => {
                let pts: Vec<Pos2> = bp.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Toposolid(ts) => {
                let pts: Vec<Pos2> = ts.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::PropertyLine(pl) => {
                let pts: Vec<Pos2> = pl.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Road(rd) => {
                let pts: Vec<Pos2> = rd.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Sidewalk(sw) => {
                let pts: Vec<Pos2> = sw.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::Pathwalk(pw) => {
                let pts: Vec<Pos2> = pw.footprint().iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::FreeformSurface(s) => {
                let pts: Vec<Pos2> = s.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            plinth_model::Element::FreeformSolid(f) => {
                let pts: Vec<Pos2> = f.boundary.iter().map(|&p| to_screen(p)).collect();
                Some(ProjectedElementGeometry::from_polygon(pts))
            }
            _ => None,
        }
    }
}

fn is_point_in_quad(p: Pos2, pts: &[Pos2]) -> bool {
    if pts.len() < 3 {
        return false;
    }
    let mut sign = None;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if cross.abs() > 1e-4 {
            let s = cross > 0.0;
            if let Some(prev) = sign {
                if prev != s {
                    return false;
                }
            } else {
                sign = Some(s);
            }
        }
    }
    true
}

pub fn raycast_closest_face(
    doc: &Document,
    ray_origin: plinth_geom::Vec3,
    ray_dir: plinth_geom::Vec3,
) -> Option<(plinth_model::ElementId, plinth_geom::Face, plinth_geom::Vec3)> {
    let mut best: Option<(plinth_model::ElementId, plinth_geom::Face, plinth_geom::Vec3)> = None;
    let mut min_t = f64::INFINITY;

    for (&id, _el) in &doc.elements {
        for face in doc.element_faces(id) {
            let denom = face.normal.dot(ray_dir);
            if denom >= -1e-5 {
                continue; // Culled: facing away from ray
            }
            if face.pts.len() < 3 {
                continue;
            }
            let t = face.normal.dot(face.pts[0] - ray_origin) / denom;
            if t <= 0.0 || t >= min_t {
                continue;
            }

            let p_hit = ray_origin + ray_dir * t;
            let nx = face.normal.x.abs();
            let ny = face.normal.y.abs();
            let nz = face.normal.z.abs();

            let (p_2d, poly_2d) = if nz >= nx && nz >= ny {
                (
                    plinth_geom::vec2(p_hit.x, p_hit.y),
                    face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.y)).collect::<Vec<_>>(),
                )
            } else if nx >= ny {
                (
                    plinth_geom::vec2(p_hit.y, p_hit.z),
                    face.pts.iter().map(|p| plinth_geom::vec2(p.y, p.z)).collect::<Vec<_>>(),
                )
            } else {
                (
                    plinth_geom::vec2(p_hit.x, p_hit.z),
                    face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.z)).collect::<Vec<_>>(),
                )
            };

            let is_hit = plinth_geom::polygon::point_in_polygon(p_2d, &poly_2d)
                || plinth_geom::polygon::distance_to_polyline(p_2d, &poly_2d, true) <= 25.0;

            if is_hit {
                min_t = t;
                best = Some((id, face, p_hit));
            }
        }
    }
    best
}

/// Floating Viewport Header Badge with View Switcher Dropdown in the upper-left of each viewport.
fn render_viewport_header_badge(
    painter: &egui::Painter,
    ui: &Ui,
    rect: Rect,
    pane_idx: usize,
    view: &plinth_model::ViewDef,
    is_active: bool,
    total_panes: usize,
    doc: &mut plinth_model::Document,
    canvas_state: &mut CanvasState,
    theme: &AppTheme,
) -> bool {
    let icon = match view.kind {
        plinth_model::ViewKind::FloorPlan => "📐",
        plinth_model::ViewKind::CeilingPlan => "💡",
        plinth_model::ViewKind::ThreeD => "🧊",
        plinth_model::ViewKind::ElevationNorth
        | plinth_model::ViewKind::ElevationSouth
        | plinth_model::ViewKind::ElevationEast
        | plinth_model::ViewKind::ElevationWest => "🏛",
        plinth_model::ViewKind::Section => "✂",
        plinth_model::ViewKind::Detail => "🔍",
    };

    let scale_str = if view.scale > 0 && !view.kind.is_3d() {
        format!(" [1:{}]", view.scale)
    } else {
        String::new()
    };
    let title_text = format!("{} {}{} ▾", icon, view.name, scale_str);
    let font_id = egui::FontId::proportional(11.0);
    let text_w = 16.0 + title_text.len() as f32 * 6.0;
    let badge_w = text_w.clamp(100.0, 240.0);
    let badge_h = 24.0f32;
    let badge_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 8.0),
        Vec2::new(badge_w, badge_h),
    );

    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let is_hover = badge_rect.contains(mouse_pos);
    let mut consumed = is_hover;

    let bg = if is_active && total_panes > 1 {
        theme.panel_bg
    } else if is_hover {
        theme.hover_highlight
    } else {
        theme.panel_bg.gamma_multiply(0.85)
    };

    painter.rect_filled(badge_rect, 4.0, bg);
    painter.rect_stroke(
        badge_rect,
        4.0,
        Stroke::new(
            if is_active && total_panes > 1 { 1.5 } else { 1.0 },
            if is_active && total_panes > 1 { theme.accent } else { theme.border_color },
        ),
        egui::StrokeKind::Inside,
    );

    let text_color = if is_active && total_panes > 1 {
        theme.accent
    } else {
        theme.text_primary
    };

    painter.text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        &title_text,
        font_id,
        text_color,
    );

    if is_hover && clicked {
        canvas_state.panes[pane_idx].view_picker_open = !canvas_state.panes[pane_idx].view_picker_open;
    }

    if canvas_state.panes[pane_idx].view_picker_open {
        let popup_w = 210.0f32;
        let view_count = doc.views.len();
        let popup_h = (view_count as f32 * 24.0 + 8.0).clamp(60.0, 260.0);
        let popup_rect = Rect::from_min_size(
            Pos2::new(badge_rect.min.x, badge_rect.max.y + 4.0),
            Vec2::new(popup_w, popup_h),
        );
        if popup_rect.contains(mouse_pos) {
            consumed = true;
        }

        let shadow = popup_rect.translate(Vec2::new(2.0, 2.0));
        painter.rect_filled(shadow, 6.0, Color32::from_black_alpha(50));
        painter.rect_filled(popup_rect, 6.0, theme.panel_bg);
        painter.rect_stroke(popup_rect, 6.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);

        let mut row_y = popup_rect.min.y + 4.0;
        let view_items: Vec<_> = doc.views.values().cloned().collect();
        for v in view_items {
            let row_rect = Rect::from_min_size(Pos2::new(popup_rect.min.x + 4.0, row_y), Vec2::new(popup_w - 8.0, 22.0));
            let r_hover = row_rect.contains(mouse_pos);
            let is_curr = v.id == view.id;

            if is_curr || r_hover {
                painter.rect_filled(row_rect, 3.0, theme.hover_highlight);
            }

            let v_icon = match v.kind {
                plinth_model::ViewKind::FloorPlan => "📐",
                plinth_model::ViewKind::CeilingPlan => "💡",
                plinth_model::ViewKind::ThreeD => "🧊",
                plinth_model::ViewKind::ElevationNorth
                | plinth_model::ViewKind::ElevationSouth
                | plinth_model::ViewKind::ElevationEast
                | plinth_model::ViewKind::ElevationWest => "🏛",
                plinth_model::ViewKind::Section => "✂",
                plinth_model::ViewKind::Detail => "🔍",
            };
            painter.text(
                Pos2::new(row_rect.min.x + 6.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{} {}", v_icon, v.name),
                egui::FontId::proportional(10.5),
                if is_curr { theme.accent } else { theme.text_primary },
            );

            if r_hover && clicked {
                canvas_state.panes[pane_idx].view_id = Some(v.id);
                if is_active {
                    doc.active_view_id = v.id;
                }
                canvas_state.panes[pane_idx].view_picker_open = false;
            }

            row_y += 24.0;
        }

        if clicked && !badge_rect.contains(mouse_pos) && !popup_rect.contains(mouse_pos) {
            canvas_state.panes[pane_idx].view_picker_open = false;
        }
    }

    consumed
}

/// Floating Minimalistic Visual Style Selector in the lower-right corner of each viewport.
fn render_visual_style_selector(
    painter: &egui::Painter,
    ui: &Ui,
    rect: Rect,
    _pane_idx: usize,
    visual_style: &mut VisualStyle,
    style_menu_open: &mut bool,
    theme: &AppTheme,
) -> bool {
    let size = 26.0f32;
    let btn_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - size - 8.0, rect.max.y - size - 8.0),
        egui::vec2(size, size),
    );

    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let btn_hover = btn_rect.contains(mouse_pos);
    let mut consumed = btn_hover;

    let bg = if btn_hover || *style_menu_open {
        theme.hover_highlight
    } else {
        theme.panel_bg
    };
    painter.rect_filled(btn_rect, 5.0, bg);
    painter.rect_stroke(
        btn_rect,
        5.0,
        Stroke::new(
            1.2,
            if btn_hover || *style_menu_open {
                theme.accent
            } else {
                theme.border_color
            },
        ),
        egui::StrokeKind::Inside,
    );

    // Minimalistic single icon: ☀️, 📐, 🎨, or 🕸️
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        visual_style.icon(),
        egui::FontId::proportional(13.0),
        theme.text_primary,
    );

    if btn_hover && clicked {
        *style_menu_open = !*style_menu_open;
    }

    if *style_menu_open {
        let popup_w = 145.0f32;
        let popup_h = 120.0f32;
        let popup_rect = Rect::from_min_size(
            Pos2::new(btn_rect.max.x - popup_w, btn_rect.min.y - popup_h - 4.0),
            egui::vec2(popup_w, popup_h),
        );
        if popup_rect.contains(mouse_pos) {
            consumed = true;
        }

        // Popup background with shadow
        let shadow = popup_rect.translate(Vec2::new(2.0, 2.0));
        painter.rect_filled(shadow, 6.0, Color32::from_black_alpha(50));
        painter.rect_filled(popup_rect, 6.0, theme.panel_bg);
        painter.rect_stroke(popup_rect, 6.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);

        let styles = [
            (VisualStyle::Shaded, "☀️ Shaded"),
            (VisualStyle::HiddenLine, "📐 Hidden Line"),
            (VisualStyle::ConsistentColors, "🎨 Consistent Colors"),
            (VisualStyle::Wireframe, "🕸️ Wireframe"),
        ];

        for (k, (s, s_name)) in styles.into_iter().enumerate() {
            let row_rect = Rect::from_min_size(
                Pos2::new(popup_rect.min.x + 3.0, popup_rect.min.y + 3.0 + k as f32 * 28.0),
                egui::vec2(popup_w - 6.0, 26.0),
            );
            let row_hover = row_rect.contains(mouse_pos);
            let is_sel = *visual_style == s;

            if is_sel || row_hover {
                painter.rect_filled(row_rect, 4.0, theme.hover_highlight);
            }

            painter.text(
                Pos2::new(row_rect.min.x + 8.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                s_name,
                egui::FontId::proportional(11.0),
                if is_sel { theme.accent } else { theme.text_primary },
            );

            if is_sel {
                painter.text(
                    Pos2::new(row_rect.max.x - 10.0, row_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "✓",
                    egui::FontId::proportional(11.0),
                    theme.accent,
                );
            }

            if row_hover && clicked {
                *visual_style = s;
                *style_menu_open = false;
            }
        }

        if clicked && !btn_rect.contains(mouse_pos) && !popup_rect.contains(mouse_pos) {
            *style_menu_open = false;
        }
    }

    consumed
}

/// Renders interactive viewport layout split dividers and draggable handle grips.
fn render_split_viewport_overlays(
    painter: &egui::Painter,
    ui: &Ui,
    rect: Rect,
    canvas_state: &mut CanvasState,
    theme: &AppTheme,
    _doc: &Document,
) {
    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    let is_lmb_down = ui.input(|i| i.pointer.button_down(PointerButton::Primary));

    let split_x = canvas_state.split_x.clamp(0.15, 0.85);
    let split_y = canvas_state.split_y.clamp(0.15, 0.85);

    let x_s = rect.min.x + rect.width() * split_x;
    let y_s = rect.min.y + rect.height() * split_y;

    let sep_color = theme.border_color;
    let grip_bg = theme.panel_bg;
    let grip_stroke = Stroke::new(1.0, theme.accent);
    let dot_color = theme.accent;

    match canvas_state.layout {
        ViewportLayout::Single => {}

        ViewportLayout::SplitHorizontal => {
            // Horizontal divider bar across full canvas
            let sep_bar = Rect::from_min_max(Pos2::new(rect.min.x, y_s - 3.0), Pos2::new(rect.max.x, y_s + 3.0));
            painter.rect_filled(sep_bar, 0.0, sep_color);

            // Handle Grip Pill
            let grip = Rect::from_center_size(Pos2::new(rect.center().x, y_s), Vec2::new(56.0, 10.0));
            painter.rect_filled(grip, 5.0, grip_bg);
            painter.rect_stroke(grip, 5.0, grip_stroke, egui::StrokeKind::Inside);
            for d in [-14.0, 0.0, 14.0] {
                painter.circle_filled(Pos2::new(rect.center().x + d, y_s), 1.8, dot_color);
            }

            let is_hover = sep_bar.contains(mouse_pos) || (mouse_pos.y - y_s).abs() < 8.0;
            if is_hover || canvas_state.is_dragging_split == Some(SplitAxis::Horizontal) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Horizontal);
                    canvas_state.split_y = ((mouse_pos.y - rect.min.y) / rect.height()).clamp(0.15, 0.85);
                }
            }
            if !is_lmb_down {
                canvas_state.is_dragging_split = None;
            }
        }

        ViewportLayout::SplitVertical => {
            // Vertical divider bar across full canvas
            let sep_bar = Rect::from_min_max(Pos2::new(x_s - 3.0, rect.min.y), Pos2::new(x_s + 3.0, rect.max.y));
            painter.rect_filled(sep_bar, 0.0, sep_color);

            // Handle Grip Pill
            let grip = Rect::from_center_size(Pos2::new(x_s, rect.center().y), Vec2::new(10.0, 56.0));
            painter.rect_filled(grip, 5.0, grip_bg);
            painter.rect_stroke(grip, 5.0, grip_stroke, egui::StrokeKind::Inside);
            for d in [-14.0, 0.0, 14.0] {
                painter.circle_filled(Pos2::new(x_s, rect.center().y + d), 1.8, dot_color);
            }

            let is_hover = sep_bar.contains(mouse_pos) || (mouse_pos.x - x_s).abs() < 8.0;
            if is_hover || canvas_state.is_dragging_split == Some(SplitAxis::Vertical) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Vertical);
                    canvas_state.split_x = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.15, 0.85);
                }
            }
            if !is_lmb_down {
                canvas_state.is_dragging_split = None;
            }
        }

        ViewportLayout::SplitTriple => {
            // Left large pane (width split_x), Right panes split at split_y
            let vert_bar = Rect::from_min_max(Pos2::new(x_s - 3.0, rect.min.y), Pos2::new(x_s + 3.0, rect.max.y));
            painter.rect_filled(vert_bar, 0.0, sep_color);

            let horiz_bar = Rect::from_min_max(Pos2::new(x_s, y_s - 3.0), Pos2::new(rect.max.x, y_s + 3.0));
            painter.rect_filled(horiz_bar, 0.0, sep_color);

            // Vertical grip
            let grip_v = Rect::from_center_size(Pos2::new(x_s, rect.center().y), Vec2::new(10.0, 48.0));
            painter.rect_filled(grip_v, 5.0, grip_bg);
            painter.rect_stroke(grip_v, 5.0, grip_stroke, egui::StrokeKind::Inside);
            for d in [-12.0, 0.0, 12.0] {
                painter.circle_filled(Pos2::new(x_s, rect.center().y + d), 1.8, dot_color);
            }

            // Horizontal grip
            let right_mid_x = (x_s + rect.max.x) * 0.5;
            let grip_h = Rect::from_center_size(Pos2::new(right_mid_x, y_s), Vec2::new(48.0, 10.0));
            painter.rect_filled(grip_h, 5.0, grip_bg);
            painter.rect_stroke(grip_h, 5.0, grip_stroke, egui::StrokeKind::Inside);
            for d in [-12.0, 0.0, 12.0] {
                painter.circle_filled(Pos2::new(right_mid_x + d, y_s), 1.8, dot_color);
            }

            let is_v_hover = vert_bar.contains(mouse_pos) || (mouse_pos.x - x_s).abs() < 8.0;
            let is_h_hover = horiz_bar.contains(mouse_pos) || (mouse_pos.x >= x_s && (mouse_pos.y - y_s).abs() < 8.0);

            if is_v_hover || canvas_state.is_dragging_split == Some(SplitAxis::Vertical) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Vertical);
                    canvas_state.split_x = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.15, 0.85);
                }
            } else if is_h_hover || canvas_state.is_dragging_split == Some(SplitAxis::Horizontal) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Horizontal);
                    canvas_state.split_y = ((mouse_pos.y - rect.min.y) / rect.height()).clamp(0.15, 0.85);
                }
            }
            if !is_lmb_down {
                canvas_state.is_dragging_split = None;
            }
        }

        ViewportLayout::SplitQuad => {
            // 4 Quadrants: Both vertical and horizontal bars intersecting at (x_s, y_s)
            let vert_bar = Rect::from_min_max(Pos2::new(x_s - 3.0, rect.min.y), Pos2::new(x_s + 3.0, rect.max.y));
            painter.rect_filled(vert_bar, 0.0, sep_color);

            let horiz_bar = Rect::from_min_max(Pos2::new(rect.min.x, y_s - 3.0), Pos2::new(rect.max.x, y_s + 3.0));
            painter.rect_filled(horiz_bar, 0.0, sep_color);

            // Center 4-way Grip Diamond
            let center_grip = Rect::from_center_size(Pos2::new(x_s, y_s), Vec2::new(22.0, 22.0));
            painter.rect_filled(center_grip, 5.0, grip_bg);
            painter.rect_stroke(center_grip, 5.0, grip_stroke, egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(x_s - 4.5, y_s), 1.6, dot_color);
            painter.circle_filled(Pos2::new(x_s + 4.5, y_s), 1.6, dot_color);
            painter.circle_filled(Pos2::new(x_s, y_s - 4.5), 1.6, dot_color);
            painter.circle_filled(Pos2::new(x_s, y_s + 4.5), 1.6, dot_color);

            let is_c_hover = center_grip.contains(mouse_pos) || (mouse_pos.distance(Pos2::new(x_s, y_s)) < 16.0);
            let is_v_hover = vert_bar.contains(mouse_pos) || (mouse_pos.x - x_s).abs() < 8.0;
            let is_h_hover = horiz_bar.contains(mouse_pos) || (mouse_pos.y - y_s).abs() < 8.0;

            if is_c_hover || canvas_state.is_dragging_split == Some(SplitAxis::Center) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Center);
                    canvas_state.split_x = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.15, 0.85);
                    canvas_state.split_y = ((mouse_pos.y - rect.min.y) / rect.height()).clamp(0.15, 0.85);
                }
            } else if is_v_hover || canvas_state.is_dragging_split == Some(SplitAxis::Vertical) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Vertical);
                    canvas_state.split_x = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.15, 0.85);
                }
            } else if is_h_hover || canvas_state.is_dragging_split == Some(SplitAxis::Horizontal) {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
                if is_lmb_down {
                    canvas_state.is_dragging_split = Some(SplitAxis::Horizontal);
                    canvas_state.split_y = ((mouse_pos.y - rect.min.y) / rect.height()).clamp(0.15, 0.85);
                }
            }
            if !is_lmb_down {
                canvas_state.is_dragging_split = None;
            }
        }
    }
}

/// Renders the multi-sheet layout drawing desk with paper margins, BIM title blocks,
/// placed viewports, and automatic architectural drawing title bubbles.
fn render_sheet_layout_canvas(
    ui: &mut Ui,
    rect: Rect,
    response: &egui::Response,
    painter: &egui::Painter,
    doc: &mut Document,
    active_sheet_id: plinth_model::SheetId,
    canvas_state: &mut CanvasState,
    theme: &AppTheme,
) {
    // Drafting desk background
    let desk_bg = if theme.mode == crate::theme::ThemeMode::Dark {
        Color32::from_rgb(22, 25, 32)
    } else {
        Color32::from_rgb(220, 224, 230)
    };
    painter.rect_filled(rect, 0.0, desk_bg);

    // Auto-fit active sheet if sheet_zoom not initialized
    let (active_pw, active_ph) = doc.sheets.get(&active_sheet_id)
        .map(|s| s.paper_size.dimensions_mm())
        .unwrap_or((841.0, 594.0));

    if canvas_state.sheet_zoom <= 0.01 {
        let fit_w = (rect.width() - 80.0).max(100.0) / active_pw as f32;
        let fit_h = (rect.height() - 80.0).max(100.0) / active_ph as f32;
        canvas_state.sheet_zoom = fit_w.min(fit_h).clamp(0.1, 5.0);
        canvas_state.sheet_pan = Vec2::ZERO;
    }

    // Pan / Zoom for Sheet Desk (independent from 3D/2D model zoom)
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.001 {
            let pointer_screen = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
            let factor = ((scroll * 0.0025).exp()).clamp(0.88, 1.14);
            let new_zoom = (canvas_state.sheet_zoom * factor).clamp(0.05, 10.0);
            let center = rect.center();
            let screen_offset = pointer_screen - center - canvas_state.sheet_pan;
            canvas_state.sheet_pan -= screen_offset * (factor - 1.0);
            canvas_state.sheet_zoom = new_zoom;
        }
        if ui.input(|i| i.pointer.button_down(PointerButton::Middle)) {
            canvas_state.sheet_pan += response.drag_delta();
        }
    }

    let center = rect.center();
    let pan = canvas_state.sheet_pan;
    let zoom = canvas_state.sheet_zoom;

    // Grid arrangement of all sheets in project, with active sheet centered at pan (0, 0)
    let sheet_ids: Vec<_> = doc.sheets.keys().copied().collect();
    let active_idx = sheet_ids.iter().position(|id| *id == active_sheet_id).unwrap_or(0);
    let active_col = active_idx % 2;
    let active_row = active_idx / 2;

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_pressed = response.hovered() && ui.input(|i| i.pointer.primary_pressed());
    let primary_down = ui.input(|i| i.pointer.primary_down());
    let pointer_delta = ui.input(|i| i.pointer.delta());

    // 0. Interactive Viewport Hit-Testing & Free Drag/Resize on Sheet Layout Desk
    if primary_pressed {
        if let Some(mp) = pointer_pos {
            let mut hit_action: Option<(plinth_model::ViewportId, SheetVpDragMode, plinth_model::SheetId)> = None;
            let mut clicked_sheet_id = None;

            for (idx, s_id) in sheet_ids.iter().enumerate() {
                if let Some(sheet_ref) = doc.sheets.get(s_id) {
                    let (pw, ph) = sheet_ref.paper_size.dimensions_mm();
                    let col = idx % 2;
                    let row = idx / 2;
                    let col_offset = (col as f64 - active_col as f64) * (pw + 140.0);
                    let row_offset = (row as f64 - active_row as f64) * (ph + 140.0);
                    let paper_cx = center.x + pan.x + (col_offset as f32) * zoom;
                    let paper_cy = center.y + pan.y + (row_offset as f32) * zoom;
                    let paper_rect = Rect::from_center_size(
                        Pos2::new(paper_cx, paper_cy),
                        Vec2::new((pw as f32) * zoom, (ph as f32) * zoom),
                    );
                    if paper_rect.contains(mp) {
                        clicked_sheet_id = Some(*s_id);
                    }

                    // Check placed viewports in reverse order (topmost drawn wins)
                    for vp in sheet_ref.viewports.iter().rev() {
                        let vp_w_mm = vp.width_mm.max(60.0);
                        let vp_h_mm = vp.height_mm.max(45.0);
                        let vp_x0 = paper_rect.min.x + (vp.position_mm.x as f32) * zoom;
                        let vp_x1 = vp_x0 + (vp_w_mm as f32) * zoom;
                        let vp_y1 = paper_rect.max.y - (vp.position_mm.y as f32) * zoom;
                        let vp_y0 = vp_y1 - (vp_h_mm as f32) * zoom;
                        let vp_rect = Rect::from_min_max(Pos2::new(vp_x0, vp_y0), Pos2::new(vp_x1, vp_y1));

                        // Check corner resize grips first if this viewport is already selected
                        if canvas_state.selected_placed_viewport == Some(vp.id) {
                            let grip_size = Vec2::splat(14.0);
                            let corners = [
                                (vp_rect.left_top(), SheetVpDragMode::Resize { left: true, top: true, right: false, bottom: false }),
                                (vp_rect.right_top(), SheetVpDragMode::Resize { left: false, top: true, right: true, bottom: false }),
                                (vp_rect.left_bottom(), SheetVpDragMode::Resize { left: true, top: false, right: false, bottom: true }),
                                (vp_rect.right_bottom(), SheetVpDragMode::Resize { left: false, top: false, right: true, bottom: true }),
                            ];
                            if let Some((_, mode)) = corners
                                .into_iter()
                                .find(|(pt, _)| Rect::from_center_size(*pt, grip_size).contains(mp))
                            {
                                hit_action = Some((vp.id, mode, *s_id));
                                break;
                            }
                        }

                        let title_band = Rect::from_min_max(
                            Pos2::new(vp_rect.min.x, vp_rect.max.y),
                            Pos2::new(
                                vp_rect.min.x + 210.0 * zoom.clamp(0.6, 1.5),
                                vp_rect.max.y + 30.0 * zoom.clamp(0.6, 1.5),
                            ),
                        );

                        if vp_rect.contains(mp) || (vp.show_title && title_band.contains(mp)) {
                            hit_action = Some((vp.id, SheetVpDragMode::Move, *s_id));
                            break;
                        }
                    }
                    if hit_action.is_some() {
                        break;
                    }
                }
            }

            if let Some((vp_id, mode, sid)) = hit_action {
                doc.push_undo();
                doc.active_sheet_id = Some(sid);
                canvas_state.selected_placed_viewport = Some(vp_id);
                canvas_state.dragging_sheet_viewport = Some((vp_id, mode));
            } else {
                canvas_state.selected_placed_viewport = None;
                if let Some(new_sid) = clicked_sheet_id {
                    if new_sid != active_sheet_id {
                        doc.active_sheet_id = Some(new_sid);
                    }
                }
            }
        }
    }

    // Apply ongoing drag / resize delta in paper millimetres
    if let Some((drag_vp_id, drag_mode)) = canvas_state.dragging_sheet_viewport {
        if primary_down {
            if pointer_delta.length_sq() > 0.0 && zoom > 0.001 {
                let dx_mm = (pointer_delta.x / zoom) as f64;
                let dy_mm = -(pointer_delta.y / zoom) as f64;
                for sheet_mut in doc.sheets.values_mut() {
                    if let Some(vp_mut) = sheet_mut.viewports.iter_mut().find(|v| v.id == drag_vp_id) {
                        match drag_mode {
                            SheetVpDragMode::Move => {
                                vp_mut.position_mm.x += dx_mm;
                                vp_mut.position_mm.y += dy_mm;
                            }
                            SheetVpDragMode::Resize { left, bottom, right, top } => {
                                if left {
                                    let new_w = (vp_mut.width_mm - dx_mm).clamp(60.0, 1500.0);
                                    let applied = vp_mut.width_mm - new_w;
                                    vp_mut.position_mm.x += applied;
                                    vp_mut.width_mm = new_w;
                                }
                                if right {
                                    vp_mut.width_mm = (vp_mut.width_mm + dx_mm).clamp(60.0, 1500.0);
                                }
                                if bottom {
                                    let new_h = (vp_mut.height_mm - dy_mm).clamp(45.0, 1200.0);
                                    let applied = vp_mut.height_mm - new_h;
                                    vp_mut.position_mm.y += applied;
                                    vp_mut.height_mm = new_h;
                                }
                                if top {
                                    vp_mut.height_mm = (vp_mut.height_mm + dy_mm).clamp(45.0, 1200.0);
                                }
                            }
                        }
                        break;
                    }
                }
            }
            let cursor = match drag_mode {
                SheetVpDragMode::Move => egui::CursorIcon::Grabbing,
                SheetVpDragMode::Resize { left, top, right, bottom } => {
                    if (left && top) || (right && bottom) {
                        egui::CursorIcon::ResizeNwSe
                    } else {
                        egui::CursorIcon::ResizeNeSw
                    }
                }
            };
            ui.ctx().set_cursor_icon(cursor);
        } else {
            canvas_state.dragging_sheet_viewport = None;
        }
    }

    for (idx, s_id) in sheet_ids.iter().enumerate() {
        let sheet = match doc.sheets.get(s_id) {
            Some(s) => s.clone(),
            None => continue,
        };
        let (pw, ph) = sheet.paper_size.dimensions_mm();
        let col = idx % 2;
        let row = idx / 2;
        let col_offset = (col as f64 - active_col as f64) * (pw + 140.0);
        let row_offset = (row as f64 - active_row as f64) * (ph + 140.0);

        let paper_cx = center.x + pan.x + (col_offset as f32) * zoom;
        let paper_cy = center.y + pan.y + (row_offset as f32) * zoom;
        let paper_w = (pw as f32) * zoom;
        let paper_h = (ph as f32) * zoom;

        let paper_rect = Rect::from_center_size(Pos2::new(paper_cx, paper_cy), Vec2::new(paper_w, paper_h));
        let is_active = *s_id == active_sheet_id;

        // 1. Drop shadow
        let shadow_rect = paper_rect.translate(Vec2::new(6.0, 6.0));
        painter.rect_filled(shadow_rect, 2.0, Color32::from_black_alpha(70));

        // 2. White Paper
        painter.rect_filled(paper_rect, 0.0, Color32::WHITE);
        if is_active {
            painter.rect_stroke(paper_rect, 0.0, Stroke::new(2.0, theme.accent), egui::StrokeKind::Inside);
        } else {
            painter.rect_stroke(paper_rect, 0.0, Stroke::new(1.0, Color32::from_gray(180)), egui::StrokeKind::Inside);
        }

        // 3. Margins & Titleblock
        let m20 = 20.0 * zoom;
        let border_rect = paper_rect.shrink(m20);
        painter.rect_stroke(border_rect, 0.0, Stroke::new(1.4, Color32::BLACK), egui::StrokeKind::Inside);

        let m10 = 10.0 * zoom;
        let inner_rect = paper_rect.shrink(m10);
        painter.rect_stroke(inner_rect, 0.0, Stroke::new(0.6, Color32::from_gray(190)), egui::StrokeKind::Inside);

        // Architectural Title Block in bottom right (180mm x 65mm)
        let tb_w = 180.0 * zoom;
        let tb_h = 65.0 * zoom;
        let tb_rect = Rect::from_min_max(
            Pos2::new(border_rect.max.x - tb_w, border_rect.max.y - tb_h),
            border_rect.max,
        );
        painter.rect_filled(tb_rect, 0.0, Color32::from_rgb(248, 250, 252));
        painter.rect_stroke(tb_rect, 0.0, Stroke::new(1.2, Color32::BLACK), egui::StrokeKind::Inside);

        // Title Block Content
        painter.text(
            Pos2::new(tb_rect.min.x + 8.0 * zoom, tb_rect.min.y + 12.0 * zoom),
            egui::Align2::LEFT_TOP,
            &doc.project_info.name,
            egui::FontId::proportional((11.0 * zoom).clamp(8.0, 16.0)),
            Color32::BLACK,
        );
        painter.text(
            Pos2::new(tb_rect.min.x + 8.0 * zoom, tb_rect.min.y + 28.0 * zoom),
            egui::Align2::LEFT_TOP,
            &doc.project_info.client,
            egui::FontId::proportional((9.0 * zoom).clamp(7.0, 13.0)),
            Color32::from_gray(90),
        );
        painter.text(
            Pos2::new(tb_rect.max.x - 10.0 * zoom, tb_rect.max.y - 12.0 * zoom),
            egui::Align2::RIGHT_BOTTOM,
            &sheet.sheet_number,
            egui::FontId::proportional((20.0 * zoom).clamp(11.0, 26.0)),
            Color32::BLACK,
        );
        painter.text(
            Pos2::new(tb_rect.min.x + 8.0 * zoom, tb_rect.max.y - 12.0 * zoom),
            egui::Align2::LEFT_BOTTOM,
            &sheet.name,
            egui::FontId::proportional((10.0 * zoom).clamp(8.0, 14.0)),
            Color32::from_gray(50),
        );

        // 4. Placed Viewports on Sheet (Freely Draggable & Resizable)
        for vp in &sheet.viewports {
            let vp_w_mm = vp.width_mm.max(60.0);
            let vp_h_mm = vp.height_mm.max(45.0);

            let vp_x0 = paper_rect.min.x + (vp.position_mm.x as f32) * zoom;
            let vp_x1 = vp_x0 + (vp_w_mm as f32) * zoom;
            let vp_y1 = paper_rect.max.y - (vp.position_mm.y as f32) * zoom;
            let vp_y0 = vp_y1 - (vp_h_mm as f32) * zoom;
            let vp_rect = Rect::from_min_max(Pos2::new(vp_x0, vp_y0), Pos2::new(vp_x1, vp_y1));

            let is_vp_selected = canvas_state.selected_placed_viewport == Some(vp.id);
            let is_vp_hovered = canvas_state.dragging_sheet_viewport.is_none()
                && pointer_pos.map_or(false, |mp| vp_rect.contains(mp));

            if is_vp_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }

            // Viewport boundary frame
            painter.rect_filled(vp_rect, 0.0, Color32::from_rgb(252, 252, 253));
            if is_vp_selected {
                painter.rect_stroke(vp_rect, 0.0, Stroke::new(2.2, theme.accent), egui::StrokeKind::Inside);
            } else if is_vp_hovered {
                painter.rect_stroke(
                    vp_rect,
                    0.0,
                    Stroke::new(1.5, theme.accent.gamma_multiply(0.65)),
                    egui::StrokeKind::Inside,
                );
            } else {
                painter.rect_stroke(vp_rect, 0.0, Stroke::new(1.0, Color32::from_gray(180)), egui::StrokeKind::Inside);
            }

            // CRITICAL: Scissor clipping so primitives NEVER bleed out of viewport
            let vp_painter = painter.with_clip_rect(vp_rect);

            // Render view scene inside viewport
            if let Some(view_def) = doc.views.get(&vp.view_id) {
                // Placed viewports on sheet paper desks MUST ALWAYS use Light ColorTheme!
                // Paper sheets are physically white drawing paper; dark UI modes must never invert paper drawings.
                let mut color_theme = plinth_views::ColorTheme::light();
                color_theme.materials = canvas_state.materials;
                let scene = plinth_views::render_view_with_style_and_theme(doc, view_def, &color_theme, canvas_state.visual_style);
                let vp_center = vp_rect.center();
                let scale_factor = (1.0 / (vp.scale.max(1) as f32)) * zoom;

                let (model_cx, model_cy) = if scene.bounds.is_valid() {
                    let c = scene.bounds.center();
                    (c.x, c.y)
                } else {
                    (0.0, 0.0)
                };

                let to_vp_screen = |p: plinth_geom::Vec2| -> Pos2 {
                    Pos2::new(
                        vp_center.x + ((p.x - model_cx) as f32) * scale_factor,
                        vp_center.y - ((p.y - model_cy) as f32) * scale_factor,
                    )
                };

                for prim in &scene.primitives {
                    match prim {
                        RenderPrimitive::Line { start, end, color, width, dashed } => {
                            let p1 = to_vp_screen(*start);
                            let p2 = to_vp_screen(*end);
                            let stroke_color = Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                            let stroke = Stroke::new(*width, stroke_color);

                            if *dashed {
                                let dir = p2 - p1;
                                let len = dir.length();
                                if len > 1.0 {
                                    let u = dir / len;
                                    let dash_len = 8.0f32;
                                    let gap_len = 5.0f32;
                                    let mut curr = 0.0f32;
                                    while curr < len {
                                        let end_seg = (curr + dash_len).min(len);
                                        draw_line_segment_safe(&vp_painter, p1 + u * curr, p1 + u * end_seg, stroke);
                                        curr += dash_len + gap_len;
                                    }
                                }
                            } else {
                                draw_line_segment_safe(&vp_painter, p1, p2, stroke);
                            }
                        }
                        RenderPrimitive::Polygon { points, fill_color, stroke_color, stroke_width } => {
                            if points.len() >= 3 {
                                let screen_pts: Vec<Pos2> = points.iter().map(|&p| to_vp_screen(p)).collect();
                                let fill = Color32::from_rgba_premultiplied(fill_color[0], fill_color[1], fill_color[2], fill_color[3]);
                                let stroke = stroke_color.map(|c| {
                                    Stroke::new(*stroke_width, Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
                                }).unwrap_or(Stroke::NONE);
                                draw_polygon_safe(&vp_painter, &screen_pts, fill, stroke);
                            }
                        }
                        RenderPrimitive::Arc { center: arc_c, radius, start_angle, end_angle, color, width } => {
                            let center_sp = to_vp_screen(*arc_c);
                            let r_px = radius * scale_factor;
                            let stroke = Stroke::new(*width, Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]));
                            let segs = 16;
                            let sweep = end_angle - start_angle;
                            let pts: Vec<Pos2> = (0..=segs).map(|i| {
                                let theta = start_angle + sweep * (i as f32 / segs as f32);
                                Pos2::new(center_sp.x + r_px * theta.cos(), center_sp.y - r_px * theta.sin())
                            }).collect();
                            for pair in pts.windows(2) {
                                vp_painter.line_segment([pair[0], pair[1]], stroke);
                            }
                        }
                        RenderPrimitive::Circle { center: c, radius, fill_color, stroke_color, stroke_width } => {
                            let center_sp = to_vp_screen(*c);
                            let r_px = radius * scale_factor;
                            let fill = fill_color.map(|fc| Color32::from_rgba_premultiplied(fc[0], fc[1], fc[2], fc[3])).unwrap_or(Color32::TRANSPARENT);
                            let stroke = Stroke::new(*stroke_width, Color32::from_rgba_premultiplied(stroke_color[0], stroke_color[1], stroke_color[2], stroke_color[3]));
                            vp_painter.circle(center_sp, r_px, fill, stroke);
                        }
                        RenderPrimitive::Text { pos, text, color, size, align_center } => {
                            let sp = to_vp_screen(*pos);
                            let align = if *align_center { egui::Align2::CENTER_CENTER } else { egui::Align2::LEFT_TOP };
                            let text_color = Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                            vp_painter.text(sp, align, text, egui::FontId::proportional(*size), text_color);
                        }
                    }
                }
            }

            // Selected Viewport Corner Resize Grips & Live Position Badge
            if is_vp_selected {
                for corner in [
                    vp_rect.left_top(),
                    vp_rect.right_top(),
                    vp_rect.left_bottom(),
                    vp_rect.right_bottom(),
                ] {
                    let g_rect = Rect::from_center_size(corner, Vec2::splat(8.0));
                    painter.rect_filled(g_rect, 1.5, theme.accent);
                    painter.rect_stroke(g_rect, 1.5, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);
                }

                let badge_txt = format!(
                    "✥ ({:.0}, {:.0} mm)  •  {:.0}×{:.0} mm",
                    vp.position_mm.x, vp.position_mm.y, vp_w_mm, vp_h_mm
                );
                let badge_rect = Rect::from_min_size(
                    Pos2::new(vp_rect.min.x + 4.0, vp_rect.min.y - 22.0),
                    Vec2::new(205.0, 19.0),
                );
                painter.rect_filled(badge_rect, 4.0, Color32::from_rgba_unmultiplied(18, 24, 34, 230));
                painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, theme.accent), egui::StrokeKind::Inside);
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    badge_txt,
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );
            }

            // Automatic Architectural Drawing Title Bubble
            if vp.show_title {
                let bubble_center = Pos2::new(vp_rect.min.x + 18.0 * zoom.clamp(0.6, 1.5), vp_rect.max.y + 18.0 * zoom.clamp(0.6, 1.5));
                let bubble_r = 10.0 * zoom.clamp(0.6, 1.5);
                painter.circle_filled(bubble_center, bubble_r, Color32::WHITE);
                painter.circle_stroke(bubble_center, bubble_r, Stroke::new(1.2, Color32::BLACK));
                painter.text(
                    bubble_center,
                    egui::Align2::CENTER_CENTER,
                    format!("{}", vp.number),
                    egui::FontId::proportional((11.0 * zoom).clamp(8.0, 16.0)),
                    Color32::BLACK,
                );

                let title_x = bubble_center.x + 16.0 * zoom.clamp(0.6, 1.5);
                painter.text(
                    Pos2::new(title_x, bubble_center.y - 4.0 * zoom.clamp(0.6, 1.5)),
                    egui::Align2::LEFT_BOTTOM,
                    &vp.title,
                    egui::FontId::proportional((12.0 * zoom).clamp(8.0, 18.0)),
                    Color32::BLACK,
                );
                let scale_text = format!("Scale: 1:{}", vp.scale);
                painter.text(
                    Pos2::new(title_x, bubble_center.y + 12.0 * zoom.clamp(0.6, 1.5)),
                    egui::Align2::LEFT_BOTTOM,
                    scale_text,
                    egui::FontId::proportional((9.5 * zoom).clamp(7.0, 14.0)),
                    Color32::from_gray(100),
                );
                // Underline
                let line_len = 160.0 * zoom.clamp(0.6, 1.5);
                painter.line_segment(
                    [Pos2::new(bubble_center.x, bubble_center.y + bubble_r + 2.0), Pos2::new(title_x + line_len, bubble_center.y + bubble_r + 2.0)],
                    Stroke::new(1.0, Color32::BLACK),
                );
            }
        }
    }

    // Top Header Pill: Information Banner
    let banner_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 20.0, rect.min.y + 16.0),
        Vec2::new(540.0, 30.0),
    );
    painter.rect_filled(banner_rect, 6.0, theme.panel_bg);
    painter.rect_stroke(banner_rect, 6.0, Stroke::new(1.0, theme.border_color), egui::StrokeKind::Inside);
    painter.text(
        banner_rect.center(),
        egui::Align2::CENTER_CENTER,
        "📄 Sheet Layout Desk — Drag viewports to move freely • Corner grips to resize • MMB to pan",
        egui::FontId::proportional(11.0),
        theme.text_primary,
    );
}

/// Clips and renders a 2D line segment safely against the painter's viewport bounds
/// using Liang-Barsky parametric line clipping, preventing rays shooting to infinity or across panes.
pub(crate) fn draw_line_segment_safe(
    painter: &egui::Painter,
    p0: Pos2,
    p1: Pos2,
    stroke: Stroke,
) {
    if stroke == Stroke::NONE || stroke.width <= 0.0 {
        return;
    }
    let clip = painter.clip_rect().expand(4.0);
    let min_b = plinth_geom::vec2(clip.min.x as f64, clip.min.y as f64);
    let max_b = plinth_geom::vec2(clip.max.x as f64, clip.max.y as f64);

    let p0_v = plinth_geom::vec2(p0.x as f64, p0.y as f64);
    let p1_v = plinth_geom::vec2(p1.x as f64, p1.y as f64);

    if let Some((c0, c1)) = plinth_geom::polygon::clip_segment_to_box(p0_v, p1_v, min_b, max_b) {
        painter.line_segment(
            [Pos2::new(c0.x as f32, c0.y as f32), Pos2::new(c1.x as f32, c1.y as f32)],
            stroke,
        );
    }
}

/// Renders a 2D polygon safely with Sutherland-Hodgman viewport clipping and ear-clipping triangulation,
/// completely preventing screen-splitting diagonal chords, rays shooting to infinity, and tessellator overflows.
/// Renders a 2D polygon safely with trivial bounds rejection, monolithic convex rendering,
/// and ear-clipping triangulation for concave polygons.
/// Prevents screen-splitting diagonal chords, rays shooting to infinity, and tessellator overflows
/// while letting GPU scissor clipping cleanly handle viewport edges without introducing false boundary vertices or wedges.
pub(crate) fn draw_polygon_safe(
    painter: &egui::Painter,
    screen_pts: &[Pos2],
    fill: Color32,
    stroke: Stroke,
) {
    let n = screen_pts.len();
    if n < 3 {
        return;
    }

    // 1. Numerical sanity check: skip any polygon with NaN or Inf coordinates
    for p in screen_pts {
        if !p.x.is_finite() || !p.y.is_finite() {
            return;
        }
    }

    // 2. Trivial rejection check against painter's clip rect:
    // If all vertices lie entirely to the left, right, top, or bottom of the clip area, skip immediately.
    let clip = painter.clip_rect();
    let margin = 50.0f32; // Allow reasonable margin for line stroke width and anti-aliasing
    let all_left = screen_pts.iter().all(|p| p.x < clip.min.x - margin);
    let all_right = screen_pts.iter().all(|p| p.x > clip.max.x + margin);
    let all_top = screen_pts.iter().all(|p| p.y < clip.min.y - margin);
    let all_bottom = screen_pts.iter().all(|p| p.y > clip.max.y + margin);
    if all_left || all_right || all_top || all_bottom {
        return;
    }

    let pts_2d: Vec<plinth_geom::Vec2> = screen_pts
        .iter()
        .map(|p| plinth_geom::vec2(p.x as f64, p.y as f64))
        .collect();

    // 3. For any strictly convex polygon (triangles, quads, n-gons):
    // Render directly as a single monolithic polygon via egui!
    // This completely eliminates internal anti-aliased seam lines (satisfying Rule 1).
    if n == 3 || plinth_geom::polygon::is_convex(&pts_2d) {
        painter.add(egui::Shape::convex_polygon(screen_pts.to_vec(), fill, stroke));
        return;
    }

    // 4. For concave polygons (L-shaped, U-shaped, multi-wing floor plates):
    // Use ear-clipping triangulation on original clean polygon vertices into a single
    // indexed `egui::Mesh`. This completely eliminates internal anti-aliasing seam lines
    // between adjacent triangles (satisfying Rule 1).
    // The GPU hardware scissor rect (gl.scissor) set by painter.with_clip_rect()
    // cleanly cuts off any parts extending outside without CPU polygon corruption.
    let tris = plinth_geom::polygon::triangulate(&pts_2d);
    if !tris.is_empty() {
        if fill != Color32::TRANSPARENT {
            let mut mesh = egui::Mesh::default();
            for &pt in screen_pts {
                mesh.colored_vertex(pt, fill);
            }
            for t in &tris {
                mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        if stroke != Stroke::NONE && stroke.width > 0.0 {
            for i in 0..n {
                draw_line_segment_safe(painter, screen_pts[i], screen_pts[(i + 1) % n], stroke);
            }
        }
    } else {
        // Fallback when ear clipping fails: stroke the perimeter only, never fan across voids or offscreen
        if stroke != Stroke::NONE && stroke.width > 0.0 {
            for i in 0..n {
                draw_line_segment_safe(painter, screen_pts[i], screen_pts[(i + 1) % n], stroke);
            }
        }
    }
}
