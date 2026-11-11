//! Datum Management Studio: Level Manager (Storeys & Views) and Grid Manager (Orthogonal & Custom Axis Grids).

use crate::theme::AppTheme;
use egui::{
    Align2, Color32, CollapsingHeader, DragValue, FontId, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, Ui, Vec2 as EguiVec2, Window,
};
use plinth_geom::{vec2, Vec2};
use plinth_model::{Document, Element, GridId, GridLine, Level, LevelId, ViewDef, ViewKind};

// ===========================================================================
// Testable Level Management Operations
// ===========================================================================

/// Counts how many model elements are hosted on `level_id` (including wall-hosted doors/windows).
pub fn count_elements_on_level(doc: &Document, level_id: LevelId) -> usize {
    doc.elements
        .values()
        .filter(|el| match el {
            Element::Door(d) => d
                .base_level
                .or_else(|| match doc.elements.get(&d.host_wall) {
                    Some(Element::Wall(w)) => Some(w.base_level),
                    _ => None,
                })
                == Some(level_id),
            Element::Window(w) => w
                .base_level
                .or_else(|| match doc.elements.get(&w.host_wall) {
                    Some(Element::Wall(hw)) => Some(hw.base_level),
                    _ => None,
                })
                == Some(level_id),
            Element::CurtainWall(cw) => cw
                .base_level
                == level_id,
            other => other.level_id() == Some(level_id),
        })
        .count()
}

/// Ensures both a `FloorPlan` and `CeilingPlan` view exist for `level_id`.
/// Returns the number of newly created views.
pub fn ensure_views_for_level(doc: &mut Document, level_id: LevelId) -> usize {
    let Some(lvl_name) = doc.levels.get(&level_id).map(|l| l.name.clone()) else {
        return 0;
    };
    let has_fp = doc
        .views
        .values()
        .any(|v| v.kind == ViewKind::FloorPlan && v.level_id == Some(level_id));
    let has_rcp = doc
        .views
        .values()
        .any(|v| v.kind == ViewKind::CeilingPlan && v.level_id == Some(level_id));

    let mut added = 0;
    if !has_fp {
        let v = ViewDef::new_floor_plan(format!("Floor Plan: {}", lvl_name), level_id);
        doc.views.insert(v.id, v);
        added += 1;
    }
    if !has_rcp {
        let v = ViewDef::new_ceiling_plan(format!("Ceiling Plan: {}", lvl_name), level_id);
        doc.views.insert(v.id, v);
        added += 1;
    }
    added
}

/// Synchronizes standard Floor Plan and Ceiling Plan view titles when a level is renamed.
pub fn sync_level_view_names(doc: &mut Document, level_id: LevelId, new_name: &str) {
    for view in doc.views.values_mut() {
        if view.level_id == Some(level_id) {
            match view.kind {
                ViewKind::FloorPlan => {
                    view.name = format!("Floor Plan: {}", new_name);
                }
                ViewKind::CeilingPlan => {
                    view.name = format!("Ceiling Plan: {}", new_name);
                }
                _ => {}
            }
        }
    }
}

/// Adds a new building storey above the highest existing level.
pub fn add_level_above(doc: &mut Document, story_height_mm: f64, auto_create_views: bool) -> LevelId {
    doc.push_undo();
    let h = story_height_mm.max(500.0);

    let max_elev = doc
        .levels
        .values()
        .map(|l| l.elevation)
        .fold(f64::NEG_INFINITY, f64::max);
    let new_elev = if max_elev.is_finite() {
        max_elev + h
    } else {
        0.0
    };

    // Update any top level that previously had 0 story height
    for lvl in doc.levels.values_mut() {
        if (lvl.elevation - max_elev).abs() < 1.0 && lvl.story_height < 1.0 {
            lvl.story_height = h;
        }
    }

    let mut idx = doc
        .levels
        .values()
        .filter(|l| l.elevation >= -1.0)
        .count()
        + 1;
    let name = loop {
        let candidate = format!("Level {}", idx);
        if !doc.levels.values().any(|l| l.name.eq_ignore_ascii_case(&candidate)) {
            break candidate;
        }
        idx += 1;
    };

    let lvl = Level::new(name, new_elev, h);
    let id = lvl.id;
    doc.levels.insert(id, lvl);
    if auto_create_views {
        ensure_views_for_level(doc, id);
    }
    id
}

/// Adds a basement storey below the lowest existing level.
pub fn add_basement_below(doc: &mut Document, depth_mm: f64, auto_create_views: bool) -> LevelId {
    doc.push_undo();
    let h = depth_mm.abs().max(500.0);
    let min_elev = doc
        .levels
        .values()
        .map(|l| l.elevation)
        .fold(0.0_f64, f64::min);
    let new_elev = min_elev - h;

    let mut b_idx = doc.levels.values().filter(|l| l.elevation < -1.0).count() + 1;
    let name = loop {
        let candidate = format!("Basement B{}", b_idx);
        if !doc.levels.values().any(|l| l.name.eq_ignore_ascii_case(&candidate)) {
            break candidate;
        }
        b_idx += 1;
    };

    let lvl = Level::new(name, new_elev, h);
    let id = lvl.id;
    doc.levels.insert(id, lvl);
    if auto_create_views {
        ensure_views_for_level(doc, id);
    }
    id
}

/// Generates a full multi-storey stack (`Level 1..=storey_count` plus optional `Roof`),
/// reusing existing levels at matching elevations and creating missing levels & plan views.
pub fn generate_multi_storey_levels(
    doc: &mut Document,
    storey_count: usize,
    typical_height_mm: f64,
    include_roof: bool,
    auto_create_views: bool,
) -> Vec<LevelId> {
    doc.push_undo();
    let count = storey_count.clamp(1, 200);
    let h = typical_height_mm.max(1000.0);
    let mut created = Vec::new();

    // If an existing "Roof" level sits below the new roof height, move it up cleanly if it hosts no elements
    let target_roof_z = (count as f64) * h;
    let movable_roof_id = doc.levels.values().find_map(|l| {
        if l.name.eq_ignore_ascii_case("Roof")
            && l.elevation < target_roof_z - 1.0
            && count_elements_on_level(doc, l.id) == 0
        {
            Some(l.id)
        } else {
            None
        }
    });
    if let Some(rid) = movable_roof_id {
        if let Some(rl) = doc.levels.get_mut(&rid) {
            rl.elevation = target_roof_z;
        }
    }

    for i in 0..count {
        let elev = (i as f64) * h;
        let exists = doc
            .levels
            .values()
            .any(|l| (l.elevation - elev).abs() < 50.0);
        if !exists {
            let mut name = format!("Level {}", i + 1);
            if doc.levels.values().any(|l| l.name.eq_ignore_ascii_case(&name)) {
                name = format!("Storey {} (+{:.0})", i + 1, elev);
            }
            let lvl = Level::new(name, elev, h);
            let id = lvl.id;
            doc.levels.insert(id, lvl);
            if auto_create_views {
                ensure_views_for_level(doc, id);
            }
            created.push(id);
        }
    }

    if include_roof {
        let roof_exists = doc
            .levels
            .values()
            .any(|l| (l.elevation - target_roof_z).abs() < 50.0);
        if !roof_exists {
            let mut roof_name = "Roof".to_string();
            if doc
                .levels
                .values()
                .any(|l| l.name.eq_ignore_ascii_case(&roof_name))
            {
                roof_name = format!("Roof (+{:.0})", target_roof_z);
            }
            let r_lvl = Level::new(roof_name, target_roof_z, 0.0);
            let rid = r_lvl.id;
            doc.levels.insert(rid, r_lvl);
            if auto_create_views {
                ensure_views_for_level(doc, rid);
            }
            created.push(rid);
        }
    }

    created
}

/// Safely deletes a level when at least 2 levels exist, reassigning any hosted elements
/// to the nearest remaining level by elevation and cleaning up associated level views.
pub fn delete_level_safely(doc: &mut Document, target_id: LevelId) -> bool {
    if doc.levels.len() <= 1 || !doc.levels.contains_key(&target_id) {
        return false;
    }
    doc.push_undo();

    let target_elev = doc.level_elevation(target_id);
    let fallback_id = doc
        .levels
        .values()
        .filter(|l| l.id != target_id)
        .min_by(|a, b| {
            (a.elevation - target_elev)
                .abs()
                .partial_cmp(&(b.elevation - target_elev).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|l| l.id)
        .unwrap();

    // Reassign elements referencing target_id so no elements are orphaned
    for el in doc.elements.values_mut() {
        match el {
            Element::Wall(w) => {
                if w.base_level == target_id {
                    w.base_level = fallback_id;
                }
                if w.top_level == Some(target_id) {
                    w.top_level = None;
                }
            }
            Element::Door(d) => {
                if d.base_level == Some(target_id) {
                    d.base_level = None;
                }
                if d.top_level == Some(target_id) {
                    d.top_level = None;
                }
            }
            Element::Window(w) => {
                if w.base_level == Some(target_id) {
                    w.base_level = None;
                }
                if w.top_level == Some(target_id) {
                    w.top_level = None;
                }
            }
            Element::Column(c) => {
                if c.base_level == target_id {
                    c.base_level = fallback_id;
                }
                if c.top_level == Some(target_id) {
                    c.top_level = None;
                }
            }
            Element::Footing(f) => {
                if f.base_level == target_id {
                    f.base_level = fallback_id;
                }
                if f.top_level == Some(target_id) {
                    f.top_level = None;
                }
            }
            Element::Beam(b) => {
                if b.level == target_id {
                    b.level = fallback_id;
                }
                if b.top_level == Some(target_id) {
                    b.top_level = None;
                }
            }
            Element::Slab(s) => {
                if s.level == target_id {
                    s.level = fallback_id;
                }
                if s.top_level == Some(target_id) {
                    s.top_level = None;
                }
            }
            Element::Roof(r) => {
                if r.level == target_id {
                    r.level = fallback_id;
                }
                if r.top_level == Some(target_id) {
                    r.top_level = None;
                }
            }
            Element::Ceiling(c) => {
                if c.level == target_id {
                    c.level = fallback_id;
                }
                if c.top_level == Some(target_id) {
                    c.top_level = None;
                }
            }
            Element::Stair(s) => {
                if s.base_level == target_id {
                    s.base_level = fallback_id;
                }
                if s.top_level == Some(target_id) {
                    s.top_level = None;
                }
            }
            Element::Ramp(r) => {
                if r.base_level == target_id {
                    r.base_level = fallback_id;
                }
                if r.top_level == Some(target_id) {
                    r.top_level = None;
                }
            }
            Element::Railing(rl) => {
                if rl.level == target_id {
                    rl.level = fallback_id;
                }
                if rl.top_level == Some(target_id) {
                    rl.top_level = None;
                }
            }
            Element::Fascia(f) => {
                if f.level == target_id {
                    f.level = fallback_id;
                }
                if f.top_level == Some(target_id) {
                    f.top_level = None;
                }
            }
            Element::Gutter(g) => {
                if g.level == target_id {
                    g.level = fallback_id;
                }
                if g.top_level == Some(target_id) {
                    g.top_level = None;
                }
            }
            Element::CurtainWall(cw) => {
                if cw.base_level == target_id {
                    cw.base_level = fallback_id;
                }
                if cw.top_level == Some(target_id) {
                    cw.top_level = None;
                }
            }
            _ => {}
        }
    }

    doc.views.retain(|_, v| v.level_id != Some(target_id));
    doc.levels.remove(&target_id);

    if doc.active_level_id == target_id {
        doc.active_level_id = fallback_id;
    }
    if !doc.views.contains_key(&doc.active_view_id) {
        if let Some(&first_vid) = doc.views.keys().next() {
            doc.active_view_id = first_vid;
        }
    }
    true
}

// ===========================================================================
// Level Manager Dialog UI
// ===========================================================================

pub struct LevelManagerDialog {
    pub is_open: bool,
    pub add_storey_height: f64,
    pub basement_depth: f64,
    pub bulk_storey_count: usize,
    pub bulk_typical_height: f64,
    pub bulk_include_roof: bool,
    pub auto_create_views: bool,
    pub status_msg: String,
}

impl Default for LevelManagerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            add_storey_height: 3500.0,
            basement_depth: 3200.0,
            bulk_storey_count: 5,
            bulk_typical_height: 3500.0,
            bulk_include_roof: true,
            auto_create_views: true,
            status_msg: String::new(),
        }
    }
}

impl LevelManagerDialog {
    pub fn render(&mut self, ctx: &egui::Context, doc: &mut Document, theme: &AppTheme) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        Window::new("🏢 Level & Storey Manager")
            .open(&mut open)
            .default_size([880.0, 520.0])
            .min_width(720.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Project Building Storeys & Elevation Datums")
                            .strong()
                            .size(13.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut self.auto_create_views, "Auto-Create Floor & Ceiling Plans");
                    });
                });
                ui.separator();

                // Quick Add & Multi-Storey Generator Toolbar
                ui.horizontal_wrapped(|ui| {
                    ui.label("Storey ΔZ:");
                    ui.add(
                        DragValue::new(&mut self.add_storey_height)
                            .speed(50.0)
                            .range(1500.0..=20000.0)
                            .suffix(" mm"),
                    );
                    if ui
                        .button(RichText::new("➕ Add Level Above").strong())
                        .clicked()
                    {
                        let new_id =
                            add_level_above(doc, self.add_storey_height, self.auto_create_views);
                        let nm = doc
                            .levels
                            .get(&new_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_default();
                        self.status_msg = format!("Added storey '{}' above.", nm);
                    }

                    ui.separator();
                    ui.label("Basement D:");
                    ui.add(
                        DragValue::new(&mut self.basement_depth)
                            .speed(50.0)
                            .range(1800.0..=15000.0)
                            .suffix(" mm"),
                    );
                    if ui.button("⬇ Add Basement Below").clicked() {
                        let new_id =
                            add_basement_below(doc, self.basement_depth, self.auto_create_views);
                        let nm = doc
                            .levels
                            .get(&new_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_default();
                        self.status_msg = format!("Added basement '{}'.", nm);
                    }
                });

                CollapsingHeader::new("⚡ Multi-Storey Tower Generator")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Storeys:");
                            ui.add(DragValue::new(&mut self.bulk_storey_count).range(1..=120));
                            ui.label("Typical Height:");
                            ui.add(
                                DragValue::new(&mut self.bulk_typical_height)
                                    .speed(50.0)
                                    .range(2200.0..=12000.0)
                                    .suffix(" mm"),
                            );
                            ui.checkbox(&mut self.bulk_include_roof, "Include Roof Datum");
                            if ui
                                .button(
                                    RichText::new("⚡ Generate Multi-Storey Stack")
                                        .strong()
                                        .color(theme.accent),
                                )
                                .clicked()
                            {
                                let added = generate_multi_storey_levels(
                                    doc,
                                    self.bulk_storey_count,
                                    self.bulk_typical_height,
                                    self.bulk_include_roof,
                                    self.auto_create_views,
                                );
                                self.status_msg =
                                    format!("Generated {} new level(s) & plan views.", added.len());
                            }
                        });
                    });

                ui.add_space(4.0);

                // Sort level IDs descending by elevation (penthouse/roof at top, basements at bottom)
                let mut sorted_ids: Vec<LevelId> = doc.levels.keys().copied().collect();
                sorted_ids.sort_by(|a, b| {
                    let ea = doc.level_elevation(*a);
                    let eb = doc.level_elevation(*b);
                    eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
                });

                let elem_counts: Vec<(LevelId, usize)> = sorted_ids
                    .iter()
                    .map(|&lid| (lid, count_elements_on_level(doc, lid)))
                    .collect();

                let mut pending_delete: Option<LevelId> = None;
                let mut pending_ensure_views: Option<LevelId> = None;
                let mut renamed_level: Option<(LevelId, String)> = None;
                let mut activate_level: Option<LevelId> = None;
                let can_delete = sorted_ids.len() > 1;

                ui.columns(2, |cols| {
                    // Left column: Editable Storey Schedule
                    cols[0].label(RichText::new("Storey Schedule (Top → Bottom)").strong().size(12.0));
                    ScrollArea::vertical()
                        .id_salt("lvl_mgr_table_scroll")
                        .max_height(340.0)
                        .show(&mut cols[0], |ui| {
                            egui::Grid::new("lvl_mgr_grid")
                                .striped(true)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Active").strong().size(11.0));
                                    ui.label(RichText::new("Storey Name").strong().size(11.0));
                                    ui.label(RichText::new("Elevation (Z)").strong().size(11.0));
                                    ui.label(RichText::new("Height").strong().size(11.0));
                                    ui.label(RichText::new("IFC").strong().size(11.0));
                                    ui.label(RichText::new("Els").strong().size(11.0));
                                    ui.label(RichText::new("Actions").strong().size(11.0));
                                    ui.end_row();

                                    for &lid in &sorted_ids {
                                        let el_cnt = elem_counts
                                            .iter()
                                            .find(|(id, _)| *id == lid)
                                            .map(|(_, c)| *c)
                                            .unwrap_or(0);
                                        let has_fp = doc.views.values().any(|v| {
                                            v.kind == ViewKind::FloorPlan && v.level_id == Some(lid)
                                        });
                                        let is_active = doc.active_level_id == lid;

                                        if let Some(lvl) = doc.levels.get_mut(&lid) {
                                            if ui.radio(is_active, "").on_hover_text("Set as Active Modeling Level").clicked() {
                                                activate_level = Some(lid);
                                            }
                                            let prev_name = lvl.name.clone();
                                            let resp = ui.add(
                                                egui::TextEdit::singleline(&mut lvl.name)
                                                    .desired_width(110.0),
                                            );
                                            if resp.changed() && lvl.name != prev_name {
                                                renamed_level = Some((lid, lvl.name.clone()));
                                            }
                                            ui.add(
                                                DragValue::new(&mut lvl.elevation)
                                                    .speed(50.0)
                                                    .suffix(" mm"),
                                            );
                                            ui.add(
                                                DragValue::new(&mut lvl.story_height)
                                                    .speed(50.0)
                                                    .range(0.0..=30000.0)
                                                    .suffix(" mm"),
                                            );
                                            ui.checkbox(&mut lvl.is_building_storey, "");
                                            ui.label(format!("{}", el_cnt));

                                            ui.horizontal(|ui| {
                                                if !has_fp
                                                    && ui
                                                        .small_button("+Plan")
                                                        .on_hover_text("Create Floor & Ceiling Plans")
                                                        .clicked()
                                                {
                                                    pending_ensure_views = Some(lid);
                                                }
                                                if ui
                                                    .add_enabled(
                                                        can_delete,
                                                        egui::Button::new("🗑").small(),
                                                    )
                                                    .on_hover_text(
                                                        "Delete level (hosted elements reassign safely)",
                                                    )
                                                    .clicked()
                                                {
                                                    pending_delete = Some(lid);
                                                }
                                            });
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    // Right column: Live Storey Elevation Section Diagram
                    cols[1].label(
                        RichText::new("Elevation Datum Section Preview")
                            .strong()
                            .size(12.0),
                    );
                    draw_storey_section_diagram(&mut cols[1], doc, theme);
                });

                if let Some(lid) = activate_level {
                    doc.active_level_id = lid;
                    if let Some(fp_id) = doc
                        .views
                        .values()
                        .find(|v| v.kind == ViewKind::FloorPlan && v.level_id == Some(lid))
                        .map(|v| v.id)
                    {
                        doc.active_sheet_id = None;
                        doc.active_view_id = fp_id;
                    }
                }
                if let Some((lid, new_nm)) = renamed_level {
                    sync_level_view_names(doc, lid, &new_nm);
                }
                if let Some(lid) = pending_ensure_views {
                    let n = ensure_views_for_level(doc, lid);
                    self.status_msg = format!("Created {} plan view(s).", n);
                }
                if let Some(lid) = pending_delete {
                    if delete_level_safely(doc, lid) {
                        self.status_msg =
                            "Deleted level and safely reassigned any hosted elements.".into();
                    }
                }

                if !self.status_msg.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status_msg).size(11.0).color(theme.accent));
                }
            });

        self.is_open = open;
    }
}

fn draw_storey_section_diagram(ui: &mut Ui, doc: &Document, theme: &AppTheme) {
    let desired = EguiVec2::new(ui.available_width().max(240.0), 330.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);

    painter.rect_filled(rect, 6.0, Color32::from_rgb(20, 24, 31));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, theme.border_color),
        egui::StrokeKind::Inside,
    );

    let mut levels: Vec<&Level> = doc.levels.values().collect();
    if levels.is_empty() {
        return;
    }
    levels.sort_by(|a, b| {
        a.elevation
            .partial_cmp(&b.elevation)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let z_min = levels.first().unwrap().elevation.min(0.0) - 1200.0;
    let z_max = levels
        .last()
        .map(|l| l.elevation + l.story_height.max(1500.0))
        .unwrap_or(4000.0)
        + 1000.0;
    let z_span = (z_max - z_min).max(2000.0);

    let to_y = |z: f64| -> f32 {
        let t = ((z - z_min) / z_span) as f32;
        rect.bottom() - 22.0 - t * (rect.height() - 44.0)
    };

    // Ground Z=0 earth line
    let y0 = to_y(0.0);
    painter.line_segment(
        [Pos2::new(rect.left() + 14.0, y0), Pos2::new(rect.right() - 14.0, y0)],
        Stroke::new(1.5, Color32::from_rgb(110, 145, 95)),
    );
    for hx in (20..((rect.width() as i32) - 20)).step_by(14) {
        let px = rect.left() + hx as f32;
        painter.line_segment(
            [Pos2::new(px, y0), Pos2::new(px - 6.0, y0 + 7.0)],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(110, 145, 95, 110)),
        );
    }

    // Building core silhouette
    let b_left = rect.left() + 48.0;
    let b_right = (rect.right() - 150.0).max(b_left + 50.0);
    let y_bot = to_y(levels.first().unwrap().elevation);
    let y_top = to_y(levels.last().unwrap().elevation);
    if y_bot > y_top + 4.0 {
        let core_rect = Rect::from_min_max(Pos2::new(b_left, y_top), Pos2::new(b_right, y_bot));
        painter.rect_filled(
            core_rect,
            0.0,
            Color32::from_rgba_unmultiplied(70, 130, 195, 38),
        );
        painter.rect_stroke(
            core_rect,
            0.0,
            Stroke::new(1.2, Color32::from_rgb(95, 155, 220)),
            egui::StrokeKind::Inside,
        );
    }

    // Datum lines & architectural level head symbols
    for (idx, lvl) in levels.iter().enumerate() {
        let y = to_y(lvl.elevation);
        let is_active = lvl.id == doc.active_level_id;
        let line_col = if is_active {
            Color32::from_rgb(255, 145, 30)
        } else {
            Color32::from_rgb(135, 185, 240)
        };

        // Dashed datum line
        let mut x = rect.left() + 20.0;
        let x_end = rect.right() - 36.0;
        while x < x_end {
            let x2 = (x + 9.0).min(x_end);
            painter.line_segment(
                [Pos2::new(x, y), Pos2::new(x2, y)],
                Stroke::new(if is_active { 1.8 } else { 1.1 }, line_col),
            );
            x += 14.0;
        }

        // Datum circle target symbol on right
        let head_c = Pos2::new(rect.right() - 24.0, y);
        painter.circle_filled(head_c, 7.5, Color32::from_rgb(24, 30, 40));
        painter.circle_stroke(head_c, 7.5, Stroke::new(1.4, line_col));
        painter.line_segment(
            [Pos2::new(head_c.x - 7.5, head_c.y), Pos2::new(head_c.x + 7.5, head_c.y)],
            Stroke::new(1.0, line_col),
        );
        painter.line_segment(
            [Pos2::new(head_c.x, head_c.y - 7.5), Pos2::new(head_c.x, head_c.y + 7.5)],
            Stroke::new(1.0, line_col),
        );

        // Label text
        let label = format!("{} ({:+.0} mm)", lvl.name, lvl.elevation);
        painter.text(
            Pos2::new(head_c.x - 12.0, y - 8.0),
            Align2::RIGHT_BOTTOM,
            label,
            FontId::proportional(10.5),
            if is_active {
                Color32::from_rgb(255, 195, 110)
            } else {
                Color32::from_rgb(220, 230, 242)
            },
        );

        // Inter-storey ΔZ annotation
        if idx + 1 < levels.len() {
            let next_y = to_y(levels[idx + 1].elevation);
            let dz = levels[idx + 1].elevation - lvl.elevation;
            if (y - next_y).abs() > 14.0 {
                let mid_y = (y + next_y) * 0.5;
                painter.text(
                    Pos2::new(rect.left() + 24.0, mid_y),
                    Align2::LEFT_CENTER,
                    format!("Δ{:.0}", dz),
                    FontId::proportional(9.5),
                    Color32::from_rgb(150, 165, 185),
                );
            }
        }
    }
}

// ===========================================================================
// Testable Grid Management Operations
// ===========================================================================

/// Converts zero-based index to alphabetical grid tag (`0 -> "A"`, `25 -> "Z"`, `26 -> "AA"`).
pub fn alpha_grid_label(mut idx: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'A' + (idx % 26) as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    chars.iter().rev().collect()
}

/// Parses custom bay spacings string (e.g. `"6000, 7500, 6000"` or `"3*6000, 4500"`),
/// or falls back to `(grid_count - 1)` bays of `uniform_spacing`.
pub fn parse_spacings_input(custom_csv: &str, grid_count: usize, uniform_spacing: f64) -> Vec<f64> {
    let trimmed = custom_csv.trim();
    if !trimmed.is_empty() {
        let mut out = Vec::new();
        for token in trimmed.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            let tok = token.trim();
            if tok.is_empty() {
                continue;
            }
            if let Some((cnt_s, val_s)) = tok.split_once('*').or_else(|| tok.split_once('x')) {
                if let (Ok(cnt), Ok(val)) = (cnt_s.trim().parse::<usize>(), val_s.trim().parse::<f64>()) {
                    if val > 100.0 {
                        for _ in 0..cnt.min(100) {
                            out.push(val);
                        }
                    }
                }
            } else if let Ok(val) = tok.parse::<f64>() {
                if val > 100.0 {
                    out.push(val);
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }

    let bays = grid_count.saturating_sub(1);
    vec![uniform_spacing.max(500.0); bays]
}

/// Generates an orthogonal X/Y structural grid system in `doc.grids`.
/// `x_spacings` defines distances between vertical grid lines (constant X, running along Y).
/// `y_spacings` defines distances between horizontal grid lines (constant Y, running along X).
/// Returns total number of grid lines created.
pub fn generate_orthogonal_grids(
    doc: &mut Document,
    origin: Vec2,
    x_spacings: &[f64],
    y_spacings: &[f64],
    x_alpha_labels: bool,
    overhang: f64,
    replace_existing: bool,
) -> usize {
    doc.push_undo();
    if replace_existing {
        doc.grids.clear();
    }

    let mut xs = vec![origin.x];
    for &dx in x_spacings {
        xs.push(xs.last().copied().unwrap_or(origin.x) + dx.max(100.0));
    }

    let mut ys = vec![origin.y];
    for &dy in y_spacings {
        ys.push(ys.last().copied().unwrap_or(origin.y) + dy.max(100.0));
    }

    let ext = overhang.max(500.0);
    let x_min = xs.first().copied().unwrap_or(origin.x) - ext;
    let x_max = xs.last().copied().unwrap_or(origin.x + 6000.0) + ext;
    let y_min = ys.first().copied().unwrap_or(origin.y) - ext;
    let y_max = ys.last().copied().unwrap_or(origin.y + 6000.0) + ext;

    let mut created = 0;

    // Vertical grid lines (constant X, spanning y_min..y_max)
    for (i, &x) in xs.iter().enumerate() {
        let label = if x_alpha_labels {
            alpha_grid_label(i)
        } else {
            format!("{}", i + 1)
        };
        let g = GridLine::new(label, vec2(x, y_min), vec2(x, y_max));
        doc.grids.insert(g.id, g);
        created += 1;
    }

    // Horizontal grid lines (constant Y, spanning x_min..x_max)
    for (j, &y) in ys.iter().enumerate() {
        let label = if x_alpha_labels {
            format!("{}", j + 1)
        } else {
            alpha_grid_label(j)
        };
        let g = GridLine::new(label, vec2(x_min, y), vec2(x_max, y));
        doc.grids.insert(g.id, g);
        created += 1;
    }

    created
}

/// Aligns the start/end tails of all orthogonal (horizontal & vertical) grid lines cleanly.
pub fn auto_align_grid_extents(doc: &mut Document, overhang: f64) {
    if doc.grids.is_empty() {
        return;
    }
    doc.push_undo();
    let ext = overhang.max(500.0);

    let mut v_xs = Vec::new();
    let mut h_ys = Vec::new();
    for g in doc.grids.values() {
        let d = g.end - g.start;
        if d.x.abs() < 1.0 && d.y.abs() > 10.0 {
            v_xs.push(g.start.x);
        } else if d.y.abs() < 1.0 && d.x.abs() > 10.0 {
            h_ys.push(g.start.y);
        }
    }

    if v_xs.is_empty() || h_ys.is_empty() {
        return;
    }

    let min_x = v_xs.iter().copied().fold(f64::INFINITY, f64::min) - ext;
    let max_x = v_xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + ext;
    let min_y = h_ys.iter().copied().fold(f64::INFINITY, f64::min) - ext;
    let max_y = h_ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) + ext;

    for g in doc.grids.values_mut() {
        let d = g.end - g.start;
        if d.x.abs() < 1.0 && d.y.abs() > 10.0 {
            let x = g.start.x;
            g.start = vec2(x, min_y);
            g.end = vec2(x, max_y);
        } else if d.y.abs() < 1.0 && d.x.abs() > 10.0 {
            let y = g.start.y;
            g.start = vec2(min_x, y);
            g.end = vec2(max_x, y);
        }
    }
}

// ===========================================================================
// Grid Manager Dialog UI
// ===========================================================================

pub struct GridManagerDialog {
    pub is_open: bool,
    pub x_grid_count: usize,
    pub x_spacing_mm: f64,
    pub x_custom_spacings: String,
    pub y_grid_count: usize,
    pub y_spacing_mm: f64,
    pub y_custom_spacings: String,
    pub x_alpha_labels: bool,
    pub origin_x: f64,
    pub origin_y: f64,
    pub overhang_mm: f64,
    pub replace_existing: bool,
    pub status_msg: String,
}

impl Default for GridManagerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            x_grid_count: 4,
            x_spacing_mm: 6000.0,
            x_custom_spacings: String::new(),
            y_grid_count: 4,
            y_spacing_mm: 6000.0,
            y_custom_spacings: String::new(),
            x_alpha_labels: true,
            origin_x: 0.0,
            origin_y: 0.0,
            overhang_mm: 1800.0,
            replace_existing: true,
            status_msg: String::new(),
        }
    }
}

impl GridManagerDialog {
    pub fn render(&mut self, ctx: &egui::Context, doc: &mut Document, theme: &AppTheme) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        Window::new("📏 Structural & Architectural Grid Manager")
            .open(&mut open)
            .default_size([900.0, 540.0])
            .min_width(740.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Orthogonal Grid Generator Panel
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("⚡ Orthogonal Structural Bay Grid Generator")
                                .strong()
                                .size(12.5),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(
                                &mut self.x_alpha_labels,
                                "X-Grids = A,B,C / Y-Grids = 1,2,3",
                            );
                        });
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("X-Axis (Vertical Lines):").strong());
                        ui.label("Lines:");
                        ui.add(DragValue::new(&mut self.x_grid_count).range(2..=60));
                        ui.label("Spacing:");
                        ui.add(
                            DragValue::new(&mut self.x_spacing_mm)
                                .speed(100.0)
                                .range(500.0..=50000.0)
                                .suffix(" mm"),
                        );
                        ui.label("Custom Bays:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.x_custom_spacings)
                                .hint_text("e.g. 6000, 7500, 6000")
                                .desired_width(145.0),
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Y-Axis (Horizontal Lines):").strong());
                        ui.label("Lines:");
                        ui.add(DragValue::new(&mut self.y_grid_count).range(2..=60));
                        ui.label("Spacing:");
                        ui.add(
                            DragValue::new(&mut self.y_spacing_mm)
                                .speed(100.0)
                                .range(500.0..=50000.0)
                                .suffix(" mm"),
                        );
                        ui.label("Custom Bays:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.y_custom_spacings)
                                .hint_text("e.g. 3*6000, 4500")
                                .desired_width(145.0),
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label("Origin X:");
                        ui.add(DragValue::new(&mut self.origin_x).speed(100.0).suffix(" mm"));
                        ui.label("Origin Y:");
                        ui.add(DragValue::new(&mut self.origin_y).speed(100.0).suffix(" mm"));
                        ui.label("Tail Overhang:");
                        ui.add(
                            DragValue::new(&mut self.overhang_mm)
                                .speed(50.0)
                                .range(500.0..=10000.0)
                                .suffix(" mm"),
                        );
                        ui.checkbox(&mut self.replace_existing, "Replace Existing");

                        if ui
                            .button(
                                RichText::new("⚡ Generate Orthogonal Grid")
                                    .strong()
                                    .color(theme.accent),
                            )
                            .clicked()
                        {
                            let x_sp = parse_spacings_input(
                                &self.x_custom_spacings,
                                self.x_grid_count,
                                self.x_spacing_mm,
                            );
                            let y_sp = parse_spacings_input(
                                &self.y_custom_spacings,
                                self.y_grid_count,
                                self.y_spacing_mm,
                            );
                            let total = generate_orthogonal_grids(
                                doc,
                                vec2(self.origin_x, self.origin_y),
                                &x_sp,
                                &y_sp,
                                self.x_alpha_labels,
                                self.overhang_mm,
                                self.replace_existing,
                            );
                            let ix_cnt = (x_sp.len() + 1) * (y_sp.len() + 1);
                            self.status_msg = format!(
                                "Generated {} grid lines ({} structural intersections).",
                                total, ix_cnt
                            );
                        }
                    });
                });

                ui.add_space(4.0);

                // Quick Manual Grid Toolbar
                ui.horizontal_wrapped(|ui| {
                    if ui.button("➕ Add Vertical Grid").clicked() {
                        doc.push_undo();
                        let max_x = doc
                            .grids
                            .values()
                            .filter(|g| (g.end.x - g.start.x).abs() < 1.0)
                            .map(|g| g.start.x)
                            .fold(f64::NEG_INFINITY, f64::max);
                        let next_x = if max_x.is_finite() {
                            max_x + self.x_spacing_mm
                        } else {
                            self.origin_x
                        };
                        let v_cnt = doc
                            .grids
                            .values()
                            .filter(|g| (g.end.x - g.start.x).abs() < 1.0)
                            .count();
                        let tag = alpha_grid_label(v_cnt);
                        let g = GridLine::new(
                            tag,
                            vec2(next_x, self.origin_y - self.overhang_mm),
                            vec2(next_x, self.origin_y + 18000.0 + self.overhang_mm),
                        );
                        doc.grids.insert(g.id, g);
                    }
                    if ui.button("➕ Add Horizontal Grid").clicked() {
                        doc.push_undo();
                        let max_y = doc
                            .grids
                            .values()
                            .filter(|g| (g.end.y - g.start.y).abs() < 1.0)
                            .map(|g| g.start.y)
                            .fold(f64::NEG_INFINITY, f64::max);
                        let next_y = if max_y.is_finite() {
                            max_y + self.y_spacing_mm
                        } else {
                            self.origin_y
                        };
                        let h_cnt = doc
                            .grids
                            .values()
                            .filter(|g| (g.end.y - g.start.y).abs() < 1.0)
                            .count();
                        let tag = format!("{}", h_cnt + 1);
                        let g = GridLine::new(
                            tag,
                            vec2(self.origin_x - self.overhang_mm, next_y),
                            vec2(self.origin_x + 18000.0 + self.overhang_mm, next_y),
                        );
                        doc.grids.insert(g.id, g);
                    }
                    if ui.button("⇄ Align Grid Tails").clicked() {
                        auto_align_grid_extents(doc, self.overhang_mm);
                        self.status_msg = "Aligned orthogonal grid tails.".into();
                    }
                    if !doc.grids.is_empty() && ui.button("🗑 Clear All Grids").clicked() {
                        doc.push_undo();
                        doc.grids.clear();
                        self.status_msg = "Cleared all datum grids.".into();
                    }
                });

                ui.separator();

                let mut delete_gid: Option<GridId> = None;

                ui.columns(2, |cols| {
                    // Left column: Editable Grid Line Table
                    cols[0].label(
                        RichText::new(format!("Active Datum Grids ({})", doc.grids.len()))
                            .strong()
                            .size(12.0),
                    );
                    ScrollArea::vertical()
                        .id_salt("grid_mgr_table_scroll")
                        .max_height(280.0)
                        .show(&mut cols[0], |ui| {
                            if doc.grids.is_empty() {
                                ui.label(
                                    "No grids in document yet. Click '⚡ Generate Orthogonal Grid' above.",
                                );
                            } else {
                                egui::Grid::new("grid_mgr_table")
                                    .striped(true)
                                    .spacing([6.0, 5.0])
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Tag").strong().size(11.0));
                                        ui.label(RichText::new("Start X").strong().size(11.0));
                                        ui.label(RichText::new("Start Y").strong().size(11.0));
                                        ui.label(RichText::new("End X").strong().size(11.0));
                                        ui.label(RichText::new("End Y").strong().size(11.0));
                                        ui.label(RichText::new("Axis").strong().size(11.0));
                                        ui.label("");
                                        ui.end_row();

                                        for (&gid, g) in doc.grids.iter_mut() {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut g.name)
                                                    .desired_width(38.0),
                                            );
                                            ui.add(DragValue::new(&mut g.start.x).speed(50.0));
                                            ui.add(DragValue::new(&mut g.start.y).speed(50.0));
                                            ui.add(DragValue::new(&mut g.end.x).speed(50.0));
                                            ui.add(DragValue::new(&mut g.end.y).speed(50.0));
                                            let d = g.end - g.start;
                                            let axis_lbl = if d.x.abs() < 1.0 {
                                                "Vert"
                                            } else if d.y.abs() < 1.0 {
                                                "Horiz"
                                            } else {
                                                "Diag"
                                            };
                                            ui.label(axis_lbl);
                                            if ui.small_button("🗑").clicked() {
                                                delete_gid = Some(gid);
                                            }
                                            ui.end_row();
                                        }
                                    });
                            }
                        });

                    // Right column: Live 2D Grid Blueprint & Intersection Preview
                    cols[1].label(
                        RichText::new("2D Structural Grid & Intersection Blueprint")
                            .strong()
                            .size(12.0),
                    );
                    draw_grid_blueprint_preview(&mut cols[1], doc, theme);
                });

                if let Some(gid) = delete_gid {
                    doc.push_undo();
                    doc.grids.remove(&gid);
                }

                if !self.status_msg.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status_msg).size(11.0).color(theme.accent));
                }
            });

        self.is_open = open;
    }
}

fn draw_grid_blueprint_preview(ui: &mut Ui, doc: &Document, theme: &AppTheme) {
    let desired = EguiVec2::new(ui.available_width().max(240.0), 275.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);

    painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 23, 30));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, theme.border_color),
        egui::StrokeKind::Inside,
    );

    let grids: Vec<&GridLine> = doc.grids.values().collect();
    if grids.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No Datum Grids — Use Generator Above",
            FontId::proportional(11.5),
            Color32::from_rgb(145, 160, 180),
        );
        return;
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for g in &grids {
        min_x = min_x.min(g.start.x).min(g.end.x);
        max_x = max_x.max(g.start.x).max(g.end.x);
        min_y = min_y.min(g.start.y).min(g.end.y);
        max_y = max_y.max(g.start.y).max(g.end.y);
    }
    let span_x = (max_x - min_x).max(1000.0);
    let span_y = (max_y - min_y).max(1000.0);
    let pad = 28.0_f32;
    let scale = ((rect.width() - 2.0 * pad) / span_x as f32)
        .min((rect.height() - 2.0 * pad) / span_y as f32)
        .max(0.0001);

    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let to_sp = |p: Vec2| -> Pos2 {
        Pos2::new(
            rect.center().x + ((p.x - cx) as f32) * scale,
            rect.center().y - ((p.y - cy) as f32) * scale,
        )
    };

    let grid_col = Color32::from_rgb(105, 175, 240);
    for g in &grids {
        let p1 = to_sp(g.start);
        let p2 = to_sp(g.end);
        painter.line_segment([p1, p2], Stroke::new(1.1, grid_col));

        let dir = (p2 - p1).normalized();
        let bub_c = p2 + dir * 10.0;
        painter.circle_filled(bub_c, 8.0, Color32::from_rgb(24, 34, 48));
        painter.circle_stroke(bub_c, 8.0, Stroke::new(1.2, grid_col));
        painter.text(
            bub_c,
            Align2::CENTER_CENTER,
            &g.name,
            FontId::proportional(9.5),
            Color32::WHITE,
        );
    }

    // Highlight intersections
    let mut ix_count = 0usize;
    for i in 0..grids.len() {
        let r = grids[i].end - grids[i].start;
        for j in (i + 1)..grids.len() {
            let s = grids[j].end - grids[j].start;
            if let Some((t, u)) =
                plinth_geom::polygon::line_intersection(grids[i].start, r, grids[j].start, s)
            {
                if (-1e-4..=1.0001).contains(&t) && (-1e-4..=1.0001).contains(&u) {
                    ix_count += 1;
                    let sp = to_sp(grids[i].start + r * t);
                    painter.circle_filled(sp, 3.0, Color32::from_rgb(255, 140, 20));
                }
            }
        }
    }

    painter.text(
        Pos2::new(rect.left() + 10.0, rect.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        format!("{} Grids  •  {} Intersections", grids.len(), ix_count),
        FontId::proportional(10.5),
        Color32::from_rgb(255, 175, 80),
    );
}
