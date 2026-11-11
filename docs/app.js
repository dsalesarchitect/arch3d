/**
 * Arch3D Landing Page — Interactive 3D BIM Viewport Simulation & UI Controller
 */

(function () {
  'use strict';

  // --- Theme Toggle ---
  const themeToggleBtn = document.getElementById('themeToggleBtn');
  const currentTheme = localStorage.getItem('arch3d-theme') || 'dark';

  const sunIcon = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>`;
  const moonIcon = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>`;

  function setTheme(theme) {
    if (theme === 'light') {
      document.documentElement.setAttribute('data-theme', 'light');
      if (themeToggleBtn) themeToggleBtn.innerHTML = moonIcon;
    } else {
      document.documentElement.removeAttribute('data-theme');
      if (themeToggleBtn) themeToggleBtn.innerHTML = sunIcon;
    }
    localStorage.setItem('arch3d-theme', theme);
  }

  setTheme(currentTheme);

  if (themeToggleBtn) {
    themeToggleBtn.addEventListener('click', () => {
      const isLight = document.documentElement.getAttribute('data-theme') === 'light';
      setTheme(isLight ? 'dark' : 'light');
    });
  }

  // --- Mobile Navigation ---
  const mobileToggle = document.getElementById('mobileNavToggle');
  const navLinks = document.getElementById('navLinks');
  if (mobileToggle && navLinks) {
    mobileToggle.addEventListener('click', () => {
      navLinks.classList.toggle('open');
    });
  }

  // --- Copy Command Helper ---
  window.copyCommand = function (text, btnElement) {
    navigator.clipboard.writeText(text).then(() => {
      const originalText = btnElement.innerHTML;
      btnElement.innerHTML = `<code>✓ Copied to Clipboard!</code>`;
      btnElement.style.borderColor = 'var(--accent-primary)';
      setTimeout(() => {
        btnElement.innerHTML = originalText;
        btnElement.style.borderColor = '';
      }, 2200);
    }).catch(err => {
      console.error('Clipboard copy failed: ', err);
    });
  };

  // --- Interactive 3D BIM Model Viewport Simulation ---
  const canvas = document.getElementById('bimCanvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');

  let width = canvas.clientWidth;
  let height = canvas.clientHeight;
  let dpr = window.devicePixelRatio || 1;

  function resizeCanvas() {
    width = canvas.parentElement.clientWidth;
    height = canvas.parentElement.clientHeight;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
  }

  window.addEventListener('resize', resizeCanvas);
  resizeCanvas();

  // Camera State
  let yaw = 0.785; // 45 deg
  let pitch = 0.45; // ~26 deg isometric
  let distance = 850;
  let isDragging = false;
  let lastMouseX = 0;
  let lastMouseY = 0;
  let autoRotate = true;
  let visualStyle = 'shaded'; // 'shaded', 'hidden-line', 'consistent', 'wireframe'

  // Style Picker Buttons
  const styleBtns = document.querySelectorAll('.viewer-btn[data-style]');
  styleBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      styleBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      visualStyle = btn.getAttribute('data-style');
    });
  });

  const resetCamBtn = document.getElementById('resetCamBtn');
  if (resetCamBtn) {
    resetCamBtn.addEventListener('click', () => {
      yaw = 0.785;
      pitch = 0.45;
      distance = 850;
    });
  }

  // Mouse / Touch Controls
  canvas.addEventListener('mousedown', e => {
    isDragging = true;
    autoRotate = false;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
  });

  window.addEventListener('mouseup', () => {
    isDragging = false;
  });

  window.addEventListener('mousemove', e => {
    if (!isDragging) return;
    const dx = e.clientX - lastMouseX;
    const dy = e.clientY - lastMouseY;
    yaw += dx * 0.008;
    pitch = Math.max(0.05, Math.min(Math.PI * 0.48, pitch - dy * 0.008));
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
  });

  canvas.addEventListener('wheel', e => {
    e.preventDefault();
    distance = Math.max(400, Math.min(1800, distance + e.deltaY * 0.8));
    autoRotate = false;
  }, { passive: false });

  // 3D Model Geometry Definition (Two-Storey BIM Structure)
  function buildBimBuilding() {
    const solids = [];

    // Helper: create 3D box solid with 6 watertight faces
    function addBox(x, y, z, w, d, h, color, tag) {
      const x0 = x - w / 2, x1 = x + w / 2;
      const y0 = y - d / 2, y1 = y + d / 2;
      const z0 = z, z1 = z + h;

      const vertices = [
        [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0], // Bottom
        [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]  // Top
      ];

      const faces = [
        { v: [0, 1, 2, 3], norm: [0, 0, -1] }, // Bottom
        { v: [4, 7, 6, 5], norm: [0, 0, 1] },  // Top
        { v: [0, 4, 5, 1], norm: [0, -1, 0] }, // Front
        { v: [2, 6, 7, 3], norm: [0, 1, 0] },  // Back
        { v: [0, 3, 7, 4], norm: [-1, 0, 0] }, // Left
        { v: [1, 5, 6, 2], norm: [1, 0, 0] },  // Right
      ];

      solids.push({ vertices, faces, color, tag });
    }

    // 1. Foundation Ground Slab
    addBox(0, 0, 0, 480, 360, 18, [180, 185, 195], 'Foundation Slab');

    // 2. First Storey Walls
    addBox(0, -170, 18, 460, 20, 140, [220, 224, 230], 'Exterior Wall');
    addBox(0, 170, 18, 460, 20, 140, [220, 224, 230], 'Exterior Wall');
    addBox(-220, 0, 18, 20, 320, 140, [210, 215, 222], 'Exterior Wall');
    addBox(220, 0, 18, 20, 320, 140, [210, 215, 222], 'Exterior Wall');
    // Interior dividing partition
    addBox(0, 0, 18, 20, 320, 140, [230, 233, 238], 'Interior Wall');

    // 3. Structural Columns (Concrete square columns at bay corners)
    const colCoords = [
      [-190, -140], [0, -140], [190, -140],
      [-190, 0], [0, 0], [190, 0],
      [-190, 140], [0, 140], [190, 140]
    ];
    for (const [cx, cy] of colCoords) {
      addBox(cx, cy, 18, 24, 24, 140, [150, 160, 175], 'Structural Column');
    }

    // 4. Glazed Windows & Curtain Walls (Cyan glass)
    addBox(-100, -172, 50, 60, 6, 60, [0, 210, 255, 0.55], 'Glazed Window');
    addBox(100, -172, 50, 60, 6, 60, [0, 210, 255, 0.55], 'Glazed Window');
    addBox(222, 0, 40, 6, 90, 80, [0, 210, 255, 0.6], 'Curtain Wall');

    // 5. Door Entrance Cutout Frame (Warm wood)
    addBox(0, -172, 18, 50, 6, 90, [180, 100, 50], 'Door Frame');

    // 6. Level 2 Intermediate Concrete Deck Slab
    addBox(0, 0, 158, 470, 350, 16, [170, 175, 185], 'Floor Deck Slab');

    // 7. Second Storey Framing & Walls
    addBox(0, -165, 174, 450, 18, 120, [225, 228, 235], 'Exterior Wall L2');
    addBox(0, 165, 174, 450, 18, 120, [225, 228, 235], 'Exterior Wall L2');
    addBox(-215, 0, 174, 18, 312, 120, [215, 218, 226], 'Exterior Wall L2');
    addBox(215, 0, 174, 18, 312, 120, [215, 218, 226], 'Exterior Wall L2');

    // Level 2 Columns
    for (const [cx, cy] of colCoords) {
      addBox(cx, cy, 174, 22, 22, 120, [150, 160, 175], 'Structural Column L2');
    }
    // Level 2 Windows
    addBox(-110, -167, 200, 55, 6, 50, [0, 210, 255, 0.55], 'Glazed Window');
    addBox(110, -167, 200, 55, 6, 50, [0, 210, 255, 0.55], 'Glazed Window');
    addBox(0, 167, 200, 80, 6, 50, [0, 210, 255, 0.55], 'Glazed Window');

    // 8. Sloped Gable Roof with Overhang Soffits
    const rZ = 294;
    const rRidgeZ = 370;
    const rW = 490 / 2;
    const rD = 370 / 2;

    const roofVerts = [
      [-rW, -rD, rZ], [rW, -rD, rZ], [rW, rD, rZ], [-rW, rD, rZ], // Eaves
      [-rW, 0, rRidgeZ], [rW, 0, rRidgeZ]                         // Ridge
    ];
    const roofFaces = [
      { v: [0, 1, 5, 4], norm: [0, -0.6, 0.8] }, // Front pitch
      { v: [1, 2, 5], norm: [1, 0, 0] },          // Right gable
      { v: [2, 3, 4, 5], norm: [0, 0.6, 0.8] },  // Back pitch
      { v: [3, 0, 4], norm: [-1, 0, 0] },         // Left gable
    ];
    solids.push({ vertices: roofVerts, faces: roofFaces, color: [210, 110, 50], tag: 'Gable Roof' });

    return solids;
  }

  const bimModel = buildBimBuilding();

  // Rendering Loop
  function render() {
    ctx.clearRect(0, 0, width, height);

    if (autoRotate) {
      yaw += 0.003;
    }

    const cosY = Math.cos(yaw);
    const sinY = Math.sin(yaw);
    const cosP = Math.cos(pitch);
    const sinP = Math.sin(pitch);

    const lightDir = [-0.577, 0.577, 0.577];

    // Project all faces and sort back-to-front (Painter's algorithm for WebGL-free portability)
    const polygonQueue = [];

    for (const solid of bimModel) {
      for (const face of solid.faces) {
        const poly3d = face.v.map(idx => solid.vertices[idx]);

        // Transform vertices to camera view space
        const pts2d = [];
        let avgZ = 0;

        for (const [x, y, z] of poly3d) {
          // Orbit around Z center (offset Z down by 180 for building center)
          const zRel = z - 180;
          const x1 = x * cosY - y * sinY;
          const y1 = x * sinY + y * cosY;
          const z1 = zRel;

          const y2 = y1 * cosP - z1 * sinP;
          const z2 = y1 * sinP + z1 * cosP;

          // Ortho/isometric perspective projection
          const fovFactor = distance / (distance + y2 * 0.4);
          const screenX = width / 2 + x1 * fovFactor;
          const screenY = height / 2 - z2 * fovFactor;

          pts2d.push([screenX, screenY]);
          avgZ += y2;
        }

        avgZ /= poly3d.length;

        // Face normal in view space
        const [nx, ny, nz] = face.norm;
        const normX = nx * cosY - ny * sinY;
        const normY = (nx * sinY + ny * cosY) * cosP - nz * sinP;
        const normZ = (nx * sinY + ny * cosY) * sinP + nz * cosP;

        // Backface culling: cull faces pointing away from camera
        if (normY > 0.05 && visualStyle !== 'wireframe') {
          continue;
        }

        // Shading calculation
        const dot = Math.max(0.15, -(normX * lightDir[0] + normY * lightDir[1] + normZ * lightDir[2]));

        polygonQueue.push({
          pts: pts2d,
          depth: avgZ,
          color: solid.color,
          dot,
          tag: solid.tag
        });
      }
    }

    // Sort by camera depth
    polygonQueue.sort((a, b) => b.depth - a.depth);

    const isLight = document.documentElement.getAttribute('data-theme') === 'light';
    const edgeColor = isLight ? 'rgba(30, 41, 59, 0.75)' : 'rgba(255, 255, 255, 0.65)';
    const wireColor = isLight ? '#9333ea' : '#d946ef';

    // Draw Polygons
    for (const poly of polygonQueue) {
      if (poly.pts.length < 3) continue;

      ctx.beginPath();
      ctx.moveTo(poly.pts[0][0], poly.pts[0][1]);
      for (let i = 1; i < poly.pts.length; i++) {
        ctx.lineTo(poly.pts[i][0], poly.pts[i][1]);
      }
      ctx.closePath();

      if (visualStyle === 'shaded') {
        const [r, g, b, a] = poly.color;
        const alpha = a !== undefined ? a : 1.0;
        const litR = Math.round(Math.min(255, r * (0.45 + 0.55 * poly.dot)));
        const litG = Math.round(Math.min(255, g * (0.45 + 0.55 * poly.dot)));
        const litB = Math.round(Math.min(255, b * (0.45 + 0.55 * poly.dot)));
        ctx.fillStyle = `rgba(${litR}, ${litG}, ${litB}, ${alpha})`;
        ctx.fill();
        ctx.strokeStyle = edgeColor;
        ctx.lineWidth = 1.0;
        ctx.stroke();
      } else if (visualStyle === 'consistent') {
        const [r, g, b, a] = poly.color;
        const alpha = a !== undefined ? a : 1.0;
        ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${alpha})`;
        ctx.fill();
        ctx.strokeStyle = edgeColor;
        ctx.lineWidth = 1.0;
        ctx.stroke();
      } else if (visualStyle === 'hidden-line') {
        ctx.fillStyle = isLight ? '#ffffff' : '#14181f';
        ctx.fill();
        ctx.strokeStyle = isLight ? '#0f172a' : '#f0f3f6';
        ctx.lineWidth = 1.2;
        ctx.stroke();
      } else if (visualStyle === 'wireframe') {
        ctx.strokeStyle = wireColor;
        ctx.lineWidth = 1.0;
        ctx.stroke();
      }
    }

    requestAnimationFrame(render);
  }

  requestAnimationFrame(render);
})();
