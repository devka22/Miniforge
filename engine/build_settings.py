import json
import os


class BuildSettings:
    """
    Configuración del futuro runtime/export.
    Todavía no exporta, pero deja la base lista para 0.6.
    """

    def __init__(self):
        self.path = os.path.join("settings", "build_settings.json")
        self.legacy_path = os.path.join("project", "build_settings.json")

        self.data = {
            "game_name": "My MiniForge Game",
            "start_scene": "main.scene",
            "window_width": 1100,
            "window_height": 740,
            "fullscreen": False,
            "target_fps": 60,
            "show_debug": False,
            "include_all_assets": False,
        }

        os.makedirs("settings", exist_ok=True)
        self.load()

    def load(self):
        load_path = self.path

        if not os.path.exists(load_path) and os.path.exists(self.legacy_path):
            load_path = self.legacy_path

        if not os.path.exists(load_path):
            self.save()
            return

        try:
            with open(load_path, "r", encoding="utf-8") as file:
                loaded = json.load(file)

            self.data.update(loaded)
            self.save()

        except Exception:
            self.save()

    def save(self):
        os.makedirs(os.path.dirname(self.path), exist_ok=True)

        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(self.data, file, indent=4)

    def get(self, key, default=None):
        return self.data.get(key, default)

    def set(self, key, value):
        self.data[key] = value
        self.save()
