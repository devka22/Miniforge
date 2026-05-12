import os
import json
import time
import hashlib
import uuid


class AssetDatabase:
    """
    Base de datos de assets del motor.
    Escanea imágenes, audio, data, scripts, prefabs y escenas.
    """

    SUPPORTED_IMAGE = [".png", ".jpg", ".jpeg", ".bmp"]
    SUPPORTED_AUDIO = [".wav", ".ogg", ".mp3"]
    SUPPORTED_DATA = [".json", ".txt", ".csv"]
    SUPPORTED_SCRIPT = [".py"]
    SUPPORTED_PREFAB = [".prefab"]
    SUPPORTED_SCENE = [".scene"]

    def __init__(self, root="assets", project_root="."):
        self.root = os.path.normpath(root)
        self.project_root = os.path.normpath(project_root)
        self.assets = []
        self.metadata_file = os.path.join(
            self.project_root,
            "project",
            "asset_metadata.json"
        )
        self.asset_guids = {}
        self.import_settings = {}
        self.dependencies = {}

        self.ensure_folders()
        self.scan()

    def ensure_folders(self):
        os.makedirs(os.path.join(self.project_root, "project"), exist_ok=True)

        os.makedirs(self.root, exist_ok=True)
        os.makedirs(os.path.join(self.root, "sprites"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "audio"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "maps"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "data"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "prefabs"), exist_ok=True)

        os.makedirs(os.path.join(self.project_root, "scripts"), exist_ok=True)
        os.makedirs(os.path.join(self.project_root, "saves", "scenes"), exist_ok=True)
        os.makedirs(os.path.join(self.project_root, "saves", "autosave"), exist_ok=True)

        init_file = os.path.join(self.project_root, "scripts", "__init__.py")

        if not os.path.exists(init_file):
            with open(init_file, "w") as file:
                file.write("# scripts package\n")

    def get_type(self, path):
        ext = os.path.splitext(path)[1].lower()

        if ext in self.SUPPORTED_IMAGE:
            return "Sprite"

        if ext in self.SUPPORTED_AUDIO:
            return "Audio"

        if ext in self.SUPPORTED_DATA:
            return "Data"

        if ext in self.SUPPORTED_SCRIPT:
            return "Script"

        if ext in self.SUPPORTED_PREFAB:
            return "Prefab"

        if ext in self.SUPPORTED_SCENE:
            return "Scene"

        return "Unknown"

    def scan(self):
        self.assets = []
        self.load_metadata()

        search_roots = [
            self.root,
            os.path.join(self.project_root, "scripts"),
            os.path.join(self.project_root, "saves", "scenes"),
            os.path.join(self.project_root, "saves", "autosave"),
        ]

        for root in search_roots:
            if not os.path.exists(root):
                continue

            for current_root, _, files in os.walk(root):
                for filename in files:
                    if filename.startswith("."):
                        continue

                    full_path = os.path.join(current_root, filename)
                    asset_type = self.get_type(full_path)

                    if asset_type == "Unknown":
                        continue

                    relative_path = os.path.relpath(full_path, self.project_root)
                    fallback_id = hashlib.sha1(
                        relative_path.replace(os.sep, "/").encode("utf-8")
                    ).hexdigest()[:16]
                    asset_id = self.asset_guids.get(relative_path, fallback_id)

                    if relative_path not in self.asset_guids:
                        self.asset_guids[relative_path] = str(uuid.uuid4())
                        asset_id = self.asset_guids[relative_path]

                    asset = {
                        "id": asset_id,
                        "name": os.path.splitext(filename)[0],
                        "filename": filename,
                        "path": full_path,
                        "relative_path": relative_path,
                        "type": asset_type,
                        "extension": os.path.splitext(filename)[1].lower(),
                        "modified": os.path.getmtime(full_path),
                        "import_settings": self.import_settings.get(
                            relative_path,
                            self.default_import_settings(asset_type)
                        ),
                        "dependencies": [],
                    }

                    self.assets.append(asset)

        self.rebuild_dependency_graph()
        self.save_metadata()

    def default_import_settings(self, asset_type):
        if asset_type == "Sprite":
            return {
                "filter": "nearest",
                "pixels_per_unit": 32,
                "generate_collision": False,
                "compression": "lossless",
            }

        if asset_type == "Audio":
            return {
                "stream": False,
                "volume": 1.0,
                "bus": "SFX",
                "preload": True,
            }

        if asset_type == "Scene":
            return {
                "include_in_build": True,
                "addressable": True,
            }

        if asset_type == "Prefab":
            return {
                "include_in_build": True,
                "addressable": True,
            }

        return {
            "include_in_build": True,
        }

    def load_metadata(self):
        if not os.path.exists(self.metadata_file):
            return

        try:
            with open(self.metadata_file, "r", encoding="utf-8") as file:
                data = json.load(file)

            for asset in data.get("assets", []):
                relative_path = asset.get("relative_path")
                asset_id = asset.get("id")

                if relative_path and asset_id:
                    self.asset_guids[relative_path] = asset_id

                if relative_path and asset.get("import_settings"):
                    self.import_settings[relative_path] = asset.get("import_settings")

            self.dependencies = data.get("dependencies", {})

        except Exception:
            self.asset_guids = {}
            self.import_settings = {}
            self.dependencies = {}

    def save_metadata(self):
        os.makedirs(os.path.dirname(self.metadata_file), exist_ok=True)

        data = {
            "last_scan": time.time(),
            "assets": self.assets,
            "dependencies": self.dependencies,
        }

        with open(self.metadata_file, "w") as file:
            json.dump(data, file, indent=4)

    def get_assets_by_type(self, asset_type):
        return [
            asset for asset in self.assets
            if asset["type"] == asset_type
        ]

    def get_all(self):
        return self.assets

    def find_by_name(self, name):
        for asset in self.assets:
            if asset["name"] == name:
                return asset

        return None

    def find_by_id(self, asset_id):
        for asset in self.assets:
            if asset.get("id") == asset_id:
                return asset

        return None

    def find_by_relative_path(self, relative_path):
        normalized = os.path.normpath(relative_path)

        for asset in self.assets:
            if os.path.normpath(asset.get("relative_path", "")) == normalized:
                return asset

        return None

    def find_by_path(self, path):
        normalized = os.path.abspath(path)

        for asset in self.assets:
            if os.path.abspath(asset.get("path", "")) == normalized:
                return asset

        return None

    def get_import_settings(self, relative_path):
        asset = self.find_by_relative_path(relative_path)

        if asset:
            return dict(asset.get("import_settings", {}))

        return dict(self.import_settings.get(relative_path, {}))

    def set_import_setting(self, relative_path, key, value):
        relative_path = os.path.normpath(relative_path)
        settings = self.import_settings.setdefault(relative_path, {})
        settings[key] = value

        asset = self.find_by_relative_path(relative_path)

        if asset:
            asset["import_settings"] = settings

        self.save_metadata()
        return settings

    def rebuild_dependency_graph(self):
        dependency_map = {}
        known = {
            asset.get("name"): asset.get("relative_path")
            for asset in self.assets
            if asset.get("name") and asset.get("relative_path")
        }
        known.update(
            {
                asset.get("filename"): asset.get("relative_path")
                for asset in self.assets
                if asset.get("filename") and asset.get("relative_path")
            }
        )

        for asset in self.assets:
            relative_path = asset.get("relative_path")

            if not relative_path:
                continue

            dependencies = self.scan_asset_dependencies(asset, known)
            asset["dependencies"] = dependencies
            dependency_map[relative_path] = dependencies

        self.dependencies = dependency_map
        return dependency_map

    def scan_asset_dependencies(self, asset, known):
        asset_type = asset.get("type")

        if asset_type not in ("Scene", "Prefab", "Data", "Settings"):
            return []

        path = asset.get("path")

        if not path or not os.path.exists(path):
            return []

        try:
            with open(path, "r", encoding="utf-8") as file:
                text = file.read()
        except Exception:
            return []

        dependencies = set()

        for name, relative in known.items():
            if not name or relative == asset.get("relative_path"):
                continue

            if str(name) in text or str(relative) in text:
                dependencies.add(relative)

        return sorted(dependencies)

    def dependencies_for(self, relative_path):
        return list(self.dependencies.get(os.path.normpath(relative_path), []))

    def reverse_dependencies_for(self, relative_path):
        relative_path = os.path.normpath(relative_path)
        result = []

        for owner, dependencies in self.dependencies.items():
            if relative_path in [os.path.normpath(item) for item in dependencies]:
                result.append(owner)

        return sorted(result)
