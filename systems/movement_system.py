class MovementSystem:
    def __init__(self, game):
        self.game = game

    def update(self, dt):
        for unit in self.game.world.entities:
            unit.update(dt)
