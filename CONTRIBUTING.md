# Contributing to Arch3D

Thank you for your interest in contributing to **Arch3D**! Arch3D is an open-source, cleanroom BIM (Building Information Modeling) engine and authoring application powered by a high-performance pure Rust engine.

---

## Cleanroom Architecture & Intellectual Property Policy

Arch3D is strictly authored from first principles under a **strict cleanroom engineering policy**:

1. **Zero Decompiled or Proprietary Code**:
   - Do NOT decompile, reverse-engineer, dissemble, or inspect proprietary CAD/BIM binary assemblies, DLLs, or source code (e.g., Autodesk Revit, AutoCAD, Graphisoft ArchiCAD, Bentley MicroStation, Trimble SketchUp internals, Vectorworks, etc.).
   - Do NOT reference proprietary internal APIs, patent-encumbered data structures, or trade-secret algorithms.

2. **Strict Prohibition on Proprietary Software Screenshots**:
   - **DO NOT upload screenshots, UI captures, or recordings of proprietary commercial software** in GitHub Issues, Pull Requests, or Discussions.
   - Requesting features by saying *"make it look like this screenshot from software X"* is strictly prohibited.
   - Any issue or PR containing screenshots of proprietary commercial software will be **immediately closed, locked, and deleted** to prevent copyright and trade dress contamination.

3. **Acceptable References & Attachments**:
   - Screenshots and screen recordings taken directly from **Arch3D** (Desktop or Web Studio).
   - Original **hand-drawn architectural sketches**, whiteboard drawings, and drafting diagrams.
   - Official public open standards documentation from **buildingSMART** (IFC2x3, IFC4, IFC4x3, ISO 10303-21 STEP physical files).
   - Legitimate open-source software (e.g., BlenderBIM, FreeCAD).
   - Real-world architectural construction blueprints, geometry textbooks, and building code specifications.

4. **1:1 Millimetre Scale & Mathematical First Principles**:
   - All spatial elements, analytical models, and compound assemblies are designed mathematically from first principles at 1:1 real-world millimetre (`mm`) scale in Right-Handed $+Z$ Up coordinates.

---

## Reporting Bugs & Proposing Features

All issue submissions are gated by mandatory GitHub Issue Forms with required cleanroom acknowledgments:

- **Bug Reports**: Use the [Bug Report Form](https://github.com/dsalesarchitect/arch3d/issues/new?template=bug_report.yml). You must certify that no proprietary screenshots are attached and provide step-by-step reproduction steps within Arch3D.
- **Feature Requests**: Use the [Feature Proposal Form](https://github.com/dsalesarchitect/arch3d/issues/new?template=feature_request.yml). Proposals must explain real-world architectural/engineering requirements based on construction workflows or openBIM specifications—not commercial software mimicry.
- **Blank Issues Disabled**: Unformatted issues that bypass the cleanroom checklist are disabled.

---

## Workspace Crates

Arch3D is modularized into 5 focused crates:

| Crate | Description |
|---|---|
| `plinth-geom` | High-precision geometry kernel: 2D/3D vectors, AABBs, affine transforms, polygon ear-clipping triangulation, prism solid extrusion, and half-space clipping. |
| `plinth-model` | Single source of truth BIM database: elements (walls, columns, beams, slabs, ceilings, doors, windows, dimensions), levels, grids, materials, and undo/redo history. All units are real-world millimetres (1:1 scale). |
| `plinth-views` | Dynamic projection engine: automatically derives Floor Plans, Reflected Ceiling Plans (RCP), Elevations (N/S/E/W), Sections, 3D axonometric views, and Details from the single-source-of-truth 3D model. |
| `plinth-ifc` | ISO 10303-21 STEP physical file exporter and parser compliant with the buildingSMART IFC4 Coordination View schema. |
| `plinth-app` | Native cross-platform desktop UI built with `eframe`/`egui`. Features Revit-inspired Ribbon tools, Project Browser, Properties Panel, interactive canvas viewport with snapping, and Dark/Light theme toggle. |

---

## Development Workflow

### Prerequisites
- Rust 1.81+ (stable toolchain)
- Cargo

### Building and Testing
```bash
# Clone the repository
git clone https://github.com/dsalesarchitect/arch3d.git
cd arch3d

# Check all crates
cargo check --workspace

# Run all unit and integration tests
cargo test --workspace

# Run the desktop application
cargo run --bin arch3d
```

---

## Code Style and Guidelines
1. **Millimetres standard (1:1 scale)**: All coordinate and dimension calculations throughout the data model and geometry kernel must be in millimetres.
2. **Zero Warnings**: Ensure `cargo check --workspace` finishes with zero compiler warnings.
3. **Tests Required**: Every geometric routine, entity serializer, or projection filter should have unit tests.
4. **Clean Commits**: Write clear, descriptive commit messages.
