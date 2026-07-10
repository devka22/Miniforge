# MiniForge - Datos, Scripting y APIs

Este documento consolida los contratos de datos, formatos, scripting y APIs publicas del motor.

## Versiones Y Contratos

Version actual del motor:

- `ENGINE_VERSION`: `0.9.3.4`
- `ENGINE_CODENAME`: `2D Workflow Foundations`
- `ENGINE_STREAM_VERSION`: `0.9.3.4`

Formatos versionados:

- Escenas: `miniforge.scene`, schema `1`.
- Prefabs: `miniforge.prefab`, schema `2`.
- Asset metadata: `miniforge.asset-metadata`, schema `1`.
- Visual graphs: migrados por `VisualGraphSerializer`.

Regla: un documento con schema futuro se rechaza. Un documento legacy se migra si el migrador conoce la ruta.

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

`ScriptSchedulerConfig`:

- `enabled`
- `max_update_scripts_per_frame`
- `default_update_interval`
- `distant_update_interval`
- `budget_bypass_priority`

Si una entidad tiene `ScriptSchedule`, puede controlar frecuencia, prioridad y distancia.

## Comandos Luau

`ScriptCommand` cubre:

- Transform: `Move`, `SetPosition`.
- Spawn/destruccion: `Spawn`, `SpawnConfigured`, `Destroy`.
- Audio: `PlaySound`.
- Escenas: `LoadScene`.
- UI: `SetUiText`, `SetUiProgress`, `SetUiVisible`.
- Entidad: `SetTag`, `SetLayer`, `SetEnabled`, `SetVisible`.
- Componentes: `SetComponentNumber`, `SetComponentText`, `SetComponentValue`, `AddComponent`.
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

Cuando `editor` esta activo:

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

Reglas de memoria:

- El caller crea y destruye `MfEditorHandle`.
- Buffers C deben pasar capacidad.
- Si no hay capacidad suficiente se retorna `BufferTooSmall` y `required`.
- Strings tienen capacidades fijas para filas comunes.

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

