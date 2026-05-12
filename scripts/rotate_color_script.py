from engine.script import Script


class RotateColorScript(Script):
    script_name = "RotateColorScript"

    def __init__(self):
        super().__init__()
        self.timer = 0

    def update(self, entity, dt):
        if not self.enabled:
            return

        self.timer += dt

        if self.timer >= 0.5:
            self.timer = 0

            if hasattr(entity, "debug_color_flip"):
                entity.debug_color_flip = not entity.debug_color_flip
            else:
                entity.debug_color_flip = True

    def serialize(self):
        data = super().serialize()
        data.update({
            "timer": self.timer
        })
        return data

    def deserialize(self, data):
        super().deserialize(data)
        self.timer = data.get("timer", 0)