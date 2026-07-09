# MiniForge Editor Architecture

MiniForge keeps Rust as the source of truth for runtime state, scenes, entities, assets, serialization, physics, rendering data, commands and undo/redo. The Qt editor is a frontend over that core, not a rewrite of the engine in C++.

## Layers

- Rust `EditorCore`: project loading, entity snapshots, inspector fields, asset rows, command execution, console entries and viewport RGBA snapshots.
- C ABI: stable opaque-handle boundary in `include/miniforge_editor_bridge.h`.
- C++ Qt shell: `QMainWindow`, `QDockWidget`, native models, bridge adapter and viewport widget.
- QML: panel presentation, reusable controls, theme values and lightweight interactions.
- TypeScript contract: future plugin API under `editor-plugins/typescript`, intended for esbuild plus QuickJS in a later phase.

## Ownership

Rust creates `MfEditorHandle` and destroys it through `mf_editor_destroy`. C++ owns Qt objects and all output buffers passed through the ABI. Rust never returns internal pointers to Qt.

## Data Flow

Qt models request batches from the C++ bridge. The bridge calls Rust through fixed DTOs and copies data into Qt-owned containers. QML reads roles from those models. Mutations flow back through commands or explicit editor operations.

Inspector edits flow from `MfPropertyRow` to `MfEditorController::setInspectorValue`, then through `MfBridge::setInspectorValueJson` and `mf_editor_set_inspector_value_json`. Rust applies the change through `EditorCore::edit_inspector_value_json` and refreshes scene caches before Qt models repaint.

## Verification

The Qt build registers `miniforge_editor_bridge_smoke` with CTest. It is intentionally below QML: a small C++ program links the Rust editor bridge dylib and verifies the ABI against `projects/DefaultProject`, including hierarchy, selection, inspector, assets, commands, console and viewport snapshot data.

## Viewport

The current viewport is a CPU RGBA snapshot generated from real Rust scene data. It is intentionally not zero-copy yet. The next rendering phase should replace this with a Qt/Rust GPU interop path suitable for Metal on macOS and Vulkan/D3D on other platforms.

## Stability Rules

- Add ABI functions instead of changing existing structs.
- Keep legacy Macroquad/egui editor code available until panel parity exists.
- Do not install system dependencies from scripts.
- Keep C++/QML free of direct engine mutation.
