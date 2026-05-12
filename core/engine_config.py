import json
import os

from engine.version import ENGINE_VERSION


class EngineConfig:
    """
    Configuracion estable de MiniForge Beta.
    Crea engine_config.json si falta y conserva claves existentes.
    """

    DEFAULTS = {
        "engine_name": "MiniForge",
        "engine_alt_name": "Mini Forte",
        "engine_version": ENGINE_VERSION,
        "start_scene": "main.scene",
        "autosave": True,
        "autosave_interval_seconds": 60,
        "safe_mode": True,
        "pause_input_in_play": False,
        "logs": {
            "engine": "logs/engine.log",
            "error": "logs/error.log",
        },
    }

    def __init__(self, project_path="."):
        self.project_path = project_path
        self.path = os.path.join(project_path, "engine_config.json")
        self.data = {}
        self.load()

    def load(self):
        self.data = dict(self.DEFAULTS)

        if os.path.exists(self.path):
            try:
                with open(self.path, "r", encoding="utf-8") as file:
                    existing = json.load(file)

                self.deep_update(self.data, existing)
            except Exception:
                pass

        self.save()
        return self.data

    def save(self):
        os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)

        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(self.data, file, indent=4)

    def get(self, key, default=None):
        return self.data.get(key, default)

    def set(self, key, value):
        self.data[key] = value
        self.save()

    def deep_update(self, target, source):
        for key, value in source.items():
            if isinstance(value, dict) and isinstance(target.get(key), dict):
                self.deep_update(target[key], value)
            else:
                target[key] = value
