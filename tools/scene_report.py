"""Example trusted production tool: summarizes editor selection and assets."""

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
EditorToolResult = api.EditorToolResult
read_request = api.read_request


request = read_request()
context = request["context"]
result = EditorToolResult(
    message=(
        f"Scene={context.get('active_scene') or 'none'} | "
        f"selection={len(context.get('selected_entity_ids', []))} | "
        f"assets={len(context.get('assets', []))}"
    )
)
result.log(result.message)
result.emit()
