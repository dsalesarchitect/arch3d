# Contributing to Arch3D

Thank you for your interest in contributing to **Arch3D**! Arch3D is an open-source, cleanroom BIM (Building Information Modeling) engine and authoring application powered by a high-performance pure Rust engine.

---

## 🏛 Cleanroom Architecture Policy

Arch3D is strictly authored from scratch under a **cleanroom engineering policy**:
- **No decompiled, proprietary, or reverse-engineered code**: Do not copy or reference proprietary CAD/BIM source code.
- **Open Standards**: All data exchange uses the open ISO 10303-21 standard (buildingSMART IFC4 schema).
- **Original Implementations**: All geometric algorithms, projection routines, and UI panels are original implementations designed for simplicity, readability, and speed.

---

## 📦 Workspace Crates

Arch3D is modularized into 5 focused crates:

| Crate | Description |
|---|---|
| `plinth-geom` | High-precision geometry kernel: 2D/3D vectors, AABBs, affine transforms, polygon ear-clipping triangulation, prism solid extrusion, and half-space clipping. |
| `plinth-model` | Single source of truth BIM database: elements (walls, columns, beams, slabs, ceilings, doors, windows, dimensions), levels, grids, materials, and undo/redo history. All units are real-world millimetres (1:1 scale). |
| `plinth-views` | Dynamic projection engine: automatically derives Floor Plans, Reflected Ceiling Plans (RCP), Elevations (N/S/E/W), Sections, 3D axonometric views, and Details from the single-source-of-truth 3D model. |
| `plinth-ifc` | ISO 10303-21 STEP physical file exporter and parser compliant with the buildingSMART IFC4 Coordination View schema. |
| `plinth-app` | Native cross-platform desktop UI built with `eframe`/`egui`. Features Revit-inspired Ribbon tools, Project Browser, Properties Panel, interactive canvas viewport with snapping, and Dark/Light theme toggle. |

---

## 🛠 Development Workflow

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

## 🎨 Code Style and Guidelines
1. **Millimetres standard (1:1 scale)**: All coordinate and dimension calculations throughout the data model and geometry kernel must be in millimetres.
2. **Zero Warnings**: Ensure `cargo check --workspace` finishes with zero compiler warnings.
3. **Tests Required**: Every geometric routine, entity serializer, or projection filter should have unit tests.
4. **Clean Commits**: Write clear, descriptive commit messages.
