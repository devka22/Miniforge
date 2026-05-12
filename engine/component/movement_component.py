from engine.component import Component

class MovementComponent(Component):
    def update(self, entity, dt):
        entity.x += 0.01