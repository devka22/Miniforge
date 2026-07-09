# MiniForge RuntimeWorld

Estado: primera separación de Fase 2, implementada el 2026-06-23.

## Propósito

`RuntimeWorld` es el propietario canónico de las entidades y su índice espacial. Sustituye el diseño anterior, donde `Game.units` se clonaba completamente a `World.entities` en cada frame.

```text
Game
└── runtime_world: RuntimeWorld
    ├── units: Vec<GameObject>        autoridad única
    ├── spatial_index: SpatialIndex
    ├── structural_revision
    └── indexed_revision
```

`Game` implementa `Deref<Target = RuntimeWorld>` durante la ventana de compatibilidad, por lo que lecturas y mutaciones sencillas con `game.units` siguen funcionando. Código nuevo debe usar `game.runtime_world` o los métodos de `RuntimeWorld`, especialmente cuando necesita prestar simultáneamente otro servicio de `Game`.

## APIs actuales

- `entity` / `entity_mut`: acceso validado por ID.
- `entities` / `entities_mut`: slices del almacenamiento canónico.
- `replace_entities`: reemplazo de escena con revisión e índice reconstruido.
- `push`: inserción que rechaza IDs duplicados.
- `remove`: eliminación con actualización espacial.
- `query_radius`: query acelerada por tag/layer.
- `rebuild_index` / `index_is_current`: control explícito de coherencia.
- `validate`: IDs duplicados, parents inexistentes y ciclos jerárquicos.

El guardado de escena rechaza un mundo inválido antes de tocar el archivo.

## Compatibilidad

El alias de tipo `World` permanece deprecado para imports antiguos. El snapshot `World.entities` fue eliminado porque duplicaba memoria y podía divergir del estado runtime.

## Siguiente frontera

`Game` todavía contiene servicios exclusivos del editor como historial, docking, ScriptEditor y SpriteEditor. La siguiente extracción será `EditorServices`, opcional en runtime, antes de activar una build sin dependencias egui.
