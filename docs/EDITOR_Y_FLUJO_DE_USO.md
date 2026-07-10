# MiniForge - Editor y Flujo de Uso

Este documento consolida la guia de uso del editor, la migracion Qt/QML, el flujo de proyecto, las herramientas 2D y la operacion diaria del motor.

## Abrir El Motor

Comandos habituales desde la raiz del repositorio:

```bash
cargo run --bin miniforge_editor --features editor
cargo run --bin miniforge --features editor
scripts/run-editor projects/DefaultProject
```

El editor Qt/C++ se compila con:

```bash
scripts/configure-editor
scripts/build-editor
scripts/run-editor projects/DefaultProject
```

El runtime exportado se ejecuta con:

```bash
cargo run --bin miniforge_runtime --features runtime
```

Para tareas sin ventana:

```bash
cargo run --bin miniforge_headless --features runtime
cargo run --bin miniforge_dev --features editor -- doctor
cargo run --bin miniforge_dev --features editor -- project projects/DefaultProject
```

## Entradas Del Editor

MiniForge mantiene dos superficies principales:

- Editor Rust/macroquad/egui: ruta historica y de herramientas internas.
- Editor Qt/QML: shell moderno que usa C++ y el bridge FFI hacia `EditorCore`.

La ruta Qt vive en:

- `editor-cpp`: ventana, bridge C++ y modelos Qt.
- `editor-qml`: UI QML con tema oscuro, paneles y componentes.
- `include/miniforge_editor_bridge.h`: ABI C publica.
- `src/editor_ffi.rs`: implementacion Rust del ABI.
- `src/engine/editor_core.rs`: API de alto nivel usada por el bridge.

La ventana QML actual expone menu, top bar, tabs de workspace, hierarchy, command palette, viewport placeholder, inspector, content browser, consola y readiness.

## Conceptos Del Editor

Un proyecto MiniForge contiene:

- `project.json`: metadata del proyecto.
- `engine_config.json`: configuracion del motor/editor.
- `manifest.json`: resumen de runtime/assets/scripts.
- `assets/`: sprites, audio, data y prefabs.
- `scripts/`: Luau y visual graphs.
- `saves/scenes/`: escenas `.scene`.
- `settings/`: runtime, input, tags, layers, build settings y build profiles.
- `logs/`: logs del motor.
- `plugins/`: plugins de editor/proyecto.
- `builds/` y `exports/`: salidas de build/export.

`AssetTools::ensure_project_folders` crea la estructura base y archivos faltantes. Tambien limpia temporales viejos de escritura atomica.

## Flujo Base

1. Crear o abrir proyecto.
2. Verificar que `engine_config.json.start_scene` apunte a una escena existente.
3. Refrescar asset database.
4. Crear o abrir escena.
5. Crear entidades, sprites, prefabs, UI o tilemaps.
6. Agregar componentes desde el registry.
7. Adjuntar Luau o visual graph si aplica.
8. Usar Play Mode para probar.
9. Revisar consola, readiness y project validator.
10. Guardar escena/proyecto.
11. Exportar o empaquetar.

## Paneles Y Herramientas

Paneles principales:

- Hierarchy / World Outliner: muestra entidades, padres, hijos, seleccion, visibilidad y bloqueo.
- Inspector / Details: edita campos base y datos de componentes.
- Content Browser: lista assets con GUID, tipo, labels y dependencias.
- Scene View: viewport 2D con grid, zoom, pan, snap, seleccion, gizmos y overlays.
- Command Palette: busqueda fuzzy de comandos del editor.
- Console: logs por canal y severidad.
- Readiness: score de preparacion por sistema, gaps y acciones.
- Script Editor: documentos Luau, diagnostics, symbols, completions, minimap, search y code actions.
- Sprite Studio: pixel art, slicing, palette/ramp, atlas y export de spriteframes.
- Animation Editor: timeline, preview y animaciones.
- Problems Panel: issues de validacion 2D.
- UI Designer: jerarquia de widgets, canvas, seleccion, snap y preview.

El `EditorShell` coordina comandos de panel/document/runtime/plugin mediante un `EditorCommandBus`. Un panel puede pedir abrir, guardar, cerrar documento, mostrar/ocultar panel, reportar problemas o solicitar quit. La solicitud de quit solo se acepta desde `Shell`.

## Comandos De Editor

La capa Qt llama comandos por ID mediante `editorController.executeCommand(commandId)`. Ejemplos visibles en QML:

- `project.save`
- `scene.save`
- `scene.audit_tree`
- `scene.pack_selected`
- `edit.undo`
- `edit.redo`
- `project.audit`
- `assets.refresh`
- `luau.validate_scripts`
- `render.write_2d_profile`
- `sprite.new_pixel_art`
- `sprite.create_hero_template`
- `sprite.export_frames`
- `sprite.export_atlas_pages`
- `sprite.optimize_palette`
- `luau.new_controller`
- `object.create_node2d`
- `object.create_sprite_actor`
- `object.create_camera`
- `object.create_area2d`
- `object.create_character_body2d`
- `object.create_ui_text`

`EditorCore` tambien expone datos paginados para entidades, assets, comandos, consola, readiness y snapshots de viewport.

## Proyectos Y Templates

`ProjectTemplates` soporta:

- `empty`: escena vacia.
- `rts`: mapa RTS, scripts, graphs, economy/production data y prefabs base.
- `topdown`: controller, enemy brain, graph, bindings y escena topdown.
- `platformer`: motor, jump controller, graphs y escena.
- `demo` / `complete_demo` / `playable_demo`: menu, escena jugable, scripts, visual graph, particulas, shaders, materiales, audio, prefabs y save data.
- `action_rpg`: combate, enemigo, quests, loot, player/enemy/NPC prefabs.
- `survival`: day/night, crafting, recursos, campfire, recipes y biomas.

La recomendacion es empezar con `topdown`, `platformer` o `rts` si se quiere validar gameplay rapido.

## Crear Entidades

`GameObject::new` crea un objeto con:

- `Transform`
- `Selectable`
- `SpriteRenderer`
- `Collider2D`

Los comandos modernos de `EditorCore` para escena agregan ademas `Node2D` y `SceneTreeNode`, de modo que el objeto entra al indice de rutas/grupos/senales sin romper compatibilidad con escenas antiguas.

`GameObject::new_unit` crea una unidad con:

- `Transform`
- `Selectable`
- `RTSMovement`
- `SpriteRenderer`
- `Collider2D`

Luego se pueden agregar componentes desde `ComponentRegistry`. Para un jugador topdown completo, el motor ya tiene helpers de foundation que agregan componentes como `Health`, `Stats`, `Rigidbody2D`, `CharacterController2D`, `CameraFollow`, `Inventory`, `Equipment`, `Ability`, `QuestLog`, `Saveable`, `Cooldown`, `StatusEffects`, `Interaction`, `CombatTarget`, `NavAgent` y `VisualScript`.

## Escenas

Las escenas viven en `saves/scenes/*.scene`. El formato actual es:

- `format`: `miniforge.scene`
- `schema_version`: `1`
- `engine_version`
- `scene_name`
- `mode`
- `active_tool`
- `camera`
- `grid`
- `tiles` / `tilemap_layers`
- `settings`
- `entities`
- `ui_canvases`

Al cargar una escena, el editor:

- instancia `GameObject` con IDs preservados.
- asigna `scene_name`.
- sincroniza componentes.
- limpia seleccion.
- aplica grid, tilemaps, camara, herramientas y UI.
- reconstruye el mundo y toma snapshots de history.

El serializador migra escenas legacy con `objects` hacia `entities`. Versiones futuras se rechazan para evitar perdida silenciosa.

### SceneTree Y NodePath

El editor puede construir un `SceneTreeIndex` desde las entidades abiertas:

- rutas absolutas: `/World/Player/Camera`.
- rutas relativas: `../HUD`, `./WeaponSocket`.
- grupos: `GroupMembership.groups`, `editor_group`, `tag:*` y `layer:*`.
- auditoria: `scene.audit_tree` escribe resumen y warnings en consola.

Las claves `node_path`, `target_path`, `root_path`, `owner_path` y `parent_path` se validan al guardar. Si una conexion `SignalEmitter` apunta a un target inexistente o no define metodo, la validacion lo reporta antes de exportar.

### PackedScene2D

`scene.pack_selected` toma la entidad seleccionada como root, empaqueta root+hijos en `assets/packed_scenes/*.mpscene.json`, preserva la jerarquia y deja el asset index listo para usarlo desde content browser. Al instanciar, `PackedScene2D` remapea IDs y puede aplicar offset de posicion y prefijo de nombre.

## Prefabs

Los prefabs viven en `assets/prefabs/*.prefab`. El formato actual es:

- `format`: `miniforge.prefab`
- `schema_version`: `2`
- `guid`
- `prefab_name`
- `entity`
- `scripts.required`
- `settings.required`
- `dependencies`
- `metadata`

Al guardar un prefab:

- se preserva GUID si el archivo ya existe.
- se calcula `scripts.required` desde `script`, `scripts` o componentes de scripting.
- se agregan settings base: input map, tags, layers y runtime config.
- se escribe con `ProjectStorage::write_json_atomic_with_backup`.
- la entidad queda marcada como instancia con `prefab_source`, `prefab_guid` e `is_prefab_instance`.

Al instanciar un prefab, se carga el documento, se migra si aplica, se valida, se genera entidad con nuevo ID y se coloca en la escena.

## Assets

El `AssetDatabase` escanea:

- `assets/`
- `scripts/`
- `saves/scenes/`
- `settings/`
- `assets/prefabs/`

Cada asset obtiene:

- GUID estable.
- ruta relativa.
- nombre.
- tipo.
- tamano.
- fecha modificada.
- hash de contenido.
- import settings.
- labels.
- compatibilidad.
- dependencias.

Si un archivo se movio fuera del editor, el hash permite reconciliarlo y conservar su GUID. Las referencias deben usar GUID siempre que sea posible.

## Sprite Studio Y Pipeline 2D

El pipeline de assets 2D soporta:

- perfiles de importacion: Pixel Art, Smooth Sprite, UI Texture, Audio Event y Copy.
- sidecars `.mfimport.json`.
- fingerprints de fuente.
- jobs de reimport por fuente nueva, fuente cambiada, importer cambiado, generado faltante, dependencia cambiada o manual.
- sprite sheets con slices.
- atlas y paginas.
- waveform preview para WAV PCM simple.

Herramientas esperadas:

- slicing de spritesheet.
- export de `.spritesheet.json` y `.spriteframes`.
- atlas pages.
- palette ramp.
- previews de audio.

## Tilemaps Y Scene View

MiniForge tiene dos capas complementarias:

- `TilemapLayers`: grid runtime/editor de capas.
- `miniforge_2d::tilemap_editor2d`: seleccion, patrones, brushes, terrain rules, stamp brushes y object brushes.

Componentes relacionados:

- `Tilemap2D`
- `TilemapRenderer2D`
- `TilemapChunk2D`
- `Tileset2D`
- `TilemapCollider`

La capa `Collision` de tilemap alimenta el `Grid` para colisiones runtime 2D.

## UI

Hay dos modelos UI:

- Legacy por entidad: componente `UIElement`.
- Scene-level canvas: `ui_canvases` con `UiCanvasRoot`.
- MiniForge2D UI: `UiCanvas2D`, `ScreenManager2D`, widgets y focus navigation.

`UiRuntime` soporta layout, hover, click, focus, comandos y hit testing para canvas y UIElement legacy.

El modelo de `UiCanvasRoot` incluye `Panel`, `Button`, `Label` e `Image`. Cada elemento tiene `UiRect` con anchors, pivots, offsets y tamano.

## Scripting En El Editor

Los scripts Luau se editan en `scripts/*.luau` y se adjuntan por:

- `GameObject.script`
- `GameObject.scripts`
- `ScriptComponent`
- componentes con paths de script.

Visual graphs viven normalmente en `scripts/visual_graphs/*.mfgraph` y se adjuntan con `VisualScript` o `VisualGraphComponent`.

El editor valida Luau con `LuauScriptRuntime::validate_source` y visual graphs con `VisualGraphSerializer::try_migrate`.

## Play Mode

Play Mode usa los mismos sistemas que runtime, pero dentro de `Game`. El orden de sistemas respeta safe mode:

- scripts y graphs pueden desactivarse.
- plugins nativos pueden desactivarse.
- el mundo se sincroniza y reindexa despues de modificaciones.

`PlayModeManager` y `EditorHistory` guardan snapshots para volver al estado editor.

## Autosave Y Recovery

El editor tiene:

- `AutosaveManager` con intervalo por proyecto.
- `SceneSaveManager` para bootstrap y guardado.
- `SessionRecoveryManager` para sesiones del editor.
- `ProjectStorage` con backups rotativos.

Si existe `saves/autosave/autosave.scene`, el editor lo reporta al abrir el proyecto.

## Safe Mode

Safe Mode puede bloquear:

- scripts Luau.
- visual graphs.
- plugins nativos.

Se usa para abrir proyectos con fallos, evitar ejecucion insegura y aislar problemas.

## Readiness Y Auditoria

`ProjectValidator` revisa:

- carpetas base.
- `project.json`, `manifest.json`, `engine_config.json`.
- JSON invalido.
- Luau invalido.
- visual graphs incompatibles.
- escenas y prefabs legacy/futuros/corruptos.
- documentos 2D.
- plugins.
- GUIDs duplicados.
- referencias de assets.
- build settings.

`SystemAudit` produce score por area, fortalezas, gaps y acciones. La exportacion runtime incluye readiness score y acciones recomendadas.

## Automatizacion

El editor incluye:

- `AutomationBridge`: inspeccion de lenguajes, Python tools, plugins, render backend y recomendaciones.
- Python editor automation mediante `.mftool.json`.
- C# plugin scaffolding.
- MCP server en `mcp/miniforge`.
- ForgeAI para planificacion y ejecucion controlada por permisos.

## Flujos Recomendados

Topdown:

1. Crear template `topdown`.
2. Abrir escena `TopDown_Level`.
3. Confirmar player con `CharacterController2D`, `Rigidbody2D` y `CameraFollow`.
4. Agregar `EnemyBrain.luau` o visual graph.
5. Probar movimiento, colisiones, camara y UI.

Platformer:

1. Crear template `platformer`.
2. Configurar `CharacterController2D.mode = platformer`.
3. Activar gravedad en `Rigidbody2D`.
4. Agregar tiles de colision.
5. Probar coyote time, jump buffer, dash y respawn.

RTS:

1. Crear template `rts`.
2. Revisar `RTSController`, `Commandable`, `ProductionQueue`, `ResourceNode`, `Worker`.
3. Probar gather, production, rally point, fog y tactical combat.
4. Ajustar balance en `assets/data/RTSBalance.json`.

Demo completa:

1. Crear template `demo`.
2. Probar `Demo_Menu` y `Demo_Game`.
3. Validar scripts, particulas, audio, UI, save data y escena.

## Troubleshooting

- Si no abre una escena: revisar `engine_config.json.start_scene` y existencia en `saves/scenes`.
- Si un prefab no instancia: ejecutar `ProjectValidator`; revisar `scripts.required` y `settings.required`.
- Si un script no corre: revisar Safe Mode, errores de sintaxis Luau y que la entidad este activa.
- Si el editor no refleja assets: ejecutar `cargo run --bin miniforge_dev -- assets <proyecto>`.
- Si la camara no sigue: verificar `CameraFollow.target_id` y viewport settings.
- Si hay colisiones raras: revisar `Collider2D`, `Rigidbody2D`, `collision_layer`, `collision_mask`, one-way y tilemap `Collision`.
- Si el export falla: corregir errores de `ProjectValidator` antes de empaquetar.
