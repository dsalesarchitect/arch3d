//! Project Browser tree panel (Views, Sheets, Levels, Families).

use crate::theme::AppTheme;
use egui::{CollapsingHeader, ScrollArea, Ui};
use plinth_model::{Document, SheetId, ViewDef, ViewId, ViewKind};

#[derive(Default)]
pub struct BrowserResponse {
    pub activated_view: Option<ViewId>,
    pub activated_sheet: Option<SheetId>,
    pub new_sheet: bool,
    pub add_viewport_to_sheet: Option<SheetId>,
    pub open_level_manager: bool,
}

pub fn render_project_browser(
    ui: &mut Ui,
    doc: &mut Document,
    theme: &AppTheme,
) -> BrowserResponse {
    let mut response = BrowserResponse::default();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Project Browser").strong().size(12.0));
    });
    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        // 1. Views Tree
        CollapsingHeader::new(egui::RichText::new("Views (all)").strong())
            .default_open(true)
            .show(ui, |ui| {
                // Floor Plans
                CollapsingHeader::new("Floor Plans")
                    .default_open(true)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind == ViewKind::FloorPlan {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });

                // Ceiling Plans
                CollapsingHeader::new("Ceiling Plans")
                    .default_open(false)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind == ViewKind::CeilingPlan {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });

                // 3D Views
                CollapsingHeader::new("3D Views")
                    .default_open(true)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind == ViewKind::ThreeD {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });

                // Elevations
                CollapsingHeader::new("Elevations (Building Elevation)")
                    .default_open(true)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind.is_elevation_or_section() && v.kind != ViewKind::Section {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });

                // Sections
                CollapsingHeader::new("Sections (Building Section)")
                    .default_open(true)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind == ViewKind::Section {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });

                // Details
                CollapsingHeader::new("Detail Views")
                    .default_open(false)
                    .show(ui, |ui| {
                        for v in doc.views.values() {
                            if v.kind == ViewKind::Detail {
                                if view_item(ui, v, doc.active_view_id == v.id && doc.active_sheet_id.is_none(), theme) {
                                    response.activated_view = Some(v.id);
                                }
                            }
                        }
                    });
            });

        ui.add_space(4.0);

        // 2. Sheets Management Section (Multi-Sheet Layout Grid)
        CollapsingHeader::new(egui::RichText::new("Sheets (all)").strong())
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("+ New Sheet").clicked() {
                        response.new_sheet = true;
                    }
                });

                for sheet in doc.sheets.values() {
                    let is_active_sheet = doc.active_sheet_id == Some(sheet.id);
                    let sheet_label = format!("📄 {} - {}", sheet.sheet_number, sheet.name);
                    let sel = ui.selectable_label(
                        is_active_sheet,
                        if is_active_sheet {
                            egui::RichText::new(&sheet_label).strong().color(theme.accent)
                        } else {
                            egui::RichText::new(&sheet_label)
                        },
                    );
                    if sel.clicked() {
                        response.activated_sheet = Some(sheet.id);
                    }

                    // Show viewports placed on this sheet
                    if is_active_sheet || !sheet.viewports.is_empty() {
                        ui.indent(format!("sheet_vp_{}", sheet.id), |ui| {
                            for vp in &sheet.viewports {
                                let vp_text = format!("{}. {} (1:{})", vp.number, vp.title, vp.scale);
                                ui.label(egui::RichText::new(vp_text).size(10.5).color(theme.text_muted));
                            }
                            if ui.small_button("+ Place Viewport").clicked() {
                                response.add_viewport_to_sheet = Some(sheet.id);
                            }
                        });
                    }
                }
            });

        ui.add_space(4.0);

        // 3. Levels tree
        CollapsingHeader::new(egui::RichText::new("Levels (Storeys)").strong())
            .default_open(false)
            .show(ui, |ui| {
                if ui.small_button("⚙ Level Manager...").clicked() {
                    response.open_level_manager = true;
                }
                for lvl in doc.levels.values() {
                    let is_active = doc.active_level_id == lvl.id;
                    let text = format!("{} ({:.0} mm)", lvl.name, lvl.elevation);
                    if ui.selectable_label(is_active, text).clicked() {
                        doc.active_level_id = lvl.id;
                    }
                }
            });

        // 4. Families / Categories
        CollapsingHeader::new(egui::RichText::new("Families").strong())
            .default_open(false)
            .show(ui, |ui| {
                ui.label("• Basic Wall");
                ui.label("• Rectangular Column");
                ui.label("• Isolated Foundation Footing");
                ui.label("• Structural Framing (Beams)");
                ui.label("• Floor Slab");
                ui.label("• Basic Roof");
                ui.label("• Fascia Board");
                ui.label("• Eave Gutter");
                ui.label("• Compound Ceiling");
                ui.label("• Single-Flush Door");
                ui.label("• Fixed Window");
                ui.label("• Planar Surfaces (Freeform)");
                ui.label("• 3D Solids (Push/Pull)");
            });
    });

    response
}

fn view_item(ui: &mut Ui, view: &ViewDef, is_active: bool, theme: &AppTheme) -> bool {
    let text = if is_active {
        egui::RichText::new(&view.name).strong().color(theme.accent)
    } else {
        egui::RichText::new(&view.name)
    };
    ui.selectable_label(is_active, text).clicked()
}
