//! Standards-compliant IFC4 STEP physical file exporter (ISO 10303-21).

use crate::guid::uuid_to_ifc_guid;
use plinth_geom::{Vec2, Vec3};
use plinth_model::{Document, Element, ElementId, Wall};
use std::collections::BTreeMap;
use std::fmt::Write;
use uuid::Uuid;

struct IfcStepBuilder {
    next_id: usize,
    buffer: String,
}

impl IfcStepBuilder {
    fn new() -> Self {
        Self {
            next_id: 1,
            buffer: String::with_capacity(32 * 1024),
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn add_line(&mut self, entity: &str) -> usize {
        let id = self.alloc_id();
        let _ = writeln!(self.buffer, "#{id}={entity};");
        id
    }

    fn add_point_3d(&mut self, p: Vec3) -> usize {
        self.add_line(&format!("IFCCARTESIANPOINT(({:.3},{:.3},{:.3}))", p.x, p.y, p.z))
    }

    fn add_point_2d(&mut self, p: Vec2) -> usize {
        self.add_line(&format!("IFCCARTESIANPOINT(({:.3},{:.3}))", p.x, p.y))
    }

    /// Closed profile built from an `IFCPOLYLINE` whose last point repeats the first
    /// (an `IFCPOLYLOOP` is not a valid curve for `IFCARBITRARYCLOSEDPROFILEDEF`).
    fn add_arbitrary_profile(&mut self, pts: &[Vec2]) -> usize {
        let mut pt_ids = Vec::with_capacity(pts.len() + 1);
        for p in pts {
            pt_ids.push(format!("#{}", self.add_point_2d(*p)));
        }
        pt_ids.push(pt_ids[0].clone());
        let poly = self.add_line(&format!("IFCPOLYLINE(({}))", pt_ids.join(",")));
        self.add_line(&format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{poly})"))
    }

    fn add_extruded_solid(
        &mut self,
        profile_id: usize,
        pos_id: usize,
        depth: f64,
        dir_id: usize,
    ) -> usize {
        self.add_line(&format!(
            "IFCEXTRUDEDAREASOLID(#{profile_id},#{pos_id},#{dir_id},{:.3})",
            depth
        ))
    }

    fn add_axis2_3d(&mut self, origin: usize, axis: usize, ref_dir: usize) -> usize {
        self.add_line(&format!("IFCAXIS2PLACEMENT3D(#{origin},#{axis},#{ref_dir})"))
    }

    /// Builds an `IFCFACETEDBREP` (closed shell of polygonal faces) from boundary faces.
    /// Vertex Z coordinates are offset by `storey_elevation` to be local to containing storey.
    fn add_faceted_brep(&mut self, faces: &[plinth_geom::Face], storey_elevation: f64) -> Option<usize> {
        let mut point_ids: std::collections::HashMap<(i64, i64, i64), usize> =
            std::collections::HashMap::new();
        let mut face_ids = Vec::new();

        for face in faces {
            if face.pts.len() < 3 {
                continue;
            }
            // Drop consecutive duplicate vertices (degenerate loop edges)
            let mut pts: Vec<Vec3> = Vec::with_capacity(face.pts.len());
            for p in &face.pts {
                let local_p = Vec3::new(p.x, p.y, p.z - storey_elevation);
                if pts.last().map_or(true, |q| (*q - local_p).length() > 0.01) {
                    pts.push(local_p);
                }
            }
            if pts.len() > 1 && (pts[0] - pts[pts.len() - 1]).length() <= 0.01 {
                pts.pop();
            }
            if pts.len() < 3 {
                continue;
            }

            let mut ids = Vec::with_capacity(pts.len());
            for p in &pts {
                let key = (
                    (p.x * 100.0).round() as i64,
                    (p.y * 100.0).round() as i64,
                    (p.z * 100.0).round() as i64,
                );
                let id = match point_ids.get(&key) {
                    Some(id) => *id,
                    None => {
                        let id = self.add_point_3d(*p);
                        point_ids.insert(key, id);
                        id
                    }
                };
                ids.push(format!("#{id}"));
            }
            let poly_loop = self.add_line(&format!("IFCPOLYLOOP(({}))", ids.join(",")));
            let bound = self.add_line(&format!("IFCFACEOUTERBOUND(#{poly_loop},.T.)"));
            let face_id = self.add_line(&format!("IFCFACE((#{bound}))"));
            face_ids.push(format!("#{face_id}"));
        }

        if face_ids.is_empty() {
            return None;
        }
        let shell = self.add_line(&format!("IFCCLOSEDSHELL(({}))", face_ids.join(",")));
        Some(self.add_line(&format!("IFCFACETEDBREP(#{shell})")))
    }

    fn add_prop_single_string(&mut self, name: &str, val: &str) -> usize {
        let val_id = self.add_line(&format!("IFCTEXT('{val}')"));
        self.add_line(&format!("IFCPROPERTYSINGLEVALUE('{name}',$,#{val_id},$)"))
    }

    fn add_prop_single_bool(&mut self, name: &str, val: bool) -> usize {
        let b_str = if val { ".T." } else { ".F." };
        let val_id = self.add_line(&format!("IFCBOOLEAN({b_str})"));
        self.add_line(&format!("IFCPROPERTYSINGLEVALUE('{name}',$,#{val_id},$)"))
    }

    fn add_prop_single_real(&mut self, name: &str, val: f64) -> usize {
        let val_id = self.add_line(&format!("IFCREAL({:.3})", val));
        self.add_line(&format!("IFCPROPERTYSINGLEVALUE('{name}',$,#{val_id},$)"))
    }
}

/// Metadata describing a door, window, or embedded curtain wall opening inside a host wall.
struct HostedOpeningInfo {
    hosted_id: ElementId,
    name: String,
    center_2d: Vec2,
    width: f64,
    sill_z: f64,
    height: f64,
}

/// Scans the document for all physical openings hosted on the given wall.
fn find_wall_hosted_openings(doc: &Document, wall: &Wall) -> Vec<HostedOpeningInfo> {
    let mut result = Vec::new();
    let base_z = doc.level_elevation(wall.base_level);
    let top_z = wall.top_level.map(|lvl| doc.level_elevation(lvl));
    let (wall_z0, _) = wall.z_bounds(base_z, top_z);
    let u = wall.direction();

    for el in doc.elements.values() {
        match el {
            Element::Door(d) if d.host_wall == wall.id => {
                let d_base_z = d.base_level.map(|lvl| doc.level_elevation(lvl)).unwrap_or(wall_z0);
                let d_top_z = d.top_level.map(|lvl| doc.level_elevation(lvl));
                let (z0, z1) = d.z_bounds(d_base_z, d_top_z);
                let center_2d = wall.start + u * d.offset_along_wall;
                result.push(HostedOpeningInfo {
                    hosted_id: d.id,
                    name: d.name.clone(),
                    center_2d,
                    width: d.width,
                    sill_z: z0,
                    height: (z1 - z0).max(100.0),
                });
            }
            Element::Window(w) if w.host_wall == wall.id => {
                let w_base_z = w.base_level.map(|lvl| doc.level_elevation(lvl)).unwrap_or(wall_z0);
                let w_top_z = w.top_level.map(|lvl| doc.level_elevation(lvl));
                let (z0, z1) = w.z_bounds(w_base_z, w_top_z);
                let center_2d = wall.start + u * w.offset_along_wall;
                result.push(HostedOpeningInfo {
                    hosted_id: w.id,
                    name: w.name.clone(),
                    center_2d,
                    width: w.width,
                    sill_z: z0,
                    height: (z1 - z0).max(100.0),
                });
            }
            Element::CurtainWall(cw) if cw.automatically_embed => {
                let is_hosted = cw.host_wall == Some(wall.id);
                let is_collinear = if cw.host_wall.is_none() {
                    let hw_dir = wall.direction();
                    let hw_norm = wall.normal();
                    cw.direction().dot(hw_dir).abs() > 0.98
                        && (cw.start - wall.start).dot(hw_norm).abs() < 50.0
                } else {
                    false
                };
                if is_hosted || is_collinear {
                    let hw_dir = wall.direction();
                    let s_dist = (cw.start - wall.start).dot(hw_dir);
                    let e_dist = (cw.end - wall.start).dot(hw_dir);
                    let s = s_dist.min(e_dist).max(0.0);
                    let e = s_dist.max(e_dist).min(wall.length());
                    if e > s + 1.0 {
                        let cw_base_z = doc.level_elevation(cw.base_level);
                        let cw_top_z = cw.top_level.map(|lvl| doc.level_elevation(lvl));
                        let (z0, z1) = cw.z_bounds(cw_base_z, cw_top_z);
                        let center_2d = wall.start + hw_dir * ((s + e) * 0.5);
                        result.push(HostedOpeningInfo {
                            hosted_id: cw.id,
                            name: cw.name.clone(),
                            center_2d,
                            width: e - s,
                            sill_z: z0,
                            height: (z1 - z0).max(100.0),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    result
}

pub fn export_ifc4(doc: &Document) -> String {
    let mut b = IfcStepBuilder::new();

    // 1. Common Geometry & Unit Setup
    let origin_zero = b.add_line("IFCCARTESIANPOINT((0.,0.,0.))");
    let dir_z = b.add_line("IFCDIRECTION((0.,0.,1.))");
    let dir_x = b.add_line("IFCDIRECTION((1.,0.,0.))");
    let placement_world = b.add_axis2_3d(origin_zero, dir_z, dir_x);

    // Units (millimetres, radians, square metres)
    let unit_len = b.add_line("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
    let unit_ang = b.add_line("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    let unit_area = b.add_line("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
    let unit_assign = b.add_line(&format!("IFCUNITASSIGNMENT((#{unit_len},#{unit_ang},#{unit_area}))"));

    // Owner history & Context
    let person = b.add_line("IFCPERSON($,'Plinth Architect',$,$,$,$,$,$)");
    let org = b.add_line("IFCORGANIZATION($,'Plinth BIM',$,$,$)");
    let person_org = b.add_line(&format!("IFCPERSONANDORGANIZATION(#{person},#{org},$)"));
    let app = b.add_line(&format!("IFCAPPLICATION(#{org},'1.0','Plinth BIM','Plinth')"));
    let owner_hist = b.add_line(&format!(
        "IFCOWNERHISTORY(#{person_org},#{app},.READWRITE.,.ADDED.,1730300000,#{person_org},#{app},1730300000)"
    ));

    // Context
    let geom_context = b.add_line(&format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{placement_world},$)"
    ));

    // 'Body' sub-context (MODEL_VIEW)
    let body_context = b.add_line(&format!(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{geom_context},$,.MODEL_VIEW.,$)"
    ));

    // 2. Spatial Hierarchy: Project -> Site -> Building -> Storeys
    // Register both geometric context and body subcontext in IFCPROJECT
    let proj_guid = uuid_to_ifc_guid(Uuid::new_v4());
    let ifc_proj = b.add_line(&format!(
        "IFCPROJECT('{proj_guid}',#{owner_hist},'{}',$,$,$,$,(#{geom_context},#{body_context}),#{unit_assign})",
        doc.project_info.name
    ));

    let site_place = b.add_line(&format!("IFCLOCALPLACEMENT($,#{placement_world})"));
    let site_guid = uuid_to_ifc_guid(Uuid::new_v4());
    let ifc_site = b.add_line(&format!(
        "IFCSITE('{site_guid}',#{owner_hist},'Site',$,$,#{site_place},$,$,.ELEMENT.,$,$,$,$,$)"
    ));

    let bldg_place = b.add_line(&format!("IFCLOCALPLACEMENT(#{site_place},#{placement_world})"));
    let bldg_guid = uuid_to_ifc_guid(Uuid::new_v4());
    let ifc_bldg = b.add_line(&format!(
        "IFCBUILDING('{bldg_guid}',#{owner_hist},'Building',$,$,#{bldg_place},$,$,.ELEMENT.,$,$,$)"
    ));

    // Link Project -> Site
    let rel_proj_site = uuid_to_ifc_guid(Uuid::new_v4());
    b.add_line(&format!(
        "IFCRELAGGREGATES('{rel_proj_site}',#{owner_hist},'ProjectSite',$,#{ifc_proj},(#{ifc_site}))"
    ));

    // Link Site -> Building
    let rel_site_bldg = uuid_to_ifc_guid(Uuid::new_v4());
    b.add_line(&format!(
        "IFCRELAGGREGATES('{rel_site_bldg}',#{owner_hist},'SiteBuilding',$,#{ifc_site},(#{ifc_bldg}))"
    ));

    // Storeys
    let mut storey_entity_ids = Vec::new();
    let mut level_to_storey = BTreeMap::new();

    for (level_id, level) in &doc.levels {
        let elev_pt = b.add_point_3d(Vec3::new(0.0, 0.0, level.elevation));
        let storey_axis = b.add_axis2_3d(elev_pt, dir_z, dir_x);
        let storey_place = b.add_line(&format!("IFCLOCALPLACEMENT(#{bldg_place},#{storey_axis})"));
        let storey_guid = uuid_to_ifc_guid(level.id.0);

        let storey_id = b.add_line(&format!(
            "IFCBUILDINGSTOREY('{storey_guid}',#{owner_hist},'{}',$,$,#{storey_place},$,$,.ELEMENT.,{:.3})",
            level.name, level.elevation
        ));

        storey_entity_ids.push(storey_id);
        level_to_storey.insert(*level_id, (storey_id, storey_place, level.elevation));
    }

    // Link Building -> Storeys
    let rel_bldg_storeys = uuid_to_ifc_guid(Uuid::new_v4());
    let storeys_str = storey_entity_ids
        .iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",");
    b.add_line(&format!(
        "IFCRELAGGREGATES('{rel_bldg_storeys}',#{owner_hist},'BuildingStoreys',$,#{ifc_bldg},({storeys_str}))"
    ));

    // 3. Elements Export
    let mut storey_contained_elements: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for s_id in &storey_entity_ids {
        storey_contained_elements.insert(*s_id, Vec::new());
    }

    let (default_storey_id, default_storey_place, default_elevation) = level_to_storey
        .values()
        .next()
        .copied()
        .unwrap_or((ifc_bldg, bldg_place, 0.0));

    let all_walls: Vec<&Wall> = doc
        .elements
        .values()
        .filter_map(|e| match e {
            Element::Wall(w) => Some(w),
            _ => None,
        })
        .collect();

    let mut el_to_entity: BTreeMap<ElementId, usize> = BTreeMap::new();
    let mut void_to_fill: Vec<(usize, ElementId)> = Vec::new();

    for (el_id, el) in &doc.elements {
        // Skip non-physical elements (dimensions, detail lines, text notes, leaders, ...)
        if matches!(el, Element::Dimension(_) | Element::DetailLine(_) | Element::TextNote(_) | Element::Leader(_) | Element::FreeformSurface(_)) {
            continue;
        }

        // Spatial containment & placement: Every element is placed relative to its containing storey's
        // local coordinate system (IFCLOCALPLACEMENT with target_storey_place as parent).
        let (target_storey, target_storey_place, storey_elev) = el
            .level_id()
            .and_then(|lid| level_to_storey.get(&lid).copied())
            .unwrap_or((default_storey_id, default_storey_place, default_elevation));

        let el_guid = uuid_to_ifc_guid(el.id().0);
        let el_place = b.add_line(&format!("IFCLOCALPLACEMENT(#{target_storey_place},#{placement_world})"));

        // Prismatic elements -> parametric SweptSolid (IFCEXTRUDEDAREASOLID) so that BIM
        // authoring tools keep editable extrusions; everything else (roofs, stairs, ramps,
        // railings, curtain walls, freeform) -> exact faceted B-Rep from the viewport faces.
        let is_prismatic = matches!(
            el,
            Element::Wall(_)
                | Element::Column(_)
                | Element::Beam(_)
                | Element::Slab(_)
                | Element::Door(_)
                | Element::Window(_)
                | Element::Ceiling(_)
        );

        let mut rep_items = Vec::new();
        let mut rep_type = "SweptSolid";

        if let Element::Wall(w) = el {
            // Walls are exported as a single solid body extrusion of their full mitered footprint.
            // Openings are cleanly cut out via IFCOPENINGELEMENT + IFCRELVOIDSELEMENT (standard CSG).
            let base_z = doc.level_elevation(w.base_level);
            let top_z = w.top_level.map(|lvl| doc.level_elevation(lvl));
            let (z0, z1) = w.z_bounds(base_z, top_z);
            let height = (z1 - z0).max(10.0);
            let z_local = z0 - storey_elev;
            let mut profile = w.mitered_footprint(&all_walls).0;
            if profile.len() >= 3 {
                plinth_geom::polygon::make_ccw(&mut profile);
                let prof_id = b.add_arbitrary_profile(&profile);
                let prism_origin = b.add_point_3d(Vec3::new(0.0, 0.0, z_local));
                let prism_pos = b.add_axis2_3d(prism_origin, dir_z, dir_x);
                let solid_id = b.add_extruded_solid(prof_id, prism_pos, height, dir_z);
                rep_items.push(format!("#{solid_id}"));
            }
        } else if is_prismatic {
            for prism in &doc.element_solids(*el_id) {
                if prism.profile.len() < 3 || prism.height() < 1.0 {
                    continue;
                }
                let mut profile = prism.profile.clone();
                plinth_geom::polygon::make_ccw(&mut profile);
                let prof_id = b.add_arbitrary_profile(&profile);
                let z_local = prism.z_min - storey_elev;
                let prism_origin = b.add_point_3d(Vec3::new(0.0, 0.0, z_local));
                let prism_pos = b.add_axis2_3d(prism_origin, dir_z, dir_x);
                let solid_id = b.add_extruded_solid(prof_id, prism_pos, prism.height(), dir_z);
                rep_items.push(format!("#{solid_id}"));
            }
        }

        if rep_items.is_empty() {
            let faces = doc.element_faces(*el_id);
            if let Some(brep_id) = b.add_faceted_brep(&faces, storey_elev) {
                rep_items.push(format!("#{brep_id}"));
                rep_type = "Brep";
            }
        }

        if rep_items.is_empty() {
            continue;
        }

        let shape_rep = b.add_line(&format!(
            "IFCSHAPEREPRESENTATION(#{body_context},'Body','{rep_type}',({}))",
            rep_items.join(",")
        ));
        let prod_shape = b.add_line(&format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape_rep}))"));

        let elem_entity_id = match el {
            Element::Wall(w) => {
                let id = b.add_line(&format!(
                    "IFCWALL('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.STANDARD.)",
                    w.name, w.wall_type_name
                ));
                let p1 = b.add_prop_single_bool("LoadBearing", w.is_structural);
                let p2 = b.add_prop_single_bool("IsExternal", w.is_external);
                let p3 = b.add_prop_single_bool("Combustible", w.combustible);
                let p4 = b.add_prop_single_bool("Compartmentation", w.compartmentation);
                let p5 = b.add_prop_single_string("FireRating", &w.fire_rating);
                let p6 = b.add_prop_single_string("AcousticRating", &w.acoustic_rating);
                let p7 = b.add_prop_single_string("SurfaceSpreadOfFlame", &w.surface_spread_of_flame);
                let p8 = b.add_prop_single_real("ThermalTransmittance", w.thermal_transmittance);
                let p9 = b.add_prop_single_string("Tag", &w.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_WallCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6},#{p7},#{p8},#{p9}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));

                // Generate IFCOPENINGELEMENT and IFCRELVOIDSELEMENT for all hosted openings in this wall
                let openings = find_wall_hosted_openings(doc, w);
                let u = w.direction();
                let n = w.normal();
                for op in openings {
                    let half_w = op.width * 0.5;
                    let half_t = (w.thickness + 20.0) * 0.5; // Extends 10mm beyond wall on each face to guarantee clean CSG subtraction
                    let p0 = op.center_2d - u * half_w - n * half_t;
                    let p1 = op.center_2d + u * half_w - n * half_t;
                    let p2 = op.center_2d + u * half_w + n * half_t;
                    let p3 = op.center_2d - u * half_w + n * half_t;
                    let mut op_poly = vec![p0, p1, p2, p3];
                    plinth_geom::polygon::make_ccw(&mut op_poly);
                    let op_prof_id = b.add_arbitrary_profile(&op_poly);
                    let op_z_local = op.sill_z - storey_elev;
                    let op_origin = b.add_point_3d(Vec3::new(0.0, 0.0, op_z_local));
                    let op_pos = b.add_axis2_3d(op_origin, dir_z, dir_x);
                    let op_solid = b.add_extruded_solid(op_prof_id, op_pos, op.height, dir_z);
                    let op_shape = b.add_line(&format!(
                        "IFCSHAPEREPRESENTATION(#{body_context},'Body','SweptSolid',(#{op_solid}))"
                    ));
                    let op_prod = b.add_line(&format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{op_shape}))"));
                    let op_place = b.add_line(&format!("IFCLOCALPLACEMENT(#{target_storey_place},#{placement_world})"));
                    let op_guid = uuid_to_ifc_guid(Uuid::new_v4());
                    let op_id = b.add_line(&format!(
                        "IFCOPENINGELEMENT('{op_guid}',#{owner_hist},'{}',$,$,#{op_place},#{op_prod},$,.OPENING.)",
                        op.name
                    ));
                    let rvoid_guid = uuid_to_ifc_guid(Uuid::new_v4());
                    b.add_line(&format!(
                        "IFCRELVOIDSELEMENT('{rvoid_guid}',#{owner_hist},$,$,#{id},#{op_id})"
                    ));
                    void_to_fill.push((op_id, op.hosted_id));
                }

                id
            }
            Element::Column(c) => {
                let id = b.add_line(&format!(
                    "IFCCOLUMN('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.COLUMN.)",
                    c.name, c.column_type_name
                ));
                let p1 = b.add_prop_single_bool("LoadBearing", c.is_structural);
                let p2 = b.add_prop_single_bool("IsExternal", c.is_external);
                let p3 = b.add_prop_single_string("FireRating", &c.fire_rating);
                let p4 = b.add_prop_single_real("LoadBearingCapacity", c.load_bearing_capacity);
                let p5 = b.add_prop_single_string("Tag", &c.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_ColumnCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Beam(bm) => {
                let id = b.add_line(&format!(
                    "IFCBEAM('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.BEAM.)",
                    bm.name, bm.beam_type_name
                ));
                let p1 = b.add_prop_single_bool("LoadBearing", bm.is_structural);
                let p2 = b.add_prop_single_bool("IsExternal", bm.is_external);
                let p3 = b.add_prop_single_string("FireRating", &bm.fire_rating);
                let p4 = b.add_prop_single_real("LoadBearingCapacity", bm.load_bearing_capacity);
                let p5 = b.add_prop_single_string("Tag", &bm.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_BeamCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Slab(s) => {
                let id = b.add_line(&format!(
                    "IFCSLAB('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.FLOOR.)",
                    s.name, s.slab_type_name
                ));
                let p1 = b.add_prop_single_bool("LoadBearing", s.is_structural);
                let p2 = b.add_prop_single_bool("IsExternal", s.is_external);
                let p3 = b.add_prop_single_bool("Compartmentation", s.compartmentation);
                let p4 = b.add_prop_single_string("FireRating", &s.fire_rating);
                let p5 = b.add_prop_single_string("AcousticRating", &s.acoustic_rating);
                let p6 = b.add_prop_single_real("ThermalTransmittance", s.thermal_transmittance);
                let p7 = b.add_prop_single_real("LoadBearingCapacity", s.load_bearing_capacity);
                let p8 = b.add_prop_single_string("Tag", &s.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_SlabCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6},#{p7},#{p8}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Roof(r) => {
                let id = b.add_line(&format!(
                    "IFCROOF('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                    r.name, r.roof_type_name
                ));
                let p1 = b.add_prop_single_bool("IsStructural", r.is_structural);
                let p2 = b.add_prop_single_bool("IsExternal", r.is_external);
                let p3 = b.add_prop_single_string("FireRating", &r.fire_rating);
                let p4 = b.add_prop_single_real("ThermalTransmittance", r.thermal_transmittance);
                let p5 = b.add_prop_single_string("AcousticRating", &r.acoustic_rating);
                let p6 = b.add_prop_single_real("SolarReflectanceIndex", r.solar_reflectance_index);
                let p7 = b.add_prop_single_real("SnowLoadCapacity", r.snow_load_capacity);
                let shape_name = match r.shape {
                    plinth_model::RoofShape::Hip => "Hip",
                    plinth_model::RoofShape::Gable => "Gable",
                    plinth_model::RoofShape::Shed => "Shed",
                    plinth_model::RoofShape::Flat => "Flat",
                };
                let p8 = b.add_prop_single_string("RoofShape", shape_name);
                let p9 = b.add_prop_single_string("Tag", &r.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_RoofCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6},#{p7},#{p8},#{p9}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Fascia(f) => b.add_line(&format!(
                "IFCCOVERING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.FASCIA.)",
                f.name, f.material_name
            )),
            Element::Gutter(g) => b.add_line(&format!(
                "IFCFLOWSEGMENT('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                g.name, g.material_name
            )),
            Element::Ceiling(c) => {
                let id = b.add_line(&format!(
                    "IFCCOVERING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.CEILING.)",
                    c.name, c.ceiling_type_name
                ));
                let p1 = b.add_prop_single_bool("IsExternal", c.is_external);
                let p2 = b.add_prop_single_string("FireRating", &c.fire_rating);
                let p3 = b.add_prop_single_string("AcousticRating", &c.acoustic_rating);
                let p4 = b.add_prop_single_real("LightReflectance", c.light_reflectance);
                let p5 = b.add_prop_single_string("Tag", &c.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_CeilingCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Door(d) => {
                let id = b.add_line(&format!(
                    "IFCDOOR('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',{:.1},{:.1},.DOOR.,.NOTDEFINED.)",
                    d.name, d.door_type_name, d.tag, d.height, d.width
                ));
                let p1 = b.add_prop_single_bool("IsExternal", d.is_external);
                let p2 = b.add_prop_single_bool("FireExit", d.is_fire_exit);
                let p3 = b.add_prop_single_string("FireRating", &d.fire_rating);
                let p4 = b.add_prop_single_string("AcousticRating", &d.acoustic_rating);
                let p5 = b.add_prop_single_string("SecurityRating", &d.security_rating);
                let p6 = b.add_prop_single_real("ThermalTransmittance", d.thermal_transmittance);
                let p7 = b.add_prop_single_string("OperationType", &d.operation_type);
                let p8 = b.add_prop_single_string("Tag", &d.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_DoorCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6},#{p7},#{p8}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Window(win) => {
                let id = b.add_line(&format!(
                    "IFCWINDOW('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',{:.1},{:.1},.WINDOW.,.NOTDEFINED.)",
                    win.name, win.window_type_name, win.tag, win.height, win.width
                ));
                let p1 = b.add_prop_single_bool("IsExternal", win.is_external);
                let p2 = b.add_prop_single_string("FireRating", &win.fire_rating);
                let p3 = b.add_prop_single_string("AcousticRating", &win.acoustic_rating);
                let p4 = b.add_prop_single_real("ThermalTransmittance", win.thermal_transmittance);
                let p5 = b.add_prop_single_real("SolarHeatGainCoefficient", win.solar_heat_gain_coefficient);
                let p6 = b.add_prop_single_string("OperationType", &win.operation_type);
                let p7 = b.add_prop_single_string("Tag", &win.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_WindowCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6},#{p7}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::CurtainWall(cw) => {
                let id = b.add_line(&format!(
                    "IFCCURTAINWALL('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                    cw.name, cw.curtain_wall_type_name
                ));
                let p1 = b.add_prop_single_bool("IsExternal", cw.is_external);
                let p2 = b.add_prop_single_string("FireRating", &cw.fire_rating);
                let p3 = b.add_prop_single_string("AcousticRating", &cw.acoustic_rating);
                let p4 = b.add_prop_single_real("ThermalTransmittance", cw.thermal_transmittance);
                let p5 = b.add_prop_single_string("Tag", &cw.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_CurtainWallCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::FreeformSolid(f) => {
                let cat = f.ifc_category.as_deref().or(f.group_name.as_deref()).unwrap_or("");
                match cat {
                    "Wall" => b.add_line(&format!(
                        "IFCWALL('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.SOLIDWALL.)",
                        f.name, f.name
                    )),
                    "Floor" | "Slab" => b.add_line(&format!(
                        "IFCSLAB('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.FLOOR.)",
                        f.name, f.name
                    )),
                    "Roof" => b.add_line(&format!(
                        "IFCROOF('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                        f.name, f.name
                    )),
                    "Ceiling" => b.add_line(&format!(
                        "IFCCOVERING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.CEILING.)",
                        f.name, f.name
                    )),
                    "Column" => b.add_line(&format!(
                        "IFCCOLUMN('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.COLUMN.)",
                        f.name, f.name
                    )),
                    "Beam" => b.add_line(&format!(
                        "IFCBEAM('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.BEAM.)",
                        f.name, f.name
                    )),
                    "Door" => b.add_line(&format!(
                        "IFCDOOR('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,$,$,$,$)",
                        f.name, f.name
                    )),
                    "Window" => b.add_line(&format!(
                        "IFCWINDOW('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,$,$,$,$)",
                        f.name, f.name
                    )),
                    "Stair" => b.add_line(&format!(
                        "IFCSTAIR('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                        f.name, f.name
                    )),
                    "Ramp" => b.add_line(&format!(
                        "IFCRAMP('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                        f.name, f.name
                    )),
                    "Railing" => b.add_line(&format!(
                        "IFCRAILING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.GUARDRAIL.)",
                        f.name, f.name
                    )),
                    "CurtainWall" => b.add_line(&format!(
                        "IFCCURTAINWALL('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                        f.name, f.name
                    )),
                    "Footing" => b.add_line(&format!(
                        "IFCFOOTING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.FOOTING_BEAM.)",
                        f.name, f.name
                    )),
                    "Site" => b.add_line(&format!(
                        "IFCGEOGRAPHICELEMENT('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.TERRAIN.)",
                        f.name, f.name
                    )),
                    _ => b.add_line(&format!(
                        "IFCBUILDINGELEMENTPROXY('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},$,.NOTDEFINED.)",
                        f.name, f.name
                    )),
                }
            }
            Element::Stair(s) => {
                let id = b.add_line(&format!(
                    "IFCSTAIR('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',.STRAIGHT_RUN_STAIR.)",
                    s.name, s.stair_type_name, s.tag
                ));
                let p1 = b.add_prop_single_bool("IsExternal", s.is_external);
                let p2 = b.add_prop_single_string("FireRating", &s.fire_rating);
                let p3 = b.add_prop_single_real("NumberOfRiser", s.riser_count as f64);
                let p4 = b.add_prop_single_string("Tag", &s.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_StairCommon',$,(#{p1},#{p2},#{p3},#{p4}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Ramp(r) => {
                let id = b.add_line(&format!(
                    "IFCRAMP('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',.STRAIGHT_RUN_RAMP.)",
                    r.name, r.ramp_type_name, r.tag
                ));
                let p1 = b.add_prop_single_bool("IsExternal", r.is_external);
                let p2 = b.add_prop_single_string("FireRating", &r.fire_rating);
                let p3 = b.add_prop_single_string("Tag", &r.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_RampCommon',$,(#{p1},#{p2},#{p3}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Railing(rl) => {
                let id = b.add_line(&format!(
                    "IFCRAILING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',.BALUSTRADE.)",
                    rl.name, rl.railing_type_name, rl.tag
                ));
                let p1 = b.add_prop_single_bool("IsExternal", rl.is_external);
                let p2 = b.add_prop_single_real("Height", rl.height);
                let p3 = b.add_prop_single_string("Tag", &rl.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_RailingCommon',$,(#{p1},#{p2},#{p3}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            Element::Footing(f) => {
                let id = b.add_line(&format!(
                    "IFCFOOTING('{el_guid}',#{owner_hist},'{}',$,'{}',#{el_place},#{prod_shape},'{}',.PAD_FOOTING.)",
                    f.name, f.footing_type_name, f.tag
                ));
                let p1 = b.add_prop_single_bool("LoadBearing", true);
                let p2 = b.add_prop_single_bool("IsExternal", f.is_external);
                let p3 = b.add_prop_single_real("SoilBearingCapacity", f.soil_bearing_capacity);
                let p4 = b.add_prop_single_string("ConcreteGrade", &f.concrete_grade);
                let p5 = b.add_prop_single_real("LoadBearingCapacity", f.load_bearing_capacity);
                let p6 = b.add_prop_single_string("Tag", &f.tag);
                let pset_guid = uuid_to_ifc_guid(Uuid::new_v4());
                let pset_id = b.add_line(&format!(
                    "IFCPROPERTYSET('{pset_guid}',#{owner_hist},'Pset_FootingCommon',$,(#{p1},#{p2},#{p3},#{p4},#{p5},#{p6}))"
                ));
                let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
                b.add_line(&format!(
                    "IFCRELDEFINESBYPROPERTIES('{rel_guid}',#{owner_hist},$,$,(#{id}),#{pset_id})"
                ));
                id
            }
            _ => continue,
        };

        el_to_entity.insert(*el_id, elem_entity_id);

        storey_contained_elements
            .entry(target_storey)
            .or_default()
            .push(elem_entity_id);
    }

    // Link Openings to their hosted Doors/Windows with IFCRELFILLSELEMENT
    for (void_id, fill_el_id) in void_to_fill {
        if let Some(fill_entity_id) = el_to_entity.get(&fill_el_id) {
            let rel_fill_guid = uuid_to_ifc_guid(Uuid::new_v4());
            b.add_line(&format!(
                "IFCRELFILLSELEMENT('{rel_fill_guid}',#{owner_hist},$,$,#{void_id},#{fill_entity_id})"
            ));
        }
    }

    // Link Elements to Storeys with IfcRelContainedInSpatialStructure
    for (storey_id, elem_ids) in storey_contained_elements {
        if !elem_ids.is_empty() {
            let rel_guid = uuid_to_ifc_guid(Uuid::new_v4());
            let ids_str = elem_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            b.add_line(&format!(
                "IFCRELCONTAINEDINSPATIALSTRUCTURE('{rel_guid}',#{owner_hist},'SpatialStructure',$,({ids_str}),#{storey_id})"
            ));
        }
    }

    // Compose final STEP physical file with ISO header and footer
    format!(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n\
         FILE_NAME('model.ifc','2026-10-30T00:00:00',('Architect'),('Arch3D'),'Arch3D IFC Engine 1.0','Arch3D','');\n\
         FILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\n\
         DATA;\n\
         {}\
         ENDSEC;\n\
         END-ISO-10303-21;\n",
        b.buffer
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_sample_building() {
        let doc = Document::create_sample_building();
        let step = export_ifc4(&doc);
        assert!(step.contains("FILE_SCHEMA(('IFC4'));"));
        assert!(step.contains("ReferenceView_V1.2"));
        assert!(step.contains("IFCPROJECT"));
        assert!(step.contains("IFCBUILDINGSTOREY"));
        assert!(step.contains("IFCWALL"));
        assert!(step.contains("IFCOPENINGELEMENT"));
        assert!(step.contains("IFCRELVOIDSELEMENT"));
        assert!(step.contains("IFCRELFILLSELEMENT"));
        assert!(step.contains("IFCCOLUMN"));
        assert!(step.contains("IFCSLAB"));
        assert!(step.contains("IFCDOOR"));
        assert!(step.contains("IFCWINDOW"));
        assert!(step.contains("Pset_WallCommon"));
        assert!(step.contains("Pset_ColumnCommon"));
        assert!(step.contains("Pset_BeamCommon"));
        assert!(step.contains("Pset_SlabCommon"));
        assert!(step.contains("IFCRELDEFINESBYPROPERTIES"));
        assert!(step.contains("END-ISO-10303-21;"));
        // Geometry must be exported as closed B-Rep shells or single swept solids bound to the Body sub-context
        assert!(step.contains("IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body'"));
        assert!(step.contains("IFCEXTRUDEDAREASOLID"));
        assert!(step.contains("IFCPOLYLINE"));
        assert!(!step.contains("IFCPOLYLOOP((#1,"));
    }

    #[test]
    fn test_export_roof_as_faceted_brep() {
        let mut doc = Document::create_sample_building();
        let lvl = *doc.levels.keys().next().unwrap();
        let roof = plinth_model::Roof::new(
            vec![
                plinth_geom::vec2(0.0, 0.0),
                plinth_geom::vec2(12000.0, 0.0),
                plinth_geom::vec2(12000.0, 8000.0),
                plinth_geom::vec2(0.0, 8000.0),
            ],
            lvl,
        );
        doc.elements.insert(roof.id, Element::Roof(roof));
        let step = export_ifc4(&doc);
        assert!(step.contains("IFCFACETEDBREP"));
        assert!(step.contains("IFCCLOSEDSHELL"));
        assert!(step.contains("IFCFACEOUTERBOUND"));
        assert!(step.contains("'Body','Brep'"));
        assert!(step.contains("IFCROOF"));
    }
}
