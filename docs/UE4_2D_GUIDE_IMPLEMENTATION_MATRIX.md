# UE4 2D Guide Implementation Matrix

This matrix maps `UE4_2D_EDITOR_UI_ARCHITECTURE_GUIDE.md` to the MiniForge2D modules added in `src/engine/miniforge_2d`.

| Guide item | Implementation |
|---|---|
| Editor Layout guardado en `editor_layout.json` | `editor_layout::EditorLayout2D` |
| Toolbar funcional | `toolbar::Toolbar2D` |
| World Outliner conectado con escena | `world_outliner::WorldOutliner2D` |
| Inspector editable | `details_inspector::DetailsInspector2D` |
| Scene View con grid, zoom, pan y gizmos | `scene_view::SceneView2D` |
| Content Browser con previews y GUIDs | `content_browser::ContentBrowserCatalog2D` |
| Sistema de tabs | `editor_tabs::EditorTabSession2D` |
| Console con comandos | `console_panel::ConsolePanel2D` |
| Problems Panel con validación | `problems_panel::ProblemsPanel2D` |
| Project Settings con `project_config.json` | `project_settings2d::ProjectSettings2D` |
| UI2D runtime | `ui_framework::UiCanvas2D` |
| UI Designer visual | `ui_designer::UiDesigner2D` |
| Visual Graph mejorado | `blueprint::BlueprintGraph2D` |
| Blueprint Library | `blueprint_library::BlueprintLibrary2D` |
| Paper2D-like completo | `paper2d` |
| Animation Blueprint 2D | `animation_blueprint::AnimationBlueprint2D` |
| Sequencer2D | `sequencer2d::Sequencer2D` |
| Physics2D | `physics2d::Physics2DSettings` |
| AI Behavior Tree | `ai::BehaviorTree2D` |
| RTS Tools | `rts_tools::RtsTools2D` |
| AssetDatabase avanzado | `asset_registry2d` |
| Plugin System | `plugin_system2d` |
| Exporter debug/release | `exporter2d` + `packaging2d` |
| Ejemplos mínimos | `examples::minimal_examples()` and `examples/miniforge_2d/miniforge_2d_examples.json` |
| Documentación interna | `docs/MINIFORGE_2D_ARCHITECTURE.md` |

The implementation is data-first and compatible with the existing Rust + macroquad + Rhai runtime. UI-facing modules are state/controllers that the existing editor can render and wire into egui without replacing current systems.
