//! # Arch3D Plugin Manager Studio
//!
//! Provides modular, optional, and reversible plugin management.
//! Experimental plugins (such as MuTracer WebGPU Path Tracer) can be easily installed,
//! enabled, disabled, or cleanly uninstalled with one click without leaving residual clutter in core Arch3D.

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::icons::{draw_icon, IconKind};
use crate::theme::AppTheme;
use egui::{Align2, Color32, Rect, RichText, ScrollArea, Sense, Stroke, Vec2, Window};
use serde::{Deserialize, Serialize};

/// Embedded cleanroom Python addon script for Blender Live Sync.
pub const BLENDER_ADDON_SCRIPT: &str = include_str!("../../../integrations/blender/arch3d_blender_sync.py");

/// High-level classification for Arch3D plugins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    Rendering,
    Simulation,
    Interoperability,
    Automation,
    Drafting,
}

impl PluginCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rendering => "Rendering & Visualization",
            Self::Simulation => "Physics & Analysis",
            Self::Interoperability => "CAD/BIM Interoperability",
            Self::Automation => "Computational Automation",
            Self::Drafting => "Detailing & Drafting",
        }
    }
}

/// Metadata and runtime state for an individual Arch3D plugin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub category: PluginCategory,
    pub description: String,
    pub is_experimental: bool,
    pub installed: bool,
    pub enabled: bool,
    pub external_url: Option<String>,
    pub live_studio_url: Option<String>,
    pub icon_kind: IconKind,
}

/// Persisted configuration tracking user installation states.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginRegistryState {
    pub installed_plugins: HashMap<String, bool>, // id -> enabled
}

/// Dedicated Plugin Manager Dialog and registry engine.
#[derive(Clone, Debug)]
pub struct PluginManagerDialog {
    pub is_open: bool,
    pub search_query: String,
    pub selected_category: Option<PluginCategory>,
    pub registry: Vec<PluginRecord>,
    pub status_message: String,
}

impl Default for PluginManagerDialog {
    fn default() -> Self {
        let mut mgr = Self {
            is_open: false,
            search_query: String::new(),
            selected_category: None,
            registry: Self::builtin_catalog(),
            status_message: String::new(),
        };
        mgr.load_state_from_disk();
        mgr
    }
}

impl PluginManagerDialog {
    /// Returns the standard curated catalog of Arch3D plugins.
    pub fn builtin_catalog() -> Vec<PluginRecord> {
        vec![
            PluginRecord {
                id: "mutracer-webgpu".into(),
                name: "MuTracer WebGPU Path Tracer".into(),
                version: "0.1.5 (Beta)".into(),
                author: "Meimu Studio / Pissang".into(),
                category: PluginCategory::Rendering,
                description: "Experimental real-time physically based path tracer for modern web and desktop viewports. Powered by WebGPU, OpenPBR surfaces, TinyBVH two-level acceleration, and Intel Open Image Denoise (OIDN). Renders progressive sunlight, environment HDRIs, soft shadows, and transmissive architectural glass with refractions and caustics.".into(),
                is_experimental: true,
                installed: false, // Default: Not installed until user opts in
                enabled: false,
                external_url: Some("https://github.com/pissang/mutracer".into()),
                live_studio_url: Some("render.html".into()),
                icon_kind: IconKind::RenderMuTracer,
            },
            PluginRecord {
                id: "blender-live-sync".into(),
                name: "Blender Live Sync Bridge".into(),
                version: "1.0.0".into(),
                author: "Arch3D Team & Dsales Architect".into(),
                category: PluginCategory::Interoperability,
                description: "Real-time bidirectional synchronization bridge connecting Arch3D BIM directly to Blender 3.6 / 4.x / 4.2+ Cycles and EEVEE via local TCP streaming (port 9876) and temp disk caching. Preserves materials, object hierarchies, and camera setups.".into(),
                is_experimental: false,
                installed: true,
                enabled: true,
                external_url: Some("https://github.com/redwood-ph/arch3d".into()),
                live_studio_url: None,
                icon_kind: IconKind::BlenderLiveSync,
            },
            PluginRecord {
                id: "energy-plus-bep".into(),
                name: "Building Energy Performance (BEP)".into(),
                version: "0.1.0 (Concept)".into(),
                author: "Arch3D Labs".into(),
                category: PluginCategory::Simulation,
                description: "Thermal zone analysis, daylight autonomy simulation, and ASHRAE 90.1 baseline compliance modeling directly from IFC geometry.".into(),
                is_experimental: true,
                installed: false,
                enabled: false,
                external_url: None,
                live_studio_url: None,
                icon_kind: IconKind::Settings,
            },
            PluginRecord {
                id: "structural-fem-solver".into(),
                name: "Structural FEA Frame Solver".into(),
                version: "0.2.1 (Concept)".into(),
                author: "OpenBIM Structural Group".into(),
                category: PluginCategory::Simulation,
                description: "Matrix stiffness linear elastic solver for structural column, beam, and slab load combinations with shear-moment diagrams.".into(),
                is_experimental: true,
                installed: false,
                enabled: false,
                external_url: None,
                live_studio_url: None,
                icon_kind: IconKind::AutoBeamsOnGrids,
            },
        ]
    }

    /// Checks whether a specific plugin is installed and actively enabled.
    pub fn is_plugin_active(&self, id: &str) -> bool {
        self.registry
            .iter()
            .any(|p| p.id == id && p.installed && p.enabled)
    }

    /// Installs a plugin by ID and enables it.
    pub fn install_plugin(&mut self, id: &str) -> bool {
        if let Some(p) = self.registry.iter_mut().find(|p| p.id == id) {
            p.installed = true;
            p.enabled = true;
            self.status_message = format!("Installed and enabled plugin: {}", p.name);
            self.save_state_to_disk();
            true
        } else {
            false
        }
    }

    /// Completely uninstalls a plugin, removing all triggers and active features.
    pub fn uninstall_plugin(&mut self, id: &str) -> bool {
        if let Some(p) = self.registry.iter_mut().find(|p| p.id == id) {
            p.installed = false;
            p.enabled = false;
            self.status_message = format!("Uninstalled plugin: {}", p.name);
            self.save_state_to_disk();
            true
        } else {
            false
        }
    }

    /// Toggles active state of an installed plugin.
    pub fn set_plugin_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(p) = self.registry.iter_mut().find(|p| p.id == id && p.installed) {
            p.enabled = enabled;
            let verb = if enabled { "Enabled" } else { "Disabled" };
            self.status_message = format!("{} plugin: {}", verb, p.name);
            self.save_state_to_disk();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn config_path() -> Option<PathBuf> {
        let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        p.push("arch3d_plugins.json");
        Some(p)
    }

    pub fn save_state_to_disk(&self) {
        let mut state = PluginRegistryState::default();
        for p in &self.registry {
            if p.installed {
                state.installed_plugins.insert(p.id.clone(), p.enabled);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = Self::config_path() {
            if let Ok(json) = serde_json::to_string_pretty(&state) {
                let _ = std::fs::write(path, json);
            }
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(json) = serde_json::to_string(&state) {
                    let _ = storage.set_item("arch3d_plugins", &json);
                }
            }
        }
    }

    pub fn load_state_from_disk(&mut self) {
        let mut loaded: Option<PluginRegistryState> = None;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = Self::config_path() {
            if let Ok(data) = std::fs::read_to_string(path) {
                loaded = serde_json::from_str(&data).ok();
            }
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(data)) = storage.get_item("arch3d_plugins") {
                    loaded = serde_json::from_str(&data).ok();
                }
            }
        }

        if let Some(state) = loaded {
            for (id, enabled) in state.installed_plugins {
                if let Some(p) = self.registry.iter_mut().find(|p| p.id == id) {
                    p.installed = true;
                    p.enabled = enabled;
                }
            }
        }
    }

    /// Renders the Plugin Manager modal window.
    pub fn render(&mut self, ctx: &egui::Context, theme: &AppTheme) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut action_launch_url: Option<String> = None;

        let mut is_open = self.is_open;
        let mut should_close = false;
        Window::new(RichText::new("Arch3D Plugin Manager").strong().size(15.0).color(theme.text_primary))
            .open(&mut is_open)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([760.0, 540.0])
            .min_size([600.0, 420.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

                // Header & Description Banner
                ui.horizontal(|ui| {
                    let icon_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(32.0, 32.0));
                    ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::hover());
                    draw_icon(ui.painter(), icon_rect, IconKind::PluginManager, theme.accent);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Modular Plugins & Experimental Extensions").strong().size(13.0).color(theme.text_primary));
                        ui.label(RichText::new("Enhance Arch3D with specialized rendering, simulation, and automation tools. Experimental plugins can be cleanly installed, disabled, or uninstalled at any time.").size(10.5).color(theme.text_muted));
                    });
                });

                ui.separator();

                // Search & Filter Bar
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Search:").size(11.0).color(theme.text_muted));
                    ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Filter plugins by name or keyword...").desired_width(220.0));

                    ui.separator();

                    ui.label(RichText::new("Category:").size(11.0).color(theme.text_muted));
                    if ui.selectable_label(self.selected_category.is_none(), "All").clicked() {
                        self.selected_category = None;
                    }
                    if ui.selectable_label(self.selected_category == Some(PluginCategory::Rendering), "Rendering").clicked() {
                        self.selected_category = Some(PluginCategory::Rendering);
                    }
                    if ui.selectable_label(self.selected_category == Some(PluginCategory::Simulation), "Simulation").clicked() {
                        self.selected_category = Some(PluginCategory::Simulation);
                    }
                });

                ui.separator();

                // Plugin Cards List
                let filtered_indices: Vec<usize> = self
                    .registry
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        if let Some(cat) = self.selected_category {
                            if p.category != cat {
                                return false;
                            }
                        }
                        if !self.search_query.trim().is_empty() {
                            let q = self.search_query.to_lowercase();
                            if !p.name.to_lowercase().contains(&q) && !p.description.to_lowercase().contains(&q) {
                                return false;
                            }
                        }
                        true
                    })
                    .map(|(i, _)| i)
                    .collect();

                let mut to_install: Option<String> = None;
                let mut to_uninstall: Option<String> = None;
                let mut to_toggle: Option<(String, bool)> = None;
                let mut to_export_blender_addon = false;

                ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                    if filtered_indices.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(30.0);
                            ui.label(RichText::new("No plugins found matching your search criteria.").size(12.0).color(theme.text_muted));
                        });
                    } else {
                        for idx in filtered_indices {
                            let p = &self.registry[idx];

                            egui::Frame::group(ui.style())
                                .fill(theme.panel_bg.gamma_multiply(0.6))
                                .stroke(Stroke::new(1.0, theme.border_color))
                                .corner_radius(6.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Plugin Icon
                                        let icon_size = Vec2::new(36.0, 36.0);
                                        let icon_rect = Rect::from_min_size(ui.cursor().min, icon_size);
                                        ui.allocate_exact_size(icon_size, Sense::hover());
                                        draw_icon(ui.painter(), icon_rect, p.icon_kind, theme.accent);

                                        // Title, Author & Tags
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(&p.name).strong().size(13.0).color(theme.text_primary));
                                                ui.label(RichText::new(format!("v{}", p.version)).size(10.0).color(theme.text_muted));

                                                if p.is_experimental {
                                                    let exp_color = Color32::from_rgb(245, 158, 11); // Amber
                                                    ui.label(RichText::new("EXPERIMENTAL").size(9.0).strong().color(exp_color));
                                                }

                                                if p.installed {
                                                    if p.enabled {
                                                        let active_color = Color32::from_rgb(16, 185, 129); // Green
                                                        ui.label(RichText::new("ACTIVE").size(9.0).strong().color(active_color));
                                                    } else {
                                                        ui.label(RichText::new("DISABLED").size(9.0).color(theme.text_muted));
                                                    }
                                                }
                                            });

                                            ui.label(RichText::new(format!("By {} • {}", p.author, p.category.label())).size(10.0).color(theme.text_muted));
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if p.installed {
                                                // Uninstall button
                                                if ui.button(RichText::new("Uninstall").size(10.5).color(Color32::from_rgb(239, 68, 68))).on_hover_text("Cleanly uninstall this plugin and remove all related UI elements").clicked() {
                                                    to_uninstall = Some(p.id.clone());
                                                }

                                                // Enable / Disable toggle
                                                let mut enabled = p.enabled;
                                                if ui.checkbox(&mut enabled, "Enabled").changed() {
                                                    to_toggle = Some((p.id.clone(), enabled));
                                                }

                                                // Launch studio button (if enabled)
                                                if p.enabled && p.live_studio_url.is_some() {
                                                    if ui.button(RichText::new("Launch Studio").size(10.5).strong().color(theme.accent)).on_hover_text("Open the live WebGPU path tracer in browser").clicked() {
                                                        action_launch_url = p.live_studio_url.clone();
                                                    }
                                                }

                                                // Export Blender addon script button
                                                if p.id == "blender-live-sync" {
                                                    if ui.button(RichText::new("Save Addon (.py)").size(10.5).color(theme.accent)).on_hover_text("Save arch3d_blender_sync.py to disk for installation into Blender").clicked() {
                                                        to_export_blender_addon = true;
                                                    }
                                                }
                                            } else {
                                                // Install button
                                                let btn_text = RichText::new("Install").size(11.0).strong().color(Color32::WHITE);
                                                if ui.add(egui::Button::new(btn_text).fill(theme.accent)).on_hover_text("Install this plugin into Arch3D").clicked() {
                                                    to_install = Some(p.id.clone());
                                                }

                                                // Allow direct addon download even before installing
                                                if p.id == "blender-live-sync" {
                                                    if ui.button(RichText::new("Get Addon (.py)").size(10.5)).on_hover_text("Download the arch3d_blender_sync.py Blender addon").clicked() {
                                                        to_export_blender_addon = true;
                                                    }
                                                }
                                            }
                                        });
                                    });

                                    ui.add_space(4.0);
                                    ui.label(RichText::new(&p.description).size(10.5).color(theme.text_muted));

                                    if let Some(ref url) = p.external_url {
                                        ui.horizontal(|ui| {
                                            ui.hyperlink_to(RichText::new("View Documentation & Source").size(10.0).color(theme.accent), url);
                                        });
                                    }
                                });
                            ui.add_space(6.0);
                        }
                    }
                });

                if let Some(id) = to_install {
                    self.install_plugin(&id);
                }
                if let Some(id) = to_uninstall {
                    self.uninstall_plugin(&id);
                }
                if let Some((id, state)) = to_toggle {
                    self.set_plugin_enabled(&id, state);
                }

                if to_export_blender_addon {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Blender Python Addon (*.py)", &["py"])
                            .set_file_name("arch3d_blender_sync.py")
                            .save_file()
                        {
                            if std::fs::write(&path, BLENDER_ADDON_SCRIPT).is_ok() {
                                self.status_message = format!("Saved Blender Addon to {}", path.display());
                            } else {
                                self.status_message = "Failed to save Blender addon script".into();
                            }
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::app::download_file_in_browser("arch3d_blender_sync.py", BLENDER_ADDON_SCRIPT.as_bytes(), "text/x-python");
                        self.status_message = "Downloaded arch3d_blender_sync.py".into();
                    }
                }

                ui.separator();

                // Status Bar & Close Button
                ui.horizontal(|ui| {
                    if !self.status_message.is_empty() {
                        ui.label(RichText::new(&self.status_message).size(11.0).color(theme.accent));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Close").size(11.0)).clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            is_open = false;
        }

        self.is_open = is_open;
        action_launch_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_lifecycle() {
        let mut mgr = PluginManagerDialog {
            is_open: true,
            search_query: String::new(),
            selected_category: None,
            registry: PluginManagerDialog::builtin_catalog(),
            status_message: String::new(),
        };

        // Initially MuTracer is not installed
        assert!(!mgr.is_plugin_active("mutracer-webgpu"));

        // Install
        let ok = mgr.install_plugin("mutracer-webgpu");
        assert!(ok);
        assert!(mgr.is_plugin_active("mutracer-webgpu"));

        // Disable
        mgr.set_plugin_enabled("mutracer-webgpu", false);
        assert!(!mgr.is_plugin_active("mutracer-webgpu"));

        // Enable again
        mgr.set_plugin_enabled("mutracer-webgpu", true);
        assert!(mgr.is_plugin_active("mutracer-webgpu"));

        // Uninstall cleanly
        let uninstalled = mgr.uninstall_plugin("mutracer-webgpu");
        assert!(uninstalled);
        assert!(!mgr.is_plugin_active("mutracer-webgpu"));

        // Verify Blender Live Sync plugin lifecycle and embedded addon script (pre-installed & active)
        assert!(mgr.is_plugin_active("blender-live-sync"));
        assert!(BLENDER_ADDON_SCRIPT.contains("bl_info"));
        assert!(BLENDER_ADDON_SCRIPT.contains("Arch3D Live Sync"));
        assert!(BLENDER_ADDON_SCRIPT.contains("ARCH3D_PT_LiveSyncPanel"));
        assert!(mgr.uninstall_plugin("blender-live-sync"));
        assert!(!mgr.is_plugin_active("blender-live-sync"));
        assert!(mgr.install_plugin("blender-live-sync"));
        assert!(mgr.is_plugin_active("blender-live-sync"));
    }
}
