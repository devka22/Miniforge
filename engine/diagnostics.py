import time


class Diagnostics:
    """
    Métricas rápidas del editor/runtime.
    """

    def __init__(self, game):
        self.game = game
        self.frame_times = []
        self.last_sample = time.time()

    def update(self, dt):
        self.frame_times.append(dt)

        if len(self.frame_times) > 120:
            self.frame_times.pop(0)

    def fps(self):
        if not self.frame_times:
            return 0

        avg = sum(self.frame_times) / len(self.frame_times)

        if avg <= 0:
            return 0

        return round(1.0 / avg, 1)

    def rows(self):
        return [
            ("FPS", self.fps()),
            ("Entities", len(getattr(self.game, "units", []))),
            ("Selected", len(getattr(self.game, "selected_units", []))),
            ("Assets", len(getattr(self.game.asset_database, "assets", []))),
            ("Scripts", len(getattr(self.game.script_manager, "scripts", {}))),
            ("Project Systems", len(getattr(self.game, "project_runtime_systems", []))),
            ("Engine Systems", len(getattr(self.game.system_scheduler, "entries", []))),
            ("System Errors", sum(row[-1] for row in self.game.system_scheduler.health_rows())),
            ("Animators", getattr(getattr(self.game, "profiler", None), "counters", {}).get("Animators", 0)),
            ("UI Elements", getattr(getattr(self.game, "ui_canvas", None), "stats", {}).get("elements", 0)),
            ("Visual Graphs", getattr(getattr(self.game, "visual_script_runtime", None), "stats", {}).get("graphs", 0)),
            ("Resources", getattr(getattr(self.game, "resources", None), "stats", lambda: {})()),
            ("Errors", len(getattr(getattr(self.game, "error_handler", None), "recent_errors", []))),
            ("Mode", getattr(self.game, "mode", "-")),
            ("Tab", getattr(self.game.editor_tabs, "active", "-")),
            ("Scene Dirty", getattr(self.game, "scene_dirty", False)),
            ("Console Filter", getattr(self.game.console, "filter_level", "ALL")),
        ]
