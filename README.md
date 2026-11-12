# Arch3D

<div align="center">

**A cleanroom, modern BIM authoring application powered by a high-performance pure Rust engine.**

[![Rust 1.81+](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Schema: IFC4](https://img.shields.io/badge/IFC-4_ISO_10303--21-brightgreen.svg)](https://technical.buildingsmart.org/standards/ifc/)
[![Web Studio](https://img.shields.io/badge/Web_Studio-Live_on_GitHub_Pages-cyan.svg)](https://dsalesarchitect.github.io/arch3d/)

</div>

---

## 🌟 Highlights

Arch3D offers a streamlined, developer-friendly architectural and structural modeler that delivers the core power of modern BIM authoring with the speed, memory safety, and simplicity of Rust.

- **Rust Engine**: Modular, dependency-light analytical geometry kernel and spatial document model.
- **Modern Ribbon Interface**: Intuitive top ribbon organized into **Architecture**, **Structure**, **Site**, **Free Modeling**, **Annotation**, **Modify**, **Manage**, and **Insert** panels.
- **Single Source of Truth**: All views—**Floor Plans**, **Reflected Ceiling Plans (RCP)**, **Building Elevations (N, S, E, W)**, **Building Sections**, **3D Axonometric/Perspective**, and **Detail Callouts**—are dynamically projected in real-time from a single central 3D building model.
- **Standard 1:1 Scale**: Every element is modeled at real-world scale in millimetres (`mm`), with customizable view scale representation (1:1 up to 1:1000).
- **ISO 10303-21 IFC4 Standard File Format**: Full export and import support for buildingSMART IFC4 schema (`IfcProject`, `IfcSite`, `IfcBuilding`, `IfcBuildingStorey`, `IfcWall`, `IfcColumn`, `IfcBeam`, `IfcSlab`, `IfcCovering`, `IfcDoor`, `IfcWindow`, `IfcCurtainWall`, `IfcRoof`, `IfcStair`, `IfcRamp`, `IfcRailing`, `IfcFooting`, `IfcExtrudedAreaSolid`, `IfcRelAggregates`, and `IfcRelContainedInSpatialStructure`).
- **CAD & Mesh Ingestion**: Reads `.dwg`/`.dxf` via `acadrust` and SketchUp `.skp` via `openskp` without proprietary SDKs.
- **Hardware GPU Viewport**: Direct OpenGL hardware depth-buffered rendering (`glow`) with 24-bit Z-buffer and 4x MSAA.
- **Live Sync & Interoperability**: glTF 2.0 Binary (`.glb`) export, WebGPU path tracing studio (`@meimu/mutracer`), and Blender Live Sync bidirectional socket bridge.
- **Zero-Install Web Studio**: Full WebAssembly client compiled with `wasm-bindgen` and deployable directly to GitHub Pages.

---

## 🏛 Workspace Architecture

```text
arch3d/
├── crates/
│   ├── plinth-geom/     # Geometry kernel: 2D/3D vectors, AABBs, affine transforms,
│   │                    # ear-clipping polygon triangulation, prism solids, clipping.
│   │
│   ├── plinth-model/    # Central Single Source of Truth BIM model:
│   │                    # Elements (Wall, Column, Beam, Slab, Ceiling, Door, Window,
│   │                    # Dimension, DetailLine), Levels, Grids, Materials, Undo/Redo.
│   │
│   ├── plinth-views/    # Multi-view projection engine:
│   │                    # Floor Plan (cut plane at 1200mm, door swing arcs, glazing),
│   │                    # Reflected Ceiling Plan (RCP tile grid), Elevations (N/S/E/W),
│   │                    # Section cuts (solid slicing & hatching), 3D shaded view, Details.
│   │
│   ├── plinth-ifc/      # Standards-compliant IFC4 STEP physical file exporter and parser
│   │                    # (ISO 10303-21) with buildingSMART 22-char GUID compression.
│   │
│   └── plinth-app/      # Interactive desktop application (egui / eframe):
│                        # Ribbon bar, Project Browser tree, Properties inspector,
│                        # interactive viewport canvas with grid snapping and tool state machine.
```

---

## 🎛 Ribbon Tools

### 1. Architecture
- **Wall**: Architectural partition and exterior walls with real-time thickness and length feedback.
- **Door**: Hosted single-flush door with 90° swing arc, opening cutouts, and flip options.
- **Window**: Hosted fixed/casement window with sill lines, jambs, and glazing centerlines.
- **Column**: Architectural column with rectangular or circular profile.
- **Floor**: Floor slabs with polygonal boundary editing and layer thickness.
- **Ceiling**: Suspended acoustic tile and gypsum ceiling systems with customizable tile grids.

### 2. Structure
- **Structural Column**: Load-bearing concrete and steel column framing.
- **Beam**: Structural framing beams with start/end elevations and depth profiles.
- **Structural Wall**: Reinforced shear and foundation walls.
- **Structural Slab**: Ground foundation and suspended structural floor slabs.

### 3. Draw & Annotate
- **Detail Line**: 2D drafting lines (Thin, Medium, Wide, Hidden).
- **Dimension**: Aligned linear dimensions with witness lines, slash ticks, and measured text.
- **Grid**: Datum grid lines with circular bubble tags (1, 2, 3... / A, B, C...).

### 4. Modify
- **Select**: Pick elements to inspect or edit.
- **Move**: Two-point base and destination translation.
- **Rotate**: Interactive center and angle rotation.
- **Mirror**: Axis reflection across any two points.
- **Delete**: Instant deletion of selected elements with full Undo/Redo support.

### 5. Manage
- **Project Information**: Project name, project number, client, and address.
- **Levels Manager**: Add, adjust, and inspect building storeys and elevations.
- **Materials**: Concrete, Masonry Brick, Structural Steel, Glass, Drywall, and Timber.

### 6. Insert
- **Export IFC4**: Generates standard ISO 10303-21 `.ifc` physical files compatible with Revit, Solibri, BlenderBIM, and BIMcollab.
- **Import IFC4**: Reads standard IFC4 building storeys, walls, columns, and slabs.
- **Sample Model**: One-click generation of a complete 2-storey office building.

---

## 🚀 Quick Start

### Build and Run

```bash
# Clone the repository
git clone https://github.com/dsalesarchitect/arch3d.git
cd arch3d

# Check workspace
cargo check --workspace

# Run all unit and projection tests
cargo test --workspace

# Run Arch3D Desktop
cargo run --bin arch3d
```

### Windows One-Click Launcher
Double-click `Launch-Arch3D.bat` in the repository root.

### Web Studio (GitHub Pages)
Test live in your browser without installation: [https://dsalesarchitect.github.io/arch3d/](https://dsalesarchitect.github.io/arch3d/)

---

## 📐 Single Source of Truth Projection Engine

In Arch3D, every building element is stored once in 3D world space at standard 1:1 scale (mm):
$$\mathbf{X}_{world} = (x, y, z) \in \mathbb{R}^3$$

When viewing the model:
1. **Floor Plan**: Slices the model at $Z_{cut} = Z_{level} + 1200\,\text{mm}$. Cutting solids are filled with cut hatch and outlined with heavy cut line weights. Door swing arcs and window sill geometry are generated.
2. **Reflected Ceiling Plan (RCP)**: Looks upwards from $Z_{cut} = Z_{level} + 2300\,\text{mm}$, rendering ceiling boundary perimeters and tile module grid lines (e.g. $600 \times 600\,\text{mm}$).
3. **Elevations**: Orthographically projects exterior faces onto North, South, East, and West planes with level datum markers and heavy ground lines.
4. **Sections**: Slices model prisms against arbitrary vertical cutting lines $(S_1, S_2)$, computing mathematically exact cut polygons and background projection lines.
5. **3D View**: Directional sunlight shading ($I = 0.45 + 0.55 \max(0, \mathbf{n} \cdot \mathbf{l})$), back-face culling, depth sorting, and crisp architectural edge linework.

---

## 📄 License

Arch3D is open-source software licensed under the **GNU General Public License v3.0 or later (GPL-3.0-or-later)** with the **Arch3D Commercial Plugin Exception** (see [LICENSE](LICENSE)).

- The core Arch3D application, its source code, and derivative works are protected under the GNU General Public License v3.0 or later.
- Under the **Arch3D Commercial Plugin Exception** (GNU GPLv3 Section 7), third-party and author plugins, modules, scripts, and add-ons communicating via the plugin API, WebAssembly interface, or socket/IPC bridges may be developed, distributed, and monetized under proprietary, commercial, or alternative terms without requiring the plugin itself to be open-sourced.

All contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md).
