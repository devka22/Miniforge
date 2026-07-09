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
