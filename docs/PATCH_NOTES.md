# MiniForge Patch Notes

# 0.9.1.1 Interface Overhaul Patch

- Mini parche enfocado solo en interfaz: no agrega nuevos sistemas de gameplay ni cambia la logica del motor.
- Redisenado el lenguaje visual del editor con fondos profundos, superficies unificadas, sombras suaves, bordes consistentes y acentos `cyan/teal`.
- Launcher actualizado a una apariencia moderna estilo mac: fondo con profundidad, panel principal oscuro, text fields integrados, recientes y notas del parche mas claras.
- Top bar, menus, botones, status bar, Command Palette y ventanas flotantes usan el mismo sistema visual compartido.
- Hierarchy e Inspector ahora se sienten como paneles profesionales conectados al motor, con headers, filas activas y estados visuales mas legibles.
- Browser, Graph editor, Code editor y fondos de escena reciben tratamiento visual renovado sin tocar su comportamiento.
- Botones y campos de busqueda fueron redibujados con gradientes, estados hover/active y bordes mas limpios.

# 0.9.2 Game Creation API Update

- Versionado del motor a `0.9.2 - Game Creation API Update`.
- GameAPI ampliada para juegos 2D/RTS: `has_item`, `transfer_item`, `inventory_space_left`, `equip_item`, `unequip_item`, `resource_amount`, `can_afford`, `spend_cost`, `add_resources`, `transfer_resource`, recetas de produccion, preferred recipe, gathering/deposit de worker cargo, progreso de quest y habilidades con cargas/cooldown.
- Blueprints nuevos para sistemas complejos: `InventoryRemove`, `BranchItem`, `EquipItem`, `EconomyAdd`, `EconomySpend`, `BranchResource`, `AddProductionRecipe`, `SetPreferredRecipe`, `QueuePreferredRecipe`, `AddQuest`, `QuestProgress`, `TriggerAbility` y `RechargeAbility`.
- Templates nuevos: `InventoryEconomyLoop`, `QuestAbilityLoop` y `RTSProductionEconomy` para prototipar compras, inventarios, quests, habilidades, wallets y colas de produccion sin escribir codigo del motor.
- Busqueda difusa con `strsim` en templates, node catalog y Command Palette para encontrar herramientas aunque el nombre se escriba incompleto o con errores.
- Command Palette 0.9.2 conectada a crear/adjuntar los nuevos graphs, abrir ventanas flotantes, importar/exportar paquetes, crear assets y cambiar workspaces.
- Launcher actualizado con notas de parche 0.9.2 y resumen de la orientacion a desarrollo real de juegos.
- Documentacion nueva de aprendizaje en `docs/GETTING_STARTED_0.9.2.md`, mas README y Engine Guide actualizados.

# 0.9.1 Creation Workflow Update

- `Runtime2DSystem` agregado al loop: controller top-down/platformer, jump buffer, coyote time, dash, input por tag y contadores de profiler.
- Colision runtime contra grid/tilemap para personajes con `CharacterController2D` y `Rigidbody2D`.
- `Checkpoint` ahora activa respawn por proximidad, guarda posicion de reaparicion y recupera vida/velocidad al caer o morir.
- `CameraFollow` ahora usa suavizado, viewport configurable, dead zone, zoom smooth y soporte de shake.
- `GameplaySystem` genera loot pickups desde `LootTable` al destruir entidades y evita destruir actores respawneables.
- Savegame v2 restaura entidad completa por `Saveable.save_key`, incluyendo componentes, inventario, vida y entidades persistentes faltantes.
- Paneles `Scenes` y `Sprites` del editor completados para cargar escenas, stack/additive y pintar sprites desde el panel inferior.
- Ventanas flotantes movibles para programar scripts Rhai, editar blueprints visuales, buscar blueprints y abrir Play Window.
- Jerarquia con menu contextual: seleccionar, mover filas, parentar la seleccion, limpiar parent y eliminar por click derecho.
- Prefabs con acciones visibles de apply/revert/detach desde el panel del editor.
- Consola estructurada por severidad/canal con resumen de warnings/errores y limpieza selectiva.
- Import/export de proyecto como paquete `.mfpkg.zip` usando `zip` y `walkdir`.
- Blueprint runtime ampliado con `BroadcastEvent`, `Gate`, `OpenGate`, `CloseGate`, `ToggleGate` y `FlipFlop`, ademas de nodos de vida, movimiento, inventario, cooldown, estado, quest y UI.

# 0.9.0 Productivity & Blueprint Update

- Launcher nuevo estilo mac con crear, abrir, reparar/exportar desde el flujo base y selector libre de ubicacion por ruta.
- Apartado de notas del parche en el launcher con historial de cambios visible antes de abrir un proyecto.
- Programacion visual ampliada con catalogo buscable de nodos: vida, movimiento, fisica, ramas, variables, UI, spawner y componentes.
- Blueprints con pines `exec`, `true` y `false`, conexiones mas claras y validacion de nodos duplicados, tipos desconocidos y enlaces rotos.
- Ventana flotante Blueprint Library para buscar plantillas, crear graphs y adjuntarlos al seleccionado.
- Pestañas de scripts/graphs mejoradas con cierre de tabs, reapertura ordenada y editor desacoplable para Rhai o `.mfgraph`.
- Runtime visual conectado a sistemas reales: `Health`, `Rigidbody2D`, `Spawner`, `Blackboard`, `Animator`, `UIElement` y estado de entidad.

# 0.8.0 Developer Stability Update

- Estabilizado el flujo completo crear/abrir/editar/guardar/cargar/Play Mode/export runtime.
- `engine_config.json` tiene defaults, migracion, backup y recuperacion desde backup/corrupt.
- Consola con niveles y salida a `logs/miniforge.log`.
- Panel Programming puede abrir y guardar scripts Rhai, visual graphs, escenas, prefabs y JSON sin salir del motor.
- Content Browser reorganizado con Sources, filtros, busqueda, grid visual y detalles de asset.
- Visual graphs editables como nodos conectables con pines de entrada/salida.
- Validacion reforzada para escenas, prefabs, assets, Rhai y visual graphs.
- GitHub Actions agregado para fmt, check, clippy y test.

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
