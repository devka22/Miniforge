import math
import os

import bpy
from mathutils import Vector


ROOT = os.path.dirname(os.path.abspath(__file__))
BLEND_PATH = os.path.join(ROOT, "submarino_escuela_naval.blend")
RENDER_PATH = os.path.join(ROOT, "submarino_escuela_naval_preview.png")


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def make_mat(name, color, metallic=0.0, roughness=0.55, alpha=1.0):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        if "Base Color" in bsdf.inputs:
            bsdf.inputs["Base Color"].default_value = color
        if "Metallic" in bsdf.inputs:
            bsdf.inputs["Metallic"].default_value = metallic
        if "Roughness" in bsdf.inputs:
            bsdf.inputs["Roughness"].default_value = roughness
        if "Alpha" in bsdf.inputs:
            bsdf.inputs["Alpha"].default_value = alpha
    mat.diffuse_color = color
    if alpha < 1.0:
        mat.blend_method = "BLEND"
        mat.show_transparent_back = True
        if hasattr(mat, "surface_render_method"):
            mat.surface_render_method = "BLENDED"
        if hasattr(mat, "use_screen_refraction"):
            mat.use_screen_refraction = True
    return mat


def add_bevel(obj, width=0.04, segments=2):
    mod = obj.modifiers.new("bordes suaves", "BEVEL")
    mod.width = width
    mod.segments = segments
    mod.affect = "EDGES"
    norm = obj.modifiers.new("normales ponderadas", "WEIGHTED_NORMAL")
    return obj


def shade_smooth(obj):
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    try:
        bpy.ops.object.shade_smooth()
    except Exception:
        pass
    obj.select_set(False)


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def add_cube(name, loc, scale, mat, bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(mat)
    if bevel:
        add_bevel(obj, bevel, 3)
    return obj


def add_cylinder(name, loc, radius, depth, mat, vertices=48, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=loc, rotation=rotation
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    shade_smooth(obj)
    return obj


def add_cone(name, loc, radius1, radius2, depth, mat, vertices=64, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    shade_smooth(obj)
    return obj


def add_uv_sphere(name, loc, scale, mat, segments=64, rings=24):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1, location=loc
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)
    shade_smooth(obj)
    return obj


def add_torus(name, loc, major_radius, minor_radius, mat, rotation=(0, 0, 0), major_segments=72):
    bpy.ops.mesh.primitive_torus_add(
        major_segments=major_segments,
        minor_segments=10,
        major_radius=major_radius,
        minor_radius=minor_radius,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    shade_smooth(obj)
    return obj


def add_curve(name, points, mat, bevel_depth=0.025):
    curve = bpy.data.curves.new(name, "CURVE")
    curve.dimensions = "3D"
    curve.resolution_u = 2
    curve.bevel_depth = bevel_depth
    curve.bevel_resolution = 4
    spline = curve.splines.new("POLY")
    spline.points.add(len(points) - 1)
    for point, co in zip(spline.points, points):
        point.co = (co[0], co[1], co[2], 1)
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    return obj


def add_label(name, text, loc, target=None, size=0.22, mat=None, camera_loc=(16, -20, 8)):
    if mat is None:
        mat = materials["label_text"]
    bpy.ops.object.text_add(location=loc, rotation=(math.radians(62), 0, 0))
    obj = bpy.context.object
    obj.name = name
    obj.data.body = text
    obj.data.align_x = "CENTER"
    obj.data.align_y = "CENTER"
    obj.data.size = size
    obj.data.materials.append(mat)
    if target:
        add_curve(f"flecha_{name}", [loc, target], materials["annotation"], 0.012)
        add_uv_sphere(f"punto_{name}", target, (0.055, 0.055, 0.055), materials["annotation"], 16, 8)
    return obj


def create_partial_hull(name, length, radius, thickness, theta_start, theta_end, mat):
    x_segments = 24
    arc_segments = 128
    xs = [-length / 2 + length * i / x_segments for i in range(x_segments + 1)]
    ts = [
        math.radians(theta_start + (theta_end - theta_start) * j / arc_segments)
        for j in range(arc_segments + 1)
    ]
    verts = []
    outer_index = []
    inner_index = []
    for x in xs:
        outer_row = []
        inner_row = []
        for t in ts:
            y = radius * math.cos(t)
            z = radius * math.sin(t)
            outer_row.append(len(verts))
            verts.append((x, y, z))
        for t in ts:
            r = radius - thickness
            y = r * math.cos(t)
            z = r * math.sin(t)
            inner_row.append(len(verts))
            verts.append((x, y, z))
        outer_index.append(outer_row)
        inner_index.append(inner_row)

    faces = []
    for i in range(x_segments):
        for j in range(arc_segments):
            faces.append(
                (
                    outer_index[i][j],
                    outer_index[i + 1][j],
                    outer_index[i + 1][j + 1],
                    outer_index[i][j + 1],
                )
            )
            faces.append(
                (
                    inner_index[i][j + 1],
                    inner_index[i + 1][j + 1],
                    inner_index[i + 1][j],
                    inner_index[i][j],
                )
            )
    for i in range(x_segments):
        faces.append(
            (
                outer_index[i][0],
                inner_index[i][0],
                inner_index[i + 1][0],
                outer_index[i + 1][0],
            )
        )
        faces.append(
            (
                outer_index[i + 1][-1],
                inner_index[i + 1][-1],
                inner_index[i][-1],
                outer_index[i][-1],
            )
        )
    for j in range(arc_segments):
        faces.append(
            (
                outer_index[0][j],
                outer_index[0][j + 1],
                inner_index[0][j + 1],
                inner_index[0][j],
            )
        )
        faces.append(
            (
                outer_index[-1][j + 1],
                outer_index[-1][j],
                inner_index[-1][j],
                inner_index[-1][j + 1],
            )
        )

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    shade_smooth(obj)
    add_bevel(obj, 0.015, 2)
    return obj


def add_tri_prism(name, p1, p2, p3, thickness_axis, thickness, mat):
    pts = [Vector(p1), Vector(p2), Vector(p3)]
    axis = Vector(thickness_axis).normalized() * (thickness / 2)
    verts = [tuple(p + axis) for p in pts] + [tuple(p - axis) for p in pts]
    faces = [(0, 1, 2), (5, 4, 3), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    shade_smooth(obj)
    add_bevel(obj, 0.02, 1)
    return obj


def add_screen(name, loc, scale, color_mat):
    panel = add_cube(name, loc, scale, color_mat, 0.01)
    return panel


def add_human(name, x, y, z, suit_mat):
    add_cylinder(f"{name}_cuerpo", (x, y, z + 0.35), 0.12, 0.58, suit_mat, 20)
    add_uv_sphere(f"{name}_cabeza", (x, y, z + 0.78), (0.13, 0.13, 0.13), materials["skin"], 24, 12)
    add_cylinder(f"{name}_pierna_izq", (x, y - 0.055, z + 0.03), 0.035, 0.42, suit_mat, 12)
    bpy.context.object.rotation_euler[0] = math.radians(0)
    add_cylinder(f"{name}_pierna_der", (x, y + 0.055, z + 0.03), 0.035, 0.42, suit_mat, 12)
    add_curve(f"{name}_brazo_izq", [(x, y - 0.13, z + 0.52), (x + 0.18, y - 0.22, z + 0.35)], suit_mat, 0.025)
    add_curve(f"{name}_brazo_der", [(x, y + 0.13, z + 0.52), (x + 0.16, y + 0.23, z + 0.42)], suit_mat, 0.025)


def add_compartment_floor(x1, x2, name, mat):
    add_cube(f"piso_{name}", ((x1 + x2) / 2, 0, -0.78), (x2 - x1, 3.05, 0.08), mat, 0.01)
    add_cube(f"cubierta_superior_{name}", ((x1 + x2) / 2, 0, 0.68), (x2 - x1, 2.25, 0.055), mat, 0.01)


def add_bulkhead(x, label):
    add_torus(f"mamparo_estanco_{label}", (x, 0, -0.03), 1.64, 0.045, materials["bulkhead"], (0, math.radians(90), 0), 96)
    add_torus(f"escotilla_{label}", (x - 0.015, 0, -0.05), 0.42, 0.035, materials["hatch"], (0, math.radians(90), 0), 64)
    add_cube(f"puerta_estanca_{label}", (x - 0.03, 0, -0.05), (0.035, 0.67, 0.86), materials["door"], 0.04)
    add_cylinder(f"volante_escotilla_{label}", (x - 0.06, -0.36, -0.05), 0.08, 0.025, materials["valve"], 24, (0, math.radians(90), 0))


def create_propeller():
    add_cylinder("eje_propulsion", (11.55, 0, 0), 0.13, 2.4, materials["dark_metal"], 32, (0, math.radians(90), 0))
    add_uv_sphere("cubo_helice", (12.65, 0, 0), (0.28, 0.28, 0.28), materials["bronze"], 32, 16)
    for i in range(5):
        angle = 2 * math.pi * i / 5
        y1 = 0.22 * math.cos(angle)
        z1 = 0.22 * math.sin(angle)
        y2 = 1.05 * math.cos(angle + 0.28)
        z2 = 1.05 * math.sin(angle + 0.28)
        y3 = 0.48 * math.cos(angle + 0.82)
        z3 = 0.48 * math.sin(angle + 0.82)
        add_tri_prism(
            f"pala_helice_{i+1}",
            (12.72, y1, z1),
            (12.95, y2, z2),
            (12.55, y3, z3),
            (1, 0, 0),
            0.07,
            materials["bronze"],
        )


def create_console_cluster():
    for i, x in enumerate([-3.8, -3.25, -2.7]):
        add_cube(f"consola_control_{i+1}", (x, -0.72, -0.28), (0.42, 0.42, 0.55), materials["console"], 0.04)
        add_screen(f"pantalla_control_{i+1}", (x, -0.94, 0.08), (0.34, 0.035, 0.23), materials["screen_green"])
        add_cylinder(f"silla_control_{i+1}", (x, -1.18, -0.42), 0.13, 0.08, materials["seat"], 20)
        add_cylinder(f"poste_silla_{i+1}", (x, -1.18, -0.58), 0.035, 0.32, materials["seat"], 12)

    add_cube("mesa_cartas", (-3.15, 0.18, -0.36), (0.95, 0.58, 0.18), materials["wood"], 0.03)
    add_screen("mapa_tactico_didactico", (-3.15, 0.18, -0.25), (0.82, 0.48, 0.02), materials["screen_blue"])
    add_cylinder("columna_periscopio", (-2.8, 0.52, 0.18), 0.08, 1.95, materials["dark_metal"], 32)
    add_cylinder("ocular_periscopio", (-2.8, 0.22, 0.52), 0.055, 0.62, materials["dark_metal"], 24, (math.radians(90), 0, 0))
    for j, y in enumerate([0.9, 1.15]):
        add_cube(f"rack_radio_{j+1}", (-4.05, y, -0.18), (0.5, 0.18, 0.72), materials["console"], 0.025)
        add_screen(f"pantalla_radio_{j+1}", (-4.05, y - 0.1, 0.04), (0.32, 0.025, 0.2), materials["screen_blue"])


def create_bunks_and_galley():
    for row, y in enumerate([-0.95, 0.95]):
        for level, z in enumerate([-0.22, 0.38]):
            for i, x in enumerate([0.0, 0.85, 1.7]):
                add_cube(f"litera_{row}_{level}_{i}", (x, y, z), (0.68, 0.34, 0.12), materials["bunk_frame"], 0.02)
                add_cube(f"colchon_{row}_{level}_{i}", (x, y, z + 0.08), (0.62, 0.3, 0.07), materials["mattress"], 0.015)
    add_cube("mesa_comedor", (2.55, -0.12, -0.34), (0.7, 0.55, 0.16), materials["wood"], 0.03)
    for y in [-0.48, 0.25]:
        add_cylinder(f"banco_comedor_{y}", (2.55, y, -0.42), 0.12, 0.58, materials["seat"], 16, (math.radians(90), 0, 0))
    add_cube("modulo_cocina", (3.45, 0.92, -0.23), (0.85, 0.25, 0.72), materials["stainless"], 0.025)
    add_cube("fregadero", (3.35, 0.78, 0.12), (0.3, 0.08, 0.08), materials["dark_metal"], 0.015)
    add_cube("modulo_medico", (3.45, -0.92, -0.18), (0.8, 0.25, 0.75), materials["medical"], 0.025)


def create_batteries_and_life_support():
    for row, y in enumerate([-0.75, -0.35, 0.35, 0.75]):
        for i in range(6):
            x = 4.4 + i * 0.32
            add_cube(f"celda_bateria_{row}_{i}", (x, y, -0.42), (0.22, 0.25, 0.34), materials["battery"], 0.012)
            add_cube(f"borne_bateria_{row}_{i}", (x - 0.055, y, -0.22), (0.045, 0.08, 0.035), materials["copper"], 0.004)
    for i, y in enumerate([-1.1, 1.1]):
        add_cylinder(f"tanque_aire_{i+1}", (5.55, y, 0.0), 0.22, 1.4, materials["air_tank"], 32, (math.radians(90), 0, 0))
        add_torus(f"abrazadera_tanque_{i+1}_a", (5.55, y - 0.37, 0), 0.22, 0.012, materials["dark_metal"], (math.radians(90), 0, 0), 32)
        add_torus(f"abrazadera_tanque_{i+1}_b", (5.55, y + 0.37, 0), 0.22, 0.012, materials["dark_metal"], (math.radians(90), 0, 0), 32)
    add_cube("panel_life_support", (6.15, 0, 0.1), (0.18, 0.9, 0.72), materials["console"], 0.025)
    add_screen("indicador_oxigeno", (6.05, -0.2, 0.26), (0.035, 0.28, 0.16), materials["screen_green"])
    add_screen("indicador_co2", (6.05, 0.2, 0.26), (0.035, 0.28, 0.16), materials["screen_blue"])


def create_engine_room():
    add_cylinder("motor_electrico_principal", (7.55, 0, -0.22), 0.48, 1.3, materials["engine"], 48, (0, math.radians(90), 0))
    add_cylinder("acoplamiento_motor", (8.34, 0, -0.22), 0.28, 0.28, materials["dark_metal"], 32, (0, math.radians(90), 0))
    add_cylinder("eje_interno", (9.18, 0, -0.22), 0.08, 1.55, materials["dark_metal"], 32, (0, math.radians(90), 0))
    for i, x in enumerate([6.75, 7.0, 8.7]):
        add_cube(f"gabinete_electrico_{i+1}", (x, 1.02, -0.1), (0.38, 0.26, 0.86), materials["console"], 0.025)
        add_screen(f"medidor_gabinete_{i+1}", (x, 0.87, 0.16), (0.22, 0.035, 0.14), materials["screen_green"])
    for i, x in enumerate([7.0, 7.45, 7.9]):
        add_cylinder(f"bomba_circuito_{i+1}", (x, -0.95, -0.35), 0.22, 0.35, materials["pump"], 32, (math.radians(90), 0, 0))


def create_forward_section():
    add_uv_sphere("domo_sonar_interior", (-8.9, 0, -0.02), (0.72, 0.72, 0.72), materials["sonar"], 40, 20)
    for i, z in enumerate([-0.45, -0.05, 0.35]):
        for y in [-0.48, 0.0, 0.48]:
            add_cylinder(
                f"tubo_entrenamiento_{i}_{y}",
                (-6.75, y, z),
                0.11,
                1.85,
                materials["training_tube"],
                32,
                (0, math.radians(90), 0),
            )
            add_torus(
                f"aro_tubo_entrenamiento_{i}_{y}",
                (-7.7, y, z),
                0.11,
                0.014,
                materials["hatch"],
                (0, math.radians(90), 0),
                32,
            )
    add_cube("estiba_herramientas_proa", (-5.55, 1.02, -0.2), (0.48, 0.26, 0.7), materials["console"], 0.025)
    add_cube("panel_sonar", (-5.72, -0.88, -0.16), (0.55, 0.28, 0.62), materials["console"], 0.025)
    add_screen("pantalla_sonar", (-5.72, -1.03, 0.05), (0.34, 0.035, 0.22), materials["screen_green"])


def create_ballast_and_piping():
    for side, y in [("babor", 1.55), ("estribor", -1.55)]:
        for i, x in enumerate([-6.3, -2.2, 2.2, 6.3]):
            add_cylinder(
                f"tanque_lastre_{side}_{i+1}",
                (x, y, -1.28),
                0.22,
                2.7,
                materials["ballast"],
                32,
                (0, math.radians(90), 0),
            )
            add_cube(f"soporte_tanque_{side}_{i+1}", (x, y, -1.55), (2.0, 0.08, 0.08), materials["dark_metal"], 0.01)

    add_curve(
        "linea_agua_lastre_azul",
        [(-8.1, -1.35, -0.95), (-5.0, -1.35, -0.95), (-1.0, -1.35, -0.95), (3.6, -1.35, -0.95), (7.7, -1.35, -0.95)],
        materials["pipe_water"],
        0.035,
    )
    add_curve(
        "linea_aire_verde",
        [(-6.7, 1.22, 0.92), (-2.9, 1.22, 1.0), (0.5, 1.22, 0.92), (4.8, 1.22, 0.78), (6.2, 0.9, 0.35)],
        materials["pipe_air"],
        0.026,
    )
    add_curve(
        "linea_hidraulica_amarilla",
        [(-7.2, -1.05, 0.62), (-3.5, -1.05, 0.62), (1.2, -1.05, 0.54), (5.7, -1.05, 0.45), (9.4, -0.92, 0.2)],
        materials["pipe_hydraulic"],
        0.026,
    )
    add_curve(
        "linea_extincion_roja",
        [(-8.0, -0.92, 1.05), (-4.2, -0.92, 1.12), (-0.8, -0.92, 1.08), (3.5, -0.92, 1.02), (7.2, -0.92, 0.88)],
        materials["pipe_fire"],
        0.024,
    )
    for x in [-6.3, -2.2, 2.2, 5.5, 7.2]:
        add_torus(f"valvula_lastre_{x}", (x, -1.35, -0.95), 0.09, 0.012, materials["valve"], (math.radians(90), 0, 0), 32)
    for x in [-4.5, -1.5, 1.5, 4.5, 7.4]:
        add_curve(
            f"mazo_cables_{x}",
            [(x, 0.95, 1.05), (x + 0.45, 0.95, 0.85), (x + 0.75, 0.72, 0.45)],
            materials["cable"],
            0.018,
        )


def create_hull_details(radius=2.05):
    for x in [-8.8, -7.6, -6.4, -5.2, -4.0, -2.8, -1.6, -0.4, 0.8, 2.0, 3.2, 4.4, 5.6, 6.8, 8.0]:
        add_torus(f"costilla_interior_{x}", (x, 0, 0), radius - 0.14, 0.026, materials["rib"], (0, math.radians(90), 0), 96)
    for x in [-8.4, -5.6, -2.8, 0.0, 2.8, 5.6, 8.4]:
        add_torus(f"junta_exterior_{x}", (x, 0, 0), radius + 0.015, 0.014, materials["hull_dark"], (0, math.radians(90), 0), 96)
    # Highlight the cutaway rim.
    for theta in [65, 165]:
        t = math.radians(theta)
        pts = []
        for i in range(40):
            x = -9.6 + 19.2 * i / 39
            pts.append((x, (radius + 0.04) * math.cos(t), (radius + 0.04) * math.sin(t)))
        add_curve(f"borde_corte_{theta}", pts, materials["cut_edge"], 0.04)
        for i in range(10):
            x = -9.2 + 18.4 * i / 9
            add_uv_sphere(
                f"remache_borde_{theta}_{i}",
                (x, (radius + 0.07) * math.cos(t), (radius + 0.07) * math.sin(t)),
                (0.055, 0.055, 0.055),
                materials["cut_edge"],
                16,
                8,
            )


def create_sail_and_masts():
    sail = add_cube("vela_o_torreta", (-2.35, 0, 2.35), (1.45, 1.08, 1.55), materials["hull"], 0.16)
    add_cube("base_hidrodinamica_torreta", (-2.35, 0, 1.72), (2.1, 1.42, 0.28), materials["hull"], 0.12)
    for x, h, r, mat, name in [
        (-2.9, 1.95, 0.055, materials["dark_metal"], "mastil_comunicaciones"),
        (-2.45, 2.35, 0.065, materials["dark_metal"], "periscopio"),
        (-2.05, 1.55, 0.05, materials["dark_metal"], "sensor_superficie"),
    ]:
        add_cylinder(name, (x, 0, 3.15 + h / 2), r, h, mat, 32)
        add_cylinder(f"{name}_cabezal", (x, -0.12, 3.15 + h), r * 0.8, 0.28, mat, 24, (math.radians(90), 0, 0))
    for y in [-0.56, 0.56]:
        add_cube(f"ventana_torreta_{y}", (-2.83, y, 2.72), (0.07, 0.22, 0.12), materials["window"], 0.01)
        add_cube(f"ventana_torreta_2_{y}", (-2.35, y, 2.78), (0.07, 0.22, 0.12), materials["window"], 0.01)


def create_fins():
    # Bow planes.
    for y, side in [(-2.13, "estribor"), (2.13, "babor")]:
        add_tri_prism(
            f"plano_proa_{side}",
            (-6.8, y, 0.05),
            (-5.65, y, 0.28),
            (-6.35, y, -0.38),
            (0, 1, 0),
            0.08,
            materials["hull"],
        )
    # Stern planes and rudders.
    for y, side in [(-2.05, "estribor"), (2.05, "babor")]:
        add_tri_prism(
            f"plano_popa_{side}",
            (9.25, y, 0.1),
            (10.75, y, 0.36),
            (10.08, y, -0.55),
            (0, 1, 0),
            0.08,
            materials["hull"],
        )
    add_tri_prism("timon_superior", (9.35, 0, 1.75), (10.75, 0, 1.25), (10.1, 0, 2.45), (0, 0, 1), 0.08, materials["hull"])
    add_tri_prism("timon_inferior", (9.35, 0, -1.75), (10.75, 0, -1.25), (10.1, 0, -2.45), (0, 0, 1), 0.08, materials["hull"])


def create_legend():
    add_cube("base_placa_titulo", (0, -4.05, 2.8), (8.5, 0.05, 0.75), materials["placard"], 0.04)
    add_label("titulo_modelo", "Submarino didactico: corte longitudinal y sistemas", (0, -4.12, 2.98), size=0.28)
    add_label("subtitulo_modelo", "Distribucion generica para presentacion escolar naval", (0, -4.12, 2.66), size=0.17, mat=materials["label_soft"])

    items = [
        ("agua/lastre", materials["pipe_water"]),
        ("aire", materials["pipe_air"]),
        ("hidraulico", materials["pipe_hydraulic"]),
        ("extincion", materials["pipe_fire"]),
        ("energia/datos", materials["cable"]),
    ]
    for i, (txt, mat) in enumerate(items):
        x = -3.65 + i * 1.75
        add_cube(f"muestra_{txt}", (x - 0.42, -4.1, 2.28), (0.22, 0.04, 0.1), mat, 0.01)
        add_label(f"leyenda_{i}", txt, (x + 0.16, -4.13, 2.28), size=0.13, mat=materials["label_soft"])


def create_annotations():
    annotations = [
        ("etiqueta_sonar", "Cupula sonar\n(esquematica)", (-8.55, -3.35, 1.45), (-8.9, -0.45, 0.15)),
        ("etiqueta_tubos", "Tubos de practica\nno funcionales", (-6.35, -3.45, 0.78), (-6.85, -0.35, -0.05)),
        ("etiqueta_control", "Sala de control\nnavegacion y radio", (-3.1, -3.55, 1.35), (-3.2, -0.75, 0.08)),
        ("etiqueta_mamparo", "Mamparos estancos\ncon escotillas", (-0.55, -3.65, 0.85), (-0.95, -0.22, -0.05)),
        ("etiqueta_alojamiento", "Alojamiento y vida a bordo", (1.25, -3.45, 1.32), (0.85, -0.95, 0.38)),
        ("etiqueta_baterias", "Banco de baterias\npropulsion electrica", (4.95, -3.55, 0.82), (4.95, -0.35, -0.3)),
        ("etiqueta_aire", "Aire y soporte vital", (6.25, -3.35, 1.45), (5.55, -1.1, 0.02)),
        ("etiqueta_motor", "Motor electrico y eje", (8.25, -3.45, 0.85), (7.55, -0.25, -0.22)),
        ("etiqueta_lastre", "Tanques de lastre\nlaterales", (2.55, 3.18, -1.95), (2.2, 1.55, -1.28)),
        ("etiqueta_torreta", "Torreta, periscopio\ny mastiles", (-2.25, -3.2, 3.78), (-2.45, -0.15, 4.75)),
        ("etiqueta_helice", "Helice y planos\nde gobierno", (10.4, -3.35, 1.45), (12.55, -0.1, 0.1)),
    ]
    for name, text, loc, target in annotations:
        add_label(name, text, loc, target=target, size=0.18)


def setup_world_and_camera():
    try:
        bpy.context.scene.render.engine = "BLENDER_EEVEE_NEXT"
    except Exception:
        bpy.context.scene.render.engine = "CYCLES"
        bpy.context.scene.cycles.samples = 64

    bpy.context.scene.view_settings.view_transform = "Filmic"
    bpy.context.scene.view_settings.look = "Medium High Contrast"
    bpy.context.scene.world = bpy.data.worlds.new("mundo_estudio") if not bpy.context.scene.world else bpy.context.scene.world
    bpy.context.scene.world.color = (0.025, 0.03, 0.04)

    add_cube("plataforma_neutra", (0, 0, -2.12), (27, 8.5, 0.08), materials["floor"], 0.0)
    add_curve("eje_referencia_longitudinal", [(-11.8, 0, -2.03), (12.8, 0, -2.03)], materials["axis"], 0.01)

    bpy.ops.object.light_add(type="AREA", location=(0, -6, 7.5))
    key = bpy.context.object
    key.name = "luz_area_principal"
    key.data.energy = 650
    key.data.size = 7.5
    bpy.ops.object.light_add(type="POINT", location=(-7, 3.5, 3.6))
    fill = bpy.context.object
    fill.name = "luz_relleno_interior"
    fill.data.energy = 160

    bpy.ops.object.camera_add(location=(15.0, -13.5, 6.4))
    cam = bpy.context.object
    cam.name = "Camara_presentacion"
    look_at(cam, (0.4, 0, 0.45))
    cam.data.lens = 34
    cam.data.dof.use_dof = True
    cam.data.dof.focus_distance = 17
    cam.data.dof.aperture_fstop = 6.5
    bpy.context.scene.camera = cam
    bpy.context.scene.render.resolution_x = 2200
    bpy.context.scene.render.resolution_y = 1400
    bpy.context.scene.eevee.taa_render_samples = 64 if hasattr(bpy.context.scene, "eevee") else 16


clear_scene()

materials = {
    "hull": make_mat("casco azul naval satinado", (0.035, 0.105, 0.16, 1), 0.15, 0.38),
    "hull_trans": make_mat("casco exterior semitransparente", (0.035, 0.14, 0.22, 0.42), 0.1, 0.32, 0.42),
    "hull_dark": make_mat("lineas casco oscuro", (0.01, 0.035, 0.055, 1), 0.15, 0.42),
    "cut_edge": make_mat("borde corte naranja", (1.0, 0.42, 0.07, 1), 0.0, 0.35),
    "rib": make_mat("costillas internas", (0.72, 0.76, 0.77, 1), 0.05, 0.48),
    "bulkhead": make_mat("mamparo gris claro", (0.67, 0.7, 0.7, 1), 0.05, 0.52),
    "door": make_mat("puerta estanca", (0.42, 0.48, 0.5, 1), 0.1, 0.45),
    "hatch": make_mat("aros escotilla", (0.82, 0.84, 0.78, 1), 0.1, 0.35),
    "floor_deck": make_mat("cubiertas antideslizantes", (0.22, 0.25, 0.25, 1), 0.0, 0.62),
    "console": make_mat("consolas gris oscuro", (0.11, 0.14, 0.15, 1), 0.0, 0.45),
    "screen_green": make_mat("pantallas verde sonar", (0.05, 0.9, 0.48, 1), 0.0, 0.18),
    "screen_blue": make_mat("pantallas azul navegacion", (0.05, 0.48, 0.95, 1), 0.0, 0.18),
    "seat": make_mat("asientos azul gris", (0.08, 0.14, 0.21, 1), 0.0, 0.5),
    "wood": make_mat("madera interior", (0.42, 0.25, 0.12, 1), 0.0, 0.55),
    "stainless": make_mat("acero inoxidable", (0.62, 0.66, 0.65, 1), 0.35, 0.28),
    "medical": make_mat("modulo medico blanco", (0.88, 0.88, 0.84, 1), 0.0, 0.45),
    "bunk_frame": make_mat("estructura literas", (0.32, 0.35, 0.35, 1), 0.05, 0.5),
    "mattress": make_mat("colchones azul marino", (0.04, 0.11, 0.22, 1), 0.0, 0.65),
    "battery": make_mat("baterias", (0.18, 0.2, 0.16, 1), 0.0, 0.5),
    "copper": make_mat("cobre bornes", (0.9, 0.42, 0.12, 1), 0.6, 0.28),
    "air_tank": make_mat("tanques aire verde", (0.1, 0.42, 0.29, 1), 0.1, 0.36),
    "engine": make_mat("motor electrico", (0.18, 0.28, 0.34, 1), 0.25, 0.32),
    "pump": make_mat("bombas", (0.46, 0.18, 0.18, 1), 0.15, 0.42),
    "dark_metal": make_mat("metal oscuro", (0.03, 0.034, 0.035, 1), 0.45, 0.35),
    "bronze": make_mat("bronce helice", (0.82, 0.54, 0.16, 1), 0.55, 0.26),
    "sonar": make_mat("domo sonar", (0.04, 0.2, 0.32, 0.66), 0.0, 0.2, 0.66),
    "training_tube": make_mat("tubos entrenamiento", (0.58, 0.62, 0.63, 1), 0.2, 0.35),
    "ballast": make_mat("tanques lastre azul", (0.08, 0.22, 0.32, 1), 0.1, 0.42),
    "pipe_water": make_mat("tuberia azul agua-lastre", (0.02, 0.34, 0.92, 1), 0.0, 0.28),
    "pipe_air": make_mat("tuberia verde aire", (0.08, 0.86, 0.38, 1), 0.0, 0.28),
    "pipe_hydraulic": make_mat("tuberia amarilla hidraulica", (1.0, 0.78, 0.05, 1), 0.0, 0.28),
    "pipe_fire": make_mat("tuberia roja extincion", (0.95, 0.06, 0.04, 1), 0.0, 0.28),
    "cable": make_mat("mazos cableado negro", (0.01, 0.012, 0.014, 1), 0.0, 0.35),
    "valve": make_mat("volantes valvula", (1.0, 0.58, 0.04, 1), 0.0, 0.35),
    "window": make_mat("vidrio oscuro", (0.02, 0.08, 0.12, 0.74), 0.0, 0.22, 0.74),
    "label_text": make_mat("texto etiquetas blanco", (0.95, 0.96, 0.92, 1), 0.0, 0.5),
    "label_soft": make_mat("texto etiquetas suave", (0.72, 0.82, 0.86, 1), 0.0, 0.5),
    "annotation": make_mat("lineas anotacion", (1.0, 0.72, 0.18, 1), 0.0, 0.35),
    "placard": make_mat("placa titulo semitransparente", (0.02, 0.03, 0.035, 0.72), 0.0, 0.55, 0.72),
    "floor": make_mat("piso estudio mate", (0.075, 0.085, 0.085, 1), 0.0, 0.7),
    "axis": make_mat("eje referencia tenue", (0.45, 0.5, 0.52, 1), 0.0, 0.65),
    "skin": make_mat("piel figuras escala", (0.78, 0.56, 0.42, 1), 0.0, 0.55),
    "uniform": make_mat("uniforme naval escala", (0.015, 0.035, 0.07, 1), 0.0, 0.55),
}


# Main exterior hull and appendages.
create_partial_hull("casco_abierto_con_corte", 19.2, 2.05, 0.11, 165, 425, materials["hull_trans"])
add_uv_sphere("proa_redondeada_transparente", (-10.0, 0, 0), (2.0, 2.05, 2.05), materials["hull_trans"], 72, 24)
add_cone("popa_conica", (10.75, 0, 0), 2.05, 0.55, 2.6, materials["hull_trans"], 72, (0, math.radians(90), 0))
create_hull_details()
create_sail_and_masts()
create_fins()
create_propeller()


# Interior layout.
for x1, x2, name in [
    (-8.9, -5.0, "proa"),
    (-5.0, -1.25, "control"),
    (-1.25, 2.35, "alojamiento"),
    (2.35, 4.15, "servicios"),
    (4.15, 6.45, "baterias"),
    (6.45, 9.35, "maquinas"),
]:
    add_compartment_floor(x1, x2, name, materials["floor_deck"])

for idx, x in enumerate([-8.95, -5.0, -1.25, 2.35, 4.15, 6.45, 9.35]):
    add_bulkhead(x, f"{idx+1}")

create_forward_section()
create_console_cluster()
create_bunks_and_galley()
create_batteries_and_life_support()
create_engine_room()
create_ballast_and_piping()

# Small scale figures make the interior size easier to read.
add_human("tripulante_control", -3.3, -1.18, -0.78, materials["uniform"])
add_human("tripulante_maquinas", 7.25, -0.92, -0.78, materials["uniform"])

create_legend()
create_annotations()
setup_world_and_camera()


# Add a few empty markers named as a quick index for classroom presentation.
for name, loc in [
    ("INDICE_01_Casco_corte", (0, 0, 2.05)),
    ("INDICE_02_Control", (-3.2, -0.8, 0.2)),
    ("INDICE_03_Baterias", (4.9, -0.35, -0.3)),
    ("INDICE_04_Propulsion", (8.1, 0, -0.2)),
    ("INDICE_05_Lastre", (2.2, 1.55, -1.28)),
]:
    empty = bpy.data.objects.new(name, None)
    empty.empty_display_type = "SPHERE"
    empty.empty_display_size = 0.18
    empty.location = loc
    bpy.context.collection.objects.link(empty)


bpy.ops.wm.save_as_mainfile(filepath=BLEND_PATH)
bpy.context.scene.render.filepath = RENDER_PATH
bpy.ops.render.render(write_still=True)

print(f"Archivo Blender: {BLEND_PATH}")
print(f"Render preview: {RENDER_PATH}")
