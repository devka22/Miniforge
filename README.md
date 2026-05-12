# MiniForge / Mini Forte 0.6.0 Beta

MiniForge es un motor 2D con runtime/editor principal en Rust y un arbol Python legado en retirada. La version 0.6.0 Beta estabiliza escenas JSON, entidades serializables, inspector editable, browser de assets, programacion visual dentro del motor, consola con comandos, proyectos reparables y Play Mode separado.

## Ejecutar

Editor Rust con ventana:

```bash
cargo run -- --project projects/DefaultProject --no-launcher
```

Runtime Rust con ventana:

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher
```

`main.py` ahora es solo un puente temporal hacia Rust y tambien abre la ventana:

```bash
python3 main.py --project projects/DefaultProject --no-launcher
```

Verificacion sin ventana:

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher --headless-once
```

Fallback temporal del motor Python legado:

```bash
MINIFORGE_LEGACY_PYTHON=1 python3 main.py --project projects/DefaultProject --no-launcher
```

## Rust Port

El motor principal vive en `src/`. El arbol Python queda como legado temporal y el flujo nuevo usa assets `.mfgraph`, componentes Rust, sistemas Rust y manifest con `legacy_python_scripts` separado.

```bash
cargo run -- --project projects/DefaultProject --runtime --no-launcher
```

Pruebas del port:

```bash
cargo test
```

## Editor Avanzado

- Workspaces tipo editor profesional: World, Script, Prefab, Profile y Ship.
- Paneles utiles: Scene, Game, Hierarchy, Inspector, Content Browser, Programming, Prefabs, Console, Profiler, Asset Graph, Build y Diagnostics.
- `F6` cambia workspace; `Ctrl+P` abre comandos; `Ctrl+G` crea un graph visual Rust; `Ctrl+I` instancia el primer prefab disponible.
- Browser mejorado: indexa `assets/`, `scripts/visual_graphs/`, escenas y settings; marca compatibilidad, tamano, labels, visual graphs y Python legacy.
- Profiler mas accionable: tiempos por Movement, Animation, VisualGraph, Gameplay, RTS, Physics y WorldSync.

## Programacion Dentro Del Motor

El desarrollador puede crear logica sin tocar el codigo fuente del motor usando assets `.mfgraph`:

- Templates incluidos: `LogAndMove`, `ButtonClick`, `HealthPickup`, `RTSOrder` y `Spawner`.
- Los graphs se guardan en `scripts/visual_graphs/` y se ejecutan con `VisualScriptRuntime` en Rust.
- El panel `Programming` permite crear graphs, adjuntarlos a la entidad seleccionada y ver validacion/runtime events.
- Los templates de proyecto nuevos ya priorizan `.mfgraph` y data JSON sobre scripts Python.

## Prefabs Y Escenas

- Prefabs avanzados con GUID estable, metadata, dependencias, instanciacion y variants.
- El inspector y el panel `Prefabs` permiten guardar la seleccion como prefab, crear variants e instanciar prefabs.
- Escena, prefabs y visual graphs aparecen juntos en el browser para acelerar el flujo de desarrollo.

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

El Inspector edita la entidad seleccionada:

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

Si no hay seleccion muestra `No hay entidad seleccionada.`

## File Browser

Trabaja con `assets/`, `scripts/`, `saves/scenes/`, `settings/`, `logs/` y carpetas de proyecto.

Funciones:

- crear `.py`
- crear carpetas
- renombrar archivos/carpetas
- duplicar assets
- eliminar con confirmacion
- refrescar
- abrir scripts con doble clic
- abrir escenas
- cambiar import settings
- ver dependencias del asset seleccionado
- reconstruir dependency graph

El borrado requiere confirmacion: la primera accion marca el borrado pendiente y la segunda confirma.

## Asset Pipeline

Cada asset recibe GUID persistente en `project/asset_metadata.json`. El motor tambien guarda import settings y dependencias para escenas, prefabs y data.

Comandos utiles:

```text
asset graph
asset deps
asset import
```

`asset import` alterna opciones segun tipo: sprites cambian filtro, audio cambia streaming y otros assets alternan `include_in_build`.

## Script Editor

Abre scripts desde File Browser o `F2`.

Funciones:

- New
- Save
- Run
- Reload
- tabs
- validacion simple de sintaxis
- snippets y autocompletado basico

Plantilla base:

```python
class NewScript:
    def start(self):
        pass

    def update(self, dt):
        pass
```

El runtime tambien mantiene compatibilidad con scripts antiguos `start(entity)` y `update(entity, dt)`.

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
create script PlayerController
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

Los plugins viven en `plugins/<nombre>/` o `packages/<nombre>/` y pueden incluir `plugin.json` y `plugin.py`.

Hooks soportados:

```python
def on_editor_start(game):
    game.console.log("Plugin activo", "ENGINE")

def on_scene_saved(game):
    pass

def on_asset_imported(game):
    pass
```

## Problemas Comunes

- Falta una carpeta: abrir el proyecto repara la estructura.
- Una escena esta corrupta: se registra error y se crea respaldo `.corrupt_YYYYMMDD_HHMMSS`.
- Un script no carga: revisar `logs/error.log` y el panel de errores del Script Editor.
- File Browser no muestra cambios: usar `Refresh` o comando `reload`.

## Checklist Beta

- Editor abre sin errores.
- File Browser crea, renombra, duplica, abre y elimina con confirmacion.
- Script Editor crea, edita, guarda, ejecuta y recarga scripts.
- Escenas guardan/cargan entidades, tiles, camara y settings.
- Inspector edita propiedades basicas.
- Entidades tienen ID, nombre, posicion, rotacion, escala y tamano.
- Play Mode no modifica la escena original.
- Consola ejecuta comandos basicos.
- Errores pequenos no cierran el motor.
