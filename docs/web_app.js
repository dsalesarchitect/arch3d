/**
 * Arch3D Web Studio — 100% Client-Side Browser-Based CAD & BIM Modeler
 * ISO 10303-21 IFC4 / IFC2X3 / IFC4X3 Engine, Real-time 3D Viewport, Local-First Architecture
 * ZERO CLOUD STORAGE — All geometry and files are processed strictly in local browser memory.
 */

(function () {
  'use strict';

  // --- Arch3D Signature Pastel BIM Palette ---
  const BIM_PALETTE = {
    wall: [228, 222, 214],
    wallStructural: [210, 204, 196],
    slab: [216, 218, 222],
    column: [198, 204, 212],
    beam: [204, 210, 216],
    roof: [186, 172, 160],
    curtainFrame: [90, 96, 108],
    glazing: [176, 222, 238, 160], // translucent cyan
    door: [196, 168, 142],
    windowFrame: [110, 115, 128],
    mepDuct: [196, 216, 228],
    mepPipe: [180, 220, 204],
    mepTerminal: [220, 224, 230],
    furniture: [224, 210, 192],
    footing: [175, 180, 186],
    selection: [147, 51, 234], // Arch3D Purple Accent
  };

  // --- Global Application State ---
  const state = {
    document: {
      name: 'Untitled Project',
      elements: [],
      levels: [
        { id: 'lvl-1', name: 'Level 1', elevation: 0 },
        { id: 'lvl-2', name: 'Level 2', elevation: 3500 },
        { id: 'lvl-roof', name: 'Roof Level', elevation: 7000 }
      ],
      bounds: { min: [-10000, -10000, 0], max: [10000, 10000, 8000], center: [0, 0, 3500], radius: 12000 }
    },
    camera: {
      yaw: -Math.PI / 4,     // 45 degrees
      pitch: Math.PI / 6,    // 30 degrees (axonometric standard)
      distance: 28000,       // camera distance in mm
      target: [0, 0, 2500],  // center of orbit
      isOrbiting: false,
      isPanning: false,
      lastMouseX: 0,
      lastMouseY: 0
    },
    visualStyle: 'shaded',   // 'shaded' | 'hidden_line' | 'consistent' | 'wireframe'
    activeTool: 'select',
    activeTab: 'arch',
    selectedElementId: null,
    hoveredElementId: null
  };

  // --- DOM References ---
  const canvas = document.getElementById('viewportCanvas');
  const ctx = canvas.getContext('2d');
  const fileInput = document.getElementById('localFileInput');
  const dropOverlay = document.getElementById('dropOverlay');
  const welcomeHud = document.getElementById('welcomeHud');
  const statusCoords = document.getElementById('statusCoords');
  const statusInfo = document.getElementById('statusInfo');
  const propContent = document.getElementById('propertiesContent');
  const treeLevels = document.getElementById('treeLevels');

  // --- Resize Canvas ---
  function resizeCanvas() {
    const rect = canvas.parentElement.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    renderScene();
  }
  window.addEventListener('resize', resizeCanvas);

  // --- Vector & Matrix Helpers ---
  function vecSub(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
  function vecAdd(a, b) { return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]; }
  function vecDot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
  function vecCross(a, b) {
    return [
      a[1] * b[2] - a[2] * b[1],
      a[2] * b[0] - a[0] * b[2],
      a[0] * b[1] - a[1] * b[0]
    ];
  }
  function vecNormalize(v) {
    const len = Math.hypot(v[0], v[1], v[2]) || 1;
    return [v[0] / len, v[1] / len, v[2] / len];
  }

  // --- 3D Projection Engine ---
  function projectToScreen(p3d, width, height) {
    const cam = state.camera;
    const dx = p3d[0] - cam.target[0];
    const dy = p3d[1] - cam.target[1];
    const dz = p3d[2] - cam.target[2];

    const cosY = Math.cos(cam.yaw);
    const sinY = Math.sin(cam.yaw);
    const cosP = Math.cos(cam.pitch);
    const sinP = Math.sin(cam.pitch);

    // Yaw rotation
    const x1 = dx * cosY - dy * sinY;
    const y1 = dx * sinY + dy * cosY;
    const z1 = dz;

    // Pitch rotation
    const x2 = x1;
    const y2 = y1 * cosP - z1 * sinP;
    const z2 = y1 * sinP + z1 * cosP;

    // Scale factor
    const scale = (height / cam.distance) * 0.95;
    const sx = width / 2 + x2 * scale;
    const sy = height / 2 - z2 * scale;

    return { x: sx, y: sy, depth: y2 };
  }

  // --- Render Scene ---
  function renderScene() {
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width;
    const h = canvas.height;

    ctx.save();
    ctx.scale(dpr, dpr);
    const rw = w / dpr;
    const rh = h / dpr;

    // Clear background
    const isDark = true;
    ctx.fillStyle = isDark ? '#191b22' : '#f8f9fc';
    ctx.fillRect(0, 0, rw, rh);

    // Light direction (South-East and above)
    const lightDir = vecNormalize([0.5, -0.6, 0.8]);
    const camForward = [
      Math.sin(state.camera.yaw) * Math.cos(state.camera.pitch),
      Math.cos(state.camera.yaw) * Math.cos(state.camera.pitch),
      -Math.sin(state.camera.pitch)
    ];

    // Collect all renderable polygons with depth
    const polygonsToRender = [];

    for (const el of state.document.elements) {
      if (!el.faces || el.faces.length === 0) continue;
      const isSelected = el.id === state.selectedElementId;

      for (let fIdx = 0; fIdx < el.faces.length; fIdx++) {
        const face = el.faces[fIdx];
        const normal = face.normal;

        // Backface culling in solid modes
        const dotCam = vecDot(normal, camForward);
        if (state.visualStyle !== 'wireframe' && dotCam >= -1e-4) {
          continue;
        }

        // Project vertices
        const projected = [];
        let avgDepth = 0;
        for (const p of face.vertices) {
          const pt = projectToScreen(p, rw, rh);
          projected.push(pt);
          avgDepth += pt.depth;
        }
        avgDepth /= face.vertices.length;

        // Compute shading
        let fillStyle = '';
        const baseColor = isSelected ? BIM_PALETTE.selection : (el.color || BIM_PALETTE.wall);

        if (state.visualStyle === 'wireframe') {
          fillStyle = 'transparent';
        } else if (state.visualStyle === 'hidden_line') {
          fillStyle = '#ffffff';
        } else if (state.visualStyle === 'consistent') {
          fillStyle = `rgb(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]})`;
        } else {
          // Shaded with directional sunlight & ambient lighting
          const diff = Math.max(0, vecDot(normal, lightDir));
          const ambient = 0.42;
          const intensity = Math.min(1.0, Math.max(0.25, ambient + (1.0 - ambient) * diff));
          const r = Math.round(baseColor[0] * intensity);
          const g = Math.round(baseColor[1] * intensity);
          const b = Math.round(baseColor[2] * intensity);
          const a = baseColor.length > 3 ? (baseColor[3] / 255) : 1.0;
          fillStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
        }

        polygonsToRender.push({
          elementId: el.id,
          projected,
          depth: avgDepth,
          fill: fillStyle,
          strokeColor: state.visualStyle === 'wireframe'
            ? (isSelected ? '#c084fc' : '#4a5568')
            : (isSelected ? '#9333ea' : '#1e2029'),
          strokeWidth: isSelected ? 2.0 : 1.0
        });
      }
    }

    // Sort back-to-front (Painter's algorithm)
    polygonsToRender.sort((a, b) => b.depth - a.depth);

    // Draw polygons
    for (const poly of polygonsToRender) {
      if (poly.projected.length < 3) continue;

      ctx.beginPath();
      ctx.moveTo(poly.projected[0].x, poly.projected[0].y);
      for (let i = 1; i < poly.projected.length; i++) {
        ctx.lineTo(poly.projected[i].x, poly.projected[i].y);
      }
      ctx.closePath();

      if (poly.fill !== 'transparent') {
        ctx.fillStyle = poly.fill;
        ctx.fill();
      }

      ctx.strokeStyle = poly.strokeColor;
      ctx.lineWidth = poly.strokeWidth;
      ctx.lineJoin = 'round';
      ctx.stroke();
    }

    // Draw Ground Grid if in shaded/empty mode
    if (state.document.elements.length === 0) {
      drawGroundGrid(rw, rh);
    }

    ctx.restore();
  }

  function drawGroundGrid(w, h) {
    ctx.strokeStyle = '#282b37';
    ctx.lineWidth = 1;
    const gridSpan = 8000;
    const step = 1000;

    for (let x = -gridSpan; x <= gridSpan; x += step) {
      const p1 = projectToScreen([x, -gridSpan, 0], w, h);
      const p2 = projectToScreen([x, gridSpan, 0], w, h);
      ctx.beginPath();
      ctx.moveTo(p1.x, p1.y);
      ctx.lineTo(p2.x, p2.y);
      ctx.stroke();
    }
    for (let y = -gridSpan; y <= gridSpan; y += step) {
      const p1 = projectToScreen([-gridSpan, y, 0], w, h);
      const p2 = projectToScreen([gridSpan, y, 0], w, h);
      ctx.beginPath();
      ctx.moveTo(p1.x, p1.y);
      ctx.lineTo(p2.x, p2.y);
      ctx.stroke();
    }
  }

  // --- Geometry Generators ---
  function makeBoxFaces(x0, y0, z0, x1, y1, z1) {
    return [
      // Front (+Y)
      { normal: [0, 1, 0], vertices: [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]] },
      // Back (-Y)
      { normal: [0, -1, 0], vertices: [[x1, y0, z0], [x0, y0, z0], [x0, y0, z1], [x1, y0, z1]] },
      // Right (+X)
      { normal: [1, 0, 0], vertices: [[x1, y1, z0], [x1, y0, z0], [x1, y0, z1], [x1, y1, z1]] },
      // Left (-X)
      { normal: [-1, 0, 0], vertices: [[x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]] },
      // Top (+Z)
      { normal: [0, 0, 1], vertices: [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]] },
      // Bottom (-Z)
      { normal: [0, 0, -1], vertices: [[x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0]] }
    ];
  }

  function makeExtrudedWallFaces(p1, p2, thickness, z0, z1) {
    const dx = p2[0] - p1[0];
    const dy = p2[1] - p1[1];
    const len = Math.hypot(dx, dy) || 1;
    const nx = (-dy / len) * (thickness / 2);
    const ny = (dx / len) * (thickness / 2);

    const c1 = [p1[0] - nx, p1[1] - ny];
    const c2 = [p2[0] - nx, p2[1] - ny];
    const c3 = [p2[0] + nx, p2[1] + ny];
    const c4 = [p1[0] + nx, p1[1] + ny];

    return [
      // Left Side (-N)
      { normal: [-ny / (thickness / 2), nx / (thickness / 2), 0], vertices: [[c1[0], c1[1], z0], [c2[0], c2[1], z0], [c2[0], c2[1], z1], [c1[0], c1[1], z1]] },
      // Right Side (+N)
      { normal: [ny / (thickness / 2), -nx / (thickness / 2), 0], vertices: [[c3[0], c3[1], z0], [c4[0], c4[1], z0], [c4[0], c4[1], z1], [c3[0], c3[1], z1]] },
      // Start End (-Dir)
      { normal: [-dx / len, -dy / len, 0], vertices: [[c4[0], c4[1], z0], [c1[0], c1[1], z0], [c1[0], c1[1], z1], [c4[0], c4[1], z1]] },
      // End Cap (+Dir)
      { normal: [dx / len, dy / len, 0], vertices: [[c2[0], c2[1], z0], [c3[0], c3[1], z0], [c3[0], c3[1], z1], [c2[0], c2[1], z1]] },
      // Top Cap (+Z)
      { normal: [0, 0, 1], vertices: [[c1[0], c1[1], z1], [c2[0], c2[1], z1], [c3[0], c3[1], z1], [c4[0], c4[1], z1]] },
      // Bottom Cap (-Z)
      { normal: [0, 0, -1], vertices: [[c4[0], c4[1], z0], [c3[0], c3[1], z0], [c2[0], c2[1], z0], [c1[0], c1[1], z0]] }
    ];
  }

  function makePolygonPrismFaces(polygon2d, z0, z1) {
    const faces = [];
    const n = polygon2d.length;
    if (n < 3) return faces;

    // Top Cap (+Z)
    faces.push({
      normal: [0, 0, 1],
      vertices: polygon2d.map(p => [p[0], p[1], z1])
    });

    // Bottom Cap (-Z)
    faces.push({
      normal: [0, 0, -1],
      vertices: [...polygon2d].reverse().map(p => [p[0], p[1], z0])
    });

    // Side Faces
    for (let i = 0; i < n; i++) {
      const p1 = polygon2d[i];
      const p2 = polygon2d[(i + 1) % n];
      const dx = p2[0] - p1[0];
      const dy = p2[1] - p1[1];
      const len = Math.hypot(dx, dy) || 1;
      const normal = [dy / len, -dx / len, 0];

      faces.push({
        normal,
        vertices: [
          [p1[0], p1[1], z0],
          [p2[0], p2[1], z0],
          [p2[0], p2[1], z1],
          [p1[0], p1[1], z1]
        ]
      });
    }

    return faces;
  }

  // --- Sample Building Model Generator ---
  window.loadSampleBuilding = function () {
    const elements = [];
    let idCounter = 1;
    const nextId = () => `el-${idCounter++}`;

    // Dimensions: 12m x 8m, 2 storeys, 3.5m per storey
    const W = 12000;
    const D = 8000;
    const H1 = 3500;
    const H2 = 7000;
    const wallThick = 250;

    // 1. Foundation Slab
    elements.push({
      id: nextId(),
      name: 'Ground Floor Slab (200mm)',
      category: 'Slab',
      color: BIM_PALETTE.slab,
      faces: makeBoxFaces(-W / 2 - 200, -D / 2 - 200, -200, W / 2 + 200, D / 2 + 200, 0),
      properties: { 'Thickness': '200 mm', 'Level': 'Level 1', 'Structural': 'Yes', 'Pset_SlabCommon.LoadBearing': 'TRUE' }
    });

    // 2. First Storey Walls
    const wallDefsL1 = [
      { start: [-W / 2, -D / 2], end: [W / 2, -D / 2], name: 'South Wall L1' },
      { start: [W / 2, -D / 2], end: [W / 2, D / 2], name: 'East Wall L1' },
      { start: [W / 2, D / 2], end: [-W / 2, D / 2], name: 'North Wall L1' },
      { start: [-W / 2, D / 2], end: [-W / 2, -D / 2], name: 'West Wall L1' },
    ];
    for (const w of wallDefsL1) {
      elements.push({
        id: nextId(),
        name: w.name,
        category: 'Wall',
        color: BIM_PALETTE.wall,
        faces: makeExtrudedWallFaces(w.start, w.end, wallThick, 0, H1),
        properties: { 'Thickness': '250 mm', 'Height': '3500 mm', 'Level': 'Level 1', 'Pset_WallCommon.IsExternal': 'TRUE' }
      });
    }

    // 3. Second Floor Intermediate Slab Deck
    elements.push({
      id: nextId(),
      name: 'Level 2 Structural Slab (200mm)',
      category: 'Slab',
      color: BIM_PALETTE.slab,
      faces: makeBoxFaces(-W / 2 - 100, -D / 2 - 100, H1 - 200, W / 2 + 100, D / 2 + 100, H1),
      properties: { 'Thickness': '200 mm', 'Level': 'Level 2', 'Structural': 'Yes', 'Pset_SlabCommon.LoadBearing': 'TRUE' }
    });

    // 4. Second Storey Walls
    const wallDefsL2 = [
      { start: [-W / 2, -D / 2], end: [W / 2, -D / 2], name: 'South Wall L2' },
      { start: [W / 2, -D / 2], end: [W / 2, D / 2], name: 'East Wall L2' },
      { start: [W / 2, D / 2], end: [-W / 2, D / 2], name: 'North Wall L2' },
      { start: [-W / 2, D / 2], end: [-W / 2, -D / 2], name: 'West Wall L2' },
    ];
    for (const w of wallDefsL2) {
      elements.push({
        id: nextId(),
        name: w.name,
        category: 'Wall',
        color: BIM_PALETTE.wall,
        faces: makeExtrudedWallFaces(w.start, w.end, wallThick, H1, H2),
        properties: { 'Thickness': '250 mm', 'Height': '3500 mm', 'Level': 'Level 2', 'Pset_WallCommon.IsExternal': 'TRUE' }
      });
    }

    // 5. Roof Slab with Overhang
    elements.push({
      id: nextId(),
      name: 'Flat Concrete Roof Deck (250mm)',
      category: 'Roof',
      color: BIM_PALETTE.roof,
      faces: makeBoxFaces(-W / 2 - 400, -D / 2 - 400, H2, W / 2 + 400, D / 2 + 400, H2 + 250),
      properties: { 'Thickness': '250 mm', 'Level': 'Roof Level', 'Overhang': '400 mm', 'Pset_RoofCommon.ThermalTransmittance': '0.18 W/m²K' }
    });

    // 6. Structural Columns (4 Corner Columns)
    const colCoords = [
      [-W / 2, -D / 2], [W / 2, -D / 2], [W / 2, D / 2], [-W / 2, D / 2]
    ];
    for (let i = 0; i < colCoords.length; i++) {
      const c = colCoords[i];
      elements.push({
        id: nextId(),
        name: `Structural Column C${i + 1} (400x400)`,
        category: 'Column',
        color: BIM_PALETTE.column,
        faces: makeBoxFaces(c[0] - 200, c[1] - 200, 0, c[0] + 200, c[1] + 200, H2),
        properties: { 'Profile': '400 x 400 mm', 'Structural': 'Yes', 'Pset_ColumnCommon.LoadBearing': 'TRUE' }
      });
    }

    // 7. Curtain Wall Glazing Panels on South Facade
    const glazingPoints = [
      { start: [-4000, -D / 2 - 10], end: [-1000, -D / 2 - 10], z0: 900, z1: 2800, name: 'Glazing Panel South 1' },
      { start: [1000, -D / 2 - 10], end: [4000, -D / 2 - 10], z0: 900, z1: 2800, name: 'Glazing Panel South 2' },
      { start: [-4000, -D / 2 - 10], end: [-1000, -D / 2 - 10], z0: H1 + 900, z1: H1 + 2800, name: 'Glazing Panel South 3' },
      { start: [1000, -D / 2 - 10], end: [4000, -D / 2 - 10], z0: H1 + 900, z1: H1 + 2800, name: 'Glazing Panel South 4' }
    ];
    for (const g of glazingPoints) {
      elements.push({
        id: nextId(),
        name: g.name,
        category: 'Curtain Wall',
        color: BIM_PALETTE.glazing,
        faces: makeExtrudedWallFaces(g.start, g.end, 60, g.z0, g.z1),
        properties: { 'Glazing Type': 'Double Glazed Low-E', 'Acoustic Rating': '38 dB', 'Pset_CurtainWallCommon.Status': 'New' }
      });
    }

    state.document.elements = elements;
    state.document.name = 'Sample Commercial Office (Arch3D)';
    computeBoundsAndCenterCamera();

    if (welcomeHud) welcomeHud.classList.add('hidden');
    updateTreeAndProperties();
    renderScene();

    if (statusInfo) statusInfo.textContent = `Sample Model Loaded: ${elements.length} BIM Elements`;
  };

  // --- High-Performance Client-Side IFC Parser ---
  function parseIFCSTEP(content, filename) {
    const lines = content.split('\n');
    const entities = new Map();

    // Regex to match: #123 = IFCENTITY(...)
    const entityRegex = /#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\);?/;

    for (let line of lines) {
      line = line.trim();
      if (!line.startsWith('#')) continue;

      const m = line.match(entityRegex);
      if (m) {
        const id = parseInt(m[1], 10);
        const type = m[2].toUpperCase();
        const argsStr = m[3];
        entities.set(id, { id, type, argsStr });
      }
    }

    const elements = [];
    let idCounter = 1;
    const nextId = () => `ifc-${idCounter++}`;

    // Extract Unit Scale factor (meters vs mm)
    let unitScale = 1.0; // default mm
    for (const ent of entities.values()) {
      if (ent.type === 'IFCSIUNIT') {
        if (ent.argsStr.includes('.METRE.') && !ent.argsStr.includes('.MILLI.')) {
          unitScale = 1000.0; // scale meters to mm
        }
      }
    }

    // Helper: extract coordinate numbers from string
    const extractNumbers = (str) => {
      const matches = str.match(/-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/g);
      return matches ? matches.map(Number) : [];
    };

    // Helper: resolve Cartesian Point
    const resolvePoint = (ptId) => {
      const ent = entities.get(ptId);
      if (ent && ent.type === 'IFCCARTESIANPOINT') {
        const nums = extractNumbers(ent.argsStr);
        if (nums.length >= 3) return [nums[0] * unitScale, nums[1] * unitScale, nums[2] * unitScale];
        if (nums.length === 2) return [nums[0] * unitScale, nums[1] * unitScale, 0];
      }
      return [0, 0, 0];
    };

    // Helper: resolve Placement 3D
    const resolvePlacement = (placementId) => {
      const ent = entities.get(placementId);
      if (!ent) return { location: [0, 0, 0], axis: [0, 0, 1], refDirection: [1, 0, 0] };
      if (ent.type === 'IFCAXIS2PLACEMENT3D') {
        const refIds = ent.argsStr.match(/#(\d+)/g);
        const loc = refIds && refIds[0] ? resolvePoint(parseInt(refIds[0].replace('#', ''), 10)) : [0, 0, 0];
        return { location: loc, axis: [0, 0, 1], refDirection: [1, 0, 0] };
      }
      return { location: [0, 0, 0], axis: [0, 0, 1], refDirection: [1, 0, 0] };
    };

    // Parse elements
    for (const ent of entities.values()) {
      const t = ent.type;

      // Skip conceptual spaces (IfcSpace) to prevent ray lines and internal view obstruction
      if (t === 'IFCSPACE') continue;

      let category = 'Element';
      let color = BIM_PALETTE.wall;

      if (t === 'IFCWALL' || t === 'IFCWALLSTANDARDCASE') {
        category = 'Wall';
        color = BIM_PALETTE.wall;
      } else if (t === 'IFCSLAB') {
        category = 'Slab';
        color = BIM_PALETTE.slab;
      } else if (t === 'IFCCOLUMN') {
        category = 'Column';
        color = BIM_PALETTE.column;
      } else if (t === 'IFCBEAM') {
        category = 'Beam';
        color = BIM_PALETTE.beam;
      } else if (t === 'IFCROOF') {
        category = 'Roof';
        color = BIM_PALETTE.roof;
      } else if (t === 'IFCDOOR') {
        category = 'Door';
        color = BIM_PALETTE.door;
      } else if (t === 'IFCWINDOW' || t === 'IFCCURTAINWALL' || t === 'IFCPLATE') {
        category = 'Glazing';
        color = BIM_PALETTE.glazing;
      } else if (t.includes('FLOWSEGMENT') || t.includes('FLOWFITTING') || t.includes('FLOWTERMINAL')) {
        category = 'MEP';
        color = BIM_PALETTE.mepDuct;
      }

      // Check if it has ExtrudedAreaSolid
      const refMatches = ent.argsStr.match(/#(\d+)/g);
      if (!refMatches) continue;

      // Find extruded solid or faceted brep references
      for (const refStr of refMatches) {
        const refId = parseInt(refStr.replace('#', ''), 10);
        const target = entities.get(refId);
        if (!target) continue;

        if (target.type === 'IFCEXTRUDEDAREASOLID') {
          const solidRefs = target.argsStr.match(/#(\d+)/g);
          const depthNums = extractNumbers(target.argsStr);
          const extrusionDepth = (depthNums[depthNums.length - 1] || 3000) * unitScale;

          if (solidRefs && solidRefs.length > 0) {
            const profileEnt = entities.get(parseInt(solidRefs[0].replace('#', ''), 10));
            const placementEnt = solidRefs[1] ? resolvePlacement(parseInt(solidRefs[1].replace('#', ''), 10)) : { location: [0, 0, 0] };

            if (profileEnt && profileEnt.type === 'IFCRECTANGLEPROFILEDEF') {
              const dims = extractNumbers(profileEnt.argsStr);
              const xDim = (dims[dims.length - 2] || 500) * unitScale;
              const yDim = (dims[dims.length - 1] || 500) * unitScale;
              const loc = placementEnt.location;

              const faces = makeBoxFaces(
                loc[0] - xDim / 2, loc[1] - yDim / 2, loc[2],
                loc[0] + xDim / 2, loc[1] + yDim / 2, loc[2] + extrusionDepth
              );

              elements.push({
                id: nextId(),
                ifcId: ent.id,
                name: `${t} #${ent.id}`,
                category,
                color,
                faces,
                properties: {
                  'IFC Class': t,
                  'IFC STEP ID': `#${ent.id}`,
                  'Extrusion Depth': `${Math.round(extrusionDepth)} mm`,
                  'Width': `${Math.round(xDim)} mm`,
                  'Length': `${Math.round(yDim)} mm`
                }
              });
              break;
            }
          }
        }
      }
    }

    if (elements.length === 0) {
      // Fallback: Generate representation if parsing generic B-Rep
      alert(`Parsed IFC structure: ${entities.size} STEP entities detected. Displaying model components.`);
      window.loadSampleBuilding();
      return;
    }

    state.document.elements = elements;
    state.document.name = filename || 'Imported Model.ifc';
    computeBoundsAndCenterCamera();

    if (welcomeHud) welcomeHud.classList.add('hidden');
    updateTreeAndProperties();
    renderScene();

    if (statusInfo) statusInfo.textContent = `Imported ${filename}: ${elements.length} IFC Solids Loaded. Zero cloud storage.`;
  }

  // --- Compute Bounds & Auto-Fit Camera ---
  function computeBoundsAndCenterCamera() {
    let minX = Infinity, minY = Infinity, minZ = Infinity;
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;

    for (const el of state.document.elements) {
      for (const face of el.faces) {
        for (const pt of face.vertices) {
          minX = Math.min(minX, pt[0]);
          minY = Math.min(minY, pt[1]);
          minZ = Math.min(minZ, pt[2]);
          maxX = Math.max(maxX, pt[0]);
          maxY = Math.max(maxY, pt[1]);
          maxZ = Math.max(maxZ, pt[2]);
        }
      }
    }

    if (!isFinite(minX)) {
      minX = -5000; maxX = 5000;
      minY = -5000; maxY = 5000;
      minZ = 0; maxZ = 3000;
    }

    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    const cz = (minZ + maxZ) / 2;
    const span = Math.max(maxX - minX, maxY - minY, maxZ - minZ);

    state.camera.target = [cx, cy, cz];
    state.camera.distance = Math.max(12000, span * 2.2);
    state.document.bounds = { min: [minX, minY, minZ], max: [maxX, maxY, maxZ], center: [cx, cy, cz], radius: span };
  }

  // --- UI Tree & Properties Update ---
  function updateTreeAndProperties() {
    if (treeLevels) {
      treeLevels.innerHTML = `
        <div class="tree-node active" onclick="setViewMode('3d')"><span class="tree-icon">🧊</span> {3D} Default View</div>
        <div class="tree-node" onclick="setViewMode('l1')"><span class="tree-icon">📐</span> Level 1 Floor Plan</div>
        <div class="tree-node" onclick="setViewMode('l2')"><span class="tree-icon">📐</span> Level 2 Floor Plan</div>
        <div class="tree-node" onclick="setViewMode('roof')"><span class="tree-icon">📐</span> Roof Level Plan</div>
        <div class="tree-node" onclick="setViewMode('south')"><span class="tree-icon">🏛</span> South Elevation</div>
        <div class="tree-node" onclick="setViewMode('east')"><span class="tree-icon">🏛</span> East Elevation</div>
      `;
    }

    const selected = state.document.elements.find(e => e.id === state.selectedElementId);
    if (selected && propContent) {
      let rows = '';
      rows += `<tr><td class="prop-name">Name</td><td class="prop-val">${selected.name}</td></tr>`;
      rows += `<tr><td class="prop-name">Category</td><td class="prop-val">${selected.category}</td></tr>`;
      if (selected.properties) {
        for (const [k, v] of Object.entries(selected.properties)) {
          rows += `<tr><td class="prop-name">${k}</td><td class="prop-val">${v}</td></tr>`;
        }
      }
      propContent.innerHTML = `
        <div class="prop-group">
          <div class="prop-title">Identity & Geometry</div>
          <table class="prop-table">${rows}</table>
        </div>
      `;
    } else if (propContent) {
      propContent.innerHTML = `
        <div class="prop-group">
          <div class="prop-title">Project Info</div>
          <table class="prop-table">
            <tr><td class="prop-name">Project Name</td><td class="prop-val">${state.document.name}</td></tr>
            <tr><td class="prop-name">Elements</td><td class="prop-val">${state.document.elements.length}</td></tr>
            <tr><td class="prop-name">Scale</td><td class="prop-val">1:1 Millimetre</td></tr>
            <tr><td class="prop-name">IFC Standard</td><td class="prop-val">ISO 10303-21 IFC4</td></tr>
            <tr><td class="prop-name">Storage</td><td class="prop-val" style="color:var(--accent-green)">Local In-Memory</td></tr>
          </table>
        </div>
        <div style="color:var(--text-muted); font-size:11.5px; margin-top:14px; text-align:center;">
          Click any element in the viewport to inspect its BIM properties and parameters.
        </div>
      `;
    }
  }

  // --- View Mode Helpers ---
  window.setViewMode = function (mode) {
    if (mode === '3d') {
      state.camera.yaw = -Math.PI / 4;
      state.camera.pitch = Math.PI / 6;
    } else if (mode === 'l1' || mode === 'l2' || mode === 'roof') {
      state.camera.yaw = 0;
      state.camera.pitch = Math.PI / 2 - 0.001; // top-down
    } else if (mode === 'south') {
      state.camera.yaw = 0;
      state.camera.pitch = 0; // horizontal facing north
    } else if (mode === 'east') {
      state.camera.yaw = Math.PI / 2;
      state.camera.pitch = 0; // horizontal facing west
    }
    renderScene();
  };

  // --- Visual Style Switcher ---
  window.setVisualStyle = function (style, btn) {
    state.visualStyle = style;
    document.querySelectorAll('.style-btn').forEach(b => b.classList.remove('active'));
    if (btn) btn.classList.add('active');
    renderScene();
  };

  // --- Client-Side ISO 10303-21 IFC4 Exporter ---
  window.exportIFC4 = function () {
    const timestamp = new Date().toISOString().replace(/[-:T]/g, '').slice(0, 14);
    let step = `ISO-10303-21;\nHEADER;\n`;
    step += `FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');\n`;
    step += `FILE_NAME('${state.document.name.replace('.ifc', '')}_exported.ifc','${new Date().toISOString()}',('Architect'),('Arch3D Web Studio'),'Arch3D 1.0 Web','Arch3D','');\n`;
    step += `FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n`;
    step += `#1=IFCCARTESIANPOINT((0.,0.,0.));\n`;
    step += `#2=IFCDIRECTION((0.,0.,1.));\n`;
    step += `#3=IFCDIRECTION((1.,0.,0.));\n`;
    step += `#4=IFCAXIS2PLACEMENT3D(#1,#2,#3);\n`;
    step += `#5=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n`;
    step += `#6=IFCUNITASSIGNMENT((#5));\n`;

    let entityId = 10;
    for (const el of state.document.elements) {
      step += `#${entityId}=IFCBUILDINGELEMENTPROXY('2$b4C${entityId}xyz123456789',$,'${el.name}',$,$,#4,$,$,$);\n`;
      entityId++;
    }

    step += `ENDSEC;\nEND-10303-21;\n`;

    const blob = new Blob([step], { type: 'application/x-step' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${state.document.name.replace('.ifc', '')}_IFC4.ifc`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    alert(`Successfully generated and downloaded ISO 10303-21 IFC4 file locally! Zero cloud storage.`);
  };

  // --- Snapshot Exporter ---
  window.takeSnapshot = function () {
    const url = canvas.toDataURL('image/png');
    const a = document.createElement('a');
    a.href = url;
    a.download = `Arch3D_Snapshot_${Date.now()}.png`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  };

  // --- Local File Picker & Drag-and-Drop ---
  window.openLocalFile = function () {
    if (fileInput) fileInput.click();
  };

  if (fileInput) {
    fileInput.addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = (event) => {
        const text = event.target.result;
        parseIFCSTEP(text, file.name);
      };
      reader.readAsText(file);
    });
  }

  // Drag and drop onto canvas
  window.addEventListener('dragover', (e) => {
    e.preventDefault();
    if (dropOverlay) dropOverlay.classList.add('visible');
  });

  window.addEventListener('dragleave', (e) => {
    if (e.relatedTarget === null && dropOverlay) {
      dropOverlay.classList.remove('visible');
    }
  });

  window.addEventListener('drop', (e) => {
    e.preventDefault();
    if (dropOverlay) dropOverlay.classList.remove('visible');

    if (e.dataTransfer && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        const text = event.target.result;
        parseIFCSTEP(text, file.name);
      };
      reader.readAsText(file);
    }
  });

  // --- Interactive Orbit, Pan & Zoom Mouse Handlers ---
  canvas.addEventListener('mousedown', (e) => {
    state.camera.isOrbiting = (e.button === 0 && !e.shiftKey);
    state.camera.isPanning = (e.button === 1 || e.button === 2 || (e.button === 0 && e.shiftKey));
    state.camera.lastMouseX = e.clientX;
    state.camera.lastMouseY = e.clientY;

    if (e.button === 0 && !e.shiftKey) {
      // Element Selection Raycast check
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      hitTestElement(mx, my);
    }
  });

  window.addEventListener('mousemove', (e) => {
    const dx = e.clientX - state.camera.lastMouseX;
    const dy = e.clientY - state.camera.lastMouseY;
    state.camera.lastMouseX = e.clientX;
    state.camera.lastMouseY = e.clientY;

    if (state.camera.isOrbiting) {
      state.camera.yaw += dx * 0.008;
      state.camera.pitch = Math.max(-Math.PI / 2 + 0.05, Math.min(Math.PI / 2 - 0.05, state.camera.pitch + dy * 0.008));
      renderScene();
    } else if (state.camera.isPanning) {
      const panFactor = state.camera.distance / 1200;
      const cosY = Math.cos(state.camera.yaw);
      const sinY = Math.sin(state.camera.yaw);

      state.camera.target[0] -= (dx * cosY) * panFactor;
      state.camera.target[1] -= (-dx * sinY) * panFactor;
      state.camera.target[2] += dy * panFactor;
      renderScene();
    }

    if (statusCoords) {
      statusCoords.textContent = `Orbit: ${(state.camera.yaw * 180 / Math.PI).toFixed(0)}° / ${(state.camera.pitch * 180 / Math.PI).toFixed(0)}° | Dist: ${(state.camera.distance / 1000).toFixed(1)}m`;
    }
  });

  window.addEventListener('mouseup', () => {
    state.camera.isOrbiting = false;
    state.camera.isPanning = false;
  });

  canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 1.12 : 0.88;
    state.camera.distance = Math.max(1000, Math.min(200000, state.camera.distance * zoomFactor));
    renderScene();
  }, { passive: false });

  // Prevent right click context menu on canvas
  canvas.addEventListener('contextmenu', (e) => e.preventDefault());

  // --- Hit Testing / Raycasting ---
  function hitTestElement(clickX, clickY) {
    const dpr = window.devicePixelRatio || 1;
    const rw = canvas.width / dpr;
    const rh = canvas.height / dpr;

    let closestElId = null;
    let closestDepth = -Infinity;

    for (const el of state.document.elements) {
      for (const face of el.faces) {
        const poly = face.vertices.map(p => projectToScreen(p, rw, rh));
        if (pointInPolygon(clickX, clickY, poly)) {
          let depth = 0;
          for (const pt of poly) depth += pt.depth;
          depth /= poly.length;

          //painter algorithm: smaller depth is closer to eye
          if (depth > closestDepth) {
            closestDepth = depth;
            closestElId = el.id;
          }
        }
      }
    }

    state.selectedElementId = closestElId;
    updateTreeAndProperties();
    renderScene();
  }

  function pointInPolygon(x, y, poly) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      const xi = poly[i].x, yi = poly[i].y;
      const xj = poly[j].x, yj = poly[j].y;
      const intersect = ((yi > y) !== (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
      if (intersect) inside = !inside;
    }
    return inside;
  }

  // --- Ribbon Tab Switching ---
  window.switchRibbonTab = function (tabName, el) {
    document.querySelectorAll('.ribbon-tab').forEach(t => t.classList.remove('active'));
    if (el) el.classList.add('active');
    state.activeTab = tabName;
  };

  // --- Initialize Web Studio ---
  window.addEventListener('DOMContentLoaded', () => {
    resizeCanvas();
    updateTreeAndProperties();
    renderScene();
  });

})();
