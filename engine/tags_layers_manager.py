import json
import os


class TagsLayersManager:
    """
    Maneja tags y layers desde archivos.
    Ya no dependemos solo de listas quemadas en game.py.
    """

    def __init__(self, game):
        self.game = game

        self.tags_path = "project/tags.json"
        self.layers_path = "project/layers.json"

        self.default_tags = [
            "Untagged",
            "Player",
            "Enemy",
            "Resource",
            "Building",
            "Projectile",
            "Neutral",
            "CameraTarget",
        ]

        self.default_layers = [
            "Default",
            "Ground",
            "Units",
            "Buildings",
            "UI",
            "Effects",
            "IgnoreSelection",
            "EditorOnly",
        ]

        self.tags = []
        self.layers = []

        os.makedirs("project", exist_ok=True)

        self.load()

    def load(self):
        self.tags = self.load_list(self.tags_path, self.default_tags)
        self.layers = self.load_list(self.layers_path, self.default_layers)

    def save(self):
        self.save_list(self.tags_path, self.tags)
        self.save_list(self.layers_path, self.layers)

    def load_list(self, path, default):
        if not os.path.exists(path):
            self.save_list(path, default)
            return list(default)

        try:
            with open(path, "r", encoding="utf-8") as file:
                data = json.load(file)

            values = data.get("items", default)

            if not values:
                return list(default)

            return list(dict.fromkeys(values))

        except Exception:
            self.save_list(path, default)
            return list(default)

    def save_list(self, path, values):
        with open(path, "w", encoding="utf-8") as file:
            json.dump({"items": values}, file, indent=4)

    def add_tag(self, tag):
        tag = tag.strip()

        if not tag:
            return False

        if tag not in self.tags:
            self.tags.append(tag)
            self.save()
            self.game.console.log(f"Tag creado: {tag}", "ENGINE")
            return True

        return False

    def add_layer(self, layer):
        layer = layer.strip()

        if not layer:
            return False

        if layer not in self.layers:
            self.layers.append(layer)
            self.save()
            self.game.console.log(f"Layer creada: {layer}", "ENGINE")
            return True

        return False

    def remove_tag(self, tag):
        if tag == "Untagged":
            self.game.console.log("No puedes eliminar Untagged", "WARNING")
            return False

        if tag in self.tags:
            self.tags.remove(tag)
            self.save()
            return True

        return False

    def remove_layer(self, layer):
        if layer == "Default":
            self.game.console.log("No puedes eliminar Default", "WARNING")
            return False

        if layer in self.layers:
            self.layers.remove(layer)
            self.save()
            return True

        return False

    def cycle_tag(self, current):
        if current not in self.tags:
            return self.tags[0]

        index = self.tags.index(current)
        return self.tags[(index + 1) % len(self.tags)]

    def cycle_layer(self, current):
        if current not in self.layers:
            return self.layers[0]

        index = self.layers.index(current)
        return self.layers[(index + 1) % len(self.layers)]