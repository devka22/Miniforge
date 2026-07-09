import math
import os

import bpy
from mathutils import Vector


ROOT = os.path.dirname(os.path.abspath(__file__))
OUT_BLEND = os.path.join(ROOT, "presentacion_naval_submarino_gerald_ford.blend")
RENDER_COMPONENTS = os.path.join(ROOT, "gerald_ford_componentes_preview.png")
RENDER_PROPULSION = os.path.join(ROOT, "gerald_ford_propulsion_microdetalles.png")


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


component_prefixes = (
    "ford_componentes",
    "ford_propulsion_aparte",
    "ford_radar_aparte",
    "ford_elevador_aparte",
    "ford_emals_aparte",
    "ford_armamento_aparte",
    "guia_ford_propulsion_aparte",
    "guia_ford_radar_aparte",
    "guia_ford_elevador_aparte",
    "guia_ford_emals_aparte",
    "guia_ford_armamento_aparte",
    "punto_ford_propulsion_aparte",
    "punto_ford_radar_aparte",
    "punto_ford_elevador_aparte",
    "punto_ford_emals_aparte",
    "punto_ford_armamento_aparte",
)

original_hide = {obj.name: obj.hide_render for obj in bpy.data.objects}
original_sizes = {}
for obj in bpy.data.objects:
    if obj.type == "FONT":
        original_sizes[obj.name] = obj.data.size
    keep = obj.type in {"CAMERA", "LIGHT"} or obj.name.startswith(component_prefixes)
    obj.hide_render = not keep

for obj in bpy.data.objects:
    if obj.type == "FONT" and obj.name.startswith(component_prefixes):
        obj.data.size = min(obj.data.size, 0.12)
for name in ["ford_componentes_titulo"]:
    obj = bpy.data.objects.get(name)
    if obj:
        obj.hide_render = True

bpy.ops.object.light_add(type="AREA", location=(1.0, 21.0, 7.0))
light = bpy.context.object
light.name = "ford_componentes_luz_refinada"
light.data.energy = 650
light.data.size = 8

cam = bpy.data.objects.get("Camara_Gerald_Ford_componentes_refinada")
if cam is None:
    bpy.ops.object.camera_add()
    cam = bpy.context.object
    cam.name = "Camara_Gerald_Ford_componentes_refinada"

cam.location = (1.0, 17.5, 8.2)
look_at(cam, (1.0, 28.0, 0.35))
cam.data.type = "ORTHO"
cam.data.ortho_scale = 16.2
bpy.context.scene.camera = cam
bpy.context.scene.render.resolution_x = 2600
bpy.context.scene.render.resolution_y = 1500
bpy.context.scene.render.filepath = RENDER_COMPONENTS
bpy.ops.render.render(write_still=True)

# A closer propulsion render where the bolts, shaft, turbine rings and propeller read clearly.
for obj in bpy.data.objects:
    keep = obj.type in {"CAMERA", "LIGHT"} or obj.name.startswith((
        "ford_propulsion_aparte",
        "guia_ford_propulsion_aparte",
        "punto_ford_propulsion_aparte",
    ))
    obj.hide_render = not keep

micro = bpy.data.objects.get("Camara_Gerald_Ford_propulsion_micro")
if micro is None:
    bpy.ops.object.camera_add()
    micro = bpy.context.object
    micro.name = "Camara_Gerald_Ford_propulsion_micro"

for obj in bpy.data.objects:
    if obj.type == "FONT" and obj.name.startswith("ford_propulsion_aparte"):
        obj.hide_render = True

micro.location = (-8.7, 22.8, 3.8)
look_at(micro, (-8.65, 27.75, 0.25))
micro.data.type = "ORTHO"
micro.data.ortho_scale = 4.8
bpy.context.scene.camera = micro
bpy.context.scene.render.resolution_x = 2200
bpy.context.scene.render.resolution_y = 1400
bpy.context.scene.render.filepath = RENDER_PROPULSION
bpy.ops.render.render(write_still=True)

for obj in bpy.data.objects:
    if obj.name in original_hide:
        obj.hide_render = original_hide[obj.name]
    if obj.name in original_sizes and obj.type == "FONT":
        obj.data.size = original_sizes[obj.name]

bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)
print(f"Render componentes refinado: {RENDER_COMPONENTS}")
print(f"Render propulsion microdetalles: {RENDER_PROPULSION}")
