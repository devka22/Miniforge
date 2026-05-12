import json
import os
import shutil
import sys
import time


class RuntimeExporter:
    """
    Exporta un proyecto como carpeta autocontenida.
    El build incluye una copia mínima del runtime del motor y los archivos del juego.
    """

    ENGINE_DIRS = [
        "core",
        "engine",
        "entities",
        "input",
        "map",
        "pathfinding",
        "render",
        "runtime",
        "systems",
    ]

    PROJECT_DIRS = [
        "scripts",
        "components",
        "systems",
        "saves",
        "project",
        "settings",
    ]

    IGNORE_NAMES = {
        "__pycache__",
        ".DS_Store",
        ".git",
        ".venv",
        "builds",
        "logs",
    }

    def __init__(self, game):
        self.game = game

    def log(self, message, level="BUILD"):
        if hasattr(self.game, "console"):
            self.game.console.log(message, level)

    def export(self, output_path=None):
        project_path = os.path.abspath(getattr(self.game, "project_path", os.getcwd()))
        engine_root = self.find_engine_root()

        if output_path is None:
            export_folder = self.game.build_settings.get("export_folder", "builds")
            game_name = self.game.build_settings.get("game_name", "MiniForgeGame")
            output_path = os.path.join(project_path, export_folder, self.safe_name(game_name))

        output_path = os.path.abspath(output_path)

        if os.path.exists(output_path):
            shutil.rmtree(output_path)

        os.makedirs(output_path, exist_ok=True)

        self.copy_engine_runtime(engine_root, output_path)
        self.copy_project_files(project_path, output_path)
        self.copy_project_assets(project_path, output_path)
        self.write_runtime_launcher(output_path)
        self.write_build_info(output_path, project_path)

        self.log(f"Build exportado: {output_path}")
        return output_path

    def find_engine_root(self):
        current = os.path.abspath(os.path.dirname(__file__))
        root = os.path.dirname(current)

        if os.path.exists(os.path.join(root, "main.py")):
            return root

        return os.path.abspath(os.getcwd())

    def ignore(self, _, names):
        return [name for name in names if name in self.IGNORE_NAMES]

    def copy_engine_runtime(self, engine_root, output_path):
        for folder in self.ENGINE_DIRS:
            source = os.path.join(engine_root, folder)

            if not os.path.exists(source):
                continue

            shutil.copytree(
                source,
                os.path.join(output_path, folder),
                ignore=self.ignore,
                dirs_exist_ok=True,
            )

        main_py = os.path.join(engine_root, "main.py")

        if os.path.exists(main_py):
            shutil.copy2(main_py, os.path.join(output_path, "main.py"))

    def copy_project_files(self, project_path, output_path):
        for folder in self.PROJECT_DIRS:
            source = os.path.join(project_path, folder)

            if not os.path.exists(source):
                continue

            shutil.copytree(
                source,
                os.path.join(output_path, folder),
                ignore=self.ignore,
                dirs_exist_ok=True,
            )

    def copy_project_assets(self, project_path, output_path):
        source_assets = os.path.join(project_path, "assets")
        target_assets = os.path.join(output_path, "assets")

        if not os.path.exists(source_assets):
            os.makedirs(target_assets, exist_ok=True)
            return

        if self.game.build_settings.get("include_all_assets", False):
            shutil.copytree(
                source_assets,
                target_assets,
                ignore=self.ignore,
                dirs_exist_ok=True,
            )
            return

        os.makedirs(target_assets, exist_ok=True)

        used = self.collect_used_asset_paths(project_path)

        for relative_path in used:
            source = os.path.join(project_path, relative_path)

            if not os.path.exists(source):
                continue

            target = os.path.join(output_path, relative_path)
            os.makedirs(os.path.dirname(target), exist_ok=True)
            shutil.copy2(source, target)

    def collect_used_asset_paths(self, project_path):
        used = set()

        for asset in getattr(self.game.asset_database, "assets", []):
            asset_type = asset.get("type")
            relative_path = asset.get("relative_path")
            import_settings = asset.get("import_settings", {})

            if asset_type in ["Script", "Component", "System", "Scene"]:
                continue

            if import_settings.get("include_in_build") is False:
                continue

            if not relative_path:
                continue

            if asset_type in ["Data", "Settings", "Plugin"]:
                used.add(relative_path)

        sprite_names = set()
        prefab_paths = set()

        for unit in getattr(self.game, "units", []):
            sprite_name = getattr(unit, "sprite_name", None)

            if sprite_name:
                sprite_names.add(sprite_name)

            prefab_source = getattr(unit, "prefab_source", None)

            if prefab_source:
                prefab_paths.add(prefab_source)

        for asset in getattr(self.game.asset_database, "assets", []):
            asset_type = asset.get("type")
            name = asset.get("name")
            relative_path = asset.get("relative_path")
            import_settings = asset.get("import_settings", {})

            if not relative_path:
                continue

            if import_settings.get("include_in_build") is False:
                continue

            if asset_type == "Sprite" and name in sprite_names:
                used.add(relative_path)

            if asset_type == "Prefab":
                asset_path = os.path.normpath(asset.get("path", ""))

                for prefab_path in prefab_paths:
                    if os.path.normpath(prefab_path) == asset_path:
                        used.add(relative_path)

        if not used:
            fallback_dirs = [
                os.path.join(project_path, "assets", "data"),
                os.path.join(project_path, "assets", "prefabs"),
            ]

            for folder in fallback_dirs:
                if not os.path.exists(folder):
                    continue

                for root, _, files in os.walk(folder):
                    for filename in files:
                        path = os.path.join(root, filename)
                        used.add(os.path.relpath(path, project_path))

        return used

    def write_runtime_launcher(self, output_path):
        launcher = os.path.join(output_path, "run_game.py")

        with open(launcher, "w", encoding="utf-8") as file:
            file.write(
                "import os\n"
                "import sys\n\n"
                "BASE_DIR = os.path.dirname(os.path.abspath(__file__))\n"
                "os.chdir(BASE_DIR)\n\n"
                "if BASE_DIR not in sys.path:\n"
                "    sys.path.insert(0, BASE_DIR)\n\n"
                "os.environ['MINIFORGE_RUNTIME'] = '1'\n\n"
                "from runtime.game_runner import run\n\n"
                "run(BASE_DIR)\n"
            )

    def write_build_info(self, output_path, project_path):
        data = {
            "built_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "python": sys.version.split()[0],
            "source_project": project_path,
            "start_scene": self.game.build_settings.get("start_scene", "main.scene"),
            "runtime_entry": "run_game.py",
        }

        with open(os.path.join(output_path, "build_info.json"), "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

    def safe_name(self, name):
        clean = []

        for char in str(name or "MiniForgeGame"):
            if char.isalnum() or char in ("_", "-"):
                clean.append(char)
            elif char.isspace():
                clean.append("_")

        return "".join(clean) or "MiniForgeGame"
