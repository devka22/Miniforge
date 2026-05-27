# MiniForge 0.9.1.1 Interface Overhaul Patch

MiniForge es un motor 2D con runtime/editor en Rust. La version 0.9.1.1 Interface Overhaul Patch es un mini parche enfocado solo en interfaz: redisenado visual del launcher, top bar, menus, paneles, paleta de comandos, ventanas flotantes, Hierarchy, Inspector, Browser y editores de codigo/blueprints para que el motor se sienta mas moderno, estetico y conectado.

## Ejecutar

Editor Rust (binario dedicado, recomendado):

```bash
cargo run --bin miniforge_editor -- --project projects/DefaultProject --no-launcher
```

Alias de compatibilidad (mismo editor que antes):

```bash
cargo run --bin miniforge -- --project projects/DefaultProject --no-launcher
```

Runtime player para **builds exportados** (sin UI de editor; requiere carpeta generada por Export Runtime):

```bash
cargo run --bin miniforge_runtime -- --build projects/DefaultProject/build/debug/DefaultProject
```

El runtime valida `runtime_manifest.json` y `build_info.json`, lista assets faltantes en stderr y carga el proyecto exportado en modo juego.

Editor con flags legacy (mismo binario `miniforge`):

```bash
cargo run -- --project projects/DefaultProject --no-launcher
```

Runtime Rust con ventana dentro del editor (modo `--runtime` del editor completo):

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher
```

Verificacion sin ventana:

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher --headless-once
```

## Rust Port

El motor vive en `src/`. El flujo nuevo usa assets `.mfgraph`, componentes Rust, sistemas Rust y manifests runtime Rust.

**Capas destacadas** (detalle en `docs/ENGINE_GUIDE.md` → *Mapa de capacidades*): runtime/editor Rust, Play Mode con snapshot y contador de frames en barra de estado, RTS (A*, flow fields, fog, influence), entidades por componentes, prefabs y overrides, visual graphs `.mfgraph`, Asset Database con GUID y preview, profiler/diagnostics, jerarquía + inspector, Macroquad, IA/NavAgent, tilemaps y pinceles.

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher
```

Pruebas del port:

```bash
cargo test
```

## Editor Avanzado

- Workspaces tipo editor profesional: World, Script, Prefab, Profile y Ship.
- Interfaz 0.9.1.1: fondo con profundidad, superficies consistentes, sombras suaves, bordes limpios, botones con gradiente y acentos modernos.
- Launcher estilo mac oscuro con panel principal, notas del parche, recientes y campos visualmente integrados.
- Paneles utiles: Scene, Game, Hierarchy, Inspector, Content Browser, Programming, Prefabs, Console, Profiler, Asset Graph, Build y Diagnostics.
- Ventanas flotantes movibles: Programming desacoplado para `.rhai` y `.mfgraph`, Blueprint Library buscable y Play Window embebida o separada.
- Jerarquia con click derecho para seleccionar, mover arriba/abajo, parentar la seleccion, limpiar parent y eliminar entidades.
- `F6` cambia workspace; `Ctrl+P` abre comandos; `Ctrl+G` crea un graph visual Rust; `Ctrl+I` instancia el primer prefab disponible; `Cmd/Ctrl+Z` y redo restauran operaciones del editor.
- Herramientas `Select`, `Move`, `Rotate`, `Scale` y `Paint`, con snap, bounding boxes y gizmos.
- Browser mejorado: indexa `assets/`, scripts `.rhai`, `scripts/visual_graphs/`, escenas y settings; marca compatibilidad, tamano, labels y visual graphs.
- Content Browser tiene Sources, busqueda, filtros, grid visual, preview de sprites/audio/materiales/prefabs, GUID, path, labels, import settings, dependencias, warnings y drag/drop hacia escena.
- Profiler mas accionable: tiempos por Movement, Animation, VisualGraph, Rhai, Gameplay, RTS, Physics, RhaiCollision y WorldSync.

## Developer Stability 0.8

- `engine_config.json` ahora tiene defaults versionados, migracion, backup `.bak` y recuperacion si el archivo esta corrupto.
- La consola escribe niveles `info`, `warning`, `error` y `debug` en `logs/miniforge.log`.
- El panel `Programming` abre y guarda `.rhai`, `.mfgraph`, `.scene`, `.prefab` y JSON sin reiniciar el motor.
- Content Browser ahora sigue un flujo tipo editor profesional: Sources, filtros por tipo, busqueda, grid de assets, detalles y acciones de abrir/instanciar.
- Los `.mfgraph` se editan como nodos conectables con pines de entrada/salida, manteniendo compatibilidad con el runtime de VisualScript.
- Los assets creados desde Content Browser/Programming se abren inmediatamente para edicion cuando aplica.
- Escenas y prefabs usan backups, validacion de referencias, guardado atomico y mensajes claros en consola.
- Rhai scripting y Visual Scripting reportan errores recuperables sin cerrar el editor.
- CI publica ejecuta `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings` y `cargo test`.

## Production Editor Base

- Inspector editable por campos reales: transform, stats, inventory, AI, RTS, dialogue, quest, tweens, tilemap collider y componentes custom basados en JSON.
- Animation Editor backend: clips con keyframes, timeline, preview, estados de Animator y transiciones por parametros.
- Particle System: componente `ParticleEmitter`, burst/loop, velocity/lifetime/size/color y preview estable en editor/runtime.
- Shader/Material 2D: materiales editables, shaders builtin `sprite_default` / `sprite_lit_fog`, lighting/fog flags y fallback de material.
- UI Runtime: canvas responsive, botones/paneles/labels/images, hover/click events y compatibilidad con `UIElement`.
- Script Debugger: errores runtime Rhai, trazas de funciones por linea, reload manual y panel de scripts activos.
- Add/Remove Component desde Inspector con validacion de tipos, componentes core protegidos y fallback seguro.
- Undo/redo con Command Pattern para mover/rotar/escalar entidades, editar inspector, crear, eliminar, duplicar, drop de assets y pintar tilemaps.
- Tile Palette con `Pencil`, `Eraser`, `Fill`, `Rect` y `Collision`, grid overlay y soporte de undo.
- Export runtime crea `build/<profile>/<project>/`, `runtime_manifest.json`, `build_info.json`, perfiles debug/release y warnings de assets faltantes.
- **Packaging**: menu `File > Package Debug/Release` genera `packages/game_<profile>/` copiando el export; define `MINIFORGE_RUNTIME` con la ruta al binario `miniforge_runtime` para incluir el ejecutable. Se evita copiar carpetas `build/`, `target/`, `exports/_pkg_work` dentro del paquete.
- **Guardado de escena**: backup `.scene.bak`, escritura atomica, `SceneSaveManager` con merge incremental de entidades sin cambios y flags de dirty en inspector/tilemap; campo `ui_canvases` en `.scene` para UI Canvas de escena.
- **UI Canvas de escena**: modelo con Panel/Button/Label/Image, anchors y preview responsive en Inspector cuando no hay entidad seleccionada; menu `Create > UI Canvas HUD/Label`.
- **Importadores**: `SpriteSheetImporter` (grid PNG + sidecar `.spritesheet.json`), `AtlasImporter` (JSON con regiones nombradas), `WaveformCache` para preview WAV en Asset Preview.
- **Autosave**: guardado atomico; `Game::recover_from_autosave()` y menu `File > Recover Autosave`; al abrir proyecto se valida estructura y se avisa si existe autosave.
- **Autosave reforzado**: backup `.bak`, health status y recuperacion desde backup si el archivo principal falla.
- InputMap visual incluye acciones `Move`, `Attack`, `Jump`, `Interact`, `Pause`, `Select`, `Command` y `CameraPan` con teclado, mouse y gamepad cuando aplica.

## Programacion Dentro Del Motor

El desarrollador puede crear logica sin tocar el codigo fuente del motor usando scripts `.rhai` y assets `.mfgraph`:

- Scripts Rhai por entidad: asigna `script = "PlayerController.rhai"` o agrega `{"runtime":"rhai","path":"PlayerController.rhai"}` en `scripts`.
- Eventos disponibles: `on_start()`, `on_update(dt)`, `on_key_down(key)`, `on_collision_enter(other)` y `on_destroy()`.
- API de gameplay: `move`, `set_position`, `spawn`, `destroy`, `play_sound`, `load_scene`, `input_pressed`, `ui_text` y `set_ui_text`.
- Hot reload: el runtime observa `scripts/` con `notify`, recompila cache y actualiza contadores en Profiler.

- Templates incluidos: `LogAndMove`, `PlayerVitalMovement`, `HealthCombat`, `ButtonClick`, `HealthPickup`, `RTSOrder`, `Spawner`, `BlueprintCommunication`, `InventoryEconomyLoop`, `QuestAbilityLoop` y `RTSProductionEconomy`.
- Los graphs se guardan en `scripts/visual_graphs/` y se ejecutan con `VisualScriptRuntime` en Rust.
- El panel `Programming` permite crear graphs, adjuntarlos a la entidad seleccionada, abrir scripts como codigo y editar visual graphs como nodos conectables.
- Los blueprints incluyen eventos, `ConstructionScript`, custom/broadcast events, `Sequence`, `Gate`, `DoOnce`, `FlipFlop`, ramas, variables, vida, movimiento, fisica, UI, inventario, economia, produccion RTS, habilidades, cooldowns, estados y quests.
- La paleta `Ctrl+P` tiene busqueda difusa para encontrar comandos aunque escribas incompleto: `inventry`, `econ`, `quest`, `rts production`, etc.
- Los templates de proyecto nuevos crean scripts Rhai, `.mfgraph`, prefabs y data JSON.

## Prefabs Y Escenas

- Prefabs avanzados con GUID estable, metadata, dependencias, instanciacion y variants.
- El inspector y el panel `Prefabs` permiten guardar la seleccion como prefab, crear variants, instanciar prefabs, aplicar cambios al source, revertir instancias y despegar instancias.
- Escena, prefabs y visual graphs aparecen juntos en el browser para acelerar el flujo de desarrollo.
- El panel `Scenes` lista escenas del proyecto, carga escenas normales/aditivas, empuja al stack y muestra estado runtime.
- El panel `Sprites` permite crear canvas 16/32, pintar pixeles, elegir paleta, flip horizontal/vertical, rotar y guardar PNG en `assets/sprites`.

## RTS Toolkit Rust

MiniForge ahora incluye una capa RTS en Rust lista para prototipos jugables:

- `RTSSystem`: actualiza economia, recoleccion, colas de produccion, construccion y fog of war.
- Componentes RTS: `RTSController`, `Commandable`, `Vision`, `FogOfWar`, `ProductionQueue`, `Buildable` y `ConstructionSite`.
- Ordenes: move, formation move, patrol, attack-move, gather, hold, stop y cancel.
- Formaciones: square, line, column, circle, staggered y wedge, con limpieza de slots bloqueados en grid.
- Pathfinding A* con suavizado de ruta y busqueda de tile caminable cercano.
- Flow fields para mover squads grandes hacia un mismo objetivo sin recalcular A* por unidad.
- Placement de edificios con footprint, clearance, busqueda de posicion valida y reserva en grid.
- Serializacion de comandos, rutas, patrol points y objetivos de unidades.

Desde el editor Rust:

- Boton `+Base`: crea un CommandCenter con wallet, produccion, vision y team.
- Boton `RTS Demo`: genera una escena skirmish con base, workers, recursos, enemigo, produccion y fog.
- Command Palette: `Create RTS skirmish scene`, `Create RTS template files`, `Queue worker on selected building`, `Place Barracks construction site`.

Crear archivos base para un proyecto RTS:

usa `Command Palette > Create RTS template files` o llama `game.create_project_template("RTS")`.

Demo completa de produccion:

```rust
game.create_project_template("complete_demo")?;
```

Genera menu, escena jugable, UI, audio events, save slot, RTS starter, scripts Rhai, particulas, materiales, shader lit/fog y prefabs demo.

O desde codigo Rust:

```rust
use miniforge::systems::rts_system::RTSSystem;

RTSSystem::enqueue_production(
    command_center,
    "Worker",
    "Worker",
    3.0,
    serde_json::json!({"Gold": 50.0}),
);
```

## Mejoras Core Para Juegos

- `SpatialIndex`: grid espacial reutilizable para seleccion, combate, triggers, queries por radio/rect y busqueda de entidad cercana.
- `GameClock`: reloj con fixed timestep, time scale, limite de steps por frame y proteccion contra spikes.
- `Diagnostics`: FPS, frame time actual, promedio, minimo, maximo y contador de frames.
- `EventBus`: ahora permite drenar todos los eventos, drenar por nombre y consultar contadores.
- `BuildPlacement`: validacion de construcciones sobre grid con footprints y preview JSON.
- `Runtime2DSystem`: controller top-down/platformer, jump buffer, coyote time, dash, colision contra grid/tilemap, checkpoints, respawn por caida, camera follow y stats de profiler.
- Savegame v2: `GameAPI::save_game_state`, `load_game_state` y `load_game_state_into` restauran transform, componentes, inventario, vida y entidades persistentes por `save_key`.
- GameAPI 0.9.2: `has_item`, `transfer_item`, `equip_item`, `can_afford`, `spend_cost`, `transfer_resource`, `add_production_recipe`, `enqueue_preferred_recipe`, `gather_resource`, `deposit_worker_cargo`, `set_quest_objective_progress`, `trigger_ability` y `recharge_ability`.

## Aprender MiniForge

- Guia completa 0.9.2: `docs/GETTING_STARTED_0.9.2.md`.
- Mapa tecnico del motor: `docs/ENGINE_GUIDE.md`.
- Arquitectura MiniForge2D inspirada en editores tipo UE: `docs/MINIFORGE_2D_ARCHITECTURE.md`.
- Matriz de cobertura de la guia UE4 2D: `docs/UE4_2D_GUIDE_IMPLEMENTATION_MATRIX.md`.
- Historial de cambios: `docs/PATCH_NOTES.md`.

APIs utiles:

```rust
use miniforge::engine::spatial_index::SpatialIndex;
use miniforge::engine::build_placement::{BuildFootprint, BuildPlacement};
use miniforge::map::flow_field::FlowField;

let mut index = SpatialIndex::new(4.0);
index.rebuild(&entities);
let nearest_enemy = index.nearest(x, y, 12.0, Some("Enemy"), Some("Units"));

let footprint = BuildFootprint { width: 2, height: 2, clearance: 1 };
let placement = BuildPlacement::find_nearest_valid(&grid, &entities, (10, 8), &footprint, 8, Some(1));

let flow = FlowField::build(&grid, (30, 20), 3000);
```

## Estructura de Proyecto

```text
MiniForgeProject/
├─ assets/
├─ scripts/
├─ scenes/
├─ saves/
│  ├─ scenes/
│  └─ autosave/
├─ logs/
├─ project.json
└─ engine_config.json
```

Si faltan carpetas o archivos base, el motor los crea automaticamente al abrir el proyecto.

## Escenas

- `F5`: alternar Play/Edit.
- `Cmd/Ctrl+S`: guardar escena actual.
- `Cmd/Ctrl+N`: crear escena nueva.
- `F9`: recuperar autosave.
- Consola: `save`, `load`, `new_scene`.

Las escenas se guardan como JSON en `saves/scenes/` e incluyen `scene_name`, `engine_version`, `entities`, `tiles`, `camera` y `settings`.

## Entidades

Cada entidad tiene ID unico, nombre, tipo, posicion, rotacion, escala, tamano, script asignado, estado activo y componentes. Puedes crear entidades desde Navigator, File Browser o consola:

```text
spawn player
spawn 10 10
delete selected
duplicate
```

## Inspector

El Inspector edita la entidad seleccionada con controles directos y campos de texto confirmados con Enter:

- nombre
- activo / visible / locked
- posicion X/Y
- rotacion
- escala
- tamano
- radio
- script
- tag / layer
- componentes
- stats, inventory, AI, RTS, dialogue, quest, tweens, audio y tilemap collider cuando existen

Si no hay seleccion muestra `No hay entidad seleccionada.`

Los botones `Add / Remove Component` usan el registro de componentes Rust y evitan quitar componentes core protegidos. Los cambios entran al historial de comandos, asi que `Cmd/Ctrl+Z` los revierte.

## File Browser

Trabaja con `assets/`, `scripts/`, `saves/scenes/`, `settings/`, `logs/` y carpetas de proyecto.

Funciones:

- crear `.mfgraph`
- crear carpetas
- renombrar archivos/carpetas
- duplicar assets
- eliminar con confirmacion
- refrescar
- abrir visual graphs con doble clic
- abrir escenas
- cambiar import settings
- ver dependencias del asset seleccionado
- reconstruir dependency graph

El borrado requiere confirmacion: la primera accion marca el borrado pendiente y la segunda confirma.

## Asset Pipeline

Cada asset recibe GUID persistente en `project/asset_metadata.json`. El motor tambien guarda import settings y dependencias para escenas, prefabs y data.

En 0.7 el panel `Asset Preview` muestra:

- preview visual para imagen, audio y material.
- GUID, path, labels, import settings y detalles de tamano.
- dependencias directas, reverse dependencies y warnings.
- toggle `include_in_build` y reconstruccion de dependency graph.
- drag/drop de sprites, prefabs, materiales, sonidos y visual graphs hacia Scene o entidad seleccionada.

Comandos utiles:

```text
asset graph
asset deps
asset import
```

`asset import` alterna opciones segun tipo: sprites cambian filtro, audio cambia streaming y otros assets alternan `include_in_build`.

## Graph Editor

Abre visual graphs desde File Browser o `F2`.

Funciones:

- New
- Save
- Validate
- Reload
- tabs
- validacion de JSON y nodos runtime
- snippets de nodos y plantillas

Plantilla base:

```json
{
  "kind": "MiniForgeVisualGraph",
  "runtime": "rust_visual_graph",
  "nodes": [
    {"id": "start", "type": "EventStart", "next": "log"},
    {"id": "log", "type": "Log", "message": "Hello", "next": null}
  ]
}
```

El runtime ejecuta los graphs con `VisualScriptRuntime`.

## Consola

Abrir input con la tecla de consola/backquote.

Comandos principales:

```text
help
clear
save
load
new_scene
reload
spawn player
delete selected
version
play
editor
validate
browser open
browser duplicate
browser delete
browser rename NuevoNombre
asset deps
asset import
asset graph
ui label Score
ui button Start
ui progress Health
visual log
visual button
plugin scan
plugin hook on_editor_start
example ui
example actionrpg
create graph PlayerController
component add Health
```

Los errores se muestran en consola y se guardan en `logs/error.log`.

## Play Mode

Play Mode crea un snapshot temporal de la escena. Al salir, restaura la escena original para que las pruebas no ensucien el trabajo del editor.

- `Play`: entra a Play Mode.
- `Stop`: vuelve a Editor Mode y restaura snapshot.
- `Pause`: pausa sistemas de gameplay.
- `F11`: pausa/reanuda.
- `F12`: reinicia Play Mode.

## Herramientas Visuales

La toolbar y menus activan funciones reales:

- `Move`: arrastra la entidad seleccionada en Scene View.
- `Rotate`: arrastra horizontalmente para rotar.
- `Scale`: arrastra para escalar.
- `Tools > Snap Size`: cambia el tamano de snap.
- `UI`: crea labels, botones y barras de progreso.
- `Visual`: aplica plantillas de visual scripting.
- `Plugins`: escanea plugins y ejecuta hooks.

## Plugins

Los plugins viven en `plugins/<nombre>/` o `packages/<nombre>/` y se declaran con `plugin.json`.

Hooks soportados:

```json
{
  "name": "hello",
  "enabled": true,
  "hooks": ["on_editor_start", "on_scene_saved", "on_asset_imported"]
}
```

## Problemas Comunes

- Falta una carpeta: abrir el proyecto repara la estructura.
- Una escena esta corrupta: se registra error y se crea respaldo `.corrupt_YYYYMMDD_HHMMSS`.
- Un graph no carga: revisar `logs/error.log` y el panel de errores del Graph Editor.
- File Browser no muestra cambios: usar `Refresh` o comando `reload`.

## Checklist Beta

- Editor abre sin errores.
- File Browser crea, renombra, duplica, abre y elimina con confirmacion.
- Graph Editor crea, edita, guarda, valida y recarga visual graphs.
- Escenas guardan/cargan entidades, tiles, camara y settings.
- Inspector edita propiedades basicas.
- Entidades tienen ID, nombre, posicion, rotacion, escala y tamano.
- Play Mode no modifica la escena original.
- Consola ejecuta comandos basicos.
- Errores pequenos no cierran el motor.
