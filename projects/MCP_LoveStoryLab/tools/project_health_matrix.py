from __future__ import annotations

import json
import importlib.util
import sys
from pathlib import Path

api_path = Path(__file__).with_name("miniforge_editor_api.py")
spec = importlib.util.spec_from_file_location("miniforge_editor_api", api_path)
if spec is None or spec.loader is None:
    raise RuntimeError("Could not load MiniForge Python editor API")
api = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = api
spec.loader.exec_module(api)


def count_files(root: Path, suffixes: tuple[str, ...]) -> int:
    if not root.exists():
        return 0
    total = 0
    for path in root.rglob("*"):
        if path.is_file() and any(str(path).endswith(suffix) for suffix in suffixes):
            total += 1
    return total


def main() -> None:
    request = api.read_request()
    project = Path(request["context"]["project_root"])

    matrix = {
        "scenes": count_files(project / "saves" / "scenes", (".scene",)),
        "luau_scripts": count_files(project / "scripts", (".luau", ".lua")),
        "visual_graphs": count_files(project / "scripts" / "visual_graphs", (".mfgraph",)),
        "python_tools": count_files(project / "tools", (".mftool.json",)),
        "plugins": count_files(project / "plugins", ("plugin.json",)),
        "prefabs": count_files(project / "assets" / "prefabs", (".prefab",)),
        "sprites": count_files(project / "assets" / "sprites", (".png", ".sprite.json")),
        "manifest_present": (project / "manifest.json").exists(),
        "engine_config_present": (project / "engine_config.json").exists(),
        "render_backend": "macroquad",
        "prefer_metal_on_macos": True,
        "experimental_wgpu": False,
    }

    result = api.EditorToolResult(
        success=True,
        message="Project health matrix generated",
    )
    result.log(json.dumps(matrix, ensure_ascii=False, sort_keys=True))
    result.operation("refresh_assets")
    result.emit()


if __name__ == "__main__":
    main()
