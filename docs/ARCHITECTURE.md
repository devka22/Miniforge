# MiniForge Architecture

Este documento describe el código real de MiniForge 0.9.3 y la dirección incremental acordada en la Fase 0. Las capacidades propuestas se marcan como objetivo; no deben interpretarse como implementadas.

## Principios

1. El runtime nunca depende del editor.
2. Los datos de proyecto se validan antes de convertirse en estado runtime.
3. Una identidad persistente no es un índice de memoria.
4. Todo guardado importante es atómico, recuperable y observable.
5. Los backends dependen de contratos del motor, no al revés.
6. Las migraciones preservan compatibilidad y son idempotentes.
7. Primero se crean límites y pruebas; después se extraen crates.

## Topología actual

```text
miniforge (fachada de compatibilidad; features `runtime` y `editor`)
├── bin/miniforge_editor ─┐
├── bin/miniforge ────────┴── editor_app (Macroquad + UI del editor)
├── bin/miniforge_runtime ──── runtime/game_runner
│                                └── core/Game
├── core/Game ─────────────────── world + assets + escenas + editor + sistemas
├── entities/GameObject ───────── campos directos + Vec<Component<Value>>
├── engine/* ──────────────────── 127 módulos de servicios y herramientas
├── systems/* ─────────────────── sistemas que mutan Vec<GameObject>
└── render/* ──────────────────── contrato de backend + renderer básico
```

El binario runtime usa `EngineRuntime` y `runtime::player`; con `--no-default-features --features runtime` no compila `Game`, `editor_app` ni dependencias GUI. `Game` conserva tipos editoriales únicamente como fachada del editor mientras continúa su migración interna.

## Capas actuales

### Modelo y runtime

- `entities::game_object::GameObject` es la unidad serializable.
- `engine::component::Component` aporta componentes dinámicos y editables como JSON.
- `engine::world::RuntimeWorld` posee el único `Vec<GameObject>` y el índice espacial. El snapshot clonado `World.entities` fue eliminado.
- `core::game::Game` construye y ejecuta casi todos los subsistemas.
- `systems` contiene lógica de animación, audio, gameplay, input, movimiento, partículas, física, render y RTS.

### Proyecto y contenido

- `AssetTools` define layout y helpers de archivos.
- `AssetDatabase` indexa assets por ruta, conserva metadata e infiere dependencias.
- `SceneManager`, `SceneSaveManager` y `SceneSerializer` controlan escenas.
- `PrefabManager` y `AdvancedPrefabSystem` controlan prefabs.
- `ProjectValidator` y `SceneValidator` producen diagnósticos recuperables.

### Presentación

- Macroquad posee ventana y frame loop.
- egui y egui_dock modelan las superficies de editor.
- `render::backend::RenderBackend` define una frontera inicial para Macroquad/wgpu.
- `ui_runtime` y `ui_canvas` modelan UI del juego, aunque aún viven en el mismo paquete que el editor.

### Extensión

- Luau usa `mlua`; Rhai se conserva sólo como compatibilidad deprecada.
- Los graphs `.mfgraph` y blueprints tienen modelos y validadores propios.
- Los plugins Rust/metadata, Python editor tools, C# editor plugins y la ABI nativa están modelados por manifiestos; Python/C# son editor-only y el export runtime los excluye por defecto.

## Flujo runtime actual

```text
proyecto en disco
  → AssetTools / EngineConfig / AssetDatabase
  → SceneSerializer::migrate
  → GameObject::from_data
  → RuntimeWorld.units
  → Game::run_headless_once
  → sistemas secuenciales
  → render/audio/eventos
```

La carga tolerante ayuda a compatibilidad, pero la validación y migración deben convertirse en pasos explícitos antes de materializar entidades.

## Identidad

### Actual

- Entidades: `u64` generado por un contador atómico y guardado en escena.
- Assets: GUID persistente como autoridad, índice GUID→registro y path mutable con fingerprint para reconciliar moves.
- Prefabs: GUID, source path y flags dentro de la entidad.
- Componentes: nombre string como discriminador.

Desde esta Fase 0, deserializar un ID de entidad reserva el contador global para impedir su reutilización.

### Objetivo

```text
EntityGuid (persistente, serializado)
        ↓ resolución al cargar
EntityHandle { index, generation } (sólo runtime)
```

- Las referencias de archivos usan GUID.
- Las referencias runtime usan handles generacionales.
- Un mapa de resolución permite migrar los `u64` existentes sin romper escenas.
- El editor muestra ambos sólo en diagnóstico; las APIs de juego usan handles validados.

## Formatos y migración

Formatos observados:

- Proyecto/configuración: JSON.
- Escenas: `.scene` con JSON.
- Prefabs: `.prefab` con JSON.
- Graphs: `.mfgraph` con JSON.
- Metadata/import settings: JSON.

Cabecera implementada para escenas y prefabs:

```json
{
  "format": "miniforge.scene",
  "schema_version": 1,
  "engine_version": "0.9.3",
  "version": "0.9.3",
  "scene_name": "main",
  "entities": []
}
```

`engine_version` es informativa; `schema_version` controla migraciones. Cada loader debe seguir:

1. Leer con límites de tamaño.
2. Parsear sin materializar estado runtime.
3. Validar cabecera y versión.
4. Migrar paso a paso sobre una copia/backup.
5. Validar referencias y invariantes.
6. Materializar.
7. Emitir un reporte de warnings y reparaciones.

Escenas y prefabs actuales sin cabecera se aceptan mediante migradores de versión implícita 0. Los demás documentos incorporarán este contrato gradualmente; no se consideran versionados todavía.

## Persistencia

Implementación actual de `ProjectStorage`:

```rust
ProjectStorage::write_atomic(path, bytes) -> Result<AtomicWriteReport, StorageError>
ProjectStorage::write_atomic_with_backup(path, bytes, BackupPolicy)
ProjectStorage::write_json_atomic(path, value)
ProjectStorage::cleanup_stale_temporary_files(directory, minimum_age)
```

Garantías actuales:

- Temp único en el mismo filesystem.
- Escritura completa, flush y `sync_all` antes del reemplazo.
- Conservación de permisos al reemplazar un documento.
- Reemplazo directo donde el sistema lo permite y fallback con rollback donde no.
- Bloqueo por destino para guardado manual/autosave concurrente dentro del proceso.
- Backups rotativos mediante `BackupPolicy`.
- Limpieza selectiva de temporales MiniForge antiguos al abrir un proyecto.
- `StorageError` con operación, path, `io::ErrorKind` y causa.
- Serialización terminada antes de tocar el documento o sus backups.

`AssetTools`, escenas, autosave, prefabs, documentos y manifests usan esta implementación. El bloqueo entre procesos y una estrategia específica de Windows para sincronizar directorios permanecen como mejoras futuras; no deben reaparecer helpers locales de temp + rename.

## Estabilidad operacional

- `Logger` rota por tamaño con retención configurable, serializa escrituras entre hilos y neutraliza saltos de línea inyectados.
- `CrashReporter` instala el panic hook en las entradas de editor/runtime, escribe JSON bajo `logs/crashes`, limita la retención y sustituye rutas de proyecto/home.
- `SessionRecoveryManager` guarda cada diez segundos pestañas, buffers sucios y estado básico de UI bajo `.miniforge/recovery`; un cierre limpio elimina el checkpoint sólo cuando no quedan cambios.
- `SafeModeSettings` atraviesa el composition root de `Game`. Safe mode evita ejecutar Luau/graphs, no carga plugins nativos y usa el layout inicial conocido.
- La escena no se restaura silenciosamente desde el checkpoint de sesión: se informa al usuario y se conserva el autosave como autoridad de recuperación.

## ECS y scheduling objetivo

La evolución se hará detrás de APIs, conservando `GameObject` como DTO de compatibilidad:

```text
Scene DTO → validator → WorldBuilder → RuntimeWorld
                                      ├── entity slots + generations
                                      ├── component stores
                                      ├── hierarchy index
                                      └── event queue
```

El registro de componentes debe ofrecer:

- ID de tipo estable y nombre.
- Metadatos de propiedades, rangos, categorías y tooltips.
- Serializador/deserializador y validador.
- Hooks de add/remove/enable/disable.
- Factory para inspector y scripting.
- Dependencias e incompatibilidades.

El scheduler objetivo tiene fases:

```text
PreUpdate → Update → FixedUpdate (0..N) → LateUpdate → Render
```

Cada sistema declara reads, writes, before/after y disponibilidad editor/play. Sólo sistemas sin conflictos podrán ejecutarse en paralelo. `SystemScheduler` actual puede evolucionar hacia este contrato; el orden manual de `Game` se migra sistema por sistema.

## Render

`RenderBackend` y los command structs existentes son la semilla correcta. La dirección es:

```text
extractores de mundo
  → RenderQueue estable
  → sort/cull/batch
  → RenderBackend
      ├── MacroquadBackend (actual/paridad)
      └── WgpuBackend (futuro, Metal/Vulkan/DX12)
```

El editor consume la misma salida de runtime y añade overlays/gizmos como comandos separados. Ningún componente de juego debe llamar directamente a egui.

## Física

El objetivo es que el mundo mantenga una relación estable entre entity handle y handles de Rapier:

```text
EntityHandle ↔ RigidBodyHandle / ColliderHandle
```

`PhysicsPipeline`, sets, joints, query pipeline y event collector deben persistir entre frames. El bridge actual seguirá funcionando como diagnóstico hasta que cada capacidad tenga paridad y pruebas.

## Editor y undo

El editor será una aplicación sobre APIs runtime/proyecto:

```text
egui panels → EditorCommandBus → commands/transacciones
                              → RuntimeWorld / ProjectStorage / AssetDatabase
```

- La selección pertenece al editor, no a la entidad serializada.
- Los comandos continuos se agrupan y fusionan.
- Las operaciones pequeñas guardan deltas; operaciones estructurales pueden usar snapshots acotados.
- El play mode clona un mundo runtime, no todo el estado del editor.

## Assets

La autoridad debe ser GUID→registro, con path como propiedad mutable:

```text
AssetGuid → AssetRecord { source_path, importer, settings, artifact_hash }
          → dependencies / reverse dependencies
          → imported artifact cache
```

Un rename/move actualiza la ruta bajo transacción sin cambiar GUID. Los documentos referencian GUID y el resolver ofrece placeholders/diagnósticos para faltantes.

## Scripting

- Luau es el lenguaje runtime principal.
- Python, si está disponible, es herramienta confiable sólo de editor mediante `miniforge-editor-tool-v1`.
- C# se modela como plugin editor-only por manifiesto y proyecto `dotnet`; sirve para paneles, diagnósticos, comandos externos y tooling de OpenGL/Metal.
- Rhai permanece únicamente durante migración de proyectos existentes.
- Visual scripting compila a un IR versionado y cacheable; el runtime no recorre el modelo visual.
- Todas las APIs validan handles antes de acceder al mundo.

`engine::automation_bridge` inspecciona el proyecto y devuelve una matriz de
lenguajes/capacidades: Luau, visual graphs, Python tools, plugins C#,
manifiestos nativos y selección de render. Esta capa es intencionalmente de
editor/proyecto: automatiza, diagnostica y extiende el editor sin convertir
gameplay exportado en una mezcla insegura de hosts.

## Plugins

Primera etapa: plugins compilados/features que registran descriptores en interfaces Rust estables. Segunda etapa: ABI nativa versionada con política clara. Un plugin editor no se incluye en runtime salvo declaración explícita.

La API de plugin no debe exponer `Game`, egui ni colecciones internas; debe exponer registrars y handles opacos.

## Fronteras de crates objetivo

| Crate | Puede depender de | No puede depender de |
| --- | --- | --- |
| `miniforge_core` | std, serde mínimo | editor, Macroquad, egui |
| `miniforge_ecs` | core | editor, filesystem |
| `miniforge_project` | core, serde | editor, render |
| `miniforge_assets` | core, project | editor UI |
| `miniforge_scene` | core, ecs, assets | editor UI |
| `miniforge_render` | core, ecs | egui, project dialogs |
| `miniforge_runtime` | core, ecs, scene, assets, render y sistemas | editor |
| `miniforge_editor` | todos los contratos públicos necesarios | internals privados del runtime |
| `miniforge_build` | project, assets, runtime contracts | editor UI |

## Estrategia de extracción

1. Mantener el crate actual como fachada.
2. Crear módulos frontera y tests de dependencia.
3. Extraer crates hoja sin cambiar comportamiento.
4. Reexportar temporalmente paths públicos antiguos.
5. Deprecar por una versión y documentar migración.
6. Eliminar la fachada sólo cuando editor, runtime, tests y proyectos de ejemplo sean equivalentes.

La secuencia ejecutable está en `docs/ROADMAP.md`.
