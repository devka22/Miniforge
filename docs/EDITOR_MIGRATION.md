# MiniForge Editor Migration

The Qt editor is being introduced beside the existing Macroquad/egui editor. The legacy path remains available until the Qt path reaches functional parity panel by panel.

## Phase 1: Vertical Slice

- Rust `EditorCore` wraps real `Game` services.
- C ABI exposes opaque handles and versioned DTOs.
- Qt shell hosts docking, hierarchy, inspector, content browser, console, command palette and viewport snapshot.
- Scripts configure, build, run, test and package the editor when CMake and Qt are installed.

## Phase 2: Interaction Depth

- Multi-selection, hierarchy context menus and incremental tree updates.
- Inspector editing controls per property type.
- Asset filtering, search and async thumbnails.
- Console table filtering and severity controls.
- Layout persistence per workspace.

## Phase 3: Viewport

- Replace RGBA CPU snapshots with a shared GPU path.
- Add camera controls, selection, snapping, gizmos and debug overlays.
- Route viewport input through a priority-aware editor input layer.

## Phase 4: Extensibility

- Host compiled TypeScript plugins in a sandboxed QuickJS runtime.
- Validate plugin manifests and permissions.
- Expose plugin commands, panels, importers and menu items through `EditorCore`.

## Exit Criteria

The legacy editor can only be retired after the Qt editor can open projects, edit scenes, run play mode, save reliably, inspect entities, manage assets and expose the same core workflows without data loss.
