import os
import json
import ast


class ProjectValidator:
    """
    MiniForge 0.6.0
    Validador de proyecto.

    Revisa:
    - carpetas obligatorias
    - project.json
    - manifest.json
    - JSON inválidos
    - scripts con errores de sintaxis
    - escenas faltantes
    - prefabs inválidos
    """

    def __init__(self, game):
        self.game = game
        self.errors = []
        self.warnings = []

    def validate(self):
        self.errors.clear()
        self.warnings.clear()

        self.validate_folders()
        self.validate_project_files()
        self.validate_json_files()
        self.validate_scripts()
        self.validate_scenes()
        self.validate_prefabs()
        self.validate_asset_references()
        self.validate_build_settings()

        self.print_results()

        return len(self.errors) == 0

    def paths(self):
        return self.game.project_paths

    def add_error(self, message):
        self.errors.append(message)

    def add_warning(self, message):
        self.warnings.append(message)

    def validate_folders(self):
        required = [
            "assets",
            "sprites",
            "audio",
            "data",
            "prefabs",
            "scripts",
            "components",
            "systems",
            "scenes",
            "settings",
            "logs",
            "plugins",
            "builds",
        ]

        for key in required:
            path = self.paths().get(key)

            if not path:
                self.add_error(f"Falta ruta project_paths['{key}']")
                continue

            if not os.path.exists(path):
                self.add_error(f"Falta carpeta: {path}")

    def validate_project_files(self):
        project_json_candidates = [
            os.path.join(self.game.project_path, "project.json"),
            os.path.join(self.game.project_path, "project", "project.json"),
        ]
        engine_config = os.path.join(self.game.project_path, "engine_config.json")
        manifest_json_candidates = [
            os.path.join(self.game.project_path, "manifest.json"),
            os.path.join(self.game.project_path, "project", "manifest.json"),
        ]

        if not any(os.path.exists(path) for path in project_json_candidates):
            self.add_error("Falta project.json")

        if not any(os.path.exists(path) for path in manifest_json_candidates):
            self.add_warning("Falta manifest.json")

        if not os.path.exists(engine_config):
            self.add_warning("Falta engine_config.json")

    def validate_json_files(self):
        for root, _, files in os.walk(self.game.project_path):
            for filename in files:
                if not filename.endswith(".json"):
                    continue

                path = os.path.join(root, filename)

                try:
                    with open(path, "r", encoding="utf-8") as file:
                        json.load(file)

                except Exception as error:
                    self.add_error(f"JSON inválido: {path} | {error}")

    def validate_scripts(self):
        folders = [
            self.paths()["scripts"],
            self.paths()["components"],
            self.paths()["systems"],
        ]

        for folder in folders:
            if not os.path.exists(folder):
                continue

            for root, _, files in os.walk(folder):
                for filename in files:
                    if not filename.endswith(".py"):
                        continue

                    path = os.path.join(root, filename)

                    try:
                        with open(path, "r", encoding="utf-8") as file:
                            code = file.read()

                        ast.parse(code, path)

                    except Exception as error:
                        self.add_error(f"Script con error: {path} | {error}")

    def validate_scenes(self):
        scenes = self.paths()["scenes"]

        if not os.path.exists(scenes):
            return

        found = False

        for filename in os.listdir(scenes):
            if filename.endswith(".scene"):
                found = True

        if not found:
            self.add_warning("No hay escenas .scene en el proyecto")

    def validate_prefabs(self):
        prefabs = self.paths()["prefabs"]

        if not os.path.exists(prefabs):
            return

        for root, _, files in os.walk(prefabs):
            for filename in files:
                if not filename.endswith(".prefab"):
                    continue

                path = os.path.join(root, filename)

                try:
                    with open(path, "r", encoding="utf-8") as file:
                        data = json.load(file)

                    if "entity" not in data:
                        self.add_warning(f"Prefab sin entity: {filename}")

                    if data.get("variant") and not data.get("base_prefab"):
                        self.add_warning(f"Prefab variant sin base_prefab: {filename}")

                except Exception as error:
                    self.add_error(f"Prefab inválido: {filename} | {error}")

    def validate_asset_references(self):
        assets = getattr(getattr(self.game, "asset_database", None), "assets", [])
        existing = {
            os.path.normpath(asset.get("relative_path", ""))
            for asset in assets
            if asset.get("relative_path")
        }

        for unit in getattr(self.game, "units", []):
            sprite = getattr(unit, "sprite_name", None)

            if sprite:
                candidates = [
                    path for path in existing
                    if os.path.splitext(os.path.basename(path))[0] == sprite
                ]

                if not candidates:
                    self.add_warning(f"Sprite referenciado no existe en asset database: {sprite}")

    def validate_build_settings(self):
        build_settings = getattr(self.game, "build_settings", None)

        if not build_settings:
            return

        game_name = build_settings.get("game_name", "MiniForgeGame")

        if not str(game_name).strip():
            self.add_error("Build Settings: game_name vacío")

        start_scene = build_settings.get("start_scene", None)

        if start_scene:
            scene_path = os.path.join(self.paths()["scenes"], start_scene)

            if not os.path.exists(scene_path):
                self.add_warning(f"Build Settings: start_scene no existe en scenes/: {start_scene}")

    def print_results(self):
        console = self.game.console

        console.log("Project validation started", "ENGINE")

        if not self.errors and not self.warnings:
            console.log("Proyecto validado correctamente ✅", "ENGINE")
            return

        for warning in self.warnings:
            console.log(warning, "WARNING")

        for error in self.errors:
            console.log(error, "ERROR")

        console.log(
            f"Validation finished: {len(self.errors)} error(es), {len(self.warnings)} warning(s)",
            "ENGINE"
        )
