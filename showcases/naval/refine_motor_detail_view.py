import math
import os

import bpy
from mathutils import Vector


ROOT = os.path.dirname(os.path.abspath(__file__))
OUT_BLEND = os.path.join(ROOT, "submarino_escuela_naval_ultra_detallado.blend")
MOTOR_RENDER = os.path.join(ROOT, "motor_submarino_detallado_preview.png")
MICRO_RENDER = os.path.join(ROOT, "motor_submarino_microdetalles_preview.png")


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def set_text(name, size=None, loc=None, hide=False):
    obj = bpy.data.objects.get(name)
    if not obj:
        return
    if size is not None and hasattr(obj.data, "size"):
        obj.data.size = size
    if loc is not None:
        obj.location = loc
    obj.hide_render = hide


# Make the motor exhibit labels calmer inside the file itself.
set_text("titulo_motores_aparte", 0.13, (1.8, -7.55, 2.28))
set_text("nota_motores_genericos", 0.08, (1.8, -7.55, 2.02))

for obj in bpy.data.objects:
    if obj.type == "FONT" and (
        obj.name.startswith("motor_armado_detalle_lbl")
        or obj.name.startswith("motor_explotado_detalle_lbl")
    ):
        obj.data.size *= 0.72

# For the render, temporarily hide the big exhibit title and the submarine
# background so the motor copies read clearly.
for name in ["titulo_motores_aparte", "nota_motores_genericos"]:
    set_text(name, hide=True)

original_hide = {}
for obj in bpy.data.objects:
    original_hide[obj.name] = obj.hide_render
    keep = (
        obj.type in {"CAMERA", "LIGHT"}
        or obj.name.startswith("motor_")
        or obj.name.startswith("guia_motor_")
        or obj.name.startswith("punto_motor_")
        or obj.name.startswith("plataforma_motores")
    )
    if not keep:
        obj.hide_render = True

cam = bpy.data.objects.get("Camara_detalle_motores")
if cam is None:
    bpy.ops.object.camera_add()
    cam = bpy.context.object
    cam.name = "Camara_detalle_motores"

cam.location = (0.6, -13.2, 4.55)
look_at(cam, (0.35, -6.05, 0.08))
cam.data.type = "ORTHO"
cam.data.ortho_scale = 7.25
bpy.context.scene.camera = cam

bpy.context.scene.render.resolution_x = 2600
bpy.context.scene.render.resolution_y = 1500
bpy.context.scene.render.filepath = MOTOR_RENDER
bpy.ops.render.render(write_still=True)

bpy.ops.object.camera_add(location=(-4.05, -9.6, 2.0))
micro_cam = bpy.context.object
micro_cam.name = "Camara_microdetalles_motor"
look_at(micro_cam, (-4.15, -6.05, 0.25))
micro_cam.data.type = "ORTHO"
micro_cam.data.ortho_scale = 2.35
bpy.context.scene.camera = micro_cam
bpy.context.scene.render.resolution_x = 2200
bpy.context.scene.render.resolution_y = 1400
bpy.context.scene.render.filepath = MICRO_RENDER
bpy.ops.render.render(write_still=True)

for obj in bpy.data.objects:
    if obj.name in original_hide:
        obj.hide_render = original_hide[obj.name]

for name in ["titulo_motores_aparte", "nota_motores_genericos"]:
    set_text(name, hide=False)

bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)
print(f"Render de motor refinado: {MOTOR_RENDER}")
print(f"Render de microdetalles: {MICRO_RENDER}")
