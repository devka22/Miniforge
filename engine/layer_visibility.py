import json
import os


class LayerVisibility:
    """
    Maneja visibilidad y lock de layers.
    Sirve para ocultar o bloquear capas completas desde el editor.
    """

    def __init__(self, game):
        self.game = game
        self.path = "project/layer_visibility.json"

        self.visible_layers = {}
        self.locked_layers = {}

        os.makedirs("project", exist_ok=True)
        self.load()

    def load(self):
        layers = getattr(self.game, "layers", ["Default"])

        self.visible_layers = {layer: True for layer in layers}
        self.locked_layers = {layer: False for layer in layers}

        if not os.path.exists(self.path):
            self.save()
            return

        try:
            with open(self.path, "r", encoding="utf-8") as file:
                data = json.load(file)

            self.visible_layers.update(data.get("visible_layers", {}))
            self.locked_layers.update(data.get("locked_layers", {}))

        except Exception:
            self.save()

    def save(self):
        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(
                {
                    "visible_layers": self.visible_layers,
                    "locked_layers": self.locked_layers,
                },
                file,
                indent=4
            )

    def is_layer_visible(self, layer):
        return self.visible_layers.get(layer, True)

    def is_layer_locked(self, layer):
        return self.locked_layers.get(layer, False)

    def toggle_layer_visibility(self, layer):
        self.visible_layers[layer] = not self.visible_layers.get(layer, True)
        self.save()

        state = "visible" if self.visible_layers[layer] else "oculta"
        self.game.console.log(f"Layer {layer}: {state}", "ENGINE")

    def toggle_layer_lock(self, layer):
        self.locked_layers[layer] = not self.locked_layers.get(layer, False)
        self.save()

        state = "bloqueada" if self.locked_layers[layer] else "desbloqueada"
        self.game.console.log(f"Layer {layer}: {state}", "ENGINE")

    def sync_layers(self):
        for layer in getattr(self.game, "layers", []):
            self.visible_layers.setdefault(layer, True)
            self.locked_layers.setdefault(layer, False)

        self.save()