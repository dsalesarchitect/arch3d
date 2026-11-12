# Arch3D — Master Engineering Handover Notes

**Project**: Arch3D  
**Location**: `d:\Arch3D`  
**Target**: `x86_64-pc-windows-gnu`  
**Date**: November 12, 2026  
**Status**: 100% Passing Tests (107/107 unit & integration tests passed across all crates, 0 errors)  
**Live Site**: https://dsalesarchitect.github.io/arch3d/  
**Web Studio**: https://dsalesarchitect.github.io/arch3d/app.html  
**Repository**: https://github.com/dsalesarchitect/arch3d  

---

## 0. Critical Directory Isolation Rule (Read Before Doing Anything)

> [!IMPORTANT]
> - **Arch3D Root Directory**: Arch3D is located EXCLUSIVELY in `D:\Arch3D`.
> - **Do NOT touch or pollute `D:\mymusicapp`**: That folder is an independent Python/PyTorch AI music app. Never place Arch3D files, cargo builds, or code into `D:\mymusicapp`.
> - **Legacy folders are OBSOLETE**: `D:\plinth-bim` and `D:\mymusicapp\plinth-bim` are deprecated/removed.
> - **Execution Rule**: Always run build, test, and run commands with working directory `D:\Arch3D` (e.g. `cd 'D:\Arch3D'; cargo test --workspace`).

---

## 1. System Architecture & Crates Overview

Arch3D is a native BIM (Building Information Modeling) CAD platform written in pure clean-room Rust. All dimensions and coordinates operate at **1:1 millimeter real-world scale** ($1\text{ m} = 1000\text{ mm}$), adhering to ISO 10303-21 IFC4 STEP standards.

```
d:\Arch3D/
├── crates/
│   ├── plinth-geom/     # Pure 2D/3D computational geometry, Newell normals, polygon clipping, ear-clip triangulation
│   ├── plinth-model/    # Core BIM document, parametric elements (Wall, Slab, Roof, Stair, Ramp, Railing, etc.), DWG & SKP importers
│   ├── plinth-views/    # Multi-viewport projection, 3D isometric & perspective, visual styles (Shaded, Hidden Line, Consistent, Wireframe)
│   ├── plinth-ifc/      # IFC4 / IFC2X3 STEP physical file parser and serializer
│   └── plinth-app/      # egui GUI application, multi-pane canvas, procedural vector icons, properties & settings inspectors
└── AGENTS.md            # Golden architectural rules & engineering standards
```

---

## 2. Recent Major Implementations & Bug Fixes

### A. SketchUp (.skp) Native Importer & Exploding Geometry Elimination
*Implementation*: [`crates/plinth-model/src/skp.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/skp.rs), [`crates/plinth-model/src/material.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/material.rs), [`crates/plinth-model/src/element.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/element.rs)

1. **Root Cause Analysis of Exploding Geometry & Starburst Rays**:
   - **Subtree Pruning Bug on Hidden Nodes**: SketchUp models use helper bounding boxes, guide geometry, or reference instances with `node.hidden = true`. Previously, `visit_node` did not prune `node.children`, causing untransformed or reference objects to leak into world space.
   - **Warped Non-Planar Quads**: Organic curved meshes contain thousands of 4-vertex quadrilaterals whose vertices are not coplanar. When passed into `egui::Shape::convex_polygon`, these warped polygons projected as non-convex or self-intersecting butterflies.
   - **Axis-Aligned Foreshortening on N-Gons**: Triangulating N-gons by projecting to arbitrary axis planes compressed oblique faces by $\sqrt{2}$, flipping triangle orientations and causing ear-clipping to fail.
   - **Wireframe Spiderweb Contours**: Internal diagonals and softened curved mesh edges were all given `hard = true`, causing white wireframe webs over organic surfaces.

2. **The Engineering Fixes Applied**:
   - **WYSIWYG Subtree Pruning**: In `visit_node`, any node with `node.hidden == true` immediately returns, pruning its entire descendant subtree.
   - **Quad Decomposition Along Shortest Diagonal**: Every 4-vertex quad is decomposed into two triangles along $\min(d_{02}, d_{13})$. The internal diagonal chord is strictly assigned `hard = false`.
   - **Exact Orthonormal Tangent Frame `(U, V)`**: N-gons are projected into a local orthonormal basis $(U, V)$ perpendicular to the true Newell normal before ear-clipping, preserving 100% isometry, angle, and area.
   - **Soft/Smooth Edge Filtering**: Edges are verified against `mesh.edges`. Only true crease edges get `hard = true`; internal chords and soft/smooth edges get `hard = false`.
   - **Consecutive Vertex Deduplication**: Consecutive vertices within 0.1 mm tolerance are deduplicated, and vertex indices are validated against mesh boundaries.
   - **Group & Component Hierarchy Preservation**: `FreeformSolid` now stores `group_name` and `is_group`, which are displayed in the Properties inspector.
   - **Material & Embedded Texture Extraction**: `Material` now preserves `texture_name` and embedded binary `texture_bytes` (PNG/JPEG) from `openskp::Material::Textured`.

---

### B. AutoCAD (.dwg & .dxf) Native Drawing Importer
*Implementation*: [`crates/plinth-model/src/dwg.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/dwg.rs), [`crates/plinth-app/src/icons.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/icons.rs)

- **Pure-Rust `acadrust` Integration**: Zero external C/C++ or LibreDWG dependencies (`acadrust = "0.6.8"`).
- **Unit Scaling**: Evaluates `$INSUNITS` and `$MEASUREMENT` (Inches: $\times 25.4$, Feet: $\times 304.8$, Meters: $\times 1000.0$, Metric standard: $\times 1.0$).
- **Intelligent BIM Layer Mapping**:
  - `WALL*`, `PAREDE*`, `MURO*` layers automatically generate native parametric `Element::Wall` (200 mm thickness, 3000 mm height).
  - `SLAB*`, `FLOOR*`, `PISO*`, `LAJE*` layers generate native parametric `Element::Slab`.
- **Drafting & Freeform Mesh Geometry**:
  - Explodes `EntityType::Insert` block references into world-space CAD entities.
  - Converts polylines with arc bulges ($b = \tan(\theta/4)$), circles, and arcs into `Element::DetailLine`.
  - Converts 3D faces (`Face3D`), polyface meshes, and solids into `Element::FreeformSolid`.
- **Procedural Vector Icon**: `IconKind::ImportDwg` in Ribbon Home & Insert tabs.

---

### C. Multi-Viewport Layouts & Dedicated Per-Viewport Visual Styles
*Implementation*: [`crates/plinth-app/src/canvas.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/canvas.rs), [`crates/plinth-views/src/scene.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-views/src/scene.rs)

- **Independent Multi-Pane Splits**:
  - 5 Viewport Layouts: `Single`, `SplitHorizontal`, `SplitVertical`, `SplitTriple`, `SplitQuad`.
  - Each viewport slot maintains its own `ViewId`, `pan`, `zoom`, and `visual_style`.
- **Minimalist Single Icon Visual Style Pill**:
  - Positioned at the bottom-right of each viewport as a compact 26×26 px pill showing `☀️`, `📐`, `🎨`, or `🕸️`.
  - Clicking opens a clean dropdown to switch between Shaded, Hidden Line, Consistent Colors, and Wireframe per pane.
- **Floating Viewport Header Badges**:
  - Positioned at the top-left of each viewport with view kind icon (`📐`, `💡`, `🧊`, `🏛`, `✂`, `🔍`), active accent border, and view picker dropdown.
- **Strict Scissor GPU Clipping**:
  - Enforced via `painter.with_clip_rect(slot.rect)` to guarantee geometry never bleeds across pane boundaries.

---

### D. Architectural Ribbon Compacting & Circulation Tools
*Implementation*: [`crates/plinth-app/src/ribbon.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/ribbon.rs), [`crates/plinth-app/src/settings.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/settings.rs)

- **Snug Tool Spacing**: Reduced tool button padding and gaps from 4 px down to 2 px for a dense, professional CAD layout.
- **Icon-Only Mode**:
  - User can toggle **Tool Button Labels** in Settings (`Appearance -> Ribbon Tool Style: Show labels / Icon only`).
  - When labels are hidden, ribbon height shrinks from 78 px to 44 px, maximizing drawing canvas workspace.
- **Circulation Tools**:
  - Added **Stair** (`IconKind::Stair`), **Ramp** (`IconKind::Ramp`), and **Railing** (`IconKind::Railing`) to Architecture ribbon tab and parametric data model.

---

### E. Universal Level-Hosting & Elevation Offsets
*Implementation*: [`crates/plinth-model/src/element.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/element.rs), [`crates/plinth-app/src/properties.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/properties.rs)

- Added **Base Level** dropdown, **Base Offset** (`mm`), **Top Constraint / Top Level** dropdown (`"Unconnected"` + all project levels), and **Top Offset** (`mm`) across all 13 core element types (`Wall`, `Door`, `Window`, `Column`, `Beam`, `Slab`, `Roof`, `Stair`, `Ramp`, `Railing`, `Fascia`, `Gutter`, `Ceiling`).
- Strict vertical bounds resolution via `z_bounds(base_z, top_z)`.

---

### F. Type & Assembly Configuration Studio (`tool_config.rs`)
*Implementation*: [`crates/plinth-app/src/tool_config.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/tool_config.rs), [`crates/plinth-app/src/ribbon.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/ribbon.rs)

- Double-clicking any of the 17 Architecture or Structure Ribbon tools opens `ToolConfigStudio`.
- Features curated Type Libraries, **Duplicate Type** / custom type creation, multi-layer compound assembly stacks (`AssemblyLayer`, `LayerFunction`) with live thermal $U$-value & thickness sync, procedural `egui::Painter` cross-section/ergonomics blueprints, default level hosting/offsets, and IFC4 `Pset_*Common` defaults.

---

### G. Free Sheet Viewport Movement & Corner Resizing
*Implementation*: [`crates/plinth-model/src/document.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/document.rs), [`crates/plinth-app/src/canvas.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/canvas.rs), [`crates/plinth-app/src/properties.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/properties.rs)

- Placed viewports (`PlacedViewport`) on Sheet Layout desks can be selected, freely dragged anywhere on the sheet paper (`position_mm`), and resized via 4 corner grips (`width_mm`, `height_mm`) with live mm coordinate badges and Undo/Redo snapshots.

---

### H. Level Manager & Grid Manager Dialogs
*Implementation*: [`crates/plinth-app/src/level_manager.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/level_manager.rs)

- **Manage Tab -> Level Manager**:
  - Storey elevation schedule sorted top-to-bottom, inline elevation & name editing.
  - `+ Add Level Above`, `+ Add Basement Below`, `⚡ Generate Multi-Storey Stack`.
  - Live datum section diagram with Z=0 ground line, safe deletion with automatic element reassignment to nearest remaining level.
- **Manage Tab -> Grid Manager**:
  - Orthogonal bay generator with custom spacings (`"6000, 7500, 6000"`, `"3*6000, 4500"`).
  - Automatic alphanumeric tags (X=A,B,C; Y=1,2,3), `⇄ Align Grid Tails`, and live 2D grid blueprint preview.

---

### I. Automated Structural Framing Pipeline & Foundation Footings
*Implementation*: [`crates/plinth-app/src/structure_auto.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/structure_auto.rs), [`crates/plinth-model/src/element.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/element.rs)

1. **Footing Tool (`Tool::DrawFooting`)**:
   - Parametric isolated foundation footing (`FootingStyle::Pad`, `Stepped`, `Sloped`) with `Pset_FootingCommon`, column hosting link, and watertight 3D solid hulls.
2. **4 Structural Automation Tools in Structure Tab**:
   - **Place Columns on Grids**: Automatically identifies grid intersections and places structural columns.
   - **Place Foundation Footings on Columns**: Automatically generates isolated footings underneath columns aligned with column center and rotation.
   - **Place Beams on Grids / Columns**: Frames adjacent columns and grid nodes bay-by-bay along X and Y axes.
   - **Place Slabs on Closed Column-Beam Bays**: Uses Planar Half-Edge Face Extraction to detect minimal CCW closed bays and places structural slabs aligned to beam inner faces.

---

### J. Multi-Storey Entity Replication & Smart Constraint Remapping
*Implementation*: [`crates/plinth-app/src/level_copy.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/level_copy.rs)

- `CopyActiveLevelDialog`: Interactive Source Level dropdown (`source_level_id`), selective category filter, and `☑ Include Storey Framing Beams & Deck Slabs (Ceiling Framing)` option.
- **Storey Element Resolution (`is_storey_element`)**: Accurately captures both base-hosted elements (`relative_storey = 0`) and upper-framing elements (`relative_storey = 1`, structural framing beams, deck slabs, ceilings at the storey's ceiling level $Z_{\text{top}}$).
- **Relative Storey Elevation Remapping**: `paste_clipboard_to_levels` utilizes `resolve_target_level_with_offset` to replicate beams and floor deck slabs across every destination storey without missing framing.
- `PasteToLevelsDialog`: Destination storey multi-selector with live clipboard contents breakdown (`Contents: Columns: N, Beams: N, Floors: N`), Select All Other Storeys, and automatic `ElementId`, `host_wall`, `host_column`, and `top_level` storey constraint remapping.

---

### K. Column Section Projection, Slab Thickness & Elevation/Section Selection
*Implementation*: [`crates/plinth-views/src/section.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-views/src/section.rs), [`crates/plinth-model/src/element.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/element.rs), [`crates/plinth-app/src/canvas.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/canvas.rs), [`crates/plinth-app/src/structure_auto.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-app/src/structure_auto.rs)

1. **Columns Visible in Building Section Views**:
   - Slices directly on the cut plane are rendered with thick cut lines (`width = 2.0`) and column cut fill.
   - In addition, all columns, beams, footings, and walls beyond the section cut line in the viewing direction `v_dir = vec2(-u_dir.y, u_dir.x)` (within 30m view depth) are projected onto the section plane $(t, z)$ and sorted back-to-front behind the cut elements. Columns are 100% visible across all sections.
2. **Slab Strict Thickness & Elevation/Section Bounds**:
   - Fixed `Slab::z_bounds`: A slab's top face is aligned to $Z_{\text{top}}$ (either top level or base level + offset), and its bottom face is strictly $Z_{\text{top}} - \text{thickness}$. Slabs are always their exact specified thickness (e.g. 200 mm), eliminating vertical 3500mm storey-spanning monoliths.
3. **Exact Elevation and Section Element Selection & Highlights**:
   - Hit testing in Elevation views maps click coordinates to $(u, z)$ and tests against the element's actual projected bounds $[u_{\min}, u_{\max}] \times [z_0, z_1]$. Slabs only match if the click occurs within their actual 200mm thickness slice, preventing slabs from hijacking clicks across the entire storey.
   - Selection highlights in Elevation and Section views project the element's exact 2D facade rectangle.
4. **Slab Corner Trimming & Centroid Depth Sorting**:
   - In `align_bay_to_beam_inner_surfaces`, corners supported by columns wider than beams are notched along the column inner vertical faces $(W_{col}/2, D_{col}/2)$, preventing slabs from encroaching on column top caps.
   - In `threed.rs`, structural floor slabs and bay slabs evaluate depth at their centroid (`transform_to_view(centroid).y`), preventing slabs from sorting on top of framing beams during 3D orbit.

---

### L. Watertight 6-Sided Solid B-Rep Invariant, Beam Top Surface & Accurate Curtain Wall Elevation
*Implementation*: [`crates/plinth-views/src/threed.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-views/src/threed.rs), [`crates/plinth-views/src/elevation.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-views/src/elevation.rs), [`crates/plinth-model/src/element.rs`](file:///d:/mymusicapp/plinth-bim/crates/plinth-model/src/element.rs)

1. **Universal Watertight 6-Sided Solid B-Rep Invariant**:
   - Every extrusion, mullion, transom, beam, pier, and column box in 3D MUST emit ALL 6 watertight solid faces: Front (`+norm`), Back (`-norm`), Top Cap (`+Z`), Bottom Cap (`-Z`), Left End (`-dir`), and Right End (`+dir`).
   - In `emit_mullion_box!`, added the missing left end face (`-dir`), right end face (`+dir`), and unconditioned top (`+Z`) and bottom (`-Z`) caps for ALL intermediate transoms and vertical mullions. Orbiting in 3D now displays solid, opaque, dark metallic profiles from all angles with zero hollow tube cutaways or transparent voids.
   - In `CurtainWall::solids` (`plinth-model`), added intermediate horizontal transoms (`1..n_h`) alongside rails and mullions, unifying B-Rep face extraction, IFC STEP export, and section cuts under a single source of truth.
2. **Beam Top Flange & Soffit Camera-Closest Depth Anchoring**:
   - Replaced arbitrary edge midpoint depth calculation with camera-closest corner vertex anchoring:
     $$\text{depth} = \min_{i} (Y_{\text{corner}_i}) - 1.0\text{ mm}$$
     (for top flange and bottom soffit) and $\min_i (Y_{\text{corner}_i})$ for vertical web faces.
   - Eliminates the Painter's algorithm depth sorting failure where long beam spans (6000 mm) were sorted thousands of millimeters behind adjacent geometry, which caused the beam top surface to appear transparent or white.
3. **Curtain Wall Elevation Geometry Accuracy**:
   - Replaced generic single hairline projection with accurate 2D solid rectangles matching the 3D grid 1:1:
     - Outer boundary vertical jamb mullions (`i = 0`, `i = n_v`) and intermediate mullions (`i = 1..n_v`) with `mullion_width` (50 mm).
     - Bottom sill rail, top head rail, and intermediate horizontal transoms (`j = 1..n_h`) with `mullion_width` (50 mm).
     - Individual recessed glazed glass panels inside every grid cell with translucent glass fill and crisp frame strokes.
   - The Elevation view now displays identical architectural storefront geometry as the 3D isometric view.

### M. Permanent GPU Hardware Depth-Buffered Viewport Renderer (`gpu_viewport.rs`, `glow`, `egui_glow`)
- **Root Problem Solved**: Replaced the 2D CPU Painter's algorithm with true GPU hardware depth testing, completely eliminating all sorting ambiguities, inverted mullions, and transparent beam/slab surfaces across any 360° orbit angle.
- **Hardware Z-Buffer & Anti-Aliasing**: Configured `eframe::NativeOptions` with `depth_buffer: 24` and `multisampling: 4` (4x hardware MSAA).
- **3-Pass Architecture**:
  1. *Pass 1 (Opaque Solids)*: Walls, slabs, beams, columns, mullions, roofs, stairs, ramps, and footings write depth with `GL_POLYGON_OFFSET_FILL` (offset 1.0, 1.0).
  2. *Pass 2 (CAD Silhouette & Hard Edges)*: Drawn with `GL_LINES` in theme line color; because of the polygon offset, lines sit cleanly on outer faces without z-fighting.
  3. *Pass 3 (Translucent Glazing)*: Glass panels render with `GL_BLEND` and `depth_mask(false)`, accurately reading depth from opaque walls/mullions without blocking other geometry.
- **Viewport Scissor & 2D Overlays**: GPU render is clipped strictly to viewport panes via `gl.scissor`, and 2D overlays (snapping guides, ViewCube, gizmos, dimension badges) render smoothly on top via `egui::Painter`.
- **Zero Test Regressions**: All 66 workspace tests continue to pass 100%.

---

### H. 3D Wall Corners, Opening Clean Cutouts & 3D Object Selection (Nov 1, 2026)
- **3D Wall Corner Visible Lines**: `Wall::boundary_faces` and `Wall::build_solids_with_joins` maintain building outer corner vertices as crisp hard edges (`hard = true`, `soft = false`), ensuring vertical corner lines at building miters from $Z_0$ to $Z_1$ are 100% visible across 3D visual styles.
- **Window & Door Monolithic Cutouts (Zero Vertical Seams)**: `build_gpu_scene` utilizes `doc.element_faces` (`Wall::boundary_faces`), rendering wall pier faces, sills, and lintels as seamless monolithic surfaces. The vertical edges between coplanar sill/lintel quads and adjacent piers are suppressed (`hard = false`), eliminating all vertical dividing lines above and below window openings. Vertical jamb lines are strictly confined to the opening reveal bounds $[Z_{\text{bot}}, Z_{\text{top}}]$.
- **Interactive 3D Object Click Selection & Visual Highlight**: In `canvas.rs`, click-selection in 3D views projects face geometry directly to screen space using `project_3d(p)` and evaluates both `point_in_polygon` and a 6-pixel fuzzy hit tolerance on outline edges (`closest_point_on_segment <= 6.0px`), selecting the frontmost camera-facing element under the cursor. Selected elements in 3D highlight with a translucent blue fill and 2.5px cyan hard outlines, immediately updating their full parameter schedule in the Properties Inspector.
- **Primary Click Reliability**: Fixed pointer tracking during mouse-up/click events (`pointer.interact_pos().or(hover_pos).or(latest_pos)`), ensuring primary clicks on the 3D viewport canvas reliably register without being dropped by high-DPI mouse drag deltas.

### M. Sheet Layout Viewport White Paper Theme, Monolithic Rendering & Branding Integration (Nov 5, 2026)
*Implementation*: [`crates/plinth-app/src/canvas.rs`](file:///d:/Arch3D/crates/plinth-app/src/canvas.rs), [`crates/plinth-app/src/icons.rs`](file:///d:/Arch3D/crates/plinth-app/src/icons.rs), [`crates/plinth-geom/src/polygon.rs`](file:///d:/Arch3D/crates/plinth-geom/src/polygon.rs)

1. **Root Cause Analysis of Giant Screen-Splitting Wedges and Sheet Layout 3D Background Defect**:
   - **Sutherland-Hodgman Concave Distortion**: Sutherland-Hodgman polygon clipping is mathematically strictly limited to convex polygons. In `draw_polygon_safe`, applying it to multi-wing concave slabs (such as 38-vertex apartment highrise floor plates or roof decks extending across viewport boundaries) connected non-adjacent entry and exit points along the rectangular clipping boundary. When ear-clipping triangulated this distorted boundary, giant triangles fanned across the viewport and connected to the corners, filling the entire viewport rectangle with the slab's gray color (`[175, 177, 173, 255]`) and creating giant diagonal wedge flaps.
   - **Sheet Layout Theme Inversion**: `render_sheet_layout_canvas` previously used `ColorTheme::dark()` if the app UI was in Dark Mode. On a physical white paper drawing sheet, this rendered viewport backgrounds with dark colors and inverted linework to white.
   - **Unclipped Line Segments on Sheets**: Placed viewports on sheets drew lines with raw `line_segment` without Liang-Barsky line clipping.

2. **The Engineering Fixes Applied**:
   - **Monolithic Safe Polygon Rendering & Indexed Mesh Engine (`draw_polygon_safe`)**:
     1. Trivial rejection: Skips polygons whose vertices all lie strictly outside the clip rect bounds with a 50px margin.
     2. Numerical sanity check: Rejects non-finite coordinates (`NaN`, `Inf`).
     3. Monolithic convex polygon rendering: All convex shapes (triangles, quads, convex n-gons) are emitted as single monolithic shapes via `egui::Shape::convex_polygon` with zero internal seams (Rule 1).
     4. Seamless Indexed Mesh Triangulation for Concave Slabs: Non-convex shapes (L-shaped, U-shaped, 38-vertex multi-wing highrise floor plates and roof slabs) are triangulated into a single indexed `egui::Mesh` with shared vertices (`mesh.colored_vertex`) and indexed triangles (`mesh.add_triangle`), then emitted via `painter.add(egui::Shape::mesh(mesh))`. This completely eliminates internal sub-pixel anti-aliasing seam lines across complex slabs, while GPU hardware scissor clipping (`gl.scissor` via `painter.with_clip_rect`) cleanly bounds geometry at viewport edges without CPU polygon corruption or corner wedge flaps.
     5. Border linework: Outer contour strokes are cleanly emitted via `draw_line_segment_safe` along genuine boundary edges only.
   - **Paper White Viewport Theme**: Placed viewports on Sheet Layout desks now strictly use `ColorTheme::light()`, ensuring 3D viewports on paper sheets always render on clean, crisp white paper.
   - **Liang-Barsky Line Safe Drawing on Sheets**: Sheet viewports draw lines with `draw_line_segment_safe` to prevent unbounded coordinate projection.
   - **Process Lock Management & Binary Rebuild**: Resolved file lock from previous running instances (PID 220, PID 22476) and cleanly linked `target/debug/arch3d.exe`.
   - **Official Neon Violet & Obsidian 3D Cube Branding**: Application icon is synchronized from the official `Arch3D logo.png` into 128x128 RGBA window icon (`arch3d_icon_128.bin`), Windows desktop icon (`arch3d.ico`), and web favicon (`docs/favicon.ico`). Landing page theme matches this palette with electric neon violet accents (`#d946ef`, `#a855f7`), obsidian slate dark backgrounds (`#090a0f`, `#11131a`), pure SVG stroke icons (zero emojis), screenshot visual gallery, creator profile for Dsales Architect (`dsalesarchitect@gmail.com`, Philippines), and community sponsorship channels.

---

### E. GNU GPL-3.0 Licensing with Commercial Plugin Exception & Complete Rebranding
- **License Invariant**: Licensed under the GNU General Public License v3.0 or later (`license = "GPL-3.0-or-later"`) with the **Arch3D Commercial Plugin Exception** (GNU GPLv3 Section 7). The core application is protected copyleft, while plugins, scripts, and add-ons can be distributed and monetized under proprietary terms without requiring plugin source release. Master terms are consolidated in `LICENSE`.
- **Clean Nomenclature**: Complete elimination of legacy "Plinth BIM" references from GUI, status bars, IFC writer metadata, settings dialogs, documentation, and web assets. Obsolete launcher Launch-Plinth-BIM.bat removed in favor of Launch-Arch3D.bat.

### F. Live GitHub Repository & GitHub Pages Publishing
- **Repository Setup**: Initialized and configured official GitHub repository at [https://github.com/dsalesarchitect/arch3d](https://github.com/dsalesarchitect/arch3d) with GitHub CLI authentication under user dsalesarchitect.
- **GitHub Pages Web Studio**: Automated web deployment from main branch folder /docs. Web studio accessible at [https://dsalesarchitect.github.io/arch3d/app.html](https://dsalesarchitect.github.io/arch3d/app.html) with zero-install WebAssembly in-browser BIM authoring.

### G. Universal Move, Rotate, and Mirror Tool Snapping & Free 3D Transformation
- **Full Viewport Coverage**: Tool::Move, Tool::Rotate, and Tool::Mirror execute uniformly across all 2D plan views, ceiling plans, elevations, building sections, and 3D views.
- **Universal Snapping**: Real-time magnetic snapping to all architectural and structural vertices, midpoints, and segment edges within 18 px screen-space tolerance.
- **Axis Locking & HUD**: Single-key axis constraints (X, Y, Z), orthogonal shift-lock, translation distance badge, bullseye center with degree sweep pie, and symmetry mirror line.

### H. Embedded Real-World Sample Project (Commercial_Office_Building (1).arch3d)
- **Embedded Gzip Model**: Document::create_sample_building() in [crates/plinth-model/src/document.rs](file:///d:/Arch3D/crates/plinth-model/src/document.rs) embeds and decompresses sample_project.arch3d.gz (~24.9 KB) representing the 10-storey apartment highrise model (450 elements, 10 levels, 24 views, layout sheet A101) on application startup (desktop and WebAssembly).
- **Public Download**: Published as [docs/sample_project.arch3d](file:///d:/Arch3D/docs/sample_project.arch3d) for direct file exchange testing.
- **Test Isolation**: Created Document::create_test_box_building() for unit tests requiring fixed rectangular envelope coordinates, ensuring complete independence from sample model geometry updates.

### I. Landing Page Workspace Interface Gallery & High-Res Showcase
- **High-Res Studio Screenshots**: Updated landing page gallery in [docs/index.html](file:///d:/Arch3D/docs/index.html) with authentic high-resolution captures from Arch3D:
  - docs/images/arch3d-ui-main.png: Full Arch3D studio application window showcasing ribbons, project browser hierarchy across all 10 storeys, 3D viewport, and properties inspector.
  - docs/images/arch3d-viewport-split.png: Focused 3D viewport canvas with interactive 3D ViewCube navigation widget.
  - docs/images/arch3d-multi-storey.jpg: 10-storey apartment highrise model with faceted balconies and project tree.
  - docs/images/arch3d-framing.png: Automated structural framing and substructure pipeline.
- **Refined Showcase Copy**: Updated cards for Unified Architectural & Structural Studio, Hardware-Accelerated 3D Navigation, Multi-Storey Facades, and Automated Framing.

---

## 3. Verification & Test Suite Summary

All 107 automated unit & integration tests across the 5 workspace crates pass with 100% success rate:

`
running 39 tests (crates/plinth-app)  ... ok (100%)
running 16 tests (crates/plinth-geom) ... ok (100%)
running 15 tests (crates/plinth-ifc)  ... ok (100%)
running 15 tests (crates/plinth-model)... ok (100%)
running 22 tests (crates/plinth-views)... ok (100%)
Total: 107 passed; 0 failed; 0 ignored (0 errors)
`


### D. Benchmark IFC Suite & MEP Systems Classification
1. **Schema Verification Audit**:
   - Analyzed all ZIP packages in `Downloads`: `Academic_Autodesk`, `FOJAB_Landsarkivet`, `KIT_Institute`, `KIT_Smiley-West`, `LTU_A-House`, `NBU_MedicalClinic`, `NVW_DCR-LOD`, `SGD_BODO`, `Hestia-main`.
   - Identified schemas: 90% are **`IFC2X3`** (Coordination View 2.0) and legacy **`IFC2X2_FINAL`** (SGD BODO); only modern fixtures in `Hestia-main` are authored in **`IFC4`**.
   - Verified that Arch3D seamlessly imports legacy `IFC2X2`, `IFC2X3`, modern `IFC4`, and infrastructure `IFC4X3`, upgrading and exporting them to standard `IFC4` STEP format.
2. **MEP Systems & Architectural Specialization**:
   - Categorized `IFCFLOWSEGMENT`, `IFCFLOWFITTING`, `IFCFLOWCONTROLLER`, `IFCFLOWTERMINAL`, `IFCENERGYCONVERSIONDEVICE`, `IFCFLOWMOVINGDEVICE`, `IFCFLOWTREATMENTDEVICE` into dedicated groups (`Duct`, `Pipe`, `MEP Fitting`, `MEP Controller`, `HVAC Terminal`, `Lighting`, `MEP Equipment`, `Treatment Device`).
   - Categorized `IFCPLATE` as `Curtain Wall Panel` or `Plate`.
   - Categorized `IFCFURNISHINGELEMENT` as `Furniture` and `IFCSANITARYTERMINAL` as `Plumbing Fixture`.
   - Supported non-uniform 3D scales (`Scale`, `Scale2`, `Scale3`) in `resolve_cartesian_transformation_3d` for `IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM`.
   - Added dedicated material pastel colors for MEP systems in `threed.rs` (galvanized zinc ductwork, technical piping blue, machinery steel, porcelain sanitary fixtures, luminaire gold).

### Build & Run Commands
- **Run App**: `cargo run --bin arch3d` or launch `Launch-Arch3D.bat`
- **Run Tests**: `cargo test --workspace`
- **Process Lock Safeguard**: Always terminate existing instances before linking on Windows (`taskkill /f /im arch3d.exe 2>$null`).

---

## 4. Key Rules for the Next Agent

1. **Watertight 6-Sided Solid B-Rep Invariant**: Every extrusion, mullion, transom, beam, pier, and column in 3D MUST emit all 6 watertight solid faces: Front (`+norm`), Back (`-norm`), Top Cap (`+Z`), Bottom Cap (`-Z`), Left End (`-dir`), and Right End (`+dir`). Never omit end caps or top/bottom caps on intermediate segments.
2. **Camera-Closest Depth Anchoring on Cap Faces**: Horizontal cap faces (beam top flanges, column tops, roof planes) must evaluate Painter's algorithm depth using their camera-closest corner vertex $\min_i(Y_i) - \text{bias}$, rather than an edge midpoint thousands of mm away.
3. **Never use `raw-dylib`**: Target `x86_64-pc-windows-gnu` requires standard GNU linking.
4. **Procedural Vector Icons Only**: All ribbon/toolbar icons must be mathematically rendered via `egui::Painter` in `icons.rs`. Never import bitmap/raster PNG icons.
5. **Monolithic Surfaces**: Never subdivide walls or slabs into grid tiles; `egui::Shape::convex_polygon` sub-pixel anti-aliasing creates gray hairline seam lines if surfaces are split.
6. **Scissor Rects**: Always wrap viewport rendering in `painter.with_clip_rect` to prevent geometry bleeding.
7. **Single Source of Truth**: Geometry and BIM parameters reside strictly in `plinth-model`, projected reactively to views.
8. **Ray-Plane Depth Picking Invariant**: In 3D viewports (`canvas.rs`), click selection evaluates exact ray-plane intersection along the view axis.
9. **Wireframe Internal Chord Suppression**: In Wireframe mode, edge loops evaluate `face.hard` rather than drawing all triangle boundaries. Internal triangulation diagonals remain completely hidden.
10. **Custom & IFC Floor/Ceiling CCW Boundary Normalization**: `Slab` and `Ceiling` boundary loops are strictly sanitized via `dedup_closed` and `make_ccw`, guaranteeing outward perimeter rim normals, $+Z$ top caps, $-Z$ bottom caps, and watertight solid hulls for complex atrium/courtyard geometry.
11. **Frustum & Marquee Selection Screen-Bounds Safety**: 3D selection highlights cull offscreen projected vertices outside `vp_rect.expand(200.0)`, and `draw_polygon_safe` only strokes borders if ear-clipping fails, eliminating infinite rays and screen-splitting wedges.
12. **Universal Annotation & Architectural Snapping Engine**: Ribbon Annotation tab provides Linear Dim, Circular Dim, Angular Dim, Text Note, and Leader Callout with customizable tick marks, typography, and live magnetic snapping to all architecture and structure vertices, cut opening jamb corners, centerlines, and edges.
13. **Landing Page, GitHub Pages & Cloudflare Deployment**: Zero-dependency landing page in `docs/` with interactive 3D HTML5 model canvas, ready for GitHub Pages (via `docs/.nojekyll` and `.github/workflows/deploy-pages.yml`) and Cloudflare Pages (via `docs/_headers`, `docs/_redirects`, and `wrangler.toml`). Fully documented in `DEPLOYMENT.md`.


---

## 5. Latest Implementations & Brand System (Session: November 12, 2026)

### J. Automatic Fit Zoom Extents on Startup & View Activation
- **Viewport Extents Calculation**: Added `CanvasState::zoom_extents_viewport()` in `crates/plinth-app/src/canvas.rs`. Computes the true 3D/2D bounding box across all 18 element categories and site elements.
- **Safety Margin & Centroid Centering**: Centers the camera at the model's centroid with a 12% safety margin (`0.76` fill factor).
- **Multi-Platform Consistency**: Automatically triggers on frame 1 and upon switching views/loading sample models, ensuring both desktop launcher and WebAssembly Web Studio open perfectly centered and fitted.

### K. Authentic Architectural Procedural Vector Icons
- **Floor Plan View Icon (`IconKind::FloorPlanView`)**: Renders an authentic floor plan diagram directly with mathematical strokes in `egui::Painter`: 3 partitioned rooms, door leaf, and 90° swing arc.
- **Section View Icon (`IconKind::SectionView`)**: Renders an authentic building section cut with stepped building envelope, floor slabs, dashed vertical cutting plane line, and directional viewing arrow.
- **Zero Raster Artifacts**: Maintains Arch3D's strict cleanroom procedural vector standard across all UI scales.

### L. Professional 3-Color Architectural Brand Identity & Zero Neon Glow
- **Neon Glow Removal**: Eliminated all wide 30–35px glowing box-shadow halos, neon pink/magenta gradients, and glowing radial background bursts across `docs/styles.css`, `docs/web_app.css`, and `docs/index.html`.
- **Architectural 3-Color Palette**:
  1. *Precision CAD Blueprint Blue* (`#2563eb` / `#1d4ed8` primary brand accent).
  2. *Steel Slate / Cyan* (`#0ea5e9` / `#38bdf8` secondary accent).
  3. *Architectural Obsidian & Slate Neutrals* (`#0b0f19`, `#111827`, `#1f2937` dark surfaces, crisp `#f8fafc` text; Light mode: `#ffffff` canvas with `#0f172a` slate text).
- **Subtle Architectural Elevation**: Replaced neon shadows with crisp CAD drop shadows (`0 1px 3px rgba(0,0,0,0.2)` and `0 10px 25px -5px rgba(0,0,0,0.4)`).

### M. Professional Typography & Official Brand Badges in GitHub Documentation
- **Clean Section Headers**: Removed cartoon emojis from headings in `README.md` and `CONTRIBUTING.md`, adopting clean architectural typography.
- **Flat-Square SVG Badges**: Integrated official vector logos for Rust, GPLv3, IFC4, WebAssembly, GitHub Sponsors, PayPal, and YouTube.
- **Typographic Readiness Matrix**: Transformed emoji bullet lists into structured, technical readiness tiers (`Active Baseline`, `Core Engine`, `Planned (Plugin)`, `Future Lifecycle`).

### N. Strict Cleanroom Architecture & Anti-Proprietary Screenshot Framework
- **Gated Issue Forms**: Configured `.github/ISSUE_TEMPLATE/` (`bug_report.yml`, `feature_request.yml`) with required cleanroom sign-off checkboxes.
- **Blank Issues Disabled**: `.github/ISSUE_TEMPLATE/config.yml` prevents unformatted issues that bypass compliance.
- **Anti-Proprietary Screenshot Rule**: Explicitly prohibits uploading screenshots, videos, or UI captures of commercial proprietary software (Revit, ArchiCAD, AutoCAD, MicroStation, etc.).
- **Immediate Scrubbing Policy**: Documented in `CONTRIBUTING.md` that any issue or PR containing proprietary commercial software captures will be closed and deleted on sight.
- **Pull Request Certification**: `.github/PULL_REQUEST_TEMPLATE.md` mandates contributor certification of 100% original, non-decompiled code and zero proprietary assets.

### O. Funding, Sponsorship & Windows SmartScreen Setup
- **Active PayPal Donation Checkout**: Custom donation checkout link `https://www.paypal.com/ncp/payment/PXBTVNQLUWF66` integrated across `.github/FUNDING.yml`, `README.md`, and `docs/index.html`.
- **GitHub Sponsors Onboarding**: `https://github.com/sponsors/dsalesarchitect` configured and pending GitHub review.
- **Microsoft Defender SmartScreen Notice**: Added standard unsigned open-source notice and 2-step click-through (*"More info"* -> *"Run anyway"*) in `README.md` and `docs/index.html`.

### P. Workspace Test Suite Status
All **109 tests** across all 5 workspace crates pass with 100% success rate:
- `plinth-app`: 41 passed
- `plinth-geom`: 16 passed
- `plinth-ifc`: 15 passed
- `plinth-model`: 15 passed
- `plinth-views`: 22 passed
- Total: 109 passed; 0 failed; 0 ignored (0 errors)
