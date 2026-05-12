# MiniForge Patch Notes

## 0.7.0 Production Editor Update

- Versionado del motor a `0.7.0 - Production Editor Update`.
- Inspector editable real con campos numericos, booleanos y texto para transform, identidad y datos de componentes.
- Add/Remove Component desde el Inspector con validacion y proteccion de componentes core.
- Undo/redo basado en Command Pattern para crear, borrar, duplicar, editar inspector, mover entidades, drop de assets y pintar tilemaps.
- Scene Gizmos: move, rotate, scale, snap to grid y bounding boxes visibles.
- Drag and drop desde Content Browser para sprites, prefabs, materiales, sonidos y visual graphs.
- Asset Preview con GUID, path, labels, import settings, dependencias, reverse dependencies, warnings y preview visual de imagen/audio/material.
- Export Runtime con `build/<profile>/<project>/`, perfiles debug/release, `runtime_manifest.json`, `build_info.json` y deteccion de assets faltantes.
- Input Visual System con acciones Move, Attack, Jump, Interact, Pause, Select, Command y CameraPan.
- Tile Palette + Brushes: pencil, eraser, fill, rectangle y collision brush con undo/redo.
- Estabilidad: exporter evita copiar `build/` sobre si mismo, nuevos flujos devuelven Result/Option y los errores se reportan en consola.
- Tests nuevos para inspector commands, tile brushes, asset preview/drop/export e input map visual.

## 0.6.x Rust Engine Upgrade

- Port principal del editor/runtime a Rust con `macroquad`.
- Scene view, jerarquia, inspector, consola, browser, prefabs y profiler.
- Componentes avanzados: stats, inventory, equipment, abilities, AI, nav, RTS, fog, production, saveable, dialogue, quest, tweens, timers y tilemap collider.
- Starters funcionales para TopDown, Platformer y RTS.
- Visual scripting runtime con graphs `.mfgraph`.
- Prefab workflow con variantes y overrides.
- Asset database con GUIDs, import settings, labels, compatibility warnings y dependency graph.

## 0.6.x RTS/Nav Upgrade

- Integracion de `pathfinding` para A* y Dijkstra.
- Flow fields para squads RTS.
- Rutas threat-aware e influence maps.
- `ThreatSource` e `InfluenceSource`.
- RTS skirmish con obstaculos, fog, enemigos, recursos y produccion.

## 0.6.x Serious Engine Pass

- `ArchetypeLibrary` para instanciar entidades listas de juego.
- `SquadMember`, `RtsBrain` y `ProductionRecipeBook`.
- Auto-queue de unidades por recetas.
- Combate tactico RTS con target acquisition, chase, attack, cooldowns, experiencia y destruccion.
- APIs de transform: mover X/Y, set position, scale, size, rotate y look-at.
- APIs de assets: sprite entities, audio source, sprite imports, sound cues y materiales.
- FileBrowser backend con scan, stats, rename, move, duplicate, import, create folder y asset generators.
- Play Mode con snapshot seguro, contador de frames y restauracion al salir.
- Guardado integral de proyecto: escena, assets, manifest y estado del proyecto.
- Menu superior del editor: File, Create, View, Project y RTS.
- Content Browser con acciones para sprites, sonidos, materiales, graphs y prefabs.

## 0.7.1 Beta Tecnica (Rust)

- Binarios `miniforge_editor`, `miniforge_runtime` y alias `miniforge`; `RuntimeManifestLoader` y runtime player sin UI de editor.
- `SceneSaveManager`: merge incremental de entidades, backup `.scene.bak`, escritura atomica y recuperacion si falla el guardado.
- UI Canvas de escena (`ui_canvases`), anchors, preview responsive en Inspector sin seleccion, acciones en menu Create.
- `SpriteSheetImporter` (PNG + grid), `AtlasImporter`, `WaveformCache`; tipos `SpriteSheet`/`Atlas` en Asset Database y detalles extra en Asset Preview.
- `PackagingManager`, menu File (Package/Recover autosave), autosave atomico, validacion de proyecto al abrir.
- Tests en `tests/beta_features.rs`.

## Proximo Parche Recomendado

- Edicion directa de elementos UI en la vista de escena.
- Packaging con instaladores firmados por plataforma.
- Sprite sheet importer para JPEG/WebP ademas de PNG.
