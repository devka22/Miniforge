import json
import os


class ProjectSettings:
    """
    Configuración global del motor/proyecto.
    """

    def __init__(self):
        self.data = {
            "project_name": "RTS Engine Project",
            "window_width": 1000,
            "window_height": 700,
            "target_fps": 60,
            "start_mode": "EDITOR",
            "autosave": False,
            "grid_width": 60,
            "grid_height": 40,
            "tile_size": 32
        }

    def get(self, key, default=None):
        return self.data.get(key, default)

    def set(self, key, value):
        self.data[key] = value

    def save(self, filename="project_settings.json"):
        os.makedirs("project", exist_ok=True)

        with open(f"project/{filename}", "w") as f:
            json.dump(self.data, f, indent=4)

        print("✅ Project settings guardado")

    def load(self, filename="project_settings.json"):
        path = f"project/{filename}"

        if not os.path.exists(path):
            print("⚠ No existe project_settings.json, usando defaults")
            return

        with open(path, "r") as f:
            self.data = json.load(f)

        print("✅ Project settings cargado")