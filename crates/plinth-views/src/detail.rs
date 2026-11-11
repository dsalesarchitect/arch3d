//! Detail callout and 2D drafting view projection.

use crate::scene::{ColorTheme, RenderPrimitive, RenderScene};
use plinth_model::{Document, Element, ViewDef};

pub fn render_detail(doc: &Document, view: &ViewDef, theme: &ColorTheme) -> RenderScene {
    let mut scene = RenderScene::new();

    // Render detail lines and dimensions assigned to this view
    for el in doc.elements.values() {
        match el {
            Element::DetailLine(l) => {
                if l.view_id == view.id {
                    scene.add(RenderPrimitive::Line {
                        start: l.start,
                        end: l.end,
                        color: theme.cut_line,
                        width: match l.style.as_str() {
                            "Wide" => 2.5,
                            "Medium" => 1.5,
                            _ => 1.0,
                        },
                        dashed: l.style == "Hidden",
                    });
                }
            }
            Element::Dimension(d) => {
                if d.view_id == view.id {
                    crate::annotation::render_dimension(&mut scene, d, theme, view.scale);
                }
            }
            Element::TextNote(t) => {
                if t.view_id == view.id {
                    crate::annotation::render_text_note(&mut scene, t, theme, view.scale);
                }
            }
            Element::Leader(ld) => {
                if ld.view_id == view.id {
                    crate::annotation::render_leader(&mut scene, ld, theme, view.scale);
                }
            }
            _ => {}
        }
    }

    scene
}
