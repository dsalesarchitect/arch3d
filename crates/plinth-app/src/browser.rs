//! Project Tree panel (Views, Layout Sheets, Storey Levels, Component Blocks).

use crate::icons::{draw_icon, IconKind};
use crate::theme::AppTheme;
use egui::{CollapsingHeader, RichText, ScrollArea, Stroke, Ui, Vec2};
use plinth_model::{Document, SheetId, ViewDef, ViewId, ViewKind};

#[derive(Default)]
pub struct BrowserResponse {
    pub activated_view: Option<ViewId>,
    pub activated_sheet: Option<SheetId>,
    pub new_sheet: bool,
    pub add_viewport_to_sheet: Option<SheetId>,
    pub open_level_manager: bool,
    pub filter_by_block_category: Option<String>,
}

/// Project Tree state stored across frames in egui temp data
#[derive(Clone, Default)]
pub struct ProjectTreeState {
    pub search_query: String,
}

pub fn render_project_browser(
    ui: &mut Ui,
    doc: &mut Document,
    theme: &AppTheme,
) -> BrowserResponse {
    let mut response = BrowserResponse::default();
    let state_id = ui.id().with("project_tree_state");
    let mut state: ProjectTreeState = ui.data_mut(|d| d.get_temp(state_id).unwrap_or_default());

    // --- Header: Title + Procedural Tree Icon ---
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
        draw_icon(ui.painter(), icon_rect, IconKind::ProjectTree, theme.accent);
        ui.add_space(2.0);
        ui.label(RichText::new("Project Tree").strong().size(13.0).color(theme.text_primary));
    });
    ui.add_space(4.0);

    // --- Quick Search / Filter Bar ---
    ui.horizontal(|ui| {
        let (search_icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
        draw_icon(ui.painter(), search_icon_rect, IconKind::SearchFilter, theme.text_muted);
        ui.add_space(2.0);

        let has_query = !state.search_query.is_empty();
        let target_width = ui.available_width() - if has_query { 20.0 } else { 0.0 };
        let search_box = egui::TextEdit::singleline(&mut state.search_query)
            .hint_text("Search views, levels...")
            .desired_width(target_width);
        ui.add(search_box);

        if has_query && ui.small_button("✕").clicked() {
            state.search_query.clear();
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    let query = state.search_query.trim().to_lowercase();
    let is_filtering = !query.is_empty();

    ScrollArea::vertical().show(ui, |ui| {
        // -------------------------------------------------------------
        // 1. Views Section
        // -------------------------------------------------------------
        let total_views = doc.views.len();
        let views_header_text = format!("Views ({})", total_views);

        CollapsingHeader::new(RichText::new(views_header_text).strong().size(11.5))
            .default_open(true)
            .show(ui, |ui| {
                // --- Floor Plans ---
                let floor_plans: Vec<_> = doc
                    .views
                    .values()
                    .filter(|v| v.kind == ViewKind::FloorPlan)
                    .filter(|v| !is_filtering || v.name.to_lowercase().contains(&query))
                    .collect();

                if !floor_plans.is_empty() || !is_filtering {
                    let fp_title = format!("Floor Plans ({})", floor_plans.len());
                    CollapsingHeader::new(RichText::new(fp_title).size(11.0))
                        .default_open(true)
                        .show(ui, |ui| {
                            for v in floor_plans {
                                let is_active = doc.active_view_id == v.id && doc.active_sheet_id.is_none();
                                if view_tree_row(ui, v, IconKind::FloorPlanView, is_active, theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        });
                }

                // --- 3D Axonometric Views ---
                let three_d_views: Vec<_> = doc
                    .views
                    .values()
                    .filter(|v| v.kind == ViewKind::ThreeD)
                    .filter(|v| !is_filtering || v.name.to_lowercase().contains(&query))
                    .collect();

                if !three_d_views.is_empty() || !is_filtering {
                    let threed_title = format!("3D Views ({})", three_d_views.len());
                    CollapsingHeader::new(RichText::new(threed_title).size(11.0))
                        .default_open(true)
                        .show(ui, |ui| {
                            for v in three_d_views {
                                let is_active = doc.active_view_id == v.id && doc.active_sheet_id.is_none();
                                if view_tree_row(ui, v, IconKind::ThreeD, is_active, theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        });
                }

                // --- Elevations ---
                let elevations: Vec<_> = doc
                    .views
                    .values()
                    .filter(|v| v.kind.is_elevation_or_section() && v.kind != ViewKind::Section)
                    .filter(|v| !is_filtering || v.name.to_lowercase().contains(&query))
                    .collect();

                if !elevations.is_empty() || !is_filtering {
                    let elev_title = format!("Elevations ({})", elevations.len());
                    CollapsingHeader::new(RichText::new(elev_title).size(11.0))
                        .default_open(true)
                        .show(ui, |ui| {
                            for v in elevations {
                                let is_active = doc.active_view_id == v.id && doc.active_sheet_id.is_none();
                                if view_tree_row(ui, v, IconKind::ElevationView, is_active, theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        });
                }

                // --- Building Sections ---
                let sections: Vec<_> = doc
                    .views
                    .values()
                    .filter(|v| v.kind == ViewKind::Section)
                    .filter(|v| !is_filtering || v.name.to_lowercase().contains(&query))
                    .collect();

                if !sections.is_empty() || !is_filtering {
                    let sec_title = format!("Sections ({})", sections.len());
                    CollapsingHeader::new(RichText::new(sec_title).size(11.0))
                        .default_open(!sections.is_empty())
                        .show(ui, |ui| {
                            for v in sections {
                                let is_active = doc.active_view_id == v.id && doc.active_sheet_id.is_none();
                                if view_tree_row(ui, v, IconKind::SectionView, is_active, theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        });
                }

                // --- Ceiling Plans ---
                let ceiling_plans: Vec<_> = doc
                    .views
                    .values()
                    .filter(|v| v.kind == ViewKind::CeilingPlan)
                    .filter(|v| !is_filtering || v.name.to_lowercase().contains(&query))
                    .collect();

                if !ceiling_plans.is_empty() {
                    let cp_title = format!("Ceiling Plans ({})", ceiling_plans.len());
                    CollapsingHeader::new(RichText::new(cp_title).size(11.0))
                        .default_open(false)
                        .show(ui, |ui| {
                            for v in ceiling_plans {
                                let is_active = doc.active_view_id == v.id && doc.active_sheet_id.is_none();
                                if view_tree_row(ui, v, IconKind::Ceiling, is_active, theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        });
                }
            });

        ui.add_space(4.0);

        // -------------------------------------------------------------
        // 2. Sheets Section (Layouts & Viewports)
        // -------------------------------------------------------------
        let sheets_count = doc.sheets.len();
        let sheets_header = format!("Sheets ({})", sheets_count);

        CollapsingHeader::new(RichText::new(sheets_header).strong().size(11.5))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (plus_icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                    draw_icon(ui.painter(), plus_icon_rect, IconKind::NewSheet, theme.accent);
                    if ui.button("+ New Sheet").clicked() {
                        response.new_sheet = true;
                    }
                });
                ui.add_space(2.0);

                for sheet in doc.sheets.values() {
                    let is_active_sheet = doc.active_sheet_id == Some(sheet.id);
                    let sheet_label = format!("{} • {}", sheet.sheet_number, sheet.name);

                    ui.horizontal(|ui| {
                        let (s_icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                        let s_icon_color = if is_active_sheet { theme.accent } else { theme.text_muted };
                        draw_icon(ui.painter(), s_icon_rect, IconKind::Sheet, s_icon_color);

                        let sel = ui.selectable_label(
                            is_active_sheet,
                            if is_active_sheet {
                                RichText::new(&sheet_label).strong().color(theme.accent)
                            } else {
                                RichText::new(&sheet_label)
                            },
                        );
                        if sel.clicked() {
                            response.activated_sheet = Some(sheet.id);
                        }
                    });

                    // Placed viewports on sheet
                    if is_active_sheet || !sheet.viewports.is_empty() {
                        ui.indent(format!("sheet_vp_{}", sheet.id), |ui| {
                            for vp in &sheet.viewports {
                                let vp_text = format!("{}. {} (1:{})", vp.number, vp.title, vp.scale);
                                ui.label(RichText::new(vp_text).size(10.5).color(theme.text_muted));
                            }
                            if ui.small_button("+ Place Viewport").clicked() {
                                response.add_viewport_to_sheet = Some(sheet.id);
                            }
                        });
                    }
                }
            });

        ui.add_space(4.0);

        // -------------------------------------------------------------
        // 3. Storeys (Levels) Section
        // -------------------------------------------------------------
        let levels_count = doc.levels.len();
        let levels_header = format!("Storeys ({})", levels_count);

        CollapsingHeader::new(RichText::new(levels_header).strong().size(11.5))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (lvl_icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                    draw_icon(ui.painter(), lvl_icon_rect, IconKind::LevelManager, theme.accent);
                    if ui.small_button("⚙ Level Manager...").clicked() {
                        response.open_level_manager = true;
                    }
                });
                ui.add_space(2.0);

                let mut sorted_levels: Vec<_> = doc.levels.values().collect();
                sorted_levels.sort_by(|a, b| b.elevation.partial_cmp(&a.elevation).unwrap_or(std::cmp::Ordering::Equal));

                for lvl in sorted_levels {
                    if is_filtering && !lvl.name.to_lowercase().contains(&query) {
                        continue;
                    }
                    let is_active = doc.active_level_id == lvl.id;
                    let elev_str = format!("{:+.0} mm", lvl.elevation);

                    ui.horizontal(|ui| {
                        let text = if is_active {
                            RichText::new(&lvl.name).strong().color(theme.accent)
                        } else {
                            RichText::new(&lvl.name)
                        };

                        if ui.selectable_label(is_active, text).clicked() {
                            doc.active_level_id = lvl.id;
                        }

                        // Right-aligned clean elevation datum pill
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(elev_str).size(10.0).color(theme.text_muted));
                        });
                    });
                }
            });

        ui.add_space(4.0);

        // -------------------------------------------------------------
        // 4. Component Blocks (Formerly Families)
        // -------------------------------------------------------------
        let total_elements = doc.elements.len();
        let blocks_header = format!("Blocks ({} Elements)", total_elements);

        CollapsingHeader::new(RichText::new(blocks_header).strong().size(11.5))
            .default_open(true)
            .show(ui, |ui| {
                // Tally active elements in project by category
                let mut walls = 0;
                let mut columns = 0;
                let mut beams = 0;
                let mut slabs = 0;
                let mut curtain_walls = 0;
                let mut doors = 0;
                let mut windows = 0;
                let mut roofs = 0;
                let mut footings = 0;
                let mut stairs_ramps = 0;
                let mut freeforms = 0;

                for el in doc.elements.values() {
                    match el {
                        plinth_model::Element::Wall(_) => walls += 1,
                        plinth_model::Element::Column(_) => columns += 1,
                        plinth_model::Element::Beam(_) => beams += 1,
                        plinth_model::Element::Slab(_) => slabs += 1,
                        plinth_model::Element::CurtainWall(_) => curtain_walls += 1,
                        plinth_model::Element::Door(_) => doors += 1,
                        plinth_model::Element::Window(_) => windows += 1,
                        plinth_model::Element::Roof(_) => roofs += 1,
                        plinth_model::Element::Footing(_) => footings += 1,
                        plinth_model::Element::Stair(_) | plinth_model::Element::Ramp(_) => stairs_ramps += 1,
                        plinth_model::Element::FreeformSolid(_) => freeforms += 1,
                        _ => {}
                    }
                }

                // Render clean block items with procedural vector icons and live instance counters
                block_category_row(ui, "Walls", walls, IconKind::Wall, theme);
                block_category_row(ui, "Columns", columns, IconKind::Column, theme);
                block_category_row(ui, "Beams (Framing)", beams, IconKind::Beam, theme);
                block_category_row(ui, "Slabs (Floors)", slabs, IconKind::Slab, theme);
                block_category_row(ui, "Curtain Walls", curtain_walls, IconKind::CurtainWall, theme);
                block_category_row(ui, "Doors", doors, IconKind::Door, theme);
                block_category_row(ui, "Windows", windows, IconKind::Window, theme);
                block_category_row(ui, "Roofs", roofs, IconKind::Roof, theme);
                block_category_row(ui, "Footings", footings, IconKind::Footing, theme);
                if stairs_ramps > 0 {
                    block_category_row(ui, "Stairs & Ramps", stairs_ramps, IconKind::Stair, theme);
                }
                if freeforms > 0 {
                    block_category_row(ui, "Freeform Solids", freeforms, IconKind::BoxPrimitive, theme);
                }
            });
    });

    // Save updated state back into egui temp storage
    ui.data_mut(|d| d.insert_temp(state_id, state));

    response
}

/// Helper to render a tree view row with a procedural vector icon
fn view_tree_row(ui: &mut Ui, view: &ViewDef, icon: IconKind, is_active: bool, theme: &AppTheme) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
        let icon_col = if is_active { theme.accent } else { theme.text_muted };
        draw_icon(ui.painter(), icon_rect, icon, icon_col);

        let label_text = if is_active {
            RichText::new(&view.name).strong().color(theme.accent)
        } else {
            RichText::new(&view.name)
        };

        if ui.selectable_label(is_active, label_text).clicked() {
            clicked = true;
        }

        // Active indicator dot
        if is_active {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("●").size(8.0).color(theme.accent));
            });
        }
    });
    clicked
}

/// Helper to render a Component Block row with vector icon and instance count badge
fn block_category_row(ui: &mut Ui, name: &str, count: usize, icon: IconKind, theme: &AppTheme) {
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
        let icon_col = if count > 0 { theme.text_primary } else { theme.text_muted.gamma_multiply(0.6) };
        draw_icon(ui.painter(), icon_rect, icon, icon_col);

        let text_color = if count > 0 { theme.text_primary } else { theme.text_muted };
        ui.label(RichText::new(name).size(11.0).color(text_color));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let count_str = format!("{}", count);
            let pill_rect = ui.label(RichText::new(&count_str).size(9.5).color(theme.text_muted)).rect;
            ui.painter().rect_stroke(
                pill_rect.expand(2.0),
                2.0,
                Stroke::new(1.0, theme.text_muted.gamma_multiply(0.25)),
                egui::StrokeKind::Middle,
            );
        });
    });
}
