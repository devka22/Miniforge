import os
import time


class AutosaveManager:
    """
    Sistema de autosave.
    Guarda automáticamente una copia de seguridad de la escena.
    """

    def __init__(self, game, interval_seconds=60):
        self.game = game
        self.interval_seconds = interval_seconds
        self.last_save_time = time.time()

        self.autosave_folder = "saves/autosave"
        self.autosave_path = os.path.join(self.autosave_folder, "autosave.scene")

        os.makedirs(self.autosave_folder, exist_ok=True)

    def autosave_exists(self):
        return os.path.exists(self.autosave_path)

    def update(self):
        now = time.time()

        if now - self.last_save_time >= self.interval_seconds:
            self.save()

    def save(self):
        try:
            from engine.scene_serializer import SceneSerializer

            SceneSerializer.save(self.game, self.autosave_path)
            self.last_save_time = time.time()

            self.game.console.log("Autosave realizado")

        except Exception as error:
            self.game.console.log(f"Autosave error: {error}")

    def load_autosave(self):
        try:
            from engine.scene_serializer import SceneSerializer

            if os.path.exists(self.autosave_path):
                SceneSerializer.load(self.game, self.autosave_path)
                self.game.console.log("Autosave cargado")
            else:
                self.game.console.log("No existe autosave")

        except Exception as error:
            self.game.console.log(f"Load autosave error: {error}")