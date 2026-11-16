# Arch3D Live Sync for Blender (3.6 / 4.x / 5.x / 5.2+)

Real-time bidirectional synchronization bridge connecting **Arch3D OpenBIM** directly to **Blender Cycles & EEVEE**.

---

## Compatibility

- **Blender 5.2+ & 5.0**: Fully compatible.
- **Blender 4.2 LTS & 4.3**: Fully compatible (both as Extension or Addon).
- **Blender 3.6 LTS & 4.0 - 4.1**: Fully compatible.

---

## Installation Guide

### Option A: Direct Installation (All Blender Versions: 3.6, 4.x, 5.0, 5.2+)
1. In Blender, open **Edit** > **Preferences**.
2. Go to **Add-ons** (or **Get Extensions** in 4.2+).
3. In the top-right corner, click the **dropdown / menu arrow** (or hamburger icon) and choose **Install from Disk...**.
4. Select `arch3d_blender_sync.py` and click **Install Add-on**.
5. Enable the checkbox for **Arch3D Live Sync**.

### Option B: Modern Blender Extension Package (Blender 4.2+ & 5.2+)
1. Place `arch3d_blender_sync.py` (renamed as `__init__.py`) and `blender_manifest.toml` into a `.zip` archive:
   ```
   arch3d_live_sync.zip
   ├── blender_manifest.toml
   └── __init__.py
   ```
2. In Blender Preferences > **Get Extensions**, click the top-right menu and choose **Install from Disk...**.
3. Select `arch3d_live_sync.zip`.

---

## How to Live Sync

1. In Blender's **3D Viewport**, press <kbd>N</kbd> to open the right sidebar.
2. Click the **Arch3D** tab.
3. Click **Start Live Sync Listener** (default port `9876`).
4. In Arch3D, open your project and click **Sync Blender** on the Ribbon (Home or Insert tab).
5. All BIM elements (walls, slabs, roofs, columns, beams, curtain walls, doors, windows) appear instantly in the `Arch3D_Model` collection with physically based materials configured for Cycles and EEVEE!
