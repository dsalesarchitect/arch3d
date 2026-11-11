//! Application styling and dark / light theme configuration.

use egui::{Color32, Stroke, Visuals};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

pub struct AppTheme {
    pub mode: ThemeMode,
    pub panel_bg: Color32,
    pub canvas_bg: Color32,
    pub border_color: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub hover_highlight: Color32,
    pub selection_color: Color32,
}

impl AppTheme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            panel_bg: Color32::from_rgb(20, 22, 28),
            canvas_bg: Color32::from_rgb(14, 15, 20),
            border_color: Color32::from_rgb(42, 45, 56),
            text_primary: Color32::from_rgb(240, 242, 248),
            text_muted: Color32::from_rgb(140, 145, 155),
            accent: Color32::from_rgb(217, 70, 239), // Neon magenta (#D946EF) matching official logo
            hover_highlight: Color32::from_rgba_premultiplied(217, 70, 239, 40),
            selection_color: Color32::from_rgb(168, 85, 247), // Neon purple (#A855F7)
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            panel_bg: Color32::from_rgb(248, 249, 252),
            canvas_bg: Color32::from_rgb(252, 252, 254),
            border_color: Color32::from_rgb(218, 222, 230),
            text_primary: Color32::from_rgb(25, 28, 36),
            text_muted: Color32::from_rgb(110, 115, 125),
            accent: Color32::from_rgb(162, 28, 175), // Vibrant magenta/purple in light mode (#A21CAF)
            hover_highlight: Color32::from_rgba_premultiplied(162, 28, 175, 30),
            selection_color: Color32::from_rgb(192, 38, 211), // (#C026D3)
        }
    }

    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut visuals = match self.mode {
            ThemeMode::Dark => Visuals::dark(),
            ThemeMode::Light => Visuals::light(),
        };

        visuals.panel_fill = self.panel_bg;
        visuals.window_fill = self.panel_bg;
        visuals.widgets.noninteractive.bg_fill = self.panel_bg;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_color);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_primary);

        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary);

        visuals.widgets.hovered.bg_fill = self.hover_highlight;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.accent);

        visuals.widgets.active.bg_fill = self.accent;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

        ctx.set_visuals(visuals);
    }
}
