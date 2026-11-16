//! GPU Hardware Depth-Buffered Viewport Renderer for Arch3D
//!
//! Uses OpenGL via `glow` and `egui_glow` to render 3D BIM models with true
//! 24/32-bit hardware depth testing (`GL_DEPTH_TEST`).
//! Eliminates all 2D polygon sorting ambiguities, inverted mullions, and transparent surfaces.

use std::sync::Arc;
use glow::HasContext;
use plinth_geom::{Face, Vec3
};
use plinth_model::{Camera3D, Document, Element
};
use plinth_views::VisualStyle;

/// 3D Vertex uploaded to the GPU
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],

}

/// 4x4 Matrix for GPU projection and view transforms (column-major)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub fn identity() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    
}

    pub fn mul(&self, other: &Mat4) -> Mat4 {
        let a = &self.0;
        let b = &other.0;
        let mut r = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                r[col * 4 + row] =
                    a[0 * 4 + row] * b[col * 4 + 0] +
                    a[1 * 4 + row] * b[col * 4 + 1] +
                    a[2 * 4 + row] * b[col * 4 + 2] +
                    a[3 * 4 + row] * b[col * 4 + 3];
            
}
        
}
        Mat4(r)
    
}

    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalized();
        let s = f.cross(up).normalized();
        let u = s.cross(f);

        let fx = f.x as f32;
        let fy = f.y as f32;
        let fz = f.z as f32;
        let sx = s.x as f32;
        let sy = s.y as f32;
        let sz = s.z as f32;
        let ux = u.x as f32;
        let uy = u.y as f32;
        let uz = u.z as f32;

        let ex = eye.x as f32;
        let ey = eye.y as f32;
        let ez = eye.z as f32;

        Mat4([
            sx, ux, -fx, 0.0,
            sy, uy, -fy, 0.0,
            sz, uz, -fz, 0.0,
            -(sx * ex + sy * ey + sz * ez),
            -(ux * ex + uy * ey + uz * ez),
            fx * ex + fy * ey + fz * ez,
            1.0,
        ])
    
}

    pub fn perspective(fov_y_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        let f = 1.0 / (fov_y_rad * 0.5).tan();
        let nf = 1.0 / (z_near - z_far);
        Mat4([
            f / aspect, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (z_far + z_near) * nf, -1.0,
            0.0, 0.0, 2.0 * z_far * z_near * nf, 0.0,
        ])
    
}

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let rl = 1.0 / (right - left);
        let tb = 1.0 / (top - bottom);
        let fn_inv = 1.0 / (near - far);
        Mat4([
            2.0 * rl, 0.0, 0.0, 0.0,
            0.0, 2.0 * tb, 0.0, 0.0,
            0.0, 0.0, 2.0 * fn_inv, 0.0,
            -(right + left) * rl,
            -(top + bottom) * tb,
            (far + near) * fn_inv,
            1.0,
        ])
    
}

}

/// Extracted 3D scene data ready for GPU rendering
#[derive(Default)]
pub struct GpuScene {
    pub opaque_vertices: Vec<GpuVertex>,
    pub opaque_indices: Vec<u32>,
    pub transparent_vertices: Vec<GpuVertex>,
    pub transparent_indices: Vec<u32>,
    pub edge_vertices: Vec<GpuVertex>,
    pub edge_indices: Vec<u32>,

}

impl GpuScene {
    pub fn is_empty(&self) -> bool {
        self.opaque_indices.is_empty() && self.transparent_indices.is_empty() && self.edge_indices.is_empty()
    
}

}

/// Builds the complete 3D scene geometry from `Document` using the active MaterialPalette
pub fn build_gpu_scene(
    doc: &Document,
    _visual_style: VisualStyle,
    is_dark: bool,
    materials: &plinth_views::MaterialPalette,
    selected_ids: &[plinth_model::ElementId],
) -> GpuScene {
    let mut scene = GpuScene::default();

    let sel_set: std::collections::HashSet<plinth_model::ElementId> = selected_ids.iter().copied().collect();

    let edge_color = if is_dark {
        [0.85f32, 0.88, 0.92, 1.0]
    } else {
        [0.18f32, 0.20, 0.24, 1.0]
    };
    let sel_edge_color = if is_dark {
        [0.0f32, 0.92, 1.0, 1.0]
    } else {
        [0.0f32, 0.50, 1.0, 1.0]
    };
    let sel_fill_color = if is_dark {
        [0.0f32, 0.70, 1.0, 0.35]
    } else {
        [0.0f32, 0.50, 1.0, 0.25]
    };

    let to_f32_rgb = |c: [u8; 3]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0];
    let to_f32_rgba = |c: [u8; 4]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0];

    let wall_color = to_f32_rgb(materials.wall);
    let slab_color = to_f32_rgb(materials.slab);
    let roof_color = to_f32_rgb(materials.roof);
    let ceiling_color = to_f32_rgb(materials.ceiling);
    let col_color = to_f32_rgb(materials.column);
    let beam_color = to_f32_rgb(materials.beam);
    let door_color = to_f32_rgb(materials.door);
    let win_glass_color = to_f32_rgba(materials.window_glass);
    let stair_ramp_color = to_f32_rgb(materials.stair_ramp);
    let railing_color = to_f32_rgb(materials.railing);
    let curtain_mullion_color = to_f32_rgb(materials.curtain_frame);
    let curtain_glass_color = to_f32_rgba(materials.curtain_glass);
    let footing_color = to_f32_rgb(materials.footing);
    let site_color = to_f32_rgb(materials.site);
    let freeform_default_color = to_f32_rgb(materials.freeform);

    let push_face = |scene: &mut GpuScene, face: &Face, color: [f32; 4], is_transparent: bool, is_selected: bool| {
        let n = face.pts.len();
        if n < 3 {
            return;
        }

        let norm = [face.normal.x as f32, face.normal.y as f32, face.normal.z as f32];

        // Ear-clipping triangulation for arbitrary 3D faces (eliminates starburst fan rays and flaps across notches/voids)
        let mut tri_indices: Vec<[usize; 3]> = Vec::new();
        if n == 3 {
            tri_indices.push([0, 1, 2]);
        } else {
            let norm_v = if face.normal.length() > 1e-4 {
                face.normal.normalized()
            } else {
                plinth_geom::prism::newell_normal(&face.pts)
            };
            let u = if norm_v.z.abs() < 0.9 && (norm_v.x.abs() > 1e-4 || norm_v.y.abs() > 1e-4) {
                plinth_geom::Vec3::new(-norm_v.y, norm_v.x, 0.0).normalized()
            } else {
                plinth_geom::Vec3::new(1.0, 0.0, 0.0)
            };
            let v = norm_v.cross(u).normalized();
            let pts_2d: Vec<plinth_geom::Vec2> = face
                .pts
                .iter()
                .map(|p| plinth_geom::vec2(p.dot(u), p.dot(v)))
                .collect();

            let tris = plinth_geom::polygon::triangulate(&pts_2d);
            if !tris.is_empty() {
                tri_indices = tris;
            } else if n == 4 && plinth_geom::polygon::is_convex(&pts_2d) {
                tri_indices.push([0, 1, 2]);
                tri_indices.push([0, 2, 3]);
            }
        }

        // 1. Solid Triangles
        if is_transparent {
            let base_idx = scene.transparent_vertices.len() as u32;
            let final_color = if is_selected { sel_fill_color } else { color };
            for p in &face.pts {
                scene.transparent_vertices.push(GpuVertex {
                    pos: [p.x as f32, p.y as f32, p.z as f32],
                    normal: norm,
                    color: final_color,
                });
            }
            for t in &tri_indices {
                scene.transparent_indices.push(base_idx + t[0] as u32);
                scene.transparent_indices.push(base_idx + t[1] as u32);
                scene.transparent_indices.push(base_idx + t[2] as u32);
            }
        } else {
            let base_idx = scene.opaque_vertices.len() as u32;
            for p in &face.pts {
                scene.opaque_vertices.push(GpuVertex {
                    pos: [p.x as f32, p.y as f32, p.z as f32],
                    normal: norm,
                    color,
                });
            }
            for t in &tri_indices {
                scene.opaque_indices.push(base_idx + t[0] as u32);
                scene.opaque_indices.push(base_idx + t[1] as u32);
                scene.opaque_indices.push(base_idx + t[2] as u32);
            }

            // If selected, also add translucent depth-tested highlight fill into transparent pass
            if is_selected {
                let trans_base_idx = scene.transparent_vertices.len() as u32;
                for p in &face.pts {
                    scene.transparent_vertices.push(GpuVertex {
                        pos: [p.x as f32, p.y as f32, p.z as f32],
                        normal: norm,
                        color: sel_fill_color,
                    });
                }
                for t in &tri_indices {
                    scene.transparent_indices.push(trans_base_idx + t[0] as u32);
                    scene.transparent_indices.push(trans_base_idx + t[1] as u32);
                    scene.transparent_indices.push(trans_base_idx + t[2] as u32);
                }
            }
        }

        // 2. CAD Outlines / Hard Edges
        // Always emitted (including Wireframe mode, where the solid passes are skipped
        // and the hard edges of every element form the wireframe).
        {
            let e_color = if is_selected { sel_edge_color } else { edge_color };
            for i in 0..n {
                if face.hard.get(i).copied().unwrap_or(true) {
                    let p0 = face.pts[i];
                    let p1 = face.pts[(i + 1) % n];
                    let base_e = scene.edge_vertices.len() as u32;
                    scene.edge_vertices.push(GpuVertex {
                        pos: [p0.x as f32, p0.y as f32, p0.z as f32],
                        normal: norm,
                        color: e_color,
                    });
                    scene.edge_vertices.push(GpuVertex {
                        pos: [p1.x as f32, p1.y as f32, p1.z as f32],
                        normal: norm,
                        color: e_color,
                    });
                    scene.edge_indices.push(base_e);
                    scene.edge_indices.push(base_e + 1);
                }
            }
        }
    };

    for (id, el) in &doc.elements {
        let is_selected = sel_set.contains(id);
        match el {
            Element::CurtainWall(cw) => {
                let base_z = doc.level_elevation(cw.base_level);
                let top_z = cw.top_level.map(|lvl| doc.level_elevation(lvl));
                let (z0, z1) = cw.z_bounds(base_z, top_z);
                let len = cw.length();
                let h = z1 - z0;
                if len < 10.0 || h < 10.0 {
                    continue;
                }

                // Mullions and Transoms
                for prism in cw.solids(base_z, top_z) {
                    let is_glass = (prism.z_max - prism.z_min) > cw.mullion_width * 1.5
                        && prism.profile.len() == 4
                        && (prism.profile[1] - prism.profile[0]).length() > cw.mullion_width * 1.5;

                    for face in prism.faces() {
                        if is_glass {
                            push_face(&mut scene, &face, curtain_glass_color, true, is_selected);
                        } else {
                            push_face(&mut scene, &face, curtain_mullion_color, false, is_selected);
                        }
                    }
                }
            }
            Element::Window(_win) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, win_glass_color, true, is_selected);
                }
            }
            Element::Door(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, door_color, false, is_selected);
                }
            }
            Element::Wall(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, wall_color, false, is_selected);
                }
            }
            Element::Column(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, col_color, false, is_selected);
                }
            }
            Element::Beam(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, beam_color, false, is_selected);
                }
            }
            Element::Slab(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, slab_color, false, is_selected);
                }
            }
            Element::Roof(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, roof_color, false, is_selected);
                }
            }
            Element::Ceiling(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, ceiling_color, false, is_selected);
                }
            }
            Element::Stair(_) | Element::Ramp(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, stair_ramp_color, false, is_selected);
                }
            }
            Element::Railing(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, railing_color, false, is_selected);
                }
            }
            Element::Footing(_) => {
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, footing_color, false, is_selected);
                }
            }
            Element::FreeformSolid(f) => {
                if f.group_name.as_deref() == Some("Space")
                    || f.ifc_category.as_deref() == Some("IFCSPACE")
                    || f.group_name.as_deref() == Some("Site")
                    || f.ifc_category.as_deref() == Some("IFCSITE")
                {
                    continue;
                }
                let solid_color = match f.ifc_category.as_deref().or(f.group_name.as_deref()) {
                    Some("Wall") => wall_color,
                    Some("Floor") | Some("Slab") => slab_color,
                    Some("Roof") => roof_color,
                    Some("Ceiling") => ceiling_color,
                    Some("Column") => col_color,
                    Some("Beam") => beam_color,
                    Some("Door") => door_color,
                    Some("Window") => win_glass_color,
                    Some("Stair") | Some("Ramp") => stair_ramp_color,
                    Some("Railing") => railing_color,
                    Some("CurtainWall") => curtain_glass_color,
                    Some("Footing") => footing_color,
                    Some("Site") => site_color,
                    _ => freeform_default_color,
                };
                for face in f.faces() {
                    push_face(&mut scene, &face, solid_color, false, is_selected);
                }
            }
            Element::FreeformSurface(s) => {
                let surf_color = if s.is_hole {
                    [0.95f32, 0.40, 0.20, 0.55] // Translucent orange-red void cutout indicator
                } else {
                    match s.ifc_category.as_deref() {
                        Some("Wall") => wall_color,
                        Some("Floor") | Some("Slab") => slab_color,
                        Some("Roof") => roof_color,
                        Some("Ceiling") => ceiling_color,
                        Some("Column") => col_color,
                        Some("Beam") => beam_color,
                        Some("Door") => door_color,
                        Some("Window") => win_glass_color,
                        Some("Stair") | Some("Ramp") => stair_ramp_color,
                        Some("Railing") => railing_color,
                        Some("CurtainWall") => curtain_glass_color,
                        Some("Footing") => footing_color,
                        Some("Site") => site_color,
                        _ => freeform_default_color,
                    }
                };
                for face in s.faces() {
                    push_face(&mut scene, &face, surf_color, s.is_hole, is_selected);
                }
            }
            _ => {
                let default_color = [0.78f32, 0.80, 0.84, 1.0];
                for face in doc.element_faces(*id) {
                    push_face(&mut scene, &face, default_color, false, is_selected);
                }
            }
        }
    }

    scene

}

/// GPU Hardware Viewport Renderer holding GL state
pub struct GpuViewportRenderer {
    program: glow::Program,
    line_program: glow::Program,
    u_view_proj: glow::UniformLocation,
    u_light_dir: glow::UniformLocation,
    u_visual_style: glow::UniformLocation,
    u_wire_color: glow::UniformLocation,
    u_line_view_proj: glow::UniformLocation,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ibo: glow::Buffer,

}

impl GpuViewportRenderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        unsafe {
            // 1. Shaders for 3D Solid Triangles
            let vs_src = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec4 a_color;

uniform mat4 u_view_proj;

out vec3 v_normal;
out vec4 v_color;

void main() {
    v_normal = a_normal;
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_pos, 1.0);

}
"#;

            let fs_src = r#"#version 330 core
in vec3 v_normal;
in vec4 v_color;

uniform vec3 u_light_dir;
uniform int u_visual_style;
uniform vec4 u_wire_color;

out vec4 frag_color;

void main() {
    if (u_visual_style == 1) {
        frag_color = v_color;
    } else if (u_visual_style == 2) {
        frag_color = vec4(1.0, 1.0, 1.0, v_color.a);
    } else if (u_visual_style == 3) {
        frag_color = u_wire_color;
    } else {
        float diff = max(dot(normalize(v_normal), u_light_dir), 0.0);
        float ambient = 0.42;
        float intensity = clamp(ambient + (1.0 - ambient) * diff, 0.25, 1.0);
        frag_color = vec4(v_color.rgb * intensity, v_color.a);
    }
}
"#;

            let vs_src_es = r#"#version 300 es
precision highp float;
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec4 a_color;

uniform mat4 u_view_proj;

out vec3 v_normal;
out vec4 v_color;

void main() {
    v_normal = a_normal;
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_pos, 1.0);
}
"#;

            let fs_src_es = r#"#version 300 es
precision highp float;
precision highp int;
in vec3 v_normal;
in vec4 v_color;

uniform vec3 u_light_dir;
uniform int u_visual_style;
uniform vec4 u_wire_color;

out vec4 frag_color;

void main() {
    if (u_visual_style == 1) {
        frag_color = v_color;
    } else if (u_visual_style == 2) {
        frag_color = vec4(1.0, 1.0, 1.0, v_color.a);
    } else if (u_visual_style == 3) {
        frag_color = u_wire_color;
    } else {
        float diff = max(dot(normalize(v_normal), u_light_dir), 0.0);
        float ambient = 0.42;
        float intensity = clamp(ambient + (1.0 - ambient) * diff, 0.25, 1.0);
        frag_color = vec4(v_color.rgb * intensity, v_color.a);
    }
}
"#;

            let program = create_shader_program(gl, vs_src, fs_src)
                .or_else(|_| create_shader_program(gl, vs_src_es, fs_src_es))?;
            let u_view_proj = gl.get_uniform_location(program, "u_view_proj")
                .ok_or_else(|| "Missing u_view_proj".to_string())?;
            let u_light_dir = gl.get_uniform_location(program, "u_light_dir")
                .ok_or_else(|| "Missing u_light_dir".to_string())?;
            let u_visual_style = gl.get_uniform_location(program, "u_visual_style")
                .ok_or_else(|| "Missing u_visual_style".to_string())?;
            let u_wire_color = gl.get_uniform_location(program, "u_wire_color")
                .ok_or_else(|| "Missing u_wire_color".to_string())?;

            // 2. Shaders for CAD Outline Lines
            let vs_line_src = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
layout(location = 2) in vec4 a_color;

uniform mat4 u_view_proj;
out vec4 v_color;

void main() {
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_pos, 1.0);

}
"#;

            let fs_line_src = r#"#version 330 core
in vec4 v_color;
out vec4 frag_color;

void main() {
    frag_color = v_color;
}
"#;

            let vs_line_src_es = r#"#version 300 es
precision highp float;
layout(location = 0) in vec3 a_pos;
layout(location = 2) in vec4 a_color;

uniform mat4 u_view_proj;
out vec4 v_color;

void main() {
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_pos, 1.0);
}
"#;

            let fs_line_src_es = r#"#version 300 es
precision highp float;
in vec4 v_color;
out vec4 frag_color;

void main() {
    frag_color = v_color;
}
"#;

            let line_program = create_shader_program(gl, vs_line_src, fs_line_src)
                .or_else(|_| create_shader_program(gl, vs_line_src_es, fs_line_src_es))?;
            let u_line_view_proj = gl.get_uniform_location(line_program, "u_view_proj")
                .ok_or_else(|| "Missing u_view_proj on line program".to_string())?;

            let vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
            let vbo = gl.create_buffer().map_err(|e| e.to_string())?;
            let ibo = gl.create_buffer().map_err(|e| e.to_string())?;

            Ok(Self {
                program,
                line_program,
                u_view_proj,
                u_light_dir,
                u_visual_style,
                u_wire_color,
                u_line_view_proj,
                vao,
                vbo,
                ibo,
            
})
        
}
    
}

    /// Renders the 3D scene using true hardware depth buffering into the specified egui viewport rect
    pub fn render_viewport(
        &self,
        rect: egui::Rect,
        cam: &Camera3D,
        zoom: f32,
        pan: egui::Vec2,
        scene: &GpuScene,
        visual_style: VisualStyle,
        is_dark: bool,
    ) -> egui::PaintCallback {
        let (view_proj, light_dir) = compute_matrices(rect, cam, zoom, pan);
        let style_int = match visual_style {
            VisualStyle::Shaded => 0,
            VisualStyle::ConsistentColors => 1,
            VisualStyle::HiddenLine => 2,
            VisualStyle::Wireframe => 3,
        
};

        // Flatten vertex and index buffers for Pass 1 (Opaque), Pass 2 (Edges), Pass 3 (Glass)
        let opaque_v_bytes = vertex_slice_bytes(&scene.opaque_vertices);
        let opaque_i_bytes = index_slice_bytes(&scene.opaque_indices);
        let opaque_i_count = scene.opaque_indices.len() as i32;

        let edge_v_bytes = vertex_slice_bytes(&scene.edge_vertices);
        let edge_i_bytes = index_slice_bytes(&scene.edge_indices);
        let edge_i_count = scene.edge_indices.len() as i32;

        let trans_v_bytes = vertex_slice_bytes(&scene.transparent_vertices);
        let trans_i_bytes = index_slice_bytes(&scene.transparent_indices);
        let trans_i_count = scene.transparent_indices.len() as i32;

        let program = self.program;
        let line_program = self.line_program;
        let u_view_proj = self.u_view_proj.clone();
        let u_light_dir = self.u_light_dir.clone();
        let u_visual_style = self.u_visual_style.clone();
        let u_wire_color = self.u_wire_color.clone();
        let u_line_view_proj = self.u_line_view_proj.clone();
        let vao = self.vao;
        let vbo = self.vbo;
        let ibo = self.ibo;

        let clear_color = if is_dark {
            [0.11f32, 0.12, 0.15, 1.0]
        
} else {
            [0.96f32, 0.97, 0.98, 1.0]
        
};

        let wire_color = if is_dark {
            [0.85f32, 0.88, 0.92, 1.0]
        
} else {
            [0.18f32, 0.20, 0.24, 1.0]
        
};

        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            let gl = painter.gl();
            unsafe {
                let ppp = info.pixels_per_point;
                let screen_h = info.screen_size_px[1] as f32;
                let x = (rect.min.x * ppp).round() as i32;
                let width = ((rect.max.x - rect.min.x) * ppp).round() as i32;
                let height = ((rect.max.y - rect.min.y) * ppp).round() as i32;
                let y = (screen_h - (rect.max.y * ppp)).round() as i32;

                if width <= 0 || height <= 0 {
                    return;
                
}

                // 1. Scissor and Viewport setup to isolate to this viewport pane
                gl.viewport(x, y, width, height);
                gl.scissor(x, y, width, height);
                gl.enable(glow::SCISSOR_TEST);

                // 2. Hardware Depth Buffer Clear and Setup
                gl.enable(glow::DEPTH_TEST);
                gl.depth_func(glow::LEQUAL);
                gl.depth_mask(true);
                gl.clear_color(clear_color[0], clear_color[1], clear_color[2], clear_color[3]);
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));

                let stride = std::mem::size_of::<GpuVertex>() as i32;

                // 3. PASS 1: Opaque Solids with Hardware Z-Buffer (Skipped in Wireframe to avoid diagonal triangulation chords)
                if visual_style != VisualStyle::Wireframe && opaque_i_count > 0 {
                    gl.use_program(Some(program));
                    gl.uniform_matrix_4_f32_slice(Some(&u_view_proj), false, &view_proj.0);
                    gl.uniform_3_f32(Some(&u_light_dir), light_dir[0], light_dir[1], light_dir[2]);
                    gl.uniform_1_i32(Some(&u_visual_style), style_int);
                    gl.uniform_4_f32(Some(&u_wire_color), wire_color[0], wire_color[1], wire_color[2], wire_color[3]);

                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &opaque_v_bytes, glow::DYNAMIC_DRAW);
                    gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &opaque_i_bytes, glow::DYNAMIC_DRAW);

                    // Vertex attribute pointers: 0=pos, 1=normal, 2=color
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

                    gl.enable_vertex_attrib_array(1);
                    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

                    gl.enable_vertex_attrib_array(2);
                    gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);

                    // Offset polygons slightly back so CAD lines sit cleanly on top
                    gl.enable(glow::POLYGON_OFFSET_FILL);
                    gl.polygon_offset(1.0, 1.0);

                    gl.draw_elements(glow::TRIANGLES, opaque_i_count, glow::UNSIGNED_INT, 0);

                    gl.disable(glow::POLYGON_OFFSET_FILL);
                }

                // 4. PASS 2: CAD Edge Lines / Silhouettes
                // In Wireframe mode, this pass draws all true architectural hard perimeter edges
                // with depth testing disabled, completely free of internal triangulation diagonals!
                if edge_i_count > 0 {
                    gl.use_program(Some(line_program));
                    gl.uniform_matrix_4_f32_slice(Some(&u_line_view_proj), false, &view_proj.0);

                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &edge_v_bytes, glow::DYNAMIC_DRAW);
                    gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &edge_i_bytes, glow::DYNAMIC_DRAW);

                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

                    gl.enable_vertex_attrib_array(2);
                    gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);

                    if visual_style == VisualStyle::Wireframe {
                        gl.disable(glow::DEPTH_TEST);
                    }

                    gl.draw_elements(glow::LINES, edge_i_count, glow::UNSIGNED_INT, 0);

                    if visual_style == VisualStyle::Wireframe {
                        gl.enable(glow::DEPTH_TEST);
                    }
                }

                // 5. PASS 3: Translucent Glazing (Glass panels) with Alpha Blending (Skipped in Wireframe)
                if visual_style != VisualStyle::Wireframe && trans_i_count > 0 {
                    gl.use_program(Some(program));
                    gl.uniform_matrix_4_f32_slice(Some(&u_view_proj), false, &view_proj.0);
                    gl.uniform_3_f32(Some(&u_light_dir), light_dir[0], light_dir[1], light_dir[2]);
                    gl.uniform_1_i32(Some(&u_visual_style), style_int);
                    gl.uniform_4_f32(Some(&u_wire_color), wire_color[0], wire_color[1], wire_color[2], wire_color[3]);

                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &trans_v_bytes, glow::DYNAMIC_DRAW);
                    gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &trans_i_bytes, glow::DYNAMIC_DRAW);

                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

                    gl.enable_vertex_attrib_array(1);
                    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

                    gl.enable_vertex_attrib_array(2);
                    gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);

                    // Read depth to test against opaque walls and mullions, but don't occlude other elements
                    gl.depth_mask(false);
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

                    gl.draw_elements(glow::TRIANGLES, trans_i_count, glow::UNSIGNED_INT, 0);

                    gl.disable(glow::BLEND);
                    gl.depth_mask(true);
                }

                // 6. Restore OpenGL state for egui
                gl.bind_vertex_array(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
                gl.use_program(None);

                gl.disable(glow::DEPTH_TEST);
                gl.disable(glow::SCISSOR_TEST);
            
}
        
});

        egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        
}
    
}

}

fn compute_matrices(
    rect: egui::Rect,
    cam: &Camera3D,
    zoom: f32,
    pan: egui::Vec2,
) -> (Mat4, [f32; 3]) {
    let width = (rect.max.x - rect.min.x).max(1.0);
    let height = (rect.max.y - rect.min.y).max(1.0);

    let (sin_y, cos_y) = (cam.yaw as f32).sin_cos();
    let (sin_p, cos_p) = (cam.pitch as f32).sin_cos();

    let tx = cam.target_x as f32;
    let ty = cam.target_y as f32;
    let tz = cam.target_z as f32;

    let r00 = cos_y;
    let r01 = -sin_y;
    let r02 = 0.0f32;
    let r03 = -(tx * cos_y - ty * sin_y);

    let r10 = sin_y * sin_p;
    let r11 = cos_y * sin_p;
    let r12 = cos_p;
    let r13 = -(tx * sin_y * sin_p + ty * cos_y * sin_p + tz * cos_p);

    let r20 = sin_y * cos_p;
    let r21 = cos_y * cos_p;
    let r22 = -sin_p;
    let r23 = -(tx * sin_y * cos_p + ty * cos_y * cos_p - tz * sin_p);

    let (view, proj) = if cam.fov_degrees > 1.0 {
        let cam_dist = (cam.distance as f32).max(1000.0);
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let scale_x = (cam_dist * zoom) / half_w;
        let scale_y = (cam_dist * zoom) / half_h;
        let pan_x_ndc = pan.x / half_w;
        let pan_y_ndc = pan.y / half_h;

        let z_near = 100.0f32;
        let z_far = 2000000.0f32;
        let a_z = (z_far + z_near) / (z_far - z_near);
        let b_z = -2.0 * z_far * z_near / (z_far - z_near);

        let r23_persp = r23 + cam_dist;
        let view = Mat4([
            r00, r10, r20, 0.0,
            r01, r11, r21, 0.0,
            r02, r12, r22, 0.0,
            r03, r13, r23_persp, 1.0,
        ]);

        let proj = Mat4([
            scale_x, 0.0, 0.0, 0.0,
            0.0, scale_y, 0.0, 0.0,
            pan_x_ndc, -pan_y_ndc, a_z, 1.0,
            0.0, 0.0, b_z, 0.0,
        ]);

        (view, proj)
    
} else {
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let sx = zoom / half_w;
        let sy = zoom / half_h;
        let sz = 1.0 / 500000.0f32;

        let tx_ndc = pan.x / half_w;
        let ty_ndc = -pan.y / half_h;

        let view = Mat4([
            r00, r10, r20, 0.0,
            r01, r11, r21, 0.0,
            r02, r12, r22, 0.0,
            r03, r13, r23, 1.0,
        ]);

        let proj = Mat4([
            sx, 0.0, 0.0, 0.0,
            0.0, sy, 0.0, 0.0,
            0.0, 0.0, sz, 0.0,
            tx_ndc, ty_ndc, 0.0, 1.0,
        ]);

        (view, proj)
    
};

    let view_proj = proj.mul(&view);
    let light_dir = [0.5f32, -0.6, 0.8];
    let len = (light_dir[0] * light_dir[0] + light_dir[1] * light_dir[1] + light_dir[2] * light_dir[2]).sqrt();
    let light_norm = [light_dir[0] / len, light_dir[1] / len, light_dir[2] / len];

    (view_proj, light_norm)

}

fn vertex_slice_bytes(verts: &[GpuVertex]) -> Vec<u8> {
    let byte_len = verts.len() * std::mem::size_of::<GpuVertex>();
    let mut bytes = vec![0u8; byte_len];
    if !verts.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(
                verts.as_ptr() as *const u8,
                bytes.as_mut_ptr(),
                byte_len,
            );
        
}
    
}
    bytes

}

fn index_slice_bytes(indices: &[u32]) -> Vec<u8> {
    let byte_len = indices.len() * std::mem::size_of::<u32>();
    let mut bytes = vec![0u8; byte_len];
    if !indices.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(
                indices.as_ptr() as *const u8,
                bytes.as_mut_ptr(),
                byte_len,
            );
        
}
    
}
    bytes

}

unsafe fn create_shader_program(
    gl: &glow::Context,
    vs_src: &str,
    fs_src: &str,
) -> Result<glow::Program, String> {
    let program = gl.create_program().map_err(|e| e.to_string())?;

    let vs = gl.create_shader(glow::VERTEX_SHADER).map_err(|e| e.to_string())?;
    gl.shader_source(vs, vs_src);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        let log = gl.get_shader_info_log(vs);
        return Err(format!("Vertex shader compile error: {log
}"));
    
}

    let fs = gl.create_shader(glow::FRAGMENT_SHADER).map_err(|e| e.to_string())?;
    gl.shader_source(fs, fs_src);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        let log = gl.get_shader_info_log(fs);
        return Err(format!("Fragment shader compile error: {log
}"));
    
}

    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);

    if !gl.get_program_link_status(program) {
        let log = gl.get_program_info_log(program);
        return Err(format!("Shader program link error: {log
}"));
    
}

    gl.delete_shader(vs);
    gl.delete_shader(fs);

    Ok(program)

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_gpu_scene_wireframe() {
        let doc = Document::create_sample_building();
        let palette = plinth_views::MaterialPalette::signature_pastel();
        let scene = build_gpu_scene(&doc, VisualStyle::Wireframe, false, &palette, &[]);
        println!(
            "Scene summary: opaque_indices={}, trans_indices={}, edge_indices={}, edge_verts={}",
            scene.opaque_indices.len(),
            scene.transparent_indices.len(),
            scene.edge_indices.len(),
            scene.edge_vertices.len()
        );

        let mut min_pos = [f32::INFINITY; 3];
        let mut max_pos = [f32::NEG_INFINITY; 3];
        for v in &scene.edge_vertices {
            for k in 0..3 {
                min_pos[k] = min_pos[k].min(v.pos[k]);
                max_pos[k] = max_pos[k].max(v.pos[k]);
            }
        }
        println!("Edge bounds: min={:?}, max={:?}", min_pos, max_pos);
    }

    #[test]
    fn test_compute_matrices_perspective_visibility() {
        let mut cam = Camera3D::default();
        cam.fov_degrees = 50.0;
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0));
        let (view_proj, _light) = compute_matrices(rect, &cam, 0.05, egui::Vec2::ZERO);

        // Target point
        let pt = [cam.target_x as f32, cam.target_y as f32, cam.target_z as f32, 1.0];
        let m = &view_proj.0;
        let clip_x = m[0] * pt[0] + m[4] * pt[1] + m[8] * pt[2] + m[12] * pt[3];
        let clip_y = m[1] * pt[0] + m[5] * pt[1] + m[9] * pt[2] + m[13] * pt[3];
        let clip_z = m[2] * pt[0] + m[6] * pt[1] + m[10] * pt[2] + m[14] * pt[3];
        let clip_w = m[3] * pt[0] + m[7] * pt[1] + m[11] * pt[2] + m[15] * pt[3];

        assert!(clip_w > 100.0, "Clip W must be positive and equal to distance from camera eye, got {}", clip_w);
        let ndc_x = clip_x / clip_w;
        let ndc_y = clip_y / clip_w;
        let ndc_z = clip_z / clip_w;

        assert!(ndc_x.abs() < 1e-4, "Target point NDC X should be 0, got {}", ndc_x);
        assert!(ndc_y.abs() < 1e-4, "Target point NDC Y should be 0, got {}", ndc_y);
        assert!(ndc_z >= -1.0 && ndc_z <= 1.0, "Target point NDC Z must be within [-1, 1], got {}", ndc_z);
    }

    #[test]
    fn test_gpu_scene_selection_highlight() {
        let doc = Document::create_sample_building();
        let palette = plinth_views::MaterialPalette::signature_pastel();
        let wall_id = doc.elements.iter().find_map(|(&id, el)| {
            if matches!(el, Element::Wall(_)) { Some(id) } else { None }
        }).expect("Sample building must have a wall");

        // 1. Unselected scene
        let scene_unsel = build_gpu_scene(&doc, VisualStyle::Shaded, true, &palette, &[]);

        // 2. Scene with wall selected
        let scene_sel = build_gpu_scene(&doc, VisualStyle::Shaded, true, &palette, &[wall_id]);

        // When selected:
        // - Edge vertices should contain the vibrant selection color [0.0, 0.92, 1.0, 1.0]
        let has_sel_edges = scene_sel.edge_vertices.iter().any(|v| {
            (v.color[0] - 0.0).abs() < 0.05 && (v.color[1] - 0.92).abs() < 0.05 && (v.color[2] - 1.0).abs() < 0.05
        });
        assert!(has_sel_edges, "Selected element must emit edges with selection color in GPU scene");

        // - Unselected scene must NOT have selection edges
        let unsel_has_sel_edges = scene_unsel.edge_vertices.iter().any(|v| {
            (v.color[0] - 0.0).abs() < 0.05 && (v.color[1] - 0.92).abs() < 0.05 && (v.color[2] - 1.0).abs() < 0.05
        });
        assert!(!unsel_has_sel_edges, "Unselected scene must not have selection edges");

        // - Selected element emits translucent highlight triangles in transparent pass
        assert!(scene_sel.transparent_indices.len() > scene_unsel.transparent_indices.len(),
            "Selected solid element must emit translucent highlight faces into transparent pass");
    }
}
