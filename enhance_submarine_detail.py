import math
import os

import bpy
from mathutils import Vector


ROOT = os.path.dirname(os.path.abspath(__file__))
OUT_BLEND = os.path.join(ROOT, "submarino_escuela_naval_ultra_detallado.blend")
MAIN_RENDER = os.path.join(ROOT, "submarino_escuela_naval_ultra_detallado_preview.png")
MOTOR_RENDER = os.path.join(ROOT, "motor_submarino_detallado_preview.png")


def mat(name, color, metallic=0.0, roughness=0.55, alpha=1.0):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    bsdf = material.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        if "Base Color" in bsdf.inputs:
            bsdf.inputs["Base Color"].default_value = color
        if "Metallic" in bsdf.inputs:
            bsdf.inputs["Metallic"].default_value = metallic
        if "Roughness" in bsdf.inputs:
            bsdf.inputs["Roughness"].default_value = roughness
        if "Alpha" in bsdf.inputs:
            bsdf.inputs["Alpha"].default_value = alpha
    material.diffuse_color = color
    if alpha < 1:
        material.blend_method = "BLEND"
        material.show_transparent_back = True
        if hasattr(material, "surface_render_method"):
            material.surface_render_method = "BLENDED"
    return material


M = {
    "bolt": mat("detalle tuercas acero satinado", (0.74, 0.77, 0.76, 1), 0.55, 0.26),
    "washer": mat("detalle arandelas oscuras", (0.07, 0.075, 0.075, 1), 0.5, 0.3),
    "panel": mat("detalle placas inspeccion", (0.18, 0.205, 0.205, 1), 0.05, 0.46),
    "rubber": mat("detalle juntas caucho", (0.01, 0.012, 0.012, 1), 0.0, 0.72),
    "brass": mat("detalle laton terminales", (0.92, 0.62, 0.18, 1), 0.5, 0.25),
    "copper": mat("detalle cobre bobinas", (0.95, 0.36, 0.1, 1), 0.6, 0.22),
    "red": mat("detalle cable rojo", (0.86, 0.04, 0.035, 1), 0.0, 0.35),
    "blue": mat("detalle cable azul", (0.03, 0.28, 0.92, 1), 0.0, 0.35),
    "yellow": mat("detalle cable amarillo", (1.0, 0.78, 0.06, 1), 0.0, 0.35),
    "green": mat("detalle cable verde", (0.05, 0.68, 0.25, 1), 0.0, 0.35),
    "orange": mat("detalle linea anotacion naranja", (1.0, 0.67, 0.16, 1), 0.0, 0.32),
    "white": mat("detalle texto blanco", (0.96, 0.97, 0.93, 1), 0.0, 0.52),
    "soft": mat("detalle texto celeste suave", (0.68, 0.82, 0.9, 1), 0.0, 0.52),
    "motor": mat("motor copia carcasa azul oscuro", (0.045, 0.145, 0.19, 1), 0.2, 0.34),
    "motor_trans": mat("motor carcasa seccionada transparente", (0.045, 0.145, 0.19, 0.33), 0.2, 0.28, 0.33),
    "stator": mat("motor estator laminas", (0.18, 0.21, 0.22, 1), 0.35, 0.3),
    "rotor": mat("motor rotor acero", (0.55, 0.58, 0.56, 1), 0.55, 0.22),
    "bearing": mat("motor rodamientos", (0.12, 0.13, 0.135, 1), 0.65, 0.18),
    "fan": mat("motor ventilador", (0.08, 0.09, 0.095, 1), 0.2, 0.35),
    "gasket": mat("motor juntas negras", (0.005, 0.006, 0.006, 1), 0.0, 0.7),
    "floor": mat("plataforma detalle extendida", (0.065, 0.07, 0.07, 1), 0.0, 0.72),
    "glass": mat("detalle vidrio indicador", (0.06, 0.17, 0.2, 0.56), 0.0, 0.18, 0.56),
    "nameplate": mat("detalle placa identificacion", (0.9, 0.72, 0.28, 1), 0.55, 0.24),
}


def shade(obj):
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    try:
        bpy.ops.object.shade_smooth()
    except Exception:
        pass
    obj.select_set(False)
    return obj


def bevel(obj, width=0.012, segments=2):
    mod = obj.modifiers.new("micro bisel", "BEVEL")
    mod.width = width
    mod.segments = segments
    mod.affect = "EDGES"
    obj.modifiers.new("normales detalle", "WEIGHTED_NORMAL")
    return obj


def cube(name, loc, scale, material, bevel_width=0.0, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    if bevel_width:
        bevel(obj, bevel_width, 2)
    return obj


def cyl(name, loc, radius, depth, material, vertices=32, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=loc, rotation=rot
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    shade(obj)
    return obj


def sphere(name, loc, scale, material, segments=24, rings=12):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1, location=loc
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(material)
    shade(obj)
    return obj


def torus(name, loc, major, minor, material, rot=(0, 0, 0), segments=64):
    bpy.ops.mesh.primitive_torus_add(
        major_segments=segments,
        minor_segments=10,
        major_radius=major,
        minor_radius=minor,
        location=loc,
        rotation=rot,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    shade(obj)
    return obj


def curve(name, pts, material, bevel_depth=0.018):
    c = bpy.data.curves.new(name, "CURVE")
    c.dimensions = "3D"
    c.resolution_u = 3
    c.bevel_depth = bevel_depth
    c.bevel_resolution = 4
    s = c.splines.new("POLY")
    s.points.add(len(pts) - 1)
    for p, co in zip(s.points, pts):
        p.co = (co[0], co[1], co[2], 1)
    obj = bpy.data.objects.new(name, c)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(material)
    return obj


def label(name, text, loc, target=None, size=0.13, material=None):
    if material is None:
        material = M["white"]
    bpy.ops.object.text_add(location=loc, rotation=(math.radians(62), 0, 0))
    obj = bpy.context.object
    obj.name = name
    obj.data.body = text
    obj.data.align_x = "CENTER"
    obj.data.align_y = "CENTER"
    obj.data.size = size
    obj.data.materials.append(material)
    if target:
        curve(f"guia_{name}", [loc, target], M["orange"], 0.008)
        sphere(f"punto_{name}", target, (0.035, 0.035, 0.035), M["orange"], 12, 6)
    return obj


def bolt(name, loc, radius=0.04, depth=0.025, axis="X", material=None):
    if material is None:
        material = M["bolt"]
    rot = {
        "X": (0, math.radians(90), 0),
        "Y": (math.radians(90), 0, 0),
        "Z": (0, 0, 0),
    }[axis]
    obj = cyl(name, loc, radius, depth, material, 6, rot)
    bevel(obj, radius * 0.12, 1)
    return obj


def washer(name, loc, major=0.052, minor=0.006, axis="X"):
    rot = {
        "X": (0, math.radians(90), 0),
        "Y": (math.radians(90), 0, 0),
        "Z": (0, 0, 0),
    }[axis]
    return torus(name, loc, major, minor, M["washer"], rot, 36)


def bolt_ring(prefix, x, center_y, center_z, radius, count, bolt_radius=0.035):
    for i in range(count):
        a = 2 * math.pi * i / count
        y = center_y + radius * math.cos(a)
        z = center_z + radius * math.sin(a)
        washer(f"{prefix}_arandela_{i+1:02d}", (x - 0.012, y, z), bolt_radius * 1.42, 0.004, "X")
        bolt(f"{prefix}_tuerca_hex_{i+1:02d}", (x - 0.035, y, z), bolt_radius, 0.026, "X")


def screw_panel(prefix, x, y, z, w, h, material=M["panel"]):
    cube(f"{prefix}_placa", (x, y, z), (0.035, w, h), material, 0.006)
    for sy in [-w / 2 + 0.05, w / 2 - 0.05]:
        for sz in [-h / 2 + 0.05, h / 2 - 0.05]:
            bolt(f"{prefix}_tornillo_{sy:.2f}_{sz:.2f}", (x - 0.025, y + sy, z + sz), 0.018, 0.014, "X")


def gauge(prefix, loc, radius=0.07):
    x, y, z = loc
    cyl(f"{prefix}_caja", (x, y, z), radius, 0.018, M["washer"], 32, (math.radians(90), 0, 0))
    cyl(f"{prefix}_vidrio", (x, y - 0.011, z), radius * 0.82, 0.006, M["glass"], 32, (math.radians(90), 0, 0))
    curve(f"{prefix}_aguja", [(x, y - 0.02, z), (x + radius * 0.48, y - 0.022, z + radius * 0.24)], M["red"], 0.004)
    for i in range(7):
        a = math.radians(210 - 240 * i / 6)
        p1 = (x + radius * 0.66 * math.cos(a), y - 0.022, z + radius * 0.66 * math.sin(a))
        p2 = (x + radius * 0.78 * math.cos(a), y - 0.022, z + radius * 0.78 * math.sin(a))
        curve(f"{prefix}_marca_{i}", [p1, p2], M["white"], 0.0025)


def add_detail_collection():
    collection = bpy.data.collections.new("DETALLE agregado: tornilleria, motor y etiquetas")
    bpy.context.scene.collection.children.link(collection)
    return collection


def detail_existing_submarine():
    # More hull rivets and inspection plates along the cutaway.
    for theta in [188, 210, 330, 350]:
        t = math.radians(theta)
        for i in range(22):
            x = -9.35 + 18.7 * i / 21
            y = 2.08 * math.cos(t)
            z = 2.08 * math.sin(t)
            sphere(f"micro_remache_casco_{theta}_{i:02d}", (x, y, z), (0.025, 0.025, 0.025), M["bolt"], 12, 6)

    for i, x in enumerate([-7.6, -4.55, -2.75, -0.65, 1.15, 3.0, 5.15, 7.35]):
        screw_panel(f"panel_inspeccion_casco_{i+1}", x, -1.73, 0.42, 0.48, 0.32)
    label(
        "etiqueta_micro_tornilleria_casco",
        "Remaches, paneles\n y tornillos visibles",
        (-6.6, -4.85, 1.95),
        (-7.6, -1.73, 0.42),
        0.15,
    )

    # Bulkhead bolts around every watertight ring.
    for x in [-8.95, -5.0, -1.25, 2.35, 4.15, 6.45, 9.35]:
        bolt_ring(f"mamparo_detalle_{x}", x - 0.065, 0, -0.03, 1.18, 18, 0.026)
    label("etiqueta_tuercas_mamparo", "Tuercas hexagonales\nsobre mamparos", (-0.25, -4.9, 1.62), (-1.25, -0.72, 0.9), 0.15)

    # Pipe clamps and small valve labels.
    pipe_specs = [
        ("lastre", M["blue"], -1.35, -0.95, [-7.4, -6.2, -4.9, -3.6, -2.1, -0.8, 0.8, 2.2, 3.6, 5.0, 6.3, 7.5]),
        ("aire", M["green"], 1.22, 0.92, [-6.2, -5.1, -3.8, -2.5, -1.2, 0.0, 1.3, 2.7, 4.0, 5.2]),
        ("hidraulico", M["yellow"], -1.05, 0.55, [-6.7, -5.4, -4.0, -2.7, -1.2, 0.2, 1.6, 3.0, 4.4, 5.7, 7.0]),
        ("incendio", M["red"], -0.92, 1.04, [-7.4, -6.0, -4.6, -3.2, -1.8, -0.4, 1.0, 2.4, 3.8, 5.2, 6.6]),
    ]
    for name, material, y, z, xs in pipe_specs:
        for j, x in enumerate(xs):
            cube(f"brida_{name}_{j:02d}_abrazadera", (x, y, z), (0.035, 0.13, 0.13), M["washer"], 0.003)
            bolt(f"brida_{name}_{j:02d}_tornillo_a", (x - 0.025, y - 0.07, z), 0.014, 0.012, "X")
            bolt(f"brida_{name}_{j:02d}_tornillo_b", (x - 0.025, y + 0.07, z), 0.014, 0.012, "X")
    label("etiqueta_bridas", "Bridas y abrazaderas\nen cada tuberia", (4.1, -4.92, 1.52), (3.6, -1.35, -0.95), 0.15)

    # Gauges and fine details on control and machinery panels.
    for i, x in enumerate([-4.05, -3.8, -3.25, -2.7, 6.75, 7.0, 8.7]):
        gauge(f"manometro_panel_{i+1}", (x, -0.965 if x < -2 else 0.85, 0.25), 0.055)
    label("etiqueta_manometros", "Manometros con aguja\ny escala", (-4.65, -4.9, 0.82), (-4.05, -0.965, 0.25), 0.15)

    for i, (x, y, z) in enumerate([
        (-3.8, -0.93, -0.05), (-3.25, -0.93, -0.05), (-2.7, -0.93, -0.05),
        (-5.72, -1.02, -0.08), (6.05, -0.2, 0.02), (6.05, 0.2, 0.02),
    ]):
        for k in range(3):
            cyl(f"boton_panel_{i}_{k}", (x - 0.09 + k * 0.09, y - 0.02, z), 0.018, 0.012, [M["red"], M["green"], M["yellow"]][k], 18, (math.radians(90), 0, 0))

    # More detail on the engine inside the submarine.
    for x in [7.0, 7.28, 7.56, 7.84, 8.12]:
        torus(f"aleta_refrigeracion_motor_interno_{x}", (x, 0, -0.22), 0.49, 0.008, M["bolt"], (0, math.radians(90), 0), 64)
    bolt_ring("brida_motor_interno_frontal", 8.28, 0, -0.22, 0.38, 14, 0.023)
    bolt_ring("brida_motor_interno_trasera", 6.82, 0, -0.22, 0.38, 14, 0.023)
    cube("caja_terminal_motor_interno", (7.55, -0.05, 0.42), (0.42, 0.32, 0.22), M["panel"], 0.015)
    for i, color in enumerate([M["red"], M["yellow"], M["blue"]]):
        curve(f"cable_terminal_motor_interno_{i}", [(7.45 + i * 0.09, -0.17, 0.52), (7.28 + i * 0.08, -0.78, 0.52), (7.05 + i * 0.05, -0.95, 0.18)], color, 0.012)
    label("etiqueta_motor_interno_detalle", "Motor interno:\nbridas, aletas,\nterminales y cables", (8.95, -4.9, 1.55), (7.55, -0.05, 0.42), 0.15)


def motor_body(prefix, origin, exploded=False):
    ox, oy, oz = origin
    if exploded:
        # Components spread along X to show internal order.
        x0 = ox - 2.6
        cyl(f"{prefix}_tapa_frontal", (x0, oy, oz), 0.66, 0.22, M["motor"], 64, (0, math.radians(90), 0))
        bolt_ring(f"{prefix}_tapa_frontal_bolts", x0 - 0.13, oy, oz, 0.55, 16, 0.032)
        torus(f"{prefix}_rodamiento_frontal", (x0 + 0.32, oy, oz), 0.28, 0.045, M["bearing"], (0, math.radians(90), 0), 64)

        x1 = ox - 0.75
        cyl(f"{prefix}_carcasa_cortada", (x1, oy, oz), 0.72, 1.35, M["motor_trans"], 96, (0, math.radians(90), 0))
        for i in range(9):
            x = x1 - 0.58 + i * 0.145
            torus(f"{prefix}_lamina_estator_{i}", (x, oy, oz), 0.57, 0.012, M["stator"], (0, math.radians(90), 0), 72)
        for i in range(12):
            a = 2 * math.pi * i / 12
            y = oy + 0.47 * math.cos(a)
            z = oz + 0.47 * math.sin(a)
            curve(f"{prefix}_bobina_cobre_{i}", [(x1 - 0.55, y, z), (x1, y, z), (x1 + 0.55, y, z)], M["copper"], 0.018)

        x2 = ox + 0.95
        cyl(f"{prefix}_rotor_eje_laminado", (x2, oy, oz), 0.28, 1.55, M["rotor"], 64, (0, math.radians(90), 0))
        cyl(f"{prefix}_eje_pasante", (x2, oy, oz), 0.07, 2.35, M["bolt"], 32, (0, math.radians(90), 0))
        for i in range(8):
            a = 2 * math.pi * i / 8
            curve(
                f"{prefix}_barra_rotor_{i}",
                [(x2 - 0.7, oy + 0.24 * math.cos(a), oz + 0.24 * math.sin(a)),
                 (x2 + 0.7, oy + 0.24 * math.cos(a), oz + 0.24 * math.sin(a))],
                M["brass"],
                0.01,
            )

        x3 = ox + 2.65
        cyl(f"{prefix}_tapa_trasera", (x3, oy, oz), 0.66, 0.22, M["motor"], 64, (0, math.radians(90), 0))
        bolt_ring(f"{prefix}_tapa_trasera_bolts", x3 - 0.13, oy, oz, 0.55, 16, 0.032)
        torus(f"{prefix}_rodamiento_trasero", (x3 - 0.32, oy, oz), 0.28, 0.045, M["bearing"], (0, math.radians(90), 0), 64)
        add_fan(prefix, (x3 + 0.46, oy, oz))

        curve(f"{prefix}_eje_alineacion_explosion", [(x0 - 0.65, oy, oz), (x3 + 0.95, oy, oz)], M["orange"], 0.006)
        label(f"{prefix}_lbl_tapa", "Tapa frontal\n+ tornilleria", (x0 - 0.25, oy - 1.55, oz + 1.15), (x0, oy, oz + 0.55), 0.12)
        label(f"{prefix}_lbl_estator", "Estator:\nlaminas y bobinas", (x1, oy - 1.72, oz + 1.25), (x1, oy, oz + 0.48), 0.12)
        label(f"{prefix}_lbl_rotor", "Rotor y eje\nalineados", (x2, oy - 1.72, oz + 1.2), (x2, oy, oz + 0.28), 0.12)
        label(f"{prefix}_lbl_rodamientos", "Rodamientos\nfrontales/traseros", (x3 + 0.3, oy - 1.6, oz + 1.18), (x3 - 0.32, oy, oz + 0.28), 0.12)
        label(f"{prefix}_lbl_ventilador", "Ventilador\nde enfriamiento", (x3 + 1.05, oy - 1.52, oz + 0.7), (x3 + 0.46, oy, oz + 0.42), 0.12)
        return

    # Complete assembled motor copy.
    cyl(f"{prefix}_carcasa", (ox, oy, oz), 0.72, 2.65, M["motor"], 96, (0, math.radians(90), 0))
    cyl(f"{prefix}_tapa_frontal", (ox - 1.45, oy, oz), 0.68, 0.28, M["motor"], 96, (0, math.radians(90), 0))
    cyl(f"{prefix}_tapa_trasera", (ox + 1.45, oy, oz), 0.68, 0.28, M["motor"], 96, (0, math.radians(90), 0))
    cyl(f"{prefix}_eje_salida", (ox - 1.85, oy, oz), 0.11, 0.65, M["bolt"], 32, (0, math.radians(90), 0))
    cube(f"{prefix}_chavetero_eje", (ox - 1.93, oy, oz + 0.1), (0.33, 0.045, 0.025), M["washer"], 0.002)
    torus(f"{prefix}_junta_frontal", (ox - 1.27, oy, oz), 0.69, 0.015, M["gasket"], (0, math.radians(90), 0), 72)
    torus(f"{prefix}_junta_trasera", (ox + 1.27, oy, oz), 0.69, 0.015, M["gasket"], (0, math.radians(90), 0), 72)
    bolt_ring(f"{prefix}_frente", ox - 1.61, oy, oz, 0.56, 18, 0.032)
    bolt_ring(f"{prefix}_trasera", ox + 1.31, oy, oz, 0.56, 18, 0.032)

    # Cooling ribs.
    for i in range(13):
        x = ox - 0.96 + i * 0.16
        torus(f"{prefix}_aleta_circular_{i:02d}", (x, oy, oz), 0.72, 0.008, M["bolt"], (0, math.radians(90), 0), 80)
    for i in range(14):
        a = 2 * math.pi * i / 14
        y = oy + 0.75 * math.cos(a)
        z = oz + 0.75 * math.sin(a)
        curve(f"{prefix}_nervio_longitudinal_{i:02d}", [(ox - 1.08, y, z), (ox + 1.08, y, z)], M["bolt"], 0.008)

    # Terminal box, cable glands and lugs.
    cube(f"{prefix}_caja_terminal", (ox - 0.25, oy, oz + 0.88), (0.74, 0.42, 0.28), M["panel"], 0.025)
    cube(f"{prefix}_tapa_caja_terminal", (ox - 0.25, oy - 0.22, oz + 0.9), (0.62, 0.035, 0.2), M["nameplate"], 0.006)
    for i, dx in enumerate([-0.22, 0, 0.22]):
        cyl(f"{prefix}_prensaestopa_{i+1}", (ox + dx, oy - 0.25, oz + 0.88), 0.055, 0.09, M["brass"], 24, (math.radians(90), 0, 0))
        curve(f"{prefix}_cable_salida_{i+1}", [(ox + dx, oy - 0.31, oz + 0.88), (ox + dx - 0.25, oy - 0.78, oz + 0.82), (ox - 0.6 + i * 0.25, oy - 1.12, oz + 0.55)], [M["red"], M["yellow"], M["blue"]][i], 0.018)
    for dx in [-0.31, 0.31]:
        for dz in [-0.08, 0.08]:
            bolt(f"{prefix}_tornillo_tapa_terminal_{dx}_{dz}", (ox + dx, oy - 0.245, oz + 0.9 + dz), 0.018, 0.014, "Y")

    # Mounting feet and base bolts.
    for sx in [-0.85, 0.85]:
        for sy in [-0.42, 0.42]:
            cube(f"{prefix}_pata_{sx}_{sy}", (ox + sx, oy + sy, oz - 0.72), (0.42, 0.22, 0.16), M["motor"], 0.018)
            bolt(f"{prefix}_perno_base_{sx}_{sy}_a", (ox + sx - 0.12, oy + sy, oz - 0.62), 0.024, 0.018, "Z")
            bolt(f"{prefix}_perno_base_{sx}_{sy}_b", (ox + sx + 0.12, oy + sy, oz - 0.62), 0.024, 0.018, "Z")
    cube(f"{prefix}_base_soporte", (ox, oy, oz - 0.87), (2.9, 1.3, 0.08), M["floor"], 0.01)

    add_fan(prefix, (ox + 1.8, oy, oz))
    cyl(f"{prefix}_placa_identificacion", (ox - 0.55, oy - 0.72, oz + 0.18), 0.001, 0.001, M["nameplate"], 3)
    cube(f"{prefix}_placa_identificacion_rect", (ox - 0.55, oy - 0.725, oz + 0.18), (0.46, 0.022, 0.22), M["nameplate"], 0.004)
    for i in range(4):
        curve(f"{prefix}_texto_placa_linea_{i}", [(ox - 0.74, oy - 0.74, oz + 0.25 - i * 0.045), (ox - 0.38, oy - 0.74, oz + 0.25 - i * 0.045)], M["washer"], 0.0025)

    label(f"{prefix}_lbl_nombre", "Copia A: motor electrico\narmado con microdetalles", (ox, oy - 1.8, oz + 1.45), (ox, oy, oz + 0.72), 0.13)
    label(f"{prefix}_lbl_tuercas", "18 tuercas por brida\n+ arandelas", (ox - 1.55, oy - 1.55, oz + 0.72), (ox - 1.61, oy - 0.42, oz + 0.38), 0.115)
    label(f"{prefix}_lbl_caja", "Caja de bornes,\nprensaestopas y cables", (ox + 0.75, oy - 1.62, oz + 1.1), (ox - 0.05, oy - 0.25, oz + 0.88), 0.115)
    label(f"{prefix}_lbl_aletas", "Aletas de enfriamiento\ny nervios longitudinales", (ox + 1.25, oy - 1.62, oz + 0.55), (ox + 0.45, oy, oz + 0.72), 0.115)
    label(f"{prefix}_lbl_eje", "Eje con chavetero", (ox - 2.0, oy - 1.42, oz + 0.25), (ox - 1.93, oy, oz + 0.1), 0.115)


def add_fan(prefix, origin):
    ox, oy, oz = origin
    torus(f"{prefix}_rejilla_ventilador_aro_ext", (ox, oy, oz), 0.5, 0.018, M["fan"], (0, math.radians(90), 0), 72)
    torus(f"{prefix}_rejilla_ventilador_aro_int", (ox - 0.01, oy, oz), 0.24, 0.01, M["fan"], (0, math.radians(90), 0), 48)
    cyl(f"{prefix}_cubo_ventilador", (ox - 0.025, oy, oz), 0.09, 0.06, M["fan"], 24, (0, math.radians(90), 0))
    for i in range(8):
        a = 2 * math.pi * i / 8
        curve(
            f"{prefix}_rejilla_radial_{i}",
            [(ox, oy + 0.1 * math.cos(a), oz + 0.1 * math.sin(a)),
             (ox, oy + 0.48 * math.cos(a), oz + 0.48 * math.sin(a))],
            M["fan"],
            0.006,
        )
    for i in range(6):
        a = 2 * math.pi * i / 6
        p1 = (ox - 0.05, oy + 0.11 * math.cos(a), oz + 0.11 * math.sin(a))
        p2 = (ox - 0.08, oy + 0.38 * math.cos(a + 0.4), oz + 0.38 * math.sin(a + 0.4))
        p3 = (ox - 0.08, oy + 0.22 * math.cos(a + 0.85), oz + 0.22 * math.sin(a + 0.85))
        triangular_blade(f"{prefix}_pala_ventilador_{i}", p1, p2, p3, M["fan"])


def triangular_blade(name, p1, p2, p3, material):
    axis = Vector((1, 0, 0)) * 0.018
    pts = [Vector(p1), Vector(p2), Vector(p3)]
    verts = [tuple(p + axis) for p in pts] + [tuple(p - axis) for p in pts]
    faces = [(0, 1, 2), (5, 4, 3), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(material)
    shade(obj)
    return obj


def create_motor_display_area():
    cube("plataforma_motores_aparte", (1.8, -6.25, -0.98), (12.8, 3.8, 0.08), M["floor"], 0.0)
    label("titulo_motores_aparte", "Motores aparte: armado y vista seccionada/explotada", (1.8, -7.85, 1.95), None, 0.18, M["white"])
    motor_body("motor_armado_detalle", (-3.15, -6.05, 0.0), exploded=False)
    motor_body("motor_explotado_detalle", (3.2, -6.05, 0.0), exploded=True)
    label(
        "nota_motores_genericos",
        "Detalle educativo generico: pernos, rodamientos,\ncarcasa, rotor, estator, bobinas y cableado",
        (1.9, -7.88, 1.62),
        None,
        0.12,
        M["soft"],
    )


def update_cameras_and_render():
    # Expand the old platform if it exists.
    old = bpy.data.objects.get("plataforma_neutra")
    if old:
        old.dimensions = (30, 12.5, 0.08)
        bpy.context.view_layer.objects.active = old
        old.select_set(True)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        old.select_set(False)

    # Main camera sees submarine and foreground motor area.
    cam = bpy.data.objects.get("Camara_presentacion")
    if cam is None:
        bpy.ops.object.camera_add()
        cam = bpy.context.object
        cam.name = "Camara_presentacion"
    cam.location = (15.7, -16.4, 7.8)
    look_at(cam, (1.0, -1.6, 0.55))
    cam.data.lens = 29
    bpy.context.scene.camera = cam
    bpy.context.scene.render.resolution_x = 2400
    bpy.context.scene.render.resolution_y = 1500
    bpy.context.scene.render.filepath = MAIN_RENDER
    bpy.ops.render.render(write_still=True)

    bpy.ops.object.camera_add(location=(2.5, -10.4, 3.1))
    motor_cam = bpy.context.object
    motor_cam.name = "Camara_detalle_motores"
    look_at(motor_cam, (0.6, -6.05, 0.25))
    motor_cam.data.lens = 42
    bpy.context.scene.camera = motor_cam
    bpy.context.scene.render.resolution_x = 2200
    bpy.context.scene.render.resolution_y = 1400
    bpy.context.scene.render.filepath = MOTOR_RENDER
    bpy.ops.render.render(write_still=True)


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def add_index_empties():
    for name, loc in [
        ("INDICE_06_Motor_armado_aparte", (-3.15, -6.05, 0.0)),
        ("INDICE_07_Motor_explotado_aparte", (3.2, -6.05, 0.0)),
        ("INDICE_08_Tornilleria_fina", (-7.6, -1.73, 0.42)),
        ("INDICE_09_Bridas_tuberias", (3.6, -1.35, -0.95)),
    ]:
        empty = bpy.data.objects.new(name, None)
        empty.empty_display_type = "SPHERE"
        empty.empty_display_size = 0.16
        empty.location = loc
        bpy.context.collection.objects.link(empty)


add_detail_collection()
detail_existing_submarine()
create_motor_display_area()
add_index_empties()
bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)
update_cameras_and_render()
bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)

print(f"Archivo ultra detallado: {OUT_BLEND}")
print(f"Render general: {MAIN_RENDER}")
print(f"Render motores: {MOTOR_RENDER}")
