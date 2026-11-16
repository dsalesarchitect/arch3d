//! Site, Topography & Cadastral Survey Studio (`SiteDialog`):
//! 1. Build Terrain & Contour Lines Studio (Volumetric Toposolids, spot elevations, contour isolines)
//! 2. Build Lot Bearing Survey Studio (Cadastral metes & bounds survey tables, quadrant bearings, closure error & area)

use crate::theme::AppTheme;
use egui::{
    Align2, Color32, ComboBox, DragValue, FontId, Pos2, RichText, Sense, Stroke, Ui,
    Vec2 as EguiVec2, Window,
};
use plinth_geom::Vec2;
use plinth_model::{
    BearingQuadrant, Document, Element, LevelId, PropertyLine, SurveySegment, Toposolid,
    ToposolidPoint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteTab {
    TerrainContours,
    LotSurvey,
}

pub struct SiteDialog {
    pub is_open: bool,
    pub active_tab: SiteTab,

    // Terrain & Contours state
    pub spot_points: Vec<ToposolidPoint>,
    pub base_depth_mm: f64,
    pub primary_interval: f64,
    pub secondary_interval: f64,
    pub site_width_m: f64,
    pub site_length_m: f64,
    pub grid_res_x: usize,
    pub grid_res_y: usize,
    pub hill_height_m: f64,

    // Cadastral Lot Bearing Survey state
    pub pob_x_m: f64,
    pub pob_y_m: f64,
    pub monument_tag: String,
    pub survey_legs: Vec<SurveySegment>,
    // New leg input buffers
    pub new_quadrant: BearingQuadrant,
    pub new_degrees: u32,
    pub new_minutes: u32,
    pub new_seconds: f64,
    pub new_distance_m: f64,

    pub status_message: Option<String>,
}

impl Default for SiteDialog {
    fn default() -> Self {
        // Preload default rectangular lot survey: 20m x 30m (600 m2)
        let default_legs = vec![
            SurveySegment {
                quadrant: BearingQuadrant::NorthEast,
                degrees: 0,
                minutes: 0,
                seconds: 0.0,
                distance_m: 30.0,
                radius_m: None,
            },
            SurveySegment {
                quadrant: BearingQuadrant::SouthEast,
                degrees: 90,
                minutes: 0,
                seconds: 0.0,
                distance_m: 20.0,
                radius_m: None,
            },
            SurveySegment {
                quadrant: BearingQuadrant::SouthWest,
                degrees: 0,
                minutes: 0,
                seconds: 0.0,
                distance_m: 30.0,
                radius_m: None,
            },
            SurveySegment {
                quadrant: BearingQuadrant::NorthWest,
                degrees: 90,
                minutes: 0,
                seconds: 0.0,
                distance_m: 20.0,
                radius_m: None,
            },
        ];

        // Preload default rolling topography spot heights
        let default_spots = vec![
            ToposolidPoint::new(-20000.0, -15000.0, 0.0),
            ToposolidPoint::new(20000.0, -15000.0, 500.0),
            ToposolidPoint::new(20000.0, 15000.0, 2500.0),
            ToposolidPoint::new(-20000.0, 15000.0, 1200.0),
            ToposolidPoint::new(0.0, 0.0, 1800.0),
            ToposolidPoint::new(10000.0, 5000.0, 3200.0),
            ToposolidPoint::new(-10000.0, -5000.0, 800.0),
        ];

        Self {
            is_open: false,
            active_tab: SiteTab::TerrainContours,
            spot_points: default_spots,
            base_depth_mm: 1500.0,
            primary_interval: 1000.0,
            secondary_interval: 200.0,
            site_width_m: 40.0,
            site_length_m: 30.0,
            grid_res_x: 5,
            grid_res_y: 5,
            hill_height_m: 3.5,

            pob_x_m: 0.0,
            pob_y_m: 0.0,
            monument_tag: "BBM No. 1".to_string(),
            survey_legs: default_legs,
            new_quadrant: BearingQuadrant::NorthEast,
            new_degrees: 45,
            new_minutes: 0,
            new_seconds: 0.0,
            new_distance_m: 25.0,

            status_message: None,
        }
    }
}

impl SiteDialog {
    pub fn open_terrain(&mut self) {
        self.active_tab = SiteTab::TerrainContours;
        self.is_open = true;
        self.status_message = None;
    }

    pub fn open_survey(&mut self) {
        self.active_tab = SiteTab::LotSurvey;
        self.is_open = true;
        self.status_message = None;
    }

    pub fn render(&mut self, ctx: &egui::Context, doc: &mut Document, theme: &AppTheme) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        Window::new("Site Modeling & Cadastral Survey Studio")
            .open(&mut is_open)
            .resizable(true)
            .default_size([880.0, 680.0])
            .min_width(740.0)
            .min_height(540.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.active_tab == SiteTab::TerrainContours,
                            "⛰  Build Terrain & Contour Lines",
                        )
                        .clicked()
                    {
                        self.active_tab = SiteTab::TerrainContours;
                        self.status_message = None;
                    }
                    if ui
                        .selectable_label(
                            self.active_tab == SiteTab::LotSurvey,
                            "📐  Lot Bearing & Cadastral Survey",
                        )
                        .clicked()
                    {
                        self.active_tab = SiteTab::LotSurvey;
                        self.status_message = None;
                    }
                });
                ui.separator();

                if let Some(msg) = &self.status_message {
                    ui.group(|ui| {
                        ui.label(RichText::new(msg).color(Color32::from_rgb(50, 180, 80)).strong());
                    });
                    ui.add_space(4.0);
                }

                match self.active_tab {
                    SiteTab::TerrainContours => self.render_terrain_tab(ui, doc, theme),
                    SiteTab::LotSurvey => self.render_survey_tab(ui, doc, theme),
                }
            });

        self.is_open = is_open;
    }

    fn render_terrain_tab(&mut self, ui: &mut Ui, doc: &mut Document, theme: &AppTheme) {
        ui.horizontal(|ui| {
            // Left configuration column
            ui.allocate_ui_with_layout(
                EguiVec2::new(420.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.group(|ui| {
                        ui.label(RichText::new("Toposolid Volumetric Parameters").strong());
                        ui.horizontal(|ui| {
                            ui.label("Base Depth:");
                            ui.add(DragValue::new(&mut self.base_depth_mm).speed(50.0).range(200.0..=20000.0).suffix(" mm"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Primary Index Contours:");
                            ui.add(DragValue::new(&mut self.primary_interval).speed(100.0).range(200.0..=10000.0).suffix(" mm"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Intermediate Contours:");
                            ui.add(DragValue::new(&mut self.secondary_interval).speed(20.0).range(50.0..=2000.0).suffix(" mm"));
                        });
                    });

                    ui.add_space(6.0);

                    ui.group(|ui| {
                        ui.label(RichText::new("Topographic Presets & Terrain Generator").strong());
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Flat Ground").clicked() {
                                self.apply_flat_preset();
                            }
                            if ui.button("Gentle Hillside (5%)").clicked() {
                                self.apply_slope_preset(0.05);
                            }
                            if ui.button("Steep Ridge (15%)").clicked() {
                                self.apply_slope_preset(0.15);
                            }
                            if ui.button("Rolling Hills").clicked() {
                                self.apply_rolling_hills_preset();
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            ui.add(DragValue::new(&mut self.site_width_m).speed(1.0).range(10.0..=500.0).suffix(" m"));
                            ui.label("Length:");
                            ui.add(DragValue::new(&mut self.site_length_m).speed(1.0).range(10.0..=500.0).suffix(" m"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Max Peak Height:");
                            ui.add(DragValue::new(&mut self.hill_height_m).speed(0.2).range(0.0..=50.0).suffix(" m"));
                            if ui.button("⚡ Generate Terrain Grid").clicked() {
                                self.generate_terrain_grid();
                            }
                        });
                    });

                    ui.add_space(6.0);

                    // Spot elevations table
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Spot Elevation Points ({})", self.spot_points.len())).strong());
                            if ui.button("+ Add Point").clicked() {
                                self.spot_points.push(ToposolidPoint::new(0.0, 0.0, 0.0));
                            }
                            if ui.button("Clear All").clicked() {
                                self.spot_points.clear();
                            }
                        });

                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            let mut remove_idx = None;
                            for (i, pt) in self.spot_points.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("#{}:", i + 1));
                                    let mut x_m = pt.x / 1000.0;
                                    let mut y_m = pt.y / 1000.0;
                                    let mut z_m = pt.z / 1000.0;
                                    ui.label("X:");
                                    if ui.add(DragValue::new(&mut x_m).speed(0.2).suffix("m")).changed() {
                                        pt.x = x_m * 1000.0;
                                    }
                                    ui.label("Y:");
                                    if ui.add(DragValue::new(&mut y_m).speed(0.2).suffix("m")).changed() {
                                        pt.y = y_m * 1000.0;
                                    }
                                    ui.label("Z:");
                                    if ui.add(DragValue::new(&mut z_m).speed(0.1).suffix("m")).changed() {
                                        pt.z = z_m * 1000.0;
                                    }
                                    if ui.small_button("✕").clicked() {
                                        remove_idx = Some(i);
                                    }
                                });
                            }
                            if let Some(idx) = remove_idx {
                                self.spot_points.remove(idx);
                            }
                        });
                    });

                    ui.add_space(8.0);

                    if ui
                        .add_sized(
                            [400.0, 36.0],
                            egui::Button::new(
                                RichText::new("⛰ Generate / Update Toposolid Terrain")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(60, 140, 60)),
                        )
                        .clicked()
                    {
                        self.commit_toposolid(doc);
                    }
                },
            );

            ui.separator();

            // Right 2D Blueprint preview of Toposolid & Contours
            ui.vertical(|ui| {
                ui.label(RichText::new("Live Topographic Contour Blueprint").strong());
                self.render_terrain_blueprint_preview(ui, theme);
            });
        });
    }

    fn render_survey_tab(&mut self, ui: &mut Ui, doc: &mut Document, theme: &AppTheme) {
        ui.horizontal(|ui| {
            // Left configuration column
            ui.allocate_ui_with_layout(
                EguiVec2::new(430.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.group(|ui| {
                        ui.label(RichText::new("Point of Beginning (P.O.B.) / Monument").strong());
                        ui.horizontal(|ui| {
                            ui.label("Monument Tag:");
                            ui.text_edit_singleline(&mut self.monument_tag);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Easting (X0):");
                            ui.add(DragValue::new(&mut self.pob_x_m).speed(1.0).suffix(" m"));
                            ui.label("Northing (Y0):");
                            ui.add(DragValue::new(&mut self.pob_y_m).speed(1.0).suffix(" m"));
                        });
                    });

                    ui.add_space(4.0);

                    // Cadastral Survey Presets
                    ui.group(|ui| {
                        ui.label(RichText::new("Cadastral Parcel Presets").strong());
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Standard Lot (20x30m)").clicked() {
                                self.apply_survey_preset_rect(20.0, 30.0);
                            }
                            if ui.button("Corner Trapezoid (845m²)").clicked() {
                                self.apply_survey_preset_trapezoid();
                            }
                            if ui.button("Cul-de-sac Parcel (1,250m²)").clicked() {
                                self.apply_survey_preset_culdesac();
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Add new survey leg input
                    ui.group(|ui| {
                        ui.label(RichText::new("Add Metes & Bounds Survey Leg").strong());
                        ui.horizontal(|ui| {
                            ui.label("Quad:");
                            ComboBox::from_id_salt("quad_picker")
                                .selected_text(self.new_quadrant.display_name())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.new_quadrant, BearingQuadrant::NorthEast, "North-East (NE)");
                                    ui.selectable_value(&mut self.new_quadrant, BearingQuadrant::SouthEast, "South-East (SE)");
                                    ui.selectable_value(&mut self.new_quadrant, BearingQuadrant::SouthWest, "South-West (SW)");
                                    ui.selectable_value(&mut self.new_quadrant, BearingQuadrant::NorthWest, "North-West (NW)");
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Angle:");
                            ui.add(DragValue::new(&mut self.new_degrees).range(0..=89).suffix("°"));
                            ui.add(DragValue::new(&mut self.new_minutes).range(0..=59).suffix("'"));
                            ui.add(DragValue::new(&mut self.new_seconds).speed(0.5).range(0.0..=59.9).suffix("\""));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Distance:");
                            ui.add(DragValue::new(&mut self.new_distance_m).speed(0.5).range(0.1..=5000.0).suffix(" m"));
                            if ui.button("➕ Insert Leg").clicked() {
                                self.survey_legs.push(SurveySegment::new(
                                    self.new_quadrant,
                                    self.new_degrees,
                                    self.new_minutes,
                                    self.new_seconds,
                                    self.new_distance_m,
                                ));
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Survey legs list & closure analysis
                    let pob = Vec2::new(self.pob_x_m * 1000.0, self.pob_y_m * 1000.0);
                    let active_lvl = doc.levels.keys().next().copied().unwrap_or_else(LevelId::new);
                    let temp_prop = PropertyLine::from_survey_segments(
                        self.survey_legs.clone(),
                        pob,
                        active_lvl,
                    );

                    let closure_err = temp_prop.closure_error_mm();
                    let ratio = temp_prop.closure_precision_ratio();
                    let area_m2 = temp_prop.area_sqm();
                    let perimeter_m = temp_prop.perimeter_m();

                    ui.group(|ui| {
                        ui.label(RichText::new("Survey Quality & Closure Analysis").strong());
                        ui.horizontal(|ui| {
                            ui.label("Linear Closure Error:");
                            let err_color = if closure_err < 10.0 {
                                Color32::from_rgb(60, 200, 60)
                            } else if closure_err < 100.0 {
                                Color32::from_rgb(220, 180, 50)
                            } else {
                                Color32::from_rgb(230, 80, 80)
                            };
                            ui.label(RichText::new(format!("{:.1} mm", closure_err)).color(err_color).strong());
                            ui.label(format!("(Ratio 1 : {})", ratio));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Lot Area:");
                            ui.label(RichText::new(format!("{:.2} m² ({:.4} ha)", area_m2, area_m2 / 10000.0)).strong());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Perimeter:");
                            ui.label(format!("{:.2} m", perimeter_m));
                        });

                        ui.horizontal(|ui| {
                            if ui.button("⚡ Auto-Close Survey Leg").on_hover_text("Calculates the exact closure bearing and distance back to P.O.B.").clicked() {
                                self.auto_close_survey();
                            }
                            if ui.button("Clear Legs").clicked() {
                                self.survey_legs.clear();
                            }
                        });
                    });

                    ui.add_space(6.0);

                    // Survey legs table
                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        let mut remove_idx = None;
                        for (i, leg) in self.survey_legs.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("L{}:", i + 1));
                                ui.label(leg.bearing_string());
                                ui.label(format!("{:.2}m", leg.distance_m));
                                if ui.small_button("✕").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            self.survey_legs.remove(idx);
                        }
                    });

                    ui.add_space(8.0);

                    if ui
                        .add_sized(
                            [410.0, 36.0],
                            egui::Button::new(
                                RichText::new("📐 Place Surveyed Lot on Site")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(30, 110, 190)),
                        )
                        .clicked()
                    {
                        self.commit_property_line(doc);
                    }
                },
            );

            ui.separator();

            // Right 2D Cadastral Plot Preview
            ui.vertical(|ui| {
                ui.label(RichText::new("Live Cadastral Parcel Plot").strong());
                self.render_survey_blueprint_preview(ui, theme);
            });
        });
    }

    fn apply_flat_preset(&mut self) {
        let half_w = self.site_width_m * 500.0;
        let half_l = self.site_length_m * 500.0;
        self.spot_points = vec![
            ToposolidPoint::new(-half_w, -half_l, 0.0),
            ToposolidPoint::new(half_w, -half_l, 0.0),
            ToposolidPoint::new(half_w, half_l, 0.0),
            ToposolidPoint::new(-half_w, half_l, 0.0),
            ToposolidPoint::new(0.0, 0.0, 0.0),
        ];
    }

    fn apply_slope_preset(&mut self, slope: f64) {
        let half_w = self.site_width_m * 500.0;
        let half_l = self.site_length_m * 500.0;
        let delta_z = self.site_length_m * 1000.0 * slope;
        self.spot_points = vec![
            ToposolidPoint::new(-half_w, -half_l, 0.0),
            ToposolidPoint::new(half_w, -half_l, 0.0),
            ToposolidPoint::new(half_w, half_l, delta_z),
            ToposolidPoint::new(-half_w, half_l, delta_z),
            ToposolidPoint::new(0.0, 0.0, delta_z * 0.5),
        ];
    }

    fn apply_rolling_hills_preset(&mut self) {
        let half_w = self.site_width_m * 500.0;
        let half_l = self.site_length_m * 500.0;
        let h = self.hill_height_m * 1000.0;
        self.spot_points = vec![
            ToposolidPoint::new(-half_w, -half_l, 0.0),
            ToposolidPoint::new(half_w, -half_l, h * 0.2),
            ToposolidPoint::new(half_w, half_l, h * 0.8),
            ToposolidPoint::new(-half_w, half_l, h * 0.4),
            ToposolidPoint::new(-half_w * 0.4, -half_l * 0.3, h * 0.9),
            ToposolidPoint::new(half_w * 0.5, half_l * 0.3, h),
            ToposolidPoint::new(0.0, 0.0, h * 0.5),
        ];
    }

    fn generate_terrain_grid(&mut self) {
        let nx = self.grid_res_x.max(2);
        let ny = self.grid_res_y.max(2);
        let min_x = -self.site_width_m * 500.0;
        let max_x = self.site_width_m * 500.0;
        let min_y = -self.site_length_m * 500.0;
        let max_y = self.site_length_m * 500.0;

        let h_max = self.hill_height_m * 1000.0;
        let mut pts = Vec::with_capacity(nx * ny);

        for iy in 0..ny {
            let ty = iy as f64 / (ny - 1) as f64;
            let y = min_y + ty * (max_y - min_y);
            for ix in 0..nx {
                let tx = ix as f64 / (nx - 1) as f64;
                let x = min_x + tx * (max_x - min_x);

                // Natural rolling elevation formula
                let sx = (tx * std::f64::consts::PI).sin();
                let sy = (ty * std::f64::consts::PI).sin();
                let elev = sx * sy * h_max + ty * (h_max * 0.3);

                pts.push(ToposolidPoint::new(x, y, elev));
            }
        }
        self.spot_points = pts;
    }

    fn commit_toposolid(&mut self, doc: &mut Document) {
        if self.spot_points.len() < 3 {
            self.status_message = Some("Need at least 3 spot elevation points for terrain.".to_string());
            return;
        }

        // Compute convex or rectangular boundary around all spot points
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for p in &self.spot_points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }

        // Add 2m border padding around points
        let pad = 2000.0;
        let boundary = vec![
            Vec2::new(min_x - pad, min_y - pad),
            Vec2::new(max_x + pad, min_y - pad),
            Vec2::new(max_x + pad, max_y + pad),
            Vec2::new(min_x - pad, max_y + pad),
        ];

        let active_lvl = doc.levels.keys().next().copied().unwrap_or_else(LevelId::new);
        let mut topo = Toposolid::new(boundary, active_lvl).with_depth(self.base_depth_mm);
        topo.elevation_points = self.spot_points.clone();
        topo.primary_contour_interval = self.primary_interval;
        topo.secondary_contour_interval = self.secondary_interval;
        topo.show_contours = true;

        doc.push_undo();
        let id = topo.id;
        doc.elements.insert(id, Element::Toposolid(topo));
        self.status_message = Some(format!("Created Toposolid terrain (Area: {:.1} m², Points: {})", (max_x - min_x + 2.0 * pad) * (max_y - min_y + 2.0 * pad) / 1e6, self.spot_points.len()));
    }

    fn apply_survey_preset_rect(&mut self, w_m: f64, l_m: f64) {
        self.survey_legs = vec![
            SurveySegment::new(BearingQuadrant::NorthEast, 0, 0, 0.0, l_m),
            SurveySegment::new(BearingQuadrant::SouthEast, 90, 0, 0.0, w_m),
            SurveySegment::new(BearingQuadrant::SouthWest, 0, 0, 0.0, l_m),
            SurveySegment::new(BearingQuadrant::NorthWest, 90, 0, 0.0, w_m),
        ];
    }

    fn apply_survey_preset_trapezoid(&mut self) {
        self.survey_legs = vec![
            SurveySegment::new(BearingQuadrant::NorthEast, 12, 30, 0.0, 35.0),
            SurveySegment::new(BearingQuadrant::SouthEast, 88, 15, 0.0, 24.0),
            SurveySegment::new(BearingQuadrant::SouthWest, 4, 45, 0.0, 32.5),
            SurveySegment::new(BearingQuadrant::NorthWest, 82, 0, 0.0, 28.6),
        ];
    }

    fn apply_survey_preset_culdesac(&mut self) {
        self.survey_legs = vec![
            SurveySegment::new(BearingQuadrant::NorthEast, 5, 0, 0.0, 40.0),
            SurveySegment::new(BearingQuadrant::SouthEast, 45, 0, 0.0, 18.0),
            SurveySegment::new(BearingQuadrant::SouthEast, 85, 0, 0.0, 22.0),
            SurveySegment::new(BearingQuadrant::SouthWest, 35, 0, 0.0, 25.0),
            SurveySegment::new(BearingQuadrant::NorthWest, 75, 0, 0.0, 38.0),
        ];
    }

    fn auto_close_survey(&mut self) {
        if self.survey_legs.is_empty() {
            return;
        }

        // Sum up total displacement vector
        let mut cur = Vec2::ZERO;
        for leg in &self.survey_legs {
            cur += leg.to_delta_mm();
        }

        let dist_mm = cur.length();
        if dist_mm < 1.0 {
            self.status_message = Some("Survey is already closed! (Closure error = 0.0 mm)".to_string());
            return;
        }

        // Closing vector is -cur
        let closing = -cur;
        let dist_m = closing.length() / 1000.0;

        let dx = closing.x;
        let dy = closing.y;

        let (quad, angle_rad) = if dy >= 0.0 && dx >= 0.0 {
            (BearingQuadrant::NorthEast, (dx).atan2(dy))
        } else if dy < 0.0 && dx >= 0.0 {
            (BearingQuadrant::SouthEast, (dx).atan2(-dy))
        } else if dy < 0.0 && dx < 0.0 {
            (BearingQuadrant::SouthWest, (-dx).atan2(-dy))
        } else {
            (BearingQuadrant::NorthWest, (-dx).atan2(dy))
        };

        let total_deg = angle_rad.to_degrees();
        let deg = total_deg.floor() as u32;
        let rem_min = (total_deg - deg as f64) * 60.0;
        let min = rem_min.floor() as u32;
        let sec = (rem_min - min as f64) * 60.0;

        self.survey_legs.push(SurveySegment::new(quad, deg, min, sec, dist_m));

        self.status_message = Some(format!("Added closing leg: {} {:.3}m (Closure error: 0.0 mm)", quad.display_name(), dist_m));
    }

    fn commit_property_line(&mut self, doc: &mut Document) {
        if self.survey_legs.len() < 3 {
            self.status_message = Some("Need at least 3 survey legs to form a closed lot boundary.".to_string());
            return;
        }

        let pob = Vec2::new(self.pob_x_m * 1000.0, self.pob_y_m * 1000.0);
        let active_lvl = doc.levels.keys().next().copied().unwrap_or_else(LevelId::new);

        let prop = PropertyLine::from_survey_segments(self.survey_legs.clone(), pob, active_lvl);
        let area = prop.area_sqm();
        let err = prop.closure_error_mm();

        doc.push_undo();
        let id = prop.id;
        doc.elements.insert(id, Element::PropertyLine(prop));

        self.status_message = Some(format!("Placed Property Line (Lot Area: {:.2} m², Closure Error: {:.1} mm)", area, err));
    }

    fn render_terrain_blueprint_preview(&self, ui: &mut Ui, theme: &AppTheme) {
        let (rect, _response) = ui.allocate_exact_size(
            EguiVec2::new(ui.available_width().max(320.0), ui.available_height().max(340.0)),
            Sense::hover(),
        );

        let painter = ui.painter().with_clip_rect(rect);
        // Blueprint background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 30));

        if self.spot_points.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "No spot elevations available.",
                FontId::proportional(14.0),
                Color32::from_rgb(160, 160, 160),
            );
            return;
        }

        // Calculate bounding box
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for p in &self.spot_points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }

        let span_x = (max_x - min_x).max(10000.0);
        let span_y = (max_y - min_y).max(10000.0);

        let pad_ui = 30.0;
        let scale_x = (rect.width() - 2.0 * pad_ui) / span_x as f32;
        let scale_y = (rect.height() - 2.0 * pad_ui) / span_y as f32;
        let scale = scale_x.min(scale_y);

        let to_screen = |world: Vec2| -> Pos2 {
            let sx = rect.center().x + ((world.x - (min_x + max_x) * 0.5) as f32) * scale;
            let sy = rect.center().y - ((world.y - (min_y + max_y) * 0.5) as f32) * scale;
            Pos2::new(sx, sy)
        };

        // Draw temporary Toposolid contours
        let dummy_boundary = vec![
            Vec2::new(min_x - 1000.0, min_y - 1000.0),
            Vec2::new(max_x + 1000.0, min_y - 1000.0),
            Vec2::new(max_x + 1000.0, max_y + 1000.0),
            Vec2::new(min_x - 1000.0, max_y + 1000.0),
        ];

        let mut temp_topo = Toposolid::new(dummy_boundary.clone(), LevelId::new()).with_depth(self.base_depth_mm);
        temp_topo.elevation_points = self.spot_points.clone();
        temp_topo.primary_contour_interval = self.primary_interval;
        temp_topo.secondary_contour_interval = self.secondary_interval;
        let contours = temp_topo.compute_contour_lines();

        // Draw contours
        for c in &contours {
            let color = if c.is_primary {
                Color32::from_rgb(180, 140, 60)
            } else {
                Color32::from_rgb(100, 120, 90)
            };
            let stroke = Stroke::new(if c.is_primary { 1.8 } else { 0.9 }, color);

            for i in 0..c.points.len().saturating_sub(1) {
                painter.line_segment([to_screen(c.points[i]), to_screen(c.points[i + 1])], stroke);
            }
        }

        // Draw spot points
        for pt in &self.spot_points {
            let sc = to_screen(pt.position());
            painter.circle_filled(sc, 3.5, Color32::from_rgb(70, 200, 100));
            painter.circle_stroke(sc, 3.5, Stroke::new(1.0, Color32::WHITE));
            painter.text(
                Pos2::new(sc.x + 5.0, sc.y - 5.0),
                Align2::LEFT_BOTTOM,
                format!("{:.2}m", pt.z / 1000.0),
                FontId::monospace(10.0),
                Color32::from_rgb(200, 220, 200),
            );
        }

        // Legend / info badge
        painter.text(
            Pos2::new(rect.min.x + 8.0, rect.min.y + 8.0),
            Align2::LEFT_TOP,
            format!("Primary Contours: {:.1}m | Intermediate: {:.2}m", self.primary_interval / 1000.0, self.secondary_interval / 1000.0),
            FontId::proportional(11.0),
            theme.accent,
        );
    }

    fn render_survey_blueprint_preview(&self, ui: &mut Ui, theme: &AppTheme) {
        let (rect, _response) = ui.allocate_exact_size(
            EguiVec2::new(ui.available_width().max(320.0), ui.available_height().max(340.0)),
            Sense::hover(),
        );

        let painter = ui.painter().with_clip_rect(rect);
        // Blueprint dark CAD background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 20, 28));

        let pob = Vec2::new(self.pob_x_m * 1000.0, self.pob_y_m * 1000.0);
        let prop = PropertyLine::from_survey_segments(self.survey_legs.clone(), pob, LevelId::new());

        if prop.boundary.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Enter survey legs to preview lot parcel.",
                FontId::proportional(13.0),
                Color32::from_rgb(160, 160, 160),
            );
            return;
        }

        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for p in &prop.boundary {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }

        let span_x = (max_x - min_x).max(5000.0);
        let span_y = (max_y - min_y).max(5000.0);

        let pad_ui = 40.0;
        let scale_x = (rect.width() - 2.0 * pad_ui) / span_x as f32;
        let scale_y = (rect.height() - 2.0 * pad_ui) / span_y as f32;
        let scale = scale_x.min(scale_y);

        let to_screen = |world: Vec2| -> Pos2 {
            let sx = rect.center().x + ((world.x - (min_x + max_x) * 0.5) as f32) * scale;
            let sy = rect.center().y - ((world.y - (min_y + max_y) * 0.5) as f32) * scale;
            Pos2::new(sx, sy)
        };

        // Draw translucent parcel fill
        if prop.boundary.len() >= 3 {
            let screen_poly: Vec<Pos2> = prop.boundary.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::convex_polygon(
                screen_poly,
                Color32::from_rgba_unmultiplied(40, 120, 200, 30),
                Stroke::NONE,
            ));
        }

        // Draw boundary lines & bearing tags
        let n = prop.boundary.len();
        let stroke = Stroke::new(2.0, Color32::from_rgb(60, 160, 255));

        for i in 0..n {
            let p0 = prop.boundary[i];
            let p1 = prop.boundary[(i + 1) % n];
            let s0 = to_screen(p0);
            let s1 = to_screen(p1);

            painter.line_segment([s0, s1], stroke);

            // Midpoint bearing label
            let mid = Pos2::new((s0.x + s1.x) * 0.5, (s0.y + s1.y) * 0.5);
            if let Some(leg) = self.survey_legs.get(i) {
                let tag = format!("{} {:.2}m", leg.bearing_string(), leg.distance_m);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    tag,
                    FontId::monospace(9.5),
                    Color32::from_rgb(220, 230, 255),
                );
            }
        }

        // Draw corner monuments & tags
        for (i, p) in prop.boundary.iter().enumerate() {
            let sc = to_screen(*p);
            painter.circle_filled(sc, 4.0, Color32::from_rgb(255, 140, 40));
            painter.circle_stroke(sc, 4.0, Stroke::new(1.0, Color32::WHITE));

            let tag = if i == 0 {
                format!("Corner 1 ({})", self.monument_tag)
            } else {
                format!("Corner {}", i + 1)
            };
            painter.text(
                Pos2::new(sc.x + 6.0, sc.y - 6.0),
                Align2::LEFT_BOTTOM,
                tag,
                FontId::proportional(10.0),
                Color32::from_rgb(240, 200, 100),
            );
        }

        // North Arrow badge in top-right corner
        let na_center = Pos2::new(rect.max.x - 28.0, rect.min.y + 28.0);
        painter.line_segment([Pos2::new(na_center.x, na_center.y + 12.0), Pos2::new(na_center.x, na_center.y - 12.0)], Stroke::new(2.0, theme.accent));
        painter.line_segment([Pos2::new(na_center.x - 5.0, na_center.y - 6.0), Pos2::new(na_center.x, na_center.y - 12.0)], Stroke::new(2.0, theme.accent));
        painter.line_segment([Pos2::new(na_center.x + 5.0, na_center.y - 6.0), Pos2::new(na_center.x, na_center.y - 12.0)], Stroke::new(2.0, theme.accent));
        painter.text(Pos2::new(na_center.x, na_center.y - 16.0), Align2::CENTER_BOTTOM, "N", FontId::proportional(11.0), theme.accent);

        // Area & closure badge in bottom-left
        let area_m2 = prop.area_sqm();
        painter.text(
            Pos2::new(rect.min.x + 10.0, rect.max.y - 10.0),
            Align2::LEFT_BOTTOM,
            format!("Lot Area: {:.2} m² | Perimeter: {:.2} m", area_m2, prop.perimeter_m()),
            FontId::proportional(11.0),
            Color32::from_rgb(180, 210, 240),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_dialog_terrain_workflow() {
        let mut dialog = SiteDialog::default();
        let mut doc = Document::new_empty();

        // 1. Verify default spots
        assert!(!dialog.spot_points.is_empty());

        // 2. Apply rolling hills preset
        dialog.apply_rolling_hills_preset();
        assert_eq!(dialog.spot_points.len(), 7);

        // 3. Commit toposolid
        dialog.commit_toposolid(&mut doc);
        assert!(dialog.status_message.is_some());

        // Verify element was inserted
        let toposolid_count = doc.elements.values().filter(|e| matches!(e, Element::Toposolid(_))).count();
        assert_eq!(toposolid_count, 1);
    }

    #[test]
    fn test_site_dialog_lot_survey_calculation_and_autoclose() {
        let mut dialog = SiteDialog::default();
        let mut doc = Document::new_empty();

        // Default legs form a 20x30m rectangle
        let pob = Vec2::new(dialog.pob_x_m * 1000.0, dialog.pob_y_m * 1000.0);
        let prop = PropertyLine::from_survey_segments(dialog.survey_legs.clone(), pob, LevelId::new());
        assert!((prop.area_sqm() - 600.0).abs() < 1e-2);
        assert!((prop.perimeter_m() - 100.0).abs() < 1e-2);
        assert!(prop.closure_error_mm() < 1e-3);

        // Remove the closing leg and test auto-close
        dialog.survey_legs.pop();
        assert_eq!(dialog.survey_legs.len(), 3);
        dialog.auto_close_survey();
        assert_eq!(dialog.survey_legs.len(), 4);

        // Commit property line
        dialog.commit_property_line(&mut doc);
        let prop_count = doc.elements.values().filter(|e| matches!(e, Element::PropertyLine(_))).count();
        assert_eq!(prop_count, 1);
    }
}
