class VisualScriptSystem:
    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = True
        self.run_in_play = True

    def update(self, dt):
        runtime = getattr(self.game, "visual_script_runtime", None)

        if runtime:
            runtime.update(dt)
