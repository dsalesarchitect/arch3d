//! Properties panel (Element Parameters, Level Hosting & Offsets, IFC4 Property Sets,
//! View Parameters, and Sheet Viewport Management).

use crate::theme::AppTheme;
use crate::tool_config::ToolCategory;
use egui::{Color32, ComboBox, DragValue, RichText, ScrollArea, Ui};
use plinth_model::{
    AssemblyLayer, Document, Element, ElementId, FootingStyle, LevelId, PaperSize, RampStyle,
    StairStyle, ViewDef, ViewportId,
};

/// Compact CAD/BIM Property Grid with themed category headers,
/// crisp horizontal row lines, vertical column dividing lines,
/// and outer border gridlines matching professional architectural palette standards.
pub struct PropertyGrid<'a, 'b> {
    pub ui: &'b mut Ui,
    pub theme: &'a AppTheme,
    pub avail_w: f32,
    pub label_w: f32,
}

impl<'a, 'b> PropertyGrid<'a, 'b> {
    pub fn new(ui: &'b mut Ui, theme: &'a AppTheme) -> Self {
        let avail_w = ui.available_width();
        let label_w = (avail_w * 0.44).clamp(92.0, 155.0);
        Self {
            ui,
            theme,
            avail_w,
            label_w,
        }
    }

    pub fn grid_stroke(&self) -> egui::Stroke {
        let col = if self.theme.mode == crate::theme::ThemeMode::Dark {
            Color32::from_rgb(45, 55, 72)
        } else {
            Color32::from_rgb(215, 224, 235)
        };
        egui::Stroke::new(1.0, col)
    }

    /// Renders a 2-column property row with vertical divider and horizontal gridline.
    pub fn row<F: FnOnce(&mut Ui)>(&mut self, label: &str, add_value: F) {
        let grid_stroke = self.grid_stroke();
        let label_w = self.label_w;
        let theme = self.theme;

        let row_resp = self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

            // Column 1: Property Label
            ui.allocate_ui_with_layout(
                egui::Vec2::new(label_w, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(6.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(label)
                                .size(11.0)
                                .color(theme.text_muted),
                        )
                        .truncate(),
                    );
                },
            );

            // Column 2: Value / Control
            let val_w = ui.available_width().max(20.0);
            ui.allocate_ui_with_layout(
                egui::Vec2::new(val_w, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(6.0);
                    add_value(ui);
                },
            );
        });

        let rect = row_resp.response.rect;
        let painter = self.ui.painter();

        // Horizontal bottom gridline
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], grid_stroke);

        // Center vertical dividing gridline
        let split_x = rect.min.x + label_w;
        painter.line_segment(
            [egui::pos2(split_x, rect.min.y), egui::pos2(split_x, rect.max.y)],
            grid_stroke,
        );

        // Left and right outer vertical borders
        painter.line_segment([rect.left_top(), rect.left_bottom()], grid_stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], grid_stroke);
    }

    /// Read-only text row (e.g. "Length: 5200.0 mm")
    pub fn row_text(&mut self, label: &str, text: impl Into<String>) {
        let t = text.into();
        let theme = self.theme;
        self.row(label, |ui| {
            ui.label(RichText::new(t).size(11.0).color(theme.text_primary));
        });
    }

    /// Singleline string edit row (e.g. "Type Name", "Description", "Tag / Mark")
    pub fn row_edit(&mut self, label: &str, text: &mut String) {
        self.row(label, |ui| {
            ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
        });
    }

    /// Boolean checkbox row (e.g. "LoadBearing", "IsExternal")
    pub fn row_bool(&mut self, label: &str, val: &mut bool) {
        self.row(label, |ui| {
            let caption = if *val { "Yes" } else { "No" };
            ui.checkbox(val, caption);
        });
    }

    /// Numeric drag value row
    pub fn row_drag<Num: egui::emath::Numeric>(
        &mut self,
        label: &str,
        val: &mut Num,
        speed: impl Into<f64>,
        suffix: &str,
    ) {
        let sp = speed.into();
        self.row(label, |ui| {
            ui.add(DragValue::new(val).speed(sp).suffix(suffix));
        });
    }

    /// Full-width spanned row (e.g. notes, status, multi-item summaries)
    pub fn row_full<F: FnOnce(&mut Ui)>(&mut self, add_contents: F) {
        let grid_stroke = self.grid_stroke();
        let row_resp = self.ui.horizontal(|ui| {
            ui.add_space(6.0);
            add_contents(ui);
        });
        let rect = row_resp.response.rect;
        let painter = self.ui.painter();
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], grid_stroke);
        painter.line_segment([rect.left_top(), rect.left_bottom()], grid_stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], grid_stroke);
    }
}

/// Renders a CAD/BIM styled Property Category Header with themed background fill,
/// crisp border lines, and collapse/expand toggle arrow.
pub fn prop_category<R>(
    ui: &mut Ui,
    title: &str,
    id_salt: &str,
    default_open: bool,
    theme: &AppTheme,
    add_contents: impl FnOnce(&mut PropertyGrid) -> R,
) -> Option<R> {
    let id = ui.make_persistent_id(id_salt);
    let mut is_open = ui.data_mut(|d| d.get_temp::<bool>(id).unwrap_or(default_open));

    let avail_w = ui.available_width();
    let header_h = 24.0;
    let (rect, resp) = ui.allocate_exact_size(egui::Vec2::new(avail_w, header_h), egui::Sense::click());
    if resp.clicked() {
        is_open = !is_open;
        ui.data_mut(|d| d.insert_temp(id, is_open));
    }

    let painter = ui.painter();

    // Category background color (matches user's image soft-blue in light mode / subtle slate in dark mode)
    let header_bg = if theme.mode == crate::theme::ThemeMode::Dark {
        Color32::from_rgb(28, 38, 54)
    } else {
        Color32::from_rgb(222, 235, 247)
    };
    painter.rect_filled(rect, 0.0, header_bg);

    // Header top and bottom borders
    let border_col = if theme.mode == crate::theme::ThemeMode::Dark {
        Color32::from_rgb(48, 62, 82)
    } else {
        Color32::from_rgb(175, 198, 222)
    };
    let border_stroke = egui::Stroke::new(1.0, border_col);
    painter.line_segment([rect.left_top(), rect.right_top()], border_stroke);
    painter.line_segment([rect.left_bottom(), rect.right_bottom()], border_stroke);
    painter.line_segment([rect.left_top(), rect.left_bottom()], border_stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], border_stroke);

    // Title label
    let text_col = if theme.mode == crate::theme::ThemeMode::Dark {
        Color32::from_rgb(215, 230, 252)
    } else {
        Color32::from_rgb(22, 42, 70)
    };
    painter.text(
        egui::pos2(rect.min.x + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(11.5),
        text_col,
    );

    // Chevron arrow on the right
    let arrow_str = if is_open { "˄" } else { "˅" };
    painter.text(
        egui::pos2(rect.max.x - 8.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        arrow_str,
        egui::FontId::proportional(11.0),
        if theme.mode == crate::theme::ThemeMode::Dark {
            Color32::from_rgb(140, 160, 190)
        } else {
            Color32::from_rgb(90, 115, 145)
        },
    );

    if is_open {
        let mut grid = PropertyGrid::new(ui, theme);
        let res = add_contents(&mut grid);
        ui.add_space(4.0);
        Some(res)
    } else {
        ui.add_space(3.0);
        None
    }
}

/// Helper to render Identity Data (Type Name, Tag / Mark, Description) in property grid
pub fn render_identity_data(
    grid: &mut PropertyGrid,
    type_name: &mut String,
    tag: &mut String,
    description: &mut String,
) {
    grid.row_edit("Type Name", type_name);
    grid.row_edit("Tag / Mark", tag);
    grid.row_edit("Description", description);
}

pub fn render_properties_panel(
    ui: &mut Ui,
    doc: &mut Document,
    selected_element_id: Option<ElementId>,
    selected_viewport_id: &mut Option<ViewportId>,
    theme: &AppTheme,
) -> Option<ToolCategory> {
    let mut open_studio_for: Option<ToolCategory> = None;

    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 16.0), egui::Sense::hover());
        crate::icons::draw_icon(ui.painter(), icon_rect, crate::icons::IconKind::Settings, theme.accent);
        ui.add_space(2.0);
        ui.label(RichText::new("Properties").strong().size(13.0).color(theme.text_primary));
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    // Pre-collect sorted project levels so we can edit elements mutably without borrow conflicts
    let mut sorted_levels: Vec<(LevelId, String, f64)> = doc
        .levels
        .values()
        .map(|l| (l.id, l.name.clone(), l.elevation))
        .collect();
    sorted_levels.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    // Pre-compute host wall base elevation for Door / Window if selected
    let host_wall_base_z = selected_element_id
        .and_then(|id| doc.elements.get(&id))
        .and_then(|el| match el {
            Element::Door(d) => Some(d.host_wall),
            Element::Window(w) => Some(w.host_wall),
            Element::CurtainWall(cw) => cw.host_wall,
            _ => None,
        })
        .and_then(|wid| match doc.elements.get(&wid) {
            Some(Element::Wall(hw)) => Some(doc.level_elevation(hw.base_level) + hw.base_offset),
            _ => None,
        })
        .unwrap_or(0.0);

    ScrollArea::vertical().show(ui, |ui| {
        if let Some(el_id) = selected_element_id {
            if let Some(element) = doc.elements.get_mut(&el_id) {
                open_studio_for = render_element_properties(
                    ui,
                    element,
                    &sorted_levels,
                    host_wall_base_z,
                    theme,
                );
                return;
            }
        }

        // Sheet Properties if active sheet is open
        if let Some(sheet_id) = doc.active_sheet_id {
            if let Some(sheet) = doc.sheets.get_mut(&sheet_id) {
                ui.group(|ui| {
                    ui.label(RichText::new("Sheet Layout Properties").size(11.0).strong());
                    ui.label(
                        RichText::new(format!("{} - {}", sheet.sheet_number, sheet.name))
                            .size(13.0)
                            .strong(),
                    );
                });

                ui.add_space(4.0);

                prop_category(ui, "Identity Data", "sheet_identity", true, theme, |grid| {
                    grid.row_edit("Sheet Number", &mut sheet.sheet_number);
                    grid.row_edit("Sheet Name", &mut sheet.name);
                });

                prop_category(ui, "Sheet Paper Format", "sheet_paper", true, theme, |grid| {
                    let (pw, ph) = sheet.paper_size.dimensions_mm();
                    grid.row("Paper Size", |ui| {
                        ComboBox::from_id_salt("paper_size_select")
                            .selected_text(sheet.paper_size.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut sheet.paper_size,
                                    PaperSize::A1,
                                    PaperSize::A1.name(),
                                );
                                ui.selectable_value(
                                    &mut sheet.paper_size,
                                    PaperSize::A2,
                                    PaperSize::A2.name(),
                                );
                                ui.selectable_value(
                                    &mut sheet.paper_size,
                                    PaperSize::A3,
                                    PaperSize::A3.name(),
                                );
                                ui.selectable_value(
                                    &mut sheet.paper_size,
                                    PaperSize::ArchD,
                                    PaperSize::ArchD.name(),
                                );
                            });
                    });
                    grid.row_text("Dimensions", format!("{:.1} × {:.1} mm", pw, ph));
                });

                prop_category(ui, "Placed Viewports (Drag on Sheet to Move)", "sheet_viewports", true, theme, |grid| {
                    if sheet.viewports.is_empty() {
                        grid.row_full(|ui| {
                            ui.label(
                                "No viewports placed on this sheet yet. Click '+ Place Viewport' in Project Tree.",
                            );
                        });
                    }
                    let mut remove_vp_idx: Option<usize> = None;
                    for (idx, vp) in sheet.viewports.iter_mut().enumerate() {
                        let is_sel = *selected_viewport_id == Some(vp.id);
                        grid.row_full(|ui| {
                            let frame = egui::Frame::group(ui.style()).stroke(egui::Stroke::new(
                                if is_sel { 1.5 } else { 1.0 },
                                if is_sel {
                                    theme.accent
                                } else {
                                    theme.border_color
                                },
                            ));
                            frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(is_sel, format!("VP #{}:", idx + 1))
                                        .clicked()
                                    {
                                        *selected_viewport_id = Some(vp.id);
                                    }
                                    ui.text_edit_singleline(&mut vp.title);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Detail No:");
                                    ui.add(DragValue::new(&mut vp.number).speed(1));
                                    ui.label("Scale 1:");
                                    ui.add(DragValue::new(&mut vp.scale).speed(5).range(1..=2000));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Pos X:");
                                    ui.add(
                                        DragValue::new(&mut vp.position_mm.x)
                                            .speed(2.0)
                                            .suffix(" mm"),
                                    );
                                    ui.label("Pos Y:");
                                    ui.add(
                                        DragValue::new(&mut vp.position_mm.y)
                                            .speed(2.0)
                                            .suffix(" mm"),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Width:");
                                    ui.add(
                                        DragValue::new(&mut vp.width_mm)
                                            .speed(2.0)
                                            .range(60.0..=1500.0)
                                            .suffix(" mm"),
                                    );
                                    ui.label("Height:");
                                    ui.add(
                                        DragValue::new(&mut vp.height_mm)
                                            .speed(2.0)
                                            .range(45.0..=1200.0)
                                            .suffix(" mm"),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut vp.show_title, "Show Title Bubble");
                                    if ui.small_button("🗑 Remove").clicked() {
                                        remove_vp_idx = Some(idx);
                                    }
                                });
                            });
                        });
                    }
                    if let Some(r_idx) = remove_vp_idx {
                        let removed = sheet.viewports.remove(r_idx);
                        if *selected_viewport_id == Some(removed.id) {
                            *selected_viewport_id = None;
                        }
                    }
                });
                return;
            }
        }

        // Otherwise show active View Properties
        if let Some(view) = doc.views.get_mut(&doc.active_view_id) {
            render_view_properties(ui, view, theme);
        }
    });

    open_studio_for
}

fn level_display_text(levels: &[(LevelId, String, f64)], id: LevelId) -> String {
    levels
        .iter()
        .find(|(lid, _, _)| *lid == id)
        .map(|(_, name, elev)| format!("{} ({:+.0} mm)", name, elev))
        .unwrap_or_else(|| "Level".to_string())
}

fn level_elev(levels: &[(LevelId, String, f64)], id: LevelId) -> f64 {
    levels
        .iter()
        .find(|(lid, _, _)| *lid == id)
        .map(|(_, _, elev)| *elev)
        .unwrap_or(0.0)
}

/// Unified Level-Hosting & Elevation Offsets section for standard level-hosted elements.
#[allow(clippy::too_many_arguments)]
fn render_level_hosting_constraints(
    ui: &mut Ui,
    salt: &str,
    levels: &[(LevelId, String, f64)],
    base_level: &mut LevelId,
    base_offset: &mut f64,
    base_offset_label: &str,
    top_level: &mut Option<LevelId>,
    top_offset: &mut f64,
    mut unconnected_height: Option<&mut f64>,
    fallback_span: f64,
    theme: &AppTheme,
) -> (f64, f64) {
    let mut z0 = 0.0;
    let mut z1 = 0.0;

    prop_category(
        ui,
        "Level & Elevation Constraints",
        &format!("lvl_constraints_{salt}"),
        true,
        theme,
        |grid| {
            // Base Level dropdown
            grid.row("Base Level", |ui| {
                ComboBox::from_id_salt(("base_lvl_cb", salt))
                    .selected_text(level_display_text(levels, *base_level))
                    .show_ui(ui, |ui| {
                        for (lid, name, elev) in levels {
                            let label = format!("{} ({:+.0} mm)", name, elev);
                            ui.selectable_value(base_level, *lid, label);
                        }
                    });
            });

            // Base Offset
            grid.row(base_offset_label, |ui| {
                ui.add(DragValue::new(base_offset).speed(10.0).suffix(" mm"));
            });

            // Top Level dropdown ("Unconnected" + all levels)
            let top_text = match *top_level {
                None => "Unconnected".to_string(),
                Some(tid) => level_display_text(levels, tid),
            };
            grid.row("Top Constraint", |ui| {
                ComboBox::from_id_salt(("top_lvl_cb", salt))
                    .selected_text(top_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(top_level, None, "Unconnected");
                        for (lid, name, elev) in levels {
                            let label = format!("Up to: {} ({:+.0} mm)", name, elev);
                            ui.selectable_value(top_level, Some(*lid), label);
                        }
                    });
            });

            // Top Offset
            grid.row("Top Offset", |ui| {
                ui.add(DragValue::new(top_offset).speed(10.0).suffix(" mm"));
            });

            // Unconnected Height (shown when Top Constraint is Unconnected)
            if top_level.is_none() {
                if let Some(ref mut uh) = unconnected_height {
                    grid.row("Unconnected H", |ui| {
                        ui.add(
                            DragValue::new(*uh)
                                .speed(25.0)
                                .range(50.0..=50000.0)
                                .suffix(" mm"),
                        );
                    });
                }
            }

            let base_z = level_elev(levels, *base_level);
            z0 = base_z + *base_offset;
            let unconn_span = unconnected_height
                .as_ref()
                .map(|h| **h)
                .unwrap_or(fallback_span);
            z1 = match *top_level {
                Some(tid) => level_elev(levels, tid) + *top_offset,
                None => z0 + unconn_span + *top_offset,
            };
            if z1 < z0 + 10.0 {
                z1 = z0 + unconn_span.max(50.0);
            }

            grid.row_text(
                "World Z Bounds",
                format!("{:+.0} → {:+.0} mm", z0, z1),
            );
        },
    );

    (z0, z1)
}

fn render_ifc_category_picker(ui: &mut Ui, category: &mut Option<String>, id_salt: &str) {
    let categories: &[(&str, &str)] = &[
        ("Generic / Freeform", ""),
        ("Floor / Slab (IfcSlab)", "Floor"),
        ("Wall (IfcWall)", "Wall"),
        ("Door (IfcDoor)", "Door"),
        ("Window (IfcWindow)", "Window"),
        ("Roof (IfcRoof)", "Roof"),
        ("Ceiling (IfcCovering)", "Ceiling"),
        ("Column (IfcColumn)", "Column"),
        ("Beam (IfcBeam)", "Beam"),
        ("Stair (IfcStair)", "Stair"),
        ("Ramp (IfcRamp)", "Ramp"),
        ("Railing (IfcRailing)", "Railing"),
        ("Curtain Wall (IfcCurtainWall)", "CurtainWall"),
        ("Fascia (IfcBuildingElementProxy)", "Fascia"),
        ("Gutter (IfcBuildingElementProxy)", "Gutter"),
        ("Footing (IfcFooting)", "Footing"),
        ("Site / Toposolid (IfcSite)", "Site"),
    ];

    let cur_str = category.as_deref().unwrap_or("");
    let current_label = match cur_str {
        "Floor" | "Slab" => "Floor / Slab (IfcSlab)",
        "Wall" => "Wall (IfcWall)",
        "Door" => "Door (IfcDoor)",
        "Window" => "Window (IfcWindow)",
        "Roof" => "Roof (IfcRoof)",
        "Ceiling" => "Ceiling (IfcCovering)",
        "Column" => "Column (IfcColumn)",
        "Beam" => "Beam (IfcBeam)",
        "Stair" => "Stair (IfcStair)",
        "Ramp" => "Ramp (IfcRamp)",
        "Railing" => "Railing (IfcRailing)",
        "CurtainWall" => "Curtain Wall (IfcCurtainWall)",
        "Fascia" => "Fascia (IfcBuildingElementProxy)",
        "Gutter" => "Gutter (IfcBuildingElementProxy)",
        "Footing" => "Footing (IfcFooting)",
        "Site" => "Site / Toposolid (IfcSite)",
        other if !other.is_empty() => other,
        _ => "Generic / Freeform",
    }.to_string();
    let cur_val = category.clone();
    let mut new_cat = None;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("IFC Classification:");
        egui::ComboBox::from_id_salt(format!("ifc_cat_{id_salt}"))
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for (disp, val) in categories {
                    let is_sel = if val.is_empty() {
                        cur_val.is_none() || cur_val.as_deref() == Some("")
                    } else {
                        cur_val.as_deref() == Some(*val)
                    };
                    if ui.selectable_label(is_sel, *disp).clicked() {
                        if val.is_empty() {
                            new_cat = None;
                        } else {
                            new_cat = Some(val.to_string());
                        }
                        changed = true;
                    }
                }
            });
    });
    if changed {
        *category = new_cat;
    }
}

/// Level-Hosting & Elevation Offsets section for wall-hosted openings (`Door`, `Window`).
#[allow(clippy::too_many_arguments)]
fn render_opening_level_constraints(
    ui: &mut Ui,
    salt: &str,
    levels: &[(LevelId, String, f64)],
    host_wall_base_z: f64,
    base_level: &mut Option<LevelId>,
    base_offset: &mut f64,
    sill_height: &mut f64,
    sill_label: &str,
    top_level: &mut Option<LevelId>,
    top_offset: &mut f64,
    opening_height: &mut f64,
    theme: &AppTheme,
) {
    prop_category(
        ui,
        "Level & Elevation Constraints",
        &format!("open_lvl_constraints_{salt}"),
        true,
        theme,
        |grid| {
            let base_text = match *base_level {
                None => format!("Host Wall Base ({:+.0} mm)", host_wall_base_z),
                Some(bid) => level_display_text(levels, bid),
            };
            grid.row("Base Level", |ui| {
                ComboBox::from_id_salt(("open_base_cb", salt))
                    .selected_text(base_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            base_level,
                            None,
                            format!("Host Wall Base ({:+.0} mm)", host_wall_base_z),
                        );
                        for (lid, name, elev) in levels {
                            ui.selectable_value(
                                base_level,
                                Some(*lid),
                                format!("{} ({:+.0} mm)", name, elev),
                            );
                        }
                    });
            });

            grid.row("Base Offset", |ui| {
                ui.add(DragValue::new(base_offset).speed(10.0).suffix(" mm"));
            });

            grid.row(sill_label, |ui| {
                ui.add(DragValue::new(sill_height).speed(10.0).suffix(" mm"));
            });

            let top_text = match *top_level {
                None => "Unconnected (By Height)".to_string(),
                Some(tid) => level_display_text(levels, tid),
            };
            grid.row("Top Constraint", |ui| {
                ComboBox::from_id_salt(("open_top_cb", salt))
                    .selected_text(top_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(top_level, None, "Unconnected (By Height)");
                        for (lid, name, elev) in levels {
                            ui.selectable_value(
                                top_level,
                                Some(*lid),
                                format!("Head to: {} ({:+.0} mm)", name, elev),
                            );
                        }
                    });
            });

            grid.row("Top Offset", |ui| {
                ui.add(DragValue::new(top_offset).speed(10.0).suffix(" mm"));
            });

            let ref_base = match *base_level {
                Some(bid) => level_elev(levels, bid),
                None => host_wall_base_z,
            };
            let z0 = ref_base + *sill_height + *base_offset;
            let raw_top = match *top_level {
                Some(tid) => level_elev(levels, tid) + *top_offset,
                None => z0 + *opening_height + *top_offset,
            };
            let z1 = if raw_top > z0 + 100.0 {
                raw_top
            } else {
                z0 + *opening_height
            };

            grid.row_text(
                "World Z Bounds",
                format!("{:+.0} → {:+.0} mm (Span: {:.0} mm)", z0, z1, (z1 - z0).abs()),
            );
        },
    );
}

fn render_assembly_layer_summary(ui: &mut Ui, layers: &[AssemblyLayer], theme: &AppTheme) {
    if layers.is_empty() {
        return;
    }
    prop_category(
        ui,
        &format!("Compound Layers ({})", layers.len()),
        "compound_layers",
        false,
        theme,
        |grid| {
            for (idx, l) in layers.iter().enumerate() {
                grid.row_text(
                    &format!("{}. {}", idx + 1, l.function.label()),
                    format!("{} ({:.1} mm)", l.material_name, l.thickness),
                );
            }
        },
    );
}

fn render_element_properties(
    ui: &mut Ui,
    element: &mut Element,
    levels: &[(LevelId, String, f64)],
    host_wall_base_z: f64,
    theme: &AppTheme,
) -> Option<ToolCategory> {
    let mut open_studio_for: Option<ToolCategory> = None;
    let elem_cat = ToolCategory::from_element(element);

    // Header Type Selector Box + Studio Button
    ui.group(|ui| {
        ui.label(RichText::new(element.category_name()).size(11.0).strong());
        ui.label(RichText::new(element.name()).size(13.0).strong());
        ui.horizontal(|ui| {
            ui.label(format!("ID: {}", element.id().short_id()));
            if let Some(cat) = elem_cat {
                if ui
                    .small_button(RichText::new("⚙ Edit Type & Layers...").color(theme.accent))
                    .on_hover_text("Open Arch3D Type & Assembly Studio for this element")
                    .clicked()
                {
                    open_studio_for = Some(cat);
                }
            }
        });
    });

    ui.add_space(4.0);

    match element {
        Element::Wall(w) => {
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "wall",
                levels,
                &mut w.base_level,
                &mut w.base_offset,
                "Base Offset",
                &mut w.top_level,
                &mut w.top_offset,
                Some(&mut w.unconnected_height),
                0.0,
                theme,
            );
            let actual_h = (z1 - z0).max(100.0);

            prop_category(ui, "Dimensions (IFC4 Base Quantities)", "wall_dim", true, theme, |grid| {
                grid.row("Thickness", |ui| {
                    ui.add(DragValue::new(&mut w.thickness).speed(10.0).suffix(" mm"));
                });
                grid.row_text("Length", format!("{:.1} mm", w.length()));
                let gross_side_m2 = (w.length() * actual_h) / 1_000_000.0;
                grid.row_text("Gross Side Area", format!("{:.2} m²", gross_side_m2));
                let gross_footprint_m2 = (w.length() * w.thickness) / 1_000_000.0;
                grid.row_text("Footprint Area", format!("{:.2} m²", gross_footprint_m2));
                let vol_m3 = (w.length() * w.thickness * actual_h) / 1_000_000_000.0;
                grid.row_text("Gross Volume", format!("{:.3} m³", vol_m3));
            });

            render_assembly_layer_summary(ui, &w.assembly_layers, theme);

            prop_category(ui, "IFC4 Property Set (Pset_WallCommon)", "wall_pset", true, theme, |grid| {
                grid.row_bool("LoadBearing", &mut w.is_structural);
                grid.row_bool("IsExternal", &mut w.is_external);
                grid.row_bool("Compartment", &mut w.compartmentation);
                grid.row_bool("Combustible", &mut w.combustible);
                grid.row_edit("Fire Rating", &mut w.fire_rating);
                grid.row_edit("Flame Spread", &mut w.surface_spread_of_flame);
                grid.row_edit("Acoustic Rating", &mut w.acoustic_rating);
                grid.row_drag("Thermal Trans (U)", &mut w.thermal_transmittance, 0.01, " W/m²K");
            });

            prop_category(ui, "Identity Data", "wall_ident", true, theme, |grid| {
                render_identity_data(grid, &mut w.wall_type_name, &mut w.tag, &mut w.description);
            });
        }

        Element::CurtainWall(cw) => {
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "curtain_wall",
                levels,
                &mut cw.base_level,
                &mut cw.base_offset,
                "Base Offset",
                &mut cw.top_level,
                &mut cw.top_offset,
                Some(&mut cw.unconnected_height),
                0.0,
                theme,
            );
            let actual_h = (z1 - z0).max(100.0);

            prop_category(ui, "Dimensions & Grid Layout (IFC4)", "cw_dim", true, theme, |grid| {
                grid.row_text("Length", format!("{:.1} mm", cw.length()));
                grid.row_text("Height", format!("{:.1} mm", actual_h));
                let area_m2 = (cw.length() * actual_h) / 1_000_000.0;
                grid.row_text("Glazed Area", format!("{:.2} m²", area_m2));
                grid.row("Mullion Width", |ui| {
                    ui.add(DragValue::new(&mut cw.mullion_width).speed(5.0).suffix(" mm"));
                });
                grid.row("Mullion Depth", |ui| {
                    ui.add(DragValue::new(&mut cw.mullion_depth).speed(5.0).suffix(" mm"));
                });
                grid.row("Glass Thickness", |ui| {
                    ui.add(DragValue::new(&mut cw.panel_thickness).speed(1.0).suffix(" mm"));
                });
                grid.row("Vert Grid Spacing", |ui| {
                    ui.add(DragValue::new(&mut cw.vertical_grid_spacing).speed(50.0).suffix(" mm"));
                });
                grid.row("Horiz Grid Spacing", |ui| {
                    ui.add(DragValue::new(&mut cw.horizontal_grid_spacing).speed(50.0).suffix(" mm"));
                });
            });

            prop_category(ui, "Wall Embedding & Hole Cutout", "cw_embed", true, theme, |grid| {
                if let Some(host_id) = cw.host_wall {
                    grid.row_text("Host Wall", format!("#{:?}", host_id));
                    grid.row_bool("Auto Embed", &mut cw.automatically_embed);
                } else {
                    grid.row_text("Host Wall", "None (Standalone)");
                    grid.row_bool("Auto Embed Nearby", &mut cw.automatically_embed);
                }
            });

            prop_category(ui, "IFC4 Property Set (Pset_CurtainWallCommon)", "cw_pset", true, theme, |grid| {
                grid.row_bool("IsExternal", &mut cw.is_external);
                grid.row_edit("Tag / Mark", &mut cw.tag);
                grid.row_edit("Fire Rating", &mut cw.fire_rating);
                grid.row_edit("Acoustic Rating", &mut cw.acoustic_rating);
                grid.row_drag("Thermal Trans (U)", &mut cw.thermal_transmittance, 0.01, " W/m²K");
            });

            prop_category(ui, "Type Identity", "cw_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut cw.curtain_wall_type_name);
                grid.row_edit("Tag / Mark", &mut cw.tag);
            });
        }

        Element::Roof(r) => {
            render_level_hosting_constraints(
                ui,
                "roof",
                levels,
                &mut r.level,
                &mut r.offset,
                "Base / Eave Offset",
                &mut r.top_level,
                &mut r.top_offset,
                None,
                r.thickness,
                theme,
            );

            prop_category(ui, "Roof Shape & Parameters", "roof_shape", true, theme, |grid| {
                grid.row("Roof Shape", |ui| {
                    ComboBox::from_id_salt("roof_shape_combo")
                        .selected_text(match r.shape {
                            plinth_model::RoofShape::Hip => "Hip Roof (4-Way Slope)",
                            plinth_model::RoofShape::Gable => "Gable Roof (Dual Pitch)",
                            plinth_model::RoofShape::Shed => "Shed Roof (Mono Pitch)",
                            plinth_model::RoofShape::Flat => "Flat Roof",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut r.shape,
                                plinth_model::RoofShape::Hip,
                                "Hip Roof (4-Way Slope)",
                            );
                            ui.selectable_value(
                                &mut r.shape,
                                plinth_model::RoofShape::Gable,
                                "Gable Roof (Dual Pitch)",
                            );
                            ui.selectable_value(
                                &mut r.shape,
                                plinth_model::RoofShape::Shed,
                                "Shed Roof (Mono Pitch)",
                            );
                            ui.selectable_value(
                                &mut r.shape,
                                plinth_model::RoofShape::Flat,
                                "Flat Roof",
                            );
                        });
                });
                if r.shape != plinth_model::RoofShape::Flat {
                    grid.row("Slope / Pitch", |ui| {
                        ui.add(
                            DragValue::new(&mut r.slope_degrees)
                                .speed(0.5)
                                .range(1.0..=75.0)
                                .suffix("°"),
                        );
                    });
                }
                grid.row("Eave Overhang", |ui| {
                    ui.add(
                        DragValue::new(&mut r.overhang)
                            .speed(10.0)
                            .range(0.0..=3000.0)
                            .suffix(" mm"),
                    );
                });
                grid.row("Deck Thickness", |ui| {
                    ui.add(
                        DragValue::new(&mut r.thickness)
                            .speed(10.0)
                            .range(20.0..=2000.0)
                            .suffix(" mm"),
                    );
                });
                let area_m2 = r.gross_area() / 1_000_000.0;
                grid.row_text("Surface Area", format!("{:.2} m²", area_m2));
                let vol_m3 = r.gross_volume() / 1_000_000_000.0;
                grid.row_text("Gross Volume", format!("{:.3} m³", vol_m3));
            });

            render_assembly_layer_summary(ui, &r.assembly_layers, theme);

            prop_category(ui, "IFC4 Property Set (Pset_RoofCommon)", "roof_pset", true, theme, |grid| {
                grid.row_bool("IsStructural", &mut r.is_structural);
                grid.row_bool("IsExternal", &mut r.is_external);
                grid.row_edit("Fire Rating", &mut r.fire_rating);
                grid.row_drag("Thermal Trans (U)", &mut r.thermal_transmittance, 0.01, " W/m²K");
                grid.row_edit("Acoustic Rating", &mut r.acoustic_rating);
                grid.row("Solar Refl (SRI)", |ui| {
                    ui.add(
                        DragValue::new(&mut r.solar_reflectance_index)
                            .speed(1.0)
                            .range(0.0..=100.0),
                    );
                });
                grid.row_drag("Design Snow Load", &mut r.snow_load_capacity, 0.1, " kN/m²");
            });

            prop_category(ui, "Identity Data", "roof_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut r.roof_type_name);
                grid.row_edit("Tag / Mark", &mut r.tag);
            });
        }

        Element::Fascia(f) => {
            render_level_hosting_constraints(
                ui,
                "fascia",
                levels,
                &mut f.level,
                &mut f.vertical_offset,
                "Base Offset",
                &mut f.top_level,
                &mut f.top_offset,
                None,
                f.height,
                theme,
            );

            prop_category(ui, "Fascia Dimensions", "fascia_dim", true, theme, |grid| {
                grid.row_drag("Board Height", &mut f.height, 10.0, " mm");
                grid.row_drag("Board Thickness", &mut f.thickness, 5.0, " mm");
                grid.row_text("Perimeter Run", format!("{:.1} mm", f.length()));
            });

            prop_category(ui, "Identity Data", "fascia_ident", true, theme, |grid| {
                grid.row_edit("Material", &mut f.material_name);
            });
        }

        Element::Gutter(g) => {
            render_level_hosting_constraints(
                ui,
                "gutter",
                levels,
                &mut g.level,
                &mut g.vertical_offset,
                "Base Offset",
                &mut g.top_level,
                &mut g.top_offset,
                None,
                g.depth,
                theme,
            );

            prop_category(ui, "Gutter Dimensions", "gutter_dim", true, theme, |grid| {
                grid.row_drag("Trough Width", &mut g.width, 10.0, " mm");
                grid.row_drag("Trough Depth", &mut g.depth, 10.0, " mm");
                grid.row_text("Eave Run", format!("{:.1} mm", g.length()));
            });

            prop_category(ui, "Identity Data", "gutter_ident", true, theme, |grid| {
                grid.row_edit("Material", &mut g.material_name);
            });
        }

        Element::Column(c) => {
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "column",
                levels,
                &mut c.base_level,
                &mut c.base_offset,
                "Base Offset",
                &mut c.top_level,
                &mut c.top_offset,
                Some(&mut c.unconnected_height),
                0.0,
                theme,
            );
            let actual_h = (z1 - z0).max(100.0);

            prop_category(ui, "Dimensions", "col_dim", true, theme, |grid| {
                grid.row_drag("Width", &mut c.width, 10.0, " mm");
                grid.row_drag("Depth", &mut c.depth, 10.0, " mm");
                grid.row_bool("Circular Profile", &mut c.is_round);
            });

            prop_category(ui, "IFC4 Property Set (Pset_ColumnCommon)", "col_pset", true, theme, |grid| {
                grid.row_bool("LoadBearing", &mut c.is_structural);
                grid.row_bool("IsExternal", &mut c.is_external);
                grid.row_edit("Fire Rating", &mut c.fire_rating);
                grid.row_drag("Design Capacity", &mut c.load_bearing_capacity, 10.0, " kN");
                let area_m2 = (c.width * c.depth) / 1_000_000.0;
                grid.row_text("Cross-Section Area", format!("{:.3} m²", area_m2));
                let vol_m3 = (c.width * c.depth * actual_h) / 1_000_000_000.0;
                grid.row_text("Volume", format!("{:.3} m³", vol_m3));
            });

            prop_category(ui, "Identity Data", "col_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut c.column_type_name);
                grid.row_edit("Tag / Mark", &mut c.tag);
            });
        }

        Element::Footing(f) => {
            prop_category(ui, "Level & Elevation Constraints", "lvl_constraints_footing", true, theme, |grid| {
                grid.row("Base Level", |ui| {
                    ComboBox::from_id_salt("base_lvl_cb_footing")
                        .selected_text(level_display_text(levels, f.base_level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut f.base_level,
                                    *lid,
                                    format!("{} ({:+.0} mm)", name, elev),
                                );
                            }
                        });
                });
                grid.row_drag("Base Offset", &mut f.base_offset, 10.0, " mm");
                let top_text = match f.top_level {
                    None => "Unconnected (Top at Base)".to_string(),
                    Some(tid) => level_display_text(levels, tid),
                };
                grid.row("Top Constraint", |ui| {
                    ComboBox::from_id_salt("top_lvl_cb_footing")
                        .selected_text(top_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut f.top_level,
                                None,
                                "Unconnected (Top at Base)",
                            );
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut f.top_level,
                                    Some(*lid),
                                    format!("Up to: {} ({:+.0} mm)", name, elev),
                                );
                            }
                        });
                });
                grid.row_drag("Top Offset", &mut f.top_offset, 10.0, " mm");

                let base_z = level_elev(levels, f.base_level);
                let top_z = f.top_level.map(|tid| level_elev(levels, tid));
                let (z0, z1) = f.z_bounds(base_z, top_z);
                grid.row_text(
                    "World Z Bounds",
                    format!("{:+.0} → {:+.0} mm (Depth: {:.0} mm)", z0, z1, (z1 - z0).abs()),
                );
            });

            prop_category(ui, "Footing Geometry & Style", "footing_geom", true, theme, |grid| {
                grid.row("Footing Style", |ui| {
                    ComboBox::from_id_salt("prop_footing_style")
                        .selected_text(f.footing_style.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut f.footing_style,
                                FootingStyle::Pad,
                                FootingStyle::Pad.label(),
                            );
                            ui.selectable_value(
                                &mut f.footing_style,
                                FootingStyle::Stepped,
                                FootingStyle::Stepped.label(),
                            );
                            ui.selectable_value(
                                &mut f.footing_style,
                                FootingStyle::Sloped,
                                FootingStyle::Sloped.label(),
                            );
                        });
                });
                grid.row("Width (X)", |ui| {
                    ui.add(
                        DragValue::new(&mut f.width)
                            .speed(25.0)
                            .range(200.0..=15000.0)
                            .suffix(" mm"),
                    );
                });
                grid.row("Length (Y)", |ui| {
                    ui.add(
                        DragValue::new(&mut f.length)
                            .speed(25.0)
                            .range(200.0..=15000.0)
                            .suffix(" mm"),
                    );
                });
                grid.row("Thickness (D)", |ui| {
                    ui.add(
                        DragValue::new(&mut f.thickness)
                            .speed(20.0)
                            .range(100.0..=5000.0)
                            .suffix(" mm"),
                    );
                });
                let mut rot_deg = f.rotation.to_degrees();
                grid.row("Rotation", |ui| {
                    if ui
                        .add(DragValue::new(&mut rot_deg).speed(1.0).suffix("°"))
                        .changed()
                    {
                        f.rotation = rot_deg.to_radians();
                    }
                });
                if let Some(col_id) = f.host_column {
                    grid.row_text("Hosted Column", col_id.short_id());
                }
                let area_m2 = f.gross_area() / 1_000_000.0;
                let vol_m3 = f.gross_volume() / 1_000_000_000.0;
                grid.row_text("Bearing Area", format!("{:.2} m²", area_m2));
                grid.row_text("Concrete Volume", format!("{:.3} m³", vol_m3));
            });

            prop_category(ui, "IFC4 Property Set (Pset_FootingCommon)", "footing_pset", true, theme, |grid| {
                grid.row_bool("IsExternal", &mut f.is_external);
                grid.row_edit("Concrete Grade", &mut f.concrete_grade);
                grid.row_drag("Soil Bearing Cap", &mut f.soil_bearing_capacity, 5.0, " kPa");
                grid.row_drag("Axial Load Cap", &mut f.load_bearing_capacity, 10.0, " kN");
            });

            prop_category(ui, "Identity Data", "footing_ident", true, theme, |grid| {
                grid.row_edit("Name", &mut f.name);
                grid.row_edit("Type Name", &mut f.footing_type_name);
                grid.row_edit("Tag / Mark", &mut f.tag);
            });
        }

        Element::Beam(b) => {
            render_level_hosting_constraints(
                ui,
                "beam",
                levels,
                &mut b.level,
                &mut b.z_offset,
                "Base / Reference Offset",
                &mut b.top_level,
                &mut b.top_offset,
                None,
                b.depth,
                theme,
            );

            prop_category(ui, "Dimensions", "beam_dim", true, theme, |grid| {
                grid.row_drag("Width", &mut b.width, 10.0, " mm");
                grid.row_drag("Depth", &mut b.depth, 10.0, " mm");
                grid.row_text("Length", format!("{:.1} mm", b.start.distance(b.end)));
            });

            prop_category(ui, "IFC4 Property Set (Pset_BeamCommon)", "beam_pset", true, theme, |grid| {
                grid.row_bool("LoadBearing", &mut b.is_structural);
                grid.row_bool("IsExternal", &mut b.is_external);
                grid.row_edit("Fire Rating", &mut b.fire_rating);
                grid.row_drag("Design Uniform Load", &mut b.load_bearing_capacity, 1.0, " kN/m");
                let vol_m3 = (b.width * b.depth * b.start.distance(b.end)) / 1_000_000_000.0;
                grid.row_text("Volume", format!("{:.3} m³", vol_m3));
            });

            prop_category(ui, "Identity Data", "beam_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut b.beam_type_name);
                grid.row_edit("Tag / Mark", &mut b.tag);
            });
        }

        Element::Slab(s) => {
            render_level_hosting_constraints(
                ui,
                "slab",
                levels,
                &mut s.level,
                &mut s.offset,
                "Base / Surface Offset",
                &mut s.top_level,
                &mut s.top_offset,
                None,
                s.thickness,
                theme,
            );

            prop_category(ui, "Dimensions", "slab_dim", true, theme, |grid| {
                grid.row_drag("Thickness", &mut s.thickness, 10.0, " mm");
                let area_m2 = s.gross_area() / 1_000_000.0;
                grid.row_text("Gross Area", format!("{:.2} m²", area_m2));
                let vol_m3 = s.gross_volume() / 1_000_000_000.0;
                grid.row_text("Volume", format!("{:.3} m³", vol_m3));
            });

            render_assembly_layer_summary(ui, &s.assembly_layers, theme);

            prop_category(ui, "IFC4 Property Set (Pset_SlabCommon)", "slab_pset", true, theme, |grid| {
                grid.row_bool("LoadBearing", &mut s.is_structural);
                grid.row_bool("IsExternal", &mut s.is_external);
                grid.row_bool("Compartment", &mut s.compartmentation);
                grid.row_edit("Fire Rating", &mut s.fire_rating);
                grid.row_edit("Acoustic Rating", &mut s.acoustic_rating);
                grid.row_drag("Thermal Trans (U)", &mut s.thermal_transmittance, 0.01, " W/m²K");
                grid.row_drag("Live Load Capacity", &mut s.load_bearing_capacity, 0.5, " kN/m²");
            });

            prop_category(ui, "Identity Data", "slab_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut s.slab_type_name);
                grid.row_edit("Tag / Mark", &mut s.tag);
            });
        }

        Element::Ceiling(c) => {
            render_level_hosting_constraints(
                ui,
                "ceiling",
                levels,
                &mut c.level,
                &mut c.height_above_level,
                "Base Offset (Above Floor)",
                &mut c.top_level,
                &mut c.top_offset,
                None,
                c.thickness,
                theme,
            );

            prop_category(ui, "Dimensions & Grid", "ceiling_dim", true, theme, |grid| {
                grid.row_drag("Tile Thickness", &mut c.thickness, 5.0, " mm");
                grid.row_drag("Tile Grid X", &mut c.grid_spacing_x, 100.0, " mm");
                grid.row_drag("Tile Grid Y", &mut c.grid_spacing_y, 100.0, " mm");
            });

            render_assembly_layer_summary(ui, &c.assembly_layers, theme);

            prop_category(ui, "IFC4 Property Set (Pset_CeilingCommon)", "ceiling_pset", true, theme, |grid| {
                grid.row_bool("IsExternal", &mut c.is_external);
                grid.row_edit("Fire Rating", &mut c.fire_rating);
                grid.row_edit("Acoustic Rating", &mut c.acoustic_rating);
                grid.row("Light Reflectance", |ui| {
                    ui.add(
                        DragValue::new(&mut c.light_reflectance)
                            .speed(0.05)
                            .range(0.0..=1.0),
                    );
                });
            });

            prop_category(ui, "Identity Data", "ceiling_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut c.ceiling_type_name);
                grid.row_edit("Tag / Mark", &mut c.tag);
            });
        }

        Element::Door(d) => {
            render_opening_level_constraints(
                ui,
                "door",
                levels,
                host_wall_base_z,
                &mut d.base_level,
                &mut d.base_offset,
                &mut d.sill_height,
                "Sill / Threshold",
                &mut d.top_level,
                &mut d.top_offset,
                &mut d.height,
                theme,
            );

            prop_category(ui, "Dimensions & Placement", "door_dim", true, theme, |grid| {
                grid.row_drag("Width", &mut d.width, 10.0, " mm");
                grid.row_drag("Height", &mut d.height, 10.0, " mm");
                grid.row_drag("Offset Along Wall", &mut d.offset_along_wall, 50.0, " mm");
                grid.row_bool("Flip Hand (L/R)", &mut d.flipped_hand);
                grid.row_bool("Flip Facing (In/Out)", &mut d.flipped_face);
            });

            prop_category(ui, "IFC4 Property Set (Pset_DoorCommon)", "door_pset", true, theme, |grid| {
                grid.row_bool("IsExternal", &mut d.is_external);
                grid.row_bool("FireExit", &mut d.is_fire_exit);
                grid.row_edit("Fire Rating", &mut d.fire_rating);
                grid.row_edit("Acoustic Rating", &mut d.acoustic_rating);
                grid.row_edit("Security Rating", &mut d.security_rating);
                grid.row_drag("Thermal Trans (U)", &mut d.thermal_transmittance, 0.05, " W/m²K");
                grid.row_edit("Operation Type", &mut d.operation_type);
                let area_m2 = (d.width * d.height) / 1_000_000.0;
                grid.row_text("Opening Area", format!("{:.2} m²", area_m2));
            });

            prop_category(ui, "Identity Data", "door_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut d.door_type_name);
                grid.row_edit("Tag / Mark", &mut d.tag);
            });
        }

        Element::Window(w) => {
            render_opening_level_constraints(
                ui,
                "window",
                levels,
                host_wall_base_z,
                &mut w.base_level,
                &mut w.base_offset,
                &mut w.sill_height,
                "Sill Height",
                &mut w.top_level,
                &mut w.top_offset,
                &mut w.height,
                theme,
            );

            prop_category(ui, "Dimensions & Placement", "window_dim", true, theme, |grid| {
                grid.row_drag("Width", &mut w.width, 10.0, " mm");
                grid.row_drag("Height", &mut w.height, 10.0, " mm");
                grid.row_drag("Offset Along Wall", &mut w.offset_along_wall, 50.0, " mm");
            });

            prop_category(ui, "IFC4 Property Set (Pset_WindowCommon)", "window_pset", true, theme, |grid| {
                grid.row_bool("IsExternal", &mut w.is_external);
                grid.row_edit("Fire Rating", &mut w.fire_rating);
                grid.row_edit("Acoustic Rating", &mut w.acoustic_rating);
                grid.row_drag("Thermal Trans (U)", &mut w.thermal_transmittance, 0.05, " W/m²K");
                grid.row("Solar Heat (SHGC)", |ui| {
                    ui.add(
                        DragValue::new(&mut w.solar_heat_gain_coefficient)
                            .speed(0.02)
                            .range(0.0..=1.0),
                    );
                });
                grid.row_edit("Operation Type", &mut w.operation_type);
                let area_m2 = (w.width * w.height) / 1_000_000.0;
                grid.row_text("Glazing Area", format!("{:.2} m²", area_m2));
            });

            prop_category(ui, "Identity Data", "window_ident", true, theme, |grid| {
                grid.row_edit("Type Name", &mut w.window_type_name);
                grid.row_edit("Tag / Mark", &mut w.tag);
            });
        }

        Element::FreeformSurface(s) => {
            let mut should_extrude = false;

            prop_category(ui, "Surface Actions & Workflow", "surf_actions", true, theme, |grid| {
                grid.row_full(|ui| {
                    if ui
                        .button(
                            RichText::new("⬆ Extrude into 3D Solid")
                                .strong()
                                .color(Color32::from_rgb(255, 120, 0)),
                        )
                        .on_hover_text("Convert this 2D surface into a solid 3D extrusion (default 2800 mm)")
                        .clicked()
                    {
                        should_extrude = true;
                    }

                    let hole_btn_text = if s.is_hole {
                        "🕳 Cutout Hole (Active)"
                    } else {
                        "🕳 Make Cutout / Hole"
                    };
                    if ui.button(hole_btn_text).clicked() {
                        s.is_hole = !s.is_hole;
                        if s.is_hole && s.ifc_category.is_none() {
                            s.ifc_category = Some("Opening".into());
                        }
                    }
                });

                if s.is_hole {
                    grid.row_full(|ui| {
                        ui.label(
                            RichText::new("🕳 Element flagged as Void Opening Cutout.")
                                .size(10.5)
                                .color(Color32::from_rgb(255, 120, 0)),
                        );
                    });
                }
            });

            render_ifc_category_picker(ui, &mut s.ifc_category, "freeform_surface");

            if let Some(hw_id) = s.host_wall {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏠 Host Wall:").size(11.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("{hw_id}")).size(11.0).strong().color(theme.accent));
                });
            }

            let (z0, _) = render_level_hosting_constraints(
                ui,
                "freeform_surface",
                levels,
                &mut s.level,
                &mut s.base_offset,
                "Base Offset",
                &mut s.top_level,
                &mut s.top_offset,
                None,
                0.0,
                theme,
            );
            s.elevation = z0;

            prop_category(ui, "Surface Geometry", "surf_geom", true, theme, |grid| {
                grid.row_text("Boundary Vertices", format!("{}", s.boundary.len()));
                let area_m2 = s.area() / 1_000_000.0;
                grid.row_text("Planar Area", format!("{:.2} m²", area_m2));
                grid.row_drag("Elevation (Z)", &mut s.elevation, 50.0, " mm");
            });

            prop_category(ui, "Identity Data", "surf_ident", true, theme, |grid| {
                grid.row_edit("Name", &mut s.name);
            });

            if should_extrude {
                let solid = s.extrude(2800.0);
                *element = Element::FreeformSolid(solid);
                return open_studio_for;
            }
        }

        Element::FreeformSolid(f) => {
            render_ifc_category_picker(ui, &mut f.ifc_category, "freeform_solid");
            if let Some(cat) = &f.ifc_category {
                f.group_name = Some(cat.clone());
            }

            if let Some(hw_id) = f.host_wall {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏠 Host Wall:").size(11.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("{hw_id}")).size(11.0).strong().color(theme.accent));
                });
            }

            let default_h = f.height();
            let mut h = default_h;
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "freeform_solid",
                levels,
                &mut f.level,
                &mut f.base_offset,
                "Base Offset",
                &mut f.top_level,
                &mut f.top_offset,
                Some(&mut h),
                default_h,
                theme,
            );
            f.z_min = z0;
            f.z_max = z1;

            prop_category(ui, "Solid Dimensions", "solid_dim", true, theme, |grid| {
                let type_label = match &f.primitive {
                    plinth_model::PrimitiveKind::CustomExtrusion => {
                        "Custom 3D Extrusion".to_string()
                    }
                    plinth_model::PrimitiveKind::Box { width, length, height } => {
                        format!("Box Primitive ({:.0} × {:.0} × {:.0} mm)", width, length, height)
                    }
                    plinth_model::PrimitiveKind::Cylinder { radius, height, .. } => {
                        format!("Cylinder Primitive (Ø{:.0} mm × {:.0} mm)", radius * 2.0, height)
                    }
                    plinth_model::PrimitiveKind::Sphere { radius, .. } => {
                        format!("Sphere Primitive (Ø{:.0} mm)", radius * 2.0)
                    }
                    plinth_model::PrimitiveKind::Mesh { faces } => {
                        format!("Custom Solid ({} faces)", faces.len())
                    }
                };
                grid.row_text("Geometry", type_label);
                let mut h = f.height();
                grid.row("Extrusion Height", |ui| {
                    if ui
                        .add(
                            DragValue::new(&mut h)
                                .speed(50.0)
                                .range(10.0..=100000.0)
                                .suffix(" mm"),
                        )
                        .changed()
                    {
                        f.z_max = f.z_min + h;
                    }
                });
                grid.row_drag("Base Elevation (Z Min)", &mut f.z_min, 50.0, " mm");
                grid.row_drag("Top Elevation (Z Max)", &mut f.z_max, 50.0, " mm");
                let vol_m3 = f.volume() / 1_000_000_000.0;
                grid.row_text("Volume", format!("{:.3} m³", vol_m3));
                let area_m2 = f.surface_area() / 1_000_000.0;
                grid.row_text("Total Surface Area", format!("{:.2} m²", area_m2));
            });

            prop_category(ui, "Identity Data", "solid_ident", true, theme, |grid| {
                grid.row_edit("Name", &mut f.name);
                if let Some(grp) = &f.group_name {
                    let label = if f.is_group { "Group Hierarchy" } else { "Component / Category" };
                    grid.row_text(label, grp);
                }
            });
        }

        Element::Stair(s) => {
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "stair",
                levels,
                &mut s.base_level,
                &mut s.base_offset,
                "Base Offset",
                &mut s.top_level,
                &mut s.top_offset,
                Some(&mut s.height),
                0.0,
                theme,
            );
            let total_rise = (z1 - z0).max(100.0);

            prop_category(ui, "Stair Dimensions & 3D Solid Style", "stair_dim", true, theme, |grid| {
                grid.row("Solid Style", |ui| {
                    ComboBox::from_id_salt("prop_stair_style")
                        .selected_text(s.stair_style.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut s.stair_style,
                                StairStyle::Monolithic,
                                StairStyle::Monolithic.label(),
                            );
                            ui.selectable_value(
                                &mut s.stair_style,
                                StairStyle::SolidStepped,
                                StairStyle::SolidStepped.label(),
                            );
                            ui.selectable_value(
                                &mut s.stair_style,
                                StairStyle::OpenRiser,
                                StairStyle::OpenRiser.label(),
                            );
                        });
                });
                grid.row("Flight Width", |ui| {
                    ui.add(
                        DragValue::new(&mut s.width)
                            .speed(25.0)
                            .range(300.0..=5000.0)
                            .suffix(" mm"),
                    );
                });
                if s.stair_style == StairStyle::Monolithic {
                    grid.row("Waist Slab", |ui| {
                        ui.add(
                            DragValue::new(&mut s.waist_thickness)
                                .speed(5.0)
                                .range(60.0..=500.0)
                                .suffix(" mm"),
                        );
                    });
                }
                grid.row("Riser Count", |ui| {
                    ui.add(DragValue::new(&mut s.riser_count).range(2..=100));
                });
                let run = s.length();
                let riser_h = total_rise / (s.riser_count.max(1) as f64);
                let tread_d = run / (s.riser_count.max(1) as f64);
                grid.row_text("Run Length", format!("{:.0} mm", run));
                grid.row_text("Riser Height (R)", format!("{:.1} mm", riser_h));
                grid.row_text("Tread Going (G)", format!("{:.1} mm", tread_d));
                grid.row_text("Blondel (2R+G)", format!("{:.0} mm", 2.0 * riser_h + tread_d));
            });

            prop_category(ui, "Identity & IFC4 (Pset_StairCommon)", "stair_pset", true, theme, |grid| {
                grid.row_edit("Name", &mut s.name);
                grid.row_edit("Stair Type", &mut s.stair_type_name);
                grid.row_edit("Mark / Tag", &mut s.tag);
                grid.row_edit("Fire Rating", &mut s.fire_rating);
                grid.row_bool("Exterior Circ.", &mut s.is_external);
            });
        }

        Element::Ramp(r) => {
            let (z0, z1) = render_level_hosting_constraints(
                ui,
                "ramp",
                levels,
                &mut r.base_level,
                &mut r.base_offset,
                "Base Offset",
                &mut r.top_level,
                &mut r.top_offset,
                Some(&mut r.height),
                0.0,
                theme,
            );
            let total_rise = (z1 - z0).max(50.0);

            prop_category(ui, "Ramp Dimensions & Slope", "ramp_dim", true, theme, |grid| {
                grid.row("Solid Style", |ui| {
                    ComboBox::from_id_salt("prop_ramp_style")
                        .selected_text(r.ramp_style.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut r.ramp_style,
                                RampStyle::SlopedSlab,
                                RampStyle::SlopedSlab.label(),
                            );
                            ui.selectable_value(
                                &mut r.ramp_style,
                                RampStyle::SolidWedge,
                                RampStyle::SolidWedge.label(),
                            );
                        });
                });
                grid.row("Ramp Width", |ui| {
                    ui.add(
                        DragValue::new(&mut r.width)
                            .speed(25.0)
                            .range(500.0..=5000.0)
                            .suffix(" mm"),
                    );
                });
                if r.ramp_style == RampStyle::SlopedSlab {
                    grid.row("Slab Thickness", |ui| {
                        ui.add(
                            DragValue::new(&mut r.thickness)
                                .speed(10.0)
                                .range(50.0..=1000.0)
                                .suffix(" mm"),
                        );
                    });
                }
                let run = r.length();
                grid.row_text("Run Length", format!("{:.0} mm", run));
                let slope = if run > 1e-3 {
                    (total_rise / run) * 100.0
                } else {
                    0.0
                };
                grid.row_text(
                    "Calculated Slope",
                    format!("{:.1}% (1:{:.1})", slope, if slope > 1e-3 { 100.0 / slope } else { 0.0 }),
                );
            });

            prop_category(ui, "Identity & IFC4 (Pset_RampCommon)", "ramp_pset", true, theme, |grid| {
                grid.row_edit("Name", &mut r.name);
                grid.row_edit("Ramp Type", &mut r.ramp_type_name);
                grid.row_edit("Mark / Tag", &mut r.tag);
                grid.row_edit("Fire Rating", &mut r.fire_rating);
                grid.row_bool("Exterior Ramp", &mut r.is_external);
            });
        }

        Element::Railing(rl) => {
            render_level_hosting_constraints(
                ui,
                "railing",
                levels,
                &mut rl.level,
                &mut rl.base_offset,
                "Base Offset",
                &mut rl.top_level,
                &mut rl.top_offset,
                Some(&mut rl.height),
                0.0,
                theme,
            );

            prop_category(ui, "Railing Dimensions", "railing_dim", true, theme, |grid| {
                grid.row("Baluster Spacing", |ui| {
                    ui.add(
                        DragValue::new(&mut rl.baluster_spacing)
                            .speed(10.0)
                            .range(50.0..=1000.0)
                            .suffix(" mm"),
                    );
                });
                grid.row("Mid-Rails", |ui| {
                    ui.add(DragValue::new(&mut rl.mid_rail_count).range(0..=8));
                });
                grid.row_text("Path Vertices", format!("{}", rl.path.len()));
            });

            prop_category(ui, "Identity & IFC4 (Pset_RailingCommon)", "railing_pset", true, theme, |grid| {
                grid.row_edit("Name", &mut rl.name);
                grid.row_edit("Railing Type", &mut rl.railing_type_name);
                grid.row_edit("Material", &mut rl.material_name);
                grid.row_edit("Mark / Tag", &mut rl.tag);
                grid.row_bool("Exterior Guard", &mut rl.is_external);
            });
        }

        Element::Dimension(d) => {
            prop_category(ui, "Dimension Properties", "dim_props", true, theme, |grid| {
                grid.row_text("Measured Value", d.formatted_text());
                grid.row_text(
                    "Dimension Type",
                    match d.kind {
                        plinth_model::DimensionKind::Linear => "Linear",
                        plinth_model::DimensionKind::Circular => "Circular (Radius / Diameter)",
                        plinth_model::DimensionKind::Angular => "Angular (Degrees)",
                    },
                );

                grid.row("Tick Mark", |ui| {
                    ComboBox::from_id_salt("prop_dim_tick")
                        .selected_text(match d.tick_mark {
                            plinth_model::TickMark::Tick45 => "45° Architectural Slash",
                            plinth_model::TickMark::Arrow => "Closed Arrowhead",
                            plinth_model::TickMark::Dot => "Filled Datum Dot",
                            plinth_model::TickMark::OpenArrow => "Open Arrowhead",
                            plinth_model::TickMark::None => "None",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut d.tick_mark, plinth_model::TickMark::Tick45, "45° Architectural Slash");
                            ui.selectable_value(&mut d.tick_mark, plinth_model::TickMark::Arrow, "Closed Arrowhead");
                            ui.selectable_value(&mut d.tick_mark, plinth_model::TickMark::Dot, "Filled Datum Dot");
                            ui.selectable_value(&mut d.tick_mark, plinth_model::TickMark::OpenArrow, "Open Arrowhead");
                            ui.selectable_value(&mut d.tick_mark, plinth_model::TickMark::None, "None");
                        });
                });

                grid.row("Font Size", |ui| {
                    ui.add(DragValue::new(&mut d.font_size).range(6.0..=48.0).suffix(" pt").speed(0.5));
                });

                grid.row("Font Style", |ui| {
                    ComboBox::from_id_salt("prop_dim_style")
                        .selected_text(match d.font_style {
                            plinth_model::FontStyle::Regular => "Regular",
                            plinth_model::FontStyle::Bold => "Bold",
                            plinth_model::FontStyle::Italic => "Italic",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut d.font_style, plinth_model::FontStyle::Regular, "Regular");
                            ui.selectable_value(&mut d.font_style, plinth_model::FontStyle::Bold, "Bold");
                            ui.selectable_value(&mut d.font_style, plinth_model::FontStyle::Italic, "Italic");
                        });
                });

                grid.row("Color", |ui| {
                    let mut c = Color32::from_rgba_premultiplied(d.color[0], d.color[1], d.color[2], d.color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        d.color = [c.r(), c.g(), c.b(), c.a()];
                    }
                });

                grid.row_drag("Line Offset", &mut d.offset, 10.0, " mm");

                if d.kind == plinth_model::DimensionKind::Circular {
                    grid.row("Display Mode", |ui| {
                        if ui.checkbox(&mut d.is_diameter, "Display as Diameter (Ø)").changed() {
                            d.prefix = Some(if d.is_diameter { "Ø ".to_string() } else { "R ".to_string() });
                        }
                    });
                }
            });
        }

        Element::TextNote(t) => {
            prop_category(ui, "Text Annotation", "text_props", true, theme, |grid| {
                grid.row("Text Content", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(180.0));
                });
                grid.row("Font Size", |ui| {
                    ui.add(DragValue::new(&mut t.font_size).range(6.0..=48.0).suffix(" pt").speed(0.5));
                });
                grid.row("Font Style", |ui| {
                    ComboBox::from_id_salt("prop_txt_style")
                        .selected_text(match t.font_style {
                            plinth_model::FontStyle::Regular => "Regular",
                            plinth_model::FontStyle::Bold => "Bold",
                            plinth_model::FontStyle::Italic => "Italic",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut t.font_style, plinth_model::FontStyle::Regular, "Regular");
                            ui.selectable_value(&mut t.font_style, plinth_model::FontStyle::Bold, "Bold");
                            ui.selectable_value(&mut t.font_style, plinth_model::FontStyle::Italic, "Italic");
                        });
                });
                grid.row("Color", |ui| {
                    let mut c = Color32::from_rgba_premultiplied(t.color[0], t.color[1], t.color[2], t.color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        t.color = [c.r(), c.g(), c.b(), c.a()];
                    }
                });
                grid.row_drag("Rotation", &mut t.angle_deg, 1.0, "°");
            });
        }

        Element::Leader(l) => {
            prop_category(ui, "Leader Note", "ldr_props", true, theme, |grid| {
                grid.row("Callout Text", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut l.text).desired_rows(2).desired_width(180.0));
                });
                grid.row("Arrowhead", |ui| {
                    ComboBox::from_id_salt("prop_ldr_tick")
                        .selected_text(match l.tick_mark {
                            plinth_model::TickMark::Tick45 => "45° Architectural Slash",
                            plinth_model::TickMark::Arrow => "Closed Arrowhead",
                            plinth_model::TickMark::Dot => "Filled Datum Dot",
                            plinth_model::TickMark::OpenArrow => "Open Arrowhead",
                            plinth_model::TickMark::None => "None",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut l.tick_mark, plinth_model::TickMark::Tick45, "45° Architectural Slash");
                            ui.selectable_value(&mut l.tick_mark, plinth_model::TickMark::Arrow, "Closed Arrowhead");
                            ui.selectable_value(&mut l.tick_mark, plinth_model::TickMark::Dot, "Filled Datum Dot");
                            ui.selectable_value(&mut l.tick_mark, plinth_model::TickMark::OpenArrow, "Open Arrowhead");
                            ui.selectable_value(&mut l.tick_mark, plinth_model::TickMark::None, "None");
                        });
                });
                grid.row("Font Size", |ui| {
                    ui.add(DragValue::new(&mut l.font_size).range(6.0..=48.0).suffix(" pt").speed(0.5));
                });
                grid.row("Font Style", |ui| {
                    ComboBox::from_id_salt("prop_ldr_style")
                        .selected_text(match l.font_style {
                            plinth_model::FontStyle::Regular => "Regular",
                            plinth_model::FontStyle::Bold => "Bold",
                            plinth_model::FontStyle::Italic => "Italic",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut l.font_style, plinth_model::FontStyle::Regular, "Regular");
                            ui.selectable_value(&mut l.font_style, plinth_model::FontStyle::Bold, "Bold");
                            ui.selectable_value(&mut l.font_style, plinth_model::FontStyle::Italic, "Italic");
                        });
                });
                grid.row("Color", |ui| {
                    let mut c = Color32::from_rgba_premultiplied(l.color[0], l.color[1], l.color[2], l.color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        l.color = [c.r(), c.g(), c.b(), c.a()];
                    }
                });
                grid.row_text("Shelf Length", format!("{:.1} mm", l.shoulder_pt.distance(l.text_pt)));
            });
        }

        Element::BuildingPad(bp) => {
            prop_category(ui, "Level & Elevation Constraints", "pad_constraints", true, theme, |grid| {
                grid.row("Base Level", |ui| {
                    ComboBox::from_id_salt("pad_lvl")
                        .selected_text(level_display_text(levels, bp.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut bp.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
                grid.row_drag("Elevation Offset", &mut bp.elevation_offset, 25.0, " mm");
            });

            prop_category(ui, "Excavation Dimensions", "pad_dims", true, theme, |grid| {
                grid.row_drag("Pad Thickness", &mut bp.thickness, 10.0, " mm");
                grid.row_drag("Slope Angle", &mut bp.slope_deg, 0.5, " °");
                let area_m2 = bp.area() / 1_000_000.0;
                grid.row_text("Excavation Footprint", format!("{area_m2:.2} m²"));
                let vol_m3 = (bp.area() * bp.thickness) / 1_000_000_000.0;
                grid.row_text("Excavation Volume", format!("{vol_m3:.3} m³"));
                grid.row_text("Boundary Vertices", format!("{}", bp.boundary.len()));
            });

            prop_category(ui, "IFC4 Export (IfcBuildingElementProxy)", "pad_ifc", true, theme, |grid| {
                grid.row_text("Predefined Type", "PROVISIONFORVOID / PAD");
                grid.row_text("LoadBearing", "FALSE");
            });
        }

        Element::Toposolid(ts) => {
            prop_category(ui, "Level & Base Depth", "topo_constraints", true, theme, |grid| {
                grid.row("Host Level", |ui| {
                    ComboBox::from_id_salt("topo_lvl")
                        .selected_text(level_display_text(levels, ts.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut ts.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
                grid.row_drag("Base Depth", &mut ts.base_depth, 50.0, " mm");
            });

            prop_category(ui, "Contour Display Settings", "topo_contours", true, theme, |grid| {
                grid.row("Show Contours", |ui| {
                    ui.checkbox(&mut ts.show_contours, "Display Isolines");
                });
                grid.row_drag("Primary Interval", &mut ts.primary_contour_interval, 100.0, " mm");
                grid.row_drag("Intermediate Interval", &mut ts.secondary_contour_interval, 20.0, " mm");
            });

            prop_category(ui, "Topography Dimensions", "topo_dims", true, theme, |grid| {
                let area_m2 = ts.area() / 1_000_000.0;
                grid.row_text("Projected Area", format!("{area_m2:.2} m²"));
                grid.row_text("Spot Elevations", format!("{} points", ts.elevation_points.len()));
                grid.row_text("Boundary Vertices", format!("{}", ts.boundary.len()));
            });

            prop_category(ui, "IFC4 Export (IfcGeographicElement)", "topo_ifc", true, theme, |grid| {
                grid.row_text("Predefined Type", "TERRAIN");
                grid.row_text("Substructure", "TRUE");
            });
        }

        Element::PropertyLine(pl) => {
            prop_category(ui, "Cadastral Parcel Identification", "prop_ident", true, theme, |grid| {
                grid.row_text("Creation Mode", match pl.mode {
                    plinth_model::PropertyLineCreationMode::BySurveyTable => "Metes & Bounds Survey Table",
                    plinth_model::PropertyLineCreationMode::BySketch => "Direct CAD Boundary Sketch",
                });
                grid.row("Host Level", |ui| {
                    ComboBox::from_id_salt("prop_lvl")
                        .selected_text(level_display_text(levels, pl.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut pl.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
            });

            prop_category(ui, "Survey Metrics & Quality", "prop_metrics", true, theme, |grid| {
                let area_m2 = pl.area_sqm();
                grid.row_text("Lot Area", format!("{area_m2:.2} m² ({:.4} ha)", area_m2 / 10000.0));
                grid.row_text("Perimeter", format!("{:.2} m", pl.perimeter_m()));
                grid.row_text("Linear Closure Error", format!("{:.1} mm", pl.closure_error_mm()));
                grid.row_text("Closure Ratio", format!("1 : {}", pl.closure_precision_ratio()));
                grid.row_text("Survey Calls / Segments", format!("{}", pl.survey_table.len()));
            });
        }

        Element::Road(rd) => {
            let cat = ToolCategory::Road;
            if ui.button(format!("✏ Edit Type ({})", cat.title())).clicked() {
                open_studio_for = Some(cat);
            }

            prop_category(ui, "Level Constraints", "rd_constraints", true, theme, |grid| {
                grid.row("Base Level", |ui| {
                    ComboBox::from_id_salt("rd_lvl")
                        .selected_text(level_display_text(levels, rd.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut rd.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
                grid.row_drag("Base Offset", &mut rd.base_offset, 25.0, " mm");
            });

            prop_category(ui, "Road Dimensions & Profile", "rd_dims", true, theme, |grid| {
                grid.row_drag("Total Width", &mut rd.width, 100.0, " mm");
                grid.row_drag("Number of Lanes", &mut rd.lanes, 1, "");
                grid.row_drag("Lane Width", &mut rd.lane_width, 50.0, " mm");
                grid.row_drag("Pavement Thickness", &mut rd.thickness, 10.0, " mm");
                grid.row_drag("Crown Cross-Slope", &mut rd.crown_slope_pct, 0.1, " %");
                grid.row("Curb Profile", |ui| {
                    ComboBox::from_id_salt("rd_curb_prop")
                        .selected_text(match rd.curb_type {
                            plinth_model::RoadCurb::Barrier => "Barrier Curb",
                            plinth_model::RoadCurb::Mountable => "Mountable Curb",
                            plinth_model::RoadCurb::None => "No Curb",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut rd.curb_type, plinth_model::RoadCurb::Barrier, "Barrier Curb");
                            ui.selectable_value(&mut rd.curb_type, plinth_model::RoadCurb::Mountable, "Mountable Curb");
                            ui.selectable_value(&mut rd.curb_type, plinth_model::RoadCurb::None, "No Curb");
                        });
                });
                grid.row_drag("Curb Width", &mut rd.curb_width, 10.0, " mm");
                grid.row_drag("Curb Height", &mut rd.curb_height, 10.0, " mm");
                grid.row("Centerline", |ui| {
                    ui.checkbox(&mut rd.has_centerline, "Painted Centerline");
                });
                grid.row_text("Corridor Length", format!("{:.2} m", rd.length_m()));
            });

            prop_category(ui, "IFC4 Export (IfcCivilElement)", "rd_ifc", true, theme, |grid| {
                grid.row_text("Predefined Type", "ROADWAY");
                grid.row_text("Standard Pset", "Pset_RoadCommon");
            });
        }

        Element::Sidewalk(sw) => {
            let cat = ToolCategory::Sidewalk;
            if ui.button(format!("✏ Edit Type ({})", cat.title())).clicked() {
                open_studio_for = Some(cat);
            }

            prop_category(ui, "Level Constraints", "sw_constraints", true, theme, |grid| {
                grid.row("Base Level", |ui| {
                    ComboBox::from_id_salt("sw_lvl")
                        .selected_text(level_display_text(levels, sw.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut sw.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
                grid.row_drag("Base Offset", &mut sw.base_offset, 25.0, " mm");
            });

            prop_category(ui, "Walkway Dimensions & Profile", "sw_dims", true, theme, |grid| {
                grid.row_drag("Walkway Width", &mut sw.width, 50.0, " mm");
                grid.row_drag("Slab Thickness", &mut sw.thickness, 5.0, " mm");
                grid.row_drag("Curb Height", &mut sw.curb_height, 5.0, " mm");
                grid.row("Edge Profile", |ui| {
                    ComboBox::from_id_salt("sw_edge_prop")
                        .selected_text(match sw.edge_profile {
                            plinth_model::SidewalkEdgeProfile::Rounded { radius } => format!("Bullnose Rounded (R{:.0}mm)", radius),
                            plinth_model::SidewalkEdgeProfile::Square => "Square Edge".to_string(),
                            plinth_model::SidewalkEdgeProfile::Beveled { chamfer } => format!("Beveled ({:.0}mm)", chamfer),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut sw.edge_profile, plinth_model::SidewalkEdgeProfile::Rounded { radius: 25.0 }, "Bullnose Rounded (R25mm)");
                            ui.selectable_value(&mut sw.edge_profile, plinth_model::SidewalkEdgeProfile::Square, "Square Edge");
                            ui.selectable_value(&mut sw.edge_profile, plinth_model::SidewalkEdgeProfile::Beveled { chamfer: 20.0 }, "Beveled (20mm)");
                        });
                });
                grid.row("Top Contraction Joints", |ui| {
                    ui.checkbox(&mut sw.has_top_groove, "Contraction Grooves");
                });
                if sw.has_top_groove {
                    grid.row_drag("Groove Spacing", &mut sw.groove_spacing, 50.0, " mm");
                    grid.row_drag("Groove Depth", &mut sw.groove_depth, 1.0, " mm");
                }
                grid.row_text("Walkway Length", format!("{:.2} m", sw.length_m()));
            });

            prop_category(ui, "IFC4 Export (IfcCivilElement)", "sw_ifc", true, theme, |grid| {
                grid.row_text("Predefined Type", "SIDEWALK");
                grid.row_text("Standard Pset", "Pset_SidewalkCommon");
            });
        }

        Element::Pathwalk(pw) => {
            let cat = ToolCategory::Pathwalk;
            if ui.button(format!("✏ Edit Type ({})", cat.title())).clicked() {
                open_studio_for = Some(cat);
            }

            prop_category(ui, "Level Constraints", "pw_constraints", true, theme, |grid| {
                grid.row("Base Level", |ui| {
                    ComboBox::from_id_salt("pw_lvl")
                        .selected_text(level_display_text(levels, pw.level))
                        .show_ui(ui, |ui| {
                            for (lid, name, elev) in levels {
                                ui.selectable_value(
                                    &mut pw.level,
                                    *lid,
                                    format!("{name} ({elev:+.0} mm)"),
                                );
                            }
                        });
                });
                grid.row_drag("Base Offset", &mut pw.base_offset, 25.0, " mm");
            });

            prop_category(ui, "Pathwalk Dimensions & Surface", "pw_dims", true, theme, |grid| {
                grid.row_drag("Path Width", &mut pw.width, 50.0, " mm");
                grid.row_drag("Bed Thickness", &mut pw.thickness, 5.0, " mm");
                grid.row("Surface Material", |ui| {
                    ComboBox::from_id_salt("pw_surf_prop")
                        .selected_text(match pw.profile {
                            plinth_model::PathwalkProfile::PaverBlocks => "Interlocking Paver Blocks",
                            plinth_model::PathwalkProfile::Cobblestone => "Natural Cobblestone",
                            plinth_model::PathwalkProfile::Flagstone => "Flagstone Stepping Stones",
                            plinth_model::PathwalkProfile::GravelBed => "Gravel Bed",
                            plinth_model::PathwalkProfile::TimberBoardwalk => "Timber Boardwalk",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut pw.profile, plinth_model::PathwalkProfile::PaverBlocks, "Interlocking Paver Blocks");
                            ui.selectable_value(&mut pw.profile, plinth_model::PathwalkProfile::Cobblestone, "Natural Cobblestone");
                            ui.selectable_value(&mut pw.profile, plinth_model::PathwalkProfile::Flagstone, "Flagstone Stepping Stones");
                            ui.selectable_value(&mut pw.profile, plinth_model::PathwalkProfile::GravelBed, "Gravel Bed");
                            ui.selectable_value(&mut pw.profile, plinth_model::PathwalkProfile::TimberBoardwalk, "Timber Boardwalk");
                        });
                });
                grid.row("Border Edging", |ui| {
                    ui.checkbox(&mut pw.has_edging, "Border Restraint Edging");
                });
                if pw.has_edging {
                    grid.row_drag("Edging Width", &mut pw.edging_width, 5.0, " mm");
                }
                grid.row_drag("Joint Gap Width", &mut pw.groove_width, 1.0, " mm");
                grid.row_text("Path Length", format!("{:.2} m", pw.length_m()));
            });

            prop_category(ui, "IFC4 Export (IfcCivilElement)", "pw_ifc", true, theme, |grid| {
                grid.row_text("Predefined Type", "PAVEDPATH");
                grid.row_text("Standard Pset", "Pset_PathwalkCommon");
            });
        }

        _ => {
            ui.label("Standard annotation element.");
        }
    }

    open_studio_for
}

fn render_view_properties(ui: &mut Ui, view: &mut ViewDef, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(RichText::new("View Properties").strong().size(11.0));
        ui.label(RichText::new(&view.name).size(13.0).strong());
    });

    ui.add_space(4.0);

    prop_category(ui, "Graphics", "view_graphics", true, theme, |grid| {
        grid.row_drag("View Scale 1:", &mut view.scale, 10, "");
        grid.row_text("Discipline", "Architectural / Structural");
    });

    prop_category(ui, "Extents", "view_extents", true, theme, |grid| {
        grid.row_drag("Cut Plane Offset", &mut view.cut_offset, 50.0, " mm");
        grid.row_drag("Top Clip Offset", &mut view.top_offset, 50.0, " mm");
        grid.row_drag("Bottom Clip Offset", &mut view.bottom_offset, 50.0, " mm");
    });

    prop_category(ui, "Identity Data", "view_identity", true, theme, |grid| {
        grid.row_edit("View Name", &mut view.name);
    });
}
