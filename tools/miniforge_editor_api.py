"""Tiny JSON protocol helpers for trusted MiniForge editor automation.

Python tools receive one JSON request on stdin and must print one JSON result as
their last non-empty stdout line. They never run in exported gameplay builds.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from typing import Any


@dataclass
class EditorToolResult:
    success: bool = True
    message: str = ""
    operations: list[dict[str, Any]] = field(default_factory=list)
    generated_files: list[str] = field(default_factory=list)

    def log(self, message: str) -> None:
        self.operations.append({"operation": "log", "value": message})

    def request_reimport(self, asset_path: str) -> None:
        self.operations.append(
            {"operation": "request_reimport", "target": asset_path}
        )

    def refresh_assets(self) -> None:
        """Request one asset database scan after the tool finishes."""
        self.operation("refresh_assets")

    def select_entities(self, entity_ids: list[int]) -> None:
        """Replace the editor selection with existing entity ids."""
        self.operation("select_entities", value=entity_ids)

    def set_selection_properties(
        self,
        *,
        visible: bool | None = None,
        locked: bool | None = None,
        enabled: bool | None = None,
    ) -> None:
        """Apply common editor flags to the selection as one undo step."""
        values = {
            key: value
            for key, value in {
                "visible": visible,
                "locked": locked,
                "enabled": enabled,
            }.items()
            if value is not None
        }
        if values:
            self.operation("set_editor_property", "selection", values)

    def batch_import_assets(
        self, source: str, destination: str = "assets/imported"
    ) -> None:
        self.operation(
            "batch_import_assets", source, {"destination": destination, "recursive": True}
        )

    def convert_sprites(
        self, source: str, destination: str = "assets/sprites/converted"
    ) -> None:
        self.operation(
            "convert_sprites", source, {"destination": destination, "format": "png"}
        )

    def generate_atlas(
        self,
        source: str,
        destination: str = "assets/atlases",
        *,
        size: int = 4096,
        extrude: int = 1,
    ) -> None:
        self.operation(
            "generate_atlas",
            source,
            {"destination": destination, "size": size, "extrude": extrude},
        )

    def operation(
        self, operation: str, target: str = "", value: Any = None
    ) -> None:
        """Queue a validated editor operation for the Rust host to apply."""
        self.operations.append(
            {"operation": operation, "target": target, "value": value}
        )

    def emit(self) -> None:
        print(json.dumps(self.__dict__, ensure_ascii=False))


def read_request() -> dict[str, Any]:
    payload = json.load(sys.stdin)
    if payload.get("protocol") != "miniforge-editor-tool-v1":
        raise ValueError("Unsupported MiniForge editor tool protocol")
    return payload
