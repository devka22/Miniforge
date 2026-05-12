import json
import os
import time


class BuildReport:
    def __init__(self, game):
        self.game = game
        self.last_report = None

    def generate(self, build_path):
        build_path = os.path.abspath(build_path)
        report = {
            "version": "0.6.0",
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "build_path": build_path,
            "project_path": os.path.abspath(getattr(self.game, "project_path", os.getcwd())),
            "summary": self.summary(build_path),
            "assets": self.asset_stats(build_path),
            "scenes": self.scene_stats(build_path),
            "scripts": self.script_stats(build_path),
            "systems": self.system_stats(),
            "warnings": self.collect_warnings(build_path),
            "validation": self.validation_snapshot(),
            "errors": getattr(getattr(self.game, "error_handler", None), "summary", lambda: {})(),
        }
        self.last_report = report
        self.write(report, os.path.join(build_path, "build_report.json"))

        project_copy = self.game.project_join("project", "last_build_report.json")
        self.write(report, project_copy)
        self.log(f"Build report generado: {os.path.join(build_path, 'build_report.json')}")
        return report

    def summary(self, build_path):
        total_files = 0
        total_size = 0

        for root, _, files in os.walk(build_path):
            for filename in files:
                path = os.path.join(root, filename)
                total_files += 1
                total_size += os.path.getsize(path)

        return {
            "files": total_files,
            "size_bytes": total_size,
            "size_mb": round(total_size / (1024 * 1024), 3),
            "entities": len(getattr(self.game, "units", [])),
            "selected_build_profile": getattr(getattr(self.game, "build_profiles", None), "active", None),
        }

    def asset_stats(self, build_path):
        by_type = {}
        entries = []

        for asset in getattr(getattr(self.game, "asset_database", None), "assets", []):
            asset_type = asset.get("type", "Unknown")
            relative = asset.get("relative_path")
            size = 0
            included = False

            if relative:
                candidate = os.path.join(build_path, relative)
                included = os.path.exists(candidate)
                if included:
                    size = os.path.getsize(candidate)

            by_type.setdefault(asset_type, {"count": 0, "included": 0, "size_bytes": 0})
            by_type[asset_type]["count"] += 1
            by_type[asset_type]["included"] += 1 if included else 0
            by_type[asset_type]["size_bytes"] += size

            entries.append({
                "name": asset.get("name"),
                "type": asset_type,
                "relative_path": relative,
                "included": included,
                "size_bytes": size,
                "import_settings": asset.get("import_settings", {}),
                "dependencies": asset.get("dependencies", []),
            })

        return {
            "by_type": by_type,
            "entries": entries[:500],
        }

    def scene_stats(self, build_path):
        scenes_root = os.path.join(build_path, "saves", "scenes")
        scenes = []

        if os.path.exists(scenes_root):
            for filename in sorted(os.listdir(scenes_root)):
                if not filename.endswith(".scene"):
                    continue

                path = os.path.join(scenes_root, filename)
                scenes.append({
                    "name": filename,
                    "size_bytes": os.path.getsize(path),
                })

        return {
            "count": len(scenes),
            "start_scene": self.game.build_settings.get("start_scene", "main.scene"),
            "entries": scenes,
        }

    def script_stats(self, build_path):
        roots = [
            os.path.join(build_path, "scripts"),
            os.path.join(build_path, "components"),
            os.path.join(build_path, "systems"),
        ]
        entries = []

        for folder in roots:
            if not os.path.exists(folder):
                continue

            for root, _, files in os.walk(folder):
                for filename in files:
                    if filename.endswith(".py"):
                        path = os.path.join(root, filename)
                        entries.append({
                            "relative_path": os.path.relpath(path, build_path),
                            "size_bytes": os.path.getsize(path),
                        })

        return {
            "count": len(entries),
            "entries": entries[:300],
        }

    def system_stats(self):
        physics = getattr(self.game, "systems", [])
        system_names = [system.__class__.__name__ for system in physics]
        project_systems = [
            system.__class__.__name__
            for system in getattr(self.game, "project_runtime_systems", [])
        ]

        return {
            "engine_systems": system_names,
            "project_systems": project_systems,
            "tilemap": getattr(getattr(self.game, "tilemap_layers", None), "stats", lambda: {})(),
            "audio": getattr(getattr(self.game, "audio_mixer", None), "stats", {}),
            "animation": getattr(getattr(self.game, "animation_graphs", None), "names", lambda: [])(),
            "ui": getattr(getattr(self.game, "ui_canvas", None), "stats", {}),
            "visual_scripting": getattr(getattr(self.game, "visual_script_runtime", None), "stats", {}),
            "profiler": getattr(getattr(self.game, "profiler", None), "serialize_snapshot", lambda: {})(),
        }

    def collect_warnings(self, build_path):
        warnings = []

        if not os.path.exists(os.path.join(build_path, "run_game.py")):
            warnings.append("Falta run_game.py")

        if not self.scene_stats(build_path)["entries"]:
            warnings.append("No se encontraron escenas exportadas")

        missing_assets = [
            asset.get("relative_path")
            for asset in getattr(getattr(self.game, "asset_database", None), "assets", [])
            if asset.get("relative_path")
            and not os.path.exists(os.path.join(build_path, asset.get("relative_path")))
            and asset.get("type") in ("Sprite", "Audio", "Data", "Prefab")
        ]

        if missing_assets:
            warnings.append(f"Assets no incluidos por perfil: {len(missing_assets)}")

        return warnings

    def validation_snapshot(self):
        scene_validator = getattr(self.game, "scene_validator", None)
        project_validator = getattr(self.game, "project_validator", None)

        return {
            "scene": {
                "warnings": list(getattr(scene_validator, "warnings", [])),
                "errors": list(getattr(scene_validator, "errors", [])),
            },
            "project": {
                "warnings": list(getattr(project_validator, "warnings", [])),
                "errors": list(getattr(project_validator, "errors", [])),
            },
        }

    def write(self, report, path):
        os.makedirs(os.path.dirname(path), exist_ok=True)

        with open(path, "w", encoding="utf-8") as file:
            json.dump(report, file, indent=4)

    def log(self, message, level="BUILD"):
        if hasattr(self.game, "console"):
            self.game.console.log(message, level)
