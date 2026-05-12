import time


class SystemScheduler:
    """
    Scheduler de sistemas con prioridades.
    Respeta modo EDITOR/PLAY, mide tiempos y mantiene estado de salud.
    """

    def __init__(self, game):
        self.game = game
        self.entries = []
        self.max_dt = 0.1

    def register(self, system, priority=100, name=None, enabled=True):
        self.entries.append(
            {
                "system": system,
                "priority": priority,
                "name": name or system.__class__.__name__,
                "enabled": enabled,
                "last_ms": 0.0,
                "average_ms": 0.0,
                "updates": 0,
                "errors": 0,
            }
        )
        self.entries.sort(key=lambda entry: entry["priority"])

    def update(self, dt):
        dt = min(max(float(dt), 0.0), self.max_dt)

        for entry in self.entries:
            if not entry.get("enabled", True):
                continue

            system = entry["system"]

            if not self.should_update(system):
                continue

            play_mode = getattr(self.game, "play_mode_manager", None)

            if getattr(play_mode, "paused", False):
                if not getattr(system, "update_when_paused", False):
                    continue

            update = getattr(system, "update", None)

            if not update:
                continue

            start = time.perf_counter()
            result = self.game.error_handler.safe_call(
                f"System update {entry['name']}",
                update,
                dt
            )
            elapsed = (time.perf_counter() - start) * 1000.0
            entry["last_ms"] = round(elapsed, 4)
            entry["updates"] += 1
            entry["average_ms"] = round(
                entry["average_ms"] * 0.9 + elapsed * 0.1,
                4
            )

            if result is None and getattr(self.game.error_handler, "last_call_failed", False):
                entry["errors"] += 1

            profiler = getattr(self.game, "profiler", None)

            if profiler:
                profiler.record_system(entry["name"], elapsed)

    def should_update(self, system):
        mode = getattr(self.game, "mode", "EDITOR")

        if mode == "EDITOR" and not getattr(system, "run_in_editor", True):
            return False

        if mode == "PLAY" and not getattr(system, "run_in_play", True):
            return False

        return getattr(system, "enabled", True)

    def set_enabled(self, name, enabled):
        for entry in self.entries:
            if entry["name"] == name:
                entry["enabled"] = bool(enabled)
                return True
        return False

    def names(self):
        return [entry["name"] for entry in self.entries]

    def health_rows(self):
        return [
            (
                entry["name"],
                entry["enabled"],
                entry["last_ms"],
                entry["average_ms"],
                entry["errors"],
            )
            for entry in self.entries
        ]
