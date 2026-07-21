# MiniForge - Desarrollo, build y extensión

Esta guía cubre toolchain, build, pruebas, exportación, packaging y extensiones. Para la
arquitectura consulta [Arquitectura y runtime](ARQUITECTURA_Y_RUNTIME.md); para operar la
aplicación consulta [Editor y flujo de uso](EDITOR_Y_FLUJO_DE_USO.md).

## Requisitos

El crate declara:

- Rust edition `2024`.
- `rust-version = 1.95`.
- paquete `miniforge`.
- version Cargo `0.9.3`.
- version runtime visible `0.9.3.4`.

Toolchain mínima:

| Herramienta | Requisito |
|---|---|
| Rust/Cargo | Rust `1.95+`, edition 2024 |
| CMake | `3.24+` |
| C++ | C++20 |
| Qt | Qt `6.5+`: Widgets, Quick, QuickWidgets y QML |
| Git | Repositorio y colaboración |
| Node/npm | Opcional; typecheck de plugins TypeScript |
| Python | Opcional; automation tools de proyecto |

En macOS instala Xcode Command Line Tools, CMake y Qt 6. Si CMake no localiza Qt, exporta
`CMAKE_PREFIX_PATH` con el prefijo de Qt. En Windows usa Rust MSVC y Visual Studio C++ Build
Tools. En Linux añade los development packages de Qt, audio, OpenGL/windowing y un compilador
C++ de la distribución.

Dependencias relevantes:

- Editor nativo: Qt 6/C++/QML en `editor-cpp` y `editor-qml`.
- Backend de editor: `EditorCore`, C ABI, `arboard`, `syntect` y servicios de filesystem sin toolkit visual Rust.
- Runtime/render: `macroquad`, `image`, `guillotiere`.
- Audio: `kira`.
- Fisica: `rapier2d`.
- Scripting: `mlua` con features `luau` y `vendored`.
- Assets/fs: `walkdir`, `notify`, `zip`, `trash`, `open`.
- Datos: `serde`, `serde_json`.
- Performance: `rayon`, `rstar`, `sysinfo`, `crossbeam-channel`.
- Graphs: `petgraph`.
- SVG/vector: `lyon`, `resvg`, `usvg`.

## Features De Cargo

```toml
default = ["editor_core"]
runtime = []
editor_ffi = ["editor_core"]
editor_core = ["runtime", ...]
```

Reglas:

- Codigo runtime debe compilar sin feature `editor_core`.
- Servicios no visuales del editor deben estar bajo `#[cfg(feature = "editor_core")]`.
- FFI del editor depende de `editor_core` y no incorpora un toolkit UI Rust.
- Export runtime usa `EngineRuntime`, no `Game`.

## Binaries

```bash
cargo run --no-default-features --features runtime --bin miniforge_runtime \
  -- --build path/to/export
cargo run --no-default-features --features runtime --bin miniforge_headless \
  -- projects/DefaultProject 120
cargo dev -- help
```

`miniforge_runtime` abre el jugador runtime con macroquad.

`miniforge_headless` simula el proyecto sin ventana y devuelve un reporte JSON. Su estado de
salida es no-cero si el mundo es inválido o si Luau falla, por lo que sirve para CI.

`miniforge_dev` contiene herramientas de desarrollo:

- `doctor`
- `project`
- `assets`
- `automation`
- `scaffold-csharp-plugin` / `csharp-plugin`
- `export`
- `bench` / `benchmark` / `benchmarks`
- workflows internos desde `development_workflow`.

## Scripts

Scripts principales:

```bash
scripts/configure-editor
scripts/build-editor
scripts/run-editor projects/DefaultProject
scripts/test-editor
scripts/package-editor
```

`scripts/run-editor` compila el editor Qt si hace falta y abre el proyecto indicado. Por defecto usa `projects/DefaultProject`.

Responsabilidad de cada script:

| Script | Responsabilidad |
|---|---|
| `configure-editor` | Detectar CMake/Qt y generar `build/editor-qt` |
| `build-editor` | Configurar y compilar incrementalmente |
| `run-editor` | Compilar y ejecutar el bundle/binario con argumentos intactos |
| `test-editor` | Gates Rust, runtime-only, CMake, CTest, QML y TypeScript opcional |
| `package-editor` | Construir el target CMake `package` |
| `check-no-rust-editor` | Impedir que vuelva el target/editor visual Rust retirado |
| `check-qt-backend-contract` | Verificar que métodos/propiedades usados por QML existan en C++ |

Opciones útiles del ejecutable:

```bash
scripts/run-editor --project projects/DefaultProject --workspace Scripting
scripts/run-editor --project projects/DefaultProject --safe-mode
scripts/run-editor --launcher
scripts/run-editor --project projects/DefaultProject --runtime
scripts/run-editor --project projects/DefaultProject --headless-once
scripts/run-editor --create-project projects/MiJuego --template TopDown
scripts/run-editor --create-project projects/MiRTS --template RTS --force
scripts/run-editor --reset-layout projects/DefaultProject
QT_QPA_PLATFORM=offscreen scripts/run-editor \
  --project projects/DefaultProject --screenshot /tmp/miniforge.png
```

Variables utiles:

- `MINIFORGE_QT_BUILD_DIR`
- `MINIFORGE_QT_CMAKE`
- `MINIFORGE_RUNTIME`
- `MINIFORGE_ENGINE_ROOT`

Variables CMake relevantes:

- `MINIFORGE_EDITOR_RUST_PROFILE=debug|release` controla el bridge Rust enlazado.
- `MINIFORGE_EDITOR_BUILD_APP=ON|OFF` controla la aplicación.
- `MINIFORGE_EDITOR_BUILD_TESTS=ON|OFF` controla los smoke tests.
- `MINIFORGE_EDITOR_ENABLE_QML_LINT=ON|OFF` controla el target de lint si existe.
- `MINIFORGE_DEFAULT_PROJECT` selecciona el fixture usado por los smoke tests.

## Desarrollo Diario

Flujo recomendado:

1. `cargo fmt --all --check`
2. `cargo check --locked --all-targets --all-features`
3. `cargo check --locked --no-default-features --features runtime --all-targets`
4. `cargo test --locked --features editor_core`
5. `cargo dev -- doctor`
6. `cargo dev -- project projects/DefaultProject`
7. Probar editor o runtime según la tarea.

Para cambios de runtime, ejecutar tambien:

```bash
cargo dev -- export projects/DefaultProject builds debug
```

Para cambios en la capa de escena inspirada en Godot, cubrir al menos:

```bash
cargo check --locked --features editor_core
cargo check --locked --no-default-features --features runtime --all-targets
cargo test --locked --features editor_core --lib
```

Archivos principales:

- `src/engine/node_path.rs`
- `src/engine/scene_tree.rs`
- `src/engine/scene_signal.rs`
- `src/engine/packed_scene.rs`
- `src/engine/scene_validator.rs`
- `src/engine/project_validator.rs`
- `src/engine/editor_core.rs`

## Pruebas

El repositorio incluye tests para:

- schema versioning.
- update 0.9.3 y 0.9.3.4.
- `NodePath`, `SceneTreeIndex`, `SceneSignalBus` y `PackedScene2D`.
- Luau scripting e API 2D.
- scripting avanzado.
- Python automation y efectos.
- ForgeAI vertical slice.
- editor asset workflow.
- script editor productivity.
- sistemas upgrade.
- backend avanzado.
- runtime/performance de proyectos showcase.
- MCP/grand strategy.
- guias next-level.
- Qt/C++ bridge smoke test en `editor-cpp/tests`.

Los fixtures de formato viven en `tests/fixtures/formats`:

- `scene_v0.scene`, `scene_v1.scene`, `scene_future.scene`, `scene_broken.scene`.
- `prefab_v0.prefab`, `prefab_v1.prefab`, `prefab_future.prefab`, `prefab_broken.prefab`.

## Matriz De Validación

| Cambio | Validación mínima |
|---|---|
| Rust compartido | fmt, check all-features, runtime-only y tests afectados |
| EditorCore/FFI | `scripts/test-editor` |
| QML/C++ | build CMake, CTest y qmllint si está instalado |
| Contrato QML/C++ | `scripts/check-qt-backend-contract` |
| Luau API | tests Luau, `types/miniforge.luau`, snippets y API browser |
| Schema | tests de round-trip, legacy, actual, futuro y documento roto |
| Assets | import/move/duplicate/trash, GUID y dependency graph |
| Runtime | headless, export fixture y `miniforge_runtime --build` |
| Performance | `cargo dev -- bench` antes/después con workload anotado |
| Packaging | paquete local, manifest, runtime incluido/warning y launch |

`scripts/test-editor` ejecuta:

1. gate anti-editor-Rust;
2. rustfmt específico de `editor_core.rs` y `editor_ffi.rs`;
3. check del ABI/editor core;
4. tests `editor_core`;
5. check runtime-only;
6. configuración y build de `miniforge_editor_checks`;
7. CTest;
8. qmllint global cuando está disponible;
9. typecheck TypeScript cuando existen `node_modules`.

Los pasos opcionales se anuncian como omitidos; que falte qmllint o npm dependencies no equivale
a que esos checks hayan pasado.

Ejecuta además `scripts/check-qt-backend-contract` cuando cambies un nombre invocable o una
propiedad usada desde QML. El script compara referencias QML con `Q_INVOKABLE`, `Q_PROPERTY` y sus
implementaciones C++ para detectar bridges visuales sin backend.

## Configuracion De Proyecto

`AssetTools::ensure_project_files` crea defaults para:

- `project.json`
- `engine_config.json`
- `manifest.json`
- `settings/runtime_config.json`
- `settings/build_settings.json`
- `settings/build_profiles.json`
- `settings/input_map.json`
- `settings/tags.json`
- `settings/layers.json`

Puntos actuales de config:

- rendering backend `macroquad`.
- `experimental_wgpu = false`.
- `prefer_metal_on_macos = true`.
- pixel perfect activo.
- batching, culling, occlusion, LOD y post process activos por defecto.
- 3D desactivado por defecto.
- target FPS `60`.
- scheduler de scripts activo.
- perfiles graficos low/medium/high/ultra en runtime config.

## Export Runtime

`RuntimeExporter` copia un proyecto a un output de build validado. Perfiles:

- `Debug`
- `Release`
- `Shipping`
- `WebFuture`
- `MacosAppFuture`

Comando:

```bash
cargo dev -- export projects/DefaultProject projects/DefaultProject/builds debug
```

La exportacion:

1. Valida proyecto con `ProjectValidator`.
2. Bloquea si hay errores.
3. Copia el arbol del proyecto ignorando cache, git, target, builds, exports, logs, tools, plugins y temporales.
4. Construye manifest con `ManifestBuilder`.
5. Detecta assets usados y faltantes.
6. Calcula `EngineBackend::plan_project`.
7. Escribe `runtime_manifest.json`.
8. Escribe `build_info.json`.

El reporte contiene:

- output path.
- profile.
- cantidad de archivos copiados.
- used assets.
- missing assets.
- validation errors/warnings.
- release optimization.
- manifest path.
- readiness score.
- readiness actions.

## Packaging

`PackagingManager` arma una carpeta distribuible desde una exportacion runtime.

Funciones:

- `package_project`
- `package_project_with_installer_plan`
- `write_installer_plan`

Plataformas:

- macOS: `.dmg`
- Windows: `.msi`
- Linux: `AppImage`

El paquete intenta incluir `miniforge_runtime`. Si no lo encuentra, deja warnings y scripts/manifest para ejecutar manualmente con runtime externo.

El plan de instalador incluye:

- nombre de instalador.
- path de output.
- comandos sugeridos.
- estado de firma.
- warnings.
- artefactos.

Firma:

- macOS requiere identity Developer ID.
- Windows acepta identity o certificado.
- Linux acepta identity o certificado.

## Build Settings Y Profiles

Los settings viven en:

- `settings/build_settings.json`
- `settings/build_profiles.json`

Se validan contra escenas existentes y se usan para export/build. La configuracion runtime y los perfiles de calidad alimentan `GameClock`, render, budget y scheduler.

## Editor Qt/C++

La ruta Qt se compone de:

- Rust `EditorCore`.
- Rust FFI `editor_ffi`.
- Header C `include/miniforge_editor_bridge.h`.
- C++ `MfBridge`.
- QML `MainWindow.qml` y paneles.

El ABI usa structs C con:

- `abi_version`
- `struct_size`
- buffers fijos para strings comunes.

Reglas:

- El handle se crea con `mf_editor_create`.
- Se destruye con `mf_editor_destroy`.
- Las funciones devuelven `MfStatus`.
- Los errores se escriben en `MfError`.
- Los buffers string/reportan `required` si falta capacidad.

Capacidades del bridge:

- proyecto con opciones, launcher, creación/reparación y settings;
- jerarquía, selección, acciones de entidad y Scene Browser;
- inspector single/multi y catálogo de componentes;
- viewport RGBA/state, picking y transform batch;
- sesiones de Animation, Tilemap, UI Designer y otras tools;
- Prefab Studio y Visual Graph;
- assets, Content Browser, texto y operaciones seguras;
- profiler, dependencies, readiness y Runtime Health;
- Luau read/save/validate/API/recovery/debugger;
- Python automation, editor externo y ForgeAI;
- export, project operations y external launch;
- canvas de sprite, edición transaccional, transforms y save.

El frontend se reparte entre:

- `MainWindow.cpp`: shell, menús, toolbar, docks, workspaces y proceso externo;
- `MfBridge`: wrapper QObject del ABI;
- `MfModels`: modelos de jerarquía, assets, comandos, consola y readiness;
- `ViewportWidget`: Scene/Game View, input, selección y gizmos;
- `SpriteEditorWidget`: pixel/spritesheet editing;
- `LuauSyntaxHighlighter`: highlighting nativo;
- `editor-qml/panels`: herramientas QML;
- `editor-qml/components` y `Theme.qml`: componentes y tokens visuales.

Cuando añadas una herramienta:

1. define el estado y la mutación en un servicio frontend-neutral si contiene lógica;
2. expón DTO/ABI sin punteros internos;
3. agrega wrapper `MfBridge` y signal de refresh;
4. implementa panel/widget con estados empty/loading/error;
5. añade round-trip y smoke test;
6. registra dock, workspace, command palette y documentación.

Una acción destructiva no debe exponerse como consulta measure/retry. Usa función status-only y
un snapshot separado para que el probe de tamaño no repita la operación.

## Plugins

`PluginManager` escanea `plugins/` y `packages/` buscando `plugin.json`.

El manifest puede declarar:

- `name`
- `enabled`
- `dependencies`
- hooks.
- capabilities por lenguaje, sistema, componente, panel, importer, runtime feature, service, render backend o automation tool.

`load_plan`:

- ordena plugins por dependencias.
- bloquea dependencias faltantes/desactivadas.
- detecta ciclos.
- agrega hooks.
- resume capacidades.

`set_enabled` edita el manifest para activar/desactivar plugins.

## TypeScript Plugins

La carpeta `editor-plugins/typescript` contiene:

- schema de manifest.
- API `.d.ts`.
- ejemplo `hello-plugin`.
- package/tsconfig.

Uso recomendado:

1. Crear plugin con `plugin.json`.
2. Validar manifest.
3. Declarar extension points, comandos o paneles.
4. Mantener la integracion editor-only salvo que exista adapter runtime seguro.

## C# Plugins

`miniforge_dev scaffold-csharp-plugin` crea scaffolding de plugin C#:

```bash
cargo dev -- scaffold-csharp-plugin projects/DefaultProject RenderDiagnostics
```

El ejemplo `projects/MCP_LoveStoryLab/plugins/RenderDiagnostics` muestra:

- `plugin.json`.
- proyecto `.csproj`.
- fuente C#.

C# se considera disponible para editor/plugin tooling, no runtime-safe por defecto.

## Native Libraries

El sistema nativo usa `native.json` y `miniforge_native_entry_v1`.

Categorias:

- codec.
- platform SDK.
- audio.
- navigation.
- middleware.
- Steam.
- console.
- other.

Requisitos:

- ABI version compatible.
- `invoke_json` requerido.
- `free_string` requerido.
- `initialize` y `shutdown` opcionales.
- la libreria permanece cargada mientras el descriptor este vivo.

Safe Mode puede impedir carga de librerias nativas.

## Python Automation

`AutomationBridge` inspecciona:

- lenguajes disponibles.
- herramientas Python.
- plugins.
- render backend.
- recomendaciones.

Puede instalar herramientas Python del motor al proyecto:

```bash
cargo dev -- automation projects/DefaultProject --install-python
```

Herramientas de produccion incluidas:

- batch asset import.
- sprite converter.
- animation processor.
- atlas generator.
- scene report.
- project health matrix.
- project data export.
- automated build.
- procedural level.
- bulk properties.
- documentation generator.

## MCP

El servidor MCP vive en `mcp/miniforge`.

Funciones del servidor:

- leer version del motor desde `Cargo.toml` y `src/engine/version.rs`.
- crear proyectos y escenas.
- generar entidades/componentes/scripts.
- escribir JSON, scripts y assets base.
- operar sobre proyectos dentro de `projects/`.
- mantener feedback en `docs/MINIFORGE_MCP_FEEDBACK.md`.

Variable:

- `MINIFORGE_ENGINE_ROOT`: raiz del motor para el servidor MCP.

El servidor usa paths relativos al engine root y genera proyectos con estructura compatible con `AssetTools`.

## ForgeAI

ForgeAI separa planificacion IA de internals del editor/runtime.

Modulos:

- `context`: contexto de proyecto, entidades, componentes, assets y fisica.
- `providers`: proveedor local rule-based y contratos de provider.
- `planner`: planes, pasos, estado y riesgo.
- `validator`: validacion de planes y API Luau documentada.
- `executor`: host de editor, opciones, reportes, cambios de archivos y validacion.
- `permissions`: niveles/politicas.
- `diagnostics`: project doctor.
- `optimizer`: sugerencias.
- `testing`: suites y casos.
- `memory`: decisiones y memoria por proyecto.

Flujo:

1. Recibe `ForgeAiRequest`.
2. Genera `AiPlan`.
3. Valida plan.
4. Analiza diagnosticos.
5. Revisa cambios propuestos.
6. Ejecuta con `AiExecutor` si `approved` lo permite.
7. Puede hacer dry-run.

Regla: ForgeAI debe ejecutar cambios a traves de un host autorizado, no editar estado del motor directamente.

## Project Validator Y Auto Fix

`ProjectValidator` valida el proyecto y puede hacer auto-fix seguro:

- crea carpetas base.
- crea backups de config.
- restaura autosave de escena si aplica.
- resetea layouts corruptos.
- regenera GUIDs faltantes.
- elimina referencias null.
- marca assets faltantes.
- desactiva plugins rotos.
- reconstruye asset index/dependency graph.

El auto-fix reporta acciones y skipped.

## Development Workflow CLI

`development_workflow` permite:

- inspeccionar entorno.
- inspeccionar proyecto.
- correr workflows.
- correr microbenchmarks.

El comando `doctor` revisa herramientas requeridas/opcionales y sale con codigo `1` si el entorno no esta saludable.

El comando `project` muestra:

- path.
- validez.
- readiness score.
- errores.
- warnings.
- next actions.

El alias se define en `.cargo/config.toml`; la forma recomendada es `cargo dev -- <comando>`.

| Comando | Función |
|---|---|
| `doctor [--json]` | Toolchain requerido/opcional y salud del entorno |
| `project [path] [--json]` | Validez, readiness, errores, warnings y acciones |
| `assets [path] [--json]` | Reescanear y persistir metadata |
| `automation [path] [--install-python]` | Inspeccionar/instalar automation tools |
| `scaffold-csharp-plugin [project] [name]` | Crear manifest, csproj y fuente base |
| `export [project] [output] [profile]` | Validar, exportar y verificar assets |
| `bench [flags]` | Microbenchmarks de mundo, queries, física y Luau |
| `quick` | Feedback loop corto |
| `verify` | Gates de calidad/CI |
| `test` | Workflow de tests |
| `docs` | Workflow de documentación |
| `ship` | Gates y build del runtime shipping |

Los workflows aceptan `--json`; `quick` y `verify` aceptan además `--keep-going`.

## Benchmarks

`miniforge_dev bench` mide cargas de runtime:

- entidades.
- consultas espaciales.
- raycasts.
- entidades con scripts.
- frames simulados.

Usar antes/despues de cambios en:

- `RuntimeWorld`.
- `PhysicsSystem`.
- `SpatialIndex`.
- `LuauScriptRuntime`.
- scheduler Luau y counters `LuauUpdate*`, `LuauDistanceThrottled`, `LuauNearby*`.
- render/asset pipeline.
- RTS masivo.

## Roadmap Tecnico Actual

Capacidades ya presentes:

- runtime/editor separados por composition roots.
- escritura atomica compartida.
- schemas de escena/prefab.
- asset GUIDs.
- Luau y visual graphs.
- scheduler de mundo abierto con prioridad/distancia y `Entity.nearby` indexado.
- `RuntimeStabilityGuard`, Runtime Health y cuarentena/saneamiento de entidades.
- `SystemScheduler` con Fixed/Update/Late, dependencias, criticidad y budgets como base extensible.
- runtime 2D con fisica, camara, controllers y tilemap.
- RTS, gameplay, UI, particulas, audio, narrativa.
- Qt/C++ bridge.
- workbench Qt con workspaces, Content Browser profesional, Luau Studio y editores de sprite,
  graph, animation, tilemap, UI y prefabs.
- export/packaging.
- plugin manager, native ABI, TypeScript/C#/Python tooling.
- ForgeAI vertical slice.

Capacidades en estado inicial/futuro:

- WGPU/Metal completo.
- GPU particles reales.
- tile compute culling real.
- 3D hibrido productivo.
- sandbox fuerte para plugins externos.
- documentacion generada automaticamente desde registry.
- adopción del scheduler programable como orquestador universal del loop principal.
- debugger Luau instrucción-a-instrucción.

## Reglas Para Contribuir

- Mantener `EngineRuntime` libre de dependencias de editor.
- Usar `ProjectStorage` para escrituras durables de proyecto.
- Versionar formatos nuevos con `format` y `schema_version`.
- Rechazar schemas futuros explicitamente.
- Preferir GUIDs para referencias de assets.
- Mantener defaults de componentes en `default_component`.
- Agregar categorias al `ComponentRegistry` cuando se introduzcan componentes.
- Actualizar tests de fixtures cuando cambie un formato.
- Ejecutar runtime-only check al tocar sistemas compartidos.
- No cargar plugins nativos si Safe Mode los bloquea.
- Evitar que valores runtime con `_` se serialicen.

## Checklist De Release

Antes de publicar una version:

1. `cargo fmt --all --check`.
2. `cargo check --locked --all-targets --all-features`.
3. `cargo check --locked --no-default-features --features runtime --all-targets`.
4. `cargo test --locked --features editor_core`.
5. Tests de fixtures de escena/prefab.
6. Abrir proyecto demo en editor.
7. Validar `miniforge_dev doctor`.
8. Validar `miniforge_dev project`.
9. Refrescar assets.
10. Export runtime debug.
11. Validar `RuntimeManifestLoader`.
12. Empaquetar en carpeta local.
13. Probar `miniforge_runtime`.
14. Revisar readiness score y acciones.
15. Actualizar version/documentacion consolidada.

Además ejecuta `scripts/check-no-rust-editor` y `scripts/test-editor` antes de declarar listo un
cambio de release que toque la superficie desktop.

Consulta el [índice de documentación](README.md) para las fuentes canónicas y la clasificación
del feedback histórico.
