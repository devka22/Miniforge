import json
import os


class BuildProfiles:
    """
    Perfiles de build 0.6.0: Debug, Release y Desktop.
    Cada perfil puede sobreescribir BuildSettings antes de exportar.
    """

    def __init__(self, path=None):
        self.path = path or os.path.join("settings", "build_profiles.json")
        self.active = "Debug"
        self.profiles = {
            "Debug": {
                "show_debug": True,
                "include_all_assets": True,
                "target_fps": 60,
            },
            "Release": {
                "show_debug": False,
                "include_all_assets": False,
                "target_fps": 60,
            },
            "Desktop": {
                "fullscreen": False,
                "window_width": 1280,
                "window_height": 720,
                "show_debug": False,
            },
        }
        self.load()

    def load(self):
        if not os.path.exists(self.path):
            self.save()
            return

        try:
            with open(self.path, "r", encoding="utf-8") as file:
                data = json.load(file)

            self.active = data.get("active", self.active)
            self.profiles.update(data.get("profiles", {}))
        except Exception:
            self.save()

    def save(self):
        os.makedirs(os.path.dirname(self.path), exist_ok=True)

        with open(self.path, "w", encoding="utf-8") as file:
            json.dump(
                {
                    "active": self.active,
                    "profiles": self.profiles,
                },
                file,
                indent=4,
            )

    def names(self):
        return sorted(self.profiles.keys())

    def set_active(self, name):
        if name in self.profiles:
            self.active = name
            self.save()
            return True

        return False

    def cycle(self):
        names = self.names()
        index = names.index(self.active) if self.active in names else 0
        self.active = names[(index + 1) % len(names)]
        self.save()
        return self.active

    def apply_to(self, build_settings):
        for key, value in self.profiles.get(self.active, {}).items():
            build_settings.data[key] = value

        build_settings.save()
