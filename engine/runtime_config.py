import json
import os


class RuntimeConfig:
    """
    Configuración del runtime futuro.
    Prepara el motor para exportación en 0.6.
    """

    def __init__(self):
        self.path = "project/runtime_config.json"

        self.data = {
            "game_name": "MiniForge Game",
            "start_scene": "main.scene",
            "window_width": 1100,
            "window_height": 740,
            "fullscreen": False,
            "target_fps": 60,
            "show_debug": False,
            "show_fps": True,
            "allow_console": True,
            "allow_editor_hotkeys": True,
        }

        os.makedirs("project", exist_ok=True)
        self.load()

    def load(self):
        if not os.path.exists(self.path):
            self.save()
            return

        try:
            with open(self.path, "r", encoding="utf-8") as file:
                loaded = json.load(file)

            self.data.update(loaded)

        except Exception:
            self.save()

    def save(self):
        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(self.data, file, indent=4)

    def get(self, key, default=None):
        return self.data.get(key, default)

    def set(self, key, value):
        self.data[key] = value
        self.save()

    def serialize(self):
        return dict(self.data)