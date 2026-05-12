import json
import os


class EditorViewSettings:
    """
    Opciones visuales del viewport.
    Controla grid, gizmos, paths, nombres, colliders, chunks, coordenadas, etc.
    """

    def __init__(self, game):
        self.game = game
        self.path = "project/editor_view_settings.json"

        self.data = {
            "show_grid": True,
            "show_gizmos": True,
            "show_paths": True,
            "show_names": True,
            "show_bounds": True,
            "show_colliders": True,
            "show_chunks": False,
            "show_tile_coordinates": False,
            "show_mouse_tile": True,
            "show_entity_ids": False,
            "show_layer_colors": False,
            "show_brush_preview": True,
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

    def get(self, key, default=False):
        return self.data.get(key, default)

    def set(self, key, value):
        self.data[key] = value
        self.save()

    def toggle(self, key):
        self.data[key] = not self.data.get(key, False)
        self.save()

        self.game.console.log(
            f"{key}: {self.data[key]}",
            "EDITOR"
        )

    def serialize(self):
        return dict(self.data)