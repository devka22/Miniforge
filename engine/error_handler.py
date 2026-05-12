import os
import traceback
import datetime


class ErrorHandler:
    """
    Manejo centralizado de errores del motor.
    Evita que el motor crashee por errores pequeños y guarda logs.
    """

    def __init__(self, game):
        self.game = game
        project_path = getattr(game, "project_path", None)
        self.log_folder = os.path.join(project_path, "logs") if project_path else "logs"
        self.log_file = os.path.join(self.log_folder, "engine.log")
        self.error_log_file = os.path.join(self.log_folder, "error.log")
        self.recent_errors = []
        self.error_counts = {}
        self.last_call_failed = False

        os.makedirs(self.log_folder, exist_ok=True)

    def write_error(self, title, error):
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        trace = traceback.format_exc()
        key = f"{title}:{error.__class__.__name__}"
        self.error_counts[key] = self.error_counts.get(key, 0) + 1

        text = (
            f"\n[{timestamp}] {title}\n"
            f"ERROR: {error}\n"
            f"{trace}\n"
            f"{'-' * 60}\n"
        )

        self.recent_errors.append(
            {
                "timestamp": timestamp,
                "title": title,
                "error": str(error),
                "type": error.__class__.__name__,
                "count": self.error_counts[key],
            }
        )
        self.recent_errors = self.recent_errors[-50:]

        try:
            os.makedirs(self.log_folder, exist_ok=True)

            with open(self.log_file, "a", encoding="utf-8") as file:
                file.write(text)

            with open(self.error_log_file, "a", encoding="utf-8") as file:
                file.write(text)
        except Exception:
            pass

        if hasattr(self.game, "console"):
            self.game.console.log(f"{title}: {error}", "ERROR")

    def safe_call(self, title, func, *args, **kwargs):
        self.last_call_failed = False

        try:
            return func(*args, **kwargs)

        except Exception as error:
            self.last_call_failed = True
            self.write_error(title, error)
            return None

    def summary(self):
        return {
            "recent": list(self.recent_errors),
            "counts": dict(self.error_counts),
            "log_file": self.log_file,
            "error_log_file": self.error_log_file,
        }
