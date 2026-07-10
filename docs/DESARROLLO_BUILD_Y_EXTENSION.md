# MiniForge - Desarrollo, Build y Extension

Este documento consolida el flujo de desarrollo del motor, comandos, pruebas, exportacion, empaquetado, plugins, automatizacion, MCP y ForgeAI.

## Requisitos

El crate declara:

- Rust edition `2024`.
- `rust-version = 1.95`.
- paquete `miniforge`.
- version Cargo `0.9.3`.
- version runtime visible `0.9.3.4`.

Dependencias relevantes:

- UI/editor: `egui`, `egui_dock`, `egui_extras`, `egui-phosphor`, `rfd`, `arboard`, `syntect`.
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
default = ["editor"]
runtime = []
editor_ffi = ["editor"]
editor = ["runtime", ...]
```

Reglas:

- Codigo runtime debe compilar sin feature `editor`.
- Servicios de editor deben estar bajo `#[cfg(feature = "editor")]`.
- FFI del editor depende de `editor`.
- Export runtime usa `EngineRuntime`, no `Game`.

## Binaries

```bash
cargo run --bin miniforge --features editor
cargo run --bin miniforge_editor --features editor
cargo run --bin miniforge_runtime --features runtime
cargo run --bin miniforge_headless --features runtime
cargo run --bin miniforge_dev --features editor -- help
```

`miniforge_runtime` abre el jugador runtime con macroquad.

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

Variables utiles:

- `MINIFORGE_QT_BUILD_DIR`
- `MINIFORGE_QT_CMAKE`
- `MINIFORGE_RUNTIME`
- `MINIFORGE_ENGINE_ROOT`

## Desarrollo Diario

Flujo recomendado:

1. `cargo fmt`
2. `cargo check --features editor`
3. `cargo check --no-default-features --features runtime`
4. `cargo test --features editor`
5. `cargo run --bin miniforge_dev --features editor -- doctor`
6. `cargo run --bin miniforge_dev --features editor -- project projects/DefaultProject`
7. Probar editor o runtime segun la tarea.

Para cambios de runtime, ejecutar tambien:

```bash
cargo run --bin miniforge_dev --features editor -- export projects/DefaultProject
```

Para cambios en la capa de escena inspirada en Godot, cubrir al menos:

```bash
cargo check --features editor
cargo check --no-default-features --features runtime
cargo test --features editor --lib
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
cargo run --bin miniforge_dev --features editor -- export projects/DefaultProject projects/DefaultProject/builds debug
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

- abrir proyecto.
- listar entidades y seleccion.
- leer/editar inspector.
- listar assets.
- listar/ejecutar comandos.
- leer consola.
- leer readiness.
- snapshot de viewport RGBA.

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
cargo run --bin miniforge_dev --features editor -- csharp-plugin projects/DefaultProject RenderDiagnostics
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
cargo run --bin miniforge_dev --features editor -- automation projects/DefaultProject --install-tools
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
- runtime 2D con fisica, camara, controllers y tilemap.
- RTS, gameplay, UI, particulas, audio, narrativa.
- Qt/C++ bridge.
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

1. `cargo fmt`.
2. `cargo check --features editor`.
3. `cargo check --no-default-features --features runtime`.
4. `cargo test --features editor`.
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
