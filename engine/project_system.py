import os
import json
import datetime

from engine.asset_tools import AssetTools
from engine.version import ENGINE_VERSION


class ProjectSystem:
    """
    Sistema real de proyectos.
    Cada juego vive en su propia carpeta dentro de /projects.
    """

    ROOT_FOLDER = "projects"
    RECENT_FILE = "project/recent_projects.json"

    def __init__(self):
        os.makedirs(self.ROOT_FOLDER, exist_ok=True)
        os.makedirs("project", exist_ok=True)

        self.current_project_path = None
        self.current_project_name = None

    def create_project(self, name):
        safe_name = self.safe_name(name)
        path = os.path.join(self.ROOT_FOLDER, safe_name)

        os.makedirs(path, exist_ok=True)

        folders = [
            "assets",
            "assets/sprites",
            "assets/audio",
            "assets/data",
            "assets/prefabs",
            "components",
            "scripts",
            "systems",
            "saves",
            "saves/scenes",
            "saves/autosave",
            "scenes",
            "settings",
            "logs",
            "project",
            "builds",
            "plugins",
        ]

        for folder in folders:
            os.makedirs(os.path.join(path, folder), exist_ok=True)

        init_file = os.path.join(path, "scripts", "__init__.py")

        if not os.path.exists(init_file):
            with open(init_file, "w", encoding="utf-8") as file:
                file.write("# scripts package\n")

        project_json = os.path.join(path, "project", "project.json")

        if not os.path.exists(project_json):
            data = {
                "project_name": safe_name,
                "engine_version": ENGINE_VERSION,
                "current_scene": "main.scene",
                "start_scene": "main.scene",
                "created_at": datetime.datetime.now().isoformat(),
                "created_with": "MiniForge",
            }

            with open(project_json, "w", encoding="utf-8") as file:
                json.dump(data, file, indent=4)

        main_scene = os.path.join(path, "saves", "scenes", "main.scene")

        if not os.path.exists(main_scene):
            with open(main_scene, "w", encoding="utf-8") as file:
                json.dump(
                    {
                        "version": "0.6.0",
                        "engine_version": ENGINE_VERSION,
                        "scene_name": "main.scene",
                        "mode": "EDITOR",
                        "active_tool": "Select",
                        "tile_brush": 0,
                        "camera": {
                            "x": 0,
                            "y": 0,
                            "zoom": 1.0
                        },
                        "control_groups": {},
                        "grid": None,
                        "entities": []
                    },
                    file,
                    indent=4
                )

        root_project_json = os.path.join(path, "project.json")

        if not os.path.exists(root_project_json):
            with open(root_project_json, "w", encoding="utf-8") as file:
                json.dump(
                    {
                        "project_name": safe_name,
                        "engine_version": ENGINE_VERSION,
                        "start_scene": "main.scene",
                    },
                    file,
                    indent=4
                )

        AssetTools.ensure_project_folders(path)
        self.set_current_project(path)
        return path

    def open_project(self, path):
        if not os.path.exists(path):
            return False

        AssetTools.ensure_project_folders(path)
        self.set_current_project(path)
        return True

    def set_current_project(self, path):
        self.current_project_path = os.path.abspath(path)
        self.current_project_name = os.path.basename(self.current_project_path)

        self.add_recent_project(self.current_project_path)

    def apply_project_as_working_directory(self):
        """
        Esto es clave:
        cambia el directorio activo al proyecto.
        Así assets/, scripts/, saves/, etc. son propios de cada proyecto.
        """
        if self.current_project_path:
            os.chdir(self.current_project_path)

    def get_recent_projects(self):
        if not os.path.exists(self.RECENT_FILE):
            return []

        try:
            with open(self.RECENT_FILE, "r", encoding="utf-8") as file:
                data = json.load(file)

            return data.get("recent_projects", [])

        except Exception:
            return []

    def add_recent_project(self, path):
        recent = self.get_recent_projects()

        if path in recent:
            recent.remove(path)

        recent.insert(0, path)
        recent = recent[:8]

        with open(self.RECENT_FILE, "w", encoding="utf-8") as file:
            json.dump({"recent_projects": recent}, file, indent=4)

    def get_default_project(self):
        recent = self.get_recent_projects()

        if recent and os.path.exists(recent[0]):
            return recent[0]

        return self.create_project("DefaultProject")

    def safe_name(self, name):
        clean = ""

        for char in name:
            if char.isalnum() or char in ["_", "-"]:
                clean += char

        if not clean:
            clean = "NewProject"

        return clean
