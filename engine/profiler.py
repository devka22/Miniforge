import time


class Profiler:
    def __init__(self, game=None, max_samples=180):
        self.game = game
        self.max_samples = max_samples
        self.enabled = True
        self.paused = False
        self.frame_index = 0
        self.system_samples = {}
        self.frame_samples = []
        self.counters = {}
        self._frame_start = None

    def begin_frame(self):
        if not self.enabled or self.paused:
            return

        self._frame_start = time.perf_counter()

    def end_frame(self):
        if not self.enabled or self.paused or self._frame_start is None:
            return

        elapsed = (time.perf_counter() - self._frame_start) * 1000.0
        self.frame_samples.append(elapsed)
        self.frame_samples = self.frame_samples[-self.max_samples:]
        self.frame_index += 1
        self._frame_start = None

    def record_system(self, name, milliseconds):
        if not self.enabled or self.paused:
            return

        samples = self.system_samples.setdefault(name, [])
        samples.append(float(milliseconds))

        if len(samples) > self.max_samples:
            samples.pop(0)

    def set_counter(self, name, value):
        self.counters[name] = value

    def average(self, values):
        if not values:
            return 0.0
        return round(sum(values) / len(values), 3)

    def rows(self):
        rows = [
            ("Frame ms", self.average(self.frame_samples)),
            ("Frame", self.frame_index),
        ]

        for name, samples in sorted(
            self.system_samples.items(),
            key=lambda item: self.average(item[1]),
            reverse=True,
        )[:12]:
            rows.append((name, f"{self.average(samples)} ms"))

        for key, value in sorted(self.counters.items()):
            rows.append((key, value))

        return rows

    def toggle_pause(self):
        self.paused = not self.paused
        return self.paused

    def serialize_snapshot(self):
        return {
            "frame_index": self.frame_index,
            "frame_ms": self.average(self.frame_samples),
            "systems": {
                name: self.average(samples)
                for name, samples in self.system_samples.items()
            },
            "counters": self.counters,
        }
