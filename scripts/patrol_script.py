from engine.script import Script


class PatrolScript(Script):
    script_name = "PatrolScript"

    def __init__(self):
        super().__init__()
        self.direction = 1
        self.distance = 0
        self.max_distance = 3

    def update(self, entity, dt):
        if not self.enabled:
            return

        move = self.direction * dt

        entity.x += move
        self.distance += abs(move)

        if self.distance >= self.max_distance:
            self.direction *= -1
            self.distance = 0

    def serialize(self):
        data = super().serialize()
        data.update({
            "direction": self.direction,
            "distance": self.distance,
            "max_distance": self.max_distance
        })
        return data

    def deserialize(self, data):
        super().deserialize(data)
        self.direction = data.get("direction", 1)
        self.distance = data.get("distance", 0)
        self.max_distance = data.get("max_distance", 3)