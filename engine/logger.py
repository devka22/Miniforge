# core/logger.py

import os
from datetime import datetime


class Logger:
    def __init__(self, log_folder="logs", log_file="engine.log"):
        self.log_folder = log_folder
        self.log_file = log_file

        if not os.path.exists(self.log_folder):
            os.makedirs(self.log_folder)

        self.log_path = os.path.join(self.log_folder, self.log_file)
        self.error_log_path = os.path.join(self.log_folder, "error.log")

    def _write(self, level, message):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        final_message = f"[{timestamp}] [{level}] {message}"

        print(final_message)

        with open(self.log_path, "a", encoding="utf-8") as file:
            file.write(final_message + "\n")

        if level == "ERROR":
            with open(self.error_log_path, "a", encoding="utf-8") as file:
                file.write(final_message + "\n")

    def info(self, message):
        self._write("INFO", message)

    def warning(self, message):
        self._write("WARNING", message)

    def error(self, message):
        self._write("ERROR", message)

    def debug(self, message):
        self._write("DEBUG", message)
