//! Pure-Rust ISO 32000-1 vector PDF generation for architectural sheet layouts.
//!
//! Generates lossless, infinitely sharp, print-ready PDF drawings with paper borders,
//! BIM title blocks, multi-viewport scissor clipping, and pure vector linework.

use std::io::Write as IoWrite;
use plinth_geom::Vec2;
use plinth_model::{Document, Sheet, SheetId};
use crate::scene::{ColorTheme, RenderPrimitive, VisualStyle};
use crate::render_view_with_style_and_theme;

const MM_TO_PT: f64 = 72.0 / 25.4; // 2.8346456692913386 points per millimetre

/// Internal buffer for writing PostScript/PDF vector drawing commands.
#[derive(Default)]
struct PdfStream {
    buf: Vec<u8>,
}

impl PdfStream {
    fn new() -> Self {
        Self { buf: Vec::with_capacity(32 * 1024) }
    }

    fn write_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn move_to(&mut self, x: f64, y: f64) {
        let _ = write!(self.buf, "{:.2} {:.2} m\n", x, y);
    }

    fn line_to(&mut self, x: f64, y: f64) {
        let _ = write!(self.buf, "{:.2} {:.2} l\n", x, y);
    }

    fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        let _ = write!(self.buf, "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n", x1, y1, x2, y2, x3, y3);
    }

    fn close_path(&mut self) {
        self.write_str("h\n");
    }

    fn stroke(&mut self) {
        self.write_str("S\n");
    }

    fn fill(&mut self) {
        self.write_str("f\n");
    }

    fn fill_and_stroke(&mut self) {
        self.write_str("B\n");
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let _ = write!(self.buf, "{:.2} {:.2} {:.2} {:.2} re\n", x, y, w, h);
    }

    fn set_stroke_color(&mut self, r: f32, g: f32, b: f32) {
        let _ = write!(self.buf, "{:.3} {:.3} {:.3} RG\n", r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
    }

    fn set_fill_color(&mut self, r: f32, g: f32, b: f32) {
        let _ = write!(self.buf, "{:.3} {:.3} {:.3} rg\n", r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
    }

    fn set_line_width(&mut self, w: f64) {
        let _ = write!(self.buf, "{:.2} w\n", w.max(0.1));
    }

    fn set_dash(&mut self, dash: &[f64], phase: f64) {
        if dash.is_empty() {
            self.write_str("[] 0 d\n");
        } else {
            let mut s = String::from("[");
            for (i, d) in dash.iter().enumerate() {
                if i > 0 { s.push(' '); }
                s.push_str(&format!("{:.1}", d));
            }
            s.push(']');
            let _ = write!(self.buf, "{} {:.1} d\n", s, phase);
        }
    }

    fn save_state(&mut self) {
        self.write_str("q\n");
    }

    fn restore_state(&mut self) {
        self.write_str("Q\n");
    }

    fn clip(&mut self) {
        self.write_str("W n\n");
    }

    fn bezier_circle(&mut self, cx: f64, cy: f64, r: f64) {
        let d = 0.5522847498 * r;
        self.move_to(cx + r, cy);
        self.curve_to(cx + r, cy + d, cx + d, cy + r, cx, cy + r);
        self.curve_to(cx - d, cy + r, cx - r, cy + d, cx - r, cy);
        self.curve_to(cx - r, cy - d, cx - d, cy - r, cx, cy - r);
        self.curve_to(cx + d, cy - r, cx + r, cy - d, cx + r, cy);
        self.close_path();
    }

    fn draw_text(&mut self, text: &str, font: &str, size_pt: f64, x: f64, y: f64, r: f32, g: f32, b: f32) {
        let escaped = escape_pdf_string(text);
        if escaped.is_empty() { return; }
        let _ = write!(
            self.buf,
            "BT\n/{} {:.1} Tf\n{:.3} {:.3} {:.3} rg\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\nET\n",
            font, size_pt.max(4.0), r, g, b, x, y, escaped
        );
    }
}

fn escape_pdf_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            'Ø' => out.push_str("DIA "),
            '°' => out.push_str(" deg"),
            '×' => out.push('x'),
            '²' => out.push_str("2"),
            '³' => out.push_str("3"),
            _ if c.is_ascii() && !c.is_control() => out.push(c),
            _ if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

/// Renders an architectural sheet into a vector PDF content stream.
fn render_sheet_page(doc: &Document, sheet: &Sheet, visual_style: VisualStyle) -> (f64, f64, Vec<u8>) {
    let (pw_mm, ph_mm) = sheet.paper_size.dimensions_mm();
    let page_w_pt = pw_mm * MM_TO_PT;
    let page_h_pt = ph_mm * MM_TO_PT;

    let mut stream = PdfStream::new();

    // 1. Paper Background (White)
    stream.set_fill_color(1.0, 1.0, 1.0);
    stream.rect(0.0, 0.0, page_w_pt, page_h_pt);
    stream.fill();

    // 2. Margins & Outer Borders
    // Outer Border (20mm from edge)
    let m20 = 20.0 * MM_TO_PT;
    let b_w = (pw_mm - 40.0) * MM_TO_PT;
    let b_h = (ph_mm - 40.0) * MM_TO_PT;
    stream.set_stroke_color(0.0, 0.0, 0.0);
    stream.set_line_width(1.4);
    stream.set_dash(&[], 0.0);
    stream.rect(m20, m20, b_w, b_h);
    stream.stroke();

    // Inner Grid Guide (10mm from edge)
    let m10 = 10.0 * MM_TO_PT;
    let g_w = (pw_mm - 20.0) * MM_TO_PT;
    let g_h = (ph_mm - 20.0) * MM_TO_PT;
    stream.set_stroke_color(0.75, 0.78, 0.82);
    stream.set_line_width(0.6);
    stream.rect(m10, m10, g_w, g_h);
    stream.stroke();

    // 3. Architectural Title Block in Bottom Right (180mm x 65mm)
    let tb_w_mm = 180.0;
    let tb_h_mm = 65.0;
    let tb_x = (pw_mm - 20.0 - tb_w_mm) * MM_TO_PT;
    let tb_y = 20.0 * MM_TO_PT;
    let tb_w = tb_w_mm * MM_TO_PT;
    let tb_h = tb_h_mm * MM_TO_PT;

    // Titleblock Background
    stream.set_fill_color(0.975, 0.985, 0.995);
    stream.rect(tb_x, tb_y, tb_w, tb_h);
    stream.fill();

    // Titleblock Outer Border
    stream.set_stroke_color(0.0, 0.0, 0.0);
    stream.set_line_width(1.2);
    stream.rect(tb_x, tb_y, tb_w, tb_h);
    stream.stroke();

    // Titleblock Horizontal Dividers
    let div1_y = tb_y + 40.0 * MM_TO_PT;
    let div2_y = tb_y + 18.0 * MM_TO_PT;
    stream.set_stroke_color(0.3, 0.35, 0.4);
    stream.set_line_width(0.8);
    stream.move_to(tb_x, div1_y);
    stream.line_to(tb_x + tb_w, div1_y);
    stream.stroke();
    stream.move_to(tb_x, div2_y);
    stream.line_to(tb_x + tb_w, div2_y);
    stream.stroke();

    // Vertical Divider for Sheet Number in lower cell
    let div_num_x = tb_x + tb_w - 55.0 * MM_TO_PT;
    stream.move_to(div_num_x, tb_y);
    stream.line_to(div_num_x, div2_y);
    stream.stroke();

    // Titleblock Typography
    // Top Cell: Project Name & Client
    let pad = 6.0 * MM_TO_PT;
    stream.draw_text(&doc.project_info.name, "F2", 11.5, tb_x + pad, tb_y + tb_h - 13.0 * MM_TO_PT, 0.0, 0.0, 0.0);
    stream.draw_text(&format!("Client: {}", doc.project_info.client), "F1", 8.5, tb_x + pad, tb_y + tb_h - 22.0 * MM_TO_PT, 0.25, 0.25, 0.3);
    stream.draw_text(&format!("Address: {}", doc.project_info.address), "F1", 7.5, tb_x + pad, tb_y + tb_h - 30.0 * MM_TO_PT, 0.4, 0.4, 0.45);

    // Middle Cell: Sheet Title
    stream.draw_text("DRAWING TITLE", "F1", 6.5, tb_x + pad, div1_y - 8.0 * MM_TO_PT, 0.45, 0.5, 0.55);
    stream.draw_text(&sheet.name, "F2", 11.0, tb_x + pad, div1_y - 17.0 * MM_TO_PT, 0.0, 0.0, 0.0);

    // Bottom Left Cell: Metadata (Project No, Scale, Standard)
    stream.draw_text(&format!("Project No: {}   |   IFC4 BIM Standard", doc.project_info.number), "F1", 7.0, tb_x + pad, tb_y + 10.0 * MM_TO_PT, 0.3, 0.3, 0.35);
    stream.draw_text("ARCH3D BIM MODELER", "F2", 7.0, tb_x + pad, tb_y + 4.0 * MM_TO_PT, 0.0, 0.45, 0.85);

    // Bottom Right Cell: Sheet Number Tag
    stream.draw_text("SHEET NO.", "F1", 6.5, div_num_x + 6.0 * MM_TO_PT, div2_y - 6.5 * MM_TO_PT, 0.45, 0.5, 0.55);
    stream.draw_text(&sheet.sheet_number, "F2", 18.0, div_num_x + 6.0 * MM_TO_PT, tb_y + 4.5 * MM_TO_PT, 0.0, 0.0, 0.0);

    // 4. Placed Viewports on Sheet
    for vp in &sheet.viewports {
        let vp_w_mm = vp.width_mm.max(60.0);
        let vp_h_mm = vp.height_mm.max(45.0);

        let vp_x0_pt = vp.position_mm.x * MM_TO_PT;
        let vp_y0_pt = vp.position_mm.y * MM_TO_PT;
        let vp_w_pt = vp_w_mm * MM_TO_PT;
        let vp_h_pt = vp_h_mm * MM_TO_PT;

        // Viewport Background Fill (clean crisp off-white)
        stream.set_fill_color(0.995, 0.995, 1.0);
        stream.rect(vp_x0_pt, vp_y0_pt, vp_w_pt, vp_h_pt);
        stream.fill();

        // Viewport Border Line
        stream.set_stroke_color(0.72, 0.75, 0.8);
        stream.set_line_width(0.75);
        stream.set_dash(&[], 0.0);
        stream.rect(vp_x0_pt, vp_y0_pt, vp_w_pt, vp_h_pt);
        stream.stroke();

        // Viewport Scene Geometry with Scissor Clip
        if let Some(view_def) = doc.views.get(&vp.view_id) {
            let color_theme = ColorTheme::light();
            let scene = render_view_with_style_and_theme(doc, view_def, &color_theme, visual_style);

            let scale_factor = 1.0 / (vp.scale.max(1) as f64);
            let (model_cx, model_cy) = if scene.bounds.is_valid() {
                let c = scene.bounds.center();
                (c.x, c.y)
            } else {
                (0.0, 0.0)
            };

            let vp_center_x = vp.position_mm.x + vp_w_mm * 0.5;
            let vp_center_y = vp.position_mm.y + vp_h_mm * 0.5;

            // Map model mm to sheet paper pt
            let to_pt = |p: Vec2| -> (f64, f64) {
                let paper_x = vp_center_x + (p.x - model_cx) * scale_factor;
                let paper_y = vp_center_y + (p.y - model_cy) * scale_factor;
                (paper_x * MM_TO_PT, paper_y * MM_TO_PT)
            };

            // Scissor clip strictly to viewport box
            stream.save_state();
            stream.rect(vp_x0_pt, vp_y0_pt, vp_w_pt, vp_h_pt);
            stream.clip();

            for prim in &scene.primitives {
                match prim {
                    RenderPrimitive::Line { start, end, color, width, dashed } => {
                        let (x1, y1) = to_pt(*start);
                        let (x2, y2) = to_pt(*end);
                        let r = color[0] as f32 / 255.0;
                        let g = color[1] as f32 / 255.0;
                        let b = color[2] as f32 / 255.0;
                        stream.set_stroke_color(r, g, b);
                        let w_pt = ((*width as f64) * 0.75).clamp(0.4, 2.5);
                        stream.set_line_width(w_pt);
                        if *dashed {
                            stream.set_dash(&[4.0, 3.0], 0.0);
                        } else {
                            stream.set_dash(&[], 0.0);
                        }
                        stream.move_to(x1, y1);
                        stream.line_to(x2, y2);
                        stream.stroke();
                    }
                    RenderPrimitive::Polygon { points, fill_color, stroke_color, stroke_width } => {
                        if points.len() >= 3 {
                            let pts_pt: Vec<(f64, f64)> = points.iter().map(|&p| to_pt(p)).collect();
                            stream.move_to(pts_pt[0].0, pts_pt[0].1);
                            for p in &pts_pt[1..] {
                                stream.line_to(p.0, p.1);
                            }
                            stream.close_path();

                            let has_fill = fill_color[3] > 10;
                            let has_stroke = stroke_color.is_some() && *stroke_width > 0.01;

                            if has_fill {
                                let fr = fill_color[0] as f32 / 255.0;
                                let fg = fill_color[1] as f32 / 255.0;
                                let fb = fill_color[2] as f32 / 255.0;
                                stream.set_fill_color(fr, fg, fb);
                            }

                            if let Some(sc) = stroke_color {
                                if has_stroke {
                                    let sr = sc[0] as f32 / 255.0;
                                    let sg = sc[1] as f32 / 255.0;
                                    let sb = sc[2] as f32 / 255.0;
                                    stream.set_stroke_color(sr, sg, sb);
                                    let w_pt = ((*stroke_width as f64) * 0.75).clamp(0.4, 2.5);
                                    stream.set_line_width(w_pt);
                                    stream.set_dash(&[], 0.0);
                                }
                            }

                            if has_fill && has_stroke {
                                stream.fill_and_stroke();
                            } else if has_fill {
                                stream.fill();
                            } else {
                                stream.stroke();
                            }
                        }
                    }
                    RenderPrimitive::Circle { center, radius, fill_color, stroke_color, stroke_width } => {
                        let (cx, cy) = to_pt(*center);
                        let r_pt = (*radius as f64 * scale_factor) * MM_TO_PT;
                        if r_pt > 0.1 {
                            stream.bezier_circle(cx, cy, r_pt);
                            let has_fill = fill_color.map_or(false, |f| f[3] > 10);
                            let has_stroke = stroke_color[3] > 10 && *stroke_width > 0.01;
                            if let Some(f) = fill_color {
                                stream.set_fill_color(f[0] as f32 / 255.0, f[1] as f32 / 255.0, f[2] as f32 / 255.0);
                            }
                            if has_stroke {
                                stream.set_stroke_color(stroke_color[0] as f32 / 255.0, stroke_color[1] as f32 / 255.0, stroke_color[2] as f32 / 255.0);
                                stream.set_line_width((*stroke_width as f64 * 0.75).clamp(0.4, 2.5));
                                stream.set_dash(&[], 0.0);
                            }
                            if has_fill && has_stroke {
                                stream.fill_and_stroke();
                            } else if has_fill {
                                stream.fill();
                            } else {
                                stream.stroke();
                            }
                        }
                    }
                    RenderPrimitive::Arc { center, radius, start_angle, end_angle, color, width } => {
                        let r_pt = (*radius as f64 * scale_factor) * MM_TO_PT;
                        if r_pt > 0.1 {
                            let segs = 24;
                            let mut pts = Vec::with_capacity(segs + 1);
                            let r = *radius as f64;
                            for i in 0..=segs {
                                let t = i as f64 / segs as f64;
                                let a = *start_angle as f64 + (*end_angle as f64 - *start_angle as f64) * t;
                                let model_p = plinth_geom::vec2(center.x as f64 + r * a.cos(), center.y as f64 + r * a.sin());
                                pts.push(to_pt(model_p));
                            }
                            stream.set_stroke_color(color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0);
                            stream.set_line_width((*width as f64 * 0.75).clamp(0.4, 2.5));
                            stream.set_dash(&[], 0.0);
                            stream.move_to(pts[0].0, pts[0].1);
                            for p in &pts[1..] {
                                stream.line_to(p.0, p.1);
                            }
                            stream.stroke();
                        }
                    }
                    RenderPrimitive::Text { pos, text, size, color, .. } => {
                        let (tx, ty) = to_pt(*pos);
                        let r = color[0] as f32 / 255.0;
                        let g = color[1] as f32 / 255.0;
                        let b = color[2] as f32 / 255.0;
                        let font_size = (*size as f64 * 0.8).clamp(6.0, 24.0);
                        stream.draw_text(text, "F1", font_size, tx, ty, r, g, b);
                    }
                }
            }

            stream.restore_state(); // Restore graphics state (exit scissor clipping)
        }

        // Viewport Title Bubble (placed below the viewport on paper)
        if vp.show_title {
            let bubble_cx = vp_x0_pt + 8.0 * MM_TO_PT;
            let bubble_cy = vp_y0_pt - 10.0 * MM_TO_PT;
            let bubble_r = 4.5 * MM_TO_PT;

            // Detail number circle
            stream.set_fill_color(1.0, 1.0, 1.0);
            stream.bezier_circle(bubble_cx, bubble_cy, bubble_r);
            stream.fill();

            stream.set_stroke_color(0.0, 0.0, 0.0);
            stream.set_line_width(1.1);
            stream.set_dash(&[], 0.0);
            stream.bezier_circle(bubble_cx, bubble_cy, bubble_r);
            stream.stroke();

            // Detail number text
            stream.draw_text(&format!("{}", vp.number), "F2", 8.0, bubble_cx - 2.5 * MM_TO_PT, bubble_cy - 2.5 * MM_TO_PT, 0.0, 0.0, 0.0);

            // Title line extending to viewport width
            let line_x0 = bubble_cx + bubble_r + 2.0 * MM_TO_PT;
            let line_x1 = vp_x0_pt + vp_w_pt;
            stream.set_stroke_color(0.0, 0.0, 0.0);
            stream.set_line_width(0.9);
            stream.move_to(line_x0, bubble_cy);
            stream.line_to(line_x1, bubble_cy);
            stream.stroke();

            // Title & Scale text
            stream.draw_text(&vp.title, "F2", 9.0, line_x0 + 2.0 * MM_TO_PT, bubble_cy + 2.5 * MM_TO_PT, 0.0, 0.0, 0.0);
            stream.draw_text(&format!("SCALE 1:{}", vp.scale), "F1", 7.0, line_x0 + 2.0 * MM_TO_PT, bubble_cy - 7.5 * MM_TO_PT, 0.35, 0.35, 0.4);
        }
    }

    (page_w_pt, page_h_pt, stream.buf)
}

/// Assembles PDF pages into a compliant ISO 32000-1 document byte stream.
fn build_pdf_document(pages: Vec<(f64, f64, Vec<u8>)>, title: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(pages.len() * 64 * 1024 + 4096);
    out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    let num_pages = pages.len();
    let total_objs = 3 + num_pages * 2;
    let mut offsets = Vec::with_capacity(total_objs + 1);
    offsets.push(0); // Object 0 dummy

    // Object 1: Catalog
    offsets.push(out.len());
    out.extend_from_slice(b"1 0 obj\n<<\n  /Type /Catalog\n  /Pages 2 0 R\n>>\nendobj\n");

    // Object 2: Pages Parent
    offsets.push(out.len());
    let mut kids_str = String::new();
    for i in 0..num_pages {
        let page_obj_id = 4 + i * 2;
        kids_str.push_str(&format!("{} 0 R ", page_obj_id));
    }
    let pages_dict = format!(
        "2 0 obj\n<<\n  /Type /Pages\n  /Kids [{}]\n  /Count {}\n>>\nendobj\n",
        kids_str.trim(), num_pages
    );
    out.extend_from_slice(pages_dict.as_bytes());

    // Object 3: Document Info
    offsets.push(out.len());
    let escaped_title = escape_pdf_string(title);
    let info_dict = format!(
        "3 0 obj\n<<\n  /Title ({})\n  /Author (Arch3D BIM Modeler)\n  /Creator (Arch3D)\n  /Producer (Arch3D Pure-Rust Vector Engine)\n>>\nendobj\n",
        escaped_title
    );
    out.extend_from_slice(info_dict.as_bytes());

    // Objects 4+: Pages and Content Streams
    for (i, (w_pt, h_pt, content_bytes)) in pages.into_iter().enumerate() {
        let page_obj_id = 4 + i * 2;
        let stream_obj_id = page_obj_id + 1;

        // Page Object
        offsets.push(out.len());
        let page_dict = format!(
            "{} 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /MediaBox [0 0 {:.2} {:.2}]\n  /Contents {} 0 R\n  /Resources <<\n    /Font <<\n      /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\n      /F2 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\n    >>\n  >>\n>>\nendobj\n",
            page_obj_id, w_pt, h_pt, stream_obj_id
        );
        out.extend_from_slice(page_dict.as_bytes());

        // Content Stream Object
        offsets.push(out.len());
        let stream_header = format!(
            "{} 0 obj\n<<\n  /Length {}\n>>\nstream\n",
            stream_obj_id, content_bytes.len()
        );
        out.extend_from_slice(stream_header.as_bytes());
        out.extend_from_slice(&content_bytes);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Cross-Reference Table
    let xref_offset = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", total_objs + 1);
    for &off in &offsets[1..] {
        let _ = write!(out, "{:010} 00000 n \n", off);
    }

    // Trailer
    let trailer_str = format!(
        "trailer\n<<\n  /Size {}\n  /Root 1 0 R\n  /Info 3 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        total_objs + 1, xref_offset
    );
    out.extend_from_slice(trailer_str.as_bytes());

    out
}

/// Exports a single sheet to vector PDF bytes.
pub fn export_sheet_pdf(
    doc: &Document,
    sheet_id: SheetId,
    visual_style: VisualStyle,
) -> Result<Vec<u8>, String> {
    let sheet = doc.sheets.get(&sheet_id).ok_or_else(|| format!("Sheet ID {:?} not found", sheet_id))?;
    let page = render_sheet_page(doc, sheet, visual_style);
    let title = format!("{} - {}", sheet.sheet_number, sheet.name);
    Ok(build_pdf_document(vec![page], &title))
}

/// Exports all sheets in the project into a multi-page vector PDF document.
pub fn export_all_sheets_pdf(
    doc: &Document,
    visual_style: VisualStyle,
) -> Result<Vec<u8>, String> {
    if doc.sheets.is_empty() {
        return Err("No sheets found in project".to_string());
    }
    let mut pages = Vec::with_capacity(doc.sheets.len());
    for sheet in doc.sheets.values() {
        pages.push(render_sheet_page(doc, sheet, visual_style));
    }
    let title = format!("{} - Architectural Drawing Set", doc.project_info.name);
    Ok(build_pdf_document(pages, &title))
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_model::PaperSize;

    #[test]
    fn test_export_single_sheet_pdf() {
        let doc = Document::create_sample_building();
        let sheet_id = doc.sheets.keys().next().copied().unwrap();

        let pdf_bytes = export_sheet_pdf(&doc, sheet_id, VisualStyle::ConsistentColors).unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n"));
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.contains("/MediaBox [0 0 2383.94 1683.78]")); // A1 dimensions in pt
        assert!(pdf_str.contains("/Type /Page"));
        assert!(pdf_str.contains("A101")); // Sheet Number
    }

    #[test]
    fn test_export_all_sheets_multi_page_pdf() {
        let mut doc = Document::create_sample_building();
        let s2 = Sheet::new("A102", "ELEVATIONS & SECTIONS", PaperSize::A1);
        doc.sheets.insert(s2.id, s2);

        let pdf_bytes = export_all_sheets_pdf(&doc, VisualStyle::Shaded).unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n"));
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.contains("/Count 2")); // 2 pages
        assert!(pdf_str.contains("A101"));
        assert!(pdf_str.contains("A102"));
    }
}
