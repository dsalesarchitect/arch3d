//! Ribbon toolbar component (Architecture, Structure, Draw, Modify, Manage, Insert).

use crate::icons::{draw_icon, IconKind};
use crate::structure_auto::AutoMode;
use crate::theme::AppTheme;
use crate::tool::Tool;
use crate::tool_config::ToolCategory;
use egui::{Color32, Rect, Response, Sense, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonTab {
    Home,
    Architecture,
    Structure,
    Site,
    FreeModeling,
    Draw,
    Modify,
    Annotation,
    View,
    Manage,
    Insert,
}

pub const SCALE_PRESETS: &[u32] = &[
    1, 2, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300, 400, 500, 1000,
];

pub struct RibbonAction {
    pub new_tool: Option<Tool>,
    pub open_tool_config: Option<ToolCategory>,
    pub open_struct_auto: Option<AutoMode>,
    pub open_site_terrain: bool,
    pub open_lot_survey: bool,
    pub open_level_manager: bool,
    pub open_grid_manager: bool,
    pub open_copy_level: bool,
    pub open_paste_level: bool,
    pub delete_selected: bool,
    pub undo: bool,
    pub redo: bool,
    pub load_sample: bool,
    pub load_apartment_highrise: bool,
    pub export_ifc: bool,
    pub import_ifc: bool,
    pub toggle_theme: bool,
    pub open_default_3d: bool,
    pub open_settings: bool,
    pub new_project: bool,
    pub open_project: bool,
    pub save_project: bool,
    pub save_as_project: bool,
    pub zoom_extents: bool,
    pub import_file: bool,
    pub import_skp: bool,
    pub import_dwg: bool,
    pub export_file: bool,
    pub split_single: bool,
    pub split_horizontal: bool,
    pub split_vertical: bool,
    pub split_triple: bool,
    pub split_quad: bool,
    pub new_sheet: bool,
    pub export_pdf: bool,
    pub export_glb: bool,
    pub open_plugin_manager: bool,
    pub trigger_render_mutracer: bool,
    pub trigger_sync_blender: bool,
    pub set_view_scale: Option<u32>,
}

impl Default for RibbonAction {
    fn default() -> Self {
        Self {
            new_tool: None,
            open_tool_config: None,
            open_struct_auto: None,
            open_site_terrain: false,
            open_lot_survey: false,
            open_level_manager: false,
            open_grid_manager: false,
            open_copy_level: false,
            open_paste_level: false,
            delete_selected: false,
            undo: false,
            redo: false,
            load_sample: false,
            load_apartment_highrise: false,
            export_ifc: false,
            import_ifc: false,
            toggle_theme: false,
            open_default_3d: false,
            open_settings: false,
            open_plugin_manager: false,
            trigger_render_mutracer: false,
            trigger_sync_blender: false,
            set_view_scale: None,
            new_project: false,
            open_project: false,
            save_project: false,
            save_as_project: false,
            zoom_extents: false,
            import_file: false,
            import_skp: false,
            import_dwg: false,
            export_file: false,
            export_pdf: false,
            export_glb: false,
            split_single: false,
            split_horizontal: false,
            split_vertical: false,
            split_triple: false,
            split_quad: false,
            new_sheet: false,
        }
    }
}

pub fn render_ribbon(
    ui: &mut Ui,
    current_tab: &mut RibbonTab,
    active_tool: &Tool,
    has_selection: bool,
    show_tool_labels: bool,
    theme: &AppTheme,
    annotation: &mut crate::settings::AnnotationSettings,
    mutracer_active: bool,
    blender_active: bool,
) -> RibbonAction {
    let mut action = RibbonAction::default();

    // Top Quick Access + Tab bar
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

        // Arch3D App Logo Icon & Brand Title
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::hover());
        draw_icon(ui.painter(), icon_rect, IconKind::Arch3dApp, theme.accent);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);
            ui.label(egui::RichText::new("ARCH").strong().size(13.0).color(theme.text_primary));
            ui.label(egui::RichText::new("3D").strong().size(13.0).color(theme.accent));
        });
        ui.separator();

        // Quick Access Toolbar: Save, Undo, Redo, Default 3D (Icon-only)
        if quick_icon_button(ui, IconKind::Save, "Save Project (Ctrl+S)", theme).clicked() {
            action.save_project = true;
        }
        if quick_icon_button(ui, IconKind::Undo, "Undo (Ctrl+Z)", theme).clicked() {
            action.undo = true;
        }
        if quick_icon_button(ui, IconKind::Redo, "Redo (Ctrl+Y)", theme).clicked() {
            action.redo = true;
        }
        if quick_icon_button(ui, IconKind::ThreeD, "Default 3D View", theme).clicked() {
            action.open_default_3d = true;
        }

        ui.separator();

        // Ribbon Category Tabs
        for (tab, label) in [
            (RibbonTab::Home, "Home"),
            (RibbonTab::Architecture, "Architecture"),
            (RibbonTab::Structure, "Structure"),
            (RibbonTab::Site, "Site"),
            (RibbonTab::FreeModeling, "Free Modeling"),
            (RibbonTab::Draw, "Draw"),
            (RibbonTab::Modify, "Modify"),
            (RibbonTab::Annotation, "Annotation"),
            (RibbonTab::View, "View"),
            (RibbonTab::Manage, "Manage"),
            (RibbonTab::Insert, "Insert"),
        ] {
            let is_selected = *current_tab == tab;
            let text = egui::RichText::new(label)
                .size(12.0)
                .color(if is_selected { theme.accent } else { theme.text_primary });

            let btn = ui.selectable_label(is_selected, text);
            if btn.clicked() {
                *current_tab = tab;
            }
        }

        // Right side: Dark/Light Theme toggle & Official Links
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // In WebAssembly, reserve clear space for the floating "Landing Page" HTML button FIRST
            // (in right_to_left layout, the first added element occupies the rightmost edge)
            #[cfg(target_arch = "wasm32")]
            ui.add_space(132.0);

            let theme_label = match theme.mode {
                crate::theme::ThemeMode::Dark => "☀ Light Mode",
                crate::theme::ThemeMode::Light => "🌙 Dark Mode",
            };
            if ui.button(theme_label).clicked() {
                action.toggle_theme = true;
            }

            ui.add_space(6.0);
            ui.hyperlink_to(
                egui::RichText::new("▶ YouTube").size(11.0).strong().color(Color32::from_rgb(255, 90, 90)),
                "https://www.youtube.com/@OpenSourceArchitecture",
            );
        });
    });

    ui.separator();

    // Ribbon Tool Panels Bar - Condensed spacing for a clean compact aesthetic
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

        let btn = |ui: &mut Ui, label: &str, icon: IconKind, active: bool| -> Response {
            ribbon_tool_button(ui, label, icon, active, show_tool_labels, theme)
        };

        match *current_tab {
            RibbonTab::Home => {
                // Project File Management (New, Open, Save, Save As)
                if btn(ui, "New", IconKind::NewProject, false).on_hover_text("Create New BIM Project (Ctrl+N)").clicked() {
                    action.new_project = true;
                }
                if btn(ui, "Open", IconKind::OpenProject, false).on_hover_text("Open Arch3D Project (.arch3d) or CAD/BIM File (Ctrl+O)").clicked() {
                    action.open_project = true;
                }
                if btn(ui, "Save", IconKind::Save, false).on_hover_text("Save Current Project (Ctrl+S)").clicked() {
                    action.save_project = true;
                }
                if btn(ui, "Save As", IconKind::SaveAs, false).on_hover_text("Save Project to a new file").clicked() {
                    action.save_as_project = true;
                }

                ui.separator();

                // BIM Data Exchange (Import, Export)
                if btn(ui, "Import", IconKind::Import, false).clicked() {
                    action.import_file = true;
                }
                if btn(ui, "Import SKP", IconKind::ImportSkp, false).clicked() {
                    action.import_skp = true;
                }
                if btn(ui, "Import DWG", IconKind::ImportDwg, false).clicked() {
                    action.import_dwg = true;
                }
                if btn(ui, "Export", IconKind::Export, false).clicked() {
                    action.export_file = true;
                }
                if btn(ui, "Export GLB", IconKind::ExportGlb, false).on_hover_text("Export 3D Model to glTF Binary (.glb) for MuTracer / WebGPU / Three.js Path Tracing").clicked() {
                    action.export_glb = true;
                }
                if mutracer_active {
                    if btn(ui, "Render", IconKind::RenderMuTracer, false).on_hover_text("Open Live MuTracer WebGPU Path Tracer Studio").clicked() {
                        action.trigger_render_mutracer = true;
                    }
                }
                if blender_active {
                    if btn(ui, "Sync Blender", IconKind::BlenderLiveSync, false).on_hover_text("Live Sync 3D BIM model directly into Blender Cycles/EEVEE (port 9876)").clicked() {
                        action.trigger_sync_blender = true;
                    }
                }
                if btn(ui, "Export PDF", IconKind::ExportPdf, false).on_hover_text("Export Architectural Sheet Drawings to Vector PDF (Ctrl+P)").clicked() {
                    action.export_pdf = true;
                }

                ui.separator();

                // Templates & Quick Start
                if btn(ui, "Sample Model", IconKind::ThreeD, false).on_hover_text("Load 2-Storey Residential Sample").clicked() {
                    action.load_sample = true;
                }
                if btn(ui, "Apartment (IFC)", IconKind::ThreeD, false).on_hover_text("Load 10-Storey Apartment Highrise Benchmark IFC Model").clicked() {
                    action.load_apartment_highrise = true;
                }

                ui.separator();

                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Arch3D Project Workspace").size(11.0).strong().color(theme.text_primary));
                    ui.label(egui::RichText::new("Lossless .arch3d native project file & standard ISO 10303-21 IFC4 exchange").size(10.0).color(theme.text_muted));
                });
            }

            RibbonTab::Architecture => {
                let mut config_btn = |ui: &mut Ui, label: &str, icon: IconKind, active: bool, tool: Tool, cat: ToolCategory| {
                    let resp = ribbon_configurable_tool_button(ui, label, icon, active, show_tool_labels, theme);
                    if resp.double_clicked() || resp.secondary_clicked() {
                        action.new_tool = Some(tool);
                        action.open_tool_config = Some(cat);
                    } else if resp.clicked() {
                        action.new_tool = Some(tool);
                    }
                };

                config_btn(
                    ui,
                    "Wall",
                    IconKind::Wall,
                    matches!(active_tool, Tool::DrawWall { is_structural: false, .. }),
                    Tool::DrawWall { start: None, is_structural: false },
                    ToolCategory::WallArch,
                );
                config_btn(
                    ui,
                    "Curtain Wall",
                    IconKind::CurtainWall,
                    matches!(active_tool, Tool::DrawCurtainWall { .. }),
                    Tool::DrawCurtainWall { start: None },
                    ToolCategory::CurtainWall,
                );
                config_btn(
                    ui,
                    "Door",
                    IconKind::Door,
                    matches!(active_tool, Tool::PlaceDoor),
                    Tool::PlaceDoor,
                    ToolCategory::Door,
                );
                config_btn(
                    ui,
                    "Window",
                    IconKind::Window,
                    matches!(active_tool, Tool::PlaceWindow),
                    Tool::PlaceWindow,
                    ToolCategory::Window,
                );
                config_btn(
                    ui,
                    "Column",
                    IconKind::Column,
                    matches!(active_tool, Tool::DrawColumn { is_structural: false }),
                    Tool::DrawColumn { is_structural: false },
                    ToolCategory::ColumnArch,
                );
                config_btn(
                    ui,
                    "Floor",
                    IconKind::Slab,
                    matches!(active_tool, Tool::DrawSlab { is_structural: false, .. }),
                    Tool::DrawSlab { points: Vec::new(), is_structural: false },
                    ToolCategory::FloorArch,
                );
                config_btn(
                    ui,
                    "Ceiling",
                    IconKind::Ceiling,
                    matches!(active_tool, Tool::DrawCeiling { .. }),
                    Tool::DrawCeiling { points: Vec::new() },
                    ToolCategory::Ceiling,
                );
                ui.separator();
                config_btn(
                    ui,
                    "Stair",
                    IconKind::Stair,
                    matches!(active_tool, Tool::DrawStair { .. }),
                    Tool::DrawStair { start: None },
                    ToolCategory::Stair,
                );
                config_btn(
                    ui,
                    "Ramp",
                    IconKind::Ramp,
                    matches!(active_tool, Tool::DrawRamp { .. }),
                    Tool::DrawRamp { start: None },
                    ToolCategory::Ramp,
                );
                config_btn(
                    ui,
                    "Railing",
                    IconKind::Railing,
                    matches!(active_tool, Tool::DrawRailing { .. }),
                    Tool::DrawRailing { points: Vec::new() },
                    ToolCategory::Railing,
                );
                ui.separator();
                config_btn(
                    ui,
                    "Roof",
                    IconKind::Roof,
                    matches!(active_tool, Tool::DrawRoof { .. }),
                    Tool::DrawRoof { shape: plinth_model::RoofShape::Hip, points: Vec::new() },
                    ToolCategory::Roof,
                );
                config_btn(
                    ui,
                    "Fascia",
                    IconKind::Fascia,
                    matches!(active_tool, Tool::DrawFascia { .. }),
                    Tool::DrawFascia { points: Vec::new() },
                    ToolCategory::Fascia,
                );
                config_btn(
                    ui,
                    "Gutter",
                    IconKind::Gutter,
                    matches!(active_tool, Tool::DrawGutter { .. }),
                    Tool::DrawGutter { points: Vec::new() },
                    ToolCategory::Gutter,
                );
            }

            RibbonTab::Structure => {
                let mut config_btn = |ui: &mut Ui, label: &str, icon: IconKind, active: bool, tool: Tool, cat: ToolCategory| {
                    let resp = ribbon_configurable_tool_button(ui, label, icon, active, show_tool_labels, theme);
                    if resp.double_clicked() || resp.secondary_clicked() {
                        action.new_tool = Some(tool);
                        action.open_tool_config = Some(cat);
                    } else if resp.clicked() {
                        action.new_tool = Some(tool);
                    }
                };

                config_btn(
                    ui,
                    "Col (Str)",
                    IconKind::Column,
                    matches!(active_tool, Tool::DrawColumn { is_structural: true }),
                    Tool::DrawColumn { is_structural: true },
                    ToolCategory::ColumnStruct,
                );
                config_btn(
                    ui,
                    "Footing",
                    IconKind::Footing,
                    matches!(active_tool, Tool::DrawFooting),
                    Tool::DrawFooting,
                    ToolCategory::Footing,
                );
                config_btn(
                    ui,
                    "Beam",
                    IconKind::Beam,
                    matches!(active_tool, Tool::DrawBeam { .. }),
                    Tool::DrawBeam { start: None },
                    ToolCategory::Beam,
                );
                config_btn(
                    ui,
                    "Wall (Str)",
                    IconKind::Wall,
                    matches!(active_tool, Tool::DrawWall { is_structural: true, .. }),
                    Tool::DrawWall { start: None, is_structural: true },
                    ToolCategory::WallStruct,
                );
                config_btn(
                    ui,
                    "Slab (Str)",
                    IconKind::Slab,
                    matches!(active_tool, Tool::DrawSlab { is_structural: true, .. }),
                    Tool::DrawSlab { points: Vec::new(), is_structural: true },
                    ToolCategory::SlabStruct,
                );

                ui.separator();

                // Automated Structural Framing & Substructure Tools
                if btn(ui, "Cols on Grids", IconKind::AutoColumnsOnGrids, false)
                    .on_hover_text("Place Structural Columns automatically at all Grid Intersections")
                    .clicked()
                {
                    action.open_struct_auto = Some(AutoMode::ColumnsOnGrids);
                }
                if btn(ui, "Footing on Cols", IconKind::AutoFootingsOnColumns, false)
                    .on_hover_text("Place Foundation Footings automatically beneath Columns")
                    .clicked()
                {
                    action.open_struct_auto = Some(AutoMode::FootingsOnColumns);
                }
                if btn(ui, "Beams on Grids", IconKind::AutoBeamsOnGrids, false)
                    .on_hover_text("Place Structural Beams automatically along Grids & Columns")
                    .clicked()
                {
                    action.open_struct_auto = Some(AutoMode::BeamsOnGridsOrColumns);
                }
                if btn(ui, "Slab on Bays", IconKind::AutoSlabsOnBays, false)
                    .on_hover_text("Place Structural Slabs automatically inside Closed Column-Beam Bays")
                    .clicked()
                {
                    action.open_struct_auto = Some(AutoMode::SlabsOnClosedBays);
                }
            }

            RibbonTab::Site => {
                let mut config_btn = |ui: &mut Ui, label: &str, icon: IconKind, active: bool, tool: Tool, cat: ToolCategory, tip: &str| {
                    let resp = ribbon_configurable_tool_button(ui, label, icon, active, show_tool_labels, theme).on_hover_text(tip);
                    if resp.double_clicked() || resp.secondary_clicked() {
                        action.new_tool = Some(tool);
                        action.open_tool_config = Some(cat);
                    } else if resp.clicked() {
                        action.new_tool = Some(tool);
                    }
                };

                config_btn(
                    ui,
                    "Road",
                    IconKind::Road,
                    matches!(active_tool, Tool::DrawRoad { .. }),
                    Tool::DrawRoad { points: Vec::new() },
                    ToolCategory::Road,
                    "Sketch multi-lane road corridor • Double-click/R-click for Profile, Lanes & Curb settings",
                );
                config_btn(
                    ui,
                    "Sidewalk",
                    IconKind::Sidewalk,
                    matches!(active_tool, Tool::DrawSidewalk { .. }),
                    Tool::DrawSidewalk { points: Vec::new() },
                    ToolCategory::Sidewalk,
                    "Sketch sidewalk walkway • Double-click/R-click for Edge Profile, Bullnose Radius & Grooves",
                );
                config_btn(
                    ui,
                    "Pathwalk",
                    IconKind::Pathwalk,
                    matches!(active_tool, Tool::DrawPathwalk { .. }),
                    Tool::DrawPathwalk { points: Vec::new() },
                    ToolCategory::Pathwalk,
                    "Sketch landscape pathwalk • Double-click/R-click for Paver Pattern, Width & Edging",
                );

                ui.separator();

                if btn(
                    ui,
                    "Building Pad",
                    IconKind::BuildingPad,
                    matches!(active_tool, Tool::DrawBuildingPad { .. }),
                )
                .on_hover_text("Excavated foundation pad cutting through terrain")
                .clicked()
                {
                    action.new_tool = Some(Tool::DrawBuildingPad { points: Vec::new() });
                }

                if btn(
                    ui,
                    "Toposolid",
                    IconKind::Toposolid,
                    matches!(active_tool, Tool::DrawToposolid { .. }),
                )
                .on_hover_text("Volumetric solid terrain modeling with depth, soil strata, and contours")
                .clicked()
                {
                    action.new_tool = Some(Tool::DrawToposolid { points: Vec::new() });
                }

                if btn(
                    ui,
                    "Build Terrain & Contours",
                    IconKind::TerrainContours,
                    false,
                )
                .on_hover_text("Terrain spot elevations, contour lines, and topography studio")
                .clicked()
                {
                    action.open_site_terrain = true;
                }

                ui.separator();

                if btn(
                    ui,
                    "Lot Boundary (Sketch)",
                    IconKind::LotBoundary,
                    matches!(active_tool, Tool::DrawPropertyLine { .. }),
                )
                .on_hover_text("Sketch cadastral parcel property line boundary on site")
                .clicked()
                {
                    action.new_tool = Some(Tool::DrawPropertyLine { points: Vec::new() });
                }

                if btn(
                    ui,
                    "Lot Bearing Survey",
                    IconKind::LotBearing,
                    false,
                )
                .on_hover_text("Cadastral metes & bounds survey table, bearing angles, closure error & area")
                .clicked()
                {
                    action.open_lot_survey = true;
                }
            }

            RibbonTab::FreeModeling => {
                // 1. 2D Planar Surfaces
                if btn(ui, "Rectangle", IconKind::RectSurface, matches!(active_tool, Tool::DrawRectSurface { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawRectSurface { corner1: None });
                }
                if btn(ui, "Circle", IconKind::CircleSurface, matches!(active_tool, Tool::DrawCircleSurface { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawCircleSurface { center: None });
                }
                if btn(ui, "Polygon", IconKind::PolygonSurface, matches!(active_tool, Tool::DrawPolygonSurface { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawPolygonSurface { points: Vec::new() });
                }

                ui.separator();

                // 2. SketchUp-style Push/Pull
                if btn(ui, "Push / Pull", IconKind::PushPull, matches!(active_tool, Tool::PushPull { .. })).clicked() {
                    action.new_tool = Some(Tool::PushPull { target_id: None, drag_start: None, current_height: 3000.0 });
                }

                ui.separator();

                // 3. 3D Primitives
                if btn(ui, "Box", IconKind::BoxPrimitive, matches!(active_tool, Tool::DrawBoxPrimitive { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawBoxPrimitive { corner1: None });
                }
                if btn(ui, "Cylinder", IconKind::CylinderPrimitive, matches!(active_tool, Tool::DrawCylinderPrimitive { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawCylinderPrimitive { center: None });
                }
                if btn(ui, "Sphere", IconKind::SpherePrimitive, matches!(active_tool, Tool::DrawSpherePrimitive { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawSpherePrimitive { center: None });
                }
            }

            RibbonTab::Draw => {
                if btn(ui, "Line", IconKind::DetailLine, matches!(active_tool, Tool::DrawDetailLine { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawDetailLine { start: None });
                }
                if btn(ui, "Dimension", IconKind::Dimension, matches!(active_tool, Tool::DrawDimension { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawDimension { start: None, p2: None, locked_offset: None });
                }
                if btn(ui, "Grid", IconKind::Grid, matches!(active_tool, Tool::DrawGrid { .. })).clicked() {
                    action.new_tool = Some(Tool::DrawGrid { start: None });
                }
            }

            RibbonTab::Modify => {
                if btn(ui, "Select", IconKind::Select, matches!(active_tool, Tool::Select)).clicked() {
                    action.new_tool = Some(Tool::Select);
                }
                if btn(ui, "Move", IconKind::Move, matches!(active_tool, Tool::Move { .. })).clicked() {
                    action.new_tool = Some(Tool::Move { from: None });
                }
                if btn(ui, "Rotate", IconKind::Rotate, matches!(active_tool, Tool::Rotate { .. })).clicked() {
                    action.new_tool = Some(Tool::Rotate { center: None, start_angle: None });
                }
                if btn(ui, "Mirror", IconKind::Mirror, matches!(active_tool, Tool::Mirror { .. })).clicked() {
                    action.new_tool = Some(Tool::Mirror { axis_p1: None });
                }
                if has_selection {
                    if btn(ui, "Delete", IconKind::Delete, false).clicked() {
                        action.delete_selected = true;
                    }
                }

                ui.separator();

                // Zoom Extents / Fit View in 3D and 2D
                if btn(ui, "Zoom Extents", IconKind::ZoomExtents, false)
                    .on_hover_text("Fit all model elements into active viewport (ZE / Home)")
                    .clicked()
                {
                    action.zoom_extents = true;
                }
            }

            RibbonTab::Annotation => {
                let mut config_btn = |ui: &mut Ui, label: &str, icon: IconKind, active: bool, tool: Tool, cat: ToolCategory, tip: &str| {
                    let resp = ribbon_configurable_tool_button(ui, label, icon, active, show_tool_labels, theme).on_hover_text(tip);
                    if resp.double_clicked() || resp.secondary_clicked() {
                        action.new_tool = Some(tool);
                        action.open_tool_config = Some(cat);
                    } else if resp.clicked() {
                        action.new_tool = Some(tool);
                    }
                };

                config_btn(
                    ui,
                    "Linear Dim",
                    IconKind::Dimension,
                    matches!(*active_tool, Tool::DrawDimension { .. }),
                    Tool::DrawDimension { start: None, p2: None, locked_offset: None },
                    ToolCategory::DimensionLinear,
                    "Linear Dimension between 2 points (D) • Double-click/R-click for Type Settings",
                );
                config_btn(
                    ui,
                    "Circular Dim",
                    IconKind::CircularDimension,
                    matches!(*active_tool, Tool::DrawCircularDimension { .. }),
                    Tool::DrawCircularDimension { center: None },
                    ToolCategory::DimensionCircular,
                    "Radius / Diameter Dimension on arc, circle, or curved wall • Double-click/R-click for Type Settings",
                );
                config_btn(
                    ui,
                    "Angular Dim",
                    IconKind::AngularDimension,
                    matches!(*active_tool, Tool::DrawAngularDimension { .. }),
                    Tool::DrawAngularDimension { apex: None, first_ray: None },
                    ToolCategory::DimensionAngular,
                    "Angle Dimension between 3 points or intersecting edges • Double-click/R-click for Type Settings",
                );
                config_btn(
                    ui,
                    "Text Note",
                    IconKind::TextAnnotation,
                    matches!(*active_tool, Tool::DrawTextAnnotation { .. }),
                    Tool::DrawTextAnnotation { pos: None },
                    ToolCategory::TextNote,
                    "Place 2D text annotation or note (TX) • Double-click/R-click for Type Settings",
                );
                config_btn(
                    ui,
                    "Leader",
                    IconKind::Leader,
                    matches!(*active_tool, Tool::DrawLeader { .. }),
                    Tool::DrawLeader { target_pt: None, shoulder_pt: None },
                    ToolCategory::Leader,
                    "Leader line with arrowhead and text callout (LD) • Double-click/R-click for Type Settings",
                );
                config_btn(
                    ui,
                    "Detail Line",
                    IconKind::DetailLine,
                    matches!(*active_tool, Tool::DrawDetailLine { .. }),
                    Tool::DrawDetailLine { start: None },
                    ToolCategory::DetailLine,
                    "2D Drafting detail line (DL) • Double-click/R-click for Type Settings",
                );

                ui.separator();

                // Plan View Drawing Scale Control
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("View Scale:").size(10.5).strong().color(theme.accent));
                        let cur_scale = annotation.view_scale.max(1);
                        let scale_text = if SCALE_PRESETS.contains(&cur_scale) {
                            format!("1:{}", cur_scale)
                        } else {
                            format!("1:{} (Custom)", cur_scale)
                        };

                        egui::ComboBox::from_id_salt("ribbon_view_scale_select")
                            .selected_text(egui::RichText::new(scale_text).size(11.0).strong().color(theme.text_primary))
                            .width(115.0)
                            .show_ui(ui, |ui| {
                                for &sc in SCALE_PRESETS {
                                    let desc = match sc {
                                        1 => "1:1 (Full Size)",
                                        2 => "1:2 (Half Size)",
                                        5 => "1:5",
                                        10 => "1:10 (Detail)",
                                        15 => "1:15",
                                        20 => "1:20 (Enlarged Detail)",
                                        25 => "1:25",
                                        30 => "1:30",
                                        40 => "1:40",
                                        50 => "1:50 (Detailed Plan)",
                                        75 => "1:75",
                                        100 => "1:100 (Standard Plan)",
                                        150 => "1:150",
                                        200 => "1:200 (Site / Overall)",
                                        300 => "1:300",
                                        400 => "1:400",
                                        500 => "1:500 (Master Plan)",
                                        1000 => "1:1000 (Site / Map)",
                                        _ => "",
                                    };
                                    let label = if !desc.is_empty() { desc } else { &format!("1:{}", sc) };
                                    if ui.selectable_value(&mut annotation.view_scale, sc, label).clicked() {
                                        action.set_view_scale = Some(sc);
                                    }
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Custom 1:").size(10.5));
                                    let mut val = annotation.custom_scale_entry.max(1);
                                    let resp = ui.add(egui::DragValue::new(&mut val).range(1..=50000).speed(1.0));
                                    if resp.changed() {
                                        annotation.custom_scale_entry = val;
                                        annotation.view_scale = val;
                                        action.set_view_scale = Some(val);
                                    }
                                });
                            });
                    });

                    let scale_badge = match annotation.view_scale.max(1) {
                        1..=20 => "🔍 Detailed / Interior Scale",
                        21..=75 => "📐 Large Architectural Plan",
                        76..=150 => "🏛 Standard General Arrangement",
                        151..=300 => "🏢 Building Complex / Overview",
                        _ => "🗺 Site & Cadastral Scale",
                    };
                    ui.label(egui::RichText::new(scale_badge).size(9.0).color(theme.text_muted));
                });

                ui.separator();

                // Dimension Style & Typography Customization Bar
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Tick:").size(10.5).color(theme.text_muted));
                        egui::ComboBox::from_id_salt("dim_tick_mark")
                            .selected_text(match annotation.tick_mark {
                                plinth_model::TickMark::Tick45 => "45° Slash",
                                plinth_model::TickMark::Arrow => "Arrow",
                                plinth_model::TickMark::Dot => "Filled Dot",
                                plinth_model::TickMark::OpenArrow => "Open Arrow",
                                plinth_model::TickMark::None => "None",
                            })
                            .width(85.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut annotation.tick_mark, plinth_model::TickMark::Tick45, "45° Slash");
                                ui.selectable_value(&mut annotation.tick_mark, plinth_model::TickMark::Arrow, "Arrow");
                                ui.selectable_value(&mut annotation.tick_mark, plinth_model::TickMark::Dot, "Filled Dot");
                                ui.selectable_value(&mut annotation.tick_mark, plinth_model::TickMark::OpenArrow, "Open Arrow");
                                ui.selectable_value(&mut annotation.tick_mark, plinth_model::TickMark::None, "None");
                            });

                        ui.separator();

                        ui.label(egui::RichText::new("Font:").size(10.5).color(theme.text_muted));
                        ui.add(egui::DragValue::new(&mut annotation.font_size).range(6.0..=36.0).suffix(" pt").speed(0.5));

                        egui::ComboBox::from_id_salt("dim_font_style")
                            .selected_text(match annotation.font_style {
                                plinth_model::FontStyle::Regular => "Regular",
                                plinth_model::FontStyle::Bold => "Bold",
                                plinth_model::FontStyle::Italic => "Italic",
                            })
                            .width(75.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut annotation.font_style, plinth_model::FontStyle::Regular, "Regular");
                                ui.selectable_value(&mut annotation.font_style, plinth_model::FontStyle::Bold, "Bold");
                                ui.selectable_value(&mut annotation.font_style, plinth_model::FontStyle::Italic, "Italic");
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Color:").size(10.5).color(theme.text_muted));
                        let mut c = Color32::from_rgba_premultiplied(
                            annotation.color[0],
                            annotation.color[1],
                            annotation.color[2],
                            annotation.color[3],
                        );
                        if ui.color_edit_button_srgba(&mut c).changed() {
                            annotation.color = [c.r(), c.g(), c.b(), c.a()];
                        }

                        // Preset color pills
                        let swatches = [
                            ([30, 30, 30, 255], "Black"),
                            ([0, 100, 220, 255], "Blue"),
                            ([220, 40, 40, 255], "Red"),
                            ([20, 150, 60, 255], "Green"),
                        ];
                        for (rgba, tooltip) in swatches {
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::click());
                            ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3]));
                            if resp.on_hover_text(tooltip).clicked() {
                                annotation.color = rgba;
                            }
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Text:").size(10.5).color(theme.text_muted));
                        ui.add(egui::TextEdit::singleline(&mut annotation.default_text).desired_width(75.0));
                    });
                });

                ui.separator();

                // Universal Snapping Overrides
                ui.vertical(|ui| {
                    ui.checkbox(&mut annotation.snap_to_edges, egui::RichText::new("Snap Edges").size(10.5));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut annotation.snap_to_centerlines, egui::RichText::new("Centerlines").size(10.5));
                        ui.checkbox(&mut annotation.snap_to_openings, egui::RichText::new("Openings").size(10.5));
                    });
                });
            }

            RibbonTab::View => {
                // Viewport Layout Split Tools (Single, Horiz, Vert, Triple, Quad)
                if btn(ui, "Single View", IconKind::SingleViewport, false).clicked() {
                    action.split_single = true;
                }
                if btn(ui, "Split Horiz", IconKind::SplitHorizontal, false).clicked() {
                    action.split_horizontal = true;
                }
                if btn(ui, "Split Vert", IconKind::SplitVertical, false).clicked() {
                    action.split_vertical = true;
                }
                if btn(ui, "Split Triple", IconKind::SplitTriple, false).clicked() {
                    action.split_triple = true;
                }
                if btn(ui, "Split 4-Way", IconKind::SplitQuad, false).clicked() {
                    action.split_quad = true;
                }

                ui.separator();

                // Sheet Creation & 3D Overview
                if btn(ui, "New Sheet", IconKind::NewSheet, false).clicked() {
                    action.new_sheet = true;
                }
                if btn(ui, "Export PDF", IconKind::ExportPdf, false).on_hover_text("Export Architectural Sheet Drawings to Vector PDF (Ctrl+P)").clicked() {
                    action.export_pdf = true;
                }
                if btn(ui, "Default 3D", IconKind::ThreeD, false).clicked() {
                    action.open_default_3d = true;
                }
            }

            RibbonTab::Manage => {
                if btn(ui, "Level Manager", IconKind::LevelManager, false)
                    .on_hover_text("Open Storey & Elevation Level Manager")
                    .clicked()
                {
                    action.open_level_manager = true;
                }
                if btn(ui, "Grid Manager", IconKind::GridManager, false)
                    .on_hover_text("Open Orthogonal & Custom Datum Grid Manager")
                    .clicked()
                {
                    action.open_grid_manager = true;
                }

                ui.separator();

                if btn(ui, "Copy Level", IconKind::CopyLevel, false)
                    .on_hover_text("Copy Selected Entity Categories on Active Level")
                    .clicked()
                {
                    action.open_copy_level = true;
                }
                if btn(ui, "Paste to Levels", IconKind::PasteLevel, false)
                    .on_hover_text("Paste Copied Level Entities to Other Storeys")
                    .clicked()
                {
                    action.open_paste_level = true;
                }

                ui.separator();

                if btn(ui, "Settings", IconKind::Settings, false).clicked() {
                    action.open_settings = true;
                }
                if btn(ui, "Plugins", IconKind::PluginManager, false).on_hover_text("Open Arch3D Plugin Manager (Install & Manage Extensions)").clicked() {
                    action.open_plugin_manager = true;
                }

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Datum, Storey Replication & Settings").size(11.0).strong().color(theme.text_primary));
                        ui.label(egui::RichText::new("•").size(10.0).color(theme.text_muted));
                        ui.hyperlink_to(
                            egui::RichText::new("▶ @OpenSourceArchitecture").size(10.5).color(Color32::from_rgb(255, 90, 90)),
                            "https://www.youtube.com/@OpenSourceArchitecture",
                        );
                    });
                    ui.label(egui::RichText::new("Level & Grid Managers • Multi-Storey Copy/Paste • 1:1 mm IFC4").size(10.0).color(theme.text_muted));
                });
            }

            RibbonTab::Insert => {
                if btn(ui, "Export IFC4", IconKind::IfcExport, false).clicked() {
                    action.export_ifc = true;
                }
                if btn(ui, "Export GLB", IconKind::ExportGlb, false).on_hover_text("Export 3D Model to glTF Binary (.glb) for MuTracer / WebGPU / Three.js Path Tracing").clicked() {
                    action.export_glb = true;
                }
                if mutracer_active {
                    if btn(ui, "Render", IconKind::RenderMuTracer, false).on_hover_text("Open Live MuTracer WebGPU Path Tracer Studio").clicked() {
                        action.trigger_render_mutracer = true;
                    }
                }
                if blender_active {
                    if btn(ui, "Sync Blender", IconKind::BlenderLiveSync, false).on_hover_text("Live Sync 3D BIM model directly into Blender Cycles/EEVEE (port 9876)").clicked() {
                        action.trigger_sync_blender = true;
                    }
                }
                if btn(ui, "Import IFC4", IconKind::IfcImport, false).clicked() {
                    action.import_ifc = true;
                }
                if btn(ui, "Import SKP", IconKind::ImportSkp, false).clicked() {
                    action.import_skp = true;
                }
                if btn(ui, "Import DWG", IconKind::ImportDwg, false).clicked() {
                    action.import_dwg = true;
                }
                if btn(ui, "Sample Model", IconKind::ThreeD, false).clicked() {
                    action.load_sample = true;
                }
                if btn(ui, "Apartment (IFC)", IconKind::ThreeD, false).clicked() {
                    action.load_apartment_highrise = true;
                }
            }
        }
    });

    action
}

fn ribbon_tool_button(
    ui: &mut Ui,
    label: &str,
    icon: IconKind,
    active: bool,
    show_label: bool,
    theme: &AppTheme,
) -> Response {
    let button_size = if show_label {
        let min_w = (label.chars().count() as f32 * 5.6 + 10.0).max(48.0);
        Vec2::new(min_w, 48.0)
    } else {
        Vec2::new(32.0, 32.0)
    };
    let (rect, response) = ui.allocate_exact_size(button_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Background
        let bg_color = if active {
            theme.accent.gamma_multiply(0.25)
        } else if response.hovered() {
            theme.hover_highlight
        } else {
            Color32::TRANSPARENT
        };

        if bg_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 4.0, bg_color);
        }

        if active {
            painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.5, theme.accent), egui::StrokeKind::Middle);
        }

        let icon_color = if active {
            theme.accent
        } else {
            theme.text_primary
        };

        if show_label {
            // Draw Icon upper
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.min.y + 16.0),
                Vec2::new(20.0, 20.0),
            );
            draw_icon(painter, icon_rect, icon, icon_color);

            // Label lower
            painter.text(
                egui::pos2(rect.center().x, rect.max.y - 8.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(9.5),
                theme.text_primary,
            );
        } else {
            // Center icon only (compact mode)
            let icon_rect = egui::Rect::from_center_size(
                rect.center(),
                Vec2::new(20.0, 20.0),
            );
            draw_icon(painter, icon_rect, icon, icon_color);
        }
    }

    response.on_hover_text(label)
}

fn ribbon_configurable_tool_button(
    ui: &mut Ui,
    label: &str,
    icon: IconKind,
    active: bool,
    show_label: bool,
    theme: &AppTheme,
) -> Response {
    let button_size = if show_label {
        let min_w = (label.chars().count() as f32 * 5.6 + 10.0).max(48.0);
        Vec2::new(min_w, 48.0)
    } else {
        Vec2::new(32.0, 32.0)
    };
    let (rect, response) = ui.allocate_exact_size(button_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        let bg_color = if active {
            theme.accent.gamma_multiply(0.25)
        } else if response.hovered() {
            theme.hover_highlight
        } else {
            Color32::TRANSPARENT
        };

        if bg_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 4.0, bg_color);
        }

        if active {
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.5, theme.accent),
                egui::StrokeKind::Middle,
            );
        }

        let icon_color = if active {
            theme.accent
        } else {
            theme.text_primary
        };

        if show_label {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.min.y + 16.0),
                Vec2::new(20.0, 20.0),
            );
            draw_icon(painter, icon_rect, icon, icon_color);

            painter.text(
                egui::pos2(rect.center().x, rect.max.y - 8.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(9.5),
                theme.text_primary,
            );
        } else {
            let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(20.0, 20.0));
            draw_icon(painter, icon_rect, icon, icon_color);
        }

        // Subtle studio corner triangle indicator (shows double-click Type Studio capability)
        let tri_col = if active || response.hovered() {
            theme.accent
        } else {
            theme.text_muted.gamma_multiply(0.6)
        };
        let p_tr = egui::pos2(rect.max.x - 3.0, rect.min.y + 3.0);
        painter.add(egui::Shape::convex_polygon(
            vec![
                p_tr,
                egui::pos2(p_tr.x, p_tr.y + 5.0),
                egui::pos2(p_tr.x - 5.0, p_tr.y),
            ],
            tri_col,
            egui::Stroke::NONE,
        ));
    }

    response.on_hover_text(format!(
        "{}\n• Left-click: Activate {} tool\n• Right-click or Double-click: Open {} Settings & Type Studio",
        label, label, label
    ))
}

/// Compact icon-only button for the Quick Access Toolbar (Undo, Redo, 3D).
fn quick_icon_button(
    ui: &mut Ui,
    icon: IconKind,
    tooltip: &str,
    theme: &AppTheme,
) -> Response {
    let size = Vec2::new(24.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        let bg_color = if response.is_pointer_button_down_on() {
            theme.accent.gamma_multiply(0.35)
        } else if response.hovered() {
            theme.hover_highlight
        } else {
            Color32::TRANSPARENT
        };

        if bg_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 3.0, bg_color);
        }

        let icon_rect = Rect::from_center_size(rect.center(), Vec2::new(14.0, 14.0));
        let icon_color = if response.hovered() {
            theme.accent
        } else {
            theme.text_primary
        };

        draw_icon(painter, icon_rect, icon, icon_color);
    }

    response.on_hover_text(tooltip)
}
