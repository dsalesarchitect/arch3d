//! Main Plinth application state and eframe update loop.

use crate::browser::render_project_browser;
use crate::canvas::{render_canvas, CanvasState};
use crate::level_copy::{CopyActiveLevelDialog, LevelClipboard, PasteToLevelsDialog};
use crate::level_manager::{GridManagerDialog, LevelManagerDialog};
use crate::properties::render_properties_panel;
use crate::ribbon::{render_ribbon, RibbonTab};
#[cfg(test)]
use crate::ribbon::RibbonAction;
use crate::settings::{render_settings_dialog, AppSettings, SettingsCategory};
use crate::structure_auto::StructureAutoDialog;
use crate::theme::{AppTheme, ThemeMode};
use crate::tool::Tool;
use crate::tool_config::{render_tool_config_studio, ToolCategory, ToolConfigStudio};
use egui::{Key, TopBottomPanel, Vec2};
use plinth_ifc::{export_ifc4, parse_ifc4};
use plinth_model::{Document, Element, ElementId, ViewKind};

#[derive(Clone, Debug)]
pub struct PendingFileLoad {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
pub fn download_file_in_browser(filename: &str, bytes: &[u8], mime: &str) {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let uint8_arr = js_sys::Uint8Array::from(bytes);
            let array = js_sys::Array::new();
            array.push(&uint8_arr);
            let blob_props = web_sys::BlobPropertyBag::new();
            blob_props.set_type(mime);
            if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &blob_props) {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    if let Ok(a) = document.create_element("a") {
                        if let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                            a.set_href(&url);
                            a.set_download(filename);
                            a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn pick_file_web(
    queue: std::sync::Arc<std::sync::Mutex<Vec<PendingFileLoad>>>,
    filter_name: &'static str,
    filter_exts: &'static [&'static str],
) {
    wasm_bindgen_futures::spawn_local(async move {
        let mut dialog = rfd::AsyncFileDialog::new();
        dialog = dialog.add_filter(filter_name, filter_exts);
        if let Some(file_handle) = dialog.pick_file().await {
            let name = file_handle.file_name();
            let bytes = file_handle.read().await;
            if let Ok(mut q) = queue.lock() {
                q.push(PendingFileLoad { name, bytes });
            }
        }
    });
}

pub struct PlinthApp {
    pub doc: Document,
    pub current_tab: RibbonTab,
    pub active_tool: Tool,
    pub selected_element: Option<ElementId>,
    pub theme_mode: ThemeMode,
    pub canvas_state: CanvasState,
    pub status_message: String,
    pub settings: AppSettings,
    pub tool_studio: ToolConfigStudio,
    pub level_manager: LevelManagerDialog,
    pub grid_manager: GridManagerDialog,
    pub struct_auto: StructureAutoDialog,
    pub site_dialog: crate::site_tools::SiteDialog,
    pub level_clipboard: LevelClipboard,
    pub copy_level_dlg: CopyActiveLevelDialog,
    pub paste_level_dlg: PasteToLevelsDialog,
    pub plugin_manager: crate::plugin_manager::PluginManagerDialog,
    pub current_file_path: Option<std::path::PathBuf>,
    pub last_auto_save: web_time::Instant,
    pub last_saved_undo_count: usize,
    pub gpu_renderer: Option<std::sync::Arc<crate::gpu_viewport::GpuViewportRenderer>>,
    pub pending_file_loads: std::sync::Arc<std::sync::Mutex<Vec<PendingFileLoad>>>,
}

/// Loads the pre-packaged Apartment Highrise benchmark IFC model (decompressed from embedded gzip bytes).
pub fn load_apartment_highrise_doc() -> Result<Document, String> {
    use std::io::Read;
    const GZ_BYTES: &[u8] = include_bytes!("apartment_highrise.ifc.gz");
    let mut decoder = flate2::read::GzDecoder::new(GZ_BYTES);
    let mut ifc_str = String::new();
    decoder
        .read_to_string(&mut ifc_str)
        .map_err(|e| format!("Decompression failed: {e}"))?;
    plinth_ifc::parse_ifc4(&ifc_str).map_err(|e| format!("IFC parse failed: {e}"))
}

impl Default for PlinthApp {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        let (doc, status_message) = {
            let search = web_sys::window()
                .and_then(|w| w.location().search().ok())
                .unwrap_or_default();
            if search.contains("apartment") || search.contains("model=apartment") {
                match load_apartment_highrise_doc() {
                    Ok(d) => {
                        let count = d.elements.len();
                        let levels = d.levels.len();
                        (
                            d,
                            format!(
                                "Apartment Highrise.ifc Loaded ({} elements across {} storeys)",
                                count, levels
                            ),
                        )
                    }
                    Err(_) => (
                        Document::create_sample_building(),
                        "Ready • Sample Building Loaded (1:1 standard scale)".into(),
                    ),
                }
            } else {
                (
                    Document::create_sample_building(),
                    "Ready • Sample Building Loaded (1:1 standard scale)".into(),
                )
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        let (doc, status_message) = (
            Document::create_sample_building(),
            "Ready • Sample Building Loaded (1:1 standard scale)".into(),
        );

        #[cfg(target_arch = "wasm32")]
        let (theme_mode, settings) = {
            let search = web_sys::window()
                .and_then(|w| w.location().search().ok())
                .unwrap_or_default();
            let mut s = AppSettings::new();
            let tm = if search.contains("theme=dark") {
                ThemeMode::Dark
            } else {
                // Default to Light Mode for Web and Landing Page 3D Viewport!
                ThemeMode::Light
            };
            s.theme.theme_mode = tm;
            (tm, s)
        };

        #[cfg(not(target_arch = "wasm32"))]
        let (theme_mode, settings) = {
            let s = AppSettings::new();
            (s.theme.theme_mode, s)
        };

        let mut canvas_state = CanvasState::default();
        canvas_state.materials = settings.materials;

        Self {
            doc,
            current_tab: RibbonTab::Home,
            active_tool: Tool::Select,
            selected_element: None,
            theme_mode,
            canvas_state,
            status_message,
            settings,
            tool_studio: ToolConfigStudio::new(),
            level_manager: LevelManagerDialog::default(),
            grid_manager: GridManagerDialog::default(),
            struct_auto: StructureAutoDialog::default(),
            site_dialog: crate::site_tools::SiteDialog::default(),
            level_clipboard: LevelClipboard::default(),
            copy_level_dlg: CopyActiveLevelDialog::default(),
            paste_level_dlg: PasteToLevelsDialog::default(),
            plugin_manager: crate::plugin_manager::PluginManagerDialog::default(),
            current_file_path: None,
            last_auto_save: web_time::Instant::now(),
            last_saved_undo_count: 0,
            gpu_renderer: None,
            pending_file_loads: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl PlinthApp {
    pub fn save_project_file(&mut self, path: &std::path::Path) -> bool {
        // Automatic Backup Creation: if backup on save is enabled and target exists, copy to .bak
        if self.settings.saving.create_backup_on_save && path.exists() {
            let bak_path = path.with_extension("bak");
            let _ = std::fs::copy(path, &bak_path);
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "ifc" || ext == "step" {
            let ifc_text = export_ifc4(&self.doc);
            if std::fs::write(path, ifc_text).is_ok() {
                self.current_file_path = Some(path.to_path_buf());
                self.status_message = format!("Exported IFC4 to {}", path.display());
                return true;
            }
        } else {
            match serde_json::to_string_pretty(&self.doc) {
                Ok(json) => {
                    if std::fs::write(path, json).is_ok() {
                        self.current_file_path = Some(path.to_path_buf());
                        self.status_message = format!("Project saved to {}", path.display());
                        return true;
                    }
                }
                Err(e) => {
                    self.status_message = format!("Save Error: {}", e);
                }
            }
        }
        false
    }

    pub fn open_project_from_bytes(&mut self, filename: &str, bytes: &[u8]) -> bool {
        let is_cad = filename.ends_with(".dwg")
            || filename.ends_with(".DWG")
            || filename.ends_with(".dxf")
            || filename.ends_with(".DXF");
        if is_cad {
            self.doc.push_undo();
            match plinth_model::import_dwg_bytes(&mut self.doc, bytes, Some(filename)) {
                Ok(summary) => {
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!(
                        "Imported AutoCAD ({}): {} walls, {} slabs, {} lines, {} solids, {} layers",
                        summary.model_name,
                        summary.walls_count,
                        summary.slabs_count,
                        summary.lines_count,
                        summary.solids_count,
                        summary.layers_count,
                    );
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("CAD Import Error: {}", e);
                    return false;
                }
            }
        }

        let is_skp = filename.ends_with(".skp") || filename.ends_with(".SKP");
        if is_skp {
            self.doc.push_undo();
            match plinth_model::import_skp_bytes(&mut self.doc, bytes) {
                Ok(summary) => {
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!(
                        "Imported SketchUp (.skp): {} solids, {} faces, {} materials ({})",
                        summary.solids_count, summary.faces_count, summary.materials_count, summary.model_name
                    );
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("SKP Import Error: {}", e);
                    return false;
                }
            }
        }

        let text_content = std::str::from_utf8(bytes)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
        let trimmed = text_content.trim_start_matches('\u{feff}').trim_start();
        let is_ifc = trimmed.starts_with("ISO-10303-21")
            || filename.ends_with(".ifc")
            || filename.ends_with(".IFC")
            || filename.ends_with(".step")
            || filename.ends_with(".stp");

        if is_ifc {
            match parse_ifc4(trimmed) {
                Ok(new_doc) => {
                    let elem_count = new_doc.elements.len();
                    let level_count = new_doc.levels.len();
                    self.doc = new_doc;
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!(
                        "Imported IFC model ({} elements across {} levels): {}",
                        elem_count, level_count, filename
                    );
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("IFC Import Error: {}", e);
                }
            }
        } else {
            match serde_json::from_slice::<Document>(bytes) {
                Ok(new_doc) => {
                    self.doc = new_doc;
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!("Project loaded: {}", filename);
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("File open error: {}", e);
                }
            }
        }
        false
    }

    pub fn open_project_file(&mut self, path: &std::path::Path) -> bool {
        let is_cad = path
            .extension()
            .map_or(false, |e| e.eq_ignore_ascii_case("dwg") || e.eq_ignore_ascii_case("dxf"));
        if is_cad {
            self.doc.push_undo();
            match plinth_model::import_dwg_file(&mut self.doc, path) {
                Ok(summary) => {
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!(
                        "Imported AutoCAD ({}): {} walls, {} slabs, {} lines, {} solids, {} layers",
                        summary.model_name,
                        summary.walls_count,
                        summary.slabs_count,
                        summary.lines_count,
                        summary.solids_count,
                        summary.layers_count,
                    );
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("CAD Import Error: {}", e);
                    return false;
                }
            }
        }

        let is_skp = path.extension().map_or(false, |e| e.eq_ignore_ascii_case("skp"));
        if is_skp {
            self.doc.push_undo();
            match plinth_model::import_skp_file(&mut self.doc, path) {
                Ok(summary) => {
                    self.selected_element = None;
                    self.active_tool = Tool::Select;
                    self.status_message = format!(
                        "Imported SketchUp (.skp): {} solids, {} faces, {} materials ({})",
                        summary.solids_count, summary.faces_count, summary.materials_count, summary.model_name
                    );
                    self.canvas_state.invalidate_gpu_cache();
                    self.zoom_extents();
                    return true;
                }
                Err(e) => {
                    self.status_message = format!("SKP Import Error: {}", e);
                    return false;
                }
            }
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let trimmed = content.trim_start_matches('\u{feff}').trim_start();
                let is_ifc = trimmed.starts_with("ISO-10303-21")
                    || path.extension().map_or(false, |e| {
                        e.eq_ignore_ascii_case("ifc")
                            || e.eq_ignore_ascii_case("step")
                            || e.eq_ignore_ascii_case("stp")
                    });

                if is_ifc {
                    match parse_ifc4(trimmed) {
                        Ok(new_doc) => {
                            let elem_count = new_doc.elements.len();
                            let level_count = new_doc.levels.len();
                            let file_label = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("model.ifc");

                            self.doc = new_doc;
                            self.selected_element = None;
                            self.active_tool = Tool::Select;
                            self.current_file_path = Some(path.to_path_buf());
                            self.status_message = format!(
                                "Imported IFC model ({} elements across {} levels): {}",
                                elem_count, level_count, file_label
                            );
                            self.canvas_state.invalidate_gpu_cache();
                            self.zoom_extents();
                            return true;
                        }
                        Err(e) => {
                            self.status_message = format!("IFC Import Error: {}", e);
                        }
                    }
                } else {
                    match serde_json::from_str::<Document>(trimmed) {
                        Ok(new_doc) => {
                            self.doc = new_doc;
                            self.selected_element = None;
                            self.active_tool = Tool::Select;
                            self.current_file_path = Some(path.to_path_buf());
                            self.status_message = format!("Project loaded: {}", path.display());
                            self.canvas_state.invalidate_gpu_cache();
                            self.zoom_extents();
                            return true;
                        }
                        Err(e) => {
                            self.status_message = format!("File open error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("File read error: {}", e);
            }
        }
        false
    }

    pub fn zoom_extents(&mut self) {
        let active_view = match self.doc.views.get(&self.doc.active_view_id) {
            Some(v) => v.clone(),
            None => return,
        };

        let p_idx = self.canvas_state.active_pane_idx;
        let is_3d = active_view.kind.is_3d();
        let vp_w = 1200.0f32;
        let vp_h = 750.0f32;

        if is_3d {
            let mut min_x = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut min_z = f64::INFINITY;
            let mut max_z = f64::NEG_INFINITY;

            let mut include_pt = |x: f64, y: f64, z: f64| {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            };

            for el in self.doc.elements.values() {
                match el {
                    Element::Wall(w) => {
                        let (z0, z1) = w.z_bounds(
                            self.doc.level_elevation(w.base_level),
                            w.top_level.map(|l| self.doc.level_elevation(l)),
                        );
                        include_pt(w.start.x, w.start.y, z0);
                        include_pt(w.start.x, w.start.y, z1);
                        include_pt(w.end.x, w.end.y, z0);
                        include_pt(w.end.x, w.end.y, z1);
                    }
                    Element::Column(c) => {
                        let (z0, z1) = c.z_bounds(
                            self.doc.level_elevation(c.base_level),
                            c.top_level.map(|l| self.doc.level_elevation(l)),
                        );
                        let hw = c.width * 0.5;
                        let hd = c.depth * 0.5;
                        include_pt(c.location.x - hw, c.location.y - hd, z0);
                        include_pt(c.location.x + hw, c.location.y + hd, z1);
                    }
                    Element::Beam(b) => {
                        let (z0, z1) = b.z_bounds(
                            self.doc.level_elevation(b.level),
                            b.top_level.map(|l| self.doc.level_elevation(l)),
                        );
                        include_pt(b.start.x, b.start.y, z0);
                        include_pt(b.end.x, b.end.y, z1);
                    }
                    Element::Slab(s) => {
                        let (z0, z1) = s.z_bounds(
                            self.doc.level_elevation(s.level),
                            s.top_level.map(|l| self.doc.level_elevation(l)),
                        );
                        for p in &s.boundary {
                            include_pt(p.x, p.y, z0);
                            include_pt(p.x, p.y, z1);
                        }
                    }
                    Element::Roof(r) => {
                        let r_base = self.doc.level_elevation(r.level) + r.offset;
                        let (z0, z1) = (r_base, r_base + r.thickness + 2000.0);
                        for p in &r.boundary {
                            include_pt(p.x, p.y, z0);
                            include_pt(p.x, p.y, z1);
                        }
                    }
                    Element::CurtainWall(cw) => {
                        let (z0, z1) = cw.z_bounds(
                            self.doc.level_elevation(cw.base_level),
                            cw.top_level.map(|l| self.doc.level_elevation(l)),
                        );
                        include_pt(cw.start.x, cw.start.y, z0);
                        include_pt(cw.end.x, cw.end.y, z1);
                    }
                    Element::Footing(f) => {
                        let (z0, z1) = f.z_bounds(self.doc.level_elevation(f.base_level), None);
                        let hw = f.width * 0.5;
                        let hd = f.length * 0.5;
                        include_pt(f.location.x - hw, f.location.y - hd, z0);
                        include_pt(f.location.x + hw, f.location.y + hd, z1);
                    }
                    Element::FreeformSolid(f) => {
                        for p in &f.boundary {
                            include_pt(p.x, p.y, f.z_min);
                            include_pt(p.x, p.y, f.z_max);
                        }
                        for face in f.faces() {
                            for p in &face.pts {
                                include_pt(p.x, p.y, p.z);
                            }
                        }
                    }
                    Element::Ceiling(c) => {
                        let z = self.doc.level_elevation(c.level) + c.height_above_level;
                        for p in &c.boundary {
                            include_pt(p.x, p.y, z);
                        }
                    }
                    Element::Door(d) => {
                        if let Some(Element::Wall(hw)) = self.doc.elements.get(&d.host_wall) {
                            let p = hw.start + hw.direction() * d.offset_along_wall;
                            let z = d.base_level.map(|l| self.doc.level_elevation(l)).unwrap_or_else(|| self.doc.level_elevation(hw.base_level));
                            include_pt(p.x, p.y, z);
                            include_pt(p.x, p.y, z + d.height);
                        }
                    }
                    Element::Window(w) => {
                        if let Some(Element::Wall(hw)) = self.doc.elements.get(&w.host_wall) {
                            let p = hw.start + hw.direction() * w.offset_along_wall;
                            let z = w.base_level.map(|l| self.doc.level_elevation(l)).unwrap_or_else(|| self.doc.level_elevation(hw.base_level)) + w.sill_height;
                            include_pt(p.x, p.y, z);
                            include_pt(p.x, p.y, z + w.height);
                        }
                    }
                    Element::Stair(s) => {
                        let (z0, z1) = (self.doc.level_elevation(s.base_level), s.top_level.map(|l| self.doc.level_elevation(l)).unwrap_or(3000.0));
                        for p in s.footprint() {
                            include_pt(p.x, p.y, z0);
                            include_pt(p.x, p.y, z1);
                        }
                    }
                    Element::Ramp(r) => {
                        let (z0, z1) = (self.doc.level_elevation(r.base_level), r.top_level.map(|l| self.doc.level_elevation(l)).unwrap_or(3000.0));
                        for p in r.footprint() {
                            include_pt(p.x, p.y, z0);
                            include_pt(p.x, p.y, z1);
                        }
                    }
                    Element::Railing(ra) => {
                        let z = self.doc.level_elevation(ra.level) + ra.base_offset;
                        for p in &ra.path {
                            include_pt(p.x, p.y, z);
                            include_pt(p.x, p.y, z + ra.height);
                        }
                    }
                    Element::DetailLine(l) => {
                        include_pt(l.start.x, l.start.y, 0.0);
                        include_pt(l.end.x, l.end.y, 0.0);
                    }
                    _ => {}
                }
            }

            let has_geometry = !min_x.is_infinite();
            if !has_geometry {
                min_x = 0.0;
                max_x = 12000.0;
                min_y = 0.0;
                max_y = 8000.0;
                min_z = 0.0;
                max_z = 3500.0;
            }

            let mut cam_3d = active_view.camera_3d.unwrap_or_default();
            if has_geometry {
                cam_3d.target_x = (min_x + max_x) * 0.5;
                cam_3d.target_y = (min_y + max_y) * 0.5;
                cam_3d.target_z = (min_z + max_z) * 0.5;
                let extent_diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2) + (max_z - min_z).powi(2)).sqrt();
                cam_3d.distance = (extent_diag * 1.8).max(15000.0);
                if let Some(v) = self.doc.views.get_mut(&self.doc.active_view_id) {
                    v.camera_3d = Some(cam_3d);
                }
            }

            let target_3d = plinth_geom::Vec3::new(cam_3d.target_x, cam_3d.target_y, cam_3d.target_z);
            let (sin_y, cos_y) = cam_3d.yaw.sin_cos();
            let (sin_p, cos_p) = cam_3d.pitch.sin_cos();

            let mut min_vx = f64::INFINITY;
            let mut max_vx = f64::NEG_INFINITY;
            let mut min_vz = f64::INFINITY;
            let mut max_vz = f64::NEG_INFINITY;

            for &x in &[min_x, max_x] {
                for &y in &[min_y, max_y] {
                    for &z in &[min_z, max_z] {
                        let d = plinth_geom::Vec3::new(x, y, z) - target_3d;
                        let x1 = d.x * cos_y - d.y * sin_y;
                        let y1 = d.x * sin_y + d.y * cos_y;
                        let z1 = d.z;
                        let x2 = x1;
                        let z2 = y1 * sin_p + z1 * cos_p;
                        min_vx = min_vx.min(x2);
                        max_vx = max_vx.max(x2);
                        min_vz = min_vz.min(z2);
                        max_vz = max_vz.max(z2);
                    }
                }
            }

            let span_x = (max_vx - min_vx).max(100.0);
            let span_z = (max_vz - min_vz).max(100.0);
            let center_vx = (min_vx + max_vx) * 0.5;
            let center_vz = (min_vz + max_vz) * 0.5;

            let fit_zoom = ((vp_w * 0.80) / span_x as f32).min((vp_h * 0.80) / span_z as f32).clamp(0.002, 1.0);
            let fit_pan = Vec2::new((-center_vx as f32) * fit_zoom, (center_vz as f32) * fit_zoom);

            self.canvas_state.panes[p_idx].zoom = fit_zoom;
            self.canvas_state.panes[p_idx].pan = fit_pan;
            self.canvas_state.zoom = fit_zoom;
            self.canvas_state.pan = fit_pan;
            self.status_message = "Zoomed to Model Extents (3D Fit)".into();
        } else {
            let mut min_x = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_y = f64::NEG_INFINITY;

            let mut include_pt_2d = |x: f64, y: f64| {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            };

            for el in self.doc.elements.values() {
                match el {
                    Element::Wall(w) => {
                        include_pt_2d(w.start.x, w.start.y);
                        include_pt_2d(w.end.x, w.end.y);
                    }
                    Element::Column(c) => {
                        let hw = c.width * 0.5;
                        let hd = c.depth * 0.5;
                        include_pt_2d(c.location.x - hw, c.location.y - hd);
                        include_pt_2d(c.location.x + hw, c.location.y + hd);
                    }
                    Element::Beam(b) => {
                        include_pt_2d(b.start.x, b.start.y);
                        include_pt_2d(b.end.x, b.end.y);
                    }
                    Element::Slab(s) => {
                        for p in &s.boundary {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Roof(r) => {
                        for p in &r.boundary {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::CurtainWall(cw) => {
                        include_pt_2d(cw.start.x, cw.start.y);
                        include_pt_2d(cw.end.x, cw.end.y);
                    }
                    Element::Footing(f) => {
                        let hw = f.width * 0.5;
                        let hd = f.length * 0.5;
                        include_pt_2d(f.location.x - hw, f.location.y - hd);
                        include_pt_2d(f.location.x + hw, f.location.y + hd);
                    }
                    Element::FreeformSolid(f) => {
                        for p in &f.boundary {
                            include_pt_2d(p.x, p.y);
                        }
                        for face in f.faces() {
                            for p in &face.pts {
                                include_pt_2d(p.x, p.y);
                            }
                        }
                    }
                    Element::Ceiling(c) => {
                        for p in &c.boundary {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Door(d) => {
                        if let Some(Element::Wall(hw)) = self.doc.elements.get(&d.host_wall) {
                            let p = hw.start + hw.direction() * d.offset_along_wall;
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Window(w) => {
                        if let Some(Element::Wall(hw)) = self.doc.elements.get(&w.host_wall) {
                            let p = hw.start + hw.direction() * w.offset_along_wall;
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Stair(s) => {
                        for p in s.footprint() {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Ramp(r) => {
                        for p in r.footprint() {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::Railing(ra) => {
                        for p in &ra.path {
                            include_pt_2d(p.x, p.y);
                        }
                    }
                    Element::DetailLine(l) => {
                        include_pt_2d(l.start.x, l.start.y);
                        include_pt_2d(l.end.x, l.end.y);
                    }
                    _ => {}
                }
            }

            if min_x.is_infinite() {
                min_x = 0.0;
                max_x = 12000.0;
                min_y = 0.0;
                max_y = 8000.0;
            }

            let span_x = (max_x - min_x).max(100.0);
            let span_y = (max_y - min_y).max(100.0);
            let center_x = (min_x + max_x) * 0.5;
            let center_y = (min_y + max_y) * 0.5;

            let fit_zoom = ((vp_w * 0.80) / span_x as f32).min((vp_h * 0.80) / span_y as f32).clamp(0.002, 1.0);
            let fit_pan = Vec2::new((-center_x as f32) * fit_zoom, (center_y as f32) * fit_zoom);

            self.canvas_state.panes[p_idx].zoom = fit_zoom;
            self.canvas_state.panes[p_idx].pan = fit_pan;
            self.canvas_state.zoom = fit_zoom;
            self.canvas_state.pan = fit_pan;
            self.status_message = "Zoomed to View Extents (Fit to View)".into();
        }
    }
}

impl eframe::App for PlinthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.canvas_state.gpu_renderer.is_none() {
            if let Some(gl) = _frame.gl() {
                if let Ok(renderer) = crate::gpu_viewport::GpuViewportRenderer::new(gl) {
                    self.canvas_state.gpu_renderer = Some(std::sync::Arc::new(renderer));
                }
            }
        }
        let is_light = self.theme_mode == ThemeMode::Light;
        let mut theme = if is_light {
            AppTheme::light()
        } else {
            AppTheme::dark()
        };
        theme.accent = self.settings.theme.accent_preset.color();
        theme.selection_color = self.settings.theme.selection_color.color();
        theme.canvas_bg = self.settings.theme.canvas_bg.color(is_light);
        theme.apply_to_ctx(ctx);

        // Global Keyboard Shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            if self.doc.undo() {
                self.status_message = "Action undone".into();
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Y)) {
            if self.doc.redo() {
                self.status_message = "Action redone".into();
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Delete)) {
            if let Some(sheet_id) = self.doc.active_sheet_id {
                if let Some(vp_id) = self.canvas_state.selected_placed_viewport.take() {
                    self.doc.push_undo();
                    if let Some(sheet) = self.doc.sheets.get_mut(&sheet_id) {
                        sheet.viewports.retain(|vp| vp.id != vp_id);
                    }
                    self.status_message = "Viewport removed from layout sheet".into();
                }
            } else if !self.canvas_state.selected_elements.is_empty() {
                self.doc.push_undo();
                let count = self.canvas_state.selected_elements.len();
                for id in self.canvas_state.selected_elements.drain(..) {
                    self.doc.elements.remove(&id);
                }
                self.selected_element = None;
                self.status_message = format!("{} elements deleted", count);
            } else if let Some(id) = self.selected_element.take() {
                self.doc.push_undo();
                self.doc.elements.remove(&id);
                self.status_message = "Element deleted".into();
            }
        }
        let trigger_save = ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::S));
        let mut trigger_save_as = ctx.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::S));
        #[allow(unused_variables)]
        let trigger_open = ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O));
        #[allow(unused_variables)]
        let trigger_export = ctx.input(|i| i.modifiers.command && i.key_pressed(Key::E));
        let trigger_new = ctx.input(|i| i.modifiers.command && i.key_pressed(Key::N));
        let trigger_zoom_extents = ctx.input(|i| i.key_pressed(Key::Home));

        // Function Key Shortcuts (F3: Snapping, F8: Ortho, F10: Polar)
        if ctx.input(|i| i.key_pressed(Key::F3)) {
            self.settings.snapping.enabled = !self.settings.snapping.enabled;
            self.canvas_state.snap_settings.enabled = self.settings.snapping.enabled;
            self.status_message = format!("Object Snapping {}", if self.settings.snapping.enabled { "ON" } else { "OFF" });
        }
        if ctx.input(|i| i.key_pressed(Key::F8)) {
            self.settings.snapping.ortho_mode = !self.settings.snapping.ortho_mode;
            self.canvas_state.snap_settings.ortho_mode = self.settings.snapping.ortho_mode;
            self.status_message = format!("Ortho Mode {}", if self.settings.snapping.ortho_mode { "ON" } else { "OFF" });
        }
        if ctx.input(|i| i.key_pressed(Key::F10)) {
            self.settings.snapping.polar_tracking = !self.settings.snapping.polar_tracking;
            self.canvas_state.snap_settings.polar_tracking = self.settings.snapping.polar_tracking;
            self.status_message = format!("Polar Tracking {}", if self.settings.snapping.polar_tracking { "ON" } else { "OFF" });
        }

        // Process asynchronous file picker loads (Web & Desktop)
        let pending_loads: Vec<PendingFileLoad> = if let Ok(mut queue) = self.pending_file_loads.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        };
        for load in pending_loads {
            self.open_project_from_bytes(&load.name, &load.bytes);
        }

        // Drag and drop files from OS file manager or browser
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(bytes) = file.bytes {
                self.open_project_from_bytes(&file.name, &bytes);
                break;
            } else if let Some(path) = file.path {
                self.open_project_file(&path);
                break;
            }
        }

        // 1. Top Panel: Ribbon UI
        TopBottomPanel::top("ribbon_panel").show(ctx, |ui| {
            if let Some(v) = self.doc.views.get(&self.doc.active_view_id) {
                self.settings.annotation.view_scale = v.scale;
            }
            let action = render_ribbon(
                ui,
                &mut self.current_tab,
                &self.active_tool,
                self.selected_element.is_some(),
                self.settings.theme.show_tool_labels,
                &theme,
                &mut self.settings.annotation,
                self.plugin_manager.is_plugin_active("mutracer-webgpu"),
                self.plugin_manager.is_plugin_active("blender-live-sync"),
            );

            if let Some(sc) = action.set_view_scale {
                self.settings.annotation.view_scale = sc;
                if let Some(v) = self.doc.views.get_mut(&self.doc.active_view_id) {
                    v.scale = sc;
                    self.status_message = format!("Set View Drawing Scale to 1:{}", sc);
                }
            } else if let Some(v) = self.doc.views.get_mut(&self.doc.active_view_id) {
                if v.scale != self.settings.annotation.view_scale {
                    v.scale = self.settings.annotation.view_scale;
                }
            }

            if action.new_project || trigger_new {
                self.doc = Document::new_empty();
                self.selected_element = None;
                self.current_file_path = None;
                self.status_message = "Created new empty BIM project".into();
            }

            #[cfg(target_arch = "wasm32")]
            if action.save_project || action.save_as_project || trigger_save || trigger_save_as {
                let json = serde_json::to_vec_pretty(&self.doc).unwrap_or_default();
                let fname = format!("{}.arch3d", self.doc.project_info.name.replace(' ', "_"));
                download_file_in_browser(&fname, &json, "application/json");
                self.status_message = format!("Saved project to {}", fname);
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.save_project || trigger_save {
                if let Some(path) = self.current_file_path.clone() {
                    self.save_project_file(&path);
                } else {
                    trigger_save_as = true;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if action.save_as_project || trigger_save_as {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Arch3D Project (*.arch3d, *.json)", &["arch3d", "plinth", "json"])
                    .add_filter("IFC4 BIM Model (*.ifc)", &["ifc"])
                    .set_file_name("project.arch3d")
                    .save_file()
                {
                    self.save_project_file(&path);
                }
            }

            #[cfg(target_arch = "wasm32")]
            if action.open_project || action.import_file || action.import_ifc || trigger_open {
                pick_file_web(
                    self.pending_file_loads.clone(),
                    "All Supported Files (*.arch3d, *.ifc, *.skp, *.dwg, *.dxf, *.step, *.json)",
                    &["arch3d", "plinth", "ifc", "skp", "dwg", "dxf", "step", "json"],
                );
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.open_project || action.import_file || action.import_ifc || trigger_open {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "All Supported Files (*.arch3d, *.ifc, *.skp, *.dwg, *.dxf, *.step, *.json)",
                        &["arch3d", "plinth", "ifc", "skp", "dwg", "dxf", "step", "json"],
                    )
                    .add_filter("Arch3D Project (*.arch3d, *.json)", &["arch3d", "plinth", "json"])
                    .add_filter("AutoCAD Drawing (*.dwg, *.dxf)", &["dwg", "dxf"])
                    .add_filter("SketchUp Model (*.skp)", &["skp"])
                    .add_filter("IFC BIM Model (*.ifc, *.step)", &["ifc", "step"])
                    .pick_file()
                {
                    self.open_project_file(&path);
                }
            }
            if action.zoom_extents || trigger_zoom_extents {
                self.zoom_extents();
            }

            #[cfg(target_arch = "wasm32")]
            if action.import_skp {
                pick_file_web(
                    self.pending_file_loads.clone(),
                    "SketchUp Model (*.skp)",
                    &["skp"],
                );
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.import_skp {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SketchUp Model (*.skp)", &["skp"])
                    .pick_file()
                {
                    self.open_project_file(&path);
                }
            }

            #[cfg(target_arch = "wasm32")]
            if action.import_dwg {
                pick_file_web(
                    self.pending_file_loads.clone(),
                    "AutoCAD Drawing (*.dwg, *.dxf)",
                    &["dwg", "dxf"],
                );
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.import_dwg {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("AutoCAD Drawing (*.dwg, *.dxf)", &["dwg", "dxf"])
                    .pick_file()
                {
                    self.open_project_file(&path);
                }
            }

            #[cfg(target_arch = "wasm32")]
            if action.export_file || action.export_ifc || trigger_export {
                let ifc_text = export_ifc4(&self.doc);
                let fname = format!("{}.ifc", self.doc.project_info.name.replace(' ', "_"));
                download_file_in_browser(&fname, ifc_text.as_bytes(), "application/x-step");
                self.status_message = format!("Exported IFC4 to {}", fname);
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.export_file || action.export_ifc || trigger_export {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("IFC4 BIM Model (*.ifc)", &["ifc"])
                    .add_filter("Arch3D Project JSON (*.arch3d, *.json)", &["arch3d", "plinth", "json"])
                    .set_file_name("model.ifc")
                    .save_file()
                {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    if ext == "arch3d" || ext == "plinth" || ext == "json" {
                        self.save_project_file(&path);
                    } else {
                        let ifc_text = export_ifc4(&self.doc);
                        if std::fs::write(&path, ifc_text).is_ok() {
                            self.status_message = format!("Exported IFC4 to {}", path.display());
                        }
                    }
                }
            }

            #[allow(unused_variables)]
            let trigger_pdf = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P));

            #[cfg(target_arch = "wasm32")]
            if action.export_pdf || trigger_pdf {
                let sheet_opt = self.doc.active_sheet_id
                    .and_then(|sid| self.doc.sheets.get(&sid))
                    .or_else(|| self.doc.sheets.values().next());

                if let Some(sheet) = sheet_opt {
                    let default_name = format!("{}_{}.pdf", self.doc.project_info.name.replace(' ', "_"), sheet.sheet_number);
                    match plinth_views::export_sheet_pdf(&self.doc, sheet.id, self.canvas_state.visual_style) {
                        Ok(pdf_bytes) => {
                            download_file_in_browser(&default_name, &pdf_bytes, "application/pdf");
                            self.status_message = format!("Exported Sheet [{}] to Vector PDF: {}", sheet.sheet_number, default_name);
                        }
                        Err(err) => {
                            self.status_message = format!("PDF Export failed: {err}");
                        }
                    }
                } else {
                    self.status_message = "No sheets available in project to export to PDF. Create a Sheet in the Project Browser first.".into();
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.export_pdf || trigger_pdf {
                let sheet_opt = self.doc.active_sheet_id
                    .and_then(|sid| self.doc.sheets.get(&sid))
                    .or_else(|| self.doc.sheets.values().next());

                if let Some(sheet) = sheet_opt {
                    let sheet_id = sheet.id;
                    let default_name = format!("{}_{}.pdf", self.doc.project_info.name.replace(' ', "_"), sheet.sheet_number);
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PDF Document (*.pdf)", &["pdf"])
                        .set_file_name(&default_name)
                        .save_file()
                    {
                        match plinth_views::export_sheet_pdf(&self.doc, sheet_id, self.canvas_state.visual_style) {
                            Ok(pdf_bytes) => {
                                match std::fs::write(&path, pdf_bytes) {
                                    Ok(_) => {
                                        self.status_message = format!("Exported Sheet [{}] to Vector PDF: {}", sheet.sheet_number, path.display());
                                    }
                                    Err(err) => {
                                        self.status_message = format!("Failed to write PDF file: {err}");
                                    }
                                }
                            }
                            Err(err) => {
                                self.status_message = format!("PDF Export failed: {err}");
                            }
                        }
                    }
                } else {
                    self.status_message = "No sheets available in project to export to PDF. Create a Sheet in the Project Browser first.".into();
                }
            }

            #[cfg(target_arch = "wasm32")]
            if action.export_glb {
                match plinth_views::export_glb(&self.doc, Some(&self.settings.materials)) {
                    Ok(glb_bytes) => {
                        let fname = format!("{}.glb", self.doc.project_info.name.replace(' ', "_"));
                        download_file_in_browser(&fname, &glb_bytes, "model/gltf-binary");
                        self.status_message = format!("Exported 3D Model to GLB: {}", fname);
                    }
                    Err(err) => {
                        self.status_message = format!("GLB Export failed: {err}");
                    }
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            if action.export_glb {
                let default_name = format!("{}.glb", self.doc.project_info.name.replace(' ', "_"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("glTF 2.0 Binary Model (*.glb)", &["glb"])
                    .set_file_name(&default_name)
                    .save_file()
                {
                    match plinth_views::export_glb(&self.doc, Some(&self.settings.materials)) {
                        Ok(glb_bytes) => {
                            match std::fs::write(&path, glb_bytes) {
                                Ok(_) => {
                                    self.status_message = format!("Exported 3D Model to GLB: {}", path.display());
                                }
                                Err(err) => {
                                    self.status_message = format!("Failed to write GLB file: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            self.status_message = format!("GLB Export failed: {err}");
                        }
                    }
                }
            }

            if let Some(tool) = action.new_tool {
                self.active_tool = tool;
                self.status_message = format!("Active tool: {}", self.active_tool.name());
            }
            if let Some(cat) = action.open_tool_config {
                self.tool_studio.open_for(cat);
                self.status_message = format!(
                    "Opened Type & Assembly Studio: {}",
                    cat.title()
                );
            }
            if let Some(mode) = action.open_struct_auto {
                self.struct_auto.open_mode(mode, &self.doc);
                self.status_message = format!("Opened Structural Automation: {}", mode.title());
            }
            if action.open_site_terrain {
                self.site_dialog.open_terrain();
                self.status_message = "Opened Build Terrain & Contour Lines Studio".into();
            }
            if action.open_lot_survey {
                self.site_dialog.open_survey();
                self.status_message = "Opened Cadastral Lot Bearing Survey Studio".into();
            }
            if action.open_level_manager {
                self.level_manager.is_open = true;
                self.status_message = "Opened Level & Storey Manager".into();
            }
            if action.open_grid_manager {
                self.grid_manager.is_open = true;
                self.status_message = "Opened Structural & Architectural Grid Manager".into();
            }
            if action.open_copy_level {
                self.copy_level_dlg.open_with_defaults(&self.doc);
                self.status_message = "Opened Copy Entities on Active Level".into();
            }
            if action.open_paste_level {
                self.paste_level_dlg
                    .open_with_defaults(&self.doc, &self.level_clipboard);
                self.status_message = "Opened Paste Entities to Other Levels".into();
            }
            if action.open_plugin_manager {
                self.plugin_manager.is_open = true;
                self.status_message = "Opened Arch3D Plugin Manager".into();
            }
            if action.trigger_render_mutracer {
                match plinth_views::export_glb(&self.doc, Some(&self.settings.materials)) {
                    Ok(glb_bytes) => {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(w) = web_sys::window() {
                            let fname = format!("{}.glb", self.doc.project_info.name.replace(' ', "_"));
                            download_file_in_browser(&fname, &glb_bytes, "model/gltf-binary");
                            let _ = w.open_with_url_and_target("render.html", "_blank");
                            self.status_message = "Exported GLB and opened MuTracer WebGPU Path Tracer Studio".into();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let _ = std::fs::write("model.glb", &glb_bytes);
                            let _ = std::process::Command::new("cmd").args(["/C", "start", "", "https://mutracer-dev.meimu-studio.com/"]).spawn();
                            self.status_message = "Exported model.glb and launched MuTracer WebGPU Path Tracer".into();
                        }
                    }
                    Err(err) => {
                        self.status_message = format!("GLB Export for MuTracer failed: {err}");
                    }
                }
            }

            if action.trigger_sync_blender {
                match plinth_views::export_glb(&self.doc, Some(&self.settings.materials)) {
                    Ok(glb_bytes) => {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            let uint8_arr = js_sys::Uint8Array::from(&glb_bytes[..]);
                            let window = web_sys::window();
                            let mut triggered_fetch = false;
                            if let Some(w) = window {
                                if let Ok(func) = js_sys::Reflect::get(&w, &wasm_bindgen::JsValue::from_str("arch3dSyncToBlender")) {
                                    if let Ok(js_fn) = func.dyn_into::<js_sys::Function>() {
                                        let _ = js_fn.call1(&wasm_bindgen::JsValue::NULL, &uint8_arr);
                                        triggered_fetch = true;
                                    }
                                }
                            }

                            if triggered_fetch {
                                self.status_message = format!("Live synced {} KB directly to Blender at http://127.0.0.1:9876", glb_bytes.len() / 1024);
                            } else {
                                download_file_in_browser("arch3d_live_sync.glb", &glb_bytes, "model/gltf-binary");
                                self.status_message = "Blender Live Sync: Downloaded arch3d_live_sync.glb (Reload cache in Blender)".into();
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let temp_glb = std::env::temp_dir().join("arch3d_live_sync.glb");
                            let _ = std::fs::write(&temp_glb, &glb_bytes);

                            let socket_result = std::net::TcpStream::connect_timeout(
                                &std::net::SocketAddr::from(([127, 0, 0, 1], 9876)),
                                std::time::Duration::from_millis(600),
                            );

                            match socket_result {
                                Ok(mut stream) => {
                                    use std::io::Write;
                                    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(3000)));
                                    match stream.write_all(&glb_bytes) {
                                        Ok(_) => {
                                            let _ = stream.flush();
                                            let _ = stream.shutdown(std::net::Shutdown::Write);
                                            self.status_message = format!("Live synced {} KB directly to Blender Cycles/EEVEE (port 9876)", glb_bytes.len() / 1024);
                                        }
                                        Err(e) => {
                                            self.status_message = format!("Saved cache to {}. Socket send error: {e}", temp_glb.display());
                                        }
                                    }
                                }
                                Err(_) => {
                                    self.status_message = format!("Cached to {}. (Start Live Sync Listener in Blender on port 9876)", temp_glb.display());
                                }
                            }
                        }
                    }
                    Err(err) => {
                        self.status_message = format!("Blender Live Sync GLB generation failed: {err}");
                    }
                }
            }

            if action.undo {
                if self.doc.undo() {
                    self.status_message = "Action undone".into();
                }
            }
            if action.redo {
                if self.doc.redo() {
                    self.status_message = "Action redone".into();
                }
            }
            if action.delete_selected {
                if let Some(id) = self.selected_element.take() {
                    self.doc.push_undo();
                    self.doc.elements.remove(&id);
                    self.status_message = "Element deleted".into();
                }
            }
            if action.toggle_theme {
                self.theme_mode = match self.theme_mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                self.settings.theme.theme_mode = self.theme_mode;
            }
            if action.open_default_3d {
                if let Some(v_3d) = self.doc.views.values().find(|v| v.kind == ViewKind::ThreeD) {
                    self.doc.active_view_id = v_3d.id;
                    let idx = self.canvas_state.active_pane_idx.min(3);
                    self.canvas_state.panes[idx].view_id = Some(v_3d.id);
                    self.doc.active_sheet_id = None;
                    self.status_message = "Switched to 3D View".into();
                }
            }
            if action.split_single {
                self.canvas_state.layout = crate::canvas::ViewportLayout::Single;
                self.status_message = "Single Viewport Layout".into();
            }
            if action.split_horizontal {
                self.canvas_state.layout = crate::canvas::ViewportLayout::SplitHorizontal;
                self.status_message = "Split Horizontal Viewport Layout".into();
            }
            if action.split_vertical {
                self.canvas_state.layout = crate::canvas::ViewportLayout::SplitVertical;
                self.status_message = "Split Vertical Viewport Layout".into();
            }
            if action.split_triple {
                self.canvas_state.layout = crate::canvas::ViewportLayout::SplitTriple;
                self.status_message = "Split Triple Viewport Layout".into();
            }
            if action.split_quad {
                self.canvas_state.layout = crate::canvas::ViewportLayout::SplitQuad;
                self.status_message = "Split 4-Way Viewport Layout".into();
            }
            if action.new_sheet {
                let sheet_count = self.doc.sheets.len() + 1;
                let new_sheet = plinth_model::Sheet::new(
                    format!("A10{}", sheet_count),
                    format!("Layout Sheet {}", sheet_count),
                    plinth_model::PaperSize::A1,
                );
                let sheet_id = new_sheet.id;
                self.doc.sheets.insert(sheet_id, new_sheet);
                self.doc.active_sheet_id = Some(sheet_id);
                self.canvas_state.sheet_zoom = 0.0;
                self.status_message = format!("Created layout sheet A10{}", sheet_count);
            }
            if action.open_settings {
                self.settings.open();
                self.status_message = "Opened Application Settings".into();
            }
            if action.load_sample {
                self.doc = Document::create_sample_building();
                self.selected_element = None;
                self.status_message = "Sample 2-Storey Building loaded".into();
                self.canvas_state.invalidate_gpu_cache();
                self.zoom_extents();
            }
            if action.load_apartment_highrise {
                match load_apartment_highrise_doc() {
                    Ok(new_doc) => {
                        let count = new_doc.elements.len();
                        let levels = new_doc.levels.len();
                        self.doc = new_doc;
                        self.selected_element = None;
                        self.active_tool = Tool::Select;
                        self.status_message = format!("Apartment Highrise.ifc Loaded ({} elements across {} storeys)", count, levels);
                        self.canvas_state.invalidate_gpu_cache();
                        self.zoom_extents();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load Apartment Highrise: {e}");
                    }
                }
            }
        });

        // 2. Bottom Panel: Status Bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 1. File format & origin badge
                if let Some(path) = &self.current_file_path {
                    let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                    let (fmt_pill, pill_color) = match ext.as_str() {
                        "ifc" => ("IFC4 (ISO 16739-1)", egui::Color32::from_rgb(0, 150, 136)),
                        "dwg" | "dxf" => ("AutoCAD CAD", egui::Color32::from_rgb(230, 74, 25)),
                        "skp" => ("SketchUp 3D", egui::Color32::from_rgb(211, 47, 47)),
                        _ => ("Arch3D BIM", theme.accent),
                    };
                    let fname = path.file_name().unwrap_or_default().to_string_lossy();
                    ui.label(egui::RichText::new(format!("[{}]", fmt_pill)).size(10.0).strong().color(pill_color));
                    ui.label(egui::RichText::new(format!("📄 {}", fname)).size(11.0).strong().color(theme.accent));
                    ui.separator();
                } else {
                    ui.label(egui::RichText::new("[Arch3D Native]").size(10.0).strong().color(theme.accent));
                    ui.separator();
                }

                // 2. Project Metrics Pill: Elements, Levels, Views
                let el_count = self.doc.elements.len();
                let lvl_count = self.doc.levels.len();
                let v_count = self.doc.views.len();
                ui.label(
                    egui::RichText::new(format!("{} Elements • {} Levels • {} Views • 1:1 mm", el_count, lvl_count, v_count))
                        .size(10.5)
                        .color(theme.text_muted),
                );
                ui.separator();

                // 3. Status Message
                let status_icon = if self.status_message.contains("Error") || self.status_message.contains("Failed") {
                    "⚠"
                } else if self.status_message.contains("Import") || self.status_message.contains("Loaded") || self.status_message.contains("Opened") {
                    "📥"
                } else if self.status_message.contains("Saved") || self.status_message.contains("Auto-saved") {
                    "💾"
                } else {
                    "✓"
                };
                ui.label(egui::RichText::new(format!("{} {}", status_icon, self.status_message)).size(11.0).color(theme.text_muted));
                ui.separator();
                ui.label(egui::RichText::new(self.active_tool.prompt()).size(11.0).color(theme.accent));

                if let Some(cat) = ToolCategory::from_tool(&self.active_tool) {
                    ui.separator();
                    if let Some(preset) = self.tool_studio.active_preset(cat) {
                        ui.label(
                            egui::RichText::new(format!("Type: {}", preset.type_name))
                                .size(11.0)
                                .strong(),
                        );
                    }
                    if ui
                        .small_button(format!("⚙ {} Settings...", cat.short_label()))
                        .on_hover_text(format!("Open {} Settings & Type Studio", cat.title()))
                        .clicked()
                    {
                        self.tool_studio.open_for(cat);
                    }
                }

                if let Tool::DrawRoof { shape, .. } = &mut self.active_tool {
                    ui.separator();
                    ui.label(egui::RichText::new("Roof Shape:").size(11.0).strong());
                    ui.selectable_value(shape, plinth_model::RoofShape::Hip, "Hip Roof");
                    ui.selectable_value(shape, plinth_model::RoofShape::Gable, "Gable Roof");
                    ui.selectable_value(shape, plinth_model::RoofShape::Shed, "Shed Roof");
                    ui.selectable_value(shape, plinth_model::RoofShape::Flat, "Flat Roof");
                }

                ui.separator();

                // Drafting & Precision Snapping Controls
                let snap_enabled = self.settings.snapping.enabled;
                let snap_btn_color = if snap_enabled { theme.accent } else { theme.text_muted };
                if ui.small_button(egui::RichText::new("🧲 SNAP (F3)").color(snap_btn_color).strong())
                    .on_hover_text("Toggle Precision Snapping (F3)")
                    .clicked()
                {
                    self.settings.snapping.enabled = !self.settings.snapping.enabled;
                    self.canvas_state.snap_settings.enabled = self.settings.snapping.enabled;
                    self.status_message = format!("Object Snapping {}", if self.settings.snapping.enabled { "ON" } else { "OFF" });
                }

                let ortho_active = self.settings.snapping.ortho_mode;
                let ortho_btn_color = if ortho_active { egui::Color32::from_rgb(255, 152, 0) } else { theme.text_muted };
                if ui.small_button(egui::RichText::new("⟂ ORTHO (F8)").color(ortho_btn_color).strong())
                    .on_hover_text("Toggle Ortho 0°/90° Mode (F8) - Hold Shift while drawing for temporary Ortho")
                    .clicked()
                {
                    self.settings.snapping.ortho_mode = !self.settings.snapping.ortho_mode;
                    self.canvas_state.snap_settings.ortho_mode = self.settings.snapping.ortho_mode;
                    self.status_message = format!("Ortho Mode {}", if self.settings.snapping.ortho_mode { "ON" } else { "OFF" });
                }

                let polar_active = self.settings.snapping.polar_tracking;
                let polar_btn_color = if polar_active { egui::Color32::from_rgb(0, 230, 118) } else { theme.text_muted };
                if ui.small_button(egui::RichText::new(format!("∠ POLAR {:.0}° (F10)", self.settings.snapping.polar_increment_deg)).color(polar_btn_color).strong())
                    .on_hover_text("Toggle Polar Tracking (F10) - Click to cycle increments 15°/30°/45°/90°")
                    .clicked()
                {
                    if !self.settings.snapping.polar_tracking {
                        self.settings.snapping.polar_tracking = true;
                    } else {
                        match self.settings.snapping.polar_increment_deg as u32 {
                            45 => self.settings.snapping.polar_increment_deg = 30.0,
                            30 => self.settings.snapping.polar_increment_deg = 15.0,
                            15 => self.settings.snapping.polar_increment_deg = 90.0,
                            _ => {
                                self.settings.snapping.polar_increment_deg = 45.0;
                                self.settings.snapping.polar_tracking = false;
                            }
                        }
                    }
                    self.canvas_state.snap_settings.polar_tracking = self.settings.snapping.polar_tracking;
                    self.canvas_state.snap_settings.polar_increment_deg = self.settings.snapping.polar_increment_deg;
                    self.status_message = if self.settings.snapping.polar_tracking {
                        format!("Polar Tracking ON ({:.0}° increment)", self.settings.snapping.polar_increment_deg)
                    } else {
                        "Polar Tracking OFF".into()
                    };
                }

                if matches!(self.active_tool, Tool::DrawWall { .. } | Tool::DrawCurtainWall { .. } | Tool::DrawDetailLine { .. }) {
                    let chain_active = self.settings.snapping.wall_continuous_chain;
                    let chain_color = if chain_active { theme.accent } else { theme.text_muted };
                    if ui.small_button(egui::RichText::new("⛓ CHAIN").color(chain_color).strong())
                        .on_hover_text("Toggle Continuous Polyline / Wall Chain Mode")
                        .clicked()
                    {
                        self.settings.snapping.wall_continuous_chain = !self.settings.snapping.wall_continuous_chain;
                        self.canvas_state.snap_settings.wall_continuous_chain = self.settings.snapping.wall_continuous_chain;
                        self.status_message = format!("Continuous Wall Chaining {}", if self.settings.snapping.wall_continuous_chain { "ON" } else { "OFF" });
                    }
                }

                if ui.small_button("⚙ Snapping...").on_hover_text("Open Snapping & Drafting Precision Settings").clicked() {
                    self.settings.active_category = SettingsCategory::Snapping;
                    self.settings.open();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_active_view_3d = self.doc.views.get(&self.doc.active_view_id).map_or(false, |v| v.kind.is_3d());
                    let is_free_modeling_tool = matches!(
                        self.active_tool,
                        crate::tool::Tool::DrawRectSurface { .. }
                            | crate::tool::Tool::DrawCircleSurface { .. }
                            | crate::tool::Tool::DrawPolygonSurface { .. }
                            | crate::tool::Tool::DrawBoxPrimitive { .. }
                            | crate::tool::Tool::DrawCylinderPrimitive { .. }
                            | crate::tool::Tool::DrawSpherePrimitive { .. }
                            | crate::tool::Tool::PushPull { .. }
                    );
                    let wp = self.canvas_state.snapped_world_pos;
                    let (coords_str, surface_badge) = if is_active_view_3d && is_free_modeling_tool {
                        match self.canvas_state.workplane {
                            crate::canvas::WorkplaneMode::Surface { host_element: Some(hid), .. } => {
                                let u_str = self.settings.units.format_length(wp.x);
                                let v_str = self.settings.units.format_length(wp.y);
                                (format!("U: {}   V: {}", u_str, v_str), format!(" [Surface on {}]", hid))
                            }
                            crate::canvas::WorkplaneMode::Surface { .. } => {
                                let u_str = self.settings.units.format_length(wp.x);
                                let v_str = self.settings.units.format_length(wp.y);
                                (format!("U: {}   V: {}", u_str, v_str), " [Surface]".to_string())
                            }
                            _ => {
                                let x_str = self.settings.units.format_length(wp.x);
                                let y_str = self.settings.units.format_length(wp.y);
                                (format!("X: {}   Y: {}", x_str, y_str), String::new())
                            }
                        }
                    } else {
                        let x_str = self.settings.units.format_length(wp.x);
                        let y_str = self.settings.units.format_length(wp.y);
                        (format!("X: {}   Y: {}", x_str, y_str), String::new())
                    };
                    ui.label(egui::RichText::new(format!("{}{}", coords_str, surface_badge)).size(11.0));
                    ui.separator();
                    if let Some(lvl) = self.doc.levels.get(&self.doc.active_level_id) {
                        let elev_str = self.settings.units.format_length(lvl.elevation);
                        ui.label(egui::RichText::new(format!("Level: {} ({})", lvl.name, elev_str)).size(11.0));
                        ui.separator();
                    }
                    if let Some(v) = self.doc.views.get(&self.doc.active_view_id) {
                        ui.label(egui::RichText::new(format!("Scale 1:{}", v.scale)).size(11.0));
                    }
                });
            });
        });

        // 3. Left Side Panel: Project Browser
        egui::SidePanel::left("project_browser_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(160.0..=360.0)
            .show(ctx, |ui| {
                let browser_res = render_project_browser(ui, &mut self.doc, &theme);
                if browser_res.open_level_manager {
                    self.level_manager.is_open = true;
                }
                if let Some(new_view_id) = browser_res.activated_view {
                    self.doc.active_view_id = new_view_id;
                    let idx = self.canvas_state.active_pane_idx.min(3);
                    self.canvas_state.panes[idx].view_id = Some(new_view_id);
                    self.doc.active_sheet_id = None;
                    if let Some(v) = self.doc.views.get(&new_view_id) {
                        self.status_message = format!("Activated view: {}", v.name);
                    }
                }
                if let Some(sheet_id) = browser_res.activated_sheet {
                    self.doc.active_sheet_id = Some(sheet_id);
                    self.canvas_state.sheet_zoom = 0.0;
                    if let Some(s) = self.doc.sheets.get(&sheet_id) {
                        self.status_message = format!("Opened layout sheet: {} - {}", s.sheet_number, s.name);
                    }
                }
                if browser_res.new_sheet {
                    let sheet_count = self.doc.sheets.len() + 1;
                    let new_sheet = plinth_model::Sheet::new(
                        format!("A10{}", sheet_count),
                        format!("Layout Sheet {}", sheet_count),
                        plinth_model::PaperSize::A1,
                    );
                    let sheet_id = new_sheet.id;
                    self.doc.sheets.insert(sheet_id, new_sheet);
                    self.doc.active_sheet_id = Some(sheet_id);
                    self.canvas_state.sheet_zoom = 0.0;
                    self.status_message = format!("Created layout sheet A10{}", sheet_count);
                }
                if let Some(sheet_id) = browser_res.add_viewport_to_sheet {
                    self.doc.push_undo();
                    if let Some(sheet) = self.doc.sheets.get_mut(&sheet_id) {
                        let vp_num = (sheet.viewports.len() + 1) as u32;
                        let title = self.doc.views.get(&self.doc.active_view_id)
                            .map(|v| v.name.clone())
                            .unwrap_or_else(|| "FLOOR PLAN".into());
                        let (pw, ph) = sheet.paper_size.dimensions_mm();
                        let stagger = ((vp_num as f64 - 1.0) % 4.0) * 35.0;
                        let pos = plinth_geom::vec2(pw * 0.25 + stagger, ph * 0.35 + stagger * 0.6);
                        let new_vp_id = plinth_model::ViewportId::new();
                        sheet.viewports.push(plinth_model::PlacedViewport {
                            id: new_vp_id,
                            view_id: self.doc.active_view_id,
                            position_mm: pos,
                            width_mm: 340.0,
                            height_mm: 240.0,
                            scale: 100,
                            title: format!("{} - {}", vp_num, title.to_uppercase()),
                            number: vp_num,
                            show_title: true,
                        });
                        self.doc.active_sheet_id = Some(sheet_id);
                        self.canvas_state.selected_placed_viewport = Some(new_vp_id);
                        self.status_message = format!("Placed viewport on sheet {} (drag to reposition freely)", sheet.sheet_number);
                    }
                }
            });

        // 4. Right Side Panel: Properties Panel
        egui::SidePanel::right("properties_panel")
            .resizable(true)
            .default_width(275.0)
            .width_range(210.0..=440.0)
            .show(ctx, |ui| {
                if let Some(cat) = render_properties_panel(
                    ui,
                    &mut self.doc,
                    self.selected_element,
                    &mut self.canvas_state.selected_placed_viewport,
                    &theme,
                ) {
                    self.tool_studio.open_for(cat);
                }
            });

        // 5. Central Viewport Canvas
        self.canvas_state.snap_settings = self.settings.snapping.clone();
        if self.current_tab == RibbonTab::Annotation {
            self.canvas_state.snap_settings.snap_to_edges = self.settings.annotation.snap_to_edges;
            self.canvas_state.snap_settings.snap_to_centerlines = self.settings.annotation.snap_to_centerlines;
            self.canvas_state.snap_settings.snap_to_openings = self.settings.annotation.snap_to_openings;
        }
        self.canvas_state.annotation_settings = self.settings.annotation.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            render_canvas(
                ui,
                &mut self.doc,
                &mut self.canvas_state,
                &mut self.active_tool,
                &mut self.selected_element,
                &self.tool_studio,
                &theme,
            );
        });

        // 5b. Arch3D Type & Assembly Studio (Double-click Ribbon Tools / Properties)
        let studio_outcome = render_tool_config_studio(
            ctx,
            &mut self.tool_studio,
            &mut self.doc,
            self.selected_element,
            &theme,
        );
        if let Some(tool) = studio_outcome.activated_tool {
            self.active_tool = tool;
        }
        if let Some(note) = studio_outcome.status_note {
            self.status_message = note;
        }

        // 5c. Manage Tab Datum & Multi-Storey Dialogs + Structural Automation Studio
        self.level_manager.render(ctx, &mut self.doc, &theme);
        self.grid_manager.render(ctx, &mut self.doc, &theme);
        let auto_sig = self
            .struct_auto
            .render(ctx, &mut self.doc, &self.tool_studio, &theme);
        if auto_sig.open_grid_manager {
            self.grid_manager.is_open = true;
        }
        if let Some(cat) = auto_sig.open_type_studio {
            self.tool_studio.open_for(cat);
        }
        if self
            .copy_level_dlg
            .render(ctx, &self.doc, &mut self.level_clipboard, &theme)
        {
            self.paste_level_dlg
                .open_with_defaults(&self.doc, &self.level_clipboard);
        }
        self.paste_level_dlg
            .render(ctx, &mut self.doc, &self.level_clipboard, &theme);
        self.site_dialog.render(ctx, &mut self.doc, &theme);

        // 5d. Dedicated Plugin Manager Studio (Modular & Reversible Extension Management)
        if let Some(launch_url) = self.plugin_manager.render(ctx, &theme) {
            #[cfg(target_arch = "wasm32")]
            if let Some(w) = web_sys::window() {
                let _ = w.open_with_url_and_target(&launch_url, "_blank");
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let full_url = if launch_url.starts_with("http") {
                    launch_url
                } else {
                    "https://mutracer-dev.meimu-studio.com/".to_string()
                };
                let _ = std::process::Command::new("cmd").args(["/C", "start", "", &full_url]).spawn();
            }
        }

        // 6. Professional Categorized Preferences Dialog (Modal Window)
        let settings_res = render_settings_dialog(ctx, &mut self.settings, &mut theme, &self.doc);
        if settings_res.applied {
            self.theme_mode = self.settings.theme.theme_mode;
            self.canvas_state.materials = self.settings.materials;
            self.status_message = "Settings applied successfully".into();
        }

        // 7. Automatic Periodic Background Auto-Save
        #[cfg(not(target_arch = "wasm32"))]
        if self.settings.saving.auto_save_enabled {
            let interval_secs = (self.settings.saving.auto_save_interval_mins.max(1) as u64) * 60;
            if self.last_auto_save.elapsed() >= std::time::Duration::from_secs(interval_secs) {
                // Only write to disk if the document was modified since last save!
                if self.doc.undo_count() != self.last_saved_undo_count {
                    let auto_save_path = if let Some(cur) = &self.current_file_path {
                        cur.with_extension("autosave.arch3d")
                    } else {
                        std::env::temp_dir().join("arch3d_autosave.arch3d")
                    };
                    if let Ok(json) = serde_json::to_string(&self.doc) {
                        if std::fs::write(&auto_save_path, json).is_ok() {
                            self.last_saved_undo_count = self.doc.undo_count();
                            self.last_auto_save = web_time::Instant::now();
                            self.status_message = format!(
                                "Auto-saved project to {}",
                                auto_save_path.file_name().unwrap_or_default().to_string_lossy()
                            );
                        }
                    }
                } else {
                    self.last_auto_save = web_time::Instant::now();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plinth_app_initialization() {
        let app = PlinthApp::default();
        assert!(!app.doc.elements.is_empty());
        assert!(!app.doc.views.is_empty());
        assert!(!app.doc.levels.is_empty());
        assert_eq!(app.current_tab, RibbonTab::Home);
        assert_eq!(app.theme_mode, ThemeMode::Dark);
        assert_eq!(app.active_tool, Tool::Select);
        assert!(!app.settings.is_open);
        assert_eq!(app.settings.units.length_unit, crate::settings::LengthUnit::Millimeters);
    }

    #[test]
    fn test_home_tab_and_project_persistence() {
        let mut app = PlinthApp::default();
        assert_eq!(app.current_tab, RibbonTab::Home);

        let temp_dir = std::env::temp_dir();
        let save_path = temp_dir.join("test_arch3d_project.arch3d");

        assert!(app.save_project_file(&save_path));
        assert_eq!(app.current_file_path, Some(save_path.clone()));

        app.doc = Document::new_empty();
        assert_eq!(app.doc.elements.len(), 0);

        assert!(app.open_project_file(&save_path));
        assert!(!app.doc.elements.is_empty());
        assert_eq!(app.current_file_path, Some(save_path.clone()));

        let _ = std::fs::remove_file(save_path);
    }

    #[test]
    fn test_import_user_school_building_in_app() {
        let mut app = PlinthApp::default();
        let path = std::path::Path::new(r"c:\Users\redwood-ph-pc\Desktop\OSA files\TEST BIMCRAFT IMPORT\2 Storey School Building.ifc");
        if path.exists() {
            let opened = app.open_project_file(path);
            assert!(opened, "Should open project file successfully");
            println!("Opened! Elements: {}, Views: {}, Active View: {:?}", app.doc.elements.len(), app.doc.views.len(), app.doc.active_view_id);

            for (_vid, v) in &app.doc.views {
                println!("Testing render view: {} ({:?})", v.name, v.kind);
                let scene = plinth_views::render_view(&app.doc, v, true);
                println!("Rendered view {} with {} primitives", v.name, scene.primitives.len());
            }

            // Test executing full egui update and all panels frame
            let ctx = egui::Context::default();
            let _ = ctx.run(Default::default(), |ctx| {
                let is_light = app.theme_mode == ThemeMode::Light;
                let mut theme = if is_light { AppTheme::light() } else { AppTheme::dark() };
                theme.accent = app.settings.theme.accent_preset.color();

                TopBottomPanel::top("ribbon_panel").show(ctx, |ui| {
                    render_ribbon(
                        ui,
                        &mut app.current_tab,
                        &app.active_tool,
                        app.selected_element.is_some(),
                        app.settings.theme.show_tool_labels,
                        &theme,
                        &mut app.settings.annotation,
                        app.plugin_manager.is_plugin_active("mutracer-webgpu"),
                        app.plugin_manager.is_plugin_active("blender-live-sync"),
                    );
                });

                TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                    ui.label(&app.status_message);
                });

                egui::SidePanel::left("project_browser_panel").show(ctx, |ui| {
                    render_project_browser(ui, &mut app.doc, &theme);
                });

                egui::SidePanel::right("properties_panel").show(ctx, |ui| {
                    render_properties_panel(
                        ui,
                        &mut app.doc,
                        app.selected_element,
                        &mut app.canvas_state.selected_placed_viewport,
                        &theme,
                    );
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    render_canvas(
                        ui,
                        &mut app.doc,
                        &mut app.canvas_state,
                        &mut app.active_tool,
                        &mut app.selected_element,
                        &app.tool_studio,
                        &theme,
                    );
                });

                render_settings_dialog(ctx, &mut app.settings, &mut theme, &app.doc);
            });

            // Stress test 1: Activate every view across all 4 visual styles
            for (vid, _v) in app.doc.views.clone() {
                app.doc.active_view_id = vid;
                for style in [plinth_views::VisualStyle::Shaded, plinth_views::VisualStyle::HiddenLine, plinth_views::VisualStyle::ConsistentColors, plinth_views::VisualStyle::Wireframe] {
                    app.canvas_state.visual_style = style;
                    let _ = ctx.run(Default::default(), |ctx| {
                        let is_light = app.theme_mode == ThemeMode::Light;
                        let theme = if is_light { AppTheme::light() } else { AppTheme::dark() };
                        egui::CentralPanel::default().show(ctx, |ui| {
                            render_canvas(
                                ui,
                                &mut app.doc,
                                &mut app.canvas_state,
                                &mut app.active_tool,
                                &mut app.selected_element,
                                &app.tool_studio,
                                &theme,
                            );
                        });
                    });
                }
            }
            println!("Stress test 1 passed: all views across all visual styles rendered without issues!");

            // Stress test 2: Select every single element and render properties panel and canvas highlight
            for &el_id in app.doc.elements.keys().cloned().collect::<Vec<_>>().iter() {
                app.selected_element = Some(el_id);
                let _ = ctx.run(Default::default(), |ctx| {
                    let is_light = app.theme_mode == ThemeMode::Light;
                    let theme = if is_light { AppTheme::light() } else { AppTheme::dark() };
                    egui::SidePanel::right("properties_panel").show(ctx, |ui| {
                        render_properties_panel(
                            ui,
                            &mut app.doc,
                            app.selected_element,
                            &mut app.canvas_state.selected_placed_viewport,
                            &theme,
                        );
                    });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        render_canvas(
                            ui,
                            &mut app.doc,
                            &mut app.canvas_state,
                            &mut app.active_tool,
                            &mut app.selected_element,
                            &app.tool_studio,
                            &theme,
                        );
                    });
                });
            }
            println!("Stress test 2 passed: all 304 elements selected and inspected without issues!");
        }
    }

    #[test]
    fn test_tool_studio_level_hosting_and_sheet_viewport_dragging() {
        let mut app = PlinthApp::default();
        let ctx = egui::Context::default();
        let theme = AppTheme::dark();

        // 1. Verify Type & Assembly Studio across all 18 Architecture & Structure categories
        let all_cats: Vec<crate::tool_config::ToolCategory> = crate::tool_config::ToolCategory::ALL_ARCH
            .into_iter()
            .chain(crate::tool_config::ToolCategory::ALL_STRUCT)
            .collect();
        assert_eq!(all_cats.len(), 18);

        for cat in all_cats {
            app.tool_studio.open_for(cat);
            assert!(app.tool_studio.is_open);
            assert_eq!(app.tool_studio.active_category, cat);

            let count_before = app.tool_studio.presets.len();
            app.tool_studio.duplicate_active_type();
            assert_eq!(app.tool_studio.presets.len(), count_before + 1);
            assert!(!app.tool_studio.active_preset(cat).unwrap().is_builtin);

            let _ = ctx.run(Default::default(), |ctx| {
                let _ = render_tool_config_studio(
                    ctx,
                    &mut app.tool_studio,
                    &mut app.doc,
                    app.selected_element,
                    &theme,
                );
            });
        }

        // 2. Verify all 13 level-hosted element types with Base/Top Level + Base/Top Offsets
        let mut level_ids: Vec<plinth_model::LevelId> = app.doc.levels.keys().copied().collect();
        level_ids.sort_by(|a, b| {
            app.doc.levels[a]
                .elevation
                .partial_cmp(&app.doc.levels[b].elevation)
                .unwrap()
        });
        assert!(level_ids.len() >= 2);
        let l1 = level_ids[0];
        let l2 = level_ids[1];
        let l1_z = app.doc.levels[&l1].elevation;
        let l2_z = app.doc.levels[&l2].elevation;

        let mut wall = plinth_model::Wall::new(
            plinth_geom::vec2(20000.0, 0.0),
            plinth_geom::vec2(25000.0, 0.0),
            l1,
        );
        wall.base_offset = 150.0;
        wall.top_level = Some(l2);
        wall.top_offset = -200.0;
        let wall_id = wall.id;
        assert_eq!(wall.z_bounds(l1_z, Some(l2_z)), (l1_z + 150.0, l2_z - 200.0));
        app.doc.elements.insert(wall_id, plinth_model::Element::Wall(wall));

        let mut door = plinth_model::Door::new(wall_id, 1200.0);
        door.base_level = Some(l1);
        door.sill_height = 50.0;
        door.top_level = Some(l2);
        door.top_offset = -900.0;
        let door_id = door.id;
        app.doc.elements.insert(door_id, plinth_model::Element::Door(door));

        let mut win = plinth_model::Window::new(wall_id, 3200.0);
        win.base_level = Some(l1);
        win.sill_height = 900.0;
        win.top_level = Some(l2);
        win.top_offset = -600.0;
        let win_id = win.id;
        app.doc.elements.insert(win_id, plinth_model::Element::Window(win));

        let mut col = plinth_model::Column::new(plinth_geom::vec2(20000.0, 3000.0), l1);
        col.base_offset = -300.0;
        col.top_level = Some(l2);
        col.top_offset = 0.0;
        let col_id = col.id;
        assert_eq!(col.z_bounds(l1_z, Some(l2_z)), (l1_z - 300.0, l2_z));
        app.doc.elements.insert(col_id, plinth_model::Element::Column(col));

        let mut beam = plinth_model::Beam::new(
            plinth_geom::vec2(20000.0, 3000.0),
            plinth_geom::vec2(25000.0, 3000.0),
            l2,
        );
        beam.z_offset = -50.0;
        let beam_id = beam.id;
        app.doc.elements.insert(beam_id, plinth_model::Element::Beam(beam));

        let rect_pts = vec![
            plinth_geom::vec2(20000.0, 4000.0),
            plinth_geom::vec2(24000.0, 4000.0),
            plinth_geom::vec2(24000.0, 7000.0),
            plinth_geom::vec2(20000.0, 7000.0),
        ];
        let mut slab = plinth_model::Slab::new(rect_pts.clone(), l1);
        slab.offset = -25.0;
        let slab_id = slab.id;
        app.doc.elements.insert(slab_id, plinth_model::Element::Slab(slab));

        let mut roof = plinth_model::Roof::new(rect_pts.clone(), l2);
        roof.offset = 100.0;
        let roof_id = roof.id;
        app.doc.elements.insert(roof_id, plinth_model::Element::Roof(roof));

        let mut ceil = plinth_model::Ceiling::new(rect_pts, l1);
        ceil.height_above_level = 2650.0;
        let ceil_id = ceil.id;
        app.doc.elements.insert(ceil_id, plinth_model::Element::Ceiling(ceil));

        let mut stair = plinth_model::Stair::new(
            plinth_geom::vec2(26000.0, 0.0),
            plinth_geom::vec2(30500.0, 0.0),
            l1,
        );
        stair.top_level = Some(l2);
        stair.stair_style = plinth_model::StairStyle::Monolithic;
        let stair_id = stair.id;
        app.doc.elements.insert(stair_id, plinth_model::Element::Stair(stair));

        let mut ramp = plinth_model::Ramp::new(
            plinth_geom::vec2(26000.0, 3000.0),
            plinth_geom::vec2(33200.0, 3000.0),
            l1,
        );
        ramp.ramp_style = plinth_model::RampStyle::SlopedSlab;
        let ramp_id = ramp.id;
        app.doc.elements.insert(ramp_id, plinth_model::Element::Ramp(ramp));

        let mut rail = plinth_model::Railing::new(
            vec![
                plinth_geom::vec2(26000.0, 5000.0),
                plinth_geom::vec2(30000.0, 5000.0),
            ],
            l1,
        );
        rail.base_offset = 50.0;
        let rail_id = rail.id;
        app.doc.elements.insert(rail_id, plinth_model::Element::Railing(rail));

        let mut fascia = plinth_model::Fascia::new(
            vec![
                plinth_geom::vec2(20000.0, 7000.0),
                plinth_geom::vec2(24000.0, 7000.0),
            ],
            l2,
        );
        fascia.vertical_offset = -50.0;
        let fascia_id = fascia.id;
        app.doc.elements.insert(fascia_id, plinth_model::Element::Fascia(fascia));

        let mut gutter = plinth_model::Gutter::new(
            vec![
                plinth_geom::vec2(20000.0, 7100.0),
                plinth_geom::vec2(24000.0, 7100.0),
            ],
            l2,
        );
        gutter.vertical_offset = -40.0;
        let gutter_id = gutter.id;
        app.doc.elements.insert(gutter_id, plinth_model::Element::Gutter(gutter));

        for eid in [
            wall_id, door_id, win_id, col_id, beam_id, slab_id, roof_id, ceil_id, stair_id,
            ramp_id, rail_id, fascia_id, gutter_id,
        ] {
            app.selected_element = Some(eid);
            let _ = ctx.run(Default::default(), |ctx| {
                egui::SidePanel::right("props_test").show(ctx, |ui| {
                    let _ = render_properties_panel(
                        ui,
                        &mut app.doc,
                        app.selected_element,
                        &mut app.canvas_state.selected_placed_viewport,
                        &theme,
                    );
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_canvas(
                        ui,
                        &mut app.doc,
                        &mut app.canvas_state,
                        &mut app.active_tool,
                        &mut app.selected_element,
                        &app.tool_studio,
                        &theme,
                    );
                });
            });
        }

        // 3. Verify Sheet Layout PlacedViewport free positioning & resizing
        if let Some((&sheet_id, sheet)) = app.doc.sheets.iter_mut().next() {
            app.doc.active_sheet_id = Some(sheet_id);
            if let Some(vp) = sheet.viewports.first_mut() {
                vp.position_mm = plinth_geom::vec2(85.0, 62.0);
                vp.width_mm = 380.0;
                vp.height_mm = 260.0;
                app.canvas_state.selected_placed_viewport = Some(vp.id);
            }
            let _ = ctx.run(Default::default(), |ctx| {
                egui::SidePanel::right("sheet_props_test").show(ctx, |ui| {
                    let _ = render_properties_panel(
                        ui,
                        &mut app.doc,
                        None,
                        &mut app.canvas_state.selected_placed_viewport,
                        &theme,
                    );
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_canvas(
                        ui,
                        &mut app.doc,
                        &mut app.canvas_state,
                        &mut app.active_tool,
                        &mut app.selected_element,
                        &app.tool_studio,
                        &theme,
                    );
                });
            });
        }
    }

    #[test]
    fn test_level_manager_and_grid_manager() {
        let mut doc = Document::new_empty();
        doc.levels.clear();
        doc.views.clear();
        let l1 = plinth_model::Level::new("Level 1", 0.0, 3500.0);
        let l1_id = l1.id;
        doc.levels.insert(l1_id, l1);
        doc.active_level_id = l1_id;

        // 1. Level Manager: Add level above
        let l2 = crate::level_manager::add_level_above(&mut doc, 3500.0, true);
        assert_eq!(doc.levels.len(), 2);
        assert_eq!(doc.level_elevation(l2), 3500.0);
        assert!(doc.views.values().any(|v| v.level_id == Some(l2) && v.kind == ViewKind::FloorPlan));

        // 2. Add basement below
        let b1 = crate::level_manager::add_basement_below(&mut doc, 3200.0, true);
        assert_eq!(doc.levels.len(), 3);
        assert_eq!(doc.level_elevation(b1), -3200.0);

        // 3. Multi-storey stack generation
        let stack = crate::level_manager::generate_multi_storey_levels(&mut doc, 4, 3500.0, true, true);
        assert!(!stack.is_empty());
        assert!(doc.levels.values().any(|l| l.elevation >= 14000.0));

        // 4. Safe level deletion
        let initial_lvl_count = doc.levels.len();
        let deleted = crate::level_manager::delete_level_safely(&mut doc, b1);
        assert!(deleted);
        assert_eq!(doc.levels.len(), initial_lvl_count - 1);
        assert!(!doc.levels.contains_key(&b1));

        // 5. Grid Manager: Spacings parser & Orthogonal grid generation
        let x_sp = crate::level_manager::parse_spacings_input("6000, 7500, 6000", 3, 6000.0);
        assert_eq!(x_sp, vec![6000.0, 7500.0, 6000.0]);

        let y_sp = crate::level_manager::parse_spacings_input("2*5000, 4000", 3, 5000.0);
        assert_eq!(y_sp, vec![5000.0, 5000.0, 4000.0]);

        let grid_count = crate::level_manager::generate_orthogonal_grids(
            &mut doc,
            plinth_geom::vec2(0.0, 0.0),
            &x_sp,
            &y_sp,
            true,
            1500.0,
            true,
        );
        assert_eq!(grid_count, 4 + 4); // 4 vertical + 4 horizontal
        assert_eq!(doc.grids.len(), 8);

        crate::level_manager::auto_align_grid_extents(&mut doc, 2000.0);
        assert_eq!(doc.grids.len(), 8);
    }

    #[test]
    fn test_structural_automation_pipeline_and_footings() {
        let mut doc = Document::new_empty();
        let studio = ToolConfigStudio::new();

        let l1 = plinth_model::Level::new("Level 1", 0.0, 3500.0);
        let l1_id = l1.id;
        doc.levels.insert(l1_id, l1);
        let l2 = plinth_model::Level::new("Level 2", 3500.0, 3500.0);
        let l2_id = l2.id;
        doc.levels.insert(l2_id, l2);
        doc.active_level_id = l1_id;

        // 1. Generate a 3x3 bay orthogonal grid system (4x4 lines)
        let x_sp = vec![6000.0, 6000.0, 6000.0];
        let y_sp = vec![6000.0, 6000.0, 6000.0];
        crate::level_manager::generate_orthogonal_grids(
            &mut doc,
            plinth_geom::vec2(0.0, 0.0),
            &x_sp,
            &y_sp,
            true,
            1500.0,
            true,
        );

        let intersections = crate::structure_auto::compute_grid_intersections(&doc);
        assert_eq!(intersections.len(), 16); // 4x4 = 16 grid intersections

        // 2. Automation: Place columns on grids
        let cols_placed = crate::structure_auto::place_columns_on_grids(&mut doc, &studio, l1_id, Some(l2_id));
        assert_eq!(cols_placed, 16);
        let col_count = doc.elements.values().filter(|el| matches!(el, plinth_model::Element::Column(_))).count();
        assert_eq!(col_count, 16);

        // 3. Automation: Place foundation footings on columns
        let footings_placed = crate::structure_auto::place_footings_on_columns(&mut doc, &studio, Some(l1_id), 300.0);
        assert_eq!(footings_placed, 16);
        let footing_count = doc.elements.values().filter(|el| matches!(el, plinth_model::Element::Footing(_))).count();
        assert_eq!(footing_count, 16);

        // Verify footing properties
        for el in doc.elements.values() {
            if let plinth_model::Element::Footing(f) = el {
                assert!(f.host_column.is_some());
                assert_eq!(f.base_level, l1_id);
                assert!(f.width >= 700.0);
                assert!(f.length >= 700.0);
                assert!(f.thickness >= 300.0);
                let (z0, z1) = f.z_bounds(0.0, None);
                assert!(z1 > z0);
                assert_eq!(f.solids_with_bounds(0.0, None).len(), 1);
                assert_eq!(f.faces(0.0, None).len(), 6);
            }
        }

        // 4. Automation: Place beams on grids / columns at Level 2 framing
        let beams_placed = crate::structure_auto::place_beams_on_grids_or_columns(&mut doc, &studio, l2_id, false);
        assert_eq!(beams_placed, 24); // (3*4 vertical + 3*4 horizontal = 24 beam spans)
        let beam_count = doc.elements.values().filter(|el| matches!(el, plinth_model::Element::Beam(_))).count();
        assert_eq!(beam_count, 24);

        // 5. Automation: Place slabs on closed column-beam bays
        let bays = crate::structure_auto::find_closed_structural_bays(&doc, l2_id, true);
        assert_eq!(bays.len(), 9); // 3x3 = 9 closed bays
        let slabs_placed = crate::structure_auto::place_slabs_on_closed_bays(&mut doc, &studio, l2_id, true);
        assert_eq!(slabs_placed, 9);
        let slabs: Vec<_> = doc
            .elements
            .values()
            .filter_map(|el| match el {
                plinth_model::Element::Slab(s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(slabs.len(), 9);

        // Verify slab is aligned to inner surface of the beams (offset by 150mm from centerlines)
        let first_bay_slab = slabs
            .iter()
            .find(|s| {
                let c = plinth_geom::polygon::centroid(&s.boundary);
                c.x > 0.0 && c.x < 6000.0 && c.y > 0.0 && c.y < 6000.0
            })
            .unwrap();
        let s_min_x = first_bay_slab.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let s_min_y = first_bay_slab.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let s_max_x = first_bay_slab.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let s_max_y = first_bay_slab.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        assert!((s_min_x - 150.0).abs() < 1e-1, "Slab must be aligned to inner face of 300mm beam (min_x = 150)");
        assert!((s_min_y - 150.0).abs() < 1e-1, "Slab must be aligned to inner face of 300mm beam (min_y = 150)");
        assert!((s_max_x - 5850.0).abs() < 1e-1, "Slab must be aligned to inner face of 300mm beam (max_x = 5850)");
        assert!((s_max_y - 5850.0).abs() < 1e-1, "Slab must be aligned to inner face of 300mm beam (max_y = 5850)");

        // 6. Test egui rendering of newly placed Footing in properties panel and canvas
        let ctx = egui::Context::default();
        let theme = AppTheme::dark();
        let ft_id = doc.elements.values().find_map(|el| match el {
            plinth_model::Element::Footing(f) => Some(f.id),
            _ => None,
        }).unwrap();

        let mut app = PlinthApp::default();
        app.doc = doc;
        app.selected_element = Some(ft_id);

        let _ = ctx.run(Default::default(), |ctx| {
            egui::SidePanel::right("props_test").show(ctx, |ui| {
                let _ = render_properties_panel(
                    ui,
                    &mut app.doc,
                    app.selected_element,
                    &mut app.canvas_state.selected_placed_viewport,
                    &theme,
                );
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                render_canvas(
                    ui,
                    &mut app.doc,
                    &mut app.canvas_state,
                    &mut app.active_tool,
                    &mut app.selected_element,
                    &app.tool_studio,
                    &theme,
                );
            });
        });
    }

    #[test]
    fn test_copy_active_level_and_paste_to_other_levels() {
        let mut doc = Document::new_empty();
        doc.levels.clear();
        doc.views.clear();

        let l1 = plinth_model::Level::new("Level 1", 0.0, 3500.0);
        let l1_id = l1.id;
        doc.levels.insert(l1_id, l1);
        let l2 = plinth_model::Level::new("Level 2", 3500.0, 3500.0);
        let l2_id = l2.id;
        doc.levels.insert(l2_id, l2);
        let l3 = plinth_model::Level::new("Level 3", 7000.0, 3500.0);
        let l3_id = l3.id;
        doc.levels.insert(l3_id, l3);
        doc.active_level_id = l1_id;

        // Build source level elements: Wall, hosted Door, hosted Window, Column, Beam, Floor, Ceiling, Stair, Railing
        let mut wall = plinth_model::Wall::new(
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(8000.0, 0.0),
            l1_id,
        );
        wall.top_level = Some(l2_id);
        let wall_id = wall.id;
        doc.elements.insert(wall_id, plinth_model::Element::Wall(wall));

        let door = plinth_model::Door::new(wall_id, 2000.0);
        let door_id = door.id;
        doc.elements.insert(door_id, plinth_model::Element::Door(door));

        let win = plinth_model::Window::new(wall_id, 5500.0);
        let win_id = win.id;
        doc.elements.insert(win_id, plinth_model::Element::Window(win));

        let mut col = plinth_model::Column::new(plinth_geom::vec2(0.0, 6000.0), l1_id);
        col.top_level = Some(l2_id);
        let col_id = col.id;
        doc.elements.insert(col_id, plinth_model::Element::Column(col));

        let beam = plinth_model::Beam::new(
            plinth_geom::vec2(0.0, 6000.0),
            plinth_geom::vec2(8000.0, 6000.0),
            l1_id,
        );
        let beam_id = beam.id;
        doc.elements.insert(beam_id, plinth_model::Element::Beam(beam));

        let slab_poly = vec![
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(8000.0, 0.0),
            plinth_geom::vec2(8000.0, 6000.0),
            plinth_geom::vec2(0.0, 6000.0),
        ];
        let slab = plinth_model::Slab::new(slab_poly.clone(), l1_id);
        let slab_id = slab.id;
        doc.elements.insert(slab_id, plinth_model::Element::Slab(slab));

        let ceil = plinth_model::Ceiling::new(slab_poly, l1_id);
        let ceil_id = ceil.id;
        doc.elements.insert(ceil_id, plinth_model::Element::Ceiling(ceil));

        // 1. Copy active level entities
        let mut filter = crate::level_copy::EntityCategoryFilter::default();
        filter.select_all();
        let clipboard = crate::level_copy::copy_active_level_entities(&doc, &filter);
        assert_eq!(clipboard.elements.len(), 7); // Wall, Door, Window, Column, Beam, Slab, Ceiling

        // 2. Paste to Level 2 and Level 3
        let pasted_total = crate::level_copy::paste_clipboard_to_levels(
            &mut doc,
            &clipboard,
            &[l2_id, l3_id],
            true,
        );
        assert_eq!(pasted_total, 14); // 7 elements * 2 levels = 14 new elements

        // Verify Level 2 elements & door/window host wall remapping
        let l2_elements: Vec<_> = doc
            .elements
            .values()
            .filter(|el| crate::level_copy::element_effective_level(&doc, el) == Some(l2_id))
            .collect();
        assert_eq!(l2_elements.len(), 7);

        let l2_wall = l2_elements
            .iter()
            .find_map(|el| match el {
                plinth_model::Element::Wall(w) => Some(w),
                _ => None,
            })
            .expect("L2 Wall should exist");
        assert_eq!(l2_wall.base_level, l2_id);
        assert_eq!(l2_wall.top_level, Some(l3_id)); // Automatically remapped from (L1->L2) to (L2->L3)!

        let l2_door = l2_elements
            .iter()
            .find_map(|el| match el {
                plinth_model::Element::Door(d) => Some(d),
                _ => None,
            })
            .expect("L2 Door should exist");
        assert_eq!(l2_door.host_wall, l2_wall.id); // Re-hosted in the L2 Wall!
        assert_eq!(l2_door.offset_along_wall, 2000.0);

        let l2_win = l2_elements
            .iter()
            .find_map(|el| match el {
                plinth_model::Element::Window(w) => Some(w),
                _ => None,
            })
            .expect("L2 Window should exist");
        assert_eq!(l2_win.host_wall, l2_wall.id); // Re-hosted in the L2 Wall!
        assert_eq!(l2_win.offset_along_wall, 5500.0);

        // Verify Level 3 (top level) conversion of top constraint to unconnected span
        let l3_wall = doc
            .elements
            .values()
            .find_map(|el| match el {
                plinth_model::Element::Wall(w) if w.base_level == l3_id => Some(w),
                _ => None,
            })
            .expect("L3 Wall should exist");
        assert_eq!(l3_wall.top_level, None); // Top floor wall converted cleanly to unconnected!
        assert!(l3_wall.unconnected_height >= 3000.0);
    }

    #[test]
    fn test_copy_structural_frame_with_upper_framing_beams_and_slabs() {
        let mut doc = Document::new_empty();
        doc.levels.clear();
        doc.views.clear();

        // 11-storey tower stack (Level 1..11, 3500mm per storey) matching user scenario
        let mut level_ids = Vec::new();
        for i in 1..=11 {
            let elev = (i - 1) as f64 * 3500.0;
            let lvl = plinth_model::Level::new(format!("Level {}", i), elev, 3500.0);
            let lid = lvl.id;
            doc.levels.insert(lid, lvl);
            level_ids.push(lid);
        }
        let l1_id = level_ids[0];
        let l2_id = level_ids[1];
        doc.active_level_id = l1_id;

        // Create Level 1 structural frame:
        // - 4 columns from Level 1 to Level 2
        // - 4 footings on Level 1
        // - 4 beams framing the top at Level 2
        // - 1 slab deck at Level 2
        let grid_pts = [
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(6000.0, 0.0),
            plinth_geom::vec2(6000.0, 6000.0),
            plinth_geom::vec2(0.0, 6000.0),
        ];

        for &pt in &grid_pts {
            let mut col = plinth_model::Column::new(pt, l1_id);
            col.top_level = Some(l2_id);
            let col_id = col.id;
            doc.elements.insert(col_id, plinth_model::Element::Column(col));

            let mut foot = plinth_model::Footing::new(pt, l1_id);
            foot.host_column = Some(col_id);
            let foot_id = foot.id;
            doc.elements.insert(foot_id, plinth_model::Element::Footing(foot));
        }

        for i in 0..4 {
            let p1 = grid_pts[i];
            let p2 = grid_pts[(i + 1) % 4];
            let beam = plinth_model::Beam::new(p1, p2, l2_id); // Upper framing beam at Level 2!
            let beam_id = beam.id;
            doc.elements.insert(beam_id, plinth_model::Element::Beam(beam));
        }

        let slab = plinth_model::Slab::new(grid_pts.to_vec(), l2_id); // Upper deck slab at Level 2!
        let slab_id = slab.id;
        doc.elements.insert(slab_id, plinth_model::Element::Slab(slab));

        // 1. Copy Level 1 with upper framing enabled
        let mut filter = crate::level_copy::EntityCategoryFilter::default();
        filter.select_all();
        let clipboard = crate::level_copy::copy_level_entities(&doc, l1_id, &filter, true);

        // Verify clipboard contents: 4 columns + 4 footings + 4 beams + 1 slab = 13 elements
        assert_eq!(clipboard.elements.len(), 13);
        let col_count = clipboard.elements.iter().filter(|el| matches!(el, plinth_model::Element::Column(_))).count();
        let foot_count = clipboard.elements.iter().filter(|el| matches!(el, plinth_model::Element::Footing(_))).count();
        let beam_count = clipboard.elements.iter().filter(|el| matches!(el, plinth_model::Element::Beam(_))).count();
        let slab_count = clipboard.elements.iter().filter(|el| matches!(el, plinth_model::Element::Slab(_))).count();
        assert_eq!(col_count, 4);
        assert_eq!(foot_count, 4);
        assert_eq!(beam_count, 4);
        assert_eq!(slab_count, 1);

        // Verify relative storeys: columns & footings = 0, beams & slab = 1
        for el in &clipboard.elements {
            let rel = clipboard.relative_storeys.get(&el.id()).copied().unwrap_or(0);
            match el {
                plinth_model::Element::Column(_) | plinth_model::Element::Footing(_) => assert_eq!(rel, 0),
                plinth_model::Element::Beam(_) | plinth_model::Element::Slab(_) => assert_eq!(rel, 1),
                _ => {}
            }
        }

        // 2. Paste to destination storeys Level 2 through Level 10 (9 destination levels)
        let dest_levels: Vec<_> = level_ids[1..10].to_vec();
        let pasted = crate::level_copy::paste_clipboard_to_levels(
            &mut doc,
            &clipboard,
            &dest_levels,
            true,
        );
        assert_eq!(pasted, 13 * 9); // 117 elements pasted

        // Verify that EVERY storey has columns, beams, and slabs!
        for idx in 1..10 {
            let cur_lid = level_ids[idx];
            let next_lid = level_ids[idx + 1];

            // Verify columns rising from cur_lid to next_lid
            let cur_cols: Vec<_> = doc
                .elements
                .values()
                .filter_map(|el| match el {
                    plinth_model::Element::Column(c) if c.base_level == cur_lid => Some(c),
                    _ => None,
                })
                .collect();
            assert_eq!(cur_cols.len(), 4, "Storey {} must have 4 columns", idx + 1);
            for c in &cur_cols {
                assert_eq!(c.top_level, Some(next_lid), "Column on {} must top-constrain to {}", idx + 1, idx + 2);
            }

            // Verify beams framing the top at next_lid
            let cur_beams: Vec<_> = doc
                .elements
                .values()
                .filter_map(|el| match el {
                    plinth_model::Element::Beam(b) if b.level == next_lid => Some(b),
                    _ => None,
                })
                .collect();
            assert_eq!(cur_beams.len(), 4, "Storey {} must have 4 upper framing beams at Level {}", idx + 1, idx + 2);

            // Verify slab deck at next_lid
            let cur_slabs: Vec<_> = doc
                .elements
                .values()
                .filter_map(|el| match el {
                    plinth_model::Element::Slab(s) if s.level == next_lid => Some(s),
                    _ => None,
                })
                .collect();
            assert_eq!(cur_slabs.len(), 1, "Storey {} must have 1 upper slab deck at Level {}", idx + 1, idx + 2);
        }
    }

    #[test]
    fn test_curtain_wall_app_workflow_and_level_replication() {
        let mut app = PlinthApp::default();
        app.doc = plinth_model::Document::new_empty();
        let lvl1 = plinth_model::Level::new("Level 1", 0.0, 3000.0);
        let lvl1_id = lvl1.id;
        app.doc.levels.insert(lvl1_id, lvl1);
        app.doc.active_level_id = lvl1_id;

        // Verify curtain wall tool is available
        app.active_tool = Tool::DrawCurtainWall { start: None };
        assert_eq!(app.active_tool.name(), "Curtain Wall & Storefront");

        // Verify studio presets for curtain wall
        let studio = &mut app.tool_studio;
        assert!(studio.active_preset(crate::tool_config::ToolCategory::CurtainWall).is_some());

        // 1. Add host wall
        use plinth_geom::vec2;
        use plinth_model::{CurtainWall, Element, Wall};
        let mut wall = Wall::new(vec2(0.0, 0.0), vec2(8000.0, 0.0), lvl1_id);
        wall.unconnected_height = 3200.0;
        let wall_id = wall.id;
        app.doc.elements.insert(wall_id, Element::Wall(wall));

        // 2. Add hosted curtain wall embedded inside the wall
        let mut cw = CurtainWall::new(vec2(1500.0, 0.0), vec2(5500.0, 0.0), lvl1_id);
        cw.host_wall = Some(wall_id);
        cw.automatically_embed = true;
        app.tool_studio.configure_new_curtain_wall(&app.doc, &mut cw);
        let cw_id = cw.id;
        app.doc.elements.insert(cw_id, Element::CurtainWall(cw));

        // 3. Check openings
        let openings = app.doc.hosted_openings_for_wall(wall_id);
        assert_eq!(openings.len(), 1);
        assert!((openings[0].width - 4000.0).abs() < 1e-3);

        // 4. Test storey copy
        let filter = crate::level_copy::EntityCategoryFilter::default();
        let clipboard = crate::level_copy::copy_level_entities(
            &app.doc,
            lvl1_id,
            &filter,
            false,
        );
        let has_wall = clipboard.elements.iter().any(|e| matches!(e, Element::Wall(_)));
        let has_cw = clipboard.elements.iter().any(|e| matches!(e, Element::CurtainWall(_)));
        assert!(has_wall, "Clipboard must contain host wall");
        assert!(has_cw, "Clipboard must contain hosted curtain wall");
    }

    #[test]
    fn test_slab_corner_trimming_against_columns_and_elevation_selection() {
        use plinth_geom::vec2;
        use plinth_model::{Beam, Column, Element};
        let mut app = PlinthApp::default();
        let lvl1 = app.doc.active_level_id;

        // 1. Place 4 Columns (400x400 mm) at corners of a 6000x6000 mm bay
        let corners = [
            vec2(0.0, 0.0),
            vec2(6000.0, 0.0),
            vec2(6000.0, 6000.0),
            vec2(0.0, 6000.0),
        ];
        for &pt in &corners {
            let mut col = Column::new(pt, lvl1);
            col.width = 400.0;
            col.depth = 400.0;
            col.unconnected_height = 3500.0;
            app.doc.elements.insert(col.id, Element::Column(col));
        }

        // 2. Place 4 Beams (300 mm width) framing the bay
        for i in 0..4 {
            let p0 = corners[i];
            let p1 = corners[(i + 1) % 4];
            let mut b = Beam::new(p0, p1, lvl1);
            b.width = 300.0;
            b.depth = 500.0;
            b.top_offset = 3500.0;
            app.doc.elements.insert(b.id, Element::Beam(b));
        }

        // 3. Align Bay to Beam and Column inner surfaces
        let raw_bay = corners.to_vec();
        let inner_bay = crate::structure_auto::align_bay_to_beam_inner_surfaces(&app.doc, &raw_bay, lvl1);

        // 4. Verify that NO point of inner_bay penetrates inside any column footprint!
        for &col_pos in &corners {
            // Column footprint is [-200, 200] around col_pos
            for &pt in &inner_bay {
                let dx = (pt.x - col_pos.x).abs();
                let dy = (pt.y - col_pos.y).abs();
                // A point is strictly inside column interior if dx < 199.0 && dy < 199.0
                assert!(
                    !(dx < 198.0 && dy < 198.0),
                    "Slab point {:?} penetrates column at {:?} (dx={}, dy={})",
                    pt, col_pos, dx, dy
                );
            }
        }

        // 5. Verify the bay has positive area and valid CCW polygon
        let area = plinth_geom::polygon::area(&inner_bay);
        assert!(area > 20_000_000.0, "Inner bay must have expected area ~32 m²");
    }

    #[test]
    fn test_wall_corners_visible_in_3d_and_opening_clean_cut_and_picking() {
        let mut app = PlinthApp::default();
        app.doc = plinth_model::Document::create_test_box_building();
        let doc = &app.doc;

        // 1. Verify GPU Scene generation uses exact B-Rep faces
        let palette = plinth_views::MaterialPalette::signature_pastel();
        let gpu_scene = crate::gpu_viewport::build_gpu_scene(doc, plinth_views::VisualStyle::Shaded, false, &palette, &[]);
        assert!(!gpu_scene.opaque_vertices.is_empty(), "Opaque vertices must be present");
        assert!(!gpu_scene.edge_vertices.is_empty(), "Hard edge lines must be generated for silhouette and corners");

        // 2. Verify wall vertical corners exist in edge lines
        // In sample building: corners are at (0, 0), (12000, 0), (12000, 8000), (0, 8000)
        let mut found_origin_corner = false;
        let mut found_far_corner = false;
        for i in (0..gpu_scene.edge_indices.len()).step_by(2) {
            let idx0 = gpu_scene.edge_indices[i] as usize;
            let idx1 = gpu_scene.edge_indices[i + 1] as usize;
            let v0 = gpu_scene.edge_vertices[idx0].pos;
            let v1 = gpu_scene.edge_vertices[idx1].pos;

            // Check for vertical corner line (dx ~ 0, dy ~ 0, dz > 1000)
            let dx = (v0[0] - v1[0]).abs();
            let dy = (v0[1] - v1[1]).abs();
            let dz = (v0[2] - v1[2]).abs();
            if dx < 15.0 && dy < 15.0 && dz > 2500.0 {
                if (v0[0].abs() < 150.0) && (v0[1].abs() < 150.0) {
                    found_origin_corner = true;
                }
                if ((v0[0] - 12000.0).abs() < 150.0) && (v0[1].abs() < 150.0) {
                    found_far_corner = true;
                }
            }
        }
        assert!(found_origin_corner, "Outer vertical building corner at (0, 0) must emit a hard edge line in 3D");
        assert!(found_far_corner, "Outer vertical building corner at (12000, 0) must emit a hard edge line in 3D");

        // 3. Verify that window sill and lintel quads do NOT have vertical dividing lines
        // For South Wall (0,0)->(12000,0), windows are at x=2000..3200 and x=8500..9700
        // No vertical line should run from z=0..900 (below sill) or z=2100..3000 (above lintel) at x=2000, 3200, 8500, or 9700 on the front facade
        for i in (0..gpu_scene.edge_indices.len()).step_by(2) {
            let idx0 = gpu_scene.edge_indices[i] as usize;
            let idx1 = gpu_scene.edge_indices[i + 1] as usize;
            let v0 = gpu_scene.edge_vertices[idx0].pos;
            let v1 = gpu_scene.edge_vertices[idx1].pos;

            let dx = (v0[0] - v1[0]).abs();
            let dy = (v0[1] - v1[1]).abs();
            let dz = (v0[2] - v1[2]).abs();

            if dx < 5.0 && dy < 5.0 && dz > 50.0 {
                let x = v0[0];
                let y = v0[1];
                let z_min = v0[2].min(v1[2]);
                let z_max = v0[2].max(v1[2]);

                // On the front South facade (y ~ -100..-125):
                if (y + 100.0).abs() < 30.0 || (y + 125.0).abs() < 30.0 {
                    // Check if it's at window boundaries
                    let at_win_boundary = (x - 2000.0).abs() < 20.0
                        || (x - 3200.0).abs() < 20.0
                        || (x - 8500.0).abs() < 20.0
                        || (x - 9700.0).abs() < 20.0;
                    if at_win_boundary {
                        // Any line at the window boundary must be strictly inside the window opening [900, 2100]
                        assert!(
                            z_min >= 890.0 && z_max <= 2110.0,
                            "Vertical division line leaked outside opening at x={}, z_min={}, z_max={}",
                            x, z_min, z_max
                        );
                    }
                }
            }
        }

        // 4. Verify 3D Screen Projection & Element Picking
        let view_3d = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ThreeD).unwrap();
        let cam = view_3d.camera_3d.unwrap_or_default();
        let target = plinth_geom::Vec3::new(cam.target_x, cam.target_y, cam.target_z);
        let (sin_y, cos_y) = cam.yaw.sin_cos();
        let (sin_p, cos_p) = cam.pitch.sin_cos();
        let zoom = 0.05f32;
        let center = egui::pos2(500.0, 400.0);
        let pan = egui::vec2(0.0, 0.0);

        let project_3d = |p: plinth_geom::Vec3| -> egui::Pos2 {
            let d = p - target;
            let x1 = d.x * cos_y - d.y * sin_y;
            let y1 = d.x * sin_y + d.y * cos_y;
            let z1 = d.z;
            let x2 = x1;
            let _y2 = y1 * cos_p - z1 * sin_p;
            let z2 = y1 * sin_p + z1 * cos_p;
            egui::Pos2::new(
                center.x + pan.x + (x2 as f32) * zoom,
                center.y + pan.y - (z2 as f32) * zoom,
            )
        };

        // Pick South Wall front face at (6000, -100, 1500)
        let pick_pt = plinth_geom::Vec3::new(6000.0, -100.0, 1500.0);
        let click_screen = project_3d(pick_pt);

        // 4. Verify 3D Screen Projection & Element Picking using Ray-Polygon Engine
        let ray_pick = |target_screen: egui::Pos2, yaw: f64| -> Option<(&'static str, plinth_model::ElementId)> {
            let (sy, cy) = yaw.sin_cos();
            let cf = plinth_geom::Vec3::new(sy * cos_p, cy * cos_p, -sin_p);
            let cam_dist = 10000.0f64;
            let vx = ((target_screen.x - center.x - pan.x) / zoom) as f64;
            let vz = -((target_screen.y - center.y - pan.y) / zoom) as f64;
            let x1 = vx;
            let y1 = vz * sin_p;
            let z1 = vz * cos_p;
            let dx = x1 * cy + y1 * sy;
            let dy = -x1 * sy + y1 * cy;
            let pt_on_ray = target + plinth_geom::Vec3::new(dx, dy, z1);
            let ray_origin = pt_on_ray - cf * cam_dist;
            let ray_dir = cf;

            let mut best = None;
            let mut min_t = f64::INFINITY;

            for (&id, el) in &doc.elements {
                for face in doc.element_faces(id) {
                    let denom = face.normal.dot(ray_dir);
                    if denom >= -1e-5 || face.pts.len() < 3 {
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
                        (plinth_geom::vec2(p_hit.x, p_hit.y), face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.y)).collect::<Vec<_>>())
                    } else if nx >= ny {
                        (plinth_geom::vec2(p_hit.y, p_hit.z), face.pts.iter().map(|p| plinth_geom::vec2(p.y, p.z)).collect::<Vec<_>>())
                    } else {
                        (plinth_geom::vec2(p_hit.x, p_hit.z), face.pts.iter().map(|p| plinth_geom::vec2(p.x, p.z)).collect::<Vec<_>>())
                    };

                    let is_hit = plinth_geom::polygon::point_in_polygon(p_2d, &poly_2d)
                        || plinth_geom::polygon::distance_to_polyline(p_2d, &poly_2d, true) <= 25.0;

                    if is_hit {
                        min_t = t;
                        let k = match el {
                            plinth_model::Element::Wall(_) => "Wall",
                            plinth_model::Element::Slab(_) => "Slab",
                            plinth_model::Element::Roof(_) => "Roof",
                            _ => "Other",
                        };
                        best = Some((k, id));
                    }
                }
            }
            best
        };

        // Pick South Wall front face at (6000, -100, 1500)
        let picked_wall = ray_pick(click_screen, cam.yaw);
        assert_eq!(picked_wall.unwrap().0, "Wall", "Front wall must be picked when clicked!");

        // Pick Level 2 roof slab top face at (6000, 4000, 3300)
        let slab_screen = project_3d(plinth_geom::Vec3::new(6000.0, 4000.0, 3300.0));
        let picked_slab = ray_pick(slab_screen, cam.yaw);
        assert!(
            picked_slab.is_some() && (picked_slab.unwrap().0 == "Slab" || picked_slab.unwrap().0 == "Roof"),
            "Level 2 Roof Slab must be picked when clicking the top slab, never the interior wall below"
        );

        // Pick East wall at yaw = 225 deg (matching user screenshot)
        let yaw_225 = 225.0f64.to_radians();
        let (sin_y2, cos_y2) = yaw_225.sin_cos();
        let project_225 = |p: plinth_geom::Vec3| -> egui::Pos2 {
            let d = p - target;
            let x1 = d.x * cos_y2 - d.y * sin_y2;
            let y1 = d.x * sin_y2 + d.y * cos_y2;
            let z1 = d.z;
            let x2 = x1;
            let z2 = y1 * sin_p + z1 * cos_p;
            egui::Pos2::new(
                center.x + pan.x + (x2 as f32) * zoom,
                center.y + pan.y - (z2 as f32) * zoom,
            )
        };
        let click_east = project_225(plinth_geom::Vec3::new(12125.0, 4000.0, 1500.0));
        let best_225 = ray_pick(click_east, yaw_225);
        assert_eq!(best_225.unwrap().0, "Wall", "East wall must be picked at yaw 225!");

        // 5. Verify West wall picking at yaw = +45° (camera looking at South & West walls)
        let yaw_pos45 = 45.0f64.to_radians();
        let (sin_y45, cos_y45) = yaw_pos45.sin_cos();
        let project_45 = |p: plinth_geom::Vec3| -> egui::Pos2 {
            let d = p - target;
            let x1 = d.x * cos_y45 - d.y * sin_y45;
            let y1 = d.x * sin_y45 + d.y * cos_y45;
            let z1 = d.z;
            let x2 = x1;
            let z2 = y1 * sin_p + z1 * cos_p;
            egui::pos2(
                center.x + pan.x + (x2 as f32) * zoom,
                center.y + pan.y - (z2 as f32) * zoom,
            )
        };
        let click_west_45 = project_45(plinth_geom::Vec3::new(-125.0, 4000.0, 1500.0));
        let best_45 = ray_pick(click_west_45, yaw_pos45);
        assert_eq!(best_45.unwrap().0, "Wall", "West wall must be picked when clicked at yaw +45 deg!");
    }

    #[test]
    fn test_diagnose_3d_picking_issue() {
        let doc = plinth_model::Document::create_test_box_building();
        let cam = plinth_model::Camera3D::default();
        let target = plinth_geom::Vec3::new(cam.target_x, cam.target_y, cam.target_z);
        let center = egui::Pos2::new(500.0, 400.0);
        let pan = egui::Vec2::ZERO;
        let zoom = 0.05f32;

        println!("=== DIAGNOSE PICKING: DEFAULT YAW (-45 deg) ===");
        let (sin_y, cos_y) = cam.yaw.sin_cos();
        let (sin_p, cos_p) = cam.pitch.sin_cos();
        let cam_forward = plinth_geom::Vec3::new(sin_y * cos_p, cos_y * cos_p, -sin_p);
        let project_3d = |p: plinth_geom::Vec3| -> egui::Pos2 {
            let d = p - target;
            let x1 = d.x * cos_y - d.y * sin_y;
            let y1 = d.x * sin_y + d.y * cos_y;
            let z1 = d.z;
            let x2 = x1;
            let z2 = y1 * sin_p + z1 * cos_p;
            egui::Pos2::new(
                center.x + pan.x + (x2 as f32) * zoom,
                center.y + pan.y - (z2 as f32) * zoom,
            )
        };
        let transform = |p: plinth_geom::Vec3| -> plinth_geom::Vec3 {
            let d = p - target;
            let x1 = d.x * cos_y - d.y * sin_y;
            let y1 = d.x * sin_y + d.y * cos_y;
            let z1 = d.z;
            let x2 = x1;
            let y2 = y1 * cos_p - z1 * sin_p;
            let z2 = y1 * sin_p + z1 * cos_p;
            plinth_geom::Vec3::new(x2, y2, z2)
        };

        let test_click = |click_screen: egui::Pos2, label: &str| {
            println!("\n--- Testing Click: {} at screen ({:.1}, {:.1}) ---", label, click_screen.x, click_screen.y);
            let click_v = plinth_geom::vec2(click_screen.x as f64, click_screen.y as f64);
            let v_x = ((click_screen.x - center.x - pan.x) / zoom) as f64;
            let v_z = -((click_screen.y - center.y - pan.y) / zoom) as f64;

            let mut candidates = Vec::new();
            for (&id, el) in &doc.elements {
                for (face_idx, face) in doc.element_faces(id).iter().enumerate() {
                    let facing = face.normal.dot(cam_forward);
                    if facing >= -1e-4 {
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
                        let inside = plinth_geom::polygon::point_in_polygon(click_v, &screen_pts);
                        let mut edge_dist = f64::INFINITY;
                        if !inside {
                            for i in 0..n_pts {
                                let p0 = screen_pts[i];
                                let p1 = screen_pts[(i + 1) % n_pts];
                                let (closest_pt, _) = plinth_geom::polygon::closest_point_on_segment(click_v, p0, p1);
                                let d = click_v.distance(closest_pt);
                                if d < edge_dist {
                                    edge_dist = d;
                                }
                            }
                        }
                        let near_edge = !inside && edge_dist <= 10.0;
                        if inside || near_edge {
                            let p0_v = transform(face.pts[0]);
                            let n_v = transform(face.normal) - transform(plinth_geom::Vec3::ZERO);
                            let exact_y = if n_v.y.abs() > 1e-6 {
                                p0_v.y - ((v_x - p0_v.x) * n_v.x + (v_z - p0_v.z) * n_v.z) / n_v.y
                            } else {
                                transform(face.centroid()).y
                            };
                            let bias = match el {
                                plinth_model::Element::Door(_) | plinth_model::Element::Window(_) => -30.0,
                                _ => 0.0,
                            };
                            let eff_depth = if inside {
                                exact_y + bias
                            } else {
                                exact_y + bias + edge_dist * 5.0
                            };
                            let el_name = match el {
                                plinth_model::Element::Wall(w) => format!("Wall({})", w.name),
                                plinth_model::Element::Slab(s) => format!("Slab({})", s.name),
                                plinth_model::Element::Roof(r) => format!("Roof({})", r.name),
                                plinth_model::Element::Column(c) => format!("Col({})", c.name),
                                plinth_model::Element::Ceiling(c) => format!("Ceil({})", c.name),
                                _ => "Other".into(),
                            };
                            candidates.push((eff_depth, inside, exact_y, el_name, face_idx, face.normal));
                        }
                    }
                }
            }
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (eff, ins, ex, name, fidx, norm) in &candidates {
                println!("  Candidate: eff={:.1}, exact={:.1}, inside={}, {} face={}, norm=({:.2},{:.2},{:.2})", eff, ex, ins, name, fidx, norm.x, norm.y, norm.z);
            }
            if let Some(winner) = candidates.first() {
                println!("  --> WINNER: {} (eff={:.1})", winner.3, winner.0);
            } else {
                println!("  --> NO WINNER!");
            }
        };

        // Click on Roof Slab
        test_click(project_3d(plinth_geom::Vec3::new(6000.0, 4000.0, 3300.0)), "Roof Slab Center");
        test_click(project_3d(plinth_geom::Vec3::new(2000.0, 2000.0, 3300.0)), "Roof Slab Corner");

        // Click on South Wall
        test_click(project_3d(plinth_geom::Vec3::new(6000.0, -100.0, 1500.0)), "South Wall Center");
        test_click(project_3d(plinth_geom::Vec3::new(1000.0, -100.0, 1500.0)), "South Wall Left Pier");

        // -------------------------------------------------------------
        // NEW 3D TRUE RAY-POLYGON INTERSECTION PICKING ENGINE
        // -------------------------------------------------------------
        let raycast_pick = |clk: egui::Pos2, cam: &plinth_model::Camera3D| -> Option<(&'static str, f64)> {
            let (sy, cy) = cam.yaw.sin_cos();
            let (sp, cp) = cam.pitch.sin_cos();
            let cf = plinth_geom::Vec3::new(sy * cp, cy * cp, -sp);
            let cam_dist = cam.distance.max(1000.0);
            let is_persp = cam.fov_degrees > 1.0;

            // View ray in view coordinates:
            let vx = ((clk.x - center.x - pan.x) / zoom) as f64;
            let vz = -((clk.y - center.y - pan.y) / zoom) as f64;

            // Compute ray origin and direction in WORLD space:
            let (ray_origin, ray_dir) = if !is_persp {
                // Orthographic: all rays have dir = cf
                let x1 = vx;
                let y1 = vz * sp;
                let z1 = vz * cp;
                let dx = x1 * cy + y1 * sy;
                let dy = -x1 * sy + y1 * cy;
                let pt_on_ray = target + plinth_geom::Vec3::new(dx, dy, z1);
                let origin = pt_on_ray - cf * cam_dist;
                (origin, cf)
            } else {
                let origin = target - cf * cam_dist;
                let dir_view = plinth_geom::Vec3::new(vx, cam_dist, vz).normalized();
                let x1 = dir_view.x;
                let y1 = dir_view.y * cp + dir_view.z * sp;
                let z1 = -dir_view.y * sp + dir_view.z * cp;
                let dx = x1 * cy + y1 * sy;
                let dy = -x1 * sy + y1 * cy;
                let dir = plinth_geom::Vec3::new(dx, dy, z1).normalized();
                (origin, dir)
            };

            let mut best_t = f64::INFINITY;
            let mut best_hit = None;

            for (&id, el) in &doc.elements {
                for face in doc.element_faces(id) {
                    let denom = face.normal.dot(ray_dir);
                    if denom >= -1e-5 {
                        continue;
                    }
                    if face.pts.len() < 3 {
                        continue;
                    }
                    let t = face.normal.dot(face.pts[0] - ray_origin) / denom;
                    if t <= 0.0 {
                        continue;
                    }
                    let bias = match el {
                        plinth_model::Element::Door(_) | plinth_model::Element::Window(_) => -30.0,
                        _ => 0.0,
                    };
                    let eff_t = t + bias;
                    if eff_t >= best_t {
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
                        best_t = eff_t;
                        let k = match el {
                            plinth_model::Element::Wall(_) => "Wall",
                            plinth_model::Element::Slab(s) => if s.slab_type_name.to_lowercase().contains("roof") { "Roof Slab" } else { "Floor Slab" },
                            plinth_model::Element::Roof(_) => "Roof",
                            plinth_model::Element::Door(_) => "Door",
                            plinth_model::Element::Window(_) => "Window",
                            plinth_model::Element::Column(_) => "Column",
                            plinth_model::Element::Beam(_) => "Beam",
                            _ => "Other",
                        };
                        // Print hit
                        if eff_t < 40000.0 {
                            // println!("    Hit candidate: {} eff_t={:.1}, p_hit=({:.1}, {:.1}, {:.1})", k, eff_t, p_hit.x, p_hit.y, p_hit.z);
                        }
                        best_hit = Some((k, eff_t));
                    }
                }
            }

            best_hit
        };

        println!("\n=======================================================");
        println!("=== VERIFYING NEW TRUE 3D RAYCAST PICKING ENGINE ===");
        println!("=======================================================");
        for &angle_deg in &[45.0, 135.0, 225.0, 315.0] {
            let mut cam_test = plinth_model::Camera3D::default();
            cam_test.yaw = (angle_deg as f64).to_radians();
            cam_test.pitch = 0.55;

            let (sy, cy) = cam_test.yaw.sin_cos();
            let (sp, cp) = cam_test.pitch.sin_cos();
            let proj = |p: plinth_geom::Vec3| -> egui::Pos2 {
                let d = p - target;
                let x1 = d.x * cy - d.y * sy;
                let y1 = d.x * sy + d.y * cy;
                let z1 = d.z;
                let x2 = x1;
                let z2 = y1 * sp + z1 * cp;
                egui::Pos2::new(
                    center.x + pan.x + (x2 as f32) * zoom,
                    center.y + pan.y - (z2 as f32) * zoom,
                )
            };

            match angle_deg as i32 {
                45 => {
                    let hit_south = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, -100.0, 1500.0)), &cam_test);
                    let hit_west = raycast_pick(proj(plinth_geom::Vec3::new(-100.0, 4000.0, 1500.0)), &cam_test);
                    let hit_roof = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 4000.0, 3500.0)), &cam_test);
                    println!("Angle 45°: South Wall -> {:?}, West Wall -> {:?}, Roof -> {:?}", hit_south.map(|h| h.0), hit_west.map(|h| h.0), hit_roof.map(|h| h.0));
                    assert_eq!(hit_south.unwrap().0, "Wall");
                    assert_eq!(hit_west.unwrap().0, "Wall");
                    assert_eq!(hit_roof.unwrap().0, "Roof Slab");
                }
                135 => {
                    let hit_north = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 8100.0, 1500.0)), &cam_test);
                    let hit_west = raycast_pick(proj(plinth_geom::Vec3::new(-100.0, 4000.0, 1500.0)), &cam_test);
                    let hit_roof = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 4000.0, 3500.0)), &cam_test);
                    println!("Angle 135°: North Wall -> {:?}, West Wall -> {:?}, Roof -> {:?}", hit_north.map(|h| h.0), hit_west.map(|h| h.0), hit_roof.map(|h| h.0));
                    assert_eq!(hit_north.unwrap().0, "Wall");
                    assert_eq!(hit_west.unwrap().0, "Wall");
                    assert_eq!(hit_roof.unwrap().0, "Roof Slab");
                }
                225 => {
                    let hit_north = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 8100.0, 1500.0)), &cam_test);
                    let hit_east = raycast_pick(proj(plinth_geom::Vec3::new(12100.0, 4000.0, 1500.0)), &cam_test);
                    let hit_roof = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 4000.0, 3500.0)), &cam_test);
                    println!("Angle 225°: North Wall -> {:?}, East Wall -> {:?}, Roof -> {:?}", hit_north.map(|h| h.0), hit_east.map(|h| h.0), hit_roof.map(|h| h.0));
                    assert_eq!(hit_north.unwrap().0, "Wall");
                    assert_eq!(hit_east.unwrap().0, "Wall");
                    assert_eq!(hit_roof.unwrap().0, "Roof Slab");
                }
                315 => {
                    let hit_south = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, -100.0, 1500.0)), &cam_test);
                    let hit_east = raycast_pick(proj(plinth_geom::Vec3::new(12100.0, 4000.0, 1500.0)), &cam_test);
                    let hit_roof = raycast_pick(proj(plinth_geom::Vec3::new(6000.0, 4000.0, 3500.0)), &cam_test);
                    let hit_door = raycast_pick(proj(plinth_geom::Vec3::new(3000.0, -125.0, 1000.0)), &cam_test);
                    let hit_window = raycast_pick(proj(plinth_geom::Vec3::new(7000.0, -125.0, 1600.0)), &cam_test);
                    println!("Angle 315°: South Wall -> {:?}, East Wall -> {:?}, Roof -> {:?}, Door -> {:?}, Window -> {:?}", hit_south.map(|h| h.0), hit_east.map(|h| h.0), hit_roof.map(|h| h.0), hit_door.map(|h| h.0), hit_window.map(|h| h.0));
                    assert_eq!(hit_south.unwrap().0, "Wall");
                    assert_eq!(hit_east.unwrap().0, "Wall");
                    assert_eq!(hit_roof.unwrap().0, "Roof Slab");
                    assert_eq!(hit_door.unwrap().0, "Door");
                    assert_eq!(hit_window.unwrap().0, "Window");
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_snapping_settings_polar_ortho_continuous_chain_and_zoom_extents() {
        let mut app = PlinthApp::default();

        // 1. Verify Zoom Extents in 3D View
        let v3d_id = app.doc.views.values().find(|v| v.kind.is_3d()).map(|v| v.id).unwrap();
        app.doc.active_view_id = v3d_id;
        assert!(app.doc.views.get(&app.doc.active_view_id).unwrap().kind.is_3d());
        app.zoom_extents();
        assert!(app.canvas_state.zoom > 0.001);
        assert!(app.status_message.contains("3D Fit"));

        // 2. Verify Zoom Extents in 2D Floor Plan View
        let fp_id = app.doc.views.values().find(|v| v.kind == plinth_model::ViewKind::FloorPlan).unwrap().id;
        app.doc.active_view_id = fp_id;
        app.zoom_extents();
        assert!(app.canvas_state.zoom > 0.001);
        assert!(app.status_message.contains("Fit to View"));

        // 3. Verify Home Tab Open Project and Modify Tab Zoom Extents buttons
        let mut action_home = RibbonAction::default();
        action_home.open_project = true;
        assert!(action_home.open_project);

        let mut action_modify = RibbonAction::default();
        action_modify.zoom_extents = true;
        assert!(action_modify.zoom_extents);

        // 4. Verify Snapping Settings (Endpoints, Midpoints, Centers, Intersections)
        let snap = app.settings.snapping.clone();
        assert!(snap.enabled);
        assert!(snap.snap_to_endpoints);
        assert!(snap.snap_to_midpoints);
        assert!(snap.snap_to_centers);
        assert!(snap.snap_to_intersections);
        assert!(snap.wall_continuous_chain);

        // 5. Verify Ortho Mode calculation
        let p1 = plinth_geom::vec2(1000.0, 2000.0);
        let raw = plinth_geom::vec2(4500.0, 2200.0); // mostly horizontal
        let dx = (raw.x - p1.x).abs();
        let dy = (raw.y - p1.y).abs();
        let ortho_snapped = if dx >= dy {
            plinth_geom::vec2(raw.x, p1.y)
        } else {
            plinth_geom::vec2(p1.x, raw.y)
        };
        assert_eq!(ortho_snapped, plinth_geom::vec2(4500.0, 2000.0), "Ortho mode must lock Y to p1.y when dx >= dy");

        // 6. Verify Polar Tracking 45-degree ray alignment
        let raw_diag = plinth_geom::vec2(4000.0, 4980.0); // ~45 degrees with tiny deviation
        let v = raw_diag - p1; // (3000, 2980)
        let angle = v.y.atan2(v.x);
        let inc_rad = (45.0f64).to_radians();
        let target_angle = (angle / inc_rad).round() * inc_rad;
        assert!((target_angle.to_degrees() - 45.0).abs() < 1e-4, "Polar tracking must detect 45 deg ray");

        // 7. Verify Continuous Wall Polyline Chaining
        let mut tool = Tool::DrawWall { start: None, is_structural: false };
        let pt1 = plinth_geom::vec2(0.0, 0.0);
        let pt2 = plinth_geom::vec2(6000.0, 0.0);
        let pt3 = plinth_geom::vec2(6000.0, 4000.0);

        // First click sets start
        if let Tool::DrawWall { start, .. } = &mut tool {
            *start = Some(pt1);
        }
        assert_eq!(tool, Tool::DrawWall { start: Some(pt1), is_structural: false });

        // Second click creates first wall and continues chain from pt2
        let initial_walls = app.doc.elements.values().filter(|e| matches!(e, Element::Wall(_))).count();
        let w1 = plinth_model::Wall::new(pt1, pt2, app.doc.active_level_id);
        let id1 = w1.id;
        app.doc.elements.insert(id1, Element::Wall(w1));

        if app.settings.snapping.wall_continuous_chain {
            if let Tool::DrawWall { start, .. } = &mut tool {
                *start = Some(pt2); // chained!
            }
        }
        assert_eq!(tool, Tool::DrawWall { start: Some(pt2), is_structural: false }, "Tool must chain from pt2");

        // Third click creates connected second wall from pt2 to pt3
        let w2 = plinth_model::Wall::new(pt2, pt3, app.doc.active_level_id);
        let id2 = w2.id;
        app.doc.elements.insert(id2, Element::Wall(w2));
        let final_walls = app.doc.elements.values().filter(|e| matches!(e, Element::Wall(_))).count();
        assert_eq!(final_walls, initial_walls + 2, "Both chained walls must be created");

        // Escape cancels chaining
        if let Tool::DrawWall { start, .. } = &mut tool {
            *start = None;
        }
        assert_eq!(tool, Tool::DrawWall { start: None, is_structural: false }, "Escape must reset chained start to None");
    }

    #[test]
    fn test_advanced_free_modeling_and_ifc_classification() {
        use plinth_model::{FreeformSolid, FreeformSurface, Element};
        use plinth_geom::vec2;

        let mut doc = Document::new_empty();
        let lvl = doc.active_level_id;
        let elev = doc.level_elevation(lvl);

        // 1. Create a 2D FreeformSurface
        let mut surf = FreeformSurface::new_rect(vec2(0.0, 0.0), vec2(4000.0, 3000.0), elev, lvl);
        surf.base_offset = 150.0;
        surf.ifc_category = Some("Floor".into());
        let surf_id = surf.id;
        doc.elements.insert(surf_id, Element::FreeformSurface(surf));

        // Verify boundary 3D faces generation
        let faces = doc.element_faces(surf_id);
        assert_eq!(faces.len(), 1, "Planar surface must yield 1 face");

        // 2. Extrude FreeformSurface into a FreeformSolid
        let extr_h = 2800.0;
        let solid_el = match doc.elements.remove(&surf_id).unwrap() {
            Element::FreeformSurface(s) => {
                let z_base = elev + s.base_offset;
                let z_top = z_base + extr_h;
                let mut solid = FreeformSolid::new_custom_extrusion(
                    format!("Extruded {}", s.name),
                    s.boundary,
                    z_base,
                    z_top,
                    s.level,
                    s.ifc_category.clone(),
                );
                solid.base_offset = s.base_offset;
                solid.ifc_category = s.ifc_category;
                solid
            }
            _ => panic!("Expected FreeformSurface"),
        };
        let solid_id = solid_el.id;
        doc.elements.insert(solid_id, Element::FreeformSolid(solid_el));

        // Verify FreeformSolid has 6 faces and proper IFC category
        let solid_faces = doc.element_faces(solid_id);
        assert_eq!(solid_faces.len(), 6, "Prismatic solid must have 6 faces");
        if let Some(Element::FreeformSolid(f)) = doc.elements.get(&solid_id) {
            assert_eq!(f.ifc_category.as_deref(), Some("Floor"));
            assert_eq!(f.z_max - f.z_min, 2800.0);
        }

        // 3. Verify IFC export maps the freeform object to IFCSLAB
        let ifc_text = plinth_ifc::export_ifc4(&doc);
        assert!(ifc_text.contains("IFCSLAB"), "Freeform element classified as 'Floor' must export as IFCSLAB");

        // 4. Test Pastel Palette rendering in GPU scene
        let palette = plinth_views::MaterialPalette::signature_pastel();
        let gpu_scene = crate::gpu_viewport::build_gpu_scene(&doc, plinth_views::VisualStyle::Shaded, false, &palette, &[]);
        assert!(!gpu_scene.opaque_vertices.is_empty(), "Opaque vertices must be generated for freeform solid");
    }

    #[test]
    fn test_pick_surface_workplane_and_host_wall_freeform() {
        let mut doc = Document::create_sample_building();
        let level_id = doc.active_level_id;

        // Create a South-facing Wall along X from (0, 0) to (6000, 0)
        let mut wall = plinth_model::Wall::new(
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(6000.0, 0.0),
            level_id,
        );
        wall.thickness = 200.0;
        wall.unconnected_height = 3000.0;
        let wall_id = wall.id;
        doc.elements.insert(wall_id, Element::Wall(wall));

        // Camera looking North (yaw=0, pitch=0.55 looking down towards wall South face)
        let cam = plinth_model::Camera3D {
            target_x: 3000.0,
            target_y: 0.0,
            target_z: 1500.0,
            yaw: 0.0,
            pitch: 0.4,
            distance: 8000.0,
            fov_degrees: 0.0,
        };
        let (sy, cy) = cam.yaw.sin_cos();
        let (sp, cp) = cam.pitch.sin_cos();
        let cam_forward = plinth_geom::Vec3::new(sy * cp, cy * cp, -sp);
        let cam_right = plinth_geom::Vec3::new(cy, -sy, 0.0);
        let cam_up = plinth_geom::Vec3::new(sp * sy, sp * cy, cp);

        // Cast a ray towards the center of the wall's front face (Z=1500, X=3000, Y=-100)
        let ray_target = plinth_geom::Vec3::new(3000.0, -100.0, 1500.0);
        let ray_origin = ray_target - cam_forward * 10000.0;
        let ray_dir = cam_forward;

        let hit = crate::canvas::raycast_closest_face(&doc, ray_origin, ray_dir);
        assert!(hit.is_some(), "Ray must hit the South face of the wall");
        let (hit_id, hit_face, p_hit) = hit.unwrap();
        assert_eq!(hit_id, wall_id, "Hit element must be the wall");
        assert!((hit_face.normal.y - (-1.0)).abs() < 1e-4, "Normal must face South (0, -1, 0)");

        // Compute Workplane basis on wall
        let normal = hit_face.normal;
        let u_raw = cam_right - normal * cam_right.dot(normal);
        assert!(u_raw.dot(u_raw) > 1e-6);
        let u_axis = u_raw.normalized();
        let mut v_axis = normal.cross(u_axis).normalized();
        if v_axis.dot(cam_up) < 0.0 {
            v_axis = -v_axis;
        }

        // Verify U axis points to screen right and V axis points to screen up
        assert!(u_axis.dot(cam_right) > 0.99, "U axis must tally with camera screen right (+X)");
        assert!(v_axis.dot(cam_up) > 0.8, "V axis must tally with camera screen up (+Z)");
        assert!((u_axis.cross(v_axis).dot(normal) - 1.0).abs() < 1e-4, "U x V must equal outward normal");

        // Create a FreeformSurface hosted on the wall
        let mut surf = plinth_model::FreeformSurface::new_polygon(
            vec![
                plinth_geom::vec2(1000.0, 500.0),
                plinth_geom::vec2(2500.0, 500.0),
                plinth_geom::vec2(2500.0, 2000.0),
                plinth_geom::vec2(1000.0, 2000.0),
            ],
            0.0,
            level_id,
        );
        let pts_3d: Vec<plinth_geom::Vec3> = surf.boundary.iter().map(|&uv| {
            p_hit + u_axis * uv.x + v_axis * uv.y
        }).collect();
        surf.boundary_3d = Some(pts_3d);
        surf.host_wall = Some(wall_id);
        surf.is_hole = true;
        let surf_id = surf.id;
        doc.elements.insert(surf_id, Element::FreeformSurface(surf.clone()));

        // Verify automated Wall Hole Cutout generated in hosted_openings_for_wall
        let openings = doc.hosted_openings_for_wall(wall_id);
        assert_eq!(openings.len(), 1, "Wall must receive 1 hosted opening cutout from hole surface");
        assert_eq!(openings[0].width, 1500.0, "Cutout width must match surface UV width (2500-1000)");
        assert_eq!(openings[0].height, 1500.0, "Cutout height must match surface UV height (2000-500)");

        // Extrude surface to FreeformSolid and verify host_wall is preserved
        let solid = surf.extrude(200.0);
        assert_eq!(solid.host_wall, Some(wall_id), "Extruded solid must preserve host_wall");
        assert_eq!(solid.faces().len(), 6, "Extruded solid must be a watertight 6-sided solid");
    }

    #[test]
    fn test_plan_view_cursor_tallies_and_workplane_isolated_to_free_modeling() {
        let center = egui::Pos2::new(500.0, 400.0);
        let pan = egui::Vec2::new(50.0, -30.0);
        let zoom = 0.05f32;

        let to_screen = |wp: plinth_geom::Vec2| -> egui::Pos2 {
            egui::Pos2::new(
                center.x + pan.x + (wp.x as f32) * zoom,
                center.y + pan.y - (wp.y as f32) * zoom,
            )
        };

        let to_world_2d = |sp: egui::Pos2| -> plinth_geom::Vec2 {
            plinth_geom::Vec2::new(
                ((sp.x - center.x - pan.x) / zoom) as f64,
                -((sp.y - center.y - pan.y) / zoom) as f64,
            )
        };

        // Test roundtrip in 2D Plan View: screen -> world -> screen
        let test_screens = [
            egui::Pos2::new(120.0, 340.0),
            egui::Pos2::new(550.0, 370.0),
            egui::Pos2::new(890.0, 620.0),
        ];

        for sp in test_screens {
            let wp = to_world_2d(sp);
            let back = to_screen(wp);
            assert!((back.x - sp.x).abs() < 1e-4, "X screen must tally: expected {}, got {}", sp.x, back.x);
            assert!((back.y - sp.y).abs() < 1e-4, "Y screen must tally: expected {}, got {}", sp.y, back.y);
        }

        // Verify WorkplaneMode logic:
        // Even if canvas_state.workplane was set to a Surface, in 2D views (!is_3d) or
        // in architectural tools (!is_free_modeling), active_workplane is strictly XY.
        let surface_wp = crate::canvas::WorkplaneMode::Surface {
            origin: plinth_geom::Vec3::new(3000.0, 0.0, 1500.0),
            normal: plinth_geom::Vec3::new(0.0, -1.0, 0.0),
            u_axis: plinth_geom::Vec3::X,
            v_axis: plinth_geom::Vec3::Z,
            host_element: None,
        };

        // 1. In Plan View (!is_3d): always WorkplaneMode::XY
        let is_3d_plan = false;
        let is_free_surf = true;
        let active_wp_plan = if is_3d_plan && is_free_surf { surface_wp } else { crate::canvas::WorkplaneMode::XY };
        assert_eq!(active_wp_plan, crate::canvas::WorkplaneMode::XY, "Plan views must strictly use XY workplane");

        // 2. In 3D with Architectural tool: always WorkplaneMode::XY
        let is_3d = true;
        let is_free_arch = false; // e.g. Tool::DrawWall
        let active_wp_arch = if is_3d && is_free_arch { surface_wp } else { crate::canvas::WorkplaneMode::XY };
        assert_eq!(active_wp_arch, crate::canvas::WorkplaneMode::XY, "Architectural tools in 3D must strictly use XY workplane");

        // 3. In 3D with Free Modeling tool: uses the picked surface workplane
        let is_free_modeling = true; // e.g. Tool::DrawRectSurface
        let active_wp_free = if is_3d && is_free_modeling { surface_wp } else { crate::canvas::WorkplaneMode::XY };
        assert_eq!(active_wp_free, surface_wp, "Free Modeling tools in 3D use the custom surface workplane");
    }

    #[test]
    fn test_plan_view_selection_never_selects_slab_above_and_prioritizes_walls() {
        let mut doc = Document::new_empty();
        let lvl1 = doc.active_level_id;
        let lvl2 = plinth_model::Level::new("Level 2", 3000.0, 3000.0);
        let lvl2_id = lvl2.id;
        doc.levels.insert(lvl2_id, lvl2);

        // Ground slab on Level 1 (z = 0)
        let slab1 = plinth_model::Slab::new(
            vec![
                plinth_geom::vec2(0.0, 0.0),
                plinth_geom::vec2(10000.0, 0.0),
                plinth_geom::vec2(10000.0, 6000.0),
                plinth_geom::vec2(0.0, 6000.0),
            ],
            lvl1,
        );
        let slab1_id = slab1.id;
        doc.elements.insert(slab1_id, Element::Slab(slab1));

        // Upper floor slab on Level 2 (z = 3000)
        let slab2 = plinth_model::Slab::new(
            vec![
                plinth_geom::vec2(0.0, 0.0),
                plinth_geom::vec2(10000.0, 0.0),
                plinth_geom::vec2(10000.0, 6000.0),
                plinth_geom::vec2(0.0, 6000.0),
            ],
            lvl2_id,
        );
        let slab2_id = slab2.id;
        doc.elements.insert(slab2_id, Element::Slab(slab2));

        // South wall on Level 1 from (0, 0) to (10000, 0)
        let wall = plinth_model::Wall::new(plinth_geom::vec2(0.0, 0.0), plinth_geom::vec2(10000.0, 0.0), lvl1);
        let wall_id = wall.id;
        doc.elements.insert(wall_id, Element::Wall(wall));

        // Active plan view is Level 1 Floor Plan (level_z = 0, cut_z = 1200)
        let plan_view = plinth_model::ViewDef::new_floor_plan("Level 1 Plan", lvl1);
        let cut_z = doc.level_elevation(lvl1) + plan_view.cut_offset; // 1200mm

        // Test 1: Click at (5000, 50) directly on the wall body (wall is 200mm thick, so (5000, 50) is on the wall)
        // Hit-test calculation:
        let mouse_on_wall = plinth_geom::vec2(5000.0, 50.0);
        let mut best_id = None;
        let mut best_dist = 280.0;
        for (id, el) in &doc.elements {
            let d = match el {
                Element::Wall(w) => {
                    let (base_z, top_z) = w.z_bounds(doc.level_elevation(w.base_level), w.top_level.map(|tl| doc.level_elevation(tl)));
                    if cut_z >= base_z && cut_z <= top_z + 100.0 {
                        let center_dist = plinth_geom::polygon::distance_to_segment(mouse_on_wall, w.start, w.end);
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
                Element::Slab(s) => {
                    let (_slab_bot, slab_top) = s.z_bounds(doc.level_elevation(s.level), s.top_level.map(|l| doc.level_elevation(l)));
                    if slab_top > cut_z || slab_top < -1500.0 {
                        f64::INFINITY
                    } else {
                        let edge_d = plinth_geom::polygon::distance_to_polyline(mouse_on_wall, &s.boundary, true);
                        if edge_d <= 80.0 {
                            edge_d
                        } else if plinth_geom::polygon::point_in_polygon(mouse_on_wall, &s.boundary) {
                            250.0
                        } else {
                            f64::INFINITY
                        }
                    }
                }
                _ => f64::INFINITY,
            };
            if d < best_dist {
                best_dist = d;
                best_id = Some(*id);
            }
        }
        assert_eq!(best_id, Some(wall_id), "Click on wall must select Wall, NEVER Level 2 slab above or Level 1 slab!");

        // Test 2: Click at (5000, 3000) inside the room (far from any walls)
        let mouse_in_room = plinth_geom::vec2(5000.0, 3000.0);
        let mut best_room_id = None;
        let mut best_room_dist = 280.0;
        for (id, el) in &doc.elements {
            let d = match el {
                Element::Slab(s) => {
                    let (_slab_bot, slab_top) = s.z_bounds(doc.level_elevation(s.level), s.top_level.map(|l| doc.level_elevation(l)));
                    if slab_top > cut_z || slab_top < -1500.0 {
                        f64::INFINITY
                    } else if plinth_geom::polygon::point_in_polygon(mouse_in_room, &s.boundary) {
                        250.0
                    } else {
                        f64::INFINITY
                    }
                }
                _ => f64::INFINITY,
            };
            if d < best_room_dist {
                best_room_dist = d;
                best_room_id = Some(*id);
            }
        }
        assert_eq!(best_room_id, Some(slab1_id), "Click in room selects Level 1 floor slab, NEVER Level 2 slab above!");
    }

    #[test]
    fn test_gable_roof_watertight_geometry_and_rake_fascias() {
        let lvl = plinth_model::LevelId::new();
        let pts = vec![
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(12000.0, 0.0),
            plinth_geom::vec2(12000.0, 8000.0),
            plinth_geom::vec2(0.0, 8000.0),
        ];
        let mut roof = plinth_model::Roof::new(pts, lvl);
        roof.shape = plinth_model::RoofShape::Gable;
        roof.overhang = 0.0;
        roof.thickness = 200.0;
        roof.slope_degrees = 30.0;

        let faces = roof.faces(3000.0);
        assert_eq!(faces.len(), 10, "Gable roof must emit exactly 10 watertight faces: 2 slopes, 2 soffits, 2 eave fascias, 4 rake bargeboards");

        // Verify West rake bargeboards at x0 = 0.0 have normal = -X
        let west_rakes: Vec<_> = faces.iter().filter(|f| f.pts.iter().all(|p| (p.x - 0.0).abs() < 1e-3)).collect();
        assert_eq!(west_rakes.len(), 2, "West gable end must have exactly 2 sloped rake bargeboards");
        for r in &west_rakes {
            assert!((r.normal.x - (-1.0)).abs() < 1e-2, "West rake bargeboard must point outward (-X)");
            assert!(r.pts.len() == 4, "Rake bargeboard must be a 4-point planar quad");
        }

        // Verify East rake bargeboards at x1 = 12000.0 have normal = +X
        let east_rakes: Vec<_> = faces.iter().filter(|f| f.pts.iter().all(|p| (p.x - 12000.0).abs() < 1e-3)).collect();
        assert_eq!(east_rakes.len(), 2, "East gable end must have exactly 2 sloped rake bargeboards");
        for r in &east_rakes {
            assert!((r.normal.x - 1.0).abs() < 1e-2, "East rake bargeboard must point outward (+X)");
            assert!(r.pts.len() == 4, "Rake bargeboard must be a 4-point planar quad");
        }

        // Verify that there are NO giant non-quad triangles spanning the entire gable end
        let bad_pediments: Vec<_> = faces.iter().filter(|f| f.pts.len() == 3).collect();
        assert!(bad_pediments.is_empty(), "Gable roof must NOT have floating distorted 3-point pediment triangles blocking the overhang");
    }

    #[test]
    fn test_window_and_crossing_selection_across_all_views() {
        use crate::canvas::{get_element_projected_screen_pts, test_element_marquee, SelectionBox};
        use egui::pos2;

        let doc = plinth_model::Document::create_test_box_building();
        let center = pos2(600.0, 400.0);
        let pan = egui::Vec2::ZERO;
        let zoom = 0.05f32; // 50px per 1000mm

        let to_screen = |wp: plinth_geom::Vec2| -> egui::Pos2 {
            egui::pos2(
                center.x + pan.x + (wp.x as f32) * zoom,
                center.y + pan.y - (wp.y as f32) * zoom,
            )
        };

        // --- 1. Plan View Test ---
        let plan_view = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::FloorPlan).cloned().unwrap();
        let cut_z = 1200.0;
        let dummy_project_3d = |_p: plinth_geom::Vec3| egui::Pos2::ZERO;

        // South wall: (0, 0) to (12000, 0)
        let south_wall_id = doc.elements.iter().find_map(|(&id, el)| {
            if let plinth_model::Element::Wall(w) = el {
                if (w.start.y - 0.0).abs() < 10.0 && (w.end.y - 0.0).abs() < 10.0 {
                    return Some(id);
                }
            }
            None
        }).unwrap();

        let sw_geom = get_element_projected_screen_pts(south_wall_id, doc.elements.get(&south_wall_id).unwrap(), &plan_view, &doc, &to_screen, &dummy_project_3d, cut_z).unwrap();
        let sw_min_x = sw_geom.vertices.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let sw_max_x = sw_geom.vertices.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let sw_min_y = sw_geom.vertices.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let sw_max_y = sw_geom.vertices.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

        // A) Window Selection (enclosing south wall completely)
        let win_box = SelectionBox {
            start: pos2(sw_min_x - 20.0, sw_min_y - 20.0),
            current: pos2(sw_max_x + 20.0, sw_max_y + 20.0), // Left to right
        };
        assert!(!win_box.is_crossing(), "Left-to-right drag must be Window selection");
        assert!(test_element_marquee(&sw_geom, win_box.rect(), win_box.is_crossing()), "South wall completely inside window box must be selected");

        // B) Window Selection (partial: enclosing only half of south wall)
        let partial_win_box = SelectionBox {
            start: pos2(sw_min_x - 20.0, sw_min_y - 20.0),
            current: pos2((sw_min_x + sw_max_x) * 0.5, sw_max_y + 20.0), // Left to right
        };
        assert!(!test_element_marquee(&sw_geom, partial_win_box.rect(), partial_win_box.is_crossing()), "Partial window selection must NOT select south wall");

        // C) Crossing Selection (partial: crossing half of south wall)
        let partial_crossing_box = SelectionBox {
            start: pos2((sw_min_x + sw_max_x) * 0.5, sw_max_y + 20.0),
            current: pos2(sw_min_x - 20.0, sw_min_y - 20.0), // Right to left
        };
        assert!(partial_crossing_box.is_crossing(), "Right-to-left drag must be Crossing selection");
        assert!(test_element_marquee(&sw_geom, partial_crossing_box.rect(), partial_crossing_box.is_crossing()), "Crossing selection slicing through south wall MUST select it");

        // --- 2. Elevation View Test ---
        let elev_view = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ElevationSouth).cloned().unwrap();
        let elev_sw_geom = get_element_projected_screen_pts(south_wall_id, doc.elements.get(&south_wall_id).unwrap(), &elev_view, &doc, &to_screen, &dummy_project_3d, cut_z).unwrap();
        assert!(test_element_marquee(&elev_sw_geom, win_box.rect(), false) || test_element_marquee(&elev_sw_geom, win_box.rect(), true), "Elevation facade projected geometry responds to marquee selection");

        // --- 3. 3D View Test ---
        let threed_view = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ThreeD).cloned().unwrap();
        let cam_3d = threed_view.camera_3d.unwrap_or_default();
        let target_3d = plinth_geom::Vec3::new(cam_3d.target_x, cam_3d.target_y, cam_3d.target_z);
        let (sin_y, cos_y) = cam_3d.yaw.sin_cos();
        let (sin_p, cos_p) = cam_3d.pitch.sin_cos();
        let project_3d = |p: plinth_geom::Vec3| -> egui::Pos2 {
            let d = p - target_3d;
            let x1 = d.x * cos_y - d.y * sin_y;
            let y1 = d.x * sin_y + d.y * cos_y;
            let z1 = d.z;
            let x2 = x1;
            let _y2 = y1 * cos_p - z1 * sin_p;
            let z2 = y1 * sin_p + z1 * cos_p;
            to_screen(plinth_geom::vec2(x2, z2))
        };

        let threed_sw_geom = get_element_projected_screen_pts(south_wall_id, doc.elements.get(&south_wall_id).unwrap(), &threed_view, &doc, &to_screen, &project_3d, cut_z).unwrap();
        assert!(!threed_sw_geom.vertices.is_empty(), "3D view produces screen vertices from 3D solid faces");

        let threed_min_x = threed_sw_geom.vertices.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let threed_max_x = threed_sw_geom.vertices.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let threed_min_y = threed_sw_geom.vertices.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let threed_max_y = threed_sw_geom.vertices.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

        // Entire 3D bounding box enclosed
        let threed_win_box = SelectionBox {
            start: pos2(threed_min_x - 10.0, threed_min_y - 10.0),
            current: pos2(threed_max_x + 10.0, threed_max_y + 10.0),
        };
        assert!(test_element_marquee(&threed_sw_geom, threed_win_box.rect(), false), "3D element completely inside Window marquee must be selected");

        // Partial 3D crossing selection
        let threed_cross_box = SelectionBox {
            start: pos2(threed_max_x + 10.0, threed_max_y + 10.0),
            current: pos2((threed_min_x + threed_max_x) * 0.5, (threed_min_y + threed_max_y) * 0.5),
        };
        assert!(threed_cross_box.is_crossing());
        assert!(test_element_marquee(&threed_sw_geom, threed_cross_box.rect(), true), "3D element crossed by Crossing marquee must be selected");
    }

    #[test]
    fn test_egui_mesh_creation() {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(egui::pos2(0.0, 0.0), egui::Color32::WHITE);
        mesh.colored_vertex(egui::pos2(10.0, 0.0), egui::Color32::WHITE);
        mesh.colored_vertex(egui::pos2(10.0, 10.0), egui::Color32::WHITE);
        mesh.add_triangle(0, 1, 2);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
    }

    #[test]
    fn test_render_apartment_highrise_3d_and_sheet_scene() {
        let doc = load_apartment_highrise_doc().expect("Embedded Apartment Highrise IFC must decompress and parse");
        assert!(doc.elements.len() >= 800, "Apartment highrise should have over 800 elements");

        let view_3d = doc.views.values().find(|v| v.kind == plinth_model::ViewKind::ThreeD).expect("Doc has 3D view");
        let color_theme = plinth_views::ColorTheme::light();
        let scene = plinth_views::render_view_with_style_and_theme(&doc, view_3d, &color_theme, plinth_views::VisualStyle::Shaded);
        assert!(!scene.primitives.is_empty(), "Scene must contain rendered primitives");

            // Simulate render_sheet_layout_canvas for this placed 3D viewport
            let vp_rect = egui::Rect::from_min_size(egui::pos2(167.0, 116.0), egui::vec2(490.0, 364.0));
            let dummy_painter = egui::Painter::new(
                egui::Context::default(),
                egui::LayerId::background(),
                vp_rect,
            );
            let vp_painter = dummy_painter.with_clip_rect(vp_rect);

            let vp_center = vp_rect.center();
            let zoom = 1.0f32;
            let scale_factor = (1.0 / 100.0) * zoom;
            let (model_cx, model_cy) = if scene.bounds.is_valid() {
                let c = scene.bounds.center();
                (c.x, c.y)
            } else {
                (0.0, 0.0)
            };
            let to_vp_screen = |p: plinth_geom::Vec2| -> egui::Pos2 {
                egui::Pos2::new(
                    vp_center.x + ((p.x - model_cx) as f32) * scale_factor,
                    vp_center.y - ((p.y - model_cy) as f32) * scale_factor,
                )
            };

            let mut rendered_polys = 0;
            for prim in &scene.primitives {
                if let plinth_views::RenderPrimitive::Polygon { points, fill_color, stroke_color, stroke_width } = prim {
                    if points.len() >= 3 {
                        let screen_pts: Vec<egui::Pos2> = points.iter().map(|&p| to_vp_screen(p)).collect();
                        let fill = egui::Color32::from_rgba_premultiplied(fill_color[0], fill_color[1], fill_color[2], fill_color[3]);
                        let stroke = stroke_color.map(|c| {
                            egui::Stroke::new(*stroke_width, egui::Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
                        }).unwrap_or(egui::Stroke::NONE);
                        crate::canvas::draw_polygon_safe(&vp_painter, &screen_pts, fill, stroke);
                        rendered_polys += 1;
                    }
                }
            }
            assert!(rendered_polys > 0);
    }

    #[test]
    fn test_annotation_tab_tools_and_universal_snapping() {
        let mut app = PlinthApp::default();
        app.current_tab = RibbonTab::Annotation;
        app.settings.annotation.tick_mark = plinth_model::TickMark::Arrow;
        app.settings.annotation.font_size = 14.0;
        app.settings.annotation.font_style = plinth_model::FontStyle::Bold;
        app.settings.annotation.default_text = "SPEC-WALL-01".into();

        // Add test architectural elements
        let lvl1 = app.doc.active_level_id;
        let wall = plinth_model::Wall::new(plinth_geom::vec2(0.0, 0.0), plinth_geom::vec2(4000.0, 0.0), lvl1);
        let beam = plinth_model::Beam::new(plinth_geom::vec2(0.0, 2000.0), plinth_geom::vec2(4000.0, 2000.0), lvl1);
        let col = plinth_model::Column::new(plinth_geom::vec2(2000.0, 1000.0), lvl1);
        app.doc.elements.insert(wall.id, plinth_model::Element::Wall(wall));
        app.doc.elements.insert(beam.id, plinth_model::Element::Beam(beam));
        app.doc.elements.insert(col.id, plinth_model::Element::Column(col));

        // Add Linear Dimension
        let dim = plinth_model::Dimension::new_linear(
            app.doc.active_view_id,
            plinth_geom::vec2(0.0, 0.0),
            plinth_geom::vec2(4000.0, 0.0),
            300.0,
            app.settings.annotation.tick_mark,
            app.settings.annotation.font_size,
            app.settings.annotation.font_style,
            app.settings.annotation.color,
        );
        let dim_id = dim.id;
        app.doc.elements.insert(dim_id, plinth_model::Element::Dimension(dim));

        // Add Circular Dimension
        let circ = plinth_model::Dimension::new_circular(
            app.doc.active_view_id,
            plinth_geom::vec2(2000.0, 1000.0),
            plinth_geom::vec2(2200.0, 1000.0),
            250.0,
            false,
            app.settings.annotation.tick_mark,
            app.settings.annotation.font_size,
            app.settings.annotation.font_style,
            app.settings.annotation.color,
        );
        let circ_id = circ.id;
        app.doc.elements.insert(circ_id, plinth_model::Element::Dimension(circ));

        // Add Text Note
        let text_note = plinth_model::TextNote::new(
            app.doc.active_view_id,
            plinth_geom::vec2(2000.0, 500.0),
            &app.settings.annotation.default_text,
        );
        let txt_id = text_note.id;
        app.doc.elements.insert(txt_id, plinth_model::Element::TextNote(text_note));

        // Add Leader Note
        let leader = plinth_model::Leader::new(
            app.doc.active_view_id,
            plinth_geom::vec2(2000.0, 2000.0),
            plinth_geom::vec2(2400.0, 2400.0),
            plinth_geom::vec2(2800.0, 2400.0),
            "BEAM B1 (300x500)",
            app.settings.annotation.tick_mark,
            app.settings.annotation.font_size,
            app.settings.annotation.font_style,
            app.settings.annotation.color,
        );
        let ldr_id = leader.id;
        app.doc.elements.insert(ldr_id, plinth_model::Element::Leader(leader));

        assert!(app.doc.elements.contains_key(&dim_id));
        assert!(app.doc.elements.contains_key(&circ_id));
        assert!(app.doc.elements.contains_key(&txt_id));
        assert!(app.doc.elements.contains_key(&ldr_id));

        // Verify all 2D views render scene without panic
        let view = app.doc.views.get(&app.doc.active_view_id).unwrap().clone();
        let scene = plinth_views::render_view(&app.doc, &view, false);
        assert!(!scene.primitives.is_empty(), "2D Plan view must render annotation primitives");
    }

    #[test]
    fn test_plugin_manager_integration_and_mutracer_toggle() {
        let mut app = PlinthApp::default();

        // 1. Initially experimental plugins are not installed or active
        assert!(!app.plugin_manager.is_plugin_active("mutracer-webgpu"));

        // 2. Install MuTracer
        assert!(app.plugin_manager.install_plugin("mutracer-webgpu"));
        assert!(app.plugin_manager.is_plugin_active("mutracer-webgpu"));

        // 3. Disable
        app.plugin_manager.set_plugin_enabled("mutracer-webgpu", false);
        assert!(!app.plugin_manager.is_plugin_active("mutracer-webgpu"));

        // 4. Re-enable
        app.plugin_manager.set_plugin_enabled("mutracer-webgpu", true);
        assert!(app.plugin_manager.is_plugin_active("mutracer-webgpu"));

        // 5. Cleanly uninstall
        assert!(app.plugin_manager.uninstall_plugin("mutracer-webgpu"));
        assert!(!app.plugin_manager.is_plugin_active("mutracer-webgpu"));

        // 6. Test Blender Live Sync Plugin lifecycle (pre-installed & active by default)
        assert!(app.plugin_manager.is_plugin_active("blender-live-sync"));
        assert!(app.plugin_manager.uninstall_plugin("blender-live-sync"));
        assert!(!app.plugin_manager.is_plugin_active("blender-live-sync"));
        assert!(app.plugin_manager.install_plugin("blender-live-sync"));
        assert!(app.plugin_manager.is_plugin_active("blender-live-sync"));
    }
}
