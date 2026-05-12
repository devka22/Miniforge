import json
import os


class ManifestBuilder:
    """
    Manifest Builder 2.
    Genera project/manifest.json con dependencias del proyecto.
    """

    def __init__(self, game):
        self.game = game
        self.path = "project/manifest.json"

    def build_manifest(self):
        sprites = set()
        scripts = set()
        prefabs = set()
        scenes = []
        components = set()
        tags = set()
        layers = set()
        ui_elements = 0
        visual_graphs = 0
        animators = 0

        if hasattr(self.game, "scene_manager"):
            scenes = list(getattr(self.game.scene_manager, "scenes", []))

        for unit in self.game.units:
            sprite_name = getattr(unit, "sprite_name", None)

            if sprite_name:
                sprites.add(sprite_name)

            prefab_source = getattr(unit, "prefab_source", None)

            if prefab_source:
                prefabs.add(prefab_source)

            tags.add(getattr(unit, "tag", "Untagged"))
            layers.add(getattr(unit, "layer", "Default"))

            for component in getattr(unit, "components", []):
                component_type = getattr(component, "component_type", None)

                if component_type:
                    components.add(component_type)

                if component_type == "UIElement":
                    ui_elements += 1
                elif component_type == "VisualScript":
                    visual_graphs += 1
                elif component_type == "Animator":
                    animators += 1

            for script in getattr(unit, "scripts", []):
                script_name = getattr(script, "script_name", None)

                if script_name:
                    scripts.add(script_name)

        build_settings = {}

        if hasattr(self.game, "build_settings"):
            build_settings = dict(getattr(self.game.build_settings, "data", {}))

        runtime_config = {}

        if hasattr(self.game, "runtime_config"):
            runtime_config = self.game.runtime_config.serialize()

        editor_view_settings = {}

        if hasattr(self.game, "editor_view_settings"):
            editor_view_settings = self.game.editor_view_settings.serialize()

        data = {
            "engine_version": "0.6.0",
            "project": self.game.settings.get("project_name", "MiniForge Project"),
            "current_scene": self.game.scene_manager.current_scene if hasattr(self.game, "scene_manager") else None,
            "build_settings": build_settings,
            "runtime_config": runtime_config,
            "editor_view_settings": editor_view_settings,
            "assets": {
                "sprites": sorted(list(sprites)),
                "scripts": sorted(list(scripts)),
                "prefabs": sorted(list(prefabs)),
                "scenes": scenes,
            },
            "scene_data": {
                "entity_count": len(self.game.units),
                "components": sorted(list(components)),
                "tags_used": sorted(list(tags)),
                "layers_used": sorted(list(layers)),
                "ui_elements": ui_elements,
                "visual_graphs": visual_graphs,
                "animators": animators,
                "tilemap": getattr(getattr(self.game, "tilemap_layers", None), "stats", lambda: {})(),
            }
        }

        os.makedirs("project", exist_ok=True)

        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        with open("manifest.json", "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        self.game.console.log("Manifest generado: project/manifest.json", "ENGINE")
        return data
