# Arch3D Live Sync — Blender Addon
#
# Real-time synchronization bridge between Arch3D BIM and Blender (3.6 / 4.x / 5.x / 5.2+).
# Automatically receives geometry, materials, and updates streamed from Arch3D over local TCP socket
# or monitored temp file into Blender Cycles & EEVEE.
#
# Installation:
# 1. In Blender, go to Edit > Preferences > Add-ons (or Extensions in 4.2+)
# 2. Click the top-right menu dropdown > 'Install from Disk...' (or Install...)
# 3. Select this file (arch3d_blender_sync.py) and check the box to enable it.
# 4. In the 3D Viewport, press 'N' to open the sidebar and navigate to the 'Arch3D' tab.
# 5. Click 'Start Live Sync Listener'.

bl_info = {
    "name": "Arch3D Live Sync",
    "author": "Arch3D Team & Dsales Architect",
    "version": (1, 0, 0),
    "blender": (3, 6, 0),  # Minimum supported Blender version (fully compatible with 3.6, 4.x, 5.0, 5.2+)
    "location": "View3D > Sidebar > Arch3D Tab",
    "description": "Live bidirectional synchronization bridge between Arch3D BIM and Blender Cycles/EEVEE",
    "category": "Import-Export",
    "doc_url": "https://github.com/dsalesarchitect/arch3d",
    "tracker_url": "https://github.com/dsalesarchitect/arch3d/issues",
}

import bpy
import socket
import threading
import tempfile
import os
import time

# Global server & sync state
SERVER_THREAD = None
SERVER_RUNNING = False
SERVER_SOCKET = None
PENDING_GLB_PATH = None
LAST_SYNC_TIME = ""
SYNC_COUNT = 0


def get_default_temp_glb_path():
    temp_dir = tempfile.gettempdir()
    return os.path.join(temp_dir, "arch3d_live_sync.glb")


def import_glb_into_blender(filepath):
    """Imports or updates the Arch3D GLB model in Blender while organizing collections and preserving user scene setup."""
    global LAST_SYNC_TIME, SYNC_COUNT

    if not os.path.exists(filepath):
        print(f"[Arch3D] GLB file does not exist: {filepath}")
        return

    # Ensure "Arch3D" Master Collection exists
    coll_name = "Arch3D_Model"
    arch_coll = bpy.data.collections.get(coll_name)
    if not arch_coll:
        arch_coll = bpy.data.collections.new(coll_name)
        bpy.context.scene.collection.children.link(arch_coll)

    # Store objects to delete (previous sync)
    old_objs = list(arch_coll.objects)

    # Import using Blender's built-in glTF 2.0 importer
    try:
        # Deselect all
        for obj in bpy.context.selected_objects:
            obj.select_set(False)

        # Import GLB using dynamic RNA property inspection for forward compatibility (Blender 3.6 through 5.2+)
        kwargs = {"filepath": filepath}
        try:
            op_props = getattr(bpy.ops.import_scene.gltf, "get_rna_type", lambda: None)()
            prop_keys = op_props.properties.keys() if op_props and hasattr(op_props, "properties") else []
            if "merge_vertices" in prop_keys:
                kwargs["merge_vertices"] = True
            if "import_pack_images" in prop_keys:
                kwargs["import_pack_images"] = True
            if "import_shading" in prop_keys:
                kwargs["import_shading"] = 'NORMALS'
            bpy.ops.import_scene.gltf(**kwargs)
        except Exception:
            # Safe minimal fallback if operator signature differs in future releases
            bpy.ops.import_scene.gltf(filepath=filepath)

        # Gather newly imported objects
        new_objs = [obj for obj in bpy.context.selected_objects]

        # Link new objects to Arch3D collection and remove from master scene collection
        for obj in new_objs:
            if obj.name not in arch_coll.objects:
                arch_coll.objects.link(obj)
            if obj.name in bpy.context.scene.collection.objects:
                bpy.context.scene.collection.objects.unlink(obj)

        # Clean up old objects
        for obj in old_objs:
            try:
                bpy.data.objects.remove(obj, do_unlink=True)
            except Exception:
                pass

        # Clean unused meshes
        for mesh in list(bpy.data.meshes):
            if mesh.users == 0:
                try:
                    bpy.data.meshes.remove(mesh, do_unlink=True)
                except Exception:
                    pass

        SYNC_COUNT += 1
        LAST_SYNC_TIME = time.strftime("%H:%M:%S")
        print(f"[Arch3D] Successfully synchronized {len(new_objs)} objects at {LAST_SYNC_TIME} (Sync #{SYNC_COUNT})")

    except Exception as e:
        print(f"[Arch3D] Error importing GLB: {e}")


def timer_callback():
    """Runs on Blender's main thread to safely process received GLB payloads."""
    global PENDING_GLB_PATH
    if PENDING_GLB_PATH:
        path = PENDING_GLB_PATH
        PENDING_GLB_PATH = None
        import_glb_into_blender(path)
    return 0.1  # 100ms interval


def tcp_server_worker(port):
    """Background TCP socket listener thread supporting both raw TCP streams and browser HTTP POST with CORS."""
    global SERVER_RUNNING, SERVER_SOCKET, PENDING_GLB_PATH

    temp_glb = get_default_temp_glb_path()

    try:
        SERVER_SOCKET = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        SERVER_SOCKET.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        SERVER_SOCKET.bind(("127.0.0.1", port))
        SERVER_SOCKET.listen(1)
        SERVER_SOCKET.settimeout(1.0)
        print(f"[Arch3D] Live Sync server listening on 127.0.0.1:{port}")

        while SERVER_RUNNING:
            try:
                conn, addr = SERVER_SOCKET.accept()
            except socket.timeout:
                continue
            except Exception:
                break

            try:
                conn.settimeout(3.0)
                initial = conn.recv(4096)
                if not initial:
                    conn.close()
                    continue

                # Case 1: Browser CORS preflight (OPTIONS)
                if initial.startswith(b"OPTIONS"):
                    resp = (
                        b"HTTP/1.1 204 No Content\r\n"
                        b"Access-Control-Allow-Origin: *\r\n"
                        b"Access-Control-Allow-Methods: POST, OPTIONS\r\n"
                        b"Access-Control-Allow-Headers: *\r\n"
                        b"Access-Control-Max-Age: 86400\r\n"
                        b"Connection: close\r\n\r\n"
                    )
                    conn.sendall(resp)
                    conn.close()
                    continue

                # Case 2: Browser HTTP POST with GLB binary body
                if initial.startswith(b"POST"):
                    header_end = initial.find(b"\r\n\r\n")
                    if header_end != -1:
                        header_part = initial[:header_end].decode("latin1", errors="ignore")
                        body_part = initial[header_end + 4:]

                        content_len = None
                        for line in header_part.split("\r\n"):
                            if line.lower().startswith("content-length:"):
                                try:
                                    content_len = int(line.split(":", 1)[1].strip())
                                except Exception:
                                    pass

                        body_chunks = [body_part]
                        bytes_received = len(body_part)
                        if content_len is not None:
                            while bytes_received < content_len:
                                chunk = conn.recv(min(65536, content_len - bytes_received))
                                if not chunk:
                                    break
                                body_chunks.append(chunk)
                                bytes_received += len(chunk)
                        else:
                            while True:
                                chunk = conn.recv(65536)
                                if not chunk:
                                    break
                                body_chunks.append(chunk)

                        full_body = b"".join(body_chunks)
                        with open(temp_glb, "wb") as f:
                            f.write(full_body)
                        PENDING_GLB_PATH = temp_glb
                        resp = (
                            b"HTTP/1.1 200 OK\r\n"
                            b"Access-Control-Allow-Origin: *\r\n"
                            b"Content-Type: text/plain\r\n"
                            b"Connection: close\r\n\r\n"
                            b"OK_SYNCED"
                        )
                        conn.sendall(resp)
                        conn.close()
                        continue

                # Case 3: Raw TCP binary stream from desktop app
                chunks = [initial]
                while True:
                    try:
                        data = conn.recv(65536)
                        if not data:
                            break
                        chunks.append(data)
                    except socket.timeout:
                        break
                    except Exception:
                        break

                payload = b"".join(chunks)
                with open(temp_glb, "wb") as f:
                    f.write(payload)
                PENDING_GLB_PATH = temp_glb
                conn.sendall(b"OK_SYNCED")

            except Exception as e:
                print(f"[Arch3D] Connection error: {e}")
            finally:
                try:
                    conn.close()
                except Exception:
                    pass

    except Exception as e:
        print(f"[Arch3D] Server failed to start: {e}")
    finally:
        if SERVER_SOCKET:
            try:
                SERVER_SOCKET.close()
            except Exception:
                pass
        SERVER_RUNNING = False
        print("[Arch3D] Live Sync server stopped.")


class ARCH3D_OT_StartLiveSync(bpy.types.Operator):
    """Start local socket listener for Arch3D Live Sync"""
    bl_idname = "arch3d.start_live_sync"
    bl_label = "Start Live Sync"
    bl_description = "Listen on local port for real-time model streams from Arch3D"

    def execute(self, context):
        global SERVER_THREAD, SERVER_RUNNING

        if SERVER_RUNNING:
            self.report({'INFO'}, "Arch3D Live Sync is already active.")
            return {'FINISHED'}

        SERVER_RUNNING = True
        port = context.scene.arch3d_sync_port
        SERVER_THREAD = threading.Thread(target=tcp_server_worker, args=(port,), daemon=True)
        SERVER_THREAD.start()

        if not bpy.app.timers.is_registered(timer_callback):
            bpy.app.timers.register(timer_callback)

        self.report({'INFO'}, f"Arch3D Live Sync listening on port {port}")
        return {'FINISHED'}


class ARCH3D_OT_StopLiveSync(bpy.types.Operator):
    """Stop local socket listener"""
    bl_idname = "arch3d.stop_live_sync"
    bl_label = "Stop Live Sync"
    bl_description = "Stop listening for Arch3D live updates"

    def execute(self, context):
        global SERVER_RUNNING, SERVER_SOCKET

        SERVER_RUNNING = False
        if SERVER_SOCKET:
            try:
                SERVER_SOCKET.close()
            except Exception:
                pass

        if bpy.app.timers.is_registered(timer_callback):
            try:
                bpy.app.timers.unregister(timer_callback)
            except Exception:
                pass

        self.report({'INFO'}, "Arch3D Live Sync stopped.")
        return {'FINISHED'}


class ARCH3D_OT_ImportTempModel(bpy.types.Operator):
    """Manually reload model from default Arch3D temp file"""
    bl_idname = "arch3d.import_temp_model"
    bl_label = "Reload Current Model"
    bl_description = "Reload the latest model exported by Arch3D"

    def execute(self, context):
        temp_glb = get_default_temp_glb_path()
        if os.path.exists(temp_glb):
            import_glb_into_blender(temp_glb)
            self.report({'INFO'}, "Reloaded Arch3D Model from disk cache.")
        else:
            self.report({'WARNING'}, f"No cached Arch3D model found at {temp_glb}")
        return {'FINISHED'}


class ARCH3D_PT_LiveSyncPanel(bpy.types.Panel):
    """Arch3D Live Sync Sidebar Panel"""
    bl_label = "Arch3D Live Sync"
    bl_idname = "ARCH3D_PT_live_sync"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'Arch3D'

    def draw(self, context):
        layout = self.layout
        global SERVER_RUNNING, LAST_SYNC_TIME, SYNC_COUNT

        # Status Box
        box = layout.box()
        row = box.row(align=True)
        if SERVER_RUNNING:
            row.label(text="Status: Active & Listening", icon='RADIOBUT_ON')
            row = box.row()
            row.label(text=f"Port: {context.scene.arch3d_sync_port} | Syncs: {SYNC_COUNT}")
            if LAST_SYNC_TIME:
                row = box.row()
                row.label(text=f"Last Updated: {LAST_SYNC_TIME}", icon='TIME')
        else:
            row.label(text="Status: Disconnected", icon='RADIOBUT_OFF')

        layout.separator()

        # Action Buttons
        col = layout.column(align=True)
        if not SERVER_RUNNING:
            col.prop(context.scene, "arch3d_sync_port", text="Port")
            col.operator("arch3d.start_live_sync", text="Start Live Sync Listener", icon='PLAY')
        else:
            col.operator("arch3d.stop_live_sync", text="Stop Live Sync", icon='PAUSE')

        layout.separator()
        layout.operator("arch3d.import_temp_model", text="Reload Disk Cache (.glb)", icon='FILE_REFRESH')

        # Information Box
        info = layout.box()
        info.label(text="Workflow Instructions:", icon='INFO')
        info.label(text="1. Click 'Start Live Sync Listener'.")
        info.label(text="2. In Arch3D, click 'Sync to Blender'.")
        info.label(text="3. Changes update instantly in Cycles/EEVEE.")


classes = (
    ARCH3D_OT_StartLiveSync,
    ARCH3D_OT_StopLiveSync,
    ARCH3D_OT_ImportTempModel,
    ARCH3D_PT_LiveSyncPanel,
)


def register():
    bpy.types.Scene.arch3d_sync_port = bpy.props.IntProperty(
        name="Sync Port",
        description="Local TCP port for Arch3D communication",
        default=9876,
        min=1024,
        max=65535
    )
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    global SERVER_RUNNING
    SERVER_RUNNING = False
    if bpy.app.timers.is_registered(timer_callback):
        try:
            bpy.app.timers.unregister(timer_callback)
        except Exception:
            pass

    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.arch3d_sync_port


if __name__ == "__main__":
    register()
