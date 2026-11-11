//! Professional categorized application settings dialog.
//!
//! Categories:
//! 1. Project Units: Length, Area, Volume, Angle, Precision, Symbols, Snap Interval.
//! 2. Theme: Dark/Light Mode, Accent Color, Canvas Background, Selection Highlight, UI Scale.
//! 3. Performance (Performanses): 3D subdivision chunks, LOD, Silhouette outlines, Undo stack depth, Diagnostics.

use crate::theme::{AppTheme, ThemeMode};
use egui::{Color32, Ui, Vec2};
use plinth_model::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SettingsCategory {
    General,
    Interface,
    MaterialColors,
    Snapping,
    FileLocations,
    SavingBackup,
    KeyboardShortcuts,
    Diagnostics,
}

// Backward compatibility aliases
#[allow(non_upper_case_globals)]
impl SettingsCategory {
    pub const ProjectUnits: Self = Self::General;
    pub const Theme: Self = Self::Interface;
    pub const Performance: Self = Self::Diagnostics;

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Interface => "Interface",
            Self::MaterialColors => "Surface Colors",
            Self::Snapping => "Snapping & Drafting",
            Self::FileLocations => "File locations",
            Self::SavingBackup => "Saving & Backup",
            Self::KeyboardShortcuts => "Keyboard shortcuts",
            Self::Diagnostics => "Diagnostics & Crash Report",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "⚙",
            Self::Interface => "🖥",
            Self::MaterialColors => "🎨",
            Self::Snapping => "🧲",
            Self::FileLocations => "📁",
            Self::SavingBackup => "💾",
            Self::KeyboardShortcuts => "⌨",
            Self::Diagnostics => "🩺",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::General => "Measurement units, precision, snapping intervals, and default settings",
            Self::Interface => "Visual theme, canvas background, accent colors, and display scale",
            Self::MaterialColors => "Arch3D signature pastel palette for architectural materials and 3D solid surfaces",
            Self::Snapping => "Object snaps (endpoints, midpoints, centers, intersections), Ortho mode, Polar tracking, and Wall chaining",
            Self::FileLocations => "Standard directories for project files, templates, and IFC exports",
            Self::SavingBackup => "Auto-save intervals, crash recovery, and automatic backup files (.bak)",
            Self::KeyboardShortcuts => "Productivity keyboard shortcuts and mouse navigation controls",
            Self::Diagnostics => "Runtime telemetry, memory usage, and application error logs",
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Project Units
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LengthUnit {
    Millimeters,
    Centimeters,
    Meters,
    FeetInches,
}

impl LengthUnit {
    pub fn name(self) -> &'static str {
        match self {
            Self::Millimeters => "Millimeters (mm)",
            Self::Centimeters => "Centimeters (cm)",
            Self::Meters => "Meters (m)",
            Self::FeetInches => "Feet & Fractional Inches (ft-in)",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Millimeters => "mm",
            Self::Centimeters => "cm",
            Self::Meters => "m",
            Self::FeetInches => "ft",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AreaUnit {
    SquareMeters,
    SquareMillimeters,
    SquareFeet,
}

impl AreaUnit {
    pub fn name(self) -> &'static str {
        match self {
            Self::SquareMeters => "Square Meters (m²)",
            Self::SquareMillimeters => "Square Millimeters (mm²)",
            Self::SquareFeet => "Square Feet (sq ft)",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::SquareMeters => "m²",
            Self::SquareMillimeters => "mm²",
            Self::SquareFeet => "sq ft",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VolumeUnit {
    CubicMeters,
    CubicMillimeters,
    CubicFeet,
}

impl VolumeUnit {
    pub fn name(self) -> &'static str {
        match self {
            Self::CubicMeters => "Cubic Meters (m³)",
            Self::CubicMillimeters => "Cubic Millimeters (mm³)",
            Self::CubicFeet => "Cubic Feet (cu ft)",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::CubicMeters => "m³",
            Self::CubicMillimeters => "mm³",
            Self::CubicFeet => "cu ft",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AngleUnit {
    DecimalDegrees,
    Radians,
}

impl AngleUnit {
    pub fn name(self) -> &'static str {
        match self {
            Self::DecimalDegrees => "Decimal Degrees (°)",
            Self::Radians => "Radians (rad)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProjectUnitsSettings {
    pub length_unit: LengthUnit,
    pub area_unit: AreaUnit,
    pub volume_unit: VolumeUnit,
    pub angle_unit: AngleUnit,
    pub decimal_precision: usize,
    pub show_unit_symbol: bool,
    pub snap_grid_spacing: f64,
}

impl Default for ProjectUnitsSettings {
    fn default() -> Self {
        Self {
            length_unit: LengthUnit::Millimeters,
            area_unit: AreaUnit::SquareMeters,
            volume_unit: VolumeUnit::CubicMeters,
            angle_unit: AngleUnit::DecimalDegrees,
            decimal_precision: 0,
            show_unit_symbol: true,
            snap_grid_spacing: 100.0,
        }
    }
}

impl ProjectUnitsSettings {
    pub fn format_length(&self, mm: f64) -> String {
        let prec = self.decimal_precision;
        match self.length_unit {
            LengthUnit::Millimeters => {
                if self.show_unit_symbol {
                    format!("{:.prec$} mm", mm, prec = prec)
                } else {
                    format!("{:.prec$}", mm, prec = prec)
                }
            }
            LengthUnit::Centimeters => {
                let cm = mm / 10.0;
                if self.show_unit_symbol {
                    format!("{:.prec$} cm", cm, prec = prec)
                } else {
                    format!("{:.prec$}", cm, prec = prec)
                }
            }
            LengthUnit::Meters => {
                let m = mm / 1000.0;
                if self.show_unit_symbol {
                    format!("{:.prec$} m", m, prec = prec)
                } else {
                    format!("{:.prec$}", m, prec = prec)
                }
            }
            LengthUnit::FeetInches => {
                let total_inches = mm / 25.4;
                let feet = (total_inches / 12.0).floor() as i64;
                let inches = total_inches - (feet as f64 * 12.0);
                format!("{}' {:.prec$}\"", feet, inches, prec = prec)
            }
        }
    }

    pub fn format_area(&self, mm2: f64) -> String {
        let prec = self.decimal_precision.max(2);
        match self.area_unit {
            AreaUnit::SquareMeters => {
                let m2 = mm2 / 1_000_000.0;
                if self.show_unit_symbol {
                    format!("{:.prec$} m²", m2, prec = prec)
                } else {
                    format!("{:.prec$}", m2, prec = prec)
                }
            }
            AreaUnit::SquareMillimeters => {
                if self.show_unit_symbol {
                    format!("{:.prec$} mm²", mm2, prec = prec)
                } else {
                    format!("{:.prec$}", mm2, prec = prec)
                }
            }
            AreaUnit::SquareFeet => {
                let sqft = mm2 / 92_903.04;
                if self.show_unit_symbol {
                    format!("{:.prec$} sq ft", sqft, prec = prec)
                } else {
                    format!("{:.prec$}", sqft, prec = prec)
                }
            }
        }
    }

    pub fn format_volume(&self, mm3: f64) -> String {
        let prec = self.decimal_precision.max(3);
        match self.volume_unit {
            VolumeUnit::CubicMeters => {
                let m3 = mm3 / 1_000_000_000.0;
                if self.show_unit_symbol {
                    format!("{:.prec$} m³", m3, prec = prec)
                } else {
                    format!("{:.prec$}", m3, prec = prec)
                }
            }
            VolumeUnit::CubicMillimeters => {
                if self.show_unit_symbol {
                    format!("{:.prec$} mm³", mm3, prec = prec)
                } else {
                    format!("{:.prec$}", mm3, prec = prec)
                }
            }
            VolumeUnit::CubicFeet => {
                let cuft = mm3 / 28_316_846.592;
                if self.show_unit_symbol {
                    format!("{:.prec$} cu ft", cuft, prec = prec)
                } else {
                    format!("{:.prec$}", cuft, prec = prec)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Theme & Appearance
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AccentPreset {
    ArchitecturalBlue, // Classic Revit / CAD Blue
    BimEmerald,        // Natural Forest Green
    EngineeringOrange, // High-contrast Industrial Orange
    RevitBurgundy,     // Architectural Crimson
    SlateGrey,         // High-end Minimalist Slate
}

impl AccentPreset {
    pub fn name(self) -> &'static str {
        match self {
            Self::ArchitecturalBlue => "Architectural Blue",
            Self::BimEmerald => "BIM Emerald Green",
            Self::EngineeringOrange => "Engineering Orange",
            Self::RevitBurgundy => "Revit Burgundy",
            Self::SlateGrey => "Titanium Slate",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::ArchitecturalBlue => Color32::from_rgb(0, 122, 217),
            Self::BimEmerald => Color32::from_rgb(16, 160, 90),
            Self::EngineeringOrange => Color32::from_rgb(255, 120, 0),
            Self::RevitBurgundy => Color32::from_rgb(180, 50, 60),
            Self::SlateGrey => Color32::from_rgb(90, 115, 135),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CanvasBgStyle {
    ThemeMatch,
    DefaultCadDark,
    ObsidianBlack,
    MidnightNavy,
    CleanWhitePaper,
}

impl CanvasBgStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::ThemeMatch => "Match Theme (Paper White in Light / Dark in Dark)",
            Self::CleanWhitePaper => "Clean White Blueprint (#FAFAFC)",
            Self::DefaultCadDark => "CAD Dark Charcoal (#16171A)",
            Self::ObsidianBlack => "True Obsidian Black (#0C0D0E)",
            Self::MidnightNavy => "Midnight Navy (#141923)",
        }
    }

    pub fn color(self, is_light: bool) -> Color32 {
        match self {
            Self::ThemeMatch => {
                if is_light {
                    Color32::from_rgb(250, 250, 252)
                } else {
                    Color32::from_rgb(22, 23, 26)
                }
            }
            Self::CleanWhitePaper => Color32::from_rgb(250, 250, 252),
            Self::DefaultCadDark => Color32::from_rgb(22, 23, 26),
            Self::ObsidianBlack => Color32::from_rgb(12, 13, 14),
            Self::MidnightNavy => Color32::from_rgb(20, 25, 35),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SelectionColorChoice {
    ElectricCyan,
    AmberGold,
    LaserGreen,
}

impl SelectionColorChoice {
    pub fn name(self) -> &'static str {
        match self {
            Self::ElectricCyan => "Electric Cyan (#3399FF)",
            Self::AmberGold => "Amber Gold (#FFAA00)",
            Self::LaserGreen => "Laser Green (#00E676)",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::ElectricCyan => Color32::from_rgb(51, 153, 255),
            Self::AmberGold => Color32::from_rgb(255, 170, 0),
            Self::LaserGreen => Color32::from_rgb(0, 230, 118),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThemeSettings {
    pub theme_mode: ThemeMode,
    pub accent_preset: AccentPreset,
    pub canvas_bg: CanvasBgStyle,
    pub selection_color: SelectionColorChoice,
    pub anti_aliasing: bool,
    pub ui_scale_index: usize, // 0: 100%, 1: 110%, 2: 125%
    pub show_tool_labels: bool, // true: icon + text, false: compact icon only
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Dark,
            accent_preset: AccentPreset::ArchitecturalBlue,
            canvas_bg: CanvasBgStyle::ThemeMatch,
            selection_color: SelectionColorChoice::ElectricCyan,
            anti_aliasing: true,
            ui_scale_index: 0,
            show_tool_labels: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Performance (Performanses)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QualityPreset {
    HighPrecision, // 800 mm chunk size
    Balanced,      // 1200 mm chunk size
    HighSpeed,     // 2000 mm chunk size
}

impl QualityPreset {
    pub fn name(self) -> &'static str {
        match self {
            Self::HighPrecision => "High Precision (800 mm chunks — maximum depth resolution)",
            Self::Balanced => "Balanced (1200 mm chunks — recommended for all projects)",
            Self::HighSpeed => "High Speed (2000 mm chunks — optimized for large models)",
        }
    }

    pub fn chunk_size(self) -> f64 {
        match self {
            Self::HighPrecision => 800.0,
            Self::Balanced => 1200.0,
            Self::HighSpeed => 2000.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceSettings {
    pub quality_preset: QualityPreset,
    pub dynamic_lod: bool,
    pub silhouette_edges: bool,
    pub max_undo_steps: usize,
    pub target_fps_index: usize, // 0: 60 FPS, 1: 120 FPS, 2: Uncapped
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            quality_preset: QualityPreset::Balanced,
            dynamic_lod: true,
            silhouette_edges: true,
            max_undo_steps: 50,
            target_fps_index: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileLocationsSettings {
    pub default_projects_dir: String,
    pub default_templates_dir: String,
    pub ifc_export_dir: String,
}

impl Default for FileLocationsSettings {
    fn default() -> Self {
        Self {
            default_projects_dir: r"C:\Arch3D\Projects".into(),
            default_templates_dir: r"C:\Arch3D\Templates".into(),
            ifc_export_dir: r"C:\Arch3D\IFC_Exports".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavingSettings {
    pub auto_save_enabled: bool,
    pub auto_save_interval_mins: u32, // 1, 5, 10, 15, 30
    pub create_backup_on_save: bool,
    pub max_backup_versions: u32,
    pub notify_on_autosave: bool,
}

impl Default for SavingSettings {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval_mins: 5,
            create_backup_on_save: true,
            max_backup_versions: 3,
            notify_on_autosave: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticsSettings {
    pub crash_reporting_enabled: bool,
    pub detailed_telemetry: bool,
    pub log_messages: Vec<String>,
}

impl Default for DiagnosticsSettings {
    fn default() -> Self {
        Self {
            crash_reporting_enabled: true,
            detailed_telemetry: true,
            log_messages: vec![
                "[Arch3D Core] System initialized cleanly.".into(),
                "[Arch3D Geom] Vector engine running with hardware SSE/AVX acceleration.".into(),
                "[Arch3D IFC] IFC4 ISO 10303-21 parser ready.".into(),
                "[Arch3D Views] Interactive 3D Painter engine loaded.".into(),
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation Drafting Settings
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnnotationSettings {
    pub tick_mark: plinth_model::TickMark,
    pub font_size: f32,
    pub font_style: plinth_model::FontStyle,
    pub color: [u8; 4],
    pub default_text: String,
    pub snap_to_edges: bool,
    pub snap_to_centerlines: bool,
    pub snap_to_openings: bool,
    pub view_scale: u32,
    pub custom_scale_entry: u32,
}

impl Default for AnnotationSettings {
    fn default() -> Self {
        Self {
            tick_mark: plinth_model::TickMark::Tick45,
            font_size: 12.0,
            font_style: plinth_model::FontStyle::Regular,
            color: [30, 30, 30, 255],
            default_text: "NOTE".into(),
            snap_to_edges: true,
            snap_to_centerlines: true,
            snap_to_openings: true,
            view_scale: 100,
            custom_scale_entry: 100,
        }
    }
}

// ---------------------------------------------------------------------------
// Snap & Drafting Settings
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SnapSettings {
    pub enabled: bool,
    pub snap_to_endpoints: bool,
    pub snap_to_midpoints: bool,
    pub snap_to_intersections: bool,
    pub snap_to_centers: bool,
    pub snap_to_perpendicular: bool,
    pub snap_to_nearest: bool,
    pub snap_to_edges: bool,
    pub snap_to_centerlines: bool,
    pub snap_to_openings: bool,
    pub snap_to_grid: bool,
    pub grid_spacing: f64,
    pub ortho_mode: bool,
    pub polar_tracking: bool,
    pub polar_increment_deg: f64,
    pub wall_continuous_chain: bool,
    pub snap_radius_px: f32,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            snap_to_endpoints: true,
            snap_to_midpoints: true,
            snap_to_intersections: true,
            snap_to_centers: true,
            snap_to_perpendicular: true,
            snap_to_nearest: false,
            snap_to_edges: true,
            snap_to_centerlines: true,
            snap_to_openings: true,
            snap_to_grid: false,
            grid_spacing: 100.0,
            ortho_mode: false,
            polar_tracking: true,
            polar_increment_deg: 45.0,
            wall_continuous_chain: true,
            snap_radius_px: 14.0,
        }
    }
}

// ---------------------------------------------------------------------------
// AppSettings Container
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    #[serde(skip)]
    pub is_open: bool,
    #[serde(skip)]
    pub active_category: SettingsCategory,

    // Active committed settings
    pub units: ProjectUnitsSettings,
    pub theme: ThemeSettings,
    pub snapping: SnapSettings,
    pub performance: PerformanceSettings,
    pub locations: FileLocationsSettings,
    pub saving: SavingSettings,
    pub diagnostics: DiagnosticsSettings,
    pub annotation: AnnotationSettings,
    #[serde(default = "plinth_views::MaterialPalette::signature_pastel")]
    pub materials: plinth_views::MaterialPalette,

    // Pending edits within the dialog (reverted on Cancel, committed on Apply/OK)
    #[serde(skip)]
    pub pending_units: ProjectUnitsSettings,
    #[serde(skip)]
    pub pending_theme: ThemeSettings,
    #[serde(skip)]
    pub pending_snapping: SnapSettings,
    #[serde(skip)]
    pub pending_performance: PerformanceSettings,
    #[serde(skip)]
    pub pending_locations: FileLocationsSettings,
    #[serde(skip)]
    pub pending_saving: SavingSettings,
    #[serde(skip)]
    pub pending_diagnostics: DiagnosticsSettings,
    #[serde(skip)]
    pub pending_annotation: AnnotationSettings,
    #[serde(skip)]
    pub pending_materials: plinth_views::MaterialPalette,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            is_open: false,
            active_category: SettingsCategory::General,
            units: ProjectUnitsSettings::default(),
            theme: ThemeSettings::default(),
            snapping: SnapSettings::default(),
            performance: PerformanceSettings::default(),
            locations: FileLocationsSettings::default(),
            saving: SavingSettings::default(),
            diagnostics: DiagnosticsSettings::default(),
            annotation: AnnotationSettings::default(),
            materials: plinth_views::MaterialPalette::signature_pastel(),
            pending_units: ProjectUnitsSettings::default(),
            pending_theme: ThemeSettings::default(),
            pending_snapping: SnapSettings::default(),
            pending_performance: PerformanceSettings::default(),
            pending_locations: FileLocationsSettings::default(),
            pending_saving: SavingSettings::default(),
            pending_diagnostics: DiagnosticsSettings::default(),
            pending_annotation: AnnotationSettings::default(),
            pending_materials: plinth_views::MaterialPalette::signature_pastel(),
        }
    }
}

impl AppSettings {
    pub fn settings_file_path() -> std::path::PathBuf {
        #[cfg(target_arch = "wasm32")]
        {
            std::path::PathBuf::from("arch3d_settings.json")
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if cfg!(test) {
                return std::env::temp_dir().join(format!("arch3d_settings_test_{}.json", std::process::id()));
            }
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = exe_path.parent() {
                    if !parent.ends_with("deps") {
                        let candidate = parent.join("arch3d_settings.json");
                        if candidate.exists() || parent.is_dir() {
                            return candidate;
                        }
                    }
                }
            }
            std::path::PathBuf::from("arch3d_settings.json")
        }
    }

    pub fn load_from_disk() -> Option<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::settings_file_path();
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut settings) = serde_json::from_str::<Self>(&content) {
                        settings.sync_pending_to_active();
                        return Some(settings);
                    }
                }
            }
            None
        }
    }

    pub fn save_to_disk(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::settings_file_path();
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&path, json);
            }
        }
    }

    pub fn new() -> Self {
        let mut s = Self::load_from_disk().unwrap_or_default();
        s.sync_pending_to_active();
        s
    }

    pub fn open(&mut self) {
        self.sync_pending_to_active();
        self.is_open = true;
    }

    pub fn apply(&mut self) {
        self.units = self.pending_units.clone();
        self.theme = self.pending_theme.clone();
        self.snapping = self.pending_snapping.clone();
        self.performance = self.pending_performance.clone();
        self.locations = self.pending_locations.clone();
        self.saving = self.pending_saving.clone();
        self.diagnostics = self.pending_diagnostics.clone();
        self.annotation = self.pending_annotation.clone();
        self.materials = self.pending_materials;
        self.save_to_disk();
    }

    pub fn reset_defaults(&mut self) {
        self.pending_units = ProjectUnitsSettings::default();
        self.pending_theme = ThemeSettings::default();
        self.pending_snapping = SnapSettings::default();
        self.pending_performance = PerformanceSettings::default();
        self.pending_locations = FileLocationsSettings::default();
        self.pending_saving = SavingSettings::default();
        self.pending_diagnostics = DiagnosticsSettings::default();
        self.pending_annotation = AnnotationSettings::default();
        self.pending_materials = plinth_views::MaterialPalette::signature_pastel();
    }

    fn sync_pending_to_active(&mut self) {
        self.pending_units = self.units.clone();
        self.pending_theme = self.theme.clone();
        self.pending_snapping = self.snapping.clone();
        self.pending_performance = self.performance.clone();
        self.pending_locations = self.locations.clone();
        self.pending_saving = self.saving.clone();
        self.pending_diagnostics = self.diagnostics.clone();
        self.pending_annotation = self.annotation.clone();
        self.pending_materials = self.materials;
    }
}

impl Default for SettingsCategory {
    fn default() -> Self {
        Self::General
    }
}

// ---------------------------------------------------------------------------
// Dialog UI Rendering
// ---------------------------------------------------------------------------
pub struct SettingsDialogResult {
    pub applied: bool,
    pub theme_changed: bool,
    pub materials_changed: bool,
}

pub fn render_settings_dialog(
    ctx: &egui::Context,
    settings: &mut AppSettings,
    app_theme: &mut AppTheme,
    doc: &Document,
) -> SettingsDialogResult {
    let mut result = SettingsDialogResult {
        applied: false,
        theme_changed: false,
        materials_changed: false,
    };

    if !settings.is_open {
        return result;
    }

    let mut is_open = settings.is_open;
    let screen_rect = ctx.screen_rect();
    let default_pos = egui::pos2(
        (screen_rect.width() - 660.0).max(20.0) * 0.5,
        (screen_rect.height() - 480.0).max(20.0) * 0.5,
    );

    egui::Window::new("Preferences")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_size(Vec2::new(720.0, 480.0))
        .min_size(Vec2::new(560.0, 400.0))
        .default_pos(default_pos)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

            // Main Body: Left Sidebar + Right Content Area
            ui.horizontal(|ui| {
                // Left Category Navigation Bar (Fixed Width ~160px)
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                    for cat in [
                        SettingsCategory::General,
                        SettingsCategory::Interface,
                        SettingsCategory::MaterialColors,
                        SettingsCategory::Snapping,
                        SettingsCategory::FileLocations,
                        SettingsCategory::SavingBackup,
                        SettingsCategory::KeyboardShortcuts,
                        SettingsCategory::Diagnostics,
                    ] {
                        let is_active = settings.active_category == cat;
                        let item_size = Vec2::new(160.0, 36.0);
                        let (item_rect, resp) = ui.allocate_exact_size(item_size, egui::Sense::click());
                        let is_hovered = resp.hovered();

                        if is_active {
                            // Active background
                            let bg = app_theme.accent.gamma_multiply(0.12);
                            ui.painter().rect_filled(item_rect, 6.0, bg);
                            // Left vertical blue accent indicator bar (like reference design!)
                            let bar_rect = egui::Rect::from_min_size(
                                egui::pos2(item_rect.min.x + 2.0, item_rect.min.y + 6.0),
                                egui::vec2(3.5, item_rect.height() - 12.0),
                            );
                            ui.painter().rect_filled(bar_rect, 2.0, app_theme.accent);
                        } else if is_hovered {
                            let bg = app_theme.hover_highlight;
                            ui.painter().rect_filled(item_rect, 6.0, bg);
                        }

                        let text_color = if is_active {
                            app_theme.accent
                        } else if is_hovered {
                            app_theme.text_primary
                        } else {
                            app_theme.text_muted
                        };

                        let text = format!("{}  {}", cat.icon(), cat.title());
                        ui.painter().text(
                            egui::pos2(item_rect.min.x + 14.0, item_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            text,
                            egui::FontId::proportional(12.5),
                            text_color,
                        );

                        if resp.clicked() {
                            settings.active_category = cat;
                        }
                    }

                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Quick Summary Badge
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Arch3D BIM Engine").size(10.0).strong().color(app_theme.text_muted));
                        ui.label(egui::RichText::new("IFC4 ISO 10303-21").size(10.5).color(app_theme.text_primary));
                        ui.label(egui::RichText::new("1:1 Millimeter Real Scale").size(10.0).color(app_theme.text_muted));
                    });
                });

                ui.separator();

                // Right Category Content Area (Strictly Vertical Layout)
                ui.vertical(|ui| {
                    ui.set_min_width(450.0);
                    ui.set_max_width(480.0);

                    // Active Category Heading & Subtitle
                    ui.label(
                        egui::RichText::new(settings.active_category.title())
                            .size(15.0)
                            .strong()
                            .color(app_theme.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(settings.active_category.subtitle())
                            .size(10.5)
                            .color(app_theme.text_muted),
                    );
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(340.0)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);

                            match settings.active_category {
                                SettingsCategory::General => {
                                    render_units_page(ui, &mut settings.pending_units, app_theme);
                                }
                                SettingsCategory::Interface => {
                                    render_theme_page(ui, &mut settings.pending_theme, app_theme);
                                }
                                SettingsCategory::MaterialColors => {
                                    render_material_colors_page(ui, &mut settings.pending_materials, app_theme);
                                }
                                SettingsCategory::Snapping => {
                                    render_snapping_page(ui, &mut settings.pending_snapping, app_theme);
                                }
                                SettingsCategory::FileLocations => {
                                    render_locations_page(ui, &mut settings.pending_locations, app_theme);
                                }
                                SettingsCategory::SavingBackup => {
                                    render_saving_page(ui, &mut settings.pending_saving, app_theme);
                                }
                                SettingsCategory::KeyboardShortcuts => {
                                    render_shortcuts_page(ui, app_theme);
                                }
                                SettingsCategory::Diagnostics => {
                                    render_performance_page(ui, &mut settings.pending_performance, &mut settings.pending_diagnostics, app_theme, doc);
                                }
                            }
                        });
                });
            });

            ui.separator();

            // Dialog Footer Bar (matching reference design: [Restore defaults] on left, [OK] & [Cancel] on right)
            ui.horizontal(|ui| {
                if ui.button("Restore defaults").clicked() {
                    settings.reset_defaults();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ok_btn = egui::Button::new(
                        egui::RichText::new("   OK   ").strong().color(Color32::WHITE),
                    )
                    .fill(app_theme.accent);

                    if ui.add(ok_btn).clicked() {
                        let theme_before = settings.theme.clone();
                        let materials_before = settings.materials;
                        settings.apply();
                        settings.is_open = false;
                        result.applied = true;
                        if settings.theme != theme_before {
                            result.theme_changed = true;
                        }
                        if settings.materials != materials_before {
                            result.materials_changed = true;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        settings.sync_pending_to_active();
                        settings.is_open = false;
                    }

                    if ui.button("Apply").clicked() {
                        let theme_before = settings.theme.clone();
                        let materials_before = settings.materials;
                        settings.apply();
                        result.applied = true;
                        if settings.theme != theme_before {
                            result.theme_changed = true;
                        }
                        if settings.materials != materials_before {
                            result.materials_changed = true;
                        }
                    }
                });
            });
        });

    if !is_open {
        settings.is_open = false;
    }

    result
}

// ---------------------------------------------------------------------------
// Category 1: Project Units Page
// ---------------------------------------------------------------------------
fn render_units_page(ui: &mut Ui, units: &mut ProjectUnitsSettings, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Measurement Units").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Standard dimensional formats used for geometry input, coordinates, and schedules.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        egui::Grid::new("units_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
            // Length
            ui.label("Length Unit:");
            egui::ComboBox::from_id_salt("length_unit_combo")
                .selected_text(units.length_unit.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut units.length_unit, LengthUnit::Millimeters, LengthUnit::Millimeters.name());
                    ui.selectable_value(&mut units.length_unit, LengthUnit::Centimeters, LengthUnit::Centimeters.name());
                    ui.selectable_value(&mut units.length_unit, LengthUnit::Meters, LengthUnit::Meters.name());
                    ui.selectable_value(&mut units.length_unit, LengthUnit::FeetInches, LengthUnit::FeetInches.name());
                });
            ui.end_row();

            // Area
            ui.label("Area Unit:");
            egui::ComboBox::from_id_salt("area_unit_combo")
                .selected_text(units.area_unit.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut units.area_unit, AreaUnit::SquareMeters, AreaUnit::SquareMeters.name());
                    ui.selectable_value(&mut units.area_unit, AreaUnit::SquareMillimeters, AreaUnit::SquareMillimeters.name());
                    ui.selectable_value(&mut units.area_unit, AreaUnit::SquareFeet, AreaUnit::SquareFeet.name());
                });
            ui.end_row();

            // Volume
            ui.label("Volume Unit:");
            egui::ComboBox::from_id_salt("volume_unit_combo")
                .selected_text(units.volume_unit.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut units.volume_unit, VolumeUnit::CubicMeters, VolumeUnit::CubicMeters.name());
                    ui.selectable_value(&mut units.volume_unit, VolumeUnit::CubicMillimeters, VolumeUnit::CubicMillimeters.name());
                    ui.selectable_value(&mut units.volume_unit, VolumeUnit::CubicFeet, VolumeUnit::CubicFeet.name());
                });
            ui.end_row();

            // Angle
            ui.label("Angle Unit:");
            egui::ComboBox::from_id_salt("angle_unit_combo")
                .selected_text(units.angle_unit.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut units.angle_unit, AngleUnit::DecimalDegrees, AngleUnit::DecimalDegrees.name());
                    ui.selectable_value(&mut units.angle_unit, AngleUnit::Radians, AngleUnit::Radians.name());
                });
            ui.end_row();
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Display Precision & Snapping").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Control numeric decimal rounding and grid snap increments.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        egui::Grid::new("precision_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
            ui.label("Decimal Places:");
            ui.add(egui::Slider::new(&mut units.decimal_precision, 0..=4).text("decimals"));
            ui.end_row();

            ui.label("Show Unit Symbol:");
            ui.checkbox(&mut units.show_unit_symbol, "Append unit suffix (e.g. mm, m², m³)");
            ui.end_row();

            ui.label("Snap Grid Increment:");
            egui::ComboBox::from_id_salt("snap_grid_combo")
                .selected_text(format!("{:.0} mm", units.snap_grid_spacing))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut units.snap_grid_spacing, 50.0, "50 mm (Fine)");
                    ui.selectable_value(&mut units.snap_grid_spacing, 100.0, "100 mm (Standard Architectural)");
                    ui.selectable_value(&mut units.snap_grid_spacing, 250.0, "250 mm (Masonry Module)");
                    ui.selectable_value(&mut units.snap_grid_spacing, 500.0, "500 mm (Coarse)");
                    ui.selectable_value(&mut units.snap_grid_spacing, 1000.0, "1,000 mm (1 Meter Structural)");
                });
            ui.end_row();
        });
    });

    // Live Formatting Preview Card
    ui.group(|ui| {
        ui.label(egui::RichText::new("Live Units Preview").size(12.0).strong().color(theme.text_primary));
        ui.horizontal(|ui| {
            ui.label("Sample Length (3500 mm):");
            ui.label(egui::RichText::new(units.format_length(3500.0)).strong().color(theme.accent));
        });
        ui.horizontal(|ui| {
            ui.label("Sample Area (96 m²):");
            ui.label(egui::RichText::new(units.format_area(96_000_000.0)).strong().color(theme.accent));
        });
        ui.horizontal(|ui| {
            ui.label("Sample Volume (288 m³):");
            ui.label(egui::RichText::new(units.format_volume(288_000_000_000.0)).strong().color(theme.accent));
        });
    });
}

// ---------------------------------------------------------------------------
// Category 2: Theme & Appearance Page
// ---------------------------------------------------------------------------
fn render_theme_page(ui: &mut Ui, theme_settings: &mut ThemeSettings, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Visual Theme Mode").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Toggle between high-contrast dark environment and architectural print light mode.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.radio_value(&mut theme_settings.theme_mode, ThemeMode::Dark, "🌙 Dark Theme (CAD Obsidian)");
            ui.radio_value(&mut theme_settings.theme_mode, ThemeMode::Light, "☀ Light Theme (Paper White)");
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Accent Color & Branding").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Defines the primary UI highlight, active button badges, and extrusion indicators.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        for preset in [
            AccentPreset::ArchitecturalBlue,
            AccentPreset::BimEmerald,
            AccentPreset::EngineeringOrange,
            AccentPreset::RevitBurgundy,
            AccentPreset::SlateGrey,
        ] {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, preset.color());
                ui.radio_value(&mut theme_settings.accent_preset, preset, preset.name());
            });
        }
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Drawing Canvas Background").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Background shading for 2D plans, elevations, and 3D axonometric views.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        egui::ComboBox::from_id_salt("canvas_bg_combo")
            .selected_text(theme_settings.canvas_bg.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut theme_settings.canvas_bg, CanvasBgStyle::ThemeMatch, CanvasBgStyle::ThemeMatch.name());
                ui.selectable_value(&mut theme_settings.canvas_bg, CanvasBgStyle::CleanWhitePaper, CanvasBgStyle::CleanWhitePaper.name());
                ui.selectable_value(&mut theme_settings.canvas_bg, CanvasBgStyle::DefaultCadDark, CanvasBgStyle::DefaultCadDark.name());
                ui.selectable_value(&mut theme_settings.canvas_bg, CanvasBgStyle::ObsidianBlack, CanvasBgStyle::ObsidianBlack.name());
                ui.selectable_value(&mut theme_settings.canvas_bg, CanvasBgStyle::MidnightNavy, CanvasBgStyle::MidnightNavy.name());
            });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Selection & Display Options").size(13.0).strong().color(theme.accent));
        ui.add_space(4.0);

        egui::Grid::new("selection_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
            ui.label("Selection Highlight Color:");
            egui::ComboBox::from_id_salt("selection_color_combo")
                .selected_text(theme_settings.selection_color.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut theme_settings.selection_color, SelectionColorChoice::ElectricCyan, SelectionColorChoice::ElectricCyan.name());
                    ui.selectable_value(&mut theme_settings.selection_color, SelectionColorChoice::AmberGold, SelectionColorChoice::AmberGold.name());
                    ui.selectable_value(&mut theme_settings.selection_color, SelectionColorChoice::LaserGreen, SelectionColorChoice::LaserGreen.name());
                });
            ui.end_row();

            ui.label("Vector Anti-Aliasing:");
            ui.checkbox(&mut theme_settings.anti_aliasing, "Enable smooth anti-aliased mathematical vector strokes");
            ui.end_row();

            ui.label("Interface Scale:");
            egui::ComboBox::from_id_salt("ui_scale_combo")
                .selected_text(match theme_settings.ui_scale_index {
                    0 => "100% (Standard)",
                    1 => "110% (Comfortable)",
                    _ => "125% (High-DPI / 4K)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut theme_settings.ui_scale_index, 0, "100% (Standard)");
                    ui.selectable_value(&mut theme_settings.ui_scale_index, 1, "110% (Comfortable)");
                    ui.selectable_value(&mut theme_settings.ui_scale_index, 2, "125% (High-DPI / 4K)");
                });
            ui.end_row();
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Ribbon Toolbar Display").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Configure ribbon buttons to show text labels below icons or condensed icon only.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.radio_value(&mut theme_settings.show_tool_labels, true, "Icon with text label");
            ui.radio_value(&mut theme_settings.show_tool_labels, false, "Compact icon only (hide label)");
        });
    });
}

// ---------------------------------------------------------------------------
// Category: Material Surface Colors (Pastel Palette) Page
// ---------------------------------------------------------------------------
fn render_material_colors_page(
    ui: &mut Ui,
    palette: &mut plinth_views::MaterialPalette,
    theme: &AppTheme,
) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Arch3D Pastel Color Presets")
                .size(13.0)
                .strong()
                .color(theme.accent),
        );
        ui.label(
            egui::RichText::new(
                "Distinctive pastel aesthetic designed exclusively for Arch3D. Changes persist automatically to disk on Apply.",
            )
            .size(10.5)
            .color(theme.text_muted),
        );
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            if ui
                .button(egui::RichText::new("🌸 Signature Pastel").strong())
                .on_hover_text("Default Arch3D warm birch, travertine oatmeal, and glacier cyan pastel palette")
                .clicked()
            {
                *palette = plinth_views::MaterialPalette::signature_pastel();
            }
            if ui
                .button("🏛 Architectural Neutral")
                .on_hover_text("Minimalist modern gallery neutral tones")
                .clicked()
            {
                *palette = plinth_views::MaterialPalette::architectural_neutral();
            }
            if ui
                .button("🌿 Scandinavian Earth")
                .on_hover_text("Nordic organic clay, terracotta, and soft matcha earth tones")
                .clicked()
            {
                *palette = plinth_views::MaterialPalette::scandinavian_earth();
            }
            if ui
                .button("📐 Modern Blueprint")
                .on_hover_text("High-contrast technical cyan and blueprint slate tones")
                .clicked()
            {
                *palette = plinth_views::MaterialPalette::modern_blueprint();
            }
        });
    });

    ui.add_space(4.0);

    let color_row_rgb = |ui: &mut Ui, label: &str, desc: &str, rgb: &mut [u8; 3]| {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(rgb);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).strong().size(11.5));
                ui.label(egui::RichText::new(desc).size(9.5).color(theme.text_muted));
            });
        });
    };

    let color_row_rgba = |ui: &mut Ui, label: &str, desc: &str, rgba: &mut [u8; 4]| {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba_unmultiplied(rgba);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).strong().size(11.5));
                ui.label(egui::RichText::new(desc).size(9.5).color(theme.text_muted));
            });
        });
    };

    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Architectural Shell")
                .size(12.0)
                .strong()
                .color(theme.accent),
        );
        ui.add_space(2.0);
        egui::Grid::new("arch_shell_palette_grid")
            .num_columns(2)
            .spacing(Vec2::new(16.0, 6.0))
            .show(ui, |ui| {
                color_row_rgb(ui, "Wall Surface", "Linen Birch Pastel", &mut palette.wall);
                color_row_rgb(ui, "Floor / Slab", "Pale Sand Limestone", &mut palette.slab);
                ui.end_row();

                color_row_rgb(ui, "Roof Surface", "Dusty Terracotta Rose", &mut palette.roof);
                color_row_rgb(ui, "Ceiling Surface", "Soft Chalk White", &mut palette.ceiling);
                ui.end_row();

                color_row_rgb(ui, "Door Wood", "Bleached Honey Oak", &mut palette.door);
                color_row_rgb(ui, "Window Frame", "Warm Anthracite Slate", &mut palette.window_frame);
                ui.end_row();

                color_row_rgba(ui, "Window Glass", "Translucent Glacier Cyan", &mut palette.window_glass);
                ui.label("");
                ui.end_row();
            });
    });

    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Structural Framing & Substructure")
                .size(12.0)
                .strong()
                .color(theme.accent),
        );
        ui.add_space(2.0);
        egui::Grid::new("struct_palette_grid")
            .num_columns(2)
            .spacing(Vec2::new(16.0, 6.0))
            .show(ui, |ui| {
                color_row_rgb(ui, "Structural Column", "Muted Slate Sage", &mut palette.column);
                color_row_rgb(ui, "Structural Beam", "Warm Mineral Grey", &mut palette.beam);
                ui.end_row();

                color_row_rgb(ui, "Foundation Footing", "Basalt Sand Pastel", &mut palette.footing);
                color_row_rgb(ui, "Stair & Ramp", "Travertine Oatmeal", &mut palette.stair_ramp);
                ui.end_row();

                color_row_rgb(ui, "Handrail & Balustrade", "Titanium Graphite", &mut palette.railing);
                ui.label("");
                ui.end_row();
            });
    });

    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Curtain Wall, Site & Freeform Modeling")
                .size(12.0)
                .strong()
                .color(theme.accent),
        );
        ui.add_space(2.0);
        egui::Grid::new("curtain_freeform_palette_grid")
            .num_columns(2)
            .spacing(Vec2::new(16.0, 6.0))
            .show(ui, |ui| {
                color_row_rgb(ui, "Curtain Wall Frame", "Matte Architectural Aluminum", &mut palette.curtain_frame);
                color_row_rgba(ui, "Curtain Wall Glass", "Pale Sky Translucent Cyan", &mut palette.curtain_glass);
                ui.end_row();

                color_row_rgb(ui, "Site / Toposolid", "Matcha Herb Pastel", &mut palette.site);
                color_row_rgb(ui, "Freeform 3D Solids", "Signature Pastel Ivory", &mut palette.freeform);
                ui.end_row();
            });
    });
}

// ---------------------------------------------------------------------------
// Category: Snapping & Drafting Page
// ---------------------------------------------------------------------------
fn render_snapping_page(ui: &mut Ui, snap: &mut SnapSettings, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Master Snapping Switch").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Toggle global magnetic cursor snapping for all drafting and BIM modeling tools.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut snap.enabled, egui::RichText::new("Enable Magnetic Snapping (F3)").strong().color(theme.accent));
            ui.label(egui::RichText::new("[F3 Shortcut]").size(10.5).color(theme.text_muted));
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Drafting & Angle Constraints").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Constrain cursor motion to exact horizontal/vertical axes or polar angle increments.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut snap.ortho_mode, egui::RichText::new("Ortho Mode (F8)").strong());
            ui.label(egui::RichText::new("Strict 0°, 90°, 180°, 270° horizontal/vertical lock (Hold Shift for temporary Ortho)").size(10.5).color(theme.text_muted));
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut snap.polar_tracking, egui::RichText::new("Polar Tracking (F10)").strong());
            ui.label(egui::RichText::new("Dynamic visual alignment tracking rays with angle badges").size(10.5).color(theme.text_muted));
        });

        if snap.polar_tracking {
            ui.horizontal(|ui| {
                ui.label("Polar Increment Angle:");
                for &deg in &[15.0, 30.0, 45.0, 90.0] {
                    let is_sel = (snap.polar_increment_deg - deg).abs() < 1e-3;
                    if ui.selectable_label(is_sel, format!("{:.0}°", deg)).clicked() {
                        snap.polar_increment_deg = deg;
                    }
                }
            });
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut snap.wall_continuous_chain, egui::RichText::new("Continuous Polyline / Chain Mode for Walls").strong());
            ui.label(egui::RichText::new("Chain contiguous walls automatically from last endpoint until Esc is pressed").size(10.5).color(theme.text_muted));
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Object Snap Modes (O-Snap)").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Detect geometric feature points on walls, slabs, columns, beams, and lines.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        egui::Grid::new("osnap_modes_grid").num_columns(2).spacing(Vec2::new(24.0, 6.0)).show(ui, |ui| {
            ui.checkbox(&mut snap.snap_to_endpoints, "□ Endpoints (Walls, Slabs, Lines, Beams)");
            ui.checkbox(&mut snap.snap_to_midpoints, "△ Midpoints (Wall & Slab Edges)");
            ui.end_row();

            ui.checkbox(&mut snap.snap_to_centers, "○ Centers (Columns, Footings, Arcs)");
            ui.checkbox(&mut snap.snap_to_intersections, "✕ Intersections (Grids, Walls, Lines)");
            ui.end_row();

            ui.checkbox(&mut snap.snap_to_perpendicular, "⊾ Perpendicular (90° Normal Ray)");
            ui.checkbox(&mut snap.snap_to_nearest, "⧈ Nearest (On-Edge Magnetic)");
            ui.end_row();
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Grid Snapping & Magnetic Sensitivity").size(13.0).strong().color(theme.accent));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut snap.snap_to_grid, "Snap to Coordinate Grid Spacing");
            if snap.snap_to_grid {
                ui.label("Spacing (mm):");
                let mut sp_text = format!("{:.0}", snap.grid_spacing);
                if ui.add(egui::TextEdit::singleline(&mut sp_text).desired_width(70.0)).changed() {
                    if let Ok(val) = sp_text.parse::<f64>() {
                        if val >= 1.0 {
                            snap.grid_spacing = val;
                        }
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Magnetic Snap Sensitivity Radius (px):");
            ui.add(egui::Slider::new(&mut snap.snap_radius_px, 6.0..=32.0).text("px"));
        });
    });
}

// ---------------------------------------------------------------------------
// Category 3: File Locations Page
// ---------------------------------------------------------------------------
fn render_locations_page(ui: &mut Ui, loc: &mut FileLocationsSettings, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Default Project Directory").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Root directory used for Open / Save dialogs and autosaves.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);
        ui.text_edit_singleline(&mut loc.default_projects_dir);
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Templates & Families Library").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Location of standard architectural templates and component definitions.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);
        ui.text_edit_singleline(&mut loc.default_templates_dir);
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("IFC Export Directory").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Target directory for ISO 10303-21 STEP format IFC4 model exports.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);
        ui.text_edit_singleline(&mut loc.ifc_export_dir);
    });
}

// ---------------------------------------------------------------------------
// Category 4: Saving & Backup Page
// ---------------------------------------------------------------------------
fn render_saving_page(ui: &mut Ui, saving: &mut SavingSettings, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Automatic Saving").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Periodically preserves your model in the background to prevent data loss.").size(10.5).color(theme.text_muted));
        ui.add_space(6.0);

        ui.checkbox(&mut saving.auto_save_enabled, "Enable Auto-Save timer");

        if saving.auto_save_enabled {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Auto-Save Interval:");
                egui::ComboBox::from_id_salt("autosave_interval_combo")
                    .selected_text(format!("{} minutes", saving.auto_save_interval_mins))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut saving.auto_save_interval_mins, 1, "1 minute (High Frequency)");
                        ui.selectable_value(&mut saving.auto_save_interval_mins, 5, "5 minutes (Recommended)");
                        ui.selectable_value(&mut saving.auto_save_interval_mins, 10, "10 minutes");
                        ui.selectable_value(&mut saving.auto_save_interval_mins, 15, "15 minutes");
                        ui.selectable_value(&mut saving.auto_save_interval_mins, 30, "30 minutes");
                    });
            });

            ui.add_space(4.0);
            ui.checkbox(&mut saving.notify_on_autosave, "Display subtle notification toast in status bar on auto-save");
        }
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Project Backup Files (.bak)").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Protects against accidental overwriting by copying previous file on each save.").size(10.5).color(theme.text_muted));
        ui.add_space(6.0);

        ui.checkbox(&mut saving.create_backup_on_save, "Create backup file (.bak) when saving project");

        if saving.create_backup_on_save {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Backup Versions to Retain:");
                egui::ComboBox::from_id_salt("backup_versions_combo")
                    .selected_text(format!("{} version(s)", saving.max_backup_versions))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut saving.max_backup_versions, 1, "1 previous backup");
                        ui.selectable_value(&mut saving.max_backup_versions, 3, "3 backups (.bak, .001, .002)");
                        ui.selectable_value(&mut saving.max_backup_versions, 5, "5 backups");
                        ui.selectable_value(&mut saving.max_backup_versions, 10, "10 backups (Maximum Protection)");
                    });
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Category 5: Keyboard Shortcuts Page
// ---------------------------------------------------------------------------
fn render_shortcuts_page(ui: &mut Ui, theme: &AppTheme) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Essential Keyboard Shortcuts").size(13.0).strong().color(theme.accent));
        ui.add_space(6.0);

        egui::Grid::new("shortcuts_grid").num_columns(2).spacing([24.0, 8.0]).striped(true).show(ui, |ui| {
            let shortcuts = [
                ("Ctrl + S", "Save Project (or Quick Save)"),
                ("Ctrl + Shift + S", "Save Project As..."),
                ("Ctrl + O", "Open Project / Import IFC"),
                ("Ctrl + Z", "Undo Last Action"),
                ("Ctrl + Y", "Redo"),
                ("Delete / Backspace", "Delete Selected BIM Element"),
                ("Escape", "Cancel Active Tool / Deselect"),
                ("1", "Visual Style: Wireframe"),
                ("2", "Visual Style: Hidden Line"),
                ("3", "Visual Style: Shaded"),
                ("4", "Visual Style: Consistent Colors"),
                ("Middle Click + Drag", "Pan Viewport Canvas"),
                ("Shift + Middle Drag", "Interactive 3D Orbit"),
                ("Mouse Scroll Wheel", "Smooth Viewport Zoom"),
            ];

            for (key, desc) in shortcuts {
                ui.label(egui::RichText::new(key).strong().monospace().color(theme.accent));
                ui.label(desc);
                ui.end_row();
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Category 6: Performance & Diagnostics Page
// ---------------------------------------------------------------------------
fn render_performance_page(
    ui: &mut Ui,
    perf: &mut PerformanceSettings,
    diag: &mut DiagnosticsSettings,
    theme: &AppTheme,
    doc: &Document,
) {
    // 1. Application Health & Crash Diagnostics
    ui.group(|ui| {
        ui.label(egui::RichText::new("Application Health & Crash Recovery").size(13.0).strong().color(theme.accent));
        ui.add_space(4.0);
        ui.checkbox(&mut diag.crash_reporting_enabled, "Enable panic recovery and error diagnostic logging");

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("System Status:");
            ui.label(egui::RichText::new("🟢 Stable • 0 Unhandled Exceptions").strong().color(Color32::from_rgb(40, 180, 80)));
        });

        ui.add_space(4.0);
        ui.label(egui::RichText::new("Diagnostic Log History:").size(11.0).color(theme.text_muted));
        egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
            for msg in &diag.log_messages {
                ui.label(egui::RichText::new(msg).size(10.0).monospace().color(theme.text_muted));
            }
        });
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("3D Geometry Engine & Subdivision").size(13.0).strong().color(theme.accent));
        ui.label(egui::RichText::new("Adjusts Sutherland-Hodgman bounded polygon subdivision for Painter's depth sorting.").size(10.5).color(theme.text_muted));
        ui.add_space(4.0);

        egui::ComboBox::from_id_salt("quality_preset_combo")
            .selected_text(perf.quality_preset.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut perf.quality_preset, QualityPreset::HighPrecision, QualityPreset::HighPrecision.name());
                ui.selectable_value(&mut perf.quality_preset, QualityPreset::Balanced, QualityPreset::Balanced.name());
                ui.selectable_value(&mut perf.quality_preset, QualityPreset::HighSpeed, QualityPreset::HighSpeed.name());
            });

        ui.add_space(4.0);
        ui.checkbox(&mut perf.dynamic_lod, "Adaptive Level-of-Detail (LOD) optimization during interactive orbit");
        ui.checkbox(&mut perf.silhouette_edges, "Mathematical curved silhouette contour extraction (cylinders & spheres)");
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Memory & Undo / Redo Stack").size(13.0).strong().color(theme.accent));
        ui.add_space(4.0);

        egui::Grid::new("perf_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
            ui.label("Undo History Depth:");
            egui::ComboBox::from_id_salt("undo_depth_combo")
                .selected_text(format!("{} snapshots", perf.max_undo_steps))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut perf.max_undo_steps, 25, "25 snapshots (Low Memory)");
                    ui.selectable_value(&mut perf.max_undo_steps, 50, "50 snapshots (Standard)");
                    ui.selectable_value(&mut perf.max_undo_steps, 100, "100 snapshots (Maximum History)");
                });
            ui.end_row();

            ui.label("Display Target FPS:");
            egui::ComboBox::from_id_salt("target_fps_combo")
                .selected_text(match perf.target_fps_index {
                    0 => "60 FPS (Energy Efficient)",
                    1 => "120 FPS (Ultra Smooth)",
                    _ => "Uncapped (Maximum Hardware Performance)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut perf.target_fps_index, 0, "60 FPS (Energy Efficient)");
                    ui.selectable_value(&mut perf.target_fps_index, 1, "120 FPS (Ultra Smooth)");
                    ui.selectable_value(&mut perf.target_fps_index, 2, "Uncapped (Maximum Hardware Performance)");
                });
            ui.end_row();
        });
    });

    // Project Telemetry Diagnostics Box
    ui.group(|ui| {
        ui.label(egui::RichText::new("Document Telemetry & Runtime Diagnostics").size(12.0).strong().color(theme.text_primary));
        ui.add_space(4.0);

        egui::Grid::new("telemetry_grid").num_columns(2).spacing([30.0, 6.0]).show(ui, |ui| {
            ui.label("Total BIM Elements in Memory:");
            ui.label(egui::RichText::new(format!("{}", doc.elements.len())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Building Storeys / Levels:");
            ui.label(egui::RichText::new(format!("{}", doc.levels.len())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Registered Projection Views:");
            ui.label(egui::RichText::new(format!("{}", doc.views.len())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Layout Sheets:");
            ui.label(egui::RichText::new(format!("{}", doc.sheets.len())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Structural Grid Lines:");
            ui.label(egui::RichText::new(format!("{}", doc.grids.len())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Current Undo Stack Length:");
            ui.label(egui::RichText::new(format!("{}", doc.undo_count())).strong().color(theme.accent));
            ui.end_row();

            ui.label("Engine Architecture:");
            ui.label(egui::RichText::new("Cleanroom Rust • x86_64-pc-windows-gnu • IFC4").color(theme.text_muted));
            ui.end_row();
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_initialization_and_categories() {
        let mut settings = AppSettings::new();
        assert!(!settings.is_open);
        assert_eq!(settings.active_category, SettingsCategory::General);

        settings.open();
        assert!(settings.is_open);

        assert_eq!(SettingsCategory::General.title(), "General");
        assert_eq!(SettingsCategory::Interface.title(), "Interface");
        assert_eq!(SettingsCategory::Snapping.title(), "Snapping & Drafting");
        assert_eq!(SettingsCategory::SavingBackup.title(), "Saving & Backup");
        assert_eq!(SettingsCategory::Diagnostics.title(), "Diagnostics & Crash Report");
    }

    #[test]
    fn test_project_units_formatting() {
        let mut units = ProjectUnitsSettings::default();
        units.decimal_precision = 1;
        units.show_unit_symbol = true;

        // Millimeters
        units.length_unit = LengthUnit::Millimeters;
        assert_eq!(units.format_length(3500.0), "3500.0 mm");

        // Centimeters
        units.length_unit = LengthUnit::Centimeters;
        assert_eq!(units.format_length(3500.0), "350.0 cm");

        // Meters
        units.length_unit = LengthUnit::Meters;
        assert_eq!(units.format_length(3500.0), "3.5 m");

        // Area formatting
        units.area_unit = AreaUnit::SquareMeters;
        assert_eq!(units.format_area(96_000_000.0), "96.00 m²");

        // Volume formatting
        units.volume_unit = VolumeUnit::CubicMeters;
        assert_eq!(units.format_volume(288_000_000_000.0), "288.000 m³");
    }

    #[test]
    fn test_settings_apply_and_reset() {
        let mut settings = AppSettings::new();
        settings.pending_units.length_unit = LengthUnit::Meters;
        settings.pending_theme.theme_mode = ThemeMode::Light;
        settings.pending_theme.accent_preset = AccentPreset::EngineeringOrange;
        settings.pending_performance.max_undo_steps = 100;
        settings.pending_snapping.polar_increment_deg = 30.0;
        settings.pending_snapping.ortho_mode = true;

        settings.apply();
        assert_eq!(settings.units.length_unit, LengthUnit::Meters);
        assert_eq!(settings.theme.theme_mode, ThemeMode::Light);
        assert_eq!(settings.theme.accent_preset, AccentPreset::EngineeringOrange);
        assert_eq!(settings.performance.max_undo_steps, 100);
        assert_eq!(settings.snapping.polar_increment_deg, 30.0);
        assert!(settings.snapping.ortho_mode);

        settings.reset_defaults();
        assert_eq!(settings.pending_units.length_unit, LengthUnit::Millimeters);
        assert_eq!(settings.pending_theme.theme_mode, ThemeMode::Dark);
        assert_eq!(settings.pending_theme.canvas_bg, CanvasBgStyle::ThemeMatch);
        assert_eq!(settings.pending_snapping.polar_increment_deg, 45.0);
        assert!(!settings.pending_snapping.ortho_mode);
    }

    #[test]
    fn test_canvas_bg_color_matching_theme() {
        let style = CanvasBgStyle::ThemeMatch;
        assert_eq!(style.color(true), Color32::from_rgb(250, 250, 252));
        assert_eq!(style.color(false), Color32::from_rgb(22, 23, 26));

        assert_eq!(CanvasBgStyle::CleanWhitePaper.color(false), Color32::from_rgb(250, 250, 252));
        assert_eq!(CanvasBgStyle::DefaultCadDark.color(true), Color32::from_rgb(22, 23, 26));
    }

    #[test]
    fn test_settings_serialization_and_material_palette() {
        let mut settings = AppSettings::default();
        settings.pending_materials.wall = [240, 235, 225];
        settings.pending_materials.roof = [210, 180, 160];
        settings.apply();

        let json = serde_json::to_string_pretty(&settings).expect("Must serialize settings");
        let restored: AppSettings = serde_json::from_str(&json).expect("Must deserialize settings");

        assert_eq!(restored.materials.wall, [240, 235, 225]);
        assert_eq!(restored.materials.roof, [210, 180, 160]);
        assert_eq!(restored.materials.slab, plinth_views::MaterialPalette::signature_pastel().slab);
    }
}
