#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

from generate_neon_sombra_art import BUILDINGS, CITY_BACKDROP, SPRITE_ATLAS, SPRITE_MANIFEST, write_assets as write_pixel_art_assets


ROOT = Path(__file__).resolve().parents[1]
ENGINE_VERSION = "0.9.3.4"
WIDTH = 128
HEIGHT = 80
TILE_SIZE = 16
SPRITE_RECTS: dict[str, dict[str, float]] = {}


def write_json(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def transform(x: float, y: float) -> dict:
    return {
        "component_type": "Transform",
        "x": x,
        "y": y,
        "rotation": 0.0,
        "scale_x": 1.0,
        "scale_y": 1.0,
    }


def load_sprite_rects(force: bool = False) -> dict[str, dict[str, float]]:
    global SPRITE_RECTS
    if SPRITE_RECTS and not force:
        return SPRITE_RECTS
    manifest_path = ROOT / SPRITE_MANIFEST
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        SPRITE_RECTS = manifest.get("sprites", {})
    return SPRITE_RECTS


def sprite(
    name: str,
    tint: list[int],
    order: int,
    show_label: bool = False,
    visible: bool = True,
) -> dict:
    data = {
        "component_type": "SpriteRenderer",
        "sprite_name": name,
        "visible": visible,
        "sorting_order": order,
        "tint": tint,
        "show_label": show_label,
    }
    if name == "City_Backdrop":
        data["source_asset"] = CITY_BACKDROP
        data["_texture_path"] = CITY_BACKDROP
    else:
        rect = load_sprite_rects().get(name)
        if rect:
            data["source_asset"] = SPRITE_ATLAS
            data["_texture_path"] = SPRITE_ATLAS
            data["_source_rect"] = rect
    return data


def collider(
    width: float,
    height: float,
    layer: str,
    mask: list[str] | None = None,
    trigger: bool = False,
) -> dict:
    return {
        "component_type": "Collider2D",
        "shape": "rect",
        "width": width,
        "height": height,
        "is_trigger": trigger,
        "collision_layer": layer,
        "collision_mask": mask or ["*"],
    }


def rigidbody(body_type: str = "kinematic", gravity: bool = False) -> dict:
    return {
        "component_type": "Rigidbody2D",
        "body_type": body_type,
        "use_gravity": gravity,
        "gravity_scale": 0.0 if not gravity else 1.0,
        "freeze_rotation": True,
        "drag": 0.08,
    }


def static_body() -> dict:
    return {"component_type": "StaticBody2D", "body_type": "static", "friction": 0.8}


def light(
    radius: float,
    intensity: float,
    color: list[int],
    flicker: bool = False,
    casts_shadows: bool = True,
) -> dict:
    return {
        "component_type": "Light2D",
        "light_type": "point",
        "radius": radius,
        "intensity": intensity,
        "color": color,
        "falloff": 1.0,
        "flicker": flicker,
        "flicker_speed": 7.0,
        "casts_shadows": casts_shadows,
        "shadow_softness": 0.28,
    }


def blackboard(values: dict) -> dict:
    return {"component_type": "Blackboard", "values": values}


def saveable(save_key: str, persistent: bool = True, autosave: bool = True) -> dict:
    return {
        "component_type": "Saveable",
        "save_key": save_key,
        "include_components": True,
        "persistent": persistent,
        "version": 1,
        "autosave": autosave,
    }


def script_schedule(
    update_interval: float,
    max_distance: float = 0.0,
    distant_update_interval: float = 0.75,
    priority: int = 0,
    always_update: bool = False,
) -> dict:
    return {
        "component_type": "ScriptSchedule",
        "enabled": True,
        "always_update": always_update,
        "update_interval": update_interval,
        "max_distance": max_distance,
        "distant_update_interval": distant_update_interval,
        "priority": priority,
    }


def nav(speed: float) -> dict:
    return {
        "component_type": "NavAgent",
        "speed": speed,
        "stopping_distance": 0.25,
        "repath_interval": 0.18,
        "auto_repath": True,
        "avoid_obstacles": True,
        "path_smoothing": True,
    }


def entity(
    entity_id: int,
    name: str,
    x: float,
    y: float,
    width: float,
    height: float,
    tag: str,
    layer: str,
    components: list[dict],
    script: str | None = None,
    visible: bool = True,
    enabled: bool = True,
    speed: float = 3.5,
) -> dict:
    return {
        "type": "GameObject",
        "id": entity_id,
        "name": name,
        "enabled": enabled,
        "active": enabled,
        "visible": visible,
        "locked": False,
        "x": x,
        "y": y,
        "width": width,
        "height": height,
        "speed": speed,
        "radius": max(width, height) * 0.5,
        "script": script,
        "tag": tag,
        "layer": layer,
        "state": "IDLE",
        "command": "IDLE",
        "path": [],
        "patrol_points": [],
        "patrol_index": 0,
        "components": [transform(x, y), *components],
        "scripts": [],
    }


def ui(
    entity_id: int,
    name: str,
    text: str,
    x: float,
    y: float,
    width: float,
    height: float,
    order: int,
    color: list[int],
    text_color: list[int] | None = None,
    opacity: float = 0.9,
    align: str = "left",
    visible: bool = True,
    element_type: str = "Label",
    progress: float = 1.0,
    max_progress: float = 1.0,
) -> dict:
    text_color = text_color or [246, 241, 225]
    return entity(
        entity_id,
        name,
        0,
        0,
        1,
        1,
        "UI",
        "UI",
        [
            {
                "component_type": "UIElement",
                "element_type": element_type,
                "text": text,
                "x": x,
                "y": y,
                "width": width,
                "height": height,
                "color": color,
                "text_color": text_color,
                "opacity": opacity,
                "interactable": False,
                "sorting_order": order,
                "padding": 10,
                "border_radius": 4,
                "border_color": [83, 102, 132],
                "text_align": align,
                "font_size": 0,
                "progress": progress,
                "max_progress": max_progress,
            }
        ],
        visible=visible,
    )


def route(points: list[tuple[float, float]]) -> list[dict]:
    return [{"x": x, "y": y} for x, y in points]


def tile_layers(buildings: list[tuple[float, float, float, float]]) -> dict:
    ground = [[1 for _ in range(WIDTH)] for _ in range(HEIGHT)]
    decoration = [[0 for _ in range(WIDTH)] for _ in range(HEIGHT)]
    collision = [[0 for _ in range(WIDTH)] for _ in range(HEIGHT)]
    overlay = [[0 for _ in range(WIDTH)] for _ in range(HEIGHT)]

    for y in range(HEIGHT):
        for x in range(WIDTH):
            if x >= 118 and y >= 28:
                ground[y][x] = 7
                collision[y][x] = 1
            elif x >= 96 and y >= 34:
                ground[y][x] = 8
            elif y >= 64:
                ground[y][x] = 9
            elif x < 31 and y > 35:
                ground[y][x] = 3
            elif x < 32 and y < 31:
                ground[y][x] = 5
            elif 34 <= x <= 66 and 34 <= y:
                ground[y][x] = 4
            elif x > 68 and y > 31:
                ground[y][x] = 2
            elif x > 63 and y < 30:
                ground[y][x] = 6
            else:
                ground[y][x] = 1

    road_columns = [10, 24, 38, 52, 66, 80, 96, 112]
    road_rows = [8, 20, 32, 44, 56, 68]
    for x0 in road_columns:
        for y in range(HEIGHT):
            for dx in range(-1, 2):
                x = x0 + dx
                if 0 <= x < WIDTH:
                    decoration[y][x] = 2
    for y0 in road_rows:
        for x in range(WIDTH):
            for dy in range(-1, 2):
                y = y0 + dy
                if 0 <= y < HEIGHT:
                    decoration[y][x] = 2
    for x in range(0, WIDTH, 8):
        decoration[32][x] = 5
        overlay[32][x] = 1
    for x in range(108, 126):
        if 0 <= x < WIDTH:
            decoration[59][x] = 6
            collision[59][x] = 0
    for y in range(36, 74):
        if 94 < WIDTH:
            decoration[y][94] = 6
            decoration[y][95] = 6

    for x in range(WIDTH):
        collision[0][x] = 1
        collision[HEIGHT - 1][x] = 1
    for y in range(HEIGHT):
        collision[y][0] = 1
        collision[y][WIDTH - 1] = 1

    for bx, by, bw, bh in buildings:
        x0 = max(0, int(round(bx - bw / 2)))
        x1 = min(WIDTH - 1, int(round(bx + bw / 2)))
        y0 = max(0, int(round(by - bh / 2)))
        y1 = min(HEIGHT - 1, int(round(by + bh / 2)))
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                collision[y][x] = 1

    return {
        "width": WIDTH,
        "height": HEIGHT,
        "active_layer": 0,
        "layers": [
            {"name": "Ground", "visible": True, "locked": False, "tiles": ground},
            {"name": "Decoration", "visible": True, "locked": False, "tiles": decoration},
            {"name": "Collision", "visible": False, "locked": False, "tiles": collision},
            {"name": "Overlay", "visible": True, "locked": False, "tiles": overlay},
        ],
    }


def build_world() -> tuple[dict, list[dict]]:
    entities: list[dict] = []
    next_id = 5200

    def add(item: dict) -> None:
        entities.append(item)

    def alloc() -> int:
        nonlocal next_id
        value = next_id
        next_id += 1
        return value

    buildings = BUILDINGS

    for bx, by, bw, bh, name, tint in buildings:
        add(
            entity(
                alloc(),
                name,
                bx,
                by,
                bw,
                bh,
                "City",
                "WorldStatic",
                [
                    sprite(name, tint, -5, False, visible=False),
                    collider(bw, bh, "WorldStatic", ["Player", "NPC", "Police", "Vehicle", "Projectile"]),
                    static_body(),
                    {"component_type": "ShadowCaster2D", "shape": "box", "opacity": 0.72},
                ],
            )
        )

    road_strips = [
        ("Avenida_Cobalto", 52, 32, 88, 2.0),
        ("Ronda_del_Puerto", 24, 44, 46, 2.0),
        ("Via_Mirador", 80, 28, 2.0, 52),
        ("Eje_Sur", 24, 20, 2.0, 36),
    ]
    for name, x, y, w, h in road_strips:
        add(
            entity(
                alloc(),
                name,
                x,
                y,
                w,
                h,
                "Effects",
                "Ground",
                [sprite(name, [25, 31, 40], -20, False, visible=False)],
            )
        )

    add(
        entity(
            5100,
            "Player",
            13,
            47,
            0.85,
            0.95,
            "Player",
            "Player",
            [
                sprite("Player", [74, 203, 255], 20, True),
                collider(0.78, 0.88, "Player", ["WorldStatic", "NPC", "Police", "Vehicle", "Trigger", "Collectible"]),
                rigidbody("kinematic", False),
                {
                    "component_type": "CharacterController2D",
                    "mode": "topdown",
                    "walk_speed": 6.0,
                    "run_speed": 8.75,
                    "dash_speed": 11.0,
                    "dash_duration": 0.1,
                    "input_enabled": True,
                },
                {"component_type": "Camera2D", "active": True, "zoom": 1.45, "pixel_perfect": True, "pixels_per_unit": 16.0},
                {"component_type": "CameraFollow", "target_id": 5100, "smoothness": 9.0, "zoom": 1.45, "viewport_width": 1280.0, "viewport_height": 720.0},
                {"component_type": "CameraShake", "amplitude": 6.0, "duration": 0.25},
                {"component_type": "Health", "max_health": 100.0, "health": 100.0, "armor": 4.0},
                {"component_type": "Stats", "agility": 7.0, "vitality": 6.0, "attack": 12.0},
                {"component_type": "Inventory", "capacity": 18, "items": [{"id": "burner_phone", "quantity": 1}]},
                {"component_type": "QuestLog", "quests": [], "active_quest_id": None},
                blackboard({"wanted": 0, "cash": 120, "current_vehicle": None}),
                script_schedule(0.0, priority=200, always_update=True),
                saveable("player"),
                light(4.5, 1.2, [90, 205, 255], False, True),
                {"component_type": "Vision", "radius": 8.0, "reveals_fog": True},
                {"component_type": "Checkpoint", "checkpoint_id": "bajo_muelle", "respawn_x": 13, "respawn_y": 47, "active": True},
            ],
            "PlayerController.luau",
            speed=6.0,
        )
    )

    add(
        entity(
            5101,
            "CityDirector",
            0,
            0,
            1,
            1,
            "City",
            "Default",
            [
                blackboard(
                    {
                        "wanted": 0,
                        "reputation": 0,
                        "mission_index": 1,
                        "mission_id": "act1_rain_on_the_bridge",
                        "district": "Bajo Muelle",
                        "time_of_day": 20.35,
                        "weather": "clear",
                        "phase": "night",
                        "headlights": True,
                        "night_factor": 0.8,
                        "rain": 0.0,
                    }
                ),
                script_schedule(0.08, priority=180, always_update=True),
                saveable("city_director"),
            ],
            "CityDirector.luau",
            visible=False,
        )
    )
    add(
        entity(
            5102,
            "MenuDirector",
            0,
            0,
            1,
            1,
            "UI",
            "UI",
            [blackboard({}), script_schedule(0.0, priority=190, always_update=True)],
            "MenuDirector.luau",
            visible=False,
        )
    )
    add(entity(5103, "World_Ambient", 48, 32, 1, 1, "Effects", "Effects", [light(80, 0.8, [92, 119, 160], False, False)], visible=False))
    add(entity(5104, "World_Bloom", 48, 32, 1, 1, "Effects", "Effects", [{"component_type": "Bloom2D", "intensity": 0.62, "threshold": 0.78, "radius": 5.0}], visible=False))
    add(
        entity(
            5105,
            "City_Backdrop",
            WIDTH * 0.5,
            HEIGHT * 0.5,
            WIDTH,
            HEIGHT,
            "Effects",
            "Ground",
            [sprite("City_Backdrop", [255, 255, 255], -120, False)],
        )
    )

    contacts = [
        ("Mara_Contact", "Mara", "Mara", 14, 45, [255, 198, 113], "Tengo el mapa que Rojas borro."),
        ("Ivo_Contact", "Ivo", "Ivo", 18, 19, [255, 145, 105], "Te guardo un motor sin preguntas."),
        ("Valeria_Contact", "Valeria", "Valeria", 45, 28, [158, 214, 255], "Las pruebas entran por aqui."),
        ("Signal_Node", "Mara", "Nodo de senal", 59, 55, [186, 132, 255], "Esta antena escucha demasiado."),
        ("Luz_Contact", "Luz", "Mara", 110, 58, [255, 116, 202], "Pier 21 no duerme; solo cambia de musica."),
    ]
    for name, contact_id, display, x, y, tint, line in contacts:
        add(
            entity(
                alloc(),
                name,
                x,
                y,
                0.9,
                1.05,
                "Contact",
                "NPC",
                [
                    sprite(display, tint, 18, True),
                    collider(0.85, 1.0, "NPC", ["Player", "WorldStatic"], True),
                    {"component_type": "Area2D", "width": 2.2, "height": 2.2, "collision_layer": "Trigger", "collision_mask": ["Player"]},
                    blackboard({"contact_id": contact_id, "display_name": display, "line": line}),
                    script_schedule(0.12, 16.0, 0.8, priority=90),
                    light(4.2, 1.1, tint, True, True),
                    {"component_type": "Dialogue", "speaker": display, "lines": [line]},
                ],
                "MissionContact.luau",
            )
        )

    save_points = [
        ("SavePoint_Muelle", "Puerto seguro", 13, 49),
        ("SavePoint_Taller", "Taller Ivo", 18, 22),
        ("SavePoint_Civico", "Archivo civil", 46, 30),
        ("SavePoint_Ribera", "Casa de ribera", 75, 54),
        ("SavePoint_Playa", "Pier 21", 108, 57),
        ("SavePoint_Marina", "Marina Sur", 100, 70),
    ]
    for name, label, x, y in save_points:
        add(
            entity(
                alloc(),
                name,
                x,
                y,
                0.9,
                0.9,
                "SavePoint",
                "Trigger",
                [
                    sprite("SavePoint", [255, 230, 120], 17, True),
                    collider(0.9, 0.9, "Trigger", ["Player"], True),
                    {"component_type": "Area2D", "width": 2.4, "height": 2.4, "collision_layer": "Trigger", "collision_mask": ["Player"]},
                    {
                        "component_type": "Checkpoint",
                        "checkpoint_id": name,
                        "respawn_x": x,
                        "respawn_y": y,
                        "respawn_health": 100.0,
                        "activation_radius": 1.55,
                        "active": False,
                        "single_use": False,
                        "activated_by_tag": "Player",
                    },
                    blackboard({"label": label, "slot": "autosave", "saved": False}),
                    script_schedule(0.10, 12.0, 0.9, priority=95),
                    saveable(name),
                    light(3.8, 1.05, [255, 224, 112], True, True),
                ],
                "SavePointBrain.luau",
            )
        )

    marina_props = [
        ("Yacht_La_Perla", 121.5, 52.5, [242, 244, 235], 0.0),
        ("Yacht_Marea_Roja", 123.0, 61.5, [255, 225, 238], 3.0),
        ("Yacht_Sombra", 119.5, 70.0, [220, 239, 232], -4.0),
    ]
    for name, x, y, tint, rotation in marina_props:
        prop = entity(
            alloc(),
            name,
            x,
            y,
            3.35,
            1.35,
            "City",
            "WorldStatic",
            [
                sprite("Yacht", tint, 6, True),
                collider(3.1, 1.05, "WorldStatic", ["Player", "NPC", "Police", "Vehicle"]),
                static_body(),
                blackboard({"marina_prop": True, "district": "Playa Neon"}),
                light(4.2, 0.95, [105, 235, 255], True, True),
                {"component_type": "ShadowCaster2D", "shape": "box", "opacity": 0.48},
            ],
        )
        prop["components"][0]["rotation"] = rotation
        add(prop)

    pedestrian_routes = [
        [(12, 47), (24, 47), (24, 56), (12, 56)],
        [(38, 33), (52, 33), (52, 44), (38, 44)],
        [(66, 28), (80, 28), (80, 42), (66, 42)],
        [(24, 20), (38, 20), (38, 32), (24, 32)],
        [(52, 8), (66, 8), (66, 20), (52, 20)],
        [(80, 44), (90, 44), (90, 56), (80, 56)],
        [(99, 38), (110, 42), (108, 55), (98, 58)],
        [(96, 68), (112, 68), (112, 73), (96, 73)],
        [(38, 68), (54, 68), (54, 74), (38, 74)],
        [(96, 20), (112, 20), (112, 32), (96, 32)],
    ]
    personas = ["mensajera", "vendedor", "estudiante", "taxista", "enfermera", "portuario", "turista", "musico", "corredora", "guardia", "surfista", "cocinera"]
    routines = ["commute", "sell", "study", "drive", "care", "dock", "tour", "perform", "jog", "watch", "beach", "night_shift"]
    for i in range(48):
        path = pedestrian_routes[i % len(pedestrian_routes)]
        start = path[i % len(path)]
        persona = personas[i % len(personas)]
        add(
            entity(
                alloc(),
                f"NPC_{i + 1:02d}",
                start[0] + (i % 3) * 0.35,
                start[1] + ((i // 3) % 3) * 0.25,
                0.72,
                0.86,
                "NPC",
                "NPC",
                [
                    sprite("Pedestrian", [180 + (i * 17) % 55, 205 + (i * 11) % 40, 190 + (i * 13) % 45], 12, False),
                    collider(0.65, 0.78, "NPC", ["WorldStatic", "Player", "Police", "Vehicle"]),
                    rigidbody("kinematic", False),
                    nav(2.0 + (i % 5) * 0.18),
                    {"component_type": "AIController", "behavior": "wander", "target_tags": ["Player"], "detection_radius": 5.0},
                    blackboard(
                        {
                            "persona": persona,
                            "routine": routines[i % len(routines)],
                            "route": route(path),
                            "route_index": 1,
                            "speed": 2.0 + (i % 5) * 0.18,
                            "district": "Playa Neon" if i % len(pedestrian_routes) in (6, 7) else ("Bajo Muelle" if i % len(pedestrian_routes) == 0 else "Santa Aurelia"),
                            "mood": "wander",
                        }
                    ),
                    script_schedule(0.30 + (i % 4) * 0.07, 34.0, 1.25 + (i % 5) * 0.12, priority=20),
                ],
                "PedestrianBrain.luau",
                speed=2.0,
            )
        )

    police_routes = [
        [(8, 44), (24, 44), (24, 56), (8, 56)],
        [(38, 20), (52, 20), (52, 32), (38, 32)],
        [(66, 8), (80, 8), (80, 20), (66, 20)],
        [(66, 44), (90, 44), (90, 56), (66, 56)],
        [(96, 32), (112, 32), (112, 56), (96, 56)],
        [(80, 68), (112, 68), (112, 44), (80, 44)],
    ]
    for i, path in enumerate(police_routes):
        start = path[0]
        add(
            entity(
                alloc(),
                f"PatrolOfficer_{i + 1}",
                start[0],
                start[1],
                0.85,
                0.95,
                "Police",
                "Police",
                [
                    sprite("Officer", [104, 150, 255], 19, True),
                    collider(0.8, 0.9, "Police", ["WorldStatic", "Player", "NPC", "Vehicle"]),
                    rigidbody("kinematic", False),
                    nav(3.1),
                    {"component_type": "AIController", "behavior": "guard", "target_tags": ["Player"], "detection_radius": 9.0, "attack_radius": 1.1},
                    {"component_type": "DamageDealer", "damage": 5.0, "range": 1.1, "target_tags": ["Player"], "cooldown": 1.0},
                    blackboard({"route": route(path), "route_index": 1, "speed": 3.1, "state": "patrol"}),
                    script_schedule(0.16, 70.0, 0.70, priority=80),
                    light(5.0, 1.15, [130, 180, 255], False, True),
                    {"component_type": "Vision", "radius": 10.0, "reveals_fog": False},
                ],
                "PoliceBrain.luau",
                speed=3.1,
            )
        )

    roadblocks = [
        (38, 32, 4.6, 0.65, "Avenida Cobalto"),
        (52, 32, 4.6, 0.65, "Centro Civico"),
        (66, 44, 4.8, 0.65, "Ribera Este"),
        (24, 44, 4.6, 0.65, "Bajo Muelle"),
        (80, 20, 4.8, 0.65, "Mirador Norte"),
        (10, 56, 4.4, 0.65, "Muelle 17"),
        (52, 56, 4.8, 0.65, "Mercado Viejo"),
        (96, 44, 4.8, 0.65, "Ocean Drive"),
        (112, 56, 4.8, 0.65, "Pier 21"),
    ]
    for index, (x, y, width, height, district) in enumerate(roadblocks, start=1):
        add(
            entity(
                alloc(),
                f"Roadblock_{index:02d}",
                x,
                y,
                width,
                height,
                "Police",
                "WorldStatic",
                [
                    sprite("Roadblock", [66, 122, 255], 22, True),
                    collider(width, height, "WorldStatic", ["Player", "NPC", "Police", "Vehicle", "Projectile"]),
                    static_body(),
                    blackboard({"district": district, "active": False, "roadblock": True}),
                    script_schedule(0.40, 60.0, 1.25, priority=15),
                    light(4.5, 1.65, [105, 165, 255], True, True),
                    {"component_type": "ShadowCaster2D", "shape": "box", "opacity": 0.86},
                ],
                visible=False,
                enabled=False,
            )
        )

    vehicle_routes = [
        [(10, 8), (80, 8), (80, 20), (10, 20)],
        [(24, 4), (24, 56), (52, 56), (52, 8)],
        [(38, 56), (90, 56), (90, 44), (38, 44)],
        [(66, 8), (66, 56), (80, 56), (80, 8)],
        [(96, 8), (112, 8), (112, 56), (96, 56)],
        [(52, 68), (112, 68), (112, 44), (52, 44)],
    ]
    for i in range(20):
        path = vehicle_routes[i % len(vehicle_routes)]
        start = path[i % len(path)]
        police_car = i in (2, 7, 15)
        vehicle_name = f"PoliceCar_{i + 1:02d}" if police_car else f"TrafficCar_{i + 1:02d}"
        display_name = "Patrulla" if police_car else f"Auto civil {i + 1:02d}"
        cruise_speed = 4.7 if not police_car else 5.2
        drive_speed = 9.8 if not police_car else 10.8
        add(
            entity(
                alloc(),
                vehicle_name,
                start[0] + (i % 2) * 0.4,
                start[1] + (i % 3) * 0.3,
                1.65,
                0.95,
                "Police" if police_car else "Vehicle",
                "Police" if police_car else "Vehicle",
                [
                    sprite("PoliceCar" if police_car else "Car", [90, 135, 255] if police_car else [218, 181 - (i * 7) % 40, 105 + (i * 9) % 60], 14, police_car),
                    collider(1.55, 0.85, "Police" if police_car else "Vehicle", ["WorldStatic", "Player", "NPC", "Police", "Vehicle"]),
                    rigidbody("kinematic", False),
                    nav(cruise_speed),
                    blackboard({
                        "route": route(path),
                        "route_index": 1,
                        "speed": cruise_speed,
                        "drive_speed": drive_speed,
                        "display_name": display_name,
                        "drivable": True,
                        "occupied": False,
                        "vehicle_class": "police" if police_car else "civilian",
                    }),
                    script_schedule(0.18 if police_car else 0.34, 72.0 if police_car else 46.0, 0.85 if police_car else 1.35, priority=75 if police_car else 35),
                    light(3.2 if police_car else 2.2, 1.4 if police_car else 0.75, [120, 165, 255] if police_car else [255, 215, 150], police_car, True),
                ],
                "TrafficBrain.luau",
                speed=cruise_speed,
            )
        )

    lamps = []
    for x in [10, 24, 38, 52, 66, 80, 96, 112]:
        for y in [8, 20, 32, 44, 56, 68]:
            if (x + y) % 3 != 0:
                lamps.append((x + 1.4, y + 1.0))
    for i, (x, y) in enumerate(lamps[:42]):
        add(
            entity(
                alloc(),
                f"StreetLamp_{i + 1:02d}",
                x,
                y,
                0.28,
                0.28,
                "Effects",
                "Effects",
                [
                    sprite("Lamp", [255, 226, 150], 8, False),
                    light(6.0, 1.2 + (i % 4) * 0.08, [255, 219, 154], i % 5 == 0, True),
                ],
            )
        )

    pickups = [
        ("Sobre_Mara", 21, 47, 75),
        ("Chip_Camaras", 58, 55, 120),
        ("Llaves_Taller", 18, 22, 50),
        ("Prueba_Fiscal", 46, 30, 160),
        ("Dinero_Muelle", 9, 56, 90),
    ]
    for name, x, y, value in pickups:
        add(
            entity(
                alloc(),
                name,
                x,
                y,
                0.52,
                0.52,
                "Collectible",
                "Collectible",
                [
                    sprite("Pickup", [116, 232, 255], 16, True),
                    collider(0.5, 0.5, "Trigger", ["Player"], True),
                    blackboard({"value": value}),
                    script_schedule(0.16, 12.0, 1.0, priority=85),
                    light(3.4, 1.25, [116, 232, 255], True, True),
                ],
                "PickupBrain.luau",
            )
        )

    hud = [
        ("HUD_Minimap", "", 24, 512, 170, 170, 12, [8, 16, 26], [255, 255, 255], 0.96, "Minimap"),
        ("HUD_Mission", "Acto I: Lluvia sobre el puente", 260, 594, 720, 34, 10, [17, 18, 29], [255, 225, 172], 0.86, "Label"),
        ("HUD_Objective", "Habla con Mara en Bajo Muelle.", 260, 634, 720, 30, 10, [12, 23, 35], [213, 235, 248], 0.78, "Label"),
        ("HUD_Dialogue", "Mara tiene archivos que prueban que la red de camaras fue alterada.", 288, 548, 664, 34, 10, [32, 20, 35], [255, 234, 190], 0.84, "Label"),
        ("HUD_Radio", "Radio Puerto: la niebla entra baja por los muelles.", 212, 672, 570, 28, 10, [11, 23, 34], [180, 219, 238], 0.78, "Label"),
        ("HUD_Prompt", "", 438, 506, 410, 34, 11, [48, 27, 26], [255, 224, 160], 0.0, "Label"),
        ("HUD_Cash", "$000120", 1032, 24, 160, 32, 10, [7, 34, 30], [80, 255, 206], 0.86, "Label"),
        ("HUD_Clock", "20:21", 1198, 24, 60, 32, 10, [15, 26, 38], [206, 239, 255], 0.8, "Label"),
        ("HUD_Weather", "Noche clara", 1032, 226, 226, 26, 10, [12, 22, 32], [166, 234, 255], 0.78, "Label"),
        ("HUD_Health", "VIDA", 1032, 64, 160, 18, 10, [24, 18, 30], [255, 221, 230], 0.84, "StatBar"),
        ("HUD_Armor", "CHALECO", 1032, 88, 160, 18, 10, [18, 24, 35], [210, 236, 255], 0.84, "StatBar"),
        ("HUD_Weapon", "PISTOLA 333-30", 1032, 114, 226, 30, 10, [23, 20, 35], [255, 229, 170], 0.82, "Label"),
        ("HUD_Wanted", "Busqueda 0", 1032, 152, 226, 28, 10, [41, 18, 31], [255, 185, 205], 0.86, "Label"),
        ("HUD_WantedStars", "", 1032, 186, 226, 34, 10, [12, 16, 28], [255, 255, 255], 0.9, "WantedStars"),
        ("HUD_Heat", "Sospecha - patrullas curiosas", 966, 258, 292, 28, 10, [26, 23, 39], [210, 226, 255], 0.72, "Label"),
        ("HUD_District", "Bajo Muelle", 42, 486, 138, 26, 13, [12, 22, 32], [166, 234, 255], 0.82, "Label"),
        ("HUD_Reputation", "Rep 0", 42, 460, 96, 24, 13, [34, 26, 17], [255, 222, 139], 0.78, "Label"),
        ("Menu_Backdrop", "", 0, 0, 1280, 720, 100, [3, 8, 15], [255, 255, 255], 0.92, "Label"),
        ("Menu_Title", "NEON SOMBRA", 122, 84, 560, 76, 101, [7, 14, 25], [105, 232, 255], 0.42, "Label"),
        ("Menu_Subtitle", "Santa Aurelia / modo historia", 122, 176, 720, 46, 102, [34, 20, 38], [255, 202, 145], 0.62, "Label"),
        ("Menu_Command", "Continuar historia", 122, 288, 488, 54, 103, [67, 25, 39], [255, 235, 198], 0.78, "Label"),
        ("Menu_Deco_1", "BAJO MUELLE  |  CENTRO CIVICO  |  RIBERA", 122, 404, 720, 36, 104, [12, 24, 38], [147, 210, 238], 0.54, "Label"),
        ("Menu_Deco_2", "La luz no perdona. La ciudad tampoco.", 122, 462, 720, 34, 105, [20, 20, 30], [224, 203, 177], 0.46, "Label"),
    ]
    for item in hud:
        name, text, x, y, w, h, order, color, text_color, opacity, element_type = item
        progress = 100.0 if name in ("HUD_Health", "HUD_Armor") else 1.0
        max_progress = 100.0 if name in ("HUD_Health", "HUD_Armor") else 1.0
        add(ui(alloc(), name, text, x, y, w, h, order, color, text_color, opacity, element_type=element_type, progress=progress, max_progress=max_progress))

    scene = {
        "format": "miniforge.scene",
        "schema_version": 1,
        "version": ENGINE_VERSION,
        "engine_version": ENGINE_VERSION,
        "scene_name": "main",
        "mode": "PLAY",
        "active_tool": "Select",
        "tile_brush": 0,
        "brush_size": 1,
        "grid": {"width": WIDTH, "height": HEIGHT, "tile_size": TILE_SIZE, "chunk_size": 8},
        "camera": {"x": 0.0, "y": 0.0, "zoom": 1.45},
        "tilemap_layers": tile_layers([(b[0], b[1], b[2], b[3]) for b in buildings]),
        "tiles": [],
        "control_groups": {},
        "settings": {
            "world": "Santa Aurelia",
            "lighting": "raycast_light2d_shadowcasters",
            "story_mode": True,
            "world_partition_hint": {"cell_size": 24, "streaming_ready": True},
        },
        "editor_view_settings": {},
        "ui_canvases": [],
        "entities": entities,
    }
    return scene, entities


def write_data_assets() -> None:
    write_json(
        ROOT / "assets/data/StoryBible.json",
        {
            "title": "Neon Sombra",
            "city": "Santa Aurelia",
            "genre": "2D cenital de crimen, persecucion y conspiracion civica",
            "themes": ["corrupcion visible", "memoria urbana", "lealtad bajo presion"],
            "protagonist": {
                "name": "Nico Sombra",
                "role": "mensajero de alto riesgo",
                "wound": "su hermano fue culpado por una redada montada",
                "want": "sacar a la luz el archivo de camaras adulteradas",
            },
            "acts": [
                {
                    "id": "act1",
                    "name": "Lluvia sobre el puente",
                    "beats": [
                        "Mara entrega el primer mapa falso.",
                        "Ivo prepara una ruta de escape por Talleres Sur.",
                        "Rojas presenta una version oficial imposible.",
                    ],
                },
                {
                    "id": "act2",
                    "name": "La hora azul",
                    "beats": [
                        "Valeria abre una carpeta judicial.",
                        "La red policial aprende a predecir rutas del jugador.",
                        "El nodo Signal_Node revela que alguien borra barrios completos del mapa.",
                        "Luz abre Pier 21 y revela yates que viajan sin luces.",
                        "La caja negra de la marina conecta a Rojas con rutas fuera de carretera.",
                    ],
                },
                {
                    "id": "act3",
                    "name": "Ciudad cerrada",
                    "beats": [
                        "Rojas corta puentes y usa patrullas como muros moviles.",
                        "Ivo entrega un auto frio para atravesar el muro policial.",
                        "Valeria exige una entrega con cadena de custodia intacta.",
                        "Mara filtra la verdad en la radio pirata.",
                        "Nico elige entre exponer a todos o salvar a su hermano.",
                    ],
                },
            ],
        },
    )
    write_json(
        ROOT / "assets/data/CityDistricts.json",
        {
            "districts": [
                {"id": "bajo_muelle", "name": "Bajo Muelle", "mood": "humedo, industrial, leal", "danger": 2},
                {"id": "talleres_sur", "name": "Talleres Sur", "mood": "metal caliente y carreras clandestinas", "danger": 3},
                {"id": "centro_civico", "name": "Centro Civico", "mood": "vigilado, limpio, hostil", "danger": 4},
                {"id": "mirador_norte", "name": "Mirador Norte", "mood": "dinero viejo y sirenas lejanas", "danger": 2},
                {"id": "mercado_viejo", "name": "Mercado Viejo", "mood": "neon, ruido, rumores", "danger": 3},
                {"id": "ribera", "name": "La Ribera", "mood": "verde oscuro, fiestas caras, secretos", "danger": 2},
                {"id": "playa_neon", "name": "Playa Neon", "mood": "arena fria, musica baja y agua negra", "danger": 2},
                {"id": "marina_sur", "name": "Marina Sur", "mood": "gasolina marina, yates cerrados, camaras privadas", "danger": 3},
                {"id": "coral_norte", "name": "Coral Norte", "mood": "hoteles altos, valet nervioso y ventanas encendidas", "danger": 2},
            ],
            "landmarks": ["Taller_Ivo", "Centro_Civico", "Mercado_Viejo", "Comisaria_Norte", "Casa_Radio", "Pier_21_Entrada", "Yacht_Club"],
            "world_size_tiles": [WIDTH, HEIGHT],
            "streaming_plan": {"chunk_size_tiles": 24, "future_chunks": "saves/scenes/chunks"},
        },
    )
    write_json(
        ROOT / "assets/data/WeatherProfiles.json",
        {
            "simulation": "CityDirector publishes phase/weather/headlights into Blackboard for any game script.",
            "day_length_seconds": 1333,
            "profiles": [
                {"id": "clear", "name": "Claro", "ambient": 1.0, "traffic": 1.0, "headlights": False},
                {"id": "cloudy", "name": "Nublado", "ambient": 0.84, "traffic": 0.94, "headlights": False},
                {"id": "rain", "name": "Lluvia", "ambient": 0.66, "traffic": 0.82, "headlights": True},
                {"id": "storm", "name": "Tormenta suave", "ambient": 0.52, "traffic": 0.72, "headlights": True},
                {"id": "coastal_fog", "name": "Niebla costera", "ambient": 0.60, "traffic": 0.76, "headlights": True},
            ],
            "consumers": ["TrafficBrain", "PoliceBrain", "PedestrianBrain", "Light2D", "Bloom2D", "HUD_Weather"],
        },
    )
    write_json(
        ROOT / "assets/data/MissionGraph.json",
        {
            "missions": [
                {
                    "id": "act1_rain_on_the_bridge",
                    "contact": "Mara_Contact",
                    "objectives": ["meet_mara", "collect_sobre_mara", "escape_first_patrol"],
                    "unlocks": ["act1_engine_for_a_ghost"],
                },
                {
                    "id": "act1_engine_for_a_ghost",
                    "contact": "Ivo_Contact",
                    "objectives": ["meet_ivo", "learn_vehicle_routes"],
                    "unlocks": ["act2_the_blue_hour"],
                },
                {
                    "id": "act2_the_blue_hour",
                    "contact": "Valeria_Contact",
                    "objectives": ["reach_civic_center", "avoid_level_3_wanted"],
                    "unlocks": ["act2_clean_signal"],
                },
                {
                    "id": "act2_clean_signal",
                    "contact": "Signal_Node",
                    "objectives": ["disable_node", "survive_dispatch"],
                    "unlocks": ["act2_pier_21"],
                },
                {
                    "id": "act2_pier_21",
                    "contact": "Luz_Contact",
                    "objectives": ["meet_luz", "inspect_pier_21", "identify_dark_yacht"],
                    "unlocks": ["act2_marina_blackbox"],
                },
                {
                    "id": "act2_marina_blackbox",
                    "contact": "Luz_Contact",
                    "objectives": ["recover_blackbox", "leave_marina_without_level_4"],
                    "unlocks": ["act3_rojas_wall"],
                },
                {
                    "id": "act3_rojas_wall",
                    "contact": "Ivo_Contact",
                    "objectives": ["return_to_ivo", "cross_roadblocks"],
                    "unlocks": ["act3_open_file"],
                },
                {
                    "id": "act3_open_file",
                    "contact": "Valeria_Contact",
                    "objectives": ["deliver_final_archive", "keep_witnesses_alive"],
                    "unlocks": [],
                },
            ]
        },
    )
    write_json(
        ROOT / "assets/data/AIProfiles.json",
        {
            "pedestrians": {
                "states": ["wander", "comment", "uneasy", "panic", "witness", "return"],
                "stimuli": ["weapon_discharge", "police_chase", "vehicle_near_miss", "wanted_changed", "weather_changed", "time_phase_changed"],
                "social_memory": ["district", "persona", "routine", "last_line", "witness_timer"],
            },
            "police": {
                "states": ["patrol", "investigate", "chase", "search", "arrest"],
                "senses": ["distance", "raycast_line_of_sight", "wanted_level", "last_known_position", "weather_visibility"],
                "dispatch": ["foot patrols", "vehicle pursuit", "roadblocks at wanted tier 3+"],
                "wanted_tiers": [1, 2, 3, 4, 5],
            },
            "traffic": {
                "states": ["route_follow", "yield", "horn", "player_controlled", "resume"],
                "rules": ["stop near player", "stop near police", "loop district routes", "headlights in night/rain/fog", "weather traffic factor", "F enters/exits drivable cars"],
            },
        },
    )
    write_json(
        ROOT / "assets/data/LightingPlan.json",
        {
            "tech": "Runtime Light2D fan renderer using raycasts against ShadowCaster2D AABBs",
            "global": "World_Ambient changes intensity with time_of_day and weather",
            "dynamic": ["player lamp", "street lamps", "police lights", "vehicle headlights", "contact halos", "pier/yacht lights", "projectile tracers"],
            "shadow_casters": ["all buildings"],
            "runtime_pass": ["night overlay", "raycast light fans", "shadow wedges", "visible source cores"],
            "post": ["World_Bloom tracks wanted level"],
        },
    )
    write_json(
        ROOT / "assets/data/MenuStyle.json",
        {
            "name": "Neon noir dossier",
            "palette": ["cyan signage", "pink neon", "warm sodium", "deep ink", "police blue"],
            "layout": "compact GTA-style gameplay HUD plus full-screen pause dossier",
            "motion_hooks": ["pause toggle", "mission title refresh", "wanted bloom", "minimap sweep", "wanted stars"],
            "dynamic_fields": ["mission title", "district", "dispatch heat", "reputation", "objective waypoint"],
        },
    )
    write_json(
        ROOT / "assets/data/OptimizationPlan.json",
        {
            "rendering": [
                "single city backdrop texture for visual world art",
                "tilemap kept as gameplay/collision data and fallback renderer",
                "camera culling for world sprites and tile ranges",
                "debug grid disabled unless MINIFORGE_RUNTIME_DEBUG_GRID=1",
                "per-light shadow caster culling before ray fans",
                "graphics presets cap shadow lights, light samples, minimap dots and drawn entities",
            ],
            "scripting": [
                "Luau runtime clones the world snapshot once per event batch instead of once per script call",
                "script handler detection skips fixed/update/event calls when a script does not implement that handler",
                "ScriptSchedule component staggers NPC, traffic, police and prompt updates by priority and distance",
                "runtime_config.script_scheduler caps update scripts per frame while letting high priority scripts bypass the cap",
            ],
            "content": [
                "pixel-art atlas for characters, vehicles, props and roadblocks",
                "invisible collision/shadow building bodies over painted city art",
                "large beach/marina detail baked into backdrop instead of hundreds of active props",
                "runtime UI widgets for minimap and wanted stars",
            ],
            "simulation": [
                "city director publishes day phase, weather, rain, fog, traffic factor and headlights",
                "NPC, police and traffic consume the same world state without renderer-specific coupling",
                "save points use reusable Game.save_slot Luau API",
            ],
            "inspired_by": ["streamed world art", "HUD readability", "data-driven dispatch layers", "open-world time/weather loops"],
        },
    )
    write_json(
        ROOT / "assets/data/GraphicsProfiles.json",
        {
            "active": "medium",
            "profiles": {
                "low": {
                    "description": "FPS primero: luces sin sombras y menos entidades dibujadas.",
                    "lighting_enabled": True,
                    "shadow_lights_enabled": False,
                    "light_sample_budget": 18,
                    "max_shadow_lights": 0,
                    "max_drawn_entities": 260,
                },
                "medium": {
                    "description": "Equilibrado para ciudad amplia con raycast lighting selectivo.",
                    "lighting_enabled": True,
                    "shadow_lights_enabled": True,
                    "light_sample_budget": 28,
                    "max_shadow_lights": 8,
                    "max_drawn_entities": 520,
                },
                "high": {
                    "description": "Mas calidad visual con mas sombras y sprites visibles.",
                    "lighting_enabled": True,
                    "shadow_lights_enabled": True,
                    "light_sample_budget": 44,
                    "max_shadow_lights": 14,
                    "max_drawn_entities": 900,
                },
                "ultra": {
                    "description": "Capturas o equipos potentes.",
                    "lighting_enabled": True,
                    "shadow_lights_enabled": True,
                    "light_sample_budget": 72,
                    "max_shadow_lights": 32,
                    "max_drawn_entities": 1600,
                },
            },
            "runtime_override": "MINIFORGE_GRAPHICS_QUALITY=low|medium|high|ultra",
        },
    )


def write_project_config(entity_count: int) -> None:
    write_json(
        ROOT / "project.json",
        {
            "author": "MiniForge",
            "description": "Neon Sombra, juego 2D cenital de mundo abierto con historia, IA urbana y policia reactiva.",
            "engine_version": ENGINE_VERSION,
            "license": "GPL-3.0",
            "project_name": "Neon Sombra",
            "start_scene": "main.scene",
        },
    )
    engine_config = json.loads((ROOT / "engine_config.json").read_text(encoding="utf-8"))
    engine_config["project_name"] = "Neon Sombra"
    engine_config["start_scene"] = "main.scene"
    engine_config["safe_mode"] = True
    engine_config.setdefault("rendering", {})["post_processing"] = True
    engine_config.setdefault("rendering", {})["pixel_perfect"] = True
    engine_config.setdefault("runtime", {})["quality_preset"] = "high"
    engine_config.setdefault("runtime", {})["graphics_quality"] = "medium"
    write_json(ROOT / "engine_config.json", engine_config)

    runtime_config = json.loads((ROOT / "settings/runtime_config.json").read_text(encoding="utf-8"))
    runtime_config["game_name"] = "Neon Sombra"
    runtime_config["start_scene"] = "main.scene"
    runtime_config["max_entities"] = max(5000, entity_count + 1000)
    runtime_config["streaming_enabled"] = True
    runtime_config["window_width"] = 1280
    runtime_config["window_height"] = 720
    runtime_config["quality_preset"] = "medium"
    runtime_config["world_simulation"] = {
        "day_night_enabled": True,
        "weather_enabled": True,
        "vehicle_headlights": True,
        "day_length_seconds": 1333,
        "weather_min_seconds": 28,
        "weather_max_seconds": 50,
    }
    runtime_config["script_scheduler"] = {
        "enabled": True,
        "max_update_scripts_per_frame": 58,
        "default_update_interval": 0.0,
        "distant_update_interval": 1.10,
        "budget_bypass_priority": 100,
    }
    runtime_config["graphics"] = {
        "quality": "medium",
        "lighting_enabled": True,
        "shadow_lights_enabled": True,
        "light_sample_budget": 28,
        "max_shadow_lights": 8,
        "max_drawn_entities": 520,
        "profiles": {
            "low": {"light_sample_budget": 18, "max_shadow_lights": 0, "max_drawn_entities": 260},
            "medium": {"light_sample_budget": 28, "max_shadow_lights": 8, "max_drawn_entities": 520},
            "high": {"light_sample_budget": 44, "max_shadow_lights": 14, "max_drawn_entities": 900},
            "ultra": {"light_sample_budget": 72, "max_shadow_lights": 32, "max_drawn_entities": 1600},
        },
    }
    write_json(ROOT / "settings/runtime_config.json", runtime_config)

    write_json(
        ROOT / "settings/tags.json",
        {
            "items": [
                "Untagged",
                "Player",
                "NPC",
                "Police",
                "Vehicle",
                "Contact",
                "SavePoint",
                "Collectible",
                "Projectile",
                "City",
                "Effects",
                "Trigger",
                "UI",
            ]
        },
    )
    write_json(
        ROOT / "settings/layers.json",
        {
            "items": [
                "Default",
                "Ground",
                "Player",
                "NPC",
                "Police",
                "Vehicle",
                "WorldStatic",
                "Trigger",
                "SavePoint",
                "Collectible",
                "Projectile",
                "Effects",
                "UI",
                "IgnoreSelection",
                "EditorOnly",
            ]
        },
    )
    write_json(
        ROOT / "settings/input_map.json",
        {
            "actions": {
                "Move": {"display_name": "Move", "category": "Gameplay", "devices": ["keyboard", "gamepad"]},
                "Run": {"display_name": "Run", "category": "Gameplay", "devices": ["keyboard", "gamepad"]},
                "Fire": {"display_name": "Fire", "category": "Gameplay", "devices": ["mouse", "keyboard", "gamepad"]},
                "Interact": {"display_name": "Interact", "category": "Gameplay", "devices": ["keyboard", "gamepad"]},
                "Pause": {"display_name": "Pause", "category": "System", "devices": ["keyboard", "gamepad"]},
                "EnterVehicle": {"display_name": "Enter Vehicle", "category": "Gameplay", "devices": ["keyboard", "gamepad"]},
            },
            "bindings": {
                "Move": ["keyboard:wasd", "keyboard:arrows", "gamepad:left_stick"],
                "Run": ["shift", "gamepad:left_trigger"],
                "Fire": ["mouse_left", "ctrl", "gamepad:right_trigger"],
                "Interact": ["e", "enter", "gamepad:west"],
                "Pause": ["escape", "gamepad:start"],
                "EnterVehicle": ["f", "gamepad:north"],
                "move_left": ["A", "Left"],
                "move_right": ["D", "Right"],
                "move_up": ["W", "Up"],
                "move_down": ["S", "Down"],
                "run": ["Shift"],
                "fire": ["MouseLeft", "Ctrl"],
                "interact": ["E", "Enter"],
                "pause": ["Escape"],
                "enter_vehicle": ["F"],
            },
        },
    )
    write_json(
        ROOT / "manifest.json",
        {
            "engine_version": ENGINE_VERSION,
            "engine_stream_version": ENGINE_VERSION,
            "runtime": "rust",
            "scenes": ["saves/scenes/main.scene", "saves/scenes/TopDown_Level.scene"],
            "scripts": sorted(str(path.relative_to(ROOT)) for path in (ROOT / "scripts").glob("*.luau")),
            "assets": {
                "data": sorted(str(path.relative_to(ROOT)) for path in (ROOT / "assets/data").glob("*.json")),
                "sprites": sorted(str(path.relative_to(ROOT)) for path in (ROOT / "assets/sprites").glob("*")),
            },
            "components": ["Light2D", "ShadowCaster2D", "NavAgent", "AIController", "ScriptSchedule", "QuestLog", "Dialogue", "UIElement"],
            "systems": ["Luau", "Runtime2D", "Gameplay", "Physics", "Audio", "Particles"],
        },
    )


def main() -> None:
    write_pixel_art_assets()
    load_sprite_rects(True)
    write_data_assets()
    scene, entities = build_world()
    write_json(ROOT / "saves/scenes/main.scene", scene)
    write_json(ROOT / "saves/scenes/TopDown_Level.scene", scene | {"scene_name": "TopDown_Level"})
    write_project_config(len(entities))
    print(f"Generated Neon Sombra with {len(entities)} entities at {ROOT}")


if __name__ == "__main__":
    main()
