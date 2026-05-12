class UISystem:
    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = True
        self.run_in_play = True
        self.update_when_paused = True

    def update(self, dt):
        canvas = getattr(self.game, "ui_canvas", None)

        if canvas:
            canvas.update(dt)
