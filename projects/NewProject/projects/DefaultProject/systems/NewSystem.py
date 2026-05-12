class Newsystem:
    """
    Sistema personalizado del proyecto.
    Se prepara para cargarse desde project/systems.
    """

    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = False
        self.run_in_play = True

    def update(self, dt):
        if not self.enabled:
            return

        # Tu lógica global aquí
        pass
