import os
import json

from engine.project_config import DEFAULT_SCENE_NAME
from engine.version import ENGINE_VERSION


class SceneManager:
    def __init__(self, game=None, logger=None):
        self.game = game
        self.logger = logger
        self.scene_folder = os.path.join("saves", "scenes")
        self.legacy_scene_folder = "scenes"
        self.current_scene = "main.scene"
        self.current_scene_path = self.get_scene_path(self.current_scene)
        self.scenes = []
        self.refresh()

    def log(self, message, level="SCENE"):
        if self.game and hasattr(self.game, "console"):
            self.game.console.log(message, level)
            return

        if self.logger and hasattr(self.logger, "info"):
            self.logger.info(message)

    def get_scene_path(self, scene_name=DEFAULT_SCENE_NAME):
        return os.path.join(self.scene_folder, scene_name)

    def refresh(self):
        os.makedirs(self.scene_folder, exist_ok=True)
        os.makedirs(self.legacy_scene_folder, exist_ok=True)

        self.scenes = sorted(
            filename
            for filename in os.listdir(self.scene_folder)
            if filename.endswith(".scene")
        )

        if self.current_scene not in self.scenes and self.scenes:
            self.current_scene = self.scenes[0]

        if not self.scenes:
            self.ensure_default_scene()
            self.scenes = [self.current_scene]

        self.current_scene_path = self.get_scene_path(self.current_scene)
        return self.scenes

    def ensure_default_scene(self):
        default_path = self.get_scene_path(self.current_scene)

        if os.path.exists(default_path):
            return default_path

        try:
            with open(default_path, "w", encoding="utf-8") as file:
                json.dump(
                    {
                        "scene_name": self.current_scene,
                        "engine_version": ENGINE_VERSION,
                        "version": "0.6.0",
                        "entities": [],
                        "tiles": [],
                        "camera": {"x": 0, "y": 0, "zoom": 1.0},
                        "settings": {},
                    },
                    file,
                    indent=4,
                )
        except Exception as error:
            self.log(f"No se pudo crear escena inicial: {error}", "ERROR")

        return default_path

    def create_new_scene(self):
        from engine.scene_serializer import SceneSerializer

        os.makedirs(self.scene_folder, exist_ok=True)

        index = 1
        scene_name = "main.scene"

        while os.path.exists(self.get_scene_path(scene_name)):
            scene_name = f"scene_{index}.scene"
            index += 1

        self.current_scene = scene_name
        self.current_scene_path = self.get_scene_path(scene_name)

        if self.game:
            SceneSerializer.save(self.game, self.current_scene_path)
        else:
            self.save_scene([], scene_name)

        self.refresh()
        self.log(f"Escena creada: {scene_name}")
        return self.current_scene_path

    def save_current_scene(self):
        if not self.game:
            return False

        from engine.scene_serializer import SceneSerializer

        self.current_scene_path = self.get_scene_path(self.current_scene)
        SceneSerializer.save(self.game, self.current_scene_path)
        self.refresh()
        return True

    def load_current_scene(self):
        if not self.game:
            return False

        from engine.scene_serializer import SceneSerializer

        self.current_scene_path = self.get_scene_path(self.current_scene)
        loaded = SceneSerializer.load(self.game, self.current_scene_path)

        if loaded:
            self.refresh()

        return loaded

    def next_scene(self):
        self.refresh()

        if not self.scenes:
            self.log("No hay escenas disponibles", "WARNING")
            return None

        index = self.scenes.index(self.current_scene)
        self.current_scene = self.scenes[(index + 1) % len(self.scenes)]
        self.current_scene_path = self.get_scene_path(self.current_scene)
        self.load_current_scene()
        return self.current_scene

    def save_scene(self, objects, scene_name=DEFAULT_SCENE_NAME):
        """
        Guarda una escena en formato JSON.
        """
        if not os.path.exists(self.scene_folder):
            os.makedirs(self.scene_folder)

        scene_path = self.get_scene_path(scene_name)

        scene_data = {
            "scene_name": scene_name,
            "engine_version": ENGINE_VERSION,
            "version": "0.6.0",
            "objects": objects,
            "entities": objects,
            "tiles": [],
            "camera": {},
            "settings": {},
        }

        with open(scene_path, "w", encoding="utf-8") as file:
            json.dump(scene_data, file, indent=4)

        self.log(f"Escena guardada: {scene_path}")

    def load_scene(self, scene_name=DEFAULT_SCENE_NAME):
        """
        Carga una escena desde JSON.
        """
        scene_path = self.get_scene_path(scene_name)

        if not os.path.exists(scene_path):
            self.log(f"No existe la escena: {scene_path}", "WARNING")
            return []

        try:
            with open(scene_path, "r", encoding="utf-8") as file:
                scene_data = json.load(file)
        except Exception as error:
            self.log(f"No se pudo cargar escena JSON: {error}", "ERROR")
            return []

        self.log(f"Escena cargada: {scene_path}")

        return scene_data.get("entities", scene_data.get("objects", []))

    def create_empty_scene(self, scene_name=DEFAULT_SCENE_NAME):
        """
        Crea una escena vacía.
        """
        self.save_scene([], scene_name)

        self.log(f"Escena vacía creada: {scene_name}")
