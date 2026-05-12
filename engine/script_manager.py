import os
import sys
import importlib.util


class ScriptManager:
    """
    ScriptManager 0.6.0 limpio.

    Ya no usa la carpeta global antigua scripts/.

    Carga scripts desde:
    - projects/NOMBRE/scripts
    - projects/NOMBRE/components
    - projects/NOMBRE/systems
    """

    def __init__(self):
        self.scripts = {}
        self.script_paths = {}
        self.script_mtimes = {}

        self.project_path = None
        self.search_folders = []

    def scan_scripts(self, project_scripts_path=None, project_path=None):
        self.scripts.clear()
        self.script_paths.clear()

        if project_path:
            self.project_path = project_path

        self.search_folders = []

        if self.project_path:
            self.search_folders.extend(
                [
                    os.path.join(self.project_path, "scripts"),
                    os.path.join(self.project_path, "components"),
                    os.path.join(self.project_path, "systems"),
                ]
            )

        elif project_scripts_path:
            project_root = os.path.dirname(project_scripts_path)

            self.search_folders.extend(
                [
                    project_scripts_path,
                    os.path.join(project_root, "components"),
                    os.path.join(project_root, "systems"),
                ]
            )

        seen = set()

        for folder in self.search_folders:
            folder = os.path.normpath(folder)

            if folder in seen:
                continue

            seen.add(folder)
            self.scan_folder(folder)

    def scan_folder(self, folder):
        if not os.path.exists(folder):
            os.makedirs(folder, exist_ok=True)
            return

        if folder not in sys.path:
            sys.path.append(folder)

        for root, _, files in os.walk(folder):
            if "__pycache__" in root:
                continue

            for filename in files:
                if filename.startswith("_"):
                    continue

                if not filename.endswith(".py"):
                    continue

                path = os.path.join(root, filename)
                script_name = os.path.splitext(filename)[0]

                self.register_script(script_name, path)

    def register_script(self, script_name, path):
        self.script_paths[script_name] = path
        self.script_mtimes[script_name] = os.path.getmtime(path)

        try:
            module = self.load_module_from_path(script_name, path)
            cls = self.find_script_class(module, script_name)

            if cls:
                self.scripts[script_name] = cls

        except Exception as error:
            print(f"[SCRIPT ERROR] No se pudo cargar {script_name}: {error}")

    def load_module_from_path(self, script_name, path):
        module_name = f"project_script_{script_name}_{abs(hash(path))}"

        spec = importlib.util.spec_from_file_location(module_name, path)

        if not spec or not spec.loader:
            raise ImportError(f"No se pudo crear spec para {path}")

        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        return module

    def find_script_class(self, module, script_name):
        preferred_names = [
            script_name,
            self.to_class_name(script_name),
        ]

        for name in preferred_names:
            if hasattr(module, name):
                candidate = getattr(module, name)

                if isinstance(candidate, type):
                    return candidate

        for value in module.__dict__.values():
            if isinstance(value, type):
                if hasattr(value, "update") or hasattr(value, "start"):
                    return value

        return None

    def to_class_name(self, name):
        clean = str(name).replace("-", "_").replace(" ", "_")
        return "".join(part.capitalize() for part in clean.split("_") if part)

    def create(self, script_name):
        cls = self.scripts.get(script_name)

        if not cls:
            self.scan_scripts(project_path=self.project_path)
            cls = self.scripts.get(script_name)

        if not cls:
            return None

        try:
            return cls()

        except TypeError:
            try:
                return cls(None)
            except Exception:
                return None

        except Exception:
            return None

    def exists(self, script_name):
        return script_name in self.scripts

    def names(self):
        return sorted(self.scripts.keys())

    def get_script_names(self):
        return self.names()

    def get_path(self, script_name):
        return self.script_paths.get(script_name)

    def has_changes(self):
        for script_name, path in self.script_paths.items():
            if not os.path.exists(path):
                return True

            if os.path.getmtime(path) != self.script_mtimes.get(script_name):
                return True

        return False

    def reload_if_changed(self):
        if not self.has_changes():
            return False

        self.scan_scripts(project_path=self.project_path)
        return True
