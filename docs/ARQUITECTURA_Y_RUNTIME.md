# MiniForge - Arquitectura y Runtime

Este documento consolida la arquitectura actual del motor despues de revisar `Cargo.toml`, `src/lib.rs`, `src/core/game.rs`, `src/runtime/engine_runtime.rs`, `src/engine/mod.rs`, `src/systems/mod.rs`, `src/entities/game_object.rs`, `src/engine/component.rs`, `src/engine/world.rs`, los serializadores y los modulos 2D principales.

Version documentada: `MiniForge 0.9.3.4 - 2D Workflow Foundations`.

## Vision General

MiniForge es un motor 2D escrito en Rust con editor, runtime exportable, scripting Luau, visual graphs, pipeline de assets, sistema de componentes JSON, soporte inicial hibrido 2D/3D y herramientas de automatizacion. El crate se organiza para separar el editor del runtime mediante features de Cargo:

- `runtime`: ruta de juego/exportacion sin servicios de editor.
- `editor`: activa egui, docking, file dialogs, editor UI, tooling y launcher.
- `editor_ffi`: expone el nucleo del editor a C/C++/Qt mediante ABI estable.

Los binarios principales son:

- `miniforge`: entrada principal del editor historico.
- `miniforge_editor`: editor con feature `editor`.
- `miniforge_runtime`: jugador runtime para builds exportados.
- `miniforge_headless`: ejecucion sin ventana para validacion o CI.
- `miniforge_dev`: CLI de desarrollo, diagnostico, export, benchmarks y automatizacion.

## Capas

La capa publica del crate esta en `src/lib.rs` y reexporta:

- `Game`, cuando `editor` esta activo.
- `EngineRuntime`, para runtime/export.
- `RuntimeWorld`, como propietario canonico de entidades.
- Etiquetas de version desde `engine::version`.

La arquitectura se divide en estas areas:

- `src/core`: composicion de alto nivel del editor, config base y utilidades de juego.
- `src/runtime`: composicion runtime sin dependencias de editor.
- `src/engine`: servicios compartidos, serializacion, assets, editor, scripting, 2D avanzado, build, plugins, UI y diagnosticos.
- `src/systems`: sistemas que actualizan entidades por frame.
- `src/entities`: `GameObject` y entidades legacy.
- `src/map` y `src/pathfinding`: grid, A*, flow fields, mapas de distancia e influencia.
- `src/render`: abstraccion de backend de render.
- `editor-cpp`, `editor-qml` e `include`: editor Qt/C++ sobre el bridge FFI.
- `mcp/miniforge`, `tools` y `scripts`: automatizacion externa, MCP y utilidades.

## Composicion Del Editor

`core::game::Game` es la raiz del editor. Contiene servicios de proyecto, escena y herramientas:

- `project_path`, `project_paths`, `engine_config`, `runtime_config`, `build_settings`, `build_profiles`.
- `runtime_world`, `grid`, `tilemap_layers`, `camera` y `scene_manager`.
- `asset_database`, `resources`, `input_map`, `tags_layers_manager`, `component_registry`.
- `scene_validator`, `scene_save_manager`, `autosave_manager`, `ProjectStorage` con backups atomicos.
- `history`, `selection`, `hierarchy`, `inspector`, `command_palette`, `docking_workspace`, `editor_workspace`.
- `script_editor`, `script_debugger`, `sprite_editor`, `animation_graphs`, `material_library`.
- `visual_script_runtime`, `luau_script_runtime`, `ui_runtime`, `audio_system`, `physics_system`, `particle_system`.
- `advanced_prefabs`, `archetypes`, `native_libraries`, `upgrade_manifest`, `programming`.

El editor puede abrir escenas, editar entidades/componentes, crear assets, ejecutar sistemas en modo editor o play, guardar de forma durable, exportar builds y ejecutar auditorias.

## Composicion Del Runtime

`runtime::engine_runtime::EngineRuntime` es la raiz de un juego exportado. Evita importar `core::game::Game` y servicios de editor. Esto hace que el limite editor/runtime sea verificable por features.

Servicios incluidos:

- Proyecto: `project_path`, `project_paths`, `engine_config`, `runtime_config`.
- Mundo: `RuntimeWorld`, `Grid`, `TilemapLayers`, `Camera`, `SceneManager`.
- Assets: `ResourceManager`, `AssetDatabase`.
- Seguridad: `SafeModeSettings`.
- Tiempo: `GameClock`.
- Diagnostico: `DeveloperConsole`, `Profiler`, `Diagnostics`.
- Sistemas: animacion, sprites, audio, visual graphs, Luau, gameplay, RTS, runtime 2D, fisica, particulas y narrativa.

Arranque runtime:

1. Crea carpetas necesarias de logs/settings.
2. Lee `engine_config.json` y `settings/runtime_config.json`.
3. Determina `start_scene`, por defecto `main.scene`.
4. Escanea recursos y abre `AssetDatabase`.
5. Configura grid, tilemap, camara y limites.
6. Crea `LuauScriptRuntime` con scheduler desde `runtime_config.script_scheduler`.
7. Carga la escena inicial y aplica entidades, tilemaps, grid, camara y `ui_canvases`.

## RuntimeWorld

`RuntimeWorld` reemplaza al modelo anterior de snapshot clonado. El mundo mantiene una unica lista de entidades:

- `units: Vec<GameObject>`: nombre legacy, pero es el vector canonico de entidades.
- `spatial_index`: indice espacial propio del motor.
- `structural_revision` e `indexed_revision`: detectan si el indice esta actualizado.

Operaciones importantes:

- `entity`, `entity_mut`, `push`, `remove`, `replace_entities`.
- `mark_changed`, `rebuild_index`, `index_is_current`.
- `query_radius` con filtros por tag/layer.
- `scene_tree`, `node_path_for`, `resolve_node_path`, `entities_in_group`.
- `signal_bus` para construir conexiones de senales desde componentes.
- `pack_scene_from_root` para empaquetar una rama de entidades como `PackedScene2D`.
- `validate`, que detecta IDs duplicados, padres colgantes y ciclos de jerarquia.

Regla actual: todo cambio estructural debe marcar el mundo como cambiado y reconstruir indice antes de consultas espaciales confiables.

El host Luau crea un snapshot por batch de eventos con indices por id, nombre y tag, mas un `SpatialIndex` propio. Esto evita scans completos cuando scripts de mundo abierto usan `Entity.find`, `Entity.all_with_tag` o `Entity.nearby` para trafico, policia, contactos, pickups y peatones.

## SceneTree, NodePath, Senales Y PackedScene2D

La capa nueva de escena toma ideas probadas de Godot y las adapta al modelo Rust/data-first de MiniForge:

- `engine::node_path::NodePath`: parsea rutas absolutas como `/Root/Camera` y relativas como `../HUD`. Normaliza `.` y `..`.
- `engine::scene_tree::SceneTreeIndex`: construye un indice desde `RuntimeWorld.units` con raices, hijos, paths estables, grupos y warnings de nombres duplicados o padres faltantes.
- `engine::scene_signal::SceneSignalBus`: lee componentes `SignalEmitter`, resuelve `target_id` o `target_path`, valida conexiones y produce `SceneSignalDispatch` para que Luau, visual graphs o runtime Rust ejecuten la accion.
- `engine::packed_scene::PackedScene2D`: empaqueta una rama root+hijos, remapea IDs al instanciar y conserva parenting/local transforms.

Componentes asociados:

- `Node2D`: metadata de nodo 2D, process mode, prioridad y descripcion de editor.
- `SceneTreeNode`: path/owner/estado de instancia para herramientas.
- `GroupMembership`: grupos persistentes adicionales a `tag:*`, `layer:*` y `editor_group`.
- `SignalEmitter`: lista de senales y conexiones serializables.
- `PackedSceneInstance`: vincula una instancia con su asset de escena empaquetada.
- `ResourceReference`: referencia GUID/path a recursos serializables.

`SceneValidator` y `ProjectValidator` validan ahora `NodePath` conocidos (`node_path`, `target_path`, `root_path`, `owner_path`, `parent_path`) y conexiones de `SignalEmitter`. Las conexiones sin target o sin metodo bloquean guardado/export cuando aparecen en contexto de escena.

## GameObject Y Component

`GameObject` es la entidad serializable principal. Incluye identidad, transform 2D, visuales, scripting, jerarquia, prefab metadata, comandos RTS y un vector de `Component`.

Campos relevantes:

- Identidad: `id`, `name`, `entity_type`, `tag`, `layer`.
- Estado: `enabled`, `active`, `visible`, `locked`, `selected`.
- Transform: `x`, `y`, `rotation`, `scale_x`, `scale_y`, `width`, `height`, `radius`.
- Assets/script: `sprite_name`, `sprite_guid`, `script`, `scripts`.
- Jerarquia: `parent_id`, `local_x`, `local_y`, `editor_group`.
- Prefab: `prefab_source`, `prefab_guid`, `is_prefab_instance`.
- RTS/movimiento: `state`, `command`, `path`, `patrol_points`, follow/guard/attack/gather targets.

`Component` es un objeto JSON tipado por `component_type`. Sus datos se guardan en un `BTreeMap<String, Value>`. Los metodos de componente encapsulan logica frecuente: fuerza fisica, vida, stats, inventario, equipo, habilidades, cooldowns, blackboard, state machine, quests, dialogo, status effects, economia, nav, camera shake, tween y timers.

Los valores con claves que empiezan con `_` son runtime-only y no se persisten en escenas, prefabs ni snapshots de undo.

## Familias De Componentes

Componentes base:

- `Transform`, `SpriteRenderer`, `Selectable`, `MovementComponent`, `AudioSource`, `Rigidbody2D`, `Collider2D`, `Animator`, `VisualScript`, `UIElement`.
- Escena/data model: `Node2D`, `SceneTreeNode`, `GroupMembership`, `SignalEmitter`, `PackedSceneInstance`, `ResourceReference`.

MiniForge2D y gameplay:

- Actor/game framework: `Actor2D`, `GameMode2D`, `GameState2D`, `PlayerState2D`, `Pawn2D`, `Controller2D`, `PlayerController2D`, `AIController2D`.
- Movimiento/jugador: `CharacterController2D`, `CameraFollow`, `CameraShake`, `Checkpoint`, `DontDestroyOnLoad`.
- Gameplay: `Health`, `Stats`, `Inventory`, `Equipment`, `Ability`, `Interaction`, `Lifetime`, `Spawner`, `LootTable`, `EconomyWallet`.
- AI y scripting: `AIController`, `BehaviorTree2D`, `Blackboard`, `StateMachine`, `InputActions2D`, `EventBus2D`, `Timer`, `Tween`.
- Narrativa: `QuestLog`, `Dialogue`.

RTS y gran estrategia:

- `RTSController`, `Commandable`, `SquadMember`, `RtsBrain`, `ProductionRecipeBook`, `ProductionQueue`, `Buildable`, `ConstructionSite`.
- `Worker`, `ResourceNode`, `Vision`, `FogOfWar`, `ThreatSource`, `InfluenceSource`.
- `Province2D`, `Nation2D`, `PopulationPops2D`, `Market2D`, `Factory2D`, `Diplomacy2D`, `ResearchTree2D`, `ArmyStack2D`, `WarGoal2D`, `TradeRoute2D`.

2D avanzado:

- Paper/tilemaps: `TilemapRenderer2D`, `Tilemap2D`, `TilemapChunk2D`, `Tileset2D`, `TilemapCollider`, `FlipbookAnimation2D`, `AnimatedSprite`.
- UI/cinematica: `WidgetCanvas2D`, `Sequencer2D`.
- Fisica: `StaticBody2D`, `KinematicBody2D`, `CharacterBody2D`, `Area2D`, `OneWayPlatform2D`, `Trigger2D`.
- Render/efectos: `Light2D`, `ShadowCaster2D`, `NormalMap2D`, `Water2D`, `Distortion2D`, `Fire2D`, `Fog2D`, `Outline2D`, `Bloom2D`, `GpuParticles2D`, `DamageEffect2D`, `PixelArtShader2D`, `Material2D`, `ParticleEmitter`, `ParallaxLayer`.

3D hibrido inicial:

- `Transform3D`, `MeshRenderer3D`, `Camera3D`, `Light3D`, `Material3D`, `Billboard3D`, `HybridScene3D`.

Mundo masivo:

- `WorldPartition2D`, `StreamingChunk2D`, `RuntimeBudget2D`, `ObjectPool2D`, `SpawnDirector2D`, `SaveShard2D`.

## Loop De Frame

Tanto `Game::run_headless_once` como `EngineRuntime::run_headless_once` siguen el mismo orden conceptual:

1. Avanzar consola, profiler y reloj.
2. `SpriteAnimationSystem`.
3. `AnimationSystem`.
4. `ParticleSystem`.
5. `AudioSystem`.
6. `VisualScriptRuntime`, si Safe Mode permite graphs.
7. `LuauScriptRuntime`, si Safe Mode permite scripts.
8. `Runtime2DSystem`.
9. `GameplaySystem`.
10. `RTSSystem`.
11. `MovementSystem`.
12. `PhysicsSystem`.
13. Colisiones tilemap/runtime 2D.
14. Camera shake y camera follow.
15. Dispatch de eventos de colision hacia Luau.
16. `RuntimeWorld.mark_changed` y `RuntimeWorld.rebuild_index`.
17. Diagnosticos, counters y metricas de profiler.

La version runtime registra tiempos por sistema con `Profiler` y metricas como entidades, scripts Luau, fixed ticks, frame budget, fixed step saturation, celdas espaciales, uso del scheduler Luau y consultas `Entity.nearby` indexadas/lineales.

## Sistemas

`PhysicsSystem` implementa cuerpos, gravedad, capas, colisiones y triggers. Usa broadphase R-tree desde `systems::spatial_index`, computa contactos, resuelve colisiones, emite eventos `Enter`, `Stay`, `Exit` y expone queries:

- `raycast`, `raycast_filtered`, `raycast_all_filtered`.
- `box_cast_filtered`, `circle_cast_filtered`, `shape_cast_all_filtered`.
- filtros por triggers y capas.

`Runtime2DSystem` cubre controladores topdown/plataforma, dash, coyote time, jump buffer, colisiones contra tilemap, respawn por caida, checkpoints, camera follow y camera shake.

`GameplaySystem` cubre cooldowns, timers, lifetimes, status effects, regen, state machines, tweens, nav agents, interacciones, spawners, AI basica, destruccion, respawn y loot.

`RTSSystem` cubre gather, construccion, production queues, auto queue, spawn de unidades, tactical combat, fog of war, equipos y recursos.

`AudioSystem` usa Kira cuando hay backend disponible, mantiene voces por entidad, buses, musica, SFX, fades, comandos y conversion de volumen a decibeles.

`ParticleSystem` simula emisores CPU con preview en editor, bursts, bounds y actualizacion paralela con Rayon cuando hay muchas particulas.

`NarrativeSystem` maneja interaccion y elecciones de dialogo.

`AnimationSystem` y `SpriteAnimationSystem` aplican clips, animator controllers y spriteframes.

## MiniForge2D

`engine::miniforge_2d` es una capa data-first inspirada en herramientas 2D profesionales. Todo debe poder serializarse como JSON, validarse sin ventana y adaptarse al sistema `GameObject`/`Component`.

El catalogo actual incluye:

- Editor layout, toolbar, content browser, details inspector, world outliner y scene view.
- Asset import pipeline 2D y acciones contextuales.
- Script host cross-language para Luau, visual graphs, Python editor-only y C# plugins.
- Actor Component System 2D.
- Game Framework 2D.
- Gameplay Tags y Ability System 2D.
- Blueprint Graph 2D y Blueprint Library.
- Paper2D-like: sprites, tilemaps, tilesets y flipbooks.
- Tilemap Editor 2D.
- Particles2D.
- Animation Blueprint 2D.
- UMG-like UI y UI Designer.
- Sequencer2D.
- Physics2D.
- AI Behavior Trees.
- Packaging.
- RTS Tools.
- Massive World 2D.
- Render hibrido 2D + 3D.

## Render

`src/render/backend.rs` define la interfaz de backend:

- `RenderBackend` con `begin_frame`, `end_frame`, `resize`, `draw_sprite`, `draw_tilemap`, `draw_particles`, `draw_ui`, `set_camera_3d`, `draw_mesh_3d`, `draw_light_3d`.
- `MacroquadBackend`: backend estable actual.
- `WgpuBackend`: ruta experimental/futura.
- `RenderBackendConfig`: batching, pixel perfect, culling, LOD, post process, 3D, Metal y compute flags.

`engine::render_2d` modela pipeline 2D:

- atlas dinamico y export de atlas.
- sprite batching, tilemap chunk renderer, materiales, shaders, render graph, post process.
- luces, sombras, normal maps, particulas, decals, trails, debug draw.
- plan Metal/compute para particulas, tile visibility, flow fields, lighting, post process y UI layout.

`engine::render_3d` modela el arranque hibrido:

- transform, mesh, material, camera, light, render graph y compatibilidad.
- Por defecto el 3D esta desactivado y el gameplay 2D sigue siendo la ruta estable.

## Persistencia Y Formatos

La persistencia compartida vive en `ProjectStorage`. Las escrituras son atomicas por destino:

- archivo temporal unico en el mismo directorio.
- flush/sync del temporal.
- reemplazo de destino.
- sync del directorio.
- backups rotativos opcionales.
- limpieza de temporales viejos.

Formatos principales:

- Escenas: `format = "miniforge.scene"`, `schema_version = 1`.
- Prefabs: `format = "miniforge.prefab"`, `schema_version = 2`.
- Asset metadata: `format = "miniforge.asset-metadata"`, `schema_version = 1`.

Cambios relevantes:

- Las escenas legacy migran `objects` a `entities`.
- Las escenas validan `scene_name`, `entities`, `ui_canvases` y `camera`.
- Los prefabs schema 2 guardan manifiestos de scripts/settings/dependencies.
- Versiones futuras de escenas/prefabs se rechazan de forma explicita.
- Prefabs y escenas usan backups para recuperacion.

## Diagnostico Y Seguridad

Servicios principales:

- `DeveloperConsole`: entradas por frame, canal y severidad, con log de archivo.
- `Profiler`: tiempos por sistema, counters y metricas.
- `Diagnostics`: presupuesto de frame, runtime health y datos de reloj.
- `CrashReporter`: instalable en runtime/game runner.
- `SafeModeSettings`: puede bloquear scripts, graphs y plugins nativos.
- `ProjectValidator`: carpetas, config, JSON, Luau, visual graphs, escenas, prefabs, 2D docs, plugins, GUIDs, referencias y build settings.
- `SystemAudit`: readiness del motor por area.

## Cambios Recientes Integrados

- Version visible unificada en `ENGINE_VERSION = 0.9.3.4`.
- `RuntimeWorld` es el propietario canonico de entidades y tiene indice espacial/revisiones.
- Escenas y prefabs tienen serializadores con validacion y rechazo de schemas futuros.
- Prefabs subieron a schema 2 con scripts/settings requeridos y metadata.
- `ProjectStorage` centraliza escritura atomica con backups rotativos.
- `AssetDatabase` conserva GUIDs y reconcilia assets movidos por hash de contenido.
- `EngineRuntime` existe como composition root runtime-only.
- `miniforge_2d` agrega catalogo amplio: UI Designer, Sequencer, Physics2D, AI, Packaging, RTS Tools, Massive World y render hibrido.
- Luau tiene scheduler configurable, hot reload, memoria limitada, cola de comandos, politicas automaticas de mundo abierto y consultas `nearby` aceleradas por indice espacial.
- Qt/QML puede operar sobre `EditorCore` via FFI C.
- Export/runtime manifest ahora incluye readiness, backend plan y assets faltantes.
