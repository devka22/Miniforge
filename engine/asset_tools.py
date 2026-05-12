import os
import json
import shutil
from tkinter import Tk, filedialog

from engine.version import ENGINE_VERSION


class AssetTools:
    """
    AssetTools 0.6.0
    Ahora trabaja con project_path real.

    Objetivo:
    - separar archivos del juego del código fuente del motor
    - crear carpetas por proyecto
    - importar assets al proyecto actual
    - crear scripts/componentes/sistemas/json/txt/escenas/prefabs desde el editor
    """

    DEFAULT_PROJECT_NAME = "DefaultProject"

    @staticmethod
    def normalize_path(path):
        return os.path.normpath(path)

    @staticmethod
    def ensure_folder(path):
        os.makedirs(path, exist_ok=True)
        return path

    @staticmethod
    def default_project_path():
        return os.path.join("projects", AssetTools.DEFAULT_PROJECT_NAME)

    @staticmethod
    def get_project_paths(project_path=None):
        project_path = project_path or AssetTools.default_project_path()

        return {
            "project": project_path,
            "assets": os.path.join(project_path, "assets"),
            "sprites": os.path.join(project_path, "assets", "sprites"),
            "audio": os.path.join(project_path, "assets", "audio"),
            "data": os.path.join(project_path, "assets", "data"),
            "prefabs": os.path.join(project_path, "assets", "prefabs"),
            "scripts": os.path.join(project_path, "scripts"),
            "components": os.path.join(project_path, "components"),
            "systems": os.path.join(project_path, "systems"),
            "scenes": os.path.join(project_path, "saves", "scenes"),
            "root_scenes": os.path.join(project_path, "scenes"),
            "settings": os.path.join(project_path, "settings"),
            "logs": os.path.join(project_path, "logs"),
            "templates": os.path.join(project_path, "templates"),
            "plugins": os.path.join(project_path, "plugins"),
            "builds": os.path.join(project_path, "builds"),
        }

    @staticmethod
    def ensure_project_folders(project_path=None):
        paths = AssetTools.get_project_paths(project_path)

        for path in paths.values():
            AssetTools.ensure_folder(path)

        AssetTools.ensure_project_files(project_path)
        return paths

    @staticmethod
    def ensure_project_files(project_path=None):
        project_path = project_path or AssetTools.default_project_path()
        paths = AssetTools.get_project_paths(project_path)

        project_json = os.path.join(project_path, "project.json")
        engine_config_json = os.path.join(project_path, "engine_config.json")
        manifest_json = os.path.join(project_path, "manifest.json")
        runtime_json = os.path.join(paths["settings"], "runtime_config.json")
        build_json = os.path.join(paths["settings"], "build_settings.json")
        tags_json = os.path.join(paths["settings"], "tags.json")
        layers_json = os.path.join(paths["settings"], "layers.json")
        readme = os.path.join(project_path, "README.md")

        if not os.path.exists(project_json):
            AssetTools.write_json(
                project_json,
                {
                    "project_name": os.path.basename(project_path),
                    "engine_version": ENGINE_VERSION,
                    "start_scene": "main.scene",
                    "author": "",
                    "license": "GPL-3.0",
                    "description": "MiniForge 0.6.0 Beta project",
                }
            )

        if not os.path.exists(engine_config_json):
            AssetTools.write_json(
                engine_config_json,
                {
                    "engine_name": "MiniForge",
                    "engine_alt_name": "Mini Forte",
                    "engine_version": ENGINE_VERSION,
                    "project_name": os.path.basename(project_path),
                    "start_scene": "main.scene",
                    "autosave": True,
                    "autosave_interval_seconds": 60,
                    "safe_mode": True,
                    "logs": {
                        "engine": "logs/engine.log",
                        "error": "logs/error.log",
                    },
                }
            )

        if not os.path.exists(manifest_json):
            AssetTools.write_json(
                manifest_json,
                {
                    "engine_version": ENGINE_VERSION,
                    "assets": {},
                    "scenes": [],
                    "scripts": [],
                    "components": [],
                    "systems": [],
                }
            )

        if not os.path.exists(runtime_json):
            AssetTools.write_json(
                runtime_json,
                {
                    "game_name": os.path.basename(project_path),
                    "start_scene": "main.scene",
                    "window_width": 1100,
                    "window_height": 740,
                    "fullscreen": False,
                    "target_fps": 60,
                    "debug": True,
                }
            )

        if not os.path.exists(build_json):
            AssetTools.write_json(
                build_json,
                {
                    "game_name": os.path.basename(project_path),
                    "start_scene": "main.scene",
                    "target_fps": 60,
                    "window_width": 1100,
                    "window_height": 740,
                    "fullscreen": False,
                    "debug_mode": True,
                    "export_folder": "builds",
                }
            )

        if not os.path.exists(tags_json):
            AssetTools.write_json(
                tags_json,
                {
                    "items": [
                        "Untagged",
                        "Player",
                        "Enemy",
                        "Resource",
                        "Building",
                        "Projectile",
                        "Neutral",
                    ]
                }
            )

        if not os.path.exists(layers_json):
            AssetTools.write_json(
                layers_json,
                {
                    "items": [
                        "Default",
                        "Ground",
                        "Units",
                        "Buildings",
                        "UI",
                        "Effects",
                        "IgnoreSelection",
                        "EditorOnly",
                    ]
                }
            )

        if not os.path.exists(readme):
            with open(readme, "w", encoding="utf-8") as file:
                file.write(
                    f"# {os.path.basename(project_path)}\n\n"
                    f"Proyecto creado con MiniForge {ENGINE_VERSION}.\n\n"
                    "## Carpetas\n\n"
                    "- assets/sprites\n"
                    "- assets/audio\n"
                    "- assets/data\n"
                    "- assets/prefabs\n"
                    "- scripts\n"
                    "- components\n"
                    "- systems\n"
                    "- scenes\n"
                    "- settings\n"
                )

    @staticmethod
    def write_json(path, data):
        folder = os.path.dirname(path)

        if folder:
            os.makedirs(folder, exist_ok=True)

        with open(path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

    @staticmethod
    def safe_name(name, fallback="NewFile"):
        name = str(name or "").strip()

        if not name:
            name = fallback

        invalid = ['/', '\\', ':', '*', '?', '"', '<', '>', '|']

        for char in invalid:
            name = name.replace(char, "_")

        return name

    @staticmethod
    def unique_path(folder, filename):
        os.makedirs(folder, exist_ok=True)

        name, ext = os.path.splitext(filename)
        path = os.path.join(folder, filename)
        index = 1

        while os.path.exists(path):
            path = os.path.join(folder, f"{name}_{index}{ext}")
            index += 1

        return path

    @staticmethod
    def create_file(path, content="", overwrite=False):
        folder = os.path.dirname(path)

        if folder:
            os.makedirs(folder, exist_ok=True)

        if os.path.exists(path) and not overwrite:
            folder = os.path.dirname(path)
            filename = os.path.basename(path)
            path = AssetTools.unique_path(folder, filename)

        with open(path, "w", encoding="utf-8") as file:
            file.write(content)

        return path

    @staticmethod
    def create_json_file(path, data=None, overwrite=False):
        data = data if data is not None else {}

        folder = os.path.dirname(path)

        if folder:
            os.makedirs(folder, exist_ok=True)

        if os.path.exists(path) and not overwrite:
            folder = os.path.dirname(path)
            filename = os.path.basename(path)
            path = AssetTools.unique_path(folder, filename)

        with open(path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        return path

    @staticmethod
    def template_basic_script(class_name="NewScript"):
        return f'''class {class_name}:
    """
    Script creado desde MiniForge 0.6.0 Beta.
    El motor acepta start() / start(entity) y update(dt) / update(entity, dt).
    """

    def __init__(self):
        self.script_name = "{class_name}"
        self.enabled = True
        self.started = False

    def start(self):
        pass

    def update(self, dt):
        pass

    def on_selected(self, entity):
        pass

    def on_deselected(self, entity):
        pass

    def serialize(self):
        return {{
            "script": self.script_name,
            "enabled": self.enabled,
        }}

    def deserialize(self, data):
        self.enabled = data.get("enabled", True)
'''

    @staticmethod
    def template_component(class_name="NewComponent"):
        return f'''class {class_name}:
    """
    Componente personalizado del proyecto.
    Todavía es externo al engine/component.py.
    """

    def __init__(self):
        self.component_type = "{class_name}"
        self.enabled = True

    def start(self, entity):
        pass

    def update(self, entity, dt):
        pass

    def serialize(self):
        return {{
            "component_type": self.component_type,
            "enabled": self.enabled,
        }}

    def deserialize(self, data):
        self.enabled = data.get("enabled", True)
'''

    @staticmethod
    def template_system(class_name="NewSystem"):
        return f'''class {class_name}:
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
'''

    @staticmethod
    def template_scene(scene_name="main"):
        return {
            "version": "0.6.0",
            "engine_version": ENGINE_VERSION,
            "scene_name": scene_name,
            "mode": "EDITOR",
            "active_tool": "Select",
            "tile_brush": 0,
            "brush_size": 1,
            "camera": {
                "x": 0,
                "y": 0,
                "zoom": 1.0,
            },
            "control_groups": {},
            "grid": None,
            "tiles": [],
            "settings": {},
            "entities": [],
            "editor_view_settings": {},
        }

    @staticmethod
    def template_prefab(prefab_name="NewPrefab"):
        return {
            "version": "0.6.0",
            "engine_version": ENGINE_VERSION,
            "prefab_name": prefab_name,
            "entity": {
                "type": "Unit",
                "name": prefab_name,
                "enabled": True,
                "visible": True,
                "locked": False,
                "x": 0,
                "y": 0,
                "speed": 3.5,
                "radius": 0.45,
                "sprite_name": None,
                "tag": "Untagged",
                "layer": "Default",
                "state": "IDLE",
                "command": "IDLE",
                "prefab_source": None,
                "is_prefab_instance": False,
                "components": [],
                "scripts": [],
            },
        }

    @staticmethod
    def create_script(project_path, name="NewScript"):
        paths = AssetTools.get_project_paths(project_path)

        name = AssetTools.safe_name(name, "NewScript")

        if not name.endswith(".py"):
            filename = f"{name}.py"
            class_name = name
        else:
            filename = name
            class_name = os.path.splitext(name)[0]

        class_name = "".join(part.capitalize() for part in class_name.replace("-", "_").split("_"))

        if not class_name:
            class_name = "NewScript"

        path = AssetTools.unique_path(paths["scripts"], filename)
        return AssetTools.create_file(path, AssetTools.template_basic_script(class_name), overwrite=True)

    @staticmethod
    def create_component(project_path, name="NewComponent"):
        paths = AssetTools.get_project_paths(project_path)

        name = AssetTools.safe_name(name, "NewComponent")

        if not name.endswith(".py"):
            filename = f"{name}.py"
            class_name = name
        else:
            filename = name
            class_name = os.path.splitext(name)[0]

        class_name = "".join(part.capitalize() for part in class_name.replace("-", "_").split("_"))

        if not class_name:
            class_name = "NewComponent"

        path = AssetTools.unique_path(paths["components"], filename)
        return AssetTools.create_file(path, AssetTools.template_component(class_name), overwrite=True)

    @staticmethod
    def create_system(project_path, name="NewSystem"):
        paths = AssetTools.get_project_paths(project_path)

        name = AssetTools.safe_name(name, "NewSystem")

        if not name.endswith(".py"):
            filename = f"{name}.py"
            class_name = name
        else:
            filename = name
            class_name = os.path.splitext(name)[0]

        class_name = "".join(part.capitalize() for part in class_name.replace("-", "_").split("_"))

        if not class_name:
            class_name = "NewSystem"

        path = AssetTools.unique_path(paths["systems"], filename)
        return AssetTools.create_file(path, AssetTools.template_system(class_name), overwrite=True)

    @staticmethod
    def create_json(project_path, folder=None, name="NewData"):
        paths = AssetTools.get_project_paths(project_path)
        target_folder = folder or paths["data"]

        name = AssetTools.safe_name(name, "NewData")

        if not name.endswith(".json"):
            name += ".json"

        path = AssetTools.unique_path(target_folder, name)
        return AssetTools.create_json_file(path, {"created_by": "MiniForge", "version": "0.6.0"}, overwrite=True)

    @staticmethod
    def create_txt(project_path, folder=None, name="NewText"):
        paths = AssetTools.get_project_paths(project_path)
        target_folder = folder or paths["data"]

        name = AssetTools.safe_name(name, "NewText")

        if not name.endswith(".txt"):
            name += ".txt"

        path = AssetTools.unique_path(target_folder, name)
        return AssetTools.create_file(path, "New text file\n", overwrite=True)

    @staticmethod
    def create_scene(project_path, name="main"):
        paths = AssetTools.get_project_paths(project_path)

        name = AssetTools.safe_name(name, "main")

        if not name.endswith(".scene"):
            filename = f"{name}.scene"
            scene_name = name
        else:
            filename = name
            scene_name = os.path.splitext(name)[0]

        path = AssetTools.unique_path(paths["scenes"], filename)
        return AssetTools.create_json_file(path, AssetTools.template_scene(scene_name), overwrite=True)

    @staticmethod
    def create_prefab(project_path, name="NewPrefab"):
        paths = AssetTools.get_project_paths(project_path)

        name = AssetTools.safe_name(name, "NewPrefab")

        if not name.endswith(".prefab"):
            filename = f"{name}.prefab"
            prefab_name = name
        else:
            filename = name
            prefab_name = os.path.splitext(name)[0]

        path = AssetTools.unique_path(paths["prefabs"], filename)
        return AssetTools.create_json_file(path, AssetTools.template_prefab(prefab_name), overwrite=True)

    @staticmethod
    def create_special_folder(project_path, folder_type):
        paths = AssetTools.get_project_paths(project_path)

        mapping = {
            "sprites": paths["sprites"],
            "audio": paths["audio"],
            "data": paths["data"],
            "prefabs": paths["prefabs"],
            "scripts": paths["scripts"],
            "components": paths["components"],
            "systems": paths["systems"],
            "scenes": paths["scenes"],
            "settings": paths["settings"],
            "plugins": paths["plugins"],
        }

        target = mapping.get(folder_type.lower())

        if not target:
            target = os.path.join(paths["project"], folder_type)

        os.makedirs(target, exist_ok=True)
        return target

    @staticmethod
    def pick_file(filetypes):
        try:
            root = Tk()
            root.withdraw()
            root.attributes("-topmost", True)

            path = filedialog.askopenfilename(filetypes=filetypes)

            root.destroy()
            return path

        except Exception:
            return None

    @staticmethod
    def safe_copy_to_folder(source_path, target_folder, console=None, asset_type="Asset"):
        if not source_path:
            return False

        if not os.path.exists(source_path):
            if console:
                console.log(f"No existe el archivo: {source_path}", "ERROR")
            return False

        os.makedirs(target_folder, exist_ok=True)

        filename = os.path.basename(source_path)
        target = AssetTools.unique_path(target_folder, filename)

        try:
            shutil.copy2(source_path, target)

            if console:
                console.log(f"{asset_type} importado: {target}", "ASSET")

            return True

        except Exception as error:
            if console:
                console.log(f"No se pudo importar {asset_type}: {error}", "ERROR")

            return False

    @staticmethod
    def import_sprite(console=None, project_path=None):
        paths = AssetTools.get_project_paths(project_path)

        path = AssetTools.pick_file(
            [
                ("Images", "*.png *.jpg *.jpeg *.bmp *.gif *.webp"),
                ("All files", "*.*"),
            ]
        )

        return AssetTools.safe_copy_to_folder(path, paths["sprites"], console, "Sprite")

    @staticmethod
    def import_audio(console=None, project_path=None):
        paths = AssetTools.get_project_paths(project_path)

        path = AssetTools.pick_file(
            [
                ("Audio", "*.wav *.mp3 *.ogg"),
                ("All files", "*.*"),
            ]
        )

        return AssetTools.safe_copy_to_folder(path, paths["audio"], console, "Audio")

    @staticmethod
    def import_data(console=None, project_path=None):
        paths = AssetTools.get_project_paths(project_path)

        path = AssetTools.pick_file(
            [
                ("Data", "*.json *.txt *.csv"),
                ("All files", "*.*"),
            ]
        )

        return AssetTools.safe_copy_to_folder(path, paths["data"], console, "Data")
