import json
import os


class PrefabOverrides:
    """
    Calcula diferencias simples entre una instancia y su prefab fuente.
    """

    IGNORED_KEYS = {
        "id",
        "x",
        "y",
        "selected",
        "path",
        "state",
        "command",
    }

    def __init__(self, game):
        self.game = game

    def source_data(self, entity):
        source = getattr(entity, "prefab_source", None)

        if not source or not os.path.exists(source):
            return None

        try:
            with open(source, "r", encoding="utf-8") as file:
                data = json.load(file)
        except Exception:
            return None

        return data.get("entity")

    def diff(self, entity):
        source = self.source_data(entity)

        if not source:
            return []

        current = entity.serialize()
        return self.diff_dict("", source, current)

    def diff_dict(self, prefix, original, current):
        diffs = []

        keys = set(original.keys()) | set(current.keys())

        for key in sorted(keys):
            if key in self.IGNORED_KEYS:
                continue

            path = f"{prefix}.{key}" if prefix else key
            old_value = original.get(key)
            new_value = current.get(key)

            if isinstance(old_value, dict) and isinstance(new_value, dict):
                diffs.extend(self.diff_dict(path, old_value, new_value))
                continue

            if old_value != new_value:
                diffs.append(
                    {
                        "path": path,
                        "prefab": old_value,
                        "instance": new_value,
                    }
                )

        return diffs
