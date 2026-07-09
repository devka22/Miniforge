# MiniForge2D Architecture

MiniForge2D is a UE-inspired 2D layer for MiniForge. It does not copy Unreal code; it maps familiar editor/runtime ideas onto the existing Rust + macroquad + Luau stack.

## Core Rule

Every system is data-first and JSON-friendly. Scenes, prefabs, content metadata, visual graphs, animation blueprints, UI canvases, behavior trees, sequencer timelines, physics settings and package manifests can be serialized, validated and expanded without replacing the current `GameObject` + `Component` runtime.

## Modules

- `actor`: Actor2D descriptors and factories that produce compatible `GameObject` values.
- `editor_layout`: UE-style dark layout state persisted as `editor_layout.json`.
- `toolbar`: Save/Play/Build/Export/Validate actions plus editor state and alerts.
- `scene_view`: zoom, pan, grid, snap, selection, gizmo and debug overlay state.
- `editor_tabs`: document tabs with dirty state, close confirmation and session persistence.
- `console_panel`: command parser and filtered console facade over `DeveloperConsole`.
- `problems_panel`: grouped/filterable validation issues for scripts, graphs, assets, scenes, export and config.
- `project_settings2d`: typed `project_config.json` data with backup, reset and validation.
- `gameplay`: `GameMode2D`, `Pawn2D`, `PlayerController2D` and `AIController2D` configuration.
- `blueprint`: visual scripting graph model with events, exec pins, variables, functions and runtime conversion to `VisualScript`.
- `blueprint_library`: searchable graph templates for players, enemies, gameplay, RTS, UI and save points.
- `content_browser`: GUID-based asset catalog with labels, previews, filters and dependency checks.
- `details_inspector`: editable field sections for entities, components and assets.
- `world_outliner`: hierarchy, search, enabled toggles and reparent helpers.
- `paper2d`: sprites, tilemaps, tilesets, flipbooks and collision tile data.
- `animation_blueprint`: states, transitions, parameters and frame events bridged to the existing animation graph library.
- `ui_framework`: UMG-like Canvas, Panel, Text/Button/Image/ProgressBar-style widgets, anchors and callbacks.
- `ui_designer`: `.mfui` editor state for visual canvas editing, widget selection, binding picker and preview.
- `sequencer2d`: camera, audio, event and dialogue timeline tracks.
- `physics2d`: Rigidbody2D/Collider2D/Trigger2D settings, raycast queries, layers and debug draw flags.
- `ai`: Blackboard and Behavior Tree data for Patrol, Chase, Attack, Flee and RTS commands.
- `rts_tools`: RTS creation toolkit data and a minimal demo scene spec.
- `asset_registry2d`: advanced asset analysis for missing assets, duplicate GUIDs and unused assets.
- `plugin_system2d`: plugin manifest/scaffold compatible with `plugins/PluginName/plugin.json`.
- `exporter2d`: debug/release build layout and pre-export validation.
- `packaging2d`: debug/release package manifest with used assets and validation results.
- `validation`: shared validation report and reference checks.

## Guide Coverage

The implementation covers the final checklist from `UE4_2D_EDITOR_UI_ARCHITECTURE_GUIDE.md` as modular, testable state/controllers:

- Editor Layout, Toolbar, World Outliner, Inspector, Scene View.
- Content Browser, Tabs, Console, Problems Panel, Project Settings.
- UI2D runtime model, UI Designer model, Visual Graph model and Blueprint Library.
- Paper2D-like sprites/tilemaps, Animation Blueprint 2D, Sequencer2D, Physics2D.
- AI Behavior Tree, RTS Tools, AssetDatabase analysis, Plugin System and Exporter.

## Examples

Minimal JSON examples live in:

```text
examples/miniforge_2d/miniforge_2d_examples.json
```

The same examples are available from Rust via:

```rust
use miniforge::engine::miniforge_2d::examples_document;

let doc = examples_document();
```

## Compatibility Notes

- Existing components are preserved. New defaults only add component types such as `Actor2D`, `GameMode2D`, `Pawn2D`, `TilemapRenderer2D`, `AnimationBlueprint2D`, `WidgetCanvas2D`, `Sequencer2D`, `Trigger2D` and `BehaviorTree2D`.
- Existing `.mfgraph` assets continue to use the current `VisualScriptRuntime`. The new Blueprint model can export a compatible `VisualScript` component.
- `ProjectValidator` now checks graph node IDs, broken exec links and unknown node kinds/types.
- Packaging remains compatible with `PackagingManager` and `RuntimeExporter`; `PackageManifest2D` is a small manifest model for validation and tooling.
