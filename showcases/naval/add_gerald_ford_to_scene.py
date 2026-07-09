import math
import os

import bpy
from mathutils import Vector


ROOT = os.path.dirname(os.path.abspath(__file__))
OUT_BLEND = os.path.join(ROOT, "presentacion_naval_submarino_gerald_ford.blend")
RENDER_MAIN = os.path.join(ROOT, "gerald_ford_presentacion_preview.png")
RENDER_COMPONENTS = os.path.join(ROOT, "gerald_ford_componentes_preview.png")


def material(name, color, metallic=0.0, roughness=0.55, alpha=1.0):
    old = bpy.data.materials.get(name)
    if old:
        return old
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
    return mat


M = {
    "hull": material("ford casco gris naval", (0.25, 0.29, 0.31, 1), 0.05, 0.52),
    "hull_dark": material("ford casco sombra", (0.09, 0.115, 0.13, 1), 0.05, 0.55),
    "deck": material("ford cubierta vuelo antideslizante", (0.12, 0.13, 0.13, 1), 0.0, 0.72),
    "deck_edge": material("ford borde cubierta", (0.34, 0.36, 0.35, 1), 0.05, 0.5),
    "white": material("ford marcas blancas", (0.92, 0.93, 0.88, 1), 0.0, 0.6),
    "yellow": material("ford marcas amarillas", (1.0, 0.78, 0.08, 1), 0.0, 0.5),
    "red": material("ford rojo seguridad", (0.8, 0.04, 0.03, 1), 0.0, 0.45),
    "blue": material("ford azul sistemas", (0.04, 0.28, 0.9, 1), 0.0, 0.42),
    "green": material("ford verde sistemas", (0.04, 0.62, 0.28, 1), 0.0, 0.42),
    "orange": material("ford anotaciones naranja", (1.0, 0.62, 0.12, 1), 0.0, 0.35),
    "black": material("ford negro caucho", (0.01, 0.012, 0.012, 1), 0.0, 0.72),
    "metal": material("ford metal satinado", (0.58, 0.62, 0.62, 1), 0.45, 0.28),
    "dark_metal": material("ford metal oscuro", (0.04, 0.045, 0.045, 1), 0.55, 0.28),
    "brass": material("ford laton terminales", (0.88, 0.62, 0.18, 1), 0.55, 0.25),
    "glass": material("ford vidrio radar", (0.07, 0.18, 0.22, 0.55), 0.0, 0.2, 0.55),
    "interior": material("ford compartimentos interiores", (0.56, 0.58, 0.56, 1), 0.0, 0.62),
    "hangar": material("ford hangar claro", (0.64, 0.66, 0.64, 1), 0.0, 0.58),
    "reactor": material("ford reactor sellado generico", (0.18, 0.34, 0.38, 1), 0.2, 0.34),
    "turbine": material("ford turbina generica", (0.18, 0.23, 0.27, 1), 0.3, 0.3),
    "weapons": material("ford armamento defensivo gris", (0.68, 0.69, 0.67, 1), 0.2, 0.34),
    "radar": material("ford paneles radar", (0.05, 0.09, 0.1, 1), 0.05, 0.32),
    "aircraft": material("ford aviones gris", (0.43, 0.46, 0.47, 1), 0.08, 0.5),
    "aircraft_dark": material("ford cabinas aviones", (0.02, 0.06, 0.09, 0.72), 0.0, 0.24, 0.72),
    "component_base": material("ford bases componentes", (0.065, 0.072, 0.072, 1), 0.0, 0.72),
    "label": material("ford texto etiquetas", (0.96, 0.97, 0.93, 1), 0.0, 0.52),
    "label_soft": material("ford texto suave", (0.66, 0.8, 0.88, 1), 0.0, 0.52),
    "cut": material("ford borde corte didactico", (1.0, 0.38, 0.06, 1), 0.0, 0.36),
    "sea": material("ford base mar oscuro", (0.02, 0.045, 0.055, 1), 0.0, 0.8),
}


def active_collection():
    col = bpy.data.collections.new("USS Gerald R Ford CVN-78 didactico")
    bpy.context.scene.collection.children.link(col)
    bpy.context.view_layer.active_layer_collection = bpy.context.view_layer.layer_collection.children[col.name]
    return col


def shade(obj):
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    try:
        bpy.ops.object.shade_smooth()
    except Exception:
        pass
    obj.select_set(False)
    return obj


def bevel(obj, width=0.02, segments=2):
    mod = obj.modifiers.new("bisel detalle", "BEVEL")
    mod.width = width
    mod.segments = segments
    mod.affect = "EDGES"
    obj.modifiers.new("normales ponderadas", "WEIGHTED_NORMAL")
    return obj


def cube(name, loc, scale, mat, bevel_width=0.0, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(mat)
    if bevel_width:
        bevel(obj, bevel_width, 2)
    return obj


def cyl(name, loc, radius, depth, mat, vertices=32, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=loc, rotation=rot
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    shade(obj)
    return obj


def cone(name, loc, r1, r2, depth, mat, vertices=48, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices, radius1=r1, radius2=r2, depth=depth, location=loc, rotation=rot
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    shade(obj)
    return obj


def sphere(name, loc, scale, mat, segments=24, rings=12):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1, location=loc
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)
    shade(obj)
    return obj


def torus(name, loc, major, minor, mat, rot=(0, 0, 0), segments=64):
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
    obj.data.materials.append(mat)
    shade(obj)
    return obj


def curve(name, pts, mat, bevel_depth=0.02):
    c = bpy.data.curves.new(name, "CURVE")
    c.dimensions = "3D"
    c.resolution_u = 2
    c.bevel_depth = bevel_depth
    c.bevel_resolution = 4
    s = c.splines.new("POLY")
    s.points.add(len(pts) - 1)
    for p, co in zip(s.points, pts):
        p.co = (co[0], co[1], co[2], 1)
    obj = bpy.data.objects.new(name, c)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    return obj


def label(name, text, loc, target=None, size=0.18, mat=None):
    if mat is None:
        mat = M["label"]
    bpy.ops.object.text_add(location=loc, rotation=(math.radians(63), 0, 0))
    obj = bpy.context.object
    obj.name = name
    obj.data.body = text
    obj.data.align_x = "CENTER"
    obj.data.align_y = "CENTER"
    obj.data.size = size
    obj.data.materials.append(mat)
    if target:
        curve(f"guia_{name}", [loc, target], M["orange"], 0.01)
        sphere(f"punto_{name}", target, (0.045, 0.045, 0.045), M["orange"], 12, 6)
    return obj


def bolt(name, loc, radius=0.035, depth=0.025, axis="Z", mat=None):
    if mat is None:
        mat = M["metal"]
    rots = {
        "X": (0, math.radians(90), 0),
        "Y": (math.radians(90), 0, 0),
        "Z": (0, 0, 0),
    }
    obj = cyl(name, loc, radius, depth, mat, 6, rots[axis])
    bevel(obj, radius * 0.12, 1)
    return obj


def washer(name, loc, major=0.05, minor=0.006, axis="Z"):
    rots = {
        "X": (0, math.radians(90), 0),
        "Y": (math.radians(90), 0, 0),
        "Z": (0, 0, 0),
    }
    return torus(name, loc, major, minor, M["dark_metal"], rots[axis], 36)


def hull_mesh(origin):
    ox, oy, oz = origin
    length = 40.0
    sections = [
        (-20.0, 0.25, 0.2, -1.7),
        (-18.0, 2.7, 0.8, -2.0),
        (-12.0, 3.5, 1.15, -2.15),
        (-4.0, 3.75, 1.2, -2.2),
        (6.0, 3.55, 1.15, -2.12),
        (14.0, 2.9, 0.95, -1.92),
        (18.5, 1.2, 0.5, -1.45),
        (20.0, 0.35, 0.18, -0.95),
    ]
    angles = [math.radians(a) for a in [0, 35, 70, 110, 145, 180, 215, 250, 290, 325]]
    verts = []
    rings = []
    for x, half_w, top_z, bottom_z in sections:
        ring = []
        for a in angles:
            y = half_w * math.cos(a)
            # ellipse with a flatter deck side.
            if math.sin(a) >= 0:
                z = top_z * math.sin(a)
            else:
                z = -abs(bottom_z) * abs(math.sin(a))
            ring.append(len(verts))
            verts.append((ox + x, oy + y, oz + z))
        rings.append(ring)
    faces = []
    n = len(angles)
    for i in range(len(rings) - 1):
        for j in range(n):
            faces.append((rings[i][j], rings[i + 1][j], rings[i + 1][(j + 1) % n], rings[i][(j + 1) % n]))
    faces.append(tuple(reversed(rings[0])))
    faces.append(tuple(rings[-1]))
    mesh = bpy.data.meshes.new("ford casco malla")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new("USS_Gerald_R_Ford_casco_forma_principal", mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(M["hull"])
    shade(obj)
    bevel(obj, 0.04, 2)
    return obj


def flight_deck(origin):
    ox, oy, oz = origin
    # Main rectangular flight deck and angular bow/stern extensions.
    cube("ford cubierta_vuelo_principal", (ox, oy, oz + 1.36), (38.8, 8.65, 0.16), M["deck"], 0.04)
    cube("ford cubierta_proa_estrecha", (ox - 18.75, oy, oz + 1.36), (2.8, 6.0, 0.16), M["deck"], 0.04)
    cube("ford cubierta_popa_extendida", (ox + 18.45, oy - 0.35, oz + 1.36), (2.7, 8.0, 0.16), M["deck"], 0.04)
    # Angled landing area as a rotated overlay.
    cube("ford pista_angular", (ox + 1.4, oy - 1.0, oz + 1.47), (26.5, 2.5, 0.045), M["hull_dark"], 0.02, (0, 0, math.radians(-8)))
    cube("ford linea_central_pista_angular", (ox + 1.4, oy - 1.0, oz + 1.51), (25.0, 0.08, 0.035), M["white"], 0.004, (0, 0, math.radians(-8)))
    for i, x in enumerate([-10, -5, 0, 5, 10]):
        cube(f"ford_marca_pista_segmento_{i}", (ox + x, oy - 1.0 + 0.7 * math.sin(math.radians(-8)), oz + 1.53), (1.1, 0.12, 0.04), M["white"], 0.003, (0, 0, math.radians(-8)))
    # Deck edge stripes.
    cube("ford borde_babor_cubierta", (ox, oy + 4.43, oz + 1.51), (38.3, 0.06, 0.035), M["yellow"], 0.003)
    cube("ford borde_estribor_cubierta", (ox, oy - 4.43, oz + 1.51), (38.3, 0.06, 0.035), M["yellow"], 0.003)
    # Bow number and deck markings.
    label("ford_numero_cubierta_78", "78", (ox - 15.4, oy + 1.15, oz + 1.58), None, 0.55, M["white"])
    bpy.context.object.rotation_euler = (math.radians(90), 0, 0)
    label("ford_nombre_cubierta", "USS GERALD R. FORD  CVN-78", (ox - 3.0, oy - 4.15, oz + 1.61), None, 0.28, M["white"])
    # Catapults as long rails.
    for i, y in enumerate([1.35, 2.55, -0.55, -1.72]):
        x_center = ox - 7.2 if i < 2 else ox + 1.2
        length = 15.5 if i < 2 else 14.0
        cube(f"ford_emals_carril_{i+1}", (x_center, oy + y, oz + 1.57), (length, 0.055, 0.05), M["metal"], 0.004)
        cube(f"ford_emals_linea_amarilla_{i+1}", (x_center, oy + y + 0.14, oz + 1.58), (length, 0.035, 0.045), M["yellow"], 0.002)
        for k in range(5):
            cube(f"ford_emals_segmento_{i+1}_{k}", (x_center - length / 2 + 1.4 + k * 2.7, oy + y, oz + 1.61), (0.5, 0.12, 0.035), M["white"], 0.003)
    # Jet blast deflectors.
    for i, (x, y) in enumerate([(-11.9, 1.35), (-11.7, 2.55), (-1.7, -0.55), (-1.2, -1.72)]):
        cube(f"ford_jbd_panel_{i+1}", (ox + x, oy + y - 0.55, oz + 1.62), (1.2, 0.12, 0.5), M["deck_edge"], 0.01, (math.radians(70), 0, 0))
        for b in [-0.42, 0, 0.42]:
            cube(f"ford_jbd_refuerzo_{i+1}_{b}", (ox + x + b, oy + y - 0.63, oz + 1.83), (0.06, 0.08, 0.55), M["metal"], 0.003, (math.radians(70), 0, 0))
    # Arresting wires.
    for i, x in enumerate([5.1, 5.9, 6.7]):
        curve(f"ford_aag_cable_arresto_{i+1}", [(ox + x, oy - 3.2, oz + 1.62), (ox + x - 0.5, oy + 2.9, oz + 1.62)], M["metal"], 0.012)
    # Aircraft elevators: 3 visible edge platforms.
    for i, (x, y, sx, sy) in enumerate([(-5.7, -4.7, 4.7, 1.35), (4.0, -4.7, 4.8, 1.35), (12.2, 4.7, 4.3, 1.25)]):
        cube(f"ford_elevador_aeronaves_{i+1}", (ox + x, oy + y, oz + 1.6), (sx, sy, 0.09), M["deck_edge"], 0.02)
        for k in range(6):
            bolt(f"ford_elevador_aeronaves_{i+1}_perno_{k}", (ox + x - sx / 2 + 0.45 + k * sx / 6, oy + y, oz + 1.67), 0.025, 0.018, "Z")
    # Safety nets and deck edge stanchions.
    for side, y in [("babor", 4.68), ("estribor", -4.68)]:
        for i in range(25):
            x = ox - 18.0 + i * 1.5
            cyl(f"ford_barandilla_{side}_{i}", (x, oy + y, oz + 1.76), 0.018, 0.38, M["metal"], 10)
            if i > 0:
                curve(f"ford_red_seguridad_{side}_{i}", [(x - 1.5, oy + y, oz + 1.85), (x, oy + y, oz + 1.85)], M["metal"], 0.006)
                curve(f"ford_red_seguridad_baja_{side}_{i}", [(x - 1.5, oy + y, oz + 1.67), (x, oy + y, oz + 1.67)], M["metal"], 0.005)


def island(origin):
    ox, oy, oz = origin
    base_x = ox + 3.5
    base_y = oy - 3.35
    cube("ford_isla_base", (base_x, base_y, oz + 2.16), (4.2, 1.75, 1.3), M["hull"], 0.08)
    cube("ford_isla_puente", (base_x + 0.2, base_y - 0.03, oz + 3.15), (3.25, 1.45, 0.9), M["hull"], 0.06)
    cube("ford_isla_puente_superior", (base_x + 0.65, base_y - 0.03, oz + 3.95), (2.4, 1.22, 0.72), M["hull"], 0.05)
    cube("ford_chimenea_integrada", (base_x + 1.55, base_y + 0.18, oz + 4.65), (1.05, 1.0, 0.92), M["hull_dark"], 0.04)
    # Windows.
    for row, z in enumerate([3.2, 3.55, 3.95]):
        for i in range(7 - row):
            cube(f"ford_ventana_isla_{row}_{i}", (base_x - 1.25 + i * 0.42, base_y - 0.78, oz + z), (0.22, 0.035, 0.09), M["glass"], 0.005)
    # Radar faces.
    for i, (xoff, z, sx, sz) in enumerate([(-1.25, 3.85, 0.65, 0.55), (0.0, 4.12, 0.78, 0.48), (1.1, 3.65, 0.58, 0.45)]):
        cube(f"ford_radar_facetado_panel_{i+1}", (base_x + xoff, base_y - 0.82, oz + z), (sx, 0.045, sz), M["radar"], 0.01, (0, 0, math.radians(0)))
        for k in range(5):
            curve(f"ford_radar_panel_{i+1}_linea_{k}", [(base_x + xoff - sx/2 + 0.08 + k*sx/5, base_y - 0.855, oz + z - sz/2 + 0.06), (base_x + xoff - sx/2 + 0.08 + k*sx/5, base_y - 0.855, oz + z + sz/2 - 0.06)], M["metal"], 0.0025)
    # Masts and antennas.
    cyl("ford_mastil_principal", (base_x + 0.55, base_y, oz + 5.22), 0.07, 1.45, M["dark_metal"], 24)
    cyl("ford_mastil_secundario", (base_x - 0.55, base_y + 0.04, oz + 5.05), 0.05, 1.05, M["dark_metal"], 18)
    for i, xoff in enumerate([-0.9, -0.35, 0.2, 0.75, 1.3]):
        cyl(f"ford_antena_delgada_{i}", (base_x + xoff, base_y - 0.22, oz + 5.15 + 0.12 * (i % 2)), 0.018, 1.25 + 0.2*(i % 3), M["dark_metal"], 12)
    torus("ford_sensor_rotativo_generico", (base_x + 0.55, base_y, oz + 5.96), 0.34, 0.018, M["radar"], (math.radians(90), 0, 0), 48)
    cube("ford_numero_isla_78", (base_x - 1.9, base_y - 0.83, oz + 2.75), (0.045, 0.04, 0.38), M["white"], 0.004)
    label("ford_lbl_isla", "Isla: puente,\nradares y mastiles", (base_x + 2.7, base_y - 3.1, oz + 5.4), (base_x + 0.1, base_y - 0.8, oz + 4.0), 0.18)


def cutaway_compartments(origin):
    ox, oy, oz = origin
    y = oy + 4.85
    # Open side cutaway stack.
    levels = [
        ("hangar", oz + 0.72, 0.7, M["hangar"]),
        ("cubierta_2", oz - 0.02, 0.52, M["interior"]),
        ("maquinas", oz - 0.72, 0.62, M["interior"]),
        ("fondo_doble", oz - 1.42, 0.45, M["hull_dark"]),
    ]
    for lname, z, h, mat in levels:
        cube(f"ford_corte_{lname}", (ox, y, z), (32.0, 0.18, h), mat, 0.01)
        for i, x in enumerate([-14, -10, -6, -2, 2, 6, 10, 14]):
            cube(f"ford_mamparo_{lname}_{i}", (ox + x, y + 0.04, z), (0.08, 0.2, h), M["metal"], 0.004)
    # Hangar aircraft and vehicles.
    for i, x in enumerate([-11.5, -7.2, -2.8, 2.0, 6.4, 10.6]):
        mini_aircraft(f"ford_hangar_avion_{i}", (ox + x, y + 0.2, oz + 0.78), 0.55, parked=True)
    for i, x in enumerate([-13, -8.8, -4.8, 0.0, 4.8, 8.8, 13.0]):
        cube(f"ford_taller_hangar_caja_{i}", (ox + x, y + 0.12, oz + 0.22), (0.55, 0.22, 0.2), M["dark_metal"], 0.01)
        cyl(f"ford_taller_hangar_rueda_{i}a", (ox + x - 0.18, y + 0.0, oz + 0.11), 0.055, 0.035, M["black"], 16, (math.radians(90), 0, 0))
        cyl(f"ford_taller_hangar_rueda_{i}b", (ox + x + 0.18, y + 0.0, oz + 0.11), 0.055, 0.035, M["black"], 16, (math.radians(90), 0, 0))
    # Generic sealed nuclear propulsion and turbine spaces.
    for i, x in enumerate([-4.2, 2.6]):
        cyl(f"ford_modulo_reactor_sellado_{i+1}", (ox + x, y + 0.08, oz - 0.74), 0.42, 1.1, M["reactor"], 48, (math.radians(90), 0, 0))
        torus(f"ford_reactor_anillo_seguridad_{i+1}", (ox + x, y + 0.08, oz - 0.74), 0.43, 0.018, M["yellow"], (math.radians(90), 0, 0), 48)
        cube(f"ford_reactor_blindaje_{i+1}", (ox + x, y + 0.05, oz - 0.74), (1.2, 0.14, 1.0), M["glass"], 0.02)
    for i, x in enumerate([6.1, 8.0, 9.9]):
        cyl(f"ford_turbina_generica_{i}", (ox + x, y + 0.08, oz - 0.78), 0.32, 1.0, M["turbine"], 48, (0, math.radians(90), 0))
        for k in range(5):
            torus(f"ford_turbina_aleta_{i}_{k}", (ox + x - 0.38 + k * 0.19, y + 0.08, oz - 0.78), 0.32, 0.007, M["metal"], (0, math.radians(90), 0), 36)
    for i, x in enumerate([-13, -11.5, -10, 12, 13.5]):
        cube(f"ford_compartimento_tripulacion_{i}", (ox + x, y + 0.08, oz - 0.02), (1.0, 0.16, 0.4), M["hangar"], 0.008)
        for b in range(3):
            cube(f"ford_litera_tripulacion_{i}_{b}", (ox + x - 0.3 + b*0.3, y - 0.05, oz - 0.02), (0.22, 0.12, 0.06), M["blue"], 0.003)
    # Weapons elevators, 11 public-count schematic shafts.
    for i in range(11):
        x = ox - 15 + i * 3.0
        cube(f"ford_AWE_elevador_armas_publico_{i+1:02d}", (x, y + 0.16, oz + 0.15), (0.35, 0.16, 1.55), M["orange"], 0.012)
        cube(f"ford_AWE_plataforma_{i+1:02d}", (x, y + 0.28, oz + 0.75), (0.5, 0.08, 0.15), M["metal"], 0.006)
    # Magazines shown as generic inert educational blocks.
    for i, x in enumerate([-13.5, -11.8, -10.1, 11.0, 12.7, 14.4]):
        cube(f"ford_panal_municion_inerte_{i}", (ox + x, y + 0.1, oz - 1.42), (1.1, 0.14, 0.3), M["red"], 0.008)
        for k in range(4):
            cyl(f"ford_cilindro_inerte_{i}_{k}", (ox + x - 0.36 + k*0.24, y - 0.01, oz - 1.42), 0.05, 0.24, M["brass"], 18, (math.radians(90), 0, 0))
    # Service trunks.
    curve("ford_tuberia_azul_servicios", [(ox - 15.5, y + 0.18, oz - 1.0), (ox - 6, y + 0.18, oz - 1.0), (ox + 2, y + 0.18, oz - 1.02), (ox + 14.5, y + 0.18, oz - 1.0)], M["blue"], 0.025)
    curve("ford_tuberia_verde_ventilacion", [(ox - 14, y + 0.19, oz + 0.42), (ox - 3, y + 0.19, oz + 0.52), (ox + 8, y + 0.19, oz + 0.45), (ox + 15, y + 0.19, oz + 0.38)], M["green"], 0.02)
    curve("ford_bus_electrico_amarillo", [(ox - 12, y + 0.2, oz - 0.38), (ox - 3, y + 0.2, oz - 0.38), (ox + 7, y + 0.2, oz - 0.34), (ox + 13, y + 0.2, oz - 0.28)], M["yellow"], 0.018)
    label("ford_lbl_corte_interno", "Corte didactico:\nhangar, cubierta 2,\nmaquinas y fondo doble", (ox - 9.2, y + 2.1, oz + 2.4), (ox - 9.5, y + 0.15, oz + 0.5), 0.18)
    label("ford_lbl_reactores_genericos", "Modulos nucleares\nsellados (genericos)", (ox + 1.0, y + 2.1, oz + 1.55), (ox - 4.2, y + 0.08, oz - 0.74), 0.16)
    label("ford_lbl_AWE", "11 elevadores avanzados\nde armas: esquema publico", (ox + 9.5, y + 2.05, oz + 2.2), (ox + 9, y + 0.16, oz + 0.15), 0.16)


def mini_aircraft(name, origin, scale=1.0, parked=False):
    ox, oy, oz = origin
    cube(f"{name}_fuselaje", (ox, oy, oz), (1.0*scale, 0.16*scale, 0.16*scale), M["aircraft"], 0.025)
    cone(f"{name}_nariz", (ox - 0.56*scale, oy, oz), 0.11*scale, 0.02*scale, 0.28*scale, M["aircraft"], 24, (0, math.radians(90), 0))
    cone(f"{name}_tobera", (ox + 0.58*scale, oy, oz), 0.14*scale, 0.08*scale, 0.25*scale, M["dark_metal"], 24, (0, math.radians(90), 0))
    cube(f"{name}_ala_izq", (ox - 0.02*scale, oy + 0.32*scale, oz), (0.58*scale, 0.55*scale, 0.035*scale), M["aircraft"], 0.01, (0, 0, math.radians(-12)))
    cube(f"{name}_ala_der", (ox - 0.02*scale, oy - 0.32*scale, oz), (0.58*scale, 0.55*scale, 0.035*scale), M["aircraft"], 0.01, (0, 0, math.radians(12)))
    cube(f"{name}_cola_vert", (ox + 0.36*scale, oy, oz + 0.18*scale), (0.2*scale, 0.035*scale, 0.34*scale), M["aircraft"], 0.006)
    cube(f"{name}_estabilizador", (ox + 0.42*scale, oy, oz + 0.03*scale), (0.28*scale, 0.48*scale, 0.03*scale), M["aircraft"], 0.006)
    cube(f"{name}_cabina", (ox - 0.27*scale, oy, oz + 0.1*scale), (0.22*scale, 0.1*scale, 0.08*scale), M["aircraft_dark"], 0.01)
    if not parked:
        for dx in [-0.25, 0.3]:
            cyl(f"{name}_tren_{dx}", (ox + dx*scale, oy, oz - 0.13*scale), 0.025*scale, 0.14*scale, M["black"], 12)


def mini_hawkeye(name, origin, scale=1.0):
    ox, oy, oz = origin
    cube(f"{name}_fuselaje", (ox, oy, oz), (1.15*scale, 0.18*scale, 0.16*scale), M["aircraft"], 0.02)
    cube(f"{name}_alas", (ox, oy, oz), (0.58*scale, 1.35*scale, 0.035*scale), M["aircraft"], 0.008)
    cyl(f"{name}_radomo", (ox, oy, oz + 0.33*scale), 0.32*scale, 0.045*scale, M["radar"], 48)
    cyl(f"{name}_soporte_radomo", (ox, oy, oz + 0.2*scale), 0.025*scale, 0.25*scale, M["dark_metal"], 12)
    cube(f"{name}_cola", (ox + 0.45*scale, oy, oz + 0.12*scale), (0.24*scale, 0.55*scale, 0.04*scale), M["aircraft"], 0.005)


def deck_air_wing(origin):
    ox, oy, oz = origin
    coords = [
        (-14, 2.7, 0), (-11, 3.45, 4), (-8, 2.65, -2), (-5, 3.35, 3),
        (-1, 3.2, 0), (2.5, 2.4, -6), (8.0, 3.0, 2), (12.0, -3.0, -8),
        (14.0, -1.7, -8), (-4.0, -2.7, -8), (1.0, -3.0, -8)
    ]
    for i, (x, y, rot) in enumerate(coords):
        mini_aircraft(f"ford_deck_fighter_{i}", (ox + x, oy + y, oz + 1.78), 0.9)
        parent = bpy.context.object
        # Rotate the last created object set by name prefix.
        for obj in bpy.data.objects:
            if obj.name.startswith(f"ford_deck_fighter_{i}_"):
                obj.rotation_euler[2] += math.radians(rot)
    mini_hawkeye("ford_deck_hawkeye", (ox + 6.0, oy - 2.55, oz + 1.78), 0.95)
    # Tow tractors and deck crew dots.
    for i, (x, y) in enumerate([(-13, 1.2), (-9, -2.4), (-2, 0.9), (5.5, 1.1), (10, -0.8)]):
        cube(f"ford_tractor_cubierta_{i}", (ox + x, oy + y, oz + 1.72), (0.55, 0.32, 0.18), M["yellow"], 0.012)
        for sx in [-0.18, 0.18]:
            for sy in [-0.12, 0.12]:
                cyl(f"ford_tractor_rueda_{i}_{sx}_{sy}", (ox + x + sx, oy + y + sy, oz + 1.61), 0.045, 0.035, M["black"], 12, (math.radians(90), 0, 0))
    for i in range(42):
        x = ox - 16 + (i % 14) * 2.3
        y = oy - 3.6 + (i // 14) * 1.2
        cyl(f"ford_tripulante_cubierta_{i}", (x, y, oz + 1.67), 0.035, 0.16, [M["yellow"], M["green"], M["red"], M["blue"]][i % 4], 10)
        sphere(f"ford_casco_tripulante_{i}", (x, y, oz + 1.78), (0.035, 0.035, 0.035), M["white"], 12, 6)


def weapons_and_sponsons(origin):
    ox, oy, oz = origin
    mounts = [(-15.2, -4.9), (-5.0, -5.0), (11.5, -4.9), (16.0, 4.9), (-12.5, 4.9)]
    for i, (x, y) in enumerate(mounts):
        cube(f"ford_sponson_armamento_{i}", (ox + x, oy + y, oz + 0.9), (1.3, 0.65, 0.28), M["hull"], 0.03)
        if i % 3 == 0:
            ciws(f"ford_CIWS_generico_{i}", (ox + x, oy + y, oz + 1.18), side=1 if y > 0 else -1)
        elif i % 3 == 1:
            missile_box(f"ford_RAM_generico_{i}", (ox + x, oy + y, oz + 1.18), side=1 if y > 0 else -1)
        else:
            vls_box(f"ford_ESSM_lanzador_generico_{i}", (ox + x, oy + y, oz + 1.16))
    label("ford_lbl_armamento_defensivo", "Defensa visible:\nCIWS, RAM y ESSM\nrepresentacion generica", (ox + 14.0, oy - 7.7, oz + 2.8), (ox + 11.5, oy - 4.9, oz + 1.18), 0.17)


def ciws(prefix, loc, side=-1):
    x, y, z = loc
    cyl(f"{prefix}_base", (x, y, z), 0.22, 0.16, M["weapons"], 32)
    sphere(f"{prefix}_radomo", (x, y, z + 0.32), (0.28, 0.28, 0.28), M["weapons"], 32, 16)
    cyl(f"{prefix}_canion_1", (x, y + side * 0.28, z + 0.16), 0.035, 0.58, M["dark_metal"], 16, (math.radians(90), 0, 0))
    cyl(f"{prefix}_canion_2", (x + 0.07, y + side * 0.28, z + 0.16), 0.025, 0.55, M["dark_metal"], 16, (math.radians(90), 0, 0))
    for i in range(8):
        a = 2 * math.pi * i / 8
        bolt(f"{prefix}_perno_base_{i}", (x + 0.27*math.cos(a), y + 0.27*math.sin(a), z + 0.09), 0.018, 0.012, "Z")


def missile_box(prefix, loc, side=-1):
    x, y, z = loc
    cube(f"{prefix}_pedestal", (x, y, z), (0.34, 0.32, 0.18), M["weapons"], 0.012)
    cube(f"{prefix}_lanzador", (x, y + side * 0.18, z + 0.26), (0.52, 0.24, 0.36), M["weapons"], 0.018, (math.radians(0), math.radians(0), 0))
    for r in [-0.11, 0.0, 0.11]:
        cyl(f"{prefix}_tubo_{r}", (x + r, y + side * 0.31, z + 0.28), 0.035, 0.06, M["dark_metal"], 18, (math.radians(90), 0, 0))


def vls_box(prefix, loc):
    x, y, z = loc
    cube(f"{prefix}_caja", (x, y, z), (0.62, 0.42, 0.18), M["weapons"], 0.012)
    for ix in [-0.18, 0, 0.18]:
        for iy in [-0.11, 0.11]:
            cube(f"{prefix}_celda_{ix}_{iy}", (x + ix, y + iy, z + 0.105), (0.12, 0.1, 0.03), M["dark_metal"], 0.004)


def propellers_and_rudders(origin):
    ox, oy, oz = origin
    for y in [-1.45, -0.48, 0.48, 1.45]:
        cyl(f"ford_eje_propulsor_{y}", (ox + 20.4, oy + y, oz - 1.1), 0.08, 1.2, M["dark_metal"], 24, (0, math.radians(90), 0))
        sphere(f"ford_cubo_helice_{y}", (ox + 21.0, oy + y, oz - 1.1), (0.18, 0.18, 0.18), M["brass"], 24, 12)
        for i in range(5):
            a = 2 * math.pi * i / 5
            blade(f"ford_pala_helice_{y}_{i}", (ox + 21.12, oy + y, oz - 1.1), a, 0.48)
        cube(f"ford_timon_{y}", (ox + 20.2, oy + y * 1.08, oz - 0.55), (0.12, 0.08, 0.9), M["hull"], 0.012)


def blade(name, center, angle, radius):
    x, y, z = center
    p1 = Vector((x, y + 0.12 * math.cos(angle), z + 0.12 * math.sin(angle)))
    p2 = Vector((x + 0.15, y + radius * math.cos(angle + 0.28), z + radius * math.sin(angle + 0.28)))
    p3 = Vector((x - 0.05, y + 0.26 * math.cos(angle + 0.85), z + 0.26 * math.sin(angle + 0.85)))
    axis = Vector((1, 0, 0)) * 0.018
    verts = [tuple(p + axis) for p in [p1, p2, p3]] + [tuple(p - axis) for p in [p1, p2, p3]]
    faces = [(0, 1, 2), (5, 4, 3), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(M["brass"])
    shade(obj)
    return obj


def component_displays(origin):
    ox, oy, oz = origin
    cube("ford_componentes_plataforma", (ox, oy, oz - 0.1), (28, 6.6, 0.08), M["component_base"], 0.0)
    label("ford_componentes_titulo", "USS Gerald R. Ford: componentes aparte (didacticos)", (ox, oy - 3.2, oz + 2.45), None, 0.24)
    propulsion_train("ford_propulsion_aparte", (ox - 9.0, oy - 0.25, oz + 0.55))
    radar_display("ford_radar_aparte", (ox - 1.5, oy - 0.05, oz + 0.65))
    elevator_display("ford_elevador_aparte", (ox + 4.3, oy - 0.05, oz + 0.6))
    catapult_display("ford_emals_aparte", (ox + 9.5, oy - 0.05, oz + 0.55))
    weapons_display("ford_armamento_aparte", (ox + 13.0, oy - 0.25, oz + 0.62))


def propulsion_train(prefix, origin):
    ox, oy, oz = origin
    cube(f"{prefix}_base", (ox, oy, oz - 0.55), (6.8, 2.3, 0.08), M["component_base"], 0.0)
    cyl(f"{prefix}_reactor_sellado", (ox - 2.55, oy, oz), 0.55, 1.1, M["reactor"], 64, (math.radians(90), 0, 0))
    cube(f"{prefix}_blindaje", (ox - 2.55, oy, oz), (1.35, 0.12, 1.25), M["glass"], 0.018)
    cyl(f"{prefix}_generador_vapor_generico", (ox - 1.25, oy, oz), 0.34, 0.78, M["metal"], 48, (math.radians(90), 0, 0))
    cyl(f"{prefix}_turbina_alta", (ox - 0.05, oy, oz), 0.34, 1.0, M["turbine"], 64, (0, math.radians(90), 0))
    cyl(f"{prefix}_turbina_baja", (ox + 0.95, oy, oz), 0.38, 1.05, M["turbine"], 64, (0, math.radians(90), 0))
    for i, x in enumerate([-0.42, -0.18, 0.06, 0.3, 0.68, 0.94, 1.2]):
        torus(f"{prefix}_aleta_turbina_{i}", (ox + x, oy, oz), 0.35, 0.007, M["metal"], (0, math.radians(90), 0), 48)
    cyl(f"{prefix}_reductor", (ox + 2.0, oy, oz), 0.42, 0.55, M["dark_metal"], 48, (0, math.radians(90), 0))
    cyl(f"{prefix}_eje", (ox + 2.95, oy, oz), 0.08, 1.65, M["metal"], 32, (0, math.radians(90), 0))
    sphere(f"{prefix}_cubo_helice", (ox + 3.78, oy, oz), (0.2, 0.2, 0.2), M["brass"], 24, 12)
    for i in range(5):
        blade(f"{prefix}_helice_pala_{i}", (ox + 3.88, oy, oz), 2 * math.pi * i / 5, 0.45)
    curve(f"{prefix}_vapor_linea", [(ox - 2.15, oy + 0.45, oz + 0.2), (ox - 1.25, oy + 0.45, oz + 0.2), (ox - 0.3, oy + 0.38, oz + 0.15)], M["white"], 0.016)
    curve(f"{prefix}_eje_linea", [(ox + 1.35, oy, oz), (ox + 3.75, oy, oz)], M["yellow"], 0.012)
    for i in range(18):
        bolt(f"{prefix}_perno_base_{i}", (ox - 3.2 + (i % 9) * 0.75, oy - 0.98 + (i // 9) * 1.96, oz - 0.47), 0.018, 0.012, "Z")
    label(f"{prefix}_lbl", "Tren de propulsion\npublico/generico:\nreactor sellado,\nturbinas, eje, helice", (ox, oy - 1.85, oz + 1.2), (ox - 0.1, oy, oz), 0.13)


def radar_display(prefix, origin):
    ox, oy, oz = origin
    cube(f"{prefix}_base", (ox, oy, oz - 0.5), (3.6, 2.2, 0.08), M["component_base"], 0.0)
    cyl(f"{prefix}_mastil", (ox, oy, oz + 0.3), 0.07, 1.5, M["dark_metal"], 24)
    for i, angle in enumerate([0, 90, 180, 270]):
        a = math.radians(angle)
        x = ox + 0.42 * math.cos(a)
        y = oy + 0.42 * math.sin(a)
        cube(f"{prefix}_panel_fijo_{i}", (x, y, oz + 1.15), (0.62, 0.055, 0.52), M["radar"], 0.008, (0, 0, a))
        for k in range(6):
            curve(f"{prefix}_traza_panel_{i}_{k}", [(x - 0.25 + k*0.1, y, oz + 0.94), (x - 0.25 + k*0.1, y, oz + 1.34)], M["metal"], 0.002)
    torus(f"{prefix}_sensor_rotativo", (ox, oy, oz + 1.82), 0.45, 0.014, M["radar"], (math.radians(90), 0, 0), 48)
    label(f"{prefix}_lbl", "Radar y sensores:\npaneles facetados,\nantenas y mastil", (ox, oy - 1.55, oz + 1.55), (ox, oy, oz + 1.15), 0.13)


def elevator_display(prefix, origin):
    ox, oy, oz = origin
    cube(f"{prefix}_base", (ox, oy, oz - 0.52), (3.8, 2.4, 0.08), M["component_base"], 0.0)
    cube(f"{prefix}_pozo", (ox, oy, oz), (1.15, 0.95, 1.65), M["glass"], 0.015)
    cube(f"{prefix}_plataforma", (ox, oy, oz + 0.26), (1.25, 1.05, 0.08), M["deck_edge"], 0.012)
    for x in [-0.55, 0.55]:
        cyl(f"{prefix}_guia_{x}", (ox + x, oy - 0.5, oz + 0.2), 0.025, 1.8, M["metal"], 12)
        cyl(f"{prefix}_guia2_{x}", (ox + x, oy + 0.5, oz + 0.2), 0.025, 1.8, M["metal"], 12)
    for i in range(6):
        cube(f"{prefix}_motor_lineal_{i}", (ox - 0.7 + i*0.28, oy + 0.65, oz - 0.1), (0.12, 0.12, 0.62), M["orange"], 0.004)
    for i in range(12):
        bolt(f"{prefix}_perno_{i}", (ox - 0.55 + (i % 4)*0.36, oy - 0.55 + (i // 4)*0.36, oz + 0.34), 0.018, 0.012, "Z")
    label(f"{prefix}_lbl", "Elevador avanzado:\nplataforma, guias,\nmotores lineales", (ox, oy - 1.65, oz + 1.2), (ox, oy, oz + 0.26), 0.13)


def catapult_display(prefix, origin):
    ox, oy, oz = origin
    cube(f"{prefix}_base", (ox, oy, oz - 0.52), (4.2, 2.2, 0.08), M["component_base"], 0.0)
    cube(f"{prefix}_canal", (ox, oy, oz), (3.4, 0.32, 0.16), M["deck"], 0.006)
    cube(f"{prefix}_ranura", (ox, oy, oz + 0.11), (3.3, 0.045, 0.04), M["metal"], 0.002)
    cube(f"{prefix}_carro_lanzamiento", (ox - 0.45, oy, oz + 0.22), (0.38, 0.24, 0.12), M["yellow"], 0.008)
    for i in range(12):
        cube(f"{prefix}_bobina_lineal_{i}", (ox - 1.55 + i*0.28, oy - 0.36, oz - 0.02), (0.12, 0.1, 0.18), M["brass"], 0.004)
        cube(f"{prefix}_bobina_lineal_b_{i}", (ox - 1.55 + i*0.28, oy + 0.36, oz - 0.02), (0.12, 0.1, 0.18), M["brass"], 0.004)
    curve(f"{prefix}_cable_control", [(ox - 1.75, oy + 0.7, oz + 0.1), (ox, oy + 0.82, oz + 0.15), (ox + 1.6, oy + 0.7, oz + 0.1)], M["blue"], 0.012)
    label(f"{prefix}_lbl", "Catapulta EMALS:\nriel y bobinas\nesquematicas", (ox, oy - 1.55, oz + 1.1), (ox - 0.45, oy, oz + 0.22), 0.13)


def weapons_display(prefix, origin):
    ox, oy, oz = origin
    cube(f"{prefix}_base", (ox, oy, oz - 0.5), (3.8, 2.4, 0.08), M["component_base"], 0.0)
    ciws(f"{prefix}_ciws_aparte", (ox - 1.1, oy, oz - 0.2), side=-1)
    missile_box(f"{prefix}_ram_aparte", (ox, oy, oz - 0.18), side=-1)
    vls_box(f"{prefix}_essm_aparte", (ox + 1.1, oy, oz - 0.18))
    label(f"{prefix}_lbl", "Armas defensivas:\nformas publicas,\nno operativas", (ox, oy - 1.6, oz + 1.0), (ox, oy, oz + 0.05), 0.13)


def annotations(origin):
    ox, oy, oz = origin
    label("ford_titulo_general", "USS Gerald R. Ford (CVN-78) - modelo didactico detallado", (ox - 1, oy - 7.8, oz + 5.2), None, 0.28)
    label("ford_subtitulo_general", "Exterior visible, corte interno y componentes genericos para exposicion naval", (ox - 1, oy - 7.8, oz + 4.82), None, 0.16, M["label_soft"])
    label("ford_lbl_cubierta", "Cubierta de vuelo:\ncatapultas, cables AAG,\nelevadores y marcas", (ox - 12.8, oy - 7.4, oz + 3.2), (ox - 7.2, oy + 1.35, oz + 1.57), 0.17)
    label("ford_lbl_airwing", "Ala embarcada:\naviones y aeronave AEW\nsimplificados", (ox - 4.2, oy - 7.3, oz + 3.7), (ox - 4.0, oy - 2.7, oz + 1.78), 0.17)
    label("ford_lbl_elevadores_avion", "3 elevadores de aeronaves\nrepresentados en cubierta", (ox + 5.4, oy - 7.35, oz + 3.0), (ox + 4.0, oy - 4.7, oz + 1.6), 0.17)
    label("ford_lbl_propulsores", "4 ejes y helices:\nrepresentacion publica", (ox + 19.2, oy - 6.7, oz + 1.3), (ox + 21, oy - 1.45, oz - 1.1), 0.17)


def setup_camera_and_lights(origin, components_origin):
    ox, oy, oz = origin
    # Add area lights for the new exhibit.
    bpy.ops.object.light_add(type="AREA", location=(ox - 4, oy - 8, oz + 10))
    l = bpy.context.object
    l.name = "ford_luz_area_principal"
    l.data.energy = 900
    l.data.size = 9
    bpy.ops.object.light_add(type="POINT", location=(ox + 15, oy + 2, oz + 6))
    p = bpy.context.object
    p.name = "ford_luz_relleno"
    p.data.energy = 180
    cube("ford_plataforma_mar", (ox, oy, oz - 2.35), (47, 16, 0.08), M["sea"], 0.0)
    cube("ford_plataforma_componentes_fondo", (components_origin[0], components_origin[1], components_origin[2] - 0.2), (30, 7.5, 0.06), M["sea"], 0.0)

    bpy.ops.object.camera_add(location=(ox + 28, oy - 22, oz + 10.5))
    cam = bpy.context.object
    cam.name = "Camara_Gerald_Ford_general"
    look_at(cam, (ox + 0.5, oy - 0.6, oz + 1.2))
    cam.data.lens = 28
    bpy.context.scene.camera = cam
    bpy.context.scene.render.resolution_x = 2600
    bpy.context.scene.render.resolution_y = 1500
    bpy.context.scene.render.filepath = RENDER_MAIN
    bpy.ops.render.render(write_still=True)

    bpy.ops.object.camera_add(location=(components_origin[0] + 8, components_origin[1] - 9.5, components_origin[2] + 4.4))
    c2 = bpy.context.object
    c2.name = "Camara_Gerald_Ford_componentes"
    look_at(c2, (components_origin[0] + 1.5, components_origin[1], components_origin[2] + 0.3))
    c2.data.type = "ORTHO"
    c2.data.ortho_scale = 10.2
    bpy.context.scene.camera = c2
    bpy.context.scene.render.resolution_x = 2600
    bpy.context.scene.render.resolution_y = 1500
    bpy.context.scene.render.filepath = RENDER_COMPONENTS
    bpy.ops.render.render(write_still=True)


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def add_empty_index(origin, components_origin):
    entries = [
        ("INDICE_10_USS_Gerald_R_Ford", origin),
        ("INDICE_11_Cubierta_CVN78", (origin[0] - 7, origin[1] + 1.3, origin[2] + 1.7)),
        ("INDICE_12_Isla_Radares_CVN78", (origin[0] + 3.5, origin[1] - 3.35, origin[2] + 4.2)),
        ("INDICE_13_Corte_Interno_CVN78", (origin[0], origin[1] + 4.85, origin[2] + 0.3)),
        ("INDICE_14_Componentes_CVN78", components_origin),
    ]
    for name, loc in entries:
        e = bpy.data.objects.new(name, None)
        e.empty_display_type = "SPHERE"
        e.empty_display_size = 0.25
        e.location = loc
        bpy.context.collection.objects.link(e)


active_collection()

SHIP_ORIGIN = (0.0, 15.0, 0.2)
COMPONENTS_ORIGIN = (1.0, 28.0, 0.2)

hull_mesh(SHIP_ORIGIN)
flight_deck(SHIP_ORIGIN)
island(SHIP_ORIGIN)
cutaway_compartments(SHIP_ORIGIN)
deck_air_wing(SHIP_ORIGIN)
weapons_and_sponsons(SHIP_ORIGIN)
propellers_and_rudders(SHIP_ORIGIN)
component_displays(COMPONENTS_ORIGIN)
annotations(SHIP_ORIGIN)
add_empty_index(SHIP_ORIGIN, COMPONENTS_ORIGIN)

bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)
setup_camera_and_lights(SHIP_ORIGIN, COMPONENTS_ORIGIN)
bpy.ops.wm.save_as_mainfile(filepath=OUT_BLEND)

print(f"Archivo combinado: {OUT_BLEND}")
print(f"Render Gerald Ford: {RENDER_MAIN}")
print(f"Render componentes: {RENDER_COMPONENTS}")
