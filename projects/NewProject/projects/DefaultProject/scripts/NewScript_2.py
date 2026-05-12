class Newscript:
    """
    Script creado desde MiniForge.
    Este archivo vive dentro del proyecto, no dentro del motor.
    """

    def __init__(self):
        self.script_name = "Newscript"
        self.enabled = True
        self.started = False

    def start(self, entity):
        entity.game.console.log("Newscript started", "SCRIPT")

    def update(self, entity, dt):
        # Tu lógica aquí
        pass

    def on_selected(self, entity):
        pass

    def on_deselected(self, entity):
        pass

    def serialize(self):
        return {
            "script": self.script_name,
            "enabled": self.enabled,
        }

    def deserialize(self, data):
        self.enabled = data.get("enabled", True)
