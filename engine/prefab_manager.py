import os
import json

from entities.unit import Unit
from entities.game_object import GameObject, game_object_from_data
from engine.component import component_from_data


class PrefabManager:
    """
    Sistema completo de prefabs.
    Compatible con 0.6.0:
    - visible
    - locked
    - nuevos componentes
    - IDs únicos al instanciar
    - preserve_id al cargar escena
    """

    PREFAB_FOLDER = "assets/prefabs"

    @staticmethod
    def ensure_folder():
        os.makedirs(PrefabManager.PREFAB_FOLDER, exist_ok=True)

    @staticmethod
    def safe_filename(name):
        clean = "".join(c for c in name if c.isalnum() or c in ["_", "-"]).strip()

        if not clean:
            clean = "unit"

        return clean.lower()

    @staticmethod
    def save_prefab(entity, filename=None):
        PrefabManager.ensure_folder()

        if filename is None:
            base = getattr(entity, "name", "unit")
            filename = f"{PrefabManager.safe_filename(base)}.prefab"

        if not filename.endswith(".prefab"):
            filename += ".prefab"

        path = os.path.join(PrefabManager.PREFAB_FOLDER, filename)

        data = {
            "version": "0.6.0",
            "prefab_name": filename,
            "entity": entity.serialize()
        }

        with open(path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        return path

    @staticmethod
    def entity_from_data(game, entity_data, preserve_id=False):
        if not entity_data:
            return None

        entity_type = entity_data.get("type", "Unit")

        if entity_type == "GameObject":
            obj = game_object_from_data(game, entity_data, preserve_id=preserve_id)

            if obj and not preserve_id:
                source_name = entity_data.get("name", "GameObject")
                obj.name = f"{source_name}_Instance"

            return obj

        if entity_type != "Unit":
            game.console.log("Tipo de entidad no soportado", "WARNING")
            return None

        entity_id = entity_data.get("id") if preserve_id else None

        position = entity_data.get("position")
        x = entity_data.get("x", 0)
        y = entity_data.get("y", 0)

        if isinstance(position, (list, tuple)) and len(position) >= 2:
            x = position[0]
            y = position[1]

        unit = Unit(
            x,
            y,
            game,
            entity_id=entity_id,
            name=entity_data.get("name")
        )

        if not preserve_id:
            source_name = entity_data.get("name", "Unit")
            unit.name = f"{source_name}_Instance"

        unit.enabled = entity_data.get("enabled", entity_data.get("active", True))
        unit.active = unit.enabled
        unit.visible = entity_data.get("visible", True)
        unit.locked = entity_data.get("locked", False)

        unit.rotation = entity_data.get("rotation", 0.0)
        scale = entity_data.get("scale", [entity_data.get("scale_x", 1.0), entity_data.get("scale_y", 1.0)])
        size = entity_data.get("size", [entity_data.get("width", 1.0), entity_data.get("height", 1.0)])

        if isinstance(scale, (list, tuple)) and len(scale) >= 2:
            unit.scale_x = float(scale[0])
            unit.scale_y = float(scale[1])

        if isinstance(size, (list, tuple)) and len(size) >= 2:
            unit.width = float(size[0])
            unit.height = float(size[1])

        unit.speed = entity_data.get("speed", 3.5)
        unit.radius = entity_data.get("radius", 0.45)
        unit.sprite_name = entity_data.get("sprite_name")
        unit.script = entity_data.get("script")

        unit.tag = entity_data.get("tag", "Untagged")
        unit.layer = entity_data.get("layer", "Default")

        unit.state = entity_data.get("state", "IDLE")
        unit.command = entity_data.get("command", "IDLE")

        unit.prefab_source = entity_data.get("prefab_source")
        unit.prefab_guid = entity_data.get("prefab_guid")
        unit.is_prefab_instance = entity_data.get("is_prefab_instance", False)
        unit.parent_id = entity_data.get("parent_id")
        unit.local_x = entity_data.get("local_x", 0.0)
        unit.local_y = entity_data.get("local_y", 0.0)
        unit.sprite_guid = entity_data.get("sprite_guid")

        unit.patrol_points = entity_data.get("patrol_points", [])
        unit.patrol_index = entity_data.get("patrol_index", 0)

        unit.follow_target_id = entity_data.get("follow_target_id")
        unit.guard_target_id = entity_data.get("guard_target_id")
        unit.attack_move_target = entity_data.get("attack_move_target")
        unit.gather_target_id = entity_data.get("gather_target_id")

        unit.components.clear()

        for component_data in entity_data.get("components", []):
            component = component_from_data(component_data)

            if component:
                unit.add_component(component)

        for script_data in entity_data.get("scripts", []):
            script_name = script_data.get("script") or script_data.get("script_name")

            if not script_name:
                continue

            script = game.script_manager.create(script_name)

            if script:
                if hasattr(script, "deserialize"):
                    script.deserialize(script_data)

                unit.add_script(script)

        unit.sync_to_components()

        return unit

    @staticmethod
    def load_prefab(game, path):
        if not os.path.exists(path):
            game.console.log(f"No existe prefab: {path}", "WARNING")
            return None

        try:
            with open(path, "r", encoding="utf-8") as file:
                data = json.load(file)

        except Exception as error:
            game.console.log(f"Error leyendo prefab: {error}", "ERROR")
            return None

        entity_data = data.get("entity")

        if not entity_data:
            game.console.log("Prefab inválido", "ERROR")
            return None

        unit = PrefabManager.entity_from_data(
            game,
            entity_data,
            preserve_id=False
        )

        if unit:
            unit.prefab_source = path
            unit.is_prefab_instance = True

        return unit

    @staticmethod
    def instantiate_prefab(game, path, x, y):
        unit = PrefabManager.load_prefab(game, path)

        if not unit:
            return None

        unit.x = float(x)
        unit.y = float(y)
        unit.sync_to_components()

        game.units.append(unit)
        game.world.entities = game.units

        return unit
