//! Multi-Storey Entity Replication Studio (Manage Tab):
//! 1. `CopyActiveLevelDialog`: Category-filtered entity copy from Active Level with Select All / Unselect All.
//! 2. `PasteToLevelsDialog`: Multi-level destination picker with automatic `ElementId`, `host_wall`/`host_column`,
//!    and `top_level` storey constraint remapping.

use crate::theme::AppTheme;
use egui::{Color32, RichText, ScrollArea, Window};
use plinth_model::{Document, Element, ElementId, LevelId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityCategoryFilter {
    pub walls: bool,
    pub doors: bool,
    pub windows: bool,
    pub ceilings: bool,
    pub columns: bool,
    pub beams: bool,
    pub floors: bool,
    pub stairs: bool,
    pub railings: bool,
    pub ramps: bool,
    pub roofs: bool,
    pub footings: bool,
}

impl Default for EntityCategoryFilter {
    fn default() -> Self {
        Self {
            walls: true,
            doors: true,
            windows: true,
            ceilings: true,
            columns: true,
            beams: true,
            floors: true,
            stairs: true,
            railings: true,
            ramps: true,
            roofs: false,
            footings: false,
        }
    }
}

impl EntityCategoryFilter {
    pub fn select_all(&mut self) {
        self.walls = true;
        self.doors = true;
        self.windows = true;
        self.ceilings = true;
        self.columns = true;
        self.beams = true;
        self.floors = true;
        self.stairs = true;
        self.railings = true;
        self.ramps = true;
        self.roofs = true;
        self.footings = true;
    }

    pub fn unselect_all(&mut self) {
        self.walls = false;
        self.doors = false;
        self.windows = false;
        self.ceilings = false;
        self.columns = false;
        self.beams = false;
        self.floors = false;
        self.stairs = false;
        self.railings = false;
        self.ramps = false;
        self.roofs = false;
        self.footings = false;
    }

    pub fn matches(&self, el: &Element) -> bool {
        match el {
            Element::Wall(_) | Element::CurtainWall(_) => self.walls,
            Element::Door(_) => self.doors,
            Element::Window(_) => self.windows,
            Element::Ceiling(_) => self.ceilings,
            Element::Column(_) => self.columns,
            Element::Beam(_) => self.beams,
            Element::Slab(_) => self.floors,
            Element::Stair(_) => self.stairs,
            Element::Railing(_) => self.railings,
            Element::Ramp(_) => self.ramps,
            Element::Roof(_) => self.roofs,
            Element::Footing(_) => self.footings,
            _ => false,
        }
    }
}

/// Resolves the hosting building storey of an element, resolving wall-hosted `Door`s and `Window`s
/// via their `host_wall` when their explicit `base_level` override is `None`.
pub fn element_effective_level(doc: &Document, el: &Element) -> Option<LevelId> {
    match el {
        Element::Door(d) => d.base_level.or_else(|| match doc.elements.get(&d.host_wall) {
            Some(Element::Wall(hw)) => Some(hw.base_level),
            _ => None,
        }),
        Element::Window(w) => w.base_level.or_else(|| match doc.elements.get(&w.host_wall) {
            Some(Element::Wall(hw)) => Some(hw.base_level),
            _ => None,
        }),
        Element::CurtainWall(cw) => cw.host_wall.and_then(|hw_id| match doc.elements.get(&hw_id) {
            Some(Element::Wall(hw)) => Some(hw.base_level),
            _ => None,
        }).or(Some(cw.base_level)),
        other => other.level_id(),
    }
}

/// Determines whether an element belongs to the storey defined by `src_id`.
/// Returns `Some(relative_storey)`:
/// - `0`: Element is hosted directly at the base of the storey (walls, columns, footings, base slabs).
/// - `1`: Element frames the ceiling/deck at the top of the storey (beams, deck slabs, ceilings at the level above).
pub fn is_storey_element(
    doc: &Document,
    el: &Element,
    src_id: LevelId,
    sorted_levels: &[(LevelId, f64)],
    include_top_framing: bool,
) -> Option<i32> {
    // 1. Direct host match on src_id (base of storey)
    if element_effective_level(doc, el) == Some(src_id) {
        return Some(0);
    }

    if !include_top_framing {
        return None;
    }

    // 2. Upper framing check: find the level immediately above src_id
    let src_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == src_id)
        .map(|(_, z)| *z)?;

    let next_level = sorted_levels
        .iter()
        .filter(|(_, z)| *z > src_z + 1.0)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let Some(&(next_id, next_z)) = next_level else {
        return None;
    };

    match el {
        Element::Beam(b) => {
            if b.level == next_id {
                return Some(1);
            }
            if let Some(l) = doc.levels.get(&b.level) {
                if (l.elevation - next_z).abs() < 50.0 {
                    return Some(1);
                }
            }
            None
        }
        Element::Slab(s) => {
            if s.level == next_id {
                return Some(1);
            }
            if let Some(l) = doc.levels.get(&s.level) {
                if (l.elevation - next_z).abs() < 50.0 {
                    return Some(1);
                }
            }
            None
        }
        Element::Ceiling(c) => {
            if c.level == next_id {
                return Some(1);
            }
            if let Some(l) = doc.levels.get(&c.level) {
                if (l.elevation - next_z).abs() < 50.0 {
                    return Some(1);
                }
            }
            None
        }
        Element::Roof(r) => {
            if r.level == next_id {
                return Some(1);
            }
            if let Some(l) = doc.levels.get(&r.level) {
                if (l.elevation - next_z).abs() < 50.0 {
                    return Some(1);
                }
            }
            None
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Default)]
pub struct LevelClipboard {
    pub source_level_id: Option<LevelId>,
    pub source_level_name: String,
    pub source_elevation: f64,
    pub elements: Vec<Element>,
    /// Relative storey offset per element: 0 = base level, 1 = top framing level (ceiling/deck).
    pub relative_storeys: BTreeMap<ElementId, i32>,
    /// Stores original host wall `(start, end)` geometry so Doors/Windows copied without Walls
    /// can still find co-linear walls on destination levels.
    pub host_wall_axes: BTreeMap<ElementId, (plinth_geom::Vec2, plinth_geom::Vec2)>,
}

impl LevelClipboard {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// Copies all entities belonging to `source_level_id` matching `filter` into a `LevelClipboard`.
/// When `include_top_framing` is true, framing beams and deck slabs situated at the ceiling of the storey
/// are captured as relative top-framing entities (+1 storey).
pub fn copy_level_entities(
    doc: &Document,
    source_level_id: LevelId,
    filter: &EntityCategoryFilter,
    include_top_framing: bool,
) -> LevelClipboard {
    let mut sorted_levels: Vec<(LevelId, f64)> =
        doc.levels.values().map(|l| (l.id, l.elevation)).collect();
    sorted_levels.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let (src_name, src_elev) = doc
        .levels
        .get(&source_level_id)
        .map(|l| (l.name.clone(), l.elevation))
        .unwrap_or_else(|| ("Source Level".into(), 0.0));

    let mut elements = Vec::new();
    let mut relative_storeys = BTreeMap::new();
    let mut host_wall_axes = BTreeMap::new();

    for el in doc.elements.values() {
        if let Some(rel) = is_storey_element(doc, el, source_level_id, &sorted_levels, include_top_framing) {
            if filter.matches(el) {
                match el {
                    Element::Door(d) => {
                        if let Some(Element::Wall(hw)) = doc.elements.get(&d.host_wall) {
                            host_wall_axes.insert(hw.id, (hw.start, hw.end));
                        }
                    }
                    Element::Window(w) => {
                        if let Some(Element::Wall(hw)) = doc.elements.get(&w.host_wall) {
                            host_wall_axes.insert(hw.id, (hw.start, hw.end));
                        }
                    }
                    Element::CurtainWall(cw) => {
                        if let Some(hw_id) = cw.host_wall {
                            if let Some(Element::Wall(hw)) = doc.elements.get(&hw_id) {
                                host_wall_axes.insert(hw.id, (hw.start, hw.end));
                            }
                        }
                    }
                    _ => {}
                }
                relative_storeys.insert(el.id(), rel);
                elements.push(el.clone());
            }
        }
    }

    LevelClipboard {
        source_level_id: Some(source_level_id),
        source_level_name: src_name,
        source_elevation: src_elev,
        elements,
        relative_storeys,
        host_wall_axes,
    }
}

/// Backward-compatible wrapper copying entities from `doc.active_level_id` with top framing enabled.
pub fn copy_active_level_entities(doc: &Document, filter: &EntityCategoryFilter) -> LevelClipboard {
    copy_level_entities(doc, doc.active_level_id, filter, true)
}

/// Remaps an optional `top_level` from `source_level` to the corresponding storey above `target_level`.
/// Returns `(remapped_top_level, fallback_span_mm)`.
fn remap_top_constraint(
    sorted_levels: &[(LevelId, f64)],
    source_base: LevelId,
    source_top: Option<LevelId>,
    target_base: LevelId,
) -> (Option<LevelId>, Option<f64>) {
    let Some(src_top_id) = source_top else {
        return (None, None);
    };

    let src_base_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == source_base)
        .map(|(_, z)| *z)
        .unwrap_or(0.0);
    let src_top_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == src_top_id)
        .map(|(_, z)| *z)
        .unwrap_or(src_base_z + 3500.0);
    let tgt_base_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == target_base)
        .map(|(_, z)| *z)
        .unwrap_or(0.0);

    let src_span = (src_top_z - src_base_z).max(500.0);

    // Count how many distinct storeys above source_base is source_top
    let storeys_above = sorted_levels
        .iter()
        .filter(|(_, z)| *z > src_base_z + 1.0 && *z <= src_top_z + 1.0)
        .count()
        .max(1);

    // Find the level that is `storeys_above` storeys above target_base
    let higher_levels: Vec<(LevelId, f64)> = sorted_levels
        .iter()
        .copied()
        .filter(|(_, z)| *z > tgt_base_z + 1.0)
        .collect();

    if storeys_above <= higher_levels.len() {
        return (Some(higher_levels[storeys_above - 1].0), None);
    }

    // Target is at top of building with no higher level: convert cleanly to unconnected span
    (None, Some(src_span))
}

/// Resolves the destination level for an element copied with a relative storey offset.
/// - `relative_storey <= 0`: Hosted directly at `target_base`.
/// - `relative_storey > 0`: Hosted at the level `relative_storey` steps above `target_base`.
/// If destination storey exceeds the highest defined level, returns `(target_base, extra_z_offset)`.
fn resolve_target_level_with_offset(
    sorted_levels: &[(LevelId, f64)],
    target_base: LevelId,
    relative_storey: i32,
    src_span: f64,
) -> (LevelId, f64) {
    if relative_storey <= 0 {
        return (target_base, 0.0);
    }

    let tgt_base_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == target_base)
        .map(|(_, z)| *z)
        .unwrap_or(0.0);

    let mut higher_levels: Vec<(LevelId, f64)> = sorted_levels
        .iter()
        .copied()
        .filter(|(_, z)| *z > tgt_base_z + 1.0)
        .collect();
    higher_levels.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let step = relative_storey as usize;
    if step <= higher_levels.len() {
        (higher_levels[step - 1].0, 0.0)
    } else {
        // Destination storey is past the highest defined level: host on target_base with Z offset
        let extra_steps = step - higher_levels.len();
        (target_base, src_span * (extra_steps as f64))
    }
}

/// Pastes `clipboard` entities onto every level in `target_levels` with fresh `ElementId`s,
/// preserving wall-hosted Door/Window openings (`host_wall`), Column-hosted Footings (`host_column`),
/// and multi-storey `top_level` constraints.
pub fn paste_clipboard_to_levels(
    doc: &mut Document,
    clipboard: &LevelClipboard,
    target_levels: &[LevelId],
    remap_top_levels: bool,
) -> usize {
    if clipboard.elements.is_empty() || target_levels.is_empty() {
        return 0;
    }

    let valid_targets: Vec<LevelId> = target_levels
        .iter()
        .copied()
        .filter(|lid| doc.levels.contains_key(lid))
        .collect();
    if valid_targets.is_empty() {
        return 0;
    }

    let mut sorted_levels: Vec<(LevelId, f64)> =
        doc.levels.values().map(|l| (l.id, l.elevation)).collect();
    sorted_levels.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let src_lvl = clipboard.source_level_id.unwrap_or(doc.active_level_id);
    let src_base_z = sorted_levels
        .iter()
        .find(|(id, _)| *id == src_lvl)
        .map(|(_, z)| *z)
        .unwrap_or(0.0);
    let next_z = sorted_levels
        .iter()
        .filter(|(_, z)| *z > src_base_z + 1.0)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, z)| *z);
    let src_span = next_z.map(|nz| nz - src_base_z).unwrap_or(3500.0).max(500.0);

    doc.push_undo();
    let mut total_pasted = 0usize;

    for &tgt_lvl in &valid_targets {
        let mut wall_id_map: BTreeMap<ElementId, ElementId> = BTreeMap::new();
        let mut col_id_map: BTreeMap<ElementId, ElementId> = BTreeMap::new();

        // Pass 1: Paste Host Elements (Walls & Columns) first so hosted Doors/Windows/Footings map cleanly
        for el in &clipboard.elements {
            match el {
                Element::Wall(w) => {
                    let mut nw = w.clone();
                    nw.id = ElementId::new();
                    nw.base_level = tgt_lvl;
                    if remap_top_levels {
                        let (new_top, fallback_h) =
                            remap_top_constraint(&sorted_levels, src_lvl, w.top_level, tgt_lvl);
                        nw.top_level = new_top;
                        if let Some(h) = fallback_h {
                            nw.unconnected_height = h;
                        }
                    }
                    wall_id_map.insert(w.id, nw.id);
                    doc.elements.insert(nw.id, Element::Wall(nw));
                    total_pasted += 1;
                }
                Element::Column(c) => {
                    let mut nc = c.clone();
                    nc.id = ElementId::new();
                    nc.base_level = tgt_lvl;
                    if remap_top_levels {
                        let (new_top, fallback_h) =
                            remap_top_constraint(&sorted_levels, src_lvl, c.top_level, tgt_lvl);
                        nc.top_level = new_top;
                        if let Some(h) = fallback_h {
                            nc.unconnected_height = h;
                        }
                    }
                    col_id_map.insert(c.id, nc.id);
                    doc.elements.insert(nc.id, Element::Column(nc));
                    total_pasted += 1;
                }
                _ => {}
            }
        }

        // Pass 2: Paste Hosted Elements (Doors, Windows, Footings) & Remaining Level Entities
        for el in &clipboard.elements {
            match el {
                Element::Wall(_) | Element::Column(_) => {}

                Element::Door(d) => {
                    let resolved_host = wall_id_map.get(&d.host_wall).copied().or_else(|| {
                        let (ws, we) = clipboard.host_wall_axes.get(&d.host_wall).copied()?;
                        doc.elements.values().find_map(|cand| match cand {
                            Element::Wall(tw)
                                if tw.base_level == tgt_lvl
                                    && tw.start.distance(ws) < 80.0
                                    && tw.end.distance(we) < 80.0 =>
                            {
                                Some(tw.id)
                            }
                            _ => None,
                        })
                    });
                    if let Some(new_host) = resolved_host {
                        let mut nd = d.clone();
                        nd.id = ElementId::new();
                        nd.host_wall = new_host;
                        if nd.base_level == Some(src_lvl) {
                            nd.base_level = Some(tgt_lvl);
                        }
                        if remap_top_levels {
                            let (new_top, _) =
                                remap_top_constraint(&sorted_levels, src_lvl, d.top_level, tgt_lvl);
                            nd.top_level = new_top;
                        }
                        doc.elements.insert(nd.id, Element::Door(nd));
                        total_pasted += 1;
                    }
                }

                Element::Window(w) => {
                    let resolved_host = wall_id_map.get(&w.host_wall).copied().or_else(|| {
                        let (ws, we) = clipboard.host_wall_axes.get(&w.host_wall).copied()?;
                        doc.elements.values().find_map(|cand| match cand {
                            Element::Wall(tw)
                                if tw.base_level == tgt_lvl
                                    && tw.start.distance(ws) < 80.0
                                    && tw.end.distance(we) < 80.0 =>
                            {
                                Some(tw.id)
                            }
                            _ => None,
                        })
                    });
                    if let Some(new_host) = resolved_host {
                        let mut nw = w.clone();
                        nw.id = ElementId::new();
                        nw.host_wall = new_host;
                        if nw.base_level == Some(src_lvl) {
                            nw.base_level = Some(tgt_lvl);
                        }
                        if remap_top_levels {
                            let (new_top, _) =
                                remap_top_constraint(&sorted_levels, src_lvl, w.top_level, tgt_lvl);
                            nw.top_level = new_top;
                        }
                        doc.elements.insert(nw.id, Element::Window(nw));
                        total_pasted += 1;
                    }
                }

                Element::CurtainWall(cw) => {
                    let mut ncw = cw.clone();
                    ncw.id = ElementId::new();
                    ncw.base_level = tgt_lvl;
                    if let Some(old_hw) = cw.host_wall {
                        ncw.host_wall = wall_id_map.get(&old_hw).copied().or_else(|| {
                            clipboard.host_wall_axes.get(&old_hw).and_then(|&(ws, we)| {
                                doc.elements.values().find_map(|e| match e {
                                    Element::Wall(tw)
                                        if tw.base_level == tgt_lvl
                                            && tw.start.distance(ws) < 80.0
                                            && tw.end.distance(we) < 80.0 =>
                                    {
                                        Some(tw.id)
                                    }
                                    _ => None,
                                })
                            })
                        });
                    }
                    if remap_top_levels {
                        let (new_top, fallback_h) =
                            remap_top_constraint(&sorted_levels, src_lvl, cw.top_level, tgt_lvl);
                        ncw.top_level = new_top;
                        if let Some(h) = fallback_h {
                            ncw.unconnected_height = h;
                        }
                    }
                    doc.elements.insert(ncw.id, Element::CurtainWall(ncw));
                    total_pasted += 1;
                }

                Element::Footing(f) => {
                    let mut nf = f.clone();
                    nf.id = ElementId::new();
                    nf.base_level = tgt_lvl;
                    nf.host_column = f.host_column.and_then(|cid| col_id_map.get(&cid).copied());
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, f.top_level, tgt_lvl);
                        nf.top_level = new_top;
                    }
                    doc.elements.insert(nf.id, Element::Footing(nf));
                    total_pasted += 1;
                }

                Element::Ceiling(c) => {
                    let rel = clipboard.relative_storeys.get(&c.id).copied().unwrap_or(0);
                    let (dest_lvl, extra_z) =
                        resolve_target_level_with_offset(&sorted_levels, tgt_lvl, rel, src_span);
                    let mut nc = c.clone();
                    nc.id = ElementId::new();
                    nc.level = dest_lvl;
                    nc.height_above_level += extra_z;
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, c.top_level, dest_lvl);
                        nc.top_level = new_top;
                    }
                    doc.elements.insert(nc.id, Element::Ceiling(nc));
                    total_pasted += 1;
                }

                Element::Beam(b) => {
                    let rel = clipboard.relative_storeys.get(&b.id).copied().unwrap_or(0);
                    let (dest_lvl, extra_z) =
                        resolve_target_level_with_offset(&sorted_levels, tgt_lvl, rel, src_span);
                    let mut nb = b.clone();
                    nb.id = ElementId::new();
                    nb.level = dest_lvl;
                    nb.z_offset += extra_z;
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, b.top_level, dest_lvl);
                        nb.top_level = new_top;
                    }
                    doc.elements.insert(nb.id, Element::Beam(nb));
                    total_pasted += 1;
                }

                Element::Slab(s) => {
                    let rel = clipboard.relative_storeys.get(&s.id).copied().unwrap_or(0);
                    let (dest_lvl, extra_z) =
                        resolve_target_level_with_offset(&sorted_levels, tgt_lvl, rel, src_span);
                    let mut ns = s.clone();
                    ns.id = ElementId::new();
                    ns.level = dest_lvl;
                    ns.offset += extra_z;
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, s.top_level, dest_lvl);
                        ns.top_level = new_top;
                    }
                    doc.elements.insert(ns.id, Element::Slab(ns));
                    total_pasted += 1;
                }

                Element::Stair(s) => {
                    let mut ns = s.clone();
                    ns.id = ElementId::new();
                    ns.base_level = tgt_lvl;
                    if remap_top_levels {
                        let (new_top, fallback_h) =
                            remap_top_constraint(&sorted_levels, src_lvl, s.top_level, tgt_lvl);
                        ns.top_level = new_top;
                        if let Some(h) = fallback_h {
                            ns.height = h;
                        }
                    }
                    doc.elements.insert(ns.id, Element::Stair(ns));
                    total_pasted += 1;
                }

                Element::Ramp(r) => {
                    let mut nr = r.clone();
                    nr.id = ElementId::new();
                    nr.base_level = tgt_lvl;
                    if remap_top_levels {
                        let (new_top, fallback_h) =
                            remap_top_constraint(&sorted_levels, src_lvl, r.top_level, tgt_lvl);
                        nr.top_level = new_top;
                        if let Some(h) = fallback_h {
                            nr.height = h;
                        }
                    }
                    doc.elements.insert(nr.id, Element::Ramp(nr));
                    total_pasted += 1;
                }

                Element::Railing(rl) => {
                    let rel = clipboard.relative_storeys.get(&rl.id).copied().unwrap_or(0);
                    let (dest_lvl, extra_z) =
                        resolve_target_level_with_offset(&sorted_levels, tgt_lvl, rel, src_span);
                    let mut nrl = rl.clone();
                    nrl.id = ElementId::new();
                    nrl.level = dest_lvl;
                    nrl.base_offset += extra_z;
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, rl.top_level, dest_lvl);
                        nrl.top_level = new_top;
                    }
                    doc.elements.insert(nrl.id, Element::Railing(nrl));
                    total_pasted += 1;
                }

                Element::Roof(r) => {
                    let rel = clipboard.relative_storeys.get(&r.id).copied().unwrap_or(0);
                    let (dest_lvl, extra_z) =
                        resolve_target_level_with_offset(&sorted_levels, tgt_lvl, rel, src_span);
                    let mut nr = r.clone();
                    nr.id = ElementId::new();
                    nr.level = dest_lvl;
                    nr.offset += extra_z;
                    if remap_top_levels {
                        let (new_top, _) =
                            remap_top_constraint(&sorted_levels, src_lvl, r.top_level, dest_lvl);
                        nr.top_level = new_top;
                    }
                    doc.elements.insert(nr.id, Element::Roof(nr));
                    total_pasted += 1;
                }

                _ => {}
            }
        }
    }

    total_pasted
}

// ===========================================================================
// Copy Entities on Active Level Dialog UI
// ===========================================================================

pub struct CopyActiveLevelDialog {
    pub is_open: bool,
    pub source_level_id: Option<LevelId>,
    pub include_top_framing: bool,
    pub filter: EntityCategoryFilter,
    pub status_msg: String,
}

impl Default for CopyActiveLevelDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            source_level_id: None,
            include_top_framing: true,
            filter: EntityCategoryFilter::default(),
            status_msg: String::new(),
        }
    }
}

impl CopyActiveLevelDialog {
    pub fn open_with_defaults(&mut self, doc: &Document) {
        self.is_open = true;
        self.source_level_id = Some(doc.active_level_id);
        self.status_msg.clear();
    }

    /// Renders the Copy Entities dialog. Returns `true` if the user clicked "Copy & Paste to Other Levels ➡".
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        doc: &Document,
        clipboard: &mut LevelClipboard,
        theme: &AppTheme,
    ) -> bool {
        let mut open_paste_next = false;
        if !self.is_open {
            return false;
        }

        let src_id = self.source_level_id.unwrap_or(doc.active_level_id);
        let src_id = if doc.levels.contains_key(&src_id) {
            src_id
        } else {
            doc.active_level_id
        };
        self.source_level_id = Some(src_id);

        let (src_name, src_elev) = doc
            .levels
            .get(&src_id)
            .map(|l| (l.name.clone(), l.elevation))
            .unwrap_or_else(|| ("Source Level".into(), 0.0));

        let mut sorted_levels: Vec<(LevelId, String, f64)> = doc
            .levels
            .values()
            .map(|l| (l.id, l.name.clone(), l.elevation))
            .collect();
        sorted_levels.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        let sorted_level_pairs: Vec<(LevelId, f64)> =
            sorted_levels.iter().map(|(id, _, z)| (*id, *z)).collect();

        // Count elements per category for the selected source storey
        let mut c_wall = 0usize;
        let mut c_door = 0usize;
        let mut c_win = 0usize;
        let mut c_ceil = 0usize;
        let mut c_col = 0usize;
        let mut c_beam = 0usize;
        let mut c_floor = 0usize;
        let mut c_stair = 0usize;
        let mut c_rail = 0usize;
        let mut c_ramp = 0usize;
        let mut c_roof = 0usize;
        let mut c_foot = 0usize;
        let mut selected_total = 0usize;

        for el in doc.elements.values() {
            if is_storey_element(doc, el, src_id, &sorted_level_pairs, self.include_top_framing).is_none() {
                continue;
            }
            if self.filter.matches(el) {
                selected_total += 1;
            }
            match el {
                Element::Wall(_) | Element::CurtainWall(_) => c_wall += 1,
                Element::Door(_) => c_door += 1,
                Element::Window(_) => c_win += 1,
                Element::Ceiling(_) => c_ceil += 1,
                Element::Column(_) => c_col += 1,
                Element::Beam(_) => c_beam += 1,
                Element::Slab(_) => c_floor += 1,
                Element::Stair(_) => c_stair += 1,
                Element::Railing(_) => c_rail += 1,
                Element::Ramp(_) => c_ramp += 1,
                Element::Roof(_) => c_roof += 1,
                Element::Footing(_) => c_foot += 1,
                _ => {}
            }
        }

        let mut open = self.is_open;
        Window::new("📋 Copy Entities on Active Level")
            .open(&mut open)
            .default_size([540.0, 440.0])
            .resizable(true)
            .show(ctx, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Source Storey / Level:").strong().size(12.5).color(theme.accent));
                        egui::ComboBox::from_id_salt("copy_level_source_picker")
                            .selected_text(format!("{} ({:+.0} mm)", src_name, src_elev))
                            .show_ui(ui, |ui| {
                                for (lid, nm, elev) in &sorted_levels {
                                    let sel = *lid == src_id;
                                    if ui.selectable_label(sel, format!("{} ({:+.0} mm)", nm, elev)).clicked() {
                                        self.source_level_id = Some(*lid);
                                    }
                                }
                            });
                    });
                    ui.checkbox(
                        &mut self.include_top_framing,
                        "☑ Include Storey Framing Beams & Deck Slabs (Ceiling Framing)",
                    )
                    .on_hover_text("Includes structural beams, deck slabs, and ceilings at the top framing level of this storey");
                    ui.label(
                        "Choose which BIM entity categories on this storey to copy to the Level Clipboard:",
                    );
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("☑ Select All").strong()).clicked() {
                        self.filter.select_all();
                    }
                    if ui.button("☐ Unselect All").clicked() {
                        self.filter.unselect_all();
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Matching Entities: {}", selected_total)).strong(),
                    );
                });
                ui.separator();

                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("Architectural Categories").strong().size(11.5));
                    cols[0].checkbox(&mut self.filter.walls, format!("Walls ({})", c_wall));
                    cols[0].checkbox(&mut self.filter.doors, format!("Doors ({})", c_door));
                    cols[0].checkbox(&mut self.filter.windows, format!("Windows ({})", c_win));
                    cols[0].checkbox(&mut self.filter.ceilings, format!("Ceilings ({})", c_ceil));
                    cols[0].checkbox(&mut self.filter.floors, format!("Floors / Slabs ({})", c_floor));
                    cols[0].checkbox(&mut self.filter.stairs, format!("Stairs ({})", c_stair));
                    cols[0].checkbox(&mut self.filter.railings, format!("Railings ({})", c_rail));

                    cols[1].label(RichText::new("Structural & Envelope Categories").strong().size(11.5));
                    cols[1].checkbox(&mut self.filter.columns, format!("Columns ({})", c_col));
                    cols[1].checkbox(&mut self.filter.beams, format!("Beams ({})", c_beam));
                    cols[1].checkbox(
                        &mut self.filter.footings,
                        format!("Foundation Footings ({})", c_foot),
                    );
                    cols[1].checkbox(&mut self.filter.ramps, format!("Ramps ({})", c_ramp));
                    cols[1].checkbox(&mut self.filter.roofs, format!("Roofs ({})", c_roof));
                });

                ui.add_space(10.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            selected_total > 0,
                            egui::Button::new(RichText::new("📋 Copy Selected Entities").strong()),
                        )
                        .clicked()
                    {
                        *clipboard = copy_level_entities(doc, src_id, &self.filter, self.include_top_framing);
                        self.status_msg = format!(
                            "Copied {} entities from '{}'. Click 'Paste to Levels' to replicate.",
                            clipboard.elements.len(),
                            clipboard.source_level_name
                        );
                    }

                    if ui
                        .add_enabled(
                            selected_total > 0,
                            egui::Button::new(
                                RichText::new("📋 Copy & Paste to Other Levels ➡")
                                    .strong()
                                    .color(theme.accent),
                            ),
                        )
                        .clicked()
                    {
                        *clipboard = copy_level_entities(doc, src_id, &self.filter, self.include_top_framing);
                        self.status_msg = format!(
                            "Copied {} entities from '{}'.",
                            clipboard.elements.len(),
                            clipboard.source_level_name
                        );
                        open_paste_next = true;
                    }
                });

                if !self.status_msg.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&self.status_msg).size(11.0).color(theme.accent));
                }
            });

        if open_paste_next {
            open = false;
        }
        self.is_open = open;
        open_paste_next
    }
}

// ===========================================================================
// Paste Copied Entities to Other Levels Dialog UI
// ===========================================================================

pub struct PasteToLevelsDialog {
    pub is_open: bool,
    pub selected_levels: BTreeSet<LevelId>,
    pub remap_top_levels: bool,
    pub status_msg: String,
}

impl Default for PasteToLevelsDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_levels: BTreeSet::new(),
            remap_top_levels: true,
            status_msg: String::new(),
        }
    }
}

impl PasteToLevelsDialog {
    pub fn open_with_defaults(&mut self, doc: &Document, clipboard: &LevelClipboard) {
        self.is_open = true;
        self.selected_levels.retain(|lid| doc.levels.contains_key(lid));
        if self.selected_levels.is_empty() {
            // Pre-select next level above source level if available
            let src_z = clipboard.source_elevation;
            if let Some(next_lvl) = doc
                .levels
                .values()
                .filter(|l| Some(l.id) != clipboard.source_level_id && l.elevation > src_z + 10.0)
                .min_by(|a, b| {
                    a.elevation
                        .partial_cmp(&b.elevation)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                self.selected_levels.insert(next_lvl.id);
            }
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        clipboard: &LevelClipboard,
        theme: &AppTheme,
    ) {
        if !self.is_open {
            return;
        }

        let mut sorted_levels: Vec<(LevelId, String, f64)> = doc
            .levels
            .values()
            .map(|l| (l.id, l.name.clone(), l.elevation))
            .collect();
        sorted_levels.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let mut open = self.is_open;
        Window::new("📥 Paste Entities to Other Levels")
            .open(&mut open)
            .default_size([540.0, 440.0])
            .resizable(true)
            .show(ctx, |ui| {
                if clipboard.is_empty() {
                    ui.colored_label(
                        Color32::from_rgb(255, 180, 70),
                        "Level Clipboard is currently empty. Use 'Copy Level' in the Manage tab first!",
                    );
                    return;
                }

                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Clipboard: {} Entities from '{}' ({:+.0} mm)",
                            clipboard.elements.len(),
                            clipboard.source_level_name,
                            clipboard.source_elevation
                        ))
                        .strong()
                        .size(12.5)
                        .color(theme.accent),
                    );

                    // Breakdown of entities in clipboard
                    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
                    for el in &clipboard.elements {
                        let cat = match el {
                            Element::Wall(_) => "Walls",
                            Element::CurtainWall(_) => "Curtain Walls",
                            Element::Door(_) => "Doors",
                            Element::Window(_) => "Windows",
                            Element::Ceiling(_) => "Ceilings",
                            Element::Column(_) => "Columns",
                            Element::Beam(_) => "Beams",
                            Element::Slab(_) => "Floors / Slabs",
                            Element::Footing(_) => "Footings",
                            Element::Stair(_) => "Stairs",
                            Element::Railing(_) => "Railings",
                            Element::Ramp(_) => "Ramps",
                            Element::Roof(_) => "Roofs",
                            _ => "Other",
                        };
                        *counts.entry(cat).or_insert(0) += 1;
                    }
                    let summary: Vec<String> =
                        counts.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                    ui.label(
                        RichText::new(format!("Contents: {}", summary.join(", ")))
                            .size(11.0)
                            .color(theme.text_muted),
                    );
                    ui.label(
                        "Select one or multiple destination storeys to replicate these entities:",
                    );
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("☑ Select All Other Storeys").strong())
                        .clicked()
                    {
                        self.selected_levels.clear();
                        for (lid, _, _) in &sorted_levels {
                            if Some(*lid) != clipboard.source_level_id {
                                self.selected_levels.insert(*lid);
                            }
                        }
                    }
                    if ui.button("☐ Unselect All").clicked() {
                        self.selected_levels.clear();
                    }
                });

                ui.separator();
                ScrollArea::vertical()
                    .id_salt("paste_levels_scroll")
                    .max_height(190.0)
                    .show(ui, |ui| {
                        for (lid, nm, elev) in &sorted_levels {
                            let is_src = Some(*lid) == clipboard.source_level_id;
                            let mut checked = self.selected_levels.contains(lid);
                            let label = if is_src {
                                format!("{} ({:+.0} mm)  [Source Level]", nm, elev)
                            } else {
                                format!("{} ({:+.0} mm)", nm, elev)
                            };
                            if ui.checkbox(&mut checked, label).changed() {
                                if checked {
                                    self.selected_levels.insert(*lid);
                                } else {
                                    self.selected_levels.remove(lid);
                                }
                            }
                        }
                    });

                ui.separator();
                ui.checkbox(
                    &mut self.remap_top_levels,
                    "Automatically Remap Top Level Constraints per Storey",
                );

                ui.add_space(6.0);
                let target_vec: Vec<LevelId> = self.selected_levels.iter().copied().collect();
                if ui
                    .add_enabled(
                        !target_vec.is_empty(),
                        egui::Button::new(
                            RichText::new(format!(
                                "📥 Paste {} Entities to {} Selected Level(s)",
                                clipboard.elements.len(),
                                target_vec.len()
                            ))
                            .strong()
                            .size(13.0)
                            .color(theme.accent),
                        ),
                    )
                    .clicked()
                {
                    let pasted = paste_clipboard_to_levels(
                        doc,
                        clipboard,
                        &target_vec,
                        self.remap_top_levels,
                    );
                    self.status_msg = format!(
                        "Successfully pasted {} entities across {} destination level(s)!",
                        pasted,
                        target_vec.len()
                    );
                }

                if !self.status_msg.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&self.status_msg).size(11.0).color(theme.accent));
                }
            });

        self.is_open = open;
    }
}
