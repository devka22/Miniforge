# Migracion Definitiva Al Editor Qt

Estado de esta rama: la eliminacion tecnica del editor visual Rust y la migracion funcional Qt estan completas. Qt 6/C++/QML es la unica superficie desktop objetivo y Rust conserva runtime, servicios frontend-neutral, `EditorCore`, `editor_ffi` y la CLI `miniforge_dev`.

Este documento es una decisión arquitectónica y matriz de paridad, no una guía de uso. Consulta
[Editor y flujo de uso](EDITOR_Y_FLUJO_DE_USO.md) para trabajar con el motor y
[Desarrollo, build y extensión](DESARROLLO_BUILD_Y_EXTENSION.md) para ejecutar los gates.

## Veredicto De Paridad

**GO para el port Qt como única superficie visual.** Launcher, Settings/Input/Tags, Project Operations, Content Browser,
Visual Graph, Animation Timeline, Tilemap, UI Designer, Prefab Studio, Luau recovery/debugger,
Sprite Studio, Asset Management, Profiler y Asset Dependency Graph tienen superficie Qt y backend.
Project Operations recupera export/import `.mfpkg.zip`, distributables, autosave/recovery, session
checkpoints y Play/Build en proceso externo. El viewport incorpora Scene/Game, gizmos
Move/Rotate/Scale con undo único, box selection, pan/zoom/focus, drag/drop de assets, edición de
polígonos de colisión e Inspector multi-entity con quick actions de assets/scripts/graphs.

El gate final exige build nativo, CTest, QML lint cuando está disponible, smoke del ABI/modelos y
contrato QML/C++. No queda una segunda UI visual Rust que mantener. Esto no congela el producto:
las capacidades nuevas siguen entrando por EditorCore/ABI o, cuando son puramente visuales, por
C++/QML.

## Superficie Tecnica Retirada En La Rama

- Targets Cargo `miniforge` y `miniforge_editor`.
- `src/main.rs`, `src/bin/miniforge_editor.rs` y `src/editor_app.rs`.
- Feature Cargo `editor`, reemplazada por `editor_core`.
- Dependencias `egui`, `egui_dock`, `egui_extras`, `egui-phosphor` y `rfd`.
- Docking, iconos, launcher y ciclo de ventana implementados con toolkit Rust.

`editor_ffi` depende ahora de `editor_core`, no de una UI. `scripts/check-no-rust-editor` impide reintroducir targets, features o dependencias visuales Rust y forma parte de `scripts/test-editor`.

## Matriz De Paridad

| Capacidad | Qt/C++/QML | Backend Rust | Estado / gate |
|---|---|---|---|
| Ventana, menus, docks, workspaces y persistencia de layout | Completo | Intenciones de panel frontend-neutral | Cubierto; Rust no debe volver a crear ventanas de editor |
| Abrir proyecto y recordar ultimo proyecto | Completo | `EditorCore::open_project` | Cubierto |
| Hierarchy, seleccion y visibilidad | Completo | Cache/ABI de entidades | Cubierto por smoke tests del bridge y modelos |
| Inspector de entidad/componentes | Completo | DTO y comandos de `EditorCore` | Cubierto |
| Content Browser y gestion de assets | Árbol, breadcrumbs, search/filter/sort, grid/list, thumbnails, selección múltiple, drag/drop, creación, editor de texto y metadata GUID/labels/dependencias/build | Rename/duplicate/move, import con manifests y spritesheet/frames inferidos, trash recuperable, creación tipada, lectura/escritura confinada y refresh de `AssetDatabase` | Cubierto extremo a extremo por EditorCore/ABI, dock y tests de operaciones seguras |
| Asset Dependency Graph | Panel y workspace Assets | Grafo real, build order, reverse counts, ciclos y referencias no resueltas | Cubierto por rebuild/read del bridge y smoke Qt |
| Viewport 2D, snapshot, grid, seleccion y multi-edit | Scene/Game, gizmos, box select, pan/zoom/focus, drop de assets y vértices de colisión editables | Snapshot, bounds, binding de assets, transform batch, spatial tools y undo atomico | Cubierto por test Rust, bridge smoke y model smoke |
| Command Palette, consola y readiness | Completo | Comandos, logs y system audit | Cubierto |
| Runtime Health, profiler y estabilidad | Runtime Health y Profiler dedicados con workspace propio | Diagnosticos/guard y muestras reales ordenadas por coste/presupuesto | Cubierto por dock, bridge y smoke Qt |
| Luau: tabs, editar, validar, guardar, recovery y debugger | Tabs, minimap, format, quick fixes, autocomplete/API, preferencias externas y debugger | Runtime, fuente `types/miniforge.luau`, diagnosticos, breakpoints, watches y session recovery | Cubierto por round-trip ABI, modelos/highlighter y recovery de buffers sucios |
| Sprite Studio: canvas, color, zoom, transforms, spritesheet y save | Widget nativo con grid, primary/secondary, flip/rotate/crop/outline, frame overlay, scrubber y playback | `SpriteEditorCanvas`/ABI y generación de clip | Cubierto |
| Build/export y reporte estructurado | Completo | `RuntimeExporter` | Cubierto |
| ForgeAI diagnostics y tests | Completo | ForgeAI host/context | Cubierto |
| Crear proyecto, templates, recientes y repair | `ProjectLauncherPanel.qml` y dock dedicado | `ProjectLauncherState` y `ProjectTemplates` | Cubierto por create/discover/repair y round-trip FFI |
| Visual graph/Blueprint authoring | Canvas de nodos, palette, links, inspector y save | Serializador, programming environment y runtime | Cubierto por validate/save/reopen desde Qt |
| Animation timeline/sequencer | Timeline editable y dock dedicado | Sesion con acciones, undo/redo y persistencia | Cubierto por test `sequencer_actions_undo_redo_and_persist` |
| Tilemap/terrain editor | Paint workflow y dock dedicado | Tilemap session, stroke, undo y persistencia | Cubierto por test `tilemap_stroke_undo_and_persist` |
| UI Designer | Jerarquia/canvas/propiedades y dock dedicado | UI session, acciones, undo, validacion y persistencia | Cubierto por test `ui_designer_actions_undo_validate_and_persist_canvas` |
| Project Settings/Input Map/Tags-Layers | `ProjectSettingsPanel.qml` y dock dedicado | Validacion y persistencia atomica | Cubierto por round-trip reopen y FFI |
| Project package y distributables | `ProjectOperationsPanel.qml`, menu Project/Build y dock dedicado | Export/import seguro `.mfpkg.zip`, package debug/release/shipping y exclusiones de artefactos/recovery | Cubierto por E2E Rust, ABI status-only y bridge smoke |
| Autosave y session controls | Panel con configure/save/recover/checkpoint/restore/clear | `AutosaveManager` y `SessionRecoveryManager` con escritura atomica/backup | Cubierto por recovery E2E y estado JSON |
| Play/Build externo | `QProcess` administrado por `MfBridge`, prepare/launch/stop | Plan backend con export/package, ejecutable, argumentos `--build` y warnings | Cubierto por build Qt y validacion del plan; no bloquea el proceso editor |
| Prefab authoring/overrides visuales | `PrefabStudioPanel.qml` y dock dedicado | Create/instantiate/apply/revert/variant/detach | Cubierto por workflow E2E y FFI |
| Session recovery y documentos multi-tab | Integrado en Luau Studio | Recovery/document manager y debugger | Cubierto por test de dirty buffers, tabs, breakpoints y watches |
| Scene Browser y ciclo de escenas | New/duplicate/save/restart/load/add/push/pop/unload | Scene manager, stack y persistencia | Cubierto por bridge y sesiones de escena |
| Object/gameplay authoring | Menús, toolbar y Object Studio para Node2D, sprites, cámara, Area/CharacterBody, UI, Player/Enemy/Resource y RTS | Comandos tipados de `EditorCore` | Cubierto por command registry/bridge |
| Workbench y CLI | 14 workspaces, layouts por usuario, reset, launcher/direct, runtime, headless, Safe Mode, create/overwrite y screenshot | Opciones de arranque y settings Qt | Cubierto por CLI help/offscreen y shell Qt |

La matriz no conserva gaps de migracion bloqueantes; las mejoras futuras son evolucion normal de producto sobre una unica UI Qt.

## Gates De Arquitectura

1. `scripts/check-no-rust-editor` debe pasar siempre.
2. `cargo check --locked --all-targets --all-features` debe probar `editor_core` y `editor_ffi` sin toolkit visual Rust.
3. `cargo check --locked --no-default-features --features runtime --all-targets` debe demostrar que runtime no depende del backend de editor.
4. `scripts/test-editor` debe construir bridge, editor Qt, smoke tests C++/modelos y QML lint.
5. `scripts/check-qt-backend-contract` debe detectar referencias QML sin método/propiedad C++.
6. Una capacidad marcada como gap solo puede eliminarse del backend Rust cuando exista operacion Qt equivalente, persistencia round-trip y prueba automatizada.
7. Nuevas capacidades de editor deben entrar como modelo/servicio frontend-neutral + ABI, o directamente como C++/QML si son puramente visuales.

## Evidencia Y Cobertura

- `asset_management_round_trips_import_sidecars_move_duplicate_and_trash`: cubre import externo, creacion y seguimiento de sidecar, rename, duplicate, move, trash con confirmacion, refresh del cache y rechazo de traversal.
- `profiler_and_dependency_graph_expose_sorted_real_engine_data`: inyecta mediciones reales del `Profiler`, valida orden/coste/presupuesto y reconstruye un grafo con ciclo de dos assets.
- `ffi_exposes_safe_asset_operations_profiler_and_dependency_graph`: prueba los cuatro contratos C nuevos, incluido el patron de buffers JSON y la operacion destructiva status-only.
- `qmllint -I editor-qml editor-qml/panels/AssetManagementPanel.qml editor-qml/panels/ProfilerPanel.qml editor-qml/panels/AssetDependencyGraphPanel.qml` pasa sin diagnosticos.
- `miniforge_editor_checks` construye editor, bridge, modelos, highlighter y QML.
- La configuración CMake registra cuatro tests base (CLI help, bridge, modelos y highlighter) y
  agrega QML lint como quinto test cuando `qmllint` está disponible.
- `project_operations_cover_packages_autosave_session_and_external_launch` prueba configure/save/recover de autosave, session checkpoint/restore/clear, export/import de proyecto y planes externos Play/Build.
- `ffi_project_operations_are_single_shot_and_publish_structured_state` valida que las mutaciones no se repiten para medir buffers y que el snapshot publica resultados/rutas.

La mutacion `mf_editor_manage_asset` es intencionalmente status-only: una accion destructiva se ejecuta exactamente una vez y nunca se repite para consultar el tamano de un buffer. Profiler y grafo son consultas JSON sin efectos; el rebuild del grafo tiene una llamada status-only separada.

`mf_editor_project_operation` sigue la misma regla status-only. El resultado se consulta despues con `mf_editor_project_operations_json`; `MfBridge` usa ese plan para iniciar/detener `miniforge_runtime` mediante `QProcess`, manteniendo el juego exportado aislado del editor.

## Comandos De Verificacion

```bash
scripts/check-no-rust-editor
scripts/check-qt-backend-contract
cargo check --locked --all-targets --all-features
cargo check --locked --no-default-features --features runtime --all-targets
cargo test --locked --features editor_core
scripts/test-editor
```

Para operacion diaria:

```bash
scripts/configure-editor
scripts/build-editor
scripts/run-editor projects/DefaultProject
```

## Qué Se Conserva En Rust

Retirar el editor visual Rust no significa retirar Rust del editor. Se conservan:

- `EditorCore` y servicios de proyecto/escena/assets;
- serializadores, validadores, undo/session data y pipelines;
- Luau y Visual Graph runtime;
- `editor_ffi` como contrato C;
- composición `Game` para autoría/Play Mode;
- runtime gráfico/headless y CLI.

Lo prohibido es recrear una segunda ventana/workbench con toolkit visual Rust o introducir una
feature `editor` ambigua. Una herramienta frontend-neutral nueva sí debe implementarse en Rust
cuando necesite compartir lógica, tests headless o persistencia con el runtime/editor.

Consulta el [índice de documentación](README.md) para la documentación canónica actual.
