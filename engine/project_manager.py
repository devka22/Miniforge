import os
import json

from engine.project_config import (
    ENGINE_NAME,
    ENGINE_VERSION,
    PROJECT_FOLDERS,
    PROJECT_FILE,
    ENGINE_CONFIG_FILE,
    WINDOW_WIDTH,
    WINDOW_HEIGHT,
    FPS,
    DEBUG_MODE,
    SHOW_GRID,
    SHOW_FPS,
)


class ProjectManager:
    PROJECT_JSON = os.path.join("project", "project.json")
    ROOT_PROJECT_JSON = "project.json"

    def __init__(self, game=None, logger=None):
        self.game = game
        self.logger = logger
        self.data = {}

    def log(self, message, level="ENGINE"):
        if self.game and hasattr(self.game, "console"):
            self.game.console.log(message, level)
            return

        if self.logger and hasattr(self.logger, "info"):
            self.logger.info(message)

    def setup_project(self):
        """
        Crea la estructura base del proyecto si no existe.
        """
        self.create_project_folders()
        self.create_project_file()
        self.create_engine_config_file()

    def create_project_folders(self):
        """
        Crea todas las carpetas necesarias para el motor.
        """
        for folder in PROJECT_FOLDERS:
            if not os.path.exists(folder):
                os.makedirs(folder)
                self.log(f"Carpeta creada: {folder}")

    def create_project_file(self):
        """
        Crea el archivo principal del proyecto.
        """
        if not os.path.exists(PROJECT_FILE):
            project_data = {
                "project_name": ENGINE_NAME,
                "engine_version": ENGINE_VERSION,
                "main_scene": "scenes/main_scene.json",
                "created_with": ENGINE_NAME,
            }

            with open(PROJECT_FILE, "w", encoding="utf-8") as file:
                json.dump(project_data, file, indent=4)

            self.log(f"Archivo de proyecto creado: {PROJECT_FILE}")

    def create_engine_config_file(self):
        """
        Crea el archivo de configuración editable del motor.
        """
        if not os.path.exists(ENGINE_CONFIG_FILE):
            config_data = {
                "window": {
                    "width": WINDOW_WIDTH,
                    "height": WINDOW_HEIGHT,
                    "fps": FPS
                },
                "debug": {
                    "debug_mode": DEBUG_MODE,
                    "show_grid": SHOW_GRID,
                    "show_fps": SHOW_FPS
                },
                "editor": {
                    "theme": "dark",
                    "auto_save": False,
                    "auto_save_interval": 60
                }
            }

            with open(ENGINE_CONFIG_FILE, "w", encoding="utf-8") as file:
                json.dump(config_data, file, indent=4)

            self.log(f"Archivo de configuración creado: {ENGINE_CONFIG_FILE}")

    def load_project_file(self):
        """
        Carga los datos del archivo project.victoria.
        """
        if not os.path.exists(PROJECT_FILE):
            self.log("No se encontró project.victoria. Creando uno nuevo...", "WARNING")
            self.create_project_file()

        with open(PROJECT_FILE, "r", encoding="utf-8") as file:
            return json.load(file)

    def load_engine_config(self):
        """
        Carga la configuración editable del motor.
        """
        if not os.path.exists(ENGINE_CONFIG_FILE):
            self.log("No se encontró engine_settings.json. Creando uno nuevo...", "WARNING")
            self.create_engine_config_file()

        with open(ENGINE_CONFIG_FILE, "r", encoding="utf-8") as file:
            return json.load(file)

    def save_engine_config(self, config_data):
        """
        Guarda cambios en engine_settings.json.
        """
        with open(ENGINE_CONFIG_FILE, "w", encoding="utf-8") as file:
            json.dump(config_data, file, indent=4)

        self.log("Configuración del motor guardada correctamente.")

    def project_exists(self):
        """
        Verifica si existe el archivo del proyecto.
        """
        return os.path.exists(PROJECT_FILE)

    def get_project_name(self):
        """
        Devuelve el nombre del proyecto.
        """
        data = self.load_project_file()
        return data.get("project_name", ENGINE_NAME)

    def default_project_data(self):
        current_scene = "main.scene"

        if self.game and hasattr(self.game, "scene_manager"):
            current_scene = self.game.scene_manager.current_scene

        return {
            "project_name": os.path.basename(os.getcwd()),
            "engine_version": ENGINE_VERSION,
            "current_scene": current_scene,
            "created_with": ENGINE_NAME,
        }

    def load_project(self):
        """
        Carga el project.json moderno usado por el editor.
        """
        os.makedirs(os.path.dirname(self.PROJECT_JSON), exist_ok=True)

        load_path = self.PROJECT_JSON

        if not os.path.exists(load_path) and os.path.exists(self.ROOT_PROJECT_JSON):
            load_path = self.ROOT_PROJECT_JSON

        if not os.path.exists(load_path):
            self.data = self.default_project_data()
            self.save_project()
            return self.data

        try:
            with open(load_path, "r", encoding="utf-8") as file:
                self.data = json.load(file)

        except Exception as error:
            self.log(f"No se pudo cargar project.json: {error}", "WARNING")
            self.data = self.default_project_data()

        if self.game and hasattr(self.game, "scene_manager"):
            current_scene = self.data.get("current_scene") or self.data.get("start_scene")

            if current_scene:
                self.game.scene_manager.current_scene = current_scene
                self.game.scene_manager.current_scene_path = (
                    self.game.scene_manager.get_scene_path(current_scene)
                )

        self.log("Proyecto cargado")
        return self.data

    def save_project(self):
        """
        Guarda metadatos del proyecto sin tocar la escena.
        """
        os.makedirs(os.path.dirname(self.PROJECT_JSON), exist_ok=True)

        data = dict(self.data or self.default_project_data())
        data.setdefault("project_name", os.path.basename(os.getcwd()))
        data["engine_version"] = ENGINE_VERSION

        if self.game and hasattr(self.game, "scene_manager"):
            data["current_scene"] = self.game.scene_manager.current_scene

        with open(self.PROJECT_JSON, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        root_data = {
            "project_name": data.get("project_name"),
            "engine_version": data.get("engine_version"),
            "start_scene": data.get("start_scene", data.get("current_scene", "main.scene")),
            "current_scene": data.get("current_scene", "main.scene"),
        }

        with open(self.ROOT_PROJECT_JSON, "w", encoding="utf-8") as file:
            json.dump(root_data, file, indent=4)

        self.data = data
        self.log("Proyecto guardado")
        return True
