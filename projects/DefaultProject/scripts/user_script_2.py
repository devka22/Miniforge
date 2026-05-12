from engine.script import Script


class UserScript2(Script):
    script_name = "UserScript2"

    def __init__(self):
        super().__init__()

    def start(self, entity):
        # Se ejecuta una vez cuando el script inicia
        pass

    def update(self, entity, dt):
        # Se ejecuta cada frame en modo PLAY
        # ejemplo:
        # entity.x += dt * 2
        pass

    def on_selected(self, entity):
        # Se ejecuta cuando seleccionas la entidad
        pass

    def on_deselected(self, entity):
        # Se ejecuta cuando deseleccionas la entidad
        pass
