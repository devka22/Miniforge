from engine.script import Script


class UserScript(Script):
    script_name = "UserScript"

    def __init__(self):
        super().__init__()

    def start(self, entity):
        pass

    def update(self, entity, dt):
        # Programa aquí en Python
        pass

    def on_selected(self, entity):
        pass

    def on_deselected(self, entity):
        pass
