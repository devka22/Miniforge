class ViewMode:
    """
    Controla Scene View / Game View.
    """

    SCENE_VIEW = "SCENE_VIEW"
    GAME_VIEW = "GAME_VIEW"

    def __init__(self, game):
        self.game = game
        self.mode = self.SCENE_VIEW

    def toggle(self):
        if self.mode == self.SCENE_VIEW:
            self.mode = self.GAME_VIEW
            self.game.console.log("Vista: GAME VIEW", "ENGINE")
        else:
            self.mode = self.SCENE_VIEW
            self.game.console.log("Vista: SCENE VIEW", "ENGINE")

    def is_game_view(self):
        return self.mode == self.GAME_VIEW

    def is_scene_view(self):
        return self.mode == self.SCENE_VIEW