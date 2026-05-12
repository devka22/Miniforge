import json
import os

from engine.prefab_manager import PrefabManager


class AdvancedPrefabSystem:
    def __init__(self, game):
        self.game = game
        self.variant_suffix = ".variant.prefab"
        self.stats = {
            "variants_created": 0,
            "nested_instances": 0,
            "last_operation": None,
        }

    def create_variant_from_selected(self):
        entity = self.first_selected()

        if not entity:
            self.log("Selecciona una entidad para crear variant", "WARNING")
            return None

        source = getattr(entity, "prefab_source", None)

        if not source:
            source = PrefabManager.save_prefab(entity)
            entity.prefab_source = source
            entity.is_prefab_instance = True

        base = os.path.splitext(os.path.basename(source))[0]
        filename = f"{base}_variant{self.variant_suffix}"
        path = os.path.join(os.path.dirname(source), filename)
        data = {
            "version": "0.6.0",
            "prefab_name": filename,
            "base_prefab": source,
            "variant": True,
            "overrides": self.collect_overrides(entity, source),
            "entity": entity.serialize(),
        }

        with open(path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        entity.prefab_source = path
        entity.is_prefab_instance = True
        self.stats["variants_created"] += 1
        self.stats["last_operation"] = "create_variant"
        self.refresh()
        self.log(f"Prefab variant creado: {path}", "ASSET")
        return path

    def collect_overrides(self, entity, source):
        if not os.path.exists(source):
            return []

        with open(source, "r", encoding="utf-8") as file:
            data = json.load(file)

        base = data.get("entity", {})
        current = entity.serialize()
        return self.diff("", base, current)

    def diff(self, path, base, current):
        changes = []

        if isinstance(base, dict) and isinstance(current, dict):
            keys = sorted(set(base.keys()) | set(current.keys()))

            for key in keys:
                next_path = f"{path}.{key}" if path else key
                changes.extend(self.diff(next_path, base.get(key), current.get(key)))

            return changes

        if isinstance(base, list) and isinstance(current, list):
            if base != current:
                changes.append({"path": path, "base": base, "current": current})
            return changes

        if base != current:
            changes.append({"path": path, "base": base, "current": current})

        return changes

    def instantiate_nested_prefab_as_child(self, prefab_path=None):
        parent = self.first_selected()

        if not parent:
            self.log("Selecciona parent para nested prefab", "WARNING")
            return None

        prefab_path = prefab_path or getattr(parent, "prefab_source", None)

        if not prefab_path:
            self.log("No hay prefab seleccionado para anidar", "WARNING")
            return None

        child = PrefabManager.instantiate_prefab(self.game, prefab_path, parent.x + 1, parent.y + 1)

        if not child:
            return None

        if hasattr(self.game, "hierarchy_manager"):
            self.game.hierarchy_manager.set_parent(child, parent)

        child.name = f"{parent.name}_Nested"
        self.stats["nested_instances"] += 1
        self.stats["last_operation"] = "nested_instance"
        self.log(f"Nested prefab creado bajo {parent.name}", "ASSET")
        return child

    def apply_component_override(self, component_type):
        entity = self.first_selected()

        if not entity or not getattr(entity, "prefab_source", None):
            return False

        self.game.prefab_workflow.apply_selected_to_prefab()
        self.stats["last_operation"] = f"apply_{component_type}"
        return True

    def first_selected(self):
        selected = getattr(self.game, "selected_units", [])
        return selected[0] if selected else None

    def refresh(self):
        if hasattr(self.game, "refresh_project"):
            self.game.refresh_project()

        if hasattr(self.game, "asset_database"):
            self.game.asset_database.scan()

    def log(self, message, level="ASSET"):
        if hasattr(self.game, "console"):
            self.game.console.log(message, level)
