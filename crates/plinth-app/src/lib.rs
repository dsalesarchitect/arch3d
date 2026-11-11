//! # plinth-app
//!
//! Plinth BIM application components: Ribbon, Project Browser, Properties Panel,
//! Canvas viewport, Tool state machine, and Dark/Light themes.

pub mod app;
pub mod browser;
pub mod canvas;
pub mod gpu_viewport;
pub mod icons;
pub mod level_copy;
pub mod level_manager;
pub mod plugin_manager;
pub mod properties;
pub mod ribbon;
pub mod settings;
pub mod site_tools;
pub mod structure_auto;
pub mod theme;
pub mod tool;
pub mod tool_config;

pub use app::PlinthApp;
pub use plugin_manager::{PluginCategory, PluginManagerDialog, PluginRecord};
pub use settings::AppSettings;
pub use site_tools::{SiteDialog, SiteTab};
pub use tool_config::{ToolCategory, ToolConfigStudio};

/// Entry point to launch the Arch3D native desktop application.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_app() -> eframe::Result<()> {
    let _ = env_logger::try_init();

    // Ensure arch3d.ico is saved in the working directory for desktop shortcuts
    if !std::path::Path::new("arch3d.ico").exists() {
        let ico_bytes = icons::create_arch3d_ico_bytes(64);
        let _ = std::fs::write("arch3d.ico", ico_bytes);
    }

    let icon_data = icons::arch3d_icon_data();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Arch3D — Architectural & Structural Modeler (IFC4 Standard)")
            .with_icon(icon_data)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        depth_buffer: 24,
        multisampling: 4,
        ..Default::default()
    };

    let initial_file = std::env::args().nth(1).map(std::path::PathBuf::from);

    eframe::run_native(
        "Arch3D",
        options,
        Box::new(move |_cc| {
            let mut app = PlinthApp::default();
            if let Some(ref file_path) = initial_file {
                if file_path.exists() {
                    app.open_project_file(file_path);
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn start(&self, canvas_id: &str) -> Result<(), JsValue> {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str(&format!("Canvas with id '{canvas_id}' not found")))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Element is not an HtmlCanvasElement"))?;

        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_cc| Ok(Box::new(PlinthApp::default()))),
            )
            .await
    }

    #[wasm_bindgen]
    pub fn destroy(&self) {
        self.runner.destroy();
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
    }

    #[wasm_bindgen]
    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|s| s.message())
    }
}
