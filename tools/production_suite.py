"""Built-in production automations exposed by MiniForge's Python Tools window.

The Python side declares intent; the Rust editor applies the operation with its
normal asset database, undo, scene and build services. This keeps custom tools
small while avoiding direct mutation of live editor state from a subprocess.
"""

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

request = api.read_request()
context = request.get("context", {})
tool = request.get("tool", {})
tool_id = tool.get("id") or context.get("parameters", {}).get("action", "")
result = api.EditorToolResult(message=f"Automation queued: {tool_id}")

operations = {
    "batch_asset_import": (
        "batch_import_assets",
        "import_drop",
        {"destination": "assets/imported", "recursive": True},
    ),
    "sprite_converter": (
        "convert_sprites",
        "assets/sprites",
        {"destination": "assets/sprites/converted", "format": "png"},
    ),
    "atlas_generator": (
        "generate_atlas",
        "assets/sprites",
        {"destination": "assets/atlases", "size": 4096, "extrude": 1},
    ),
    "bulk_properties": (
        "set_editor_property",
        "selection",
        {"visible": True, "locked": False, "enabled": True},
    ),
    "procedural_level": (
        "create_procedural_level",
        "PythonProcedural",
        {"width": 24, "height": 16, "spacing": 2.0, "seed": 1337},
    ),
    "project_data_export": (
        "export_project_data",
        ".miniforge/generated/exports",
        {"include_scene": True, "include_assets": True},
    ),
    "automated_build": (
        "automate_build",
        "debug",
        {"validate": True, "manifest": True, "package": False},
    ),
    "animation_processor": (
        "process_animations",
        "assets/animations",
        {"normalize_fps": 12.0, "validate_frames": True},
    ),
    "documentation_generator": (
        "generate_documentation",
        ".miniforge/generated/docs",
        {"format": "markdown", "include_components": True},
    ),
}

if tool_id not in operations:
    result.success = False
    result.message = f"Unknown production automation: {tool_id}"
else:
    operation, target, value = operations[tool_id]
    if operation == "batch_import_assets":
        result.batch_import_assets(target, value["destination"])
    elif operation == "convert_sprites":
        result.convert_sprites(target, value["destination"])
    elif operation == "generate_atlas":
        result.generate_atlas(
            target,
            value["destination"],
            size=value["size"],
            extrude=value["extrude"],
        )
    elif operation == "set_editor_property":
        result.set_selection_properties(**value)
    else:
        result.operation(operation, target, value)
    result.log(result.message)

result.emit()
