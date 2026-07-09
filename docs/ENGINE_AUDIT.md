# MiniForge Engine Audit

Fecha de corte: 2026-06-22  
Versión declarada: 0.9.3  
Alcance: Fase 0, auditoría estática y validación del workspace Rust actual.

## Resumen ejecutivo

MiniForge ya es una base funcional y probada, no un proyecto vacío. Tiene un editor ejecutable, un player separado, escenas, prefabs, componentes dinámicos, Luau, scripting visual, herramientas 2D, física propia, integración parcial con Rapier2D, audio, exportación y CI. La línea base pasa `cargo check`, Clippy con warnings como errores y 197 pruebas.

El límite de concentración sigue visible dentro del editor, pero ya no atraviesa la build del juego: `EngineRuntime` es el composition root aislado, las features `runtime`/`editor` imponen la dirección de dependencias y CI compila el player sin GUI ni servicios editoriales. La exportación filtra backups, estado `.miniforge`, logs, documentación y layout del editor, genera un manifest verificable y arranca de nuevo desde la carpeta exportada. Escenas, prefabs, graphs y metadata de assets tienen versión de esquema independiente; los assets conservan GUID al moverse y la persistencia crítica pasa por `ProjectStorage`. La deuda que permanece es interna: adelgazar la fachada `Game`, completar GUID+handles generacionales para entidades y migrar referencias profundas de assets desde paths a GUID.

No se recomienda una reescritura. La ruta segura es estabilizar identidades y persistencia, definir contratos en el crate actual y extraer crates sólo cuando esos contratos estén cubiertos por pruebas.

## Método y línea base

Se revisaron:

- Los 209 archivos Rust bajo `src/` y sus declaraciones de módulos.
- Los binarios `miniforge`, `miniforge_editor`, `miniforge_runtime` y `miniforge_dev`.
- `Cargo.toml`, el árbol duplicado de dependencias, CI y los formatos de proyecto presentes.
- Entidades, componentes, escenas, prefabs, assets, historial, jerarquía, render, input, scripting, física, hot reload, exportación y validadores.
- Panics explícitos, `unwrap`, `expect`, código `unsafe`, estados globales y archivos grandes.

Estado observado antes de la mejora P0:

| Puerta | Resultado |
| --- | --- |
| `cargo check --workspace` | Pasa |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pasa |
| `cargo test --workspace` | 197 pruebas pasan |
| `panic!`, `todo!`, `unimplemented!` en `src/` | 0 |
| `unwrap` en `src/` | 29, varios en pruebas o tras validación local |
| `expect` en `src/` | 167, principalmente componentes builtin y algunos locks/inicialización |
| `unsafe` en `src/` | 19, concentrados en la ABI de plugins nativos |

El árbol de trabajo ya contenía numerosas modificaciones no confirmadas. La auditoría y el cambio P0 preservan ese trabajo.

## Arquitectura actual

MiniForge continúa como un solo paquete Cargo, pero las features `runtime` y `editor` ya separan los composition roots y evitan publicar/compilar `editor_app` en una build runtime-only.

| Área | Implementación actual | Observación |
| --- | --- | --- |
| Entrada editor | `src/editor_app.rs`, `src/main.rs`, `src/bin/miniforge_editor.rs` | `editor_app.rs` tiene unas 12 871 líneas. Los dos binarios de editor llaman a la misma función. |
| Entrada runtime | `src/bin/miniforge_runtime.rs`, `src/runtime/engine_runtime.rs`, `src/runtime/player.rs` | Construye `EngineRuntime`; no importa `Game` ni servicios de editor. |
| Orquestación | `src/core/game.rs` | `Game` tiene unas 3 380 líneas y posee mundo, assets, escena, editor, scripting y sistemas. |
| Entidades/ECS | `GameObject`, `Component`, `RuntimeWorld` | Autoridad única en `Vec`, índice espacial y validación de mundo; componentes dinámicos en `Vec<Component>` con datos JSON. Aún no hay índices generacionales. |
| Escenas | `SceneManager`, `SceneSerializer`, `SceneSaveManager`, `AutosaveManager` | Carga, backup, migración mínima, guardado incremental y autosave existentes. |
| Prefabs | `PrefabManager`, `AdvancedPrefabSystem`, `PrefabOverrides` | Instancia, variantes y diff básicos; aún no hay grafo completo de prefabs anidados/overrides por propiedad. |
| Assets | `AssetDatabase`, importadores y pipeline 2D | Escaneo paralelo, metadatos, settings, dependencias y reverse dependencies. |
| Render | `render::backend`, `render::renderer`, `engine::render_2d` | Existe un trait de backend y comandos, pero el render activo sigue acoplado a Macroquad y el renderer genérico es principalmente estadístico. |
| Física | `systems::physics_system`, `rapier_physics_bridge` | La simulación principal es propia; Rapier crea colliders para inspección/reportes, no mantiene aún un `PhysicsPipeline` persistente. |
| Scripting | `luau_scripting`, `script_host_2d`, `visual_scripting`, blueprints | Luau usa `mlua` con hot reload; los graphs tienen validación/compilación en partes del stack. Rhai queda como módulo de compatibilidad deprecado. |
| Editor | egui, egui_dock y superficies propias | Jerarquía, inspector, viewport, docking, historial, browsers y herramientas ya existen. |
| Exportación | `runtime_exporter`, `packaging_manager`, manifests | Produce y valida un árbol runtime limpio; Cargo y CI imponen el límite binario/editor. |

### Flujo de datos actual

1. `Game::from_project` crea carpetas y configuraciones, escanea recursos y assets, construye también servicios de editor y carga la escena inicial.
2. Las entidades cargadas se materializan como `GameObject` dentro de `RuntimeWorld`; `Game` mantiene acceso compatible durante la migración.
3. `run_headless_once` ejecuta los sistemas en un orden escrito manualmente dentro de `Game`; `SystemScheduler` existe, pero no gobierna ese bucle principal.
4. Los campos frecuentes de transform, sprite y collider están duplicados entre campos de `GameObject` y `Component.data`; `sync_to_components`/`sync_from_components` mantienen la coherencia.
5. Las escenas y varios documentos se serializan a JSON con extensiones `.scene`, `.prefab` y `.mfgraph`.

## Fortalezas

- La base compila limpia con Rust 2024 y Clippy estricto.
- La cobertura funcional es amplia: 197 pruebas cubren editor, assets, escenas, scripting, UI, física, RTS, packaging y flujos de proyecto.
- No hay `todo!`, `unimplemented!` ni `panic!` explícitos en producción.
- Las escenas tienen backup, recuperación y guardado incremental; la configuración también contempla recuperación de corrupción.
- `SceneValidator` detecta IDs duplicados, nombres vacíos y componentes inválidos.
- El AssetDatabase conserva metadatos ya conocidos durante un scan, usa Rayon y modela dependencias con `petgraph`.
- Luau tiene watcher real, sandbox/API controlada y errores recuperables en el bucle del juego.
- Hay abstracción inicial de backend de render, profiler por sistema, fixed timestep y spatial indexes.
- El runtime tiene un binario dedicado y valida manifests de exportación.
- CI ya exige formato, check, Clippy, tests y documentación.
- La Fase 1 añade persistencia durable, logs rotativos, crash reports sanitizados, recovery de buffers y safe mode conectado al runtime.

## Cinco mejoras P0

### P0-01 — Evitar colisiones de IDs después de cargar escenas

Estado: **implementado en esta Fase 0**.

`GameObject::from_data(..., true)` reemplazaba el ID recién generado por el ID serializado sin adelantar `NEXT_ID`. Una escena con IDs dispersos podía hacer que una entidad nueva reutilizara un ID cargado, rompiendo parenting, selección, targets, scripts y undo.

La solución añade `register_existing_entity_id`, reserva atómicamente el rango restaurado y registra también el sufijo de nombres existentes. No modifica el formato de escena. Una prueba recorre la ruta real de deserialización.

### P0-02 — Unificar persistencia atómica y durable

Estado: **implementado el 2026-06-23**.

`ProjectStorage` centraliza escritura binaria/JSON, temporales únicos, sincronización, permisos, bloqueo por documento, fallback de reemplazo con rollback, backups rotativos y errores tipados. `AssetTools`, manifests, documentos, escenas, autosaves y prefabs delegan en esta capa. Al abrir un proyecto se limpian únicamente temporales MiniForge con más de 24 horas. Las pruebas cubren reemplazo, rotación, fallo previo, fallo de serialización, concurrencia y limpieza selectiva.

Riesgo residual: el bloqueo es por proceso; dos procesos externos todavía dependen de la atomicidad del filesystem. En Windows se sincroniza el archivo temporal, pero no existe una operación portable de `fsync` para directorios en `std`.

### P0-03 — Imponer el límite editor/runtime

Estado: **implementado el 2026-06-24 para el player exportado**.

`EngineRuntime` es ahora el composition root del player y no importa `Game` ni servicios de editor. El binario `miniforge_runtime` usa un frontend Macroquad propio bajo `runtime::player`; las features `runtime`/`editor` y la puerta CI `cargo check --no-default-features --features runtime --bin miniforge_runtime` demuestran que el juego exportado compila sin egui, egui_dock, rfd, arboard, syntect, trash ni nucleo.

`Game` se conserva como fachada de compatibilidad del editor. Aún conviene mover sus campos editoriales a `EditorServices` antes de extraer crates, pero esa deuda ya no forma parte del binario runtime.

### P0-04 — Hacer estables los GUID de assets al mover archivos

Estado: **implementado el 2026-06-24**.

La metadata de assets tiene ahora contrato propio (`miniforge.asset-metadata`, schema 1), fingerprint de contenido y resolver GUID→ruta. `AssetDatabase::move_asset` mueve archivo y tabla como una operación lógica con rollback, actualiza dependencias y conserva el GUID. Un scan también reconcilia renames externos cuando el contenido identifica de forma única el registro anterior.

Las pruebas cubren move gestionado, reload de metadata, rename externo y rechazo de schemas futuros. Si dos archivos eliminados tienen contenido idéntico, el scan no adivina: se requiere la operación gestionada, evitando reasignar una identidad ambiguamente.

### P0-05 — Versionar y validar esquemas de escena/prefab

Estado: **implementado para escenas y prefabs el 2026-06-23**.

Ambos formatos tienen identificador, `schema_version` entero, migración idempotente 0→1, validación antes de materializar y rechazo controlado de versiones futuras. Los loaders no ocultan un schema futuro usando un backup viejo. Golden tests cubren legacy, actual, futuro y dañado.

Los visual graphs y la metadata de assets adoptaron después el mismo contrato (`format` + `schema_version` + `engine_version`). El riesgo residual está en documentos auxiliares que aún son datos libres y en la validación profunda de referencias por GUID, que pertenece a Fase 3.

## Problemas críticos y deuda técnica

### Acoplamiento y módulos grandes

- `editor_app.rs` concentra presentación, interacción y coordinación en unas 12 871 líneas.
- `core/game.rs` es un composition root y también un god object de unas 3 380 líneas.
- `engine/component.rs` supera 2 000 líneas y mezcla catálogo, defaults, metadatos y acceso dinámico.
- `engine/mod.rs` publica 127 módulos planos; el nombre `engine` no expresa límites de dependencia.
- Resuelto el 2026-06-23: `RuntimeWorld` eliminó la duplicación de `Game.world.entities` y el clone completo por frame.

### ECS e identidad

- `u64` es a la vez identidad serializada y handle runtime; no hay GUID persistente por entidad ni índice generacional.
- Las búsquedas habituales usan `Vec::iter().find`, O(n).
- Los componentes se localizan por string y almacenan `serde_json::Value`; esto facilita extensibilidad, pero sacrifica validación y rendimiento.
- Campos como posición, tamaño y sprite se duplican fuera y dentro de componentes.
- No hay query API, borrow model ni eventos/hook centrales de ciclo de vida.

### Escenas, prefabs y undo

- El historial captura snapshots completos de entidades y, para comandos, también tilemap, grid, cámara y UI. Es simple y correcto para escenas pequeñas, pero escala en memoria y clonados.
- Parenting sólo sincroniza traslación directa; no compone rotación/escala ni detecta ciclos en `HierarchyManager`.
- Escenas aditivas filtran colisiones de ID, pero no remapean referencias.
- Los prefabs avanzados guardan información de variante, pero no existe resolución recursiva con detección de ciclos y aplicación tipada de overrides.

### Render y rendimiento

- La abstracción `RenderBackend` es un buen inicio, pero no es aún el único camino de render.
- `Renderer::draw_entities` cuenta una draw call por entidad visible; no implementa batching real.
- El culling principal es básico; hay estructuras espaciales, pero su adopción no es uniforme.
- Clones de escena completa y strings/component lookups pueden generar allocations significativas por edición/frame.

### Física

- Rapier2D es dependencia activa, pero el bridge actual inspecciona y construye colliders temporales. La simulación no usa aún sets persistentes, pipeline, joints ni eventos Rapier.
- El fixed timestep existe en `GameClock`, aunque el orden de sistemas sigue codificado en `Game`.

### Scripting y graphs

- Luau es real y tiene hot reload, pero la inicialización contiene un `expect` y el estado compartido usa `Arc<Mutex<_>>`; se debe medir contención y recuperar poison en rutas críticas.
- Hay dos superficies de graph/blueprint con responsabilidades solapadas. Debe consolidarse un IR runtime único sin eliminar compatibilidad de archivos.

### Thread safety y paralelización

- Asset scan usa Rayon y los watchers usan canales, lo cual es apropiado.
- El mundo se muta principalmente en un solo hilo y los sistemas reciben `&mut Vec<GameObject>`; paralelizar sistemas hoy sería inseguro sin declarar accesos.
- Antes de añadir threads al ECS se necesita un scheduler con conjuntos de lectura/escritura y fases explícitas.

## Riesgos de estabilidad

- Referencias por ID pueden quedar rotas tras duplicados, carga aditiva o cambios manuales de JSON.
- Resuelto para assets gestionados: move/rename conserva GUID y el scan reconcilia renames externos no ambiguos; aún faltan referencias profundas exclusivamente por GUID.
- Temporales fijos pueden colisionar entre guardado manual y autosave concurrente.
- Backups rotativos aún no son uniformes; algunos errores de backup se descartan.
- Plugins nativos atraviesan una ABI `unsafe`; hay validación de versión, pero un plugin defectuoso comparte proceso y puede derribar el editor.
- Muchos defaults tolerantes permiten abrir archivos antiguos, pero también pueden ocultar corrupción semántica.
- CI sólo ejecuta Linux; no verifica semántica de archivos, audio, dialogs o plugins en macOS/Windows.

## Cuellos de botella probables

1. Búsquedas lineales de entidades y componentes por cada sistema.
2. Snapshots completos y clones de `GameObject` en undo/play mode/world sync.
3. Sincronización doble entre campos directos y componentes JSON.
4. Draw calls por sprite en la ruta simple y falta de una render queue única.
5. Reconstrucción de dependencias mediante búsqueda textual de cada asset en cada archivo.
6. `Game::run_headless_once` secuencial y con ownership monolítico.

Estos puntos deben medirse con el profiler antes de optimizarlos. No se recomienda introducir pools o paralelización general sin datos.

## Dependencias

### Adecuadas

- `serde`/`serde_json`: formatos y metadata actuales.
- `mlua` con Luau vendored: scripting portable.
- `rapier2d`: objetivo correcto para física 2D.
- `petgraph`: análisis de dependencias y ciclos.
- `rayon`, `notify`, `crossbeam-channel`: pipeline y hot reload.
- `egui`/`egui_dock`: editor inmediato y docking.

### A vigilar

- Macroquad trae `image 0.24`/`png 0.17`, mientras el proyecto usa `image 0.25`/`png 0.18`; aumenta tamaño y tiempo de build.
- El stack de `zip 8.6` incorpora dos generaciones de crates crypto/digest.
- `libloading` y la ABI C requieren una política de aislamiento para plugins no confiables.
- Todas las dependencias editor/runtime están en el mismo conjunto; no hay features para reducir el player.
- CI usa `stable`, mientras `Cargo.toml` declara un MSRV concreto. Falta una tarea que pruebe exactamente ese MSRV y una matriz macOS/Windows.

## Prioridades P1, P2 y P3

### P1 — Arquitectura y datos

- Introducir `RuntimeWorld` con almacenamiento indexado y handle generacional, manteniendo IDs serializados.
- Centralizar el registro de componentes con metadatos de propiedad y validación.
- Conectar el bucle principal al scheduler con fases `PreUpdate`, `Update`, `FixedUpdate`, `LateUpdate`, `Render`.
- Unificar IR de visual scripting y caché compilada.
- Convertir AssetDatabase en autoridad GUID→path y añadir moves transaccionales.
- Migrar undo a comandos/deltas para operaciones de alta frecuencia.

### P2 — Capacidad profesional

- Render queue, batching, atlases, cámaras y tilemap chunks sobre el trait existente.
- Integración persistente de Rapier2D, eventos y debug draw.
- Prefabs anidados, variantes resueltas y overrides por propiedad.
- Import jobs cancelables con progreso y cache por hash/importer version.
- Tests de macOS/Windows y builds reproducibles.

### P3 — Extensión y optimización

- API estable de plugins compilados y, después, aislamiento de plugins nativos.
- Streaming de mundos grandes, IA avanzada y optimización RTS medida.
- Backend wgpu/Metal detrás del contrato validado, sin retirar Macroquad antes de paridad.
- Pools selectivos y ejecución paralela sólo después de métricas.

## Arquitectura futura propuesta

La división objetivo debe nacer de límites comprobados, no de mover archivos de forma masiva:

```text
miniforge_core       ids, errores, tiempo, eventos, math común
miniforge_ecs        world, handles, registro, queries, scheduler
miniforge_project    config, formatos, migraciones, persistencia
miniforge_assets     AssetDatabase, importers, cache, hot reload
miniforge_scene      escenas, jerarquía, prefabs
miniforge_runtime    composition root y sistemas de juego
miniforge_render     comandos y backend neutral
miniforge_editor     egui, viewport, inspector, undo y herramientas
miniforge_build      manifests, exportación y packaging
miniforge_plugin_api contratos versionados sin UI concreta
```

Regla de dependencias: editor puede depender del runtime; runtime no puede depender del editor. Los formatos y el plugin API no deben depender de egui, Macroquad ni dialogs del sistema.

## Plan de migración incremental

1. Completar P0 de identidad, persistencia y esquemas dentro del crate actual.
2. Añadir tests de arquitectura y APIs fachada; no cambiar aún paths públicos usados por proyectos.
3. Crear `RuntimeWorld` y hacer que `Game` delegue en él. Mantener los campos/facades antiguos durante una ventana de deprecación.
4. Mover servicios de editor fuera de la construcción runtime y añadir un test que compile el player sin feature `editor`.
5. Extraer primero crates hoja (`core`, `project`, `assets`), después `scene`/`ecs`, y al final editor/runtime.
6. Versionar cada formato antes de cambiar su representación y ofrecer migración idempotente más backup.
7. Sustituir rutas antiguas sólo después de paridad funcional y benchmarks.

El mapa detallado y las reglas de dependencia están en `docs/ARCHITECTURE.md`. La secuencia y criterios de salida están en `docs/ROADMAP.md`.
