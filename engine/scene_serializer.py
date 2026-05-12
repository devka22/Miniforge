import json
import os
import shutil
import time

from engine.prefab_manager import PrefabManager
from engine.version import ENGINE_VERSION


class SceneSerializer:
    """
    Guarda/carga escenas completas.
    Compatible con 0.6.0.
    """

    @staticmethod
    def save(game, filename=None):
        if filename is None:
            filename = "saves/scenes/main.scene"

        os.makedirs(os.path.dirname(filename), exist_ok=True)

        grid_data = None

        if hasattr(game.grid, "serialize"):
            grid_data = game.grid.serialize()

        data = {
            "version": "0.6.0",
            "engine_version": ENGINE_VERSION,
            "scene_name": os.path.basename(filename),
            "mode": game.mode,
            "active_tool": game.active_tool,
            "tile_brush": game.tile_brush,
            "brush_size": getattr(game, "brush_size", 1),
            "camera": {
                "x": game.camera.x,
                "y": game.camera.y,
                "zoom": game.camera.zoom
            },
            "control_groups": game.serialize_control_groups(),
            "grid": grid_data,
            "tiles": getattr(game.grid, "tiles", []),
            "settings": {
                "target_fps": game.settings.get("target_fps", 60) if hasattr(game, "settings") else 60,
                "grid_width": getattr(game.grid, "width", 0),
                "grid_height": getattr(game.grid, "height", 0),
                "tile_size": getattr(game.grid, "tile_size", 32),
            },
            "entities": [
                unit.serialize()
                for unit in game.units
            ],
        }

        if hasattr(game, "tilemap_layers"):
            data["tilemap_layers"] = game.tilemap_layers.serialize()

        if hasattr(game, "audio_mixer"):
            data["audio_mixer"] = game.audio_mixer.serialize()

        if hasattr(game, "animation_graphs"):
            data["animation_graphs"] = game.animation_graphs.serialize()

        if hasattr(game, "editor_view_settings"):
            data["editor_view_settings"] = game.editor_view_settings.serialize()

        if os.path.exists(filename):
            backup = f"{filename}.{time.strftime('%Y%m%d_%H%M%S')}.bak"

            try:
                shutil.copy2(filename, backup)
            except Exception:
                pass

        with open(filename, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        game.console.log(f"Escena guardada: {filename}", "SCENE")

    @staticmethod
    def load(game, filename=None):
        if filename is None:
            filename = "saves/scenes/main.scene"

        if not os.path.exists(filename):
            game.console.log(f"No existe escena: {filename}", "WARNING")
            return False

        try:
            with open(filename, "r", encoding="utf-8") as file:
                data = json.load(file)

        except Exception as error:
            game.console.log(f"Error cargando escena: {error}", "ERROR")
            SceneSerializer.quarantine_corrupt_scene(game, filename)
            return False

        data = SceneSerializer.migrate(data)

        grid_data = data.get("grid")

        if grid_data and hasattr(game.grid, "deserialize"):
            try:
                game.grid.deserialize(grid_data)
            except Exception as error:
                game.console.log(f"No se pudo cargar grid: {error}", "WARNING")
        elif data.get("tiles") is not None:
            try:
                game.grid.tiles = data.get("tiles") or game.grid.tiles

                if hasattr(game.grid, "rebuild_chunks"):
                    game.grid.rebuild_chunks()
            except Exception as error:
                game.console.log(f"No se pudo cargar tiles: {error}", "WARNING")

        if hasattr(game, "tilemap_layers") and data.get("tilemap_layers"):
            try:
                game.tilemap_layers.deserialize(data.get("tilemap_layers"))
            except Exception as error:
                game.console.log(f"No se pudo cargar tilemap layers: {error}", "WARNING")

        if hasattr(game, "audio_mixer") and data.get("audio_mixer"):
            try:
                game.audio_mixer.deserialize(data.get("audio_mixer"))
            except Exception as error:
                game.console.log(f"No se pudo cargar audio mixer: {error}", "WARNING")

        if hasattr(game, "animation_graphs") and data.get("animation_graphs"):
            try:
                game.animation_graphs.deserialize(data.get("animation_graphs"))
            except Exception as error:
                game.console.log(f"No se pudo cargar animation graphs: {error}", "WARNING")

        game.units.clear()
        game.selected_units.clear()

        for entity_data in data.get("entities", []):
            unit = PrefabManager.entity_from_data(
                game,
                entity_data,
                preserve_id=True
            )

            if unit:
                game.units.append(unit)

        camera_data = data.get("camera", {})
        game.camera.x = camera_data.get("x", game.camera.x)
        game.camera.y = camera_data.get("y", game.camera.y)
        game.camera.zoom = camera_data.get("zoom", game.camera.zoom)

        game.mode = data.get("mode", game.mode)
        game.active_tool = data.get("active_tool", "Select")
        game.tile_brush = data.get("tile_brush", 0)
        game.brush_size = data.get("brush_size", getattr(game, "brush_size", 1))

        game.world.entities = game.units

        if hasattr(game, "deserialize_control_groups"):
            game.deserialize_control_groups(data.get("control_groups", {}))

        if hasattr(game, "editor_view_settings"):
            settings = data.get("editor_view_settings", {})

            for key, value in settings.items():
                game.editor_view_settings.set(key, value)

        if hasattr(game, "history"):
            game.history.take_snapshot("Load Scene")

        game.console.log(f"Escena cargada: {filename}", "SCENE")
        return True

    @staticmethod
    def migrate(data):
        version = data.get("version")

        if version == "0.6.0":
            data.setdefault("engine_version", ENGINE_VERSION)
            data.setdefault("tiles", data.get("tiles", []))
            data.setdefault("settings", {})
            return data

        data.setdefault("version", "0.6.0")
        data.setdefault("engine_version", ENGINE_VERSION)
        data.setdefault("entities", data.get("objects", []))
        data.setdefault("tiles", data.get("tiles", []))
        data.setdefault("settings", {})
        data.setdefault("control_groups", {})
        data.setdefault("brush_size", 1)
        data.setdefault("camera", {"x": 0, "y": 0, "zoom": 1})
        return data

    @staticmethod
    def quarantine_corrupt_scene(game, filename):
        if not filename or not os.path.exists(filename):
            return

        try:
            backup = f"{filename}.corrupt_{time.strftime('%Y%m%d_%H%M%S')}"
            shutil.copy2(filename, backup)
            game.console.log(f"Escena corrupta respaldada: {backup}", "WARNING")
        except Exception:
            pass
