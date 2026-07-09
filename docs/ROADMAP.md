# MiniForge Roadmap

Roadmap incremental derivado de la auditoría del 2026-06-22. Cada fase mantiene compatibilidad, compila y termina con documentación y pruebas. Los elementos no marcados como terminados son objetivos, no capacidades prometidas.

## Reglas de entrega

Una fase sólo se cierra cuando:

- `cargo fmt --all -- --check` pasa.
- `cargo check --workspace` pasa.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` pasa.
- `cargo test --workspace` pasa.
- Las rutas nuevas tienen errores tipados y pruebas de fallo.
- Los formatos modificados tienen versión y migración.
- La documentación describe el código real.
- No hay dependencia runtime→editor nueva.

## Fase 0 — Auditoría y mapa arquitectónico

Estado: **completada en esta entrega**.

Entregables:

- `docs/ENGINE_AUDIT.md`.
- `docs/ARCHITECTURE.md`.
- `docs/ROADMAP.md`.
- Línea base de build, Clippy y 197 pruebas.
- Cinco P0 priorizados.
- P0-01 implementado: el allocator reserva IDs deserializados y evita colisiones futuras.

Riesgo que permanece: `u64` sigue siendo identidad persistente y runtime. La reserva corrige corrupción inmediata, pero no reemplaza el futuro GUID + handle generacional.

## Fase 1 — Estabilidad y persistencia

Estado: **completada el 2026-06-23**.

Verificación al cierre: formato limpio, `cargo check`, Clippy con `-D warnings` y 216 pruebas pasan.

Objetivo: ninguna escritura crítica debe perder el documento anterior ni producir resultados ambiguos.

### 1.1 ProjectStorage atómico

Estado: **completada el 2026-06-23**.

- Módulo único para escritura binaria y JSON atómica.
- Temp único en el mismo directorio, bloqueo por documento y limpieza de huérfanos.
- `write_all`, flush/sync, permisos conservados y reemplazo portable con rollback.
- Backup rotativo configurable.
- Errores con operación, path, tipo IO y causa.
- Pruebas de reemplazo, serialización fallida, backup fallido, rotación, concurrencia y limpieza.
- Escenas, autosave, prefabs, metadata, settings, manifests y documentos conectados mediante las fachadas existentes.

### 1.2 Recovery y logs

Estado: **completada el 2026-06-23**.

- Recovery periódico de escena, UI, pestañas y buffers sin guardar, con límites de memoria y limpieza al cerrar sin cambios.
- Logs rotativos por tamaño/política, escritura serializada y entradas multilínea sanitizadas.
- Panic hook en editor/runtime con reportes JSON, retención limitada y rutas sensibles sustituidas.
- Safe mode por `--safe-mode` que evita Luau, graphs y plugins nativos, y arranca con layout conocido.
- Errores de escritura de log visibles mediante `DeveloperConsole::log_write_error`.
- Pruebas de rotación, sanitización, crash report, retención, recovery y arranque seguro.

### 1.3 Esquemas

Estado: **completada para los formatos críticos de escena/prefab**.

- `format`, `schema_version` y `engine_version` independientes.
- Migradores idempotentes 0→1 desde formatos actuales.
- Validación previa a materialización y rechazo claro de versiones futuras.
- Backups sólo para corrupción/incompatibilidad recuperable; nunca para ocultar schema futuro.
- Golden tests legacy, actuales, futuros y dañados, más pruebas reales de loaders/savers.

Implementado el 2026-06-24: visual graphs y metadata de assets también usan `format`, `schema_version` y `engine_version`, migran legacy y rechazan schemas futuros.

Criterio de salida: cerrar el proceso durante guardado no destruye la última versión válida en las plataformas soportadas.

## Fase 2 — Separación editor/runtime/core

Estado: **player y paquete exportado aislados; migración interna del editor en progreso**.

Verificación de este corte: Clippy estricto y 219 pruebas pasan.

Objetivo: demostrar por compilación que el runtime no depende del editor.

- Implementado: `RuntimeWorld` posee entidades/índice, elimina el clone por frame y valida IDs/jerarquía.
- Implementado: `EngineRuntime` construye sólo mundo y servicios de juego para builds exportados.
- Implementado: features `runtime`/`editor`, player separado y puerta CI sin dependencias GUI/editor.
- Implementado: export runtime verificable, sin backups, estado `.miniforge`, logs, documentación ni layout del editor; una prueba arranca directamente la carpeta exportada.
- Mover historial, docking, script editor, sprite editor y workspace a `EditorState`.
- Hacer que `Game` delegue y mantener fachada de compatibilidad.
- Extraer `miniforge_core` y `miniforge_project` cuando los límites estén estables.

Criterio de salida: el player compila y arranca sin egui, egui_dock, rfd ni módulos editor.

## Fase 3 — ECS, GUID, assets, escenas y prefabs

### 3.1 Identidad y world

- `EntityGuid` persistente con migración desde `u64`.
- `EntityHandle { index, generation }` runtime.
- Índice handle→storage y GUID→handle.
- Eventos create/destroy y validación de referencias.
- Query API inicial para componentes builtin.

### 3.2 Component registry

- Metadatos tipados de propiedades.
- Factory, validación, clonación y serialización.
- Required/incompatible components.
- Hooks y componentes de scripting.

### 3.3 AssetDatabase

- GUID independiente del path.
- Implementado: tabla versionada con fingerprint para reconciliar moves externos no ambiguos.
- Implementado: move/rename gestionado con rollback sin cambiar GUID.
- Implementado: resolver GUID→ruta y actualización de dependencias de ruta.
- Pendiente: placeholders y referencias profundas exclusivamente por GUID.
- Artifacts cacheados por hash + versión de importer.

### 3.4 Escenas/prefabs

- Scene manager aditivo con remapeo de referencias.
- Persistent entities y subescenas.
- Prefabs anidados, ciclos, overrides por propiedad y variantes.
- Diff, apply/revert y actualización de instancias.

Criterio de salida: mover un asset o instanciar una escena aditiva conserva todas las referencias válidas.

## Fase 4 — Undo, jerarquía, inspector y viewport

- Command bus transaccional como única ruta de mutación del editor.
- Merge de drags/sliders y límites de memoria.
- Deltas para propiedades y snapshots sólo para operaciones estructurales.
- Parenting con detección de ciclos y transform completo.
- Multi-selección y mixed values en inspector por metadata.
- Gizmos local/global, snapping, box selection y overlays.
- Undo de assets, prefabs, graphs, tilemaps y settings.

Criterio de salida: cada acción mutante visible en el editor tiene undo/redo determinista.

## Fase 5 — Renderer 2D y tilemaps

- Convertir `RenderBackend` en la única ruta de render.
- Render queue estable, sort layers y order-in-layer.
- Sprite batching, atlases y material batching medidos.
- Múltiples cámaras, render targets, pixel perfect y camera stacking.
- Tilemap chunks, dirty regions y culling espacial.
- Luces/sombras 2D y postprocesado detrás de capabilities.
- Mantener Macroquad hasta alcanzar paridad; wgpu/Metal queda detrás del mismo contrato.

Criterio de salida: benchmark reproducible con draw calls y frame time comparables antes/después.

## Fase 6 — Física, animación, input, audio y UI runtime

### Física

- `PhysicsPipeline` Rapier persistente y mapeo de handles.
- Rigidbodies, colliders, sensors, joints, queries y eventos.
- Fixed update, interpolación, CCD y debug draw.
- Character controller y tilemap collision baking.

### Otros subsistemas

- State machines y events de animación sobre clips versionados.
- Input contexts/rebinding/gamepad con separación editor/juego.
- Buses, spatial audio 2D, fades y pooling medido.
- UI runtime sin egui: layout, focus, input, scaling y binding.

Criterio de salida: proyecto de ejemplo jugable usa sólo APIs runtime públicas.

## Fase 7 — Luau y visual scripting

- API Luau documentada y validación de handles.
- Lifecycle completo y límites de ejecución configurables.
- Hot reload transaccional con conservación controlada de estado.
- Errores con archivo/línea y definitions para autocompletado.
- Unificar graphs/blueprints en IR versionado.
- Cache compilada, protección de loops y ejecución sin recorrer UI.
- Breakpoints, step y watch values en editor.

Criterio de salida: Luau y graph producen las mismas acciones sobre una API runtime común y testeada.

## Fase 8 — IA, navegación y RTS

- Navigation service con jobs y presupuesto por frame.
- A*, flow fields, cost/influence/threat maps y replanning.
- Steering, percepción, blackboard, behavior tree y utility AI.
- Selección/grupos/comandos RTS sobre handles validados.
- Fog, visión, producción, economía y combate con spatial queries.
- Benchmarks de 100, 1 000 y 10 000 unidades según perfil.

Criterio de salida: las optimizaciones se aceptan sólo con métricas y sin romper determinismo funcional.

## Fase 9 — Plugins, exportación y profiler

- Plugin API versionada basada en registrars y handles opacos.
- Separar plugins editor/runtime y verificar dependencias.
- Estrategia de aislamiento o política explícita para plugins nativos.
- Export incremental por manifest de assets usados.
- Builds reproducibles y perfiles por plataforma.
- Profiler con memoria, scripting, physics, render y export JSON/CSV.
- Matriz CI Linux, macOS y Windows.

Criterio de salida: un plugin defectuoso se desactiva con diagnóstico y un build exportado no contiene editor.

## Fase 10 — Consolidación

- Extraer crates restantes sólo donde los contratos ya sean estables.
- Completar guías de editor, Luau, graphs, plugins, assets, escenas, prefabs y build.
- Proyectos de ejemplo platformer, top-down y RTS.
- Golden tests de migración y proyectos antiguos.
- Benchmarks y budgets de regresión.
- Changelog y política de compatibilidad.

Criterio de salida: release candidate reproducible con documentación que coincide con el código.

## Próximo paso exacto

Iniciar **Fase 2** con un cambio acotado:

1. Extraer de `Game` historial, docking, editores y workspace hacia `EditorServices` sin romper las operaciones públicas.
2. Construir `EditorServices` sólo para el editor; runtime usará `None` y errores explícitos si una herramienta se solicita.
3. Hacer que el player construya únicamente `RuntimeWorld` y servicios de juego.
4. Añadir features `runtime`/`editor` y una puerta CI que compile el player sin editor.
5. Mantener reexports de compatibilidad mientras se migran tests y proyectos.

Este paso ataca el mayor acoplamiento restante sin una reescritura total.
