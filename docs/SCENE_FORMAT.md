# MiniForge Scene Format

Estado real: schema 1, implementado el 2026-06-23. Las escenas usan JSON con extensión `.scene`.

## Cabecera

```json
{
  "format": "miniforge.scene",
  "schema_version": 1,
  "engine_version": "0.9.3",
  "version": "0.9.3"
}
```

- `format` identifica el tipo de documento.
- `schema_version` controla validación y migraciones.
- `engine_version` indica qué motor guardó el documento.
- `version` se mantiene por compatibilidad con proyectos anteriores; no controla migraciones.

## Campos de schema 1

| Campo | Tipo | Uso |
| --- | --- | --- |
| `scene_name` | string no vacío | Nombre lógico de escena. |
| `entities` | array | Entidades serializadas. |
| `camera` | object | Posición y zoom del editor/runtime. |
| `tilemap_layers` | array/object | Datos de capas según el tilemap actual. |
| `tiles` | array/object | Alias legacy conservado. |
| `grid` | object/null | Dimensiones, tile size y chunk size. |
| `ui_canvases` | array | UI runtime de la escena. |
| `settings` | object | Ajustes extensibles. |
| `control_groups` | object | Grupos de control editor/RTS. |
| `editor_view_settings` | object | Datos exclusivos de vista del editor. |

El loader exige raíz JSON object, cabecera compatible, `scene_name`, `entities`, `camera` y `ui_canvases` válidos antes de materializar entidades.

## Migración legacy 0→1

Un documento sin `schema_version` se considera schema 0:

1. `objects` se renombra a `entities` si corresponde.
2. Se añaden cabecera y defaults ausentes.
3. `tilemap_layers` se inicializa desde `tiles`.
4. El documento sólo se migra en memoria al abrirlo.
5. El archivo se actualiza al próximo guardado normal, bajo backup atómico.

La migración es idempotente. Un schema mayor que 1 se rechaza con error; nunca se sustituye silenciosamente por un backup viejo. JSON corrupto o schema inválido no futuro sí puede recuperarse desde `.scene.bak`.

## Persistencia

`SceneManager`, `SceneSaveManager` y `AutosaveManager` usan `ProjectStorage`, con temp único, sincronización, tres generaciones de backup y rollback. Los backups son:

```text
main.scene.bak
main.scene.bak.1
main.scene.bak.2
```

Los golden tests viven en `tests/fixtures/formats/` y las pruebas de integración en `tests/schema_versioning.rs`.
