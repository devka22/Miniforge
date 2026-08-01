<div align="center">

# MiniForge

### Motor 2D nativo con Rust, editor Qt 6/C++/QML y scripting Luau

MiniForge reúne edición visual, runtime independiente, física 2D, herramientas de contenido,
automatización y exportación en un solo proyecto extensible.

**Crate 0.9.3 · engine stream 0.9.3.4 · Rust edition 2024**

[Inicio rápido](#inicio-rápido) · [Editor](#editor-qt-moderno) · [Programación](#programación-con-luau) · [Arquitectura](#arquitectura) · [Documentación](#documentación)

</div>

---

## Qué es MiniForge

MiniForge es un motor experimental de juegos 2D escrito principalmente en Rust. Su interfaz de
autoría es un editor nativo construido con Qt 6, C++ y QML. El editor visual Rust/egui histórico
fue retirado: Qt es la única superficie de escritorio, mientras Rust conserva el runtime y el
backend de autoría independiente del frontend.

El proyecto incluye:

- editor visual con paneles acoplables, workspaces y layout persistente;
- Scene View y Game View con selección, gizmos y herramientas espaciales;
- jerarquía de entidades, inspector tipado, componentes, tags y layers;
- Content Browser con árbol de carpetas, miniaturas y operaciones reales sobre assets;
- Luau Studio con validación, autocompletado, API browser y debugger;
- Visual Graph, Sprite Studio, Animation Timeline, Tilemap/Terrain y UI Designer;
- prefabs, PackedScene2D, escenas aditivas y stack de escenas;
- runtime gráfico y headless separados del editor;
- Rapier2D, Kira, animación, partículas, UI runtime, navegación y sistemas RTS;
- perfiles de build, export, packaging, autosave, recovery y Safe Mode;
- profiler, readiness, dependency graph, ForgeAI y automatización Python/C#/MCP.

> MiniForge sigue en desarrollo activo. Los formatos versionados se migran o rechazan de forma
> explícita, pero las APIs públicas y algunos workflows todavía pueden cambiar entre releases.

## Novedades de la versión actual

### Port completo al editor Qt

- Se eliminaron `src/main.rs`, `src/editor_app.rs` y el binario visual
  `miniforge_editor` basado en Rust/egui.
- `scripts/check-no-rust-editor` impide reintroducir el target o sus dependencias visuales.
- `EditorCore` concentra los servicios de autoría y `editor_ffi` publica una ABI C estable para Qt.
- El editor de macOS se genera como `MiniForge.app`; Linux y Windows conservan el ejecutable
  `miniforge_qt_editor`.
- El workbench guarda geometría, docks y workspace por usuario y permite restablecer el layout.

### Workflow visual más profundo

- Menús File, Edit, Scene, Project, Build, Create, Tools, Workspace, View y Help con submenús y
  comandos contextuales.
- Workspaces especializados: 2D, Scripting, Animation, World, UI, Prefab, Project, Assets,
  Profiler, Automation, AI, Build, Debug y Minimal.
- Command Palette navegable con teclado y búsqueda fuzzy.
- Scene Browser para crear, duplicar, guardar, reiniciar, cargar, añadir, apilar, desapilar y
  descargar escenas.
- Project Launcher y Project Operations para templates, reparación, paquetes, autosave,
  checkpoints y Play/Build externo.

### Content Browser profesional

- Árbol de carpetas, breadcrumbs, navegación hacia arriba y búsqueda recursiva o por carpeta.
- Filtros por tipo, orden, modo grid/lista, tamaño, fecha y miniaturas de texturas.
- Selección múltiple con `Shift` y `Ctrl/Cmd`.
- Drag & drop entre carpetas administradas.
- Import, rename, duplicate, move y envío a trash recuperable.
- Creación de carpetas, Luau, Scene, Prefab, JSON, Resource Config, Material, Shader,
  Visual Graph, UI, Tilemap y SoundCue.
- Editor de texto integrado para documentos compatibles y apertura en el editor del sistema.
- Escrituras atómicas, límites de tamaño y confinamiento de rutas dentro del proyecto.

### Programación y herramientas de juego

- Luau Studio con tabs recuperables, syntax highlighting, diagnostics con línea/columna,
  completions, outline, snippets, find/replace y API browser.
- Debugger Luau callback-level con breakpoints, pause/resume/step y watches de solo lectura.
- Visual Graph con palette, canvas, nodos movibles, pins, links, inspector JSON y validación Rust.
- Starters de escena TopDown, Platformer y RTS; templates de proyecto RTS, Action RPG y Survival.
- Creación directa de Player Unit, Enemy AI, Resource Node, Command Center, Worker queue y
  Barracks desde el editor.

### Estabilidad y observabilidad

- `RuntimeStabilityGuard` sanea deltas y valores no finitos, puede aislar entidades corruptas y
  reduce la cadencia de sistemas opcionales bajo presión.
- Runtime Health expone nivel stable/guarded/recovery, frame budget, reparaciones, cuarentenas,
  límite de entidades y Safe Mode.
- Profiler por sistema, readiness por área, validator, dependency graph y reportes de build.
- Autosave de escena, checkpoints de sesión, recuperación de buffers y backups rotativos.

## Inicio rápido

### Requisitos

| Herramienta | Versión/uso |
|---|---|
| Rust + Cargo | Rust 1.95 o posterior; edition 2024 |
| CMake | 3.24 o posterior |
| Qt | Qt 6.5 o posterior con Widgets, Quick, QuickWidgets y QML |
| C/C++ toolchain | Xcode CLT, GCC/Clang o Visual Studio Build Tools |
| Git | Clonado y colaboración |

macOS:

```bash
xcode-select --install
brew install cmake qt
export CMAKE_PREFIX_PATH="$(brew --prefix qt)"
```

En Linux instala además los paquetes de desarrollo de Qt 6, audio, OpenGL/windowing y el
compilador C++ de tu distribución. En Windows usa Rust MSVC, CMake, Qt 6 y Visual Studio C++
Build Tools.

### Clonar y abrir el editor

```bash
git clone https://github.com/devka22/Miniforge.git
cd Miniforge
scripts/run-editor projects/DefaultProject
```

`scripts/run-editor` configura y compila incrementalmente antes de arrancar. Para separar pasos:

```bash
scripts/configure-editor
scripts/build-editor --parallel 4
scripts/run-editor projects/DefaultProject
```

Después del primer build también puedes ejecutar directamente:

```bash
# macOS
./build/editor-qt/MiniForge.app/Contents/MacOS/MiniForge projects/DefaultProject

# Linux/Windows desde un shell compatible
./build/editor-qt/miniforge_qt_editor projects/DefaultProject
```

### Opciones de arranque del editor

| Opción | Función |
|---|---|
| `--project <ruta>` o ruta posicional | Abrir un proyecto existente |
| `--launcher` / `--no-launcher` | Mostrar Project Launcher o abrir directamente |
| `--runtime` | Abrir el proyecto y entrar en Play Mode |
| `--headless-once` | Ejecutar un paso determinista sin ventana y terminar |
| `--create-project <ruta> --template <nombre>` | Crear y abrir un proyecto desde plantilla |
| `--force` / `--overwrite` | Permitir aplicar la plantilla dentro de un directorio existente |
| `--safe-mode` | Abrir en modo de recuperación sin scripts/graphs/plugins |
| `--workspace <nombre>` | Seleccionar workspace inicial |
| `--reset-layout` | Borrar layouts persistidos |
| `--screenshot <png>` | Capturar el workbench inicial y terminar |
| `--help`, `--version` | Ayuda y versión |

Ejemplos:

```bash
scripts/run-editor --project projects/DefaultProject --workspace Scripting
scripts/run-editor --project projects/DefaultProject --safe-mode
scripts/run-editor --launcher
scripts/run-editor --project projects/DefaultProject --runtime
scripts/run-editor --project projects/DefaultProject --headless-once
scripts/run-editor --create-project projects/MiPlataformas --template Platformer
scripts/run-editor --create-project projects/MiRTS --template RTS --force
scripts/run-editor --reset-layout projects/DefaultProject

QT_QPA_PLATFORM=offscreen scripts/run-editor \
  --project projects/DefaultProject \
  --screenshot /tmp/miniforge-editor.png
```

## Editor Qt moderno

### Workbench y navegación

La ventana principal usa `QMainWindow` y `QDockWidget`: el viewport ocupa el centro; Hierarchy se
sitúa a la izquierda; Inspector y herramientas de contexto a la derecha; Content Browser,
Console, Luau, Animation, Tilemap, UI y otros editores comparten la zona inferior. Los paneles se
pueden mover, tabificar, flotar, cerrar y recuperar desde `View > Panels`.

Atajos principales:

| Atajo | Acción |
|---|---|
| `Ctrl/Cmd+S` | Guardar escena o documento activo |
| `Ctrl/Cmd+Shift+S` | Guardar proyecto / Save All |
| `Ctrl/Cmd+Z` / `Ctrl/Cmd+Shift+Z` | Undo / Redo |
| `Ctrl/Cmd+Shift+P` | Command Palette |
| `Ctrl/Cmd+D` | Duplicar selección |
| `F2` | Renombrar selección/asset |
| `Delete` | Eliminar o mover a trash según contexto |
| `F5` / `Shift+F5` | Entrar/salir de Play Mode |
| `Q`, `W`, `E`, `R` | Select, Move, Rotate, Scale en Scene View |
| `F` / `Home` | Enfocar selección / restablecer cámara del viewport |

### Scene View, jerarquía e inspector

Scene View soporta:

- Scene/Game tabs;
- selección simple, aditiva, toggle y box selection;
- pan con botón medio, `Space` o `Alt`; zoom centrado en cursor;
- gizmos de mover, rotar y escalar;
- smart snap, grid/guides, HUD, camera frame y collision overlay;
- focus selection y reset view;
- align/distribute, group/ungroup y acciones de layer;
- duplicate, delete, reset transform, unparent y PackedScene2D desde menú contextual.

Hierarchy expone padre/hijos, tipo, tag, layer, visibilidad, enabled, lock y multiselección. El
reparent valida ciclos y la eliminación no deja referencias jerárquicas colgantes.

Inspector agrupa Identity, Transform y componentes; filtra propiedades, edita valores tipados,
aplica cambios comunes a selección múltiple en un solo paso de undo y permite añadir o quitar
componentes desde el registry.

### Escenas y prefabs

Las escenas viven normalmente en `saves/scenes/*.scene`. Scene Browser administra ciclo de vida,
loaded scenes, stack y transiciones. El formato conserva IDs, jerarquía, cámara, grid, tilemaps,
settings, entidades y `ui_canvases`.

Prefab Studio permite:

- crear un prefab desde la selección;
- crear variants;
- aplicar o revertir overrides;
- detach de una instancia;
- instanciar assets existentes;
- validar scripts/settings/dependencias requeridos;
- conservar GUID y usar guardado atómico con backup.

`scene.pack_selected` empaqueta una rama jerárquica como PackedScene2D y remapea IDs al instanciar.

### Content Browser y assets

El navegador no opera directamente sobre rutas arbitrarias. Las mutaciones se limitan a raíces
administradas como `assets`, `scripts`, `scenes`, `saves`, `settings`, `components`, `systems`,
`plugins` y `templates`; se bloquean traversal y symlinks peligrosos.

El Asset Database mantiene por recurso:

- GUID estable y ruta relativa;
- tipo, tamaño, fecha y hash;
- import settings y sidecar `.mfimport.json`;
- labels, compatibilidad y dependencias;
- reconciliación de movimientos por hash sin transferir el GUID a duplicados byte-idénticos.

Asset Management ofrece operaciones masivas seguras. Dependency Graph muestra aristas,
consumidores, ciclos, referencias sin resolver y orden de build. El borrado usa trash local
recuperable en lugar de una eliminación inmediata.

### Herramientas de autoría

| Herramienta | Capacidades principales |
|---|---|
| Sprite Studio | Pixel editing, colores primario/secundario, grid, zoom, pan, undo/redo, flip H/V, rotate, crop, outline y guardado PNG |
| Sprite sheet | Frame width/height, overlay, scrubber, FPS y preview Play/Pause |
| Animation Timeline | Tracks, keys, cursor, playback, tangents/curves, waveform WAV, undo/redo y save |
| Tilemap & Terrain | Capas, pencil/line/rect/fill, selección, copy/paste, terrain sets, probabilistic rule tiles y auto-tiling |
| UI Designer | Palette, hierarchy, canvas, anchors, transform, size, reparent, bindings, callbacks, validate y save |
| Visual Graph | Templates, node palette, pins, links, variables, inspector JSON, migration, validate y save |
| Project Settings | General, start scene, Input Map, Tags y Layers |
| Project Operations | Package export/import, distributables, autosave, session recovery y external Play/Build |
| Python Automation | Instalación de tools declarativos y ejecución confiable por manifest |
| Build & Export | Perfiles Debug, Release, Shipping y reporte de readiness/missing assets |

## Programación con Luau

Luau se ejecuta mediante `mlua` con Luau vendored. Los scripts encolan comandos y el motor los
aplica después del callback para evitar referencias Rust vivas dentro de la VM.

```lua
--!strict

local speed = 180

function on_ready()
    Debug.log("Player ready")
end

function on_update(dt: number)
    local axis = Input.get_axis("A", "D")
    Transform2D.translate(Entity.current(), axis * speed * dt, 0)
end
```

Namespaces principales:

- `Vector2`, `Input`, `Time`, `Layers`;
- `Entity`, `Transform2D`, `Component`;
- `Rigidbody2D`, `CharacterBody2D`, `Physics2D`;
- `Camera`, `AnimationPlayer`, `AnimatedSprite`;
- `Tilemap`, `Tween`, `Navigation2D`;
- `Audio2D`, `Particles2D`, `Spawner`;
- `Scene`, `Game`, `Events`, `Assets`, `Debug`.

El contrato de tipos para IDE vive en `types/miniforge.luau`. Luau Studio incluye:

- tabs y buffers dirty con recuperación en `.miniforge/qt_workspace.json`;
- syntax highlighter y diagnostics estructurados;
- outline de callbacks y funciones;
- completions contextuales, snippets y documentación de APIs;
- Find/Replace, Go to Line, comentar y duplicar línea;
- breakpoints por callback, pause/resume/step y watches punteados de solo lectura.

El debugger no evalúa expresiones Luau arbitrarias desde watches. Su granularidad actual es por
callback, no instrucción por instrucción.

Consulta [Datos, scripting y APIs](docs/DATOS_SCRIPTING_Y_APIS.md) para formatos, comandos y API.

## Motor y runtime

### Sistemas principales

- `RuntimeWorld`, `GameObject`, component registry y scene tree;
- física Rapier2D, colliders y eventos de contacto;
- movement, camera, render, animation, particles y audio Kira;
- UI runtime y vector canvas;
- visual scripting y Luau scheduler;
- pathfinding, spatial index, formation y navigation;
- RTS command queue, production, economy y building placement;
- gameplay, narrative, quests, abilities, status effects y inventory;
- world partition, object pools, spawn directors y save shards;
- profiler, diagnostics, crash reporter y runtime stability.

### Loop y estabilidad

El runtime valida y limita el delta antes de actualizar. Después ejecuta fixed steps, scripts,
graphs, sistemas de gameplay, movimiento, física, colisiones, cámara, world services, diagnósticos
y profiler en un orden definido. Los sistemas programados pueden declarar fase, prioridad,
dependencias, criticidad y budget.

`RuntimeStabilityGuard` puede:

- reemplazar deltas inválidos y limitar picos;
- reparar números no finitos antes de física/render/indexado;
- cuarentenar entidades muy corruptas sin cerrar el juego;
- detectar presión sobre `max_entities`;
- pasar de `stable` a `guarded` o `recovery`;
- espaciar sistemas opcionales mientras conserva sistemas críticos.

La configuración vive en `settings/runtime_config.json` bajo `stability_guard`.

### Render y plataformas

Macroquad sigue siendo el backend de exportación predeterminado. La migración WGPU ya dispone de
surface Metal/Vulkan/DX12/WebGPU, sprites/atlas y batching persistente, cinco modos de mezcla, texto
Unicode, UI 2D, materiales WGSL integrados, normal maps iluminados, partículas compute y cámaras a
texturas sampleables creadas sin código desde el editor. Aún se conserva Macroquad como fallback
hasta cerrar paridad de postproceso, shaders personalizados y partículas compute en render targets.

## Estructura de un proyecto

```text
MiJuego/
├── project.json
├── engine_config.json
├── manifest.json
├── assets/
│   ├── sprites/
│   ├── audio/
│   ├── data/
│   └── prefabs/
├── scripts/
│   └── visual_graphs/
├── saves/
│   ├── scenes/
│   └── autosave/
├── scenes/                 # compatibilidad con escenas raíz
├── settings/
│   ├── runtime_config.json
│   ├── input_map.json
│   ├── tags.json
│   ├── layers.json
│   ├── build_settings.json
│   └── build_profiles.json
├── components/
├── systems/
├── templates/
├── plugins/
├── logs/
├── builds/
└── .miniforge/
    ├── generated/
    ├── recovery/
    ├── trash/
    └── qt_workspace.json
```

`AssetTools::ensure_project_folders` crea la estructura y archivos que falten y limpia
temporales atómicos antiguos. Los archivos existentes no se sobrescriben silenciosamente.

## Templates y workflows

Templates disponibles en backend:

| Template | Contenido |
|---|---|
| Empty | Escena mínima |
| TopDown | Player controller, enemy logic, graph, input data y escena |
| Platformer | Motor, jump controller, graphs y escena |
| RTS | Mapa, camera/unit scripts, selection graphs, economy/production data y prefabs |
| Action RPG | Combate, enemy brain, quests, loot y prefabs de player/enemy/NPC |
| Survival | Day/night, crafting, resources, campfire, recipes y biome rules |
| Complete Demo | Menú, gameplay, scripts, graph, particles, material, audio, prefabs y save data |

Workflow recomendado:

1. Crear o abrir un proyecto.
2. Elegir workspace y validar `start_scene`.
3. Crear/cargar escena y entidades.
4. Importar assets y revisar GUID/dependencias.
5. Añadir componentes, Luau, Visual Graph, UI o tilemap.
6. Probar en Play Mode; revisar Console, Runtime Health y Profiler.
7. Guardar escena/proyecto y ejecutar Project Audit.
8. Exportar con perfil y probar el runtime aislado.

## Ejecutables y CLI

| Ejecutable | Responsabilidad |
|---|---|
| `MiniForge.app` / `miniforge_qt_editor` | Editor nativo Qt/C++/QML |
| `miniforge_runtime` | Player gráfico para una carpeta exportada |
| `miniforge_headless` | Simulación/validación determinista sin ventana |
| `miniforge_dev` | Doctor, auditoría, assets, export, automation y benchmarks |

### Runtime gráfico

```bash
cargo build --no-default-features --features runtime --bin miniforge_runtime
cargo run --no-default-features --features runtime --bin miniforge_runtime \
  -- --build path/to/export
```

Safe Mode del runtime:

```bash
cargo run --no-default-features --features runtime --bin miniforge_runtime \
  -- --build path/to/export --safe-mode
```

### Runtime headless

```bash
cargo run --no-default-features --features runtime --bin miniforge_headless \
  -- path/to/project 120
```

Devuelve JSON con pasos, tiempo simulado, entidades, validez del mundo, scripts, errores Luau,
visual graphs, animaciones y voces de audio. Sale con error si el mundo es inválido o Luau falla.

### CLI de desarrollo

El alias `cargo dev` ejecuta `miniforge_dev`:

```bash
cargo dev -- doctor
cargo dev -- project projects/DefaultProject --json
cargo dev -- assets projects/DefaultProject
cargo dev -- automation projects/DefaultProject --install-python
cargo dev -- scaffold-csharp-plugin projects/DefaultProject MiPlugin
cargo dev -- export projects/DefaultProject builds release
cargo dev -- bench --entities 5000 --queries 1000 --raycasts 500
cargo dev -- quick
cargo dev -- verify
cargo dev -- test
cargo dev -- docs
cargo dev -- ship
```

## Arquitectura

```text
Qt/QML panels
      │
      ▼
C++ MfBridge + Qt models
      │  include/miniforge_editor_bridge.h
      ▼
Rust editor_ffi
      │
      ▼
EditorCore ── project services / tools / validation
      │
      ├── Game (editor + Play Mode)
      └── EngineRuntime (runtime/export, sin UI de editor)
```

Organización principal:

```text
src/core/        Game y estructuras centrales
src/engine/      Servicios de motor y backend del editor
src/entities/    GameObject y datos de entidad
src/systems/     Sistemas de frame/runtime
src/runtime/     Composition root y player exportado
src/render/      Backends y foundations de render
editor-cpp/      QMainWindow, bridge, modelos y widgets nativos
editor-qml/      Paneles, componentes y tema
include/         ABI C pública del editor
types/           Declaraciones Luau
tools/           Automatización y API externa
```

Features de Cargo:

| Feature | Descripción |
|---|---|
| `runtime` | Runtime independiente sin backend de editor |
| `editor_core` | Servicios de autoría frontend-neutral; implica runtime |
| `editor_ffi` | ABI C para Qt; implica `editor_core` |
| default | `editor_core` |

El límite runtime-only se verifica con:

```bash
cargo check --locked --no-default-features --features runtime --all-targets
```

## Build, pruebas y distribución

Validación principal del editor:

```bash
scripts/test-editor
```

El script comprueba:

- ausencia del editor visual Rust retirado;
- formato de `editor_core.rs` y `editor_ffi.rs`;
- compilación y tests de `editor_core`/`editor_ffi`;
- build runtime-only;
- bridge Rust, editor Qt, modelos y syntax highlighter;
- CTest, QML lint y TypeScript typecheck cuando sus dependencias están instaladas.

Valida además que cada llamada/propiedad usada por QML esté enlazada en C++:

```bash
scripts/check-qt-backend-contract
```

Comandos Rust habituales:

```bash
cargo fmt --all --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features --all-targets
```

Build compacto del runtime:

```bash
cargo build --locked --profile ship \
  --no-default-features --features runtime --bin miniforge_runtime
```

Paquete del editor:

```bash
scripts/package-editor
```

## Estado y límites actuales

MiniForge es adecuado para aprendizaje, prototipos 2D, herramientas personalizadas, juegos
top-down/platformer/RTS y experimentación con arquitectura de motores. Aún no se recomienda para
proyectos críticos sin una fase propia de estabilización.

Límites conocidos:

- APIs y schemas todavía pueden evolucionar antes de 1.0;
- la exportación predeterminada sigue usando Macroquad mientras WGPU termina postproceso, shaders
  personalizados, golden images multiplataforma y partículas compute dentro de render targets;
- el debugger Luau es callback-level;
- la UI runtime conserva modelos legacy y canvas nuevos durante la migración;
- distribución firmada/notarizada y matrices completas de CI multiplataforma siguen pendientes;
- algunas herramientas avanzadas requieren más pruebas con proyectos grandes y assets reales.

## Documentación

- [Índice de documentación](docs/README.md)
- [Editor y flujo de uso](docs/EDITOR_Y_FLUJO_DE_USO.md)
- [Arquitectura y runtime](docs/ARQUITECTURA_Y_RUNTIME.md)
- [Datos, scripting y APIs](docs/DATOS_SCRIPTING_Y_APIS.md)
- [Desarrollo, build y extensión](docs/DESARROLLO_BUILD_Y_EXTENSION.md)
- [Migración definitiva al editor Qt](docs/QT_EDITOR_MIGRATION.md)
- [Backlog y feedback técnico](docs/MINIFORGE_MCP_FEEDBACK.md)

## Contribuir

1. Crea una rama enfocada.
2. Conserva el límite editor/runtime y evita dependencias visuales Rust.
3. Añade tests para comportamiento nuevo y migraciones de datos.
4. Ejecuta `scripts/test-editor` y los checks Rust relevantes.
5. Documenta comandos, schemas y cambios de workflow.

```bash
git checkout -b feature/mi-mejora
cargo fmt --all --check
cargo test --locked --all-features
scripts/test-editor
```

## Licencia y autor

MiniForge se distribuye bajo la licencia MIT. Consulta `LICENSE` para el texto completo.

Creado y mantenido por [devka22](https://github.com/devka22).

Repositorio: [github.com/devka22/Miniforge](https://github.com/devka22/Miniforge)

---

<div align="center">

**MiniForge — forge your own 2D worlds.**

</div>
