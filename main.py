import os
import subprocess
import sys
from pathlib import Path


BASE_DIR = Path(__file__).resolve().parent


def run_legacy_python():
    legacy = BASE_DIR / "legacy_python_main.py"
    if not legacy.exists():
        print("Legacy Python launcher is not available.", file=sys.stderr)
        return 1
    return subprocess.call([sys.executable, str(legacy), *sys.argv[1:]], cwd=BASE_DIR)


def main():
    if os.environ.get("MINIFORGE_LEGACY_PYTHON") == "1":
        return run_legacy_python()

    binary = BASE_DIR / "target" / "debug" / "miniforge"
    if os.environ.get("MINIFORGE_USE_BUILT_BINARY") == "1" and binary.exists():
        command = [str(binary), *sys.argv[1:]]
    else:
        command = ["cargo", "run", "--", *sys.argv[1:]]

    return subprocess.call(command, cwd=BASE_DIR)


if __name__ == "__main__":
    raise SystemExit(main())
