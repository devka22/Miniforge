import json
import os
import importlib.util


class PluginManager:
    """
    Scanner simple de plugins/packages del proyecto.
    Busca plugin.json dentro de plugins/ y packages/.
    """

    def __init__(self, project_path):
        self.project_path = project_path
        self.plugins = []
        self.scan()

    def roots(self):
        return [
            os.path.join(self.project_path, "plugins"),
            os.path.join(self.project_path, "packages"),
        ]

    def scan(self):
        self.plugins = []

        for root in self.roots():
            os.makedirs(root, exist_ok=True)

            for current, _, files in os.walk(root):
                if "plugin.json" not in files:
                    continue

                path = os.path.join(current, "plugin.json")

                try:
                    with open(path, "r", encoding="utf-8") as file:
                        data = json.load(file)
                except Exception as error:
                    data = {
                        "name": os.path.basename(current),
                        "enabled": False,
                        "error": str(error),
                    }

                data.setdefault("name", os.path.basename(current))
                data.setdefault("enabled", True)
                data["path"] = current
                self.plugins.append(data)

        return self.plugins

    def enabled_plugins(self):
        return [
            plugin for plugin in self.plugins
            if plugin.get("enabled", True)
        ]

    def summary(self):
        return [
            f"{plugin.get('name')} ({'on' if plugin.get('enabled', True) else 'off'})"
            for plugin in self.plugins
        ]

    def emit_hook(self, hook_name, game):
        handled = 0

        for plugin in self.enabled_plugins():
            module_path = os.path.join(plugin.get("path", ""), "plugin.py")

            if not os.path.exists(module_path):
                continue

            try:
                module = self.load_module(module_path)
                hook = getattr(module, hook_name, None)

                if not hook:
                    continue

                hook(game)
                handled += 1
            except Exception as error:
                if hasattr(game, "console"):
                    game.console.log(
                        f"Plugin hook error {plugin.get('name')}:{hook_name}: {error}",
                        "ERROR"
                    )

        return handled

    def load_module(self, path):
        module_name = f"miniforge_plugin_{abs(hash(path))}"
        spec = importlib.util.spec_from_file_location(module_name, path)

        if not spec or not spec.loader:
            raise ImportError(f"No se pudo cargar plugin: {path}")

        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
