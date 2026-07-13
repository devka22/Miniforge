# MiniForge - Datos, scripting y APIs

Este documento consolida los contratos persistentes, Luau, Visual Graph y las APIs públicas del
motor. `types/miniforge.luau`, los serializadores y el header
`include/miniforge_editor_bridge.h` tienen prioridad si una descripción histórica discrepa.

## Versiones Y Contratos

Version actual del motor:

- crate Cargo: `0.9.3`
- `ENGINE_VERSION`: `0.9.3.4`
- `ENGINE_CODENAME`: `2D Workflow Foundations`
- `ENGINE_STREAM_VERSION`: `0.9.3.4`

Formatos versionados:

- Escenas: `miniforge.scene`, schema `1`.
- Prefabs: `miniforge.prefab`, schema `2`.
- Asset metadata: `miniforge.asset-metadata`, schema `1`.
- Visual graphs: migrados por `VisualGraphSerializer`.

Regla: un documento con schema futuro se rechaza. Un documento legacy se migra si el migrador conoce la ruta.

La versión del crate identifica el paquete Rust; `ENGINE_VERSION` identifica el stream de
documentos/runtime. No deben compararse como si fueran dos builds contradictorios.

## Estructura De Proyecto

Rutas canonicas calculadas por `AssetTools::get_project_paths`:

- `assets/`
- `assets/sprites/`
- `assets/audio/`
- `assets/data/`
- `assets/prefabs/`
- `scripts/`
- `scripts/visual_graphs/`
- `components/`
- `systems/`
- `saves/scenes/`
- `scenes/`
- `settings/`
- `logs/`
- `templates/`
- `plugins/`
- `builds/`

Archivos base:

- `project.json`
- `engine_config.json`
- `manifest.json`
- `settings/runtime_config.json`
- `settings/input_map.json`
- `settings/tags.json`
- `settings/layers.json`
- `settings/build_settings.json`
- `settings/build_profiles.json`
- `settings/prefab_settings.json`

El Content Browser permite leer y mutar contenido solo dentro de raíces administradas. Además de
las rutas canónicas anteriores, reconoce `saves`, `scenes`, `settings`, `components`, `systems`,
`plugins` y `templates`. Las rutas se normalizan contra la raíz del proyecto y las operaciones
rechazan traversal o enlaces que escapen de ella.

Tipos que puede crear directamente:

| Tipo de UI | Contrato/destino habitual |
|---|---|
| Folder | Directorio bajo la raíz administrada actual |
| Luau | Script `.luau` |
| Scene | Documento `miniforge.scene` |
| Prefab | Documento `miniforge.prefab` |
| JSON / Resource Config | Datos/configuración JSON |
| Material / Shader | Assets de render |
| Visual Graph | `.mfgraph` bajo `scripts/visual_graphs` |
| UI | Documento/canvas UI |
| Tilemap | Documento de mapa 2D |
| SoundCue | Configuración de evento de audio |

Las escrituras de documentos soportados son atómicas. Rename, duplicate, move, import y trash
refrescan Asset Database; un borrado desde el editor se envía a trash recuperable.

## Escenas

Una escena valida incluye:

```json
{
  "format": "miniforge.scene",
  "schema_version": 1,
  "engine_version": "0.9.3.4",
  "scene_name": "main",
  "mode": "EDITOR",
  "active_tool": "Select",
  "camera": {"x": 0, "y": 0, "zoom": 1.0},
  "grid": null,
  "tiles": [],
  "tilemap_layers": [],
  "settings": {},
  "entities": [],
  "editor_view_settings": {},
  "ui_canvases": []
}
```

Validaciones:

- raiz JSON object.
- `scene_name` string no vacio.
- `entities` array.
- `ui_canvases` array.
- `camera` object.
- `format` y `schema_version` compatibles.

Migracion:

- Schema `0` puede migrar `objects` a `entities`.
- Se agregan defaults de escena.
- La migracion es idempotente.

## Prefabs

Un prefab valido incluye:

```json
{
  "format": "miniforge.prefab",
  "schema_version": 2,
  "engine_version": "0.9.3.4",
  "guid": "prefab-guid",
  "prefab_name": "worker.prefab",
  "entity": {
    "name": "Worker",
    "components": []
  },
  "scripts": {
    "required": [],
    "embedded": [],
    "policy": "validate_on_instantiate"
  },
  "settings": {
    "required": [
      "settings/input_map.json",
      "settings/tags.json",
      "settings/layers.json",
      "settings/runtime_config.json"
    ],
    "defaults": {},
    "policy": "merge_missing"
  },
  "dependencies": [],
  "metadata": {
    "component_count": 0,
    "script_count": 0,
    "source": "prefab_pipeline"
  }
}
```

Validaciones:

- `entity` debe ser objeto.
- `entity.name` no puede estar vacio.
- `entity.components` debe ser array.
- `scripts.required`, `settings.required` y `dependencies` deben ser arrays de strings.

El serializador detecta scripts requeridos en:

- `entity.script`
- `entity.scripts`
- componentes cuyo tipo contenga `script`, `luau` o `rhai`.
- campos `path`, `script`, `script_path`, `source` o `file` con extension `.luau`, `.rhai` o `.mfgraph`.

## GameObject

`GameObject` es el documento de entidad. Campos persistentes importantes:

- `type`
- `id`
- `name`
- `enabled`
- `active`
- `visible`
- `locked`
- `x`, `y`, `position`
- `rotation`
- `scale`, `scale_x`, `scale_y`
- `size`, `width`, `height`
- `speed`
- `radius`
- `sprite_name`, `sprite_guid`
- `script`, `scripts`
- `tag`, `layer`
- `editor_group`
- `parent_id`, `local_x`, `local_y`
- `prefab_source`, `prefab_guid`, `is_prefab_instance`
- `scene_name`
- `state`, `command`, `path`
- `patrol_points`, `patrol_index`
- target IDs de follow/guard/attack/gather
- `components`

`sync_from_components` y `sync_to_components` mantienen compatibilidad entre campos legacy y componentes:

- `Transform` sincroniza posicion, rotacion y escala.
- `RTSMovement` sincroniza speed.
- `SpriteRenderer` sincroniza `sprite_name` y `sprite_guid`.
- `Collider2D` sincroniza radio, width y height.

## Componentes

Un componente tiene esta forma:

```json
{
  "component_type": "Transform",
  "enabled": true
}
```

Los datos adicionales se aplanan sobre el objeto. `default_component` crea defaults conocidos. `component_from_data` mezcla datos guardados sobre el default para conservar compatibilidad.

Familias del registry:

- Core: `Transform`, `Actor2D`.
- SceneTree: `Node2D`, `SceneTreeNode`, `GroupMembership`, `SignalEmitter`, `PackedSceneInstance`, `ResourceReference`.
- Rendering: `SpriteRenderer`, `Light2D`, `ShadowCaster2D`, `NormalMap2D`, `Material2D`, `ParallaxLayer`.
- Rendering3D: `Transform3D`, `MeshRenderer3D`, `Material3D`, `Billboard3D`, `HybridScene3D`.
- Camera: `Camera2D`, `Camera3D`, `CameraFollow`, `CameraShake`.
- Physics: `Rigidbody2D`, `Collider2D`, `Area2D`, `OneWayPlatform2D`, `Trigger2D`, `StaticBody2D`, `KinematicBody2D`, `CharacterBody2D`, `TilemapCollider`.
- Gameplay: `Health`, `Stats`, `Inventory`, `Equipment`, `Ability`, `Interaction`, `Lifetime`, `Spawner`, `LootTable`, `Checkpoint`, `CharacterController2D`, `EconomyWallet`.
- Combat: `DamageDealer`, `StatusEffects`, `CombatTarget`.
- RTS: `RTSController`, `Commandable`, `SquadMember`, `RtsBrain`, `ProductionRecipeBook`, `Vision`, `FogOfWar`, `ThreatSource`, `InfluenceSource`, `ProductionQueue`, `Buildable`, `ConstructionSite`.
- AI: `AIController`, `AIController2D`, `BehaviorTree2D`.
- Navigation: `NavAgent`.
- Paper2D: `TilemapRenderer2D`, `Tilemap2D`, `TilemapChunk2D`, `Tileset2D`, `FlipbookAnimation2D`.
- Animation: `Animator`, `Animator2D`, `AnimatedSprite`, `AnimationPlayer`, `AnimationBlueprint2D`.
- Scripting: `VisualScript`, `ScriptComponent`, `ScriptSchedule`, `VisualGraphComponent`, `Blackboard`, `InputActions2D`, `EventBus2D`, `StateMachine`, `Timer`, `Tween`.
- UI: `UIElement`, `WidgetCanvas2D`, `ObjectiveMarker`.
- Narrative: `QuestLog`, `Dialogue`.
- Effects: `Water2D`, `Distortion2D`, `Fire2D`, `Fog2D`, `Outline2D`, `Bloom2D`, `GpuParticles2D`, `DamageEffect2D`, `PixelArtShader2D`, `ParticleEmitter`.
- WorldStreaming: `WorldPartition2D`, `StreamingChunk2D`.
- MassiveGameplay: `ObjectPool2D`, `SpawnDirector2D`.
- Performance: `RuntimeBudget2D`.
- Persistence: `SaveShard2D`, `Saveable`, `DontDestroyOnLoad`.
- Grand Strategy: `Province2D`, `Nation2D`, `PopulationPops2D`, `Market2D`, `Factory2D`, `Diplomacy2D`, `ResearchTree2D`, `ArmyStack2D`, `WarGoal2D`, `TradeRoute2D`.

## Luau

El runtime Luau usa `mlua` con Luau vendored. La API evita entregar referencias Rust directas: los scripts encolan comandos y el motor los aplica despues del callback.

Limites actuales:

- presupuesto de interrupcion: `20_000`.
- memoria Lua: `32 MiB`.
- scheduler configurable en `settings/runtime_config.json`.
- hot reload mediante watcher.
- cache de bytecode.
- contextos persistentes por script/entidad.

Callbacks soportados:

- `on_create` / `on_start`
- `on_ready`
- `on_update(dt)`
- `on_fixed_update(dt)`
- `on_key_down(key)`
- `on_collision_enter(other)`
- `on_collision_exit(other)`
- `on_destroy`
- `on_event(name, payload)`

Un script puede declarar callbacks globales o devolver una tabla/módulo con métodos. Ejemplo
global mínimo y compatible con el contrato de tipos:

```lua
--!strict

local speed = 180

function on_ready()
    Debug.log("Player ready")
end

function on_update(dt: number)
    local direction = Input.get_axis("A", "D")
    Transform2D.translate(Entity.current(), direction * speed * dt, 0)
end
```

`Entity.current()` devuelve un handle al host del script. En scripts con tabla también está
disponible `self.entity`; los globals `entity`, `entity_id` y `entity_name` se conservan para
compatibilidad. Las mutaciones no deben esperar que el `RuntimeWorld` cambie en mitad del
callback: se convierten en `ScriptCommand` y se aplican al terminar.

### API Luau Por Namespace

| Namespace | Funciones principales |
|---|---|
| `Vector2` | construcción, length/normalize, add/sub/scale, dot, distance, lerp, move_towards |
| `Input` | pressed/is_pressed, get_axis/axis, action_pressed |
| `Time` | delta, fixed delta, tiempo y frame actuales |
| `Entity` | current, spawn, find/exists, nearby/nearest, tags, visibilidad, enabled y destroy |
| `Transform2D` | set_position y translate sobre un target |
| `Component` | add/remove/set/get/has con datos JSON-like |
| `Physics2D` | raycast, shape_cast y overlap_area con layers/triggers |
| `Rigidbody2D` | velocity e impulse |
| `CharacterBody2D` | input de movimiento/jump/run |
| `Camera` | main/current, follow, shake, zoom, límites, pixel-perfect y conversión screen/world |
| `AnimationPlayer` / `AnimatedSprite` | play y parámetros |
| `Tilemap` | get/set de tile por layer y coordenada |
| `Tween` | interpolación por property path |
| `Navigation2D` | destination de una entidad |
| `Audio2D` | play con bus, volumen y loop |
| `Particles2D` | burst |
| `Spawner` | spawn con ID estable reservado |
| `Scene` | carga de escena |
| `Game` | save/load slot y autosave |
| `Events` | eventos custom con payload |
| `Assets` | comprobar/resolver paths de asset |
| `Debug` | log, warn y error |
| `Task` | delay, defer y cancel ligados al contexto |

Un `EntityTarget` puede ser ID, nombre, proxy `EntityValue` o handle. Las queries de física
aceptan mask/layers y opción de triggers. Consulta `types/miniforge.luau` para firmas exactas: es
el archivo que deben consumir el editor, completions y herramientas externas.

`ScriptSchedulerConfig`:

- `enabled`
- `max_update_scripts_per_frame`
- `default_update_interval`
- `distant_update_interval`
- `budget_bypass_priority`
- `prioritize_by_distance`
- `open_world_auto_policy`

Si una entidad tiene `ScriptSchedule`, puede controlar frecuencia, prioridad y distancia. Cuando `open_world_auto_policy` esta activo, el runtime asigna defaults conservadores para entidades comunes de mundo abierto: jugador/directores siempre activos, policia y vehiculos con prioridad alta, peatones/pickups con intervalos mas largos y objetos de fondo con menor frecuencia. `prioritize_by_distance` ordena scripts de igual prioridad por cercania al jugador/camara antes de consumir el presupuesto de update.

El snapshot Luau mantiene indices por id, nombre, tag y un `SpatialIndex` interno. `Entity.find` y `Entity.exists` usan los indices por id/nombre. `Entity.nearby(origin, radius, options)` y `Entity.nearest(origin, radius, options)` usan broadphase espacial para consultas normales con tag/layer, incluyendo filtros multiples con deduplicacion. `Entity.nearest` excluye el origen por defecto; se puede desactivar con `exclude_origin = false`. Si se solicita `include_disabled = true`, las consultas espaciales caen a un scan lineal para preservar compatibilidad. `Entity.count_with_tag` permite contar entidades habilitadas sin construir proxies Luau.

APIs de productividad destacadas:

- `Vector2.add`, `sub`, `scale`, `dot`, `distance`, `lerp` y `move_towards`.
- `Component.add`, `remove`, `set`, `get` y `has`.
- `Spawner.spawn` y `Entity.spawn` reservan y devuelven un id estable inmediatamente; comandos posteriores del mismo callback pueden usarlo como target aunque el spawn se aplique al final del callback, incluso si ya existe otra entidad con el mismo nombre.
- `Camera.main()` es el nombre recomendado; `Camera.current()` se conserva como alias compatible.
- `Task.delay`, `Task.defer` y `Task.cancel` ofrecen timers ligados al contexto persistente del script.

Counters utiles del profiler:

- `LuauUpdateCandidates`, `LuauUpdateBudgetUsed`, `LuauSkippedBudget`, `LuauSkippedInterval`, `LuauDistanceThrottled`.
- `LuauNearbyQueries`, `LuauNearbyIndexed`, `LuauNearbyLinearScans`, `LuauNearbyCandidates`.

## Comandos Luau

`ScriptCommand` cubre:

- Transform: `Move`, `SetPosition`.
- Spawn/destruccion: `Spawn`, `SpawnConfigured`, `SpawnWithId`, `Destroy`.
- Audio: `PlaySound`.
- Escenas: `LoadScene`.
- UI: `SetUiText`, `SetUiProgress`, `SetUiVisible`.
- Entidad: `SetTag`, `SetLayer`, `SetEnabled`, `SetVisible`.
- Componentes: `SetComponentNumber`, `SetComponentText`, `SetComponentValue`, `AddComponent`, `RemoveComponent`.
- Fisica/control: `SetVelocity`, `ApplyImpulse`, `SetCharacterInput`.
- Camara: `SetCameraFollow`, `SetCameraShake`, `SetCameraPixelPerfect`.
- Animacion: `SetAnimation`, `SetAnimationParameter`.
- Tilemap: `SetTile`.
- Gameplay: `SetTween`, `SetNavDestination`, `ParticleBurst`, `SetSprite`, `SetSpriteAnimation`, `SetSpriteFlip`.
- Inventario/recursos: `AddItem`, `AddResource`.
- AI: `SetBlackboard`.
- Quest: `AddQuest`, `QuestProgress`, `CompleteQuest`.
- Persistencia: `SaveGame`, `LoadGame`.
- Ability: `TriggerAbility`, `RechargeAbility`.
- Eventos/debug: `EmitEvent`, `DebugLog`.

`LuauRunReport` reporta scripts ejecutados, comandos aplicados, spawns, destrucciones, sonidos, escenas solicitadas, UI updates, errores y mensajes debug.

`ScriptDebugSnapshot` agrega memoria usada por la VM, scripts ejecutados en el ultimo frame, contextos persistentes, scripts cacheados y snapshots de scheduler/queries. `LuauScriptRuntime::validate_source_diagnostics` devuelve errores estructurados con linea, columna y estado de entrada incompleta. Las declaraciones completas para el language server viven en `types/miniforge.luau`.

### Luau Studio

Luau Studio es la superficie Qt de edición y no una VM distinta. Abre varios tabs, conserva
buffers dirty, restaura la sesión y llama al mismo validator/runtime Rust usado por Play Mode.
Incluye:

- syntax highlighting nativo;
- diagnostics con línea, columna y estado de código incompleto;
- outline de callbacks y funciones;
- completions contextuales y API browser;
- templates/snippets alineados con callbacks reales;
- Find/Replace, Go to Line, comment y duplicate line;
- guardar, abrir en editor externo y navegación desde Content Browser;
- breakpoints, pause/resume/step y watches.

El estado recuperable se guarda atómicamente en `.miniforge/qt_workspace.json`: tabs, documento
activo, texto dirty, breakpoints y watches. Al reabrir, el buffer recuperado no se sustituye en
silencio por el archivo de disco. El usuario decide guardarlo o descartarlo.

Flujo recomendado:

1. Crear/abrir el script y validar antes de adjuntarlo.
2. Corregir diagnostics desde la lista de problemas.
3. Guardar y adjuntar el path con `GameObject.script`, `scripts` o `ScriptComponent`.
4. Entrar en Play Mode; revisar Console y profiler Luau.
5. Agregar breakpoints en la declaración de callbacks y watches punteados.

### Debugger Luau

El debugger integrado usa breakpoints reales a nivel de callback. Un breakpoint puede identificar `path + function` o la linea donde se declara el callback; el runtime pausa antes de ejecutar `on_ready`, `on_update`, `on_fixed_update`, `on_event` u otro handler soportado. `resume` ejecuta una vez el callback pausado sin volver a disparar el mismo breakpoint y `step` ejecuta ese callback y solicita pausa antes del siguiente callback elegible.

El frame pausado expone entidad, script, callback, evento, linea de declaracion, variables publicas de `self`, snapshot serializado de `entity` y `Time`. Los watches aceptan solamente identificadores punteados como `self.speed`, `entity.name` o `event.payload.quest`; no ejecutan codigo Luau arbitrario. La granularidad es callback-level, no instruccion-a-instruccion, por lo que una linea interna de un callback no se presenta falsamente como breakpoint ejecutable.

El ABI del editor expone estado, breakpoints, comandos y watches mediante `mf_editor_luau_debug_state_json`, `mf_editor_luau_set_breakpoints_json`, `mf_editor_luau_debug_command` y `mf_editor_luau_watches_json`.

Limitaciones deliberadas:

- no evalúa expresiones Luau arbitrarias desde watches;
- no detiene el VM en una instrucción interna;
- un breakpoint en una línea no ejecutable se asocia al callback declarado allí;
- `step` avanza un callback elegible, no una línea;
- Safe Mode impide ejecutar scripts aunque el documento se pueda abrir y validar.

### Visual Graph round-trip

El editor Qt valida y guarda `.mfgraph` a traves de `VisualGraphSerializer`: migra documentos legacy en memoria, aplica el header `miniforge.visual-graph`, rechaza versiones futuras y guarda JSON normalizado con backup atomico. El ABI correspondiente es `mf_editor_visual_graph_validate_json` y `mf_editor_visual_graph_save`.

## Eventos De Fisica En Luau

Cuando `PhysicsSystem` detecta enter/exit:

- Ejecuta `on_collision_enter` / `on_collision_exit` en ambas entidades.
- Emite eventos custom:
  - `physics_collision_enter`
  - `physics_collision_exit`
  - `physics_trigger_enter`
  - `physics_trigger_exit`

Payload:

```json
{
  "self_id": 1,
  "other_id": 2,
  "other_name": "Enemy",
  "pair_type": "collision",
  "phase": "enter",
  "normal": {"x": 0.0, "y": -1.0},
  "depth": 0.25
}
```

## Visual Graphs

`VisualScriptRuntime` ejecuta componentes `VisualScript` en modo `PLAY`, salvo que `run_in_editor` sea `true`.

El editor crea `.mfgraph` desde Content Browser o Visual Graph. El panel ofrece lista de graphs,
templates, palette, canvas, nodos movibles, pins/links, variables e inspector JSON. Antes de
guardar hace validación local de IDs/enlaces y validación final mediante
`VisualGraphSerializer`; los documentos legacy se migran en memoria, los futuros se rechazan y
el guardado normalizado usa backup atómico.

El flujo de ejecución sigue una arista `next` desde un entry node. El graph no debe depender de
la posición visual de los nodos para definir orden. IDs de nodo deben ser únicos y toda conexión
debe apuntar a un ID existente.

Eventos/entry nodes:

- `construction`
- `start`
- `update`
- `EventStart`
- `EventUpdate`
- `EventClick`
- `EventTrigger`
- `ConstructionScript`
- `CustomEvent`

Nodos implementados incluyen:

- Flujo: `CallEvent`, `BroadcastEvent`, `Sequence`, `DoOnce`, `ResetDoOnce`, `Gate`, `OpenGate`, `CloseGate`, `ToggleGate`, `FlipFlop`.
- Movimiento: `Move`, `MoveTowards`, `SetVelocity`, `AddForce`, `StopMovement`, `SetSpeed`, `SetPosition`, `SetRotation`, `SetScale`.
- Debug: `Log`.
- Gameplay: `Damage`, `Heal`, `SetHealth`, `BranchHealth`.
- El runtime tiene limite global de `4096` nodos por frame y limite de cadena de `128` nodos.

Las variables runtime internas se guardan en el componente, pero claves `_` no se persisten.

## ScriptHost2D

`ScriptHost2D` define una matriz cross-language:

- Luau: `BuiltIn`, adaptador `mlua-luau`, sandboxed, hot reload.
- Blueprint/Visual Graph: `BuiltIn`, adaptador `mfgraph`, sandboxed, hot reload.
- Python: `Available`, adaptador `miniforge-editor-tool-v1`, editor-only.
- C#: `Available`, adaptador `dotnet-plugin-manifest`, plugins/herramientas.

Lenguajes detectados:

- `.luau` y `.lua`: Luau.
- `.mfgraph`: Blueprint.
- `.py` y `.mftool.json`: Python.
- `.cs` y `.csproj`: C#.

Solo Luau y Blueprint se consideran runtime-safe por defecto.

## Python Automation

Las herramientas Python editor-only se describen con `.mftool.json`. El contrato general:

- discovery por proyecto.
- ejecucion confiable fuera del runtime.
- entrada/salida JSON.
- timeouts.
- operaciones validadas.

Herramientas incluidas en `tools/`:

- `scene_report.py`
- `project_health_matrix.py`
- `production_suite.py`
- import/export/bulk tools descritas por `.mftool.json`
- `documentation_generator.mftool.json`
- `batch_asset_import.mftool.json`

## UI APIs

Hay tres superficies UI:

1. `UIElement` como componente por entidad.
2. `UiCanvasRoot` dentro de `ui_canvases` de escena.
3. `miniforge_2d::ui_framework::UiCanvas2D` para UI mas avanzada.

`UiRuntime` entrega:

- layout por viewport.
- hover enter/exit.
- click.
- focus.
- comandos por widget.
- hit testing legacy.

Eventos:

- `HoverEnter`
- `HoverExit`
- `Click`

## Fisica APIs

`PhysicsSystem` expone:

- layer collision matrix.
- `PhysicsEvent` con pair type, phase, normal y depth.
- `RaycastHit`.
- `PhysicsQueryFilter`.
- `BoxCastQuery`.
- `CircleCastQuery`.
- `ShapeCastQuery`.

Tipos de pair:

- `Collision`
- `Trigger`

Fases:

- `Enter`
- `Stay`
- `Exit`

Componentes principales:

- `Rigidbody2D`
- `Collider2D`
- `Area2D`
- `Trigger2D`
- `StaticBody2D`
- `KinematicBody2D`
- `CharacterBody2D`
- `OneWayPlatform2D`

## Pathfinding Y Mapas

`map::grid::Grid` es la grilla base. `map::pathfinding` incluye:

- A*.
- heuristic.
- neighbors.
- reconstruccion de path.
- smoothing simple y con visibilidad.
- A* threat-aware.
- reportes de path query.
- distance map.
- influence map.

`map::flow_field::FlowField` se usa como base para RTS y movimiento masivo.

## Render APIs

`RenderBackend` es el contrato de backend. Comandos:

- `SpriteDrawCommand`
- `TilemapDrawCommand`
- `ParticleDrawCommand`
- `UiDrawCommand`
- `MeshDrawCommand3D`
- `LightDrawCommand3D`
- `CameraCommand3D`

Backends:

- `MacroquadBackend`: estable.
- `WgpuBackend`: experimental/futuro.

`Render2DCompatibilityProfile` describe compatibilidad, limites de atlas/batching, compute, GPU particles, tile compute culling, persistent buffers y fallbacks.

## Rust API Publica

Desde el crate:

```rust
use miniforge::EngineRuntime;
use miniforge::RuntimeWorld;
use miniforge::ENGINE_VERSION;
```

Cuando `editor_core` está activo:

```rust
use miniforge::Game;
```

El runtime headless:

```rust
use miniforge::runtime::game_runner::{run, run_with_options, RuntimeRunOptions};
```

## C ABI / Qt Bridge

El ABI de editor esta versionado por `MF_EDITOR_CORE_API_VERSION = 1`.

Tipos principales:

- `MfEditorHandle`
- `MfStatus`
- `MfError`
- `MfEntityRow`
- `MfInspectorField`
- `MfAssetRow`
- `MfCommandDescriptor`
- `MfConsoleEntry`
- `MfReadinessRow`
- `MfViewportInfo`

Funciones principales:

- crear/destruir editor.
- abrir proyecto.
- leer path de proyecto.
- listar entidades, seleccion, inspector, assets, comandos, consola y readiness.
- seleccionar entidad.
- editar inspector por JSON.
- ejecutar comando.
- tomar snapshot RGBA de viewport.
- administrar assets con una unica llamada status-only: `mf_editor_manage_asset` (`rename`, `duplicate`, `move`, `delete`, `import`).
- consultar telemetry estructurada con `mf_editor_profiler_snapshot_json`.
- reconstruir y consultar dependencias con `mf_editor_rebuild_asset_dependencies` y `mf_editor_asset_dependency_graph_json`.
- ejecutar package/autosave/session/external-build con `mf_editor_project_operation` y consultar su estado/plan con `mf_editor_project_operations_json`.

Superficies conectadas actualmente a Qt:

| Área | Contrato |
|---|---|
| Proyecto | open con opciones, launcher, create/repair, settings y operations |
| Mundo | jerarquía, selección, acciones de entidad, scene state y Scene Browser |
| Viewport | pick, snapshot/state, transform batch y overlays |
| Inspector | catálogo de componentes, campos single/multi y edición JSON |
| Herramientas | sesiones/action JSON para animation, tilemap, UI y otros tools |
| Prefabs | state y create/instantiate/apply/revert/variant/detach |
| Assets | listado, Content Browser, texto, import/move/duplicate/rename/trash |
| Observabilidad | console, readiness, profiler, dependency graph y Runtime Health |
| Luau | scripts, read/save/validate, API, debugger, breakpoints y watches |
| Visual Graph | catálogo, templates, validate/save |
| Automatización | Python tools, external editor y ForgeAI diagnostics/tests |
| Build | export y plan/launch/stop de proceso externo |
| Sprite | canvas RGBA, transacciones de edición, transforms, undo/redo y save |

`MfBridge` es el adaptador C++ invocable desde QML. Mantiene propiedades observables, traduce
colecciones C a modelos Qt y publica signals después de cada refresh/mutación. Los widgets nativos
de viewport y sprite usan el mismo bridge; no tienen un backend paralelo.

Reglas de memoria:

- El caller crea y destruye `MfEditorHandle`.
- Buffers C deben pasar capacidad.
- Las consultas JSON admiten el patron `BufferTooSmall` para medir y reintentar. `mf_editor_manage_asset` no devuelve buffer y nunca debe invocarse dos veces para una misma accion de UI.
- `mf_editor_project_operation` tambien es status-only; package, restore o prepare-build se ejecutan una vez y su resultado se lee despues desde el snapshot.
- Si no hay capacidad suficiente se retorna `BufferTooSmall` y `required`.
- Strings tienen capacidades fijas para filas comunes.

La regla de ownership es estricta: el caller conserva sus buffers, Rust conserva el handle y el
bridge no almacena punteros a memoria temporal. Los payloads JSON permiten ampliar operaciones
sin romper structs C existentes; cambios incompatibles requieren incrementar la versión ABI.

## Native Plugin ABI

El ABI nativo vive en `engine::native_library`:

- `MINIFORGE_NATIVE_ABI_VERSION = 1`.
- symbol requerido: `miniforge_native_entry_v1`.
- descriptor: `MiniForgeNativePluginV1`.
- callbacks: `initialize`, `shutdown`, `invoke_json`, `free_string`.
- host: `MiniForgeNativeHostV1`.

Los manifiestos `native.json` declaran:

- `id`
- `library`
- `enabled`
- `required`
- `abi_version`
- `category`
- `platforms`
- `services`

Safe Mode puede desactivar plugins nativos.

## Asset Metadata API

`AssetDatabase` mantiene `AssetRecord`:

- `guid`
- `relative_path`
- `name`
- `asset_type`
- `size_bytes`
- `modified_unix`
- `content_hash`
- `import_settings`
- `labels`
- `compatibility`
- `dependencies`

Funciones importantes:

- `scan`
- `save_metadata`
- `path_for_guid`
- `record_for_guid`
- `move_asset`
- `rebuild_dependency_graph`

## Runtime Manifest

La exportacion runtime escribe:

- `runtime_manifest.json`
- `build_info.json`

Incluye:

- engine version.
- profile.
- release optimization.
- used assets.
- missing assets.
- validation errors/warnings.
- backend plan.
- readiness score.
- source manifest.

`RuntimeManifestLoader` valida builds exportados y detecta assets faltantes.

## Compatibilidad Y Evolución

- No cambies un schema persistente sin incrementar `schema_version` o agregar migración.
- Rechaza documentos futuros; no elimines campos desconocidos mediante un guardado silencioso.
- Mantén `types/miniforge.luau` sincronizado con bindings, snippets y API browser.
- Conserva aliases de Luau solo cuando estén documentados como compatibilidad.
- Usa GUID para referencias durables de assets y path para interacción humana/importación.
- Mantén las mutaciones FFI como llamadas únicas y separa sus snapshots de resultado.
- Considera Python, C# y TypeScript editor-only salvo que un adapter runtime declare lo contrario.

Consulta el [índice de documentación](README.md), la guía del
[editor](EDITOR_Y_FLUJO_DE_USO.md) y la arquitectura del
[runtime](ARQUITECTURA_Y_RUNTIME.md).
