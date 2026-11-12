## Pull Request — Cleanroom & Code Compliance

### ⚠️ Cleanroom Contributor Certification (Mandatory)
Please read and verify all checkboxes before requesting review. Any PR failing cleanroom certification will be immediately rejected and closed.

- [ ] **Cleanroom Verification**: I certify that this code is 100% original work authored by me, or provided under compatible open-source terms (GPL-3.0-or-later).
- [ ] **No Proprietary Decompilation**: I certify that I have NOT decompiled, reverse-engineered, or referenced source code, binary assemblies, or internal APIs from proprietary CAD/BIM software (e.g., Autodesk Revit, AutoCAD, Graphisoft ArchiCAD, Bentley Systems, Trimble, etc.).
- [ ] **Zero Proprietary Screenshots / Assets**: I certify that this PR and its commits contain NO proprietary binary blobs, patented trade-secret formulas, commercial UI icons, or proprietary software screenshots.
- [ ] **Architectural Standards**: All analytical calculations maintain 1:1 real-world millimetre (`mm`) scale and Right-Handed $+Z$ Up coordinates.
- [ ] **Test Verification**: I have run `cargo test --workspace` locally and verified that all existing and new unit tests pass with zero compiler warnings.

---

### Description of Changes
*Provide a concise summary of what this PR implements, fixes, or enhances.*

### Architectural / Geometric Impact
- **Elements Affected**: (e.g., Wall, Slab, Column, CurtainWall, IFC Exporter, Viewport Projection)
- **Mathematical / Projection Behavior**: (e.g., Miter join algorithm, B-Rep watertight solid hull)

### Related Issue
Closes #

### Verification Checklist
- [ ] `cargo check --workspace` passes with 0 warnings.
- [ ] `cargo test --workspace` passes all unit and projection tests.
- [ ] Tested interactively in desktop launcher (`Launch-Arch3D.bat`) or Web Studio.
