# Godot 2D + Editor Extraction Matrix

Objetivo: tomar ideas utiles de Godot para MiniForge sin copiar su arquitectura completa ni pegar codigo C++ dentro del motor Rust. Godot esta bajo licencia MIT, pero esta matriz trata sus piezas como patrones de diseno que aterrizan en APIs pequenas, serializables y testeables.

## Fuentes revisadas

- Godot repo: <https://github.com/godotengine/godot>
- Runtime 2D: <https://github.com/godotengine/godot/tree/master/scene/2d>
- Recursos 2D: <https://github.com/godotengine/godot/tree/master/scene/resources/2d>
- Canvas editor: <https://github.com/godotengine/godot/blob/master/editor/scene/canvas_item_editor_plugin.h>
- TileMap editor: <https://github.com/godotengine/godot/tree/master/editor/scene/2d/tiles>
- EditorPlugin API: <https://github.com/godotengine/godot/blob/master/editor/plugins/editor_plugin.h>
- Scene tree editor: <https://github.com/godotengine/godot/blob/master/editor/scene/scene_tree_editor.cpp>
- SpriteFrames editor: <https://github.com/godotengine/godot/blob/master/editor/scene/sprite_frames_editor_plugin.cpp>
- Core Object/ClassDB: <https://github.com/godotengine/godot/tree/master/core/object>
- ResourceLoader: <https://github.com/godotengine/godot/blob/master/core/io/resource_loader.h>
- EditorData: <https://github.com/godotengine/godot/blob/master/editor/editor_data.h>

## Adoptado ahora

| Godot | Patron util | MiniForge |
| --- | --- | --- |
| `Node2D` | Transform local/global, `to_local`, `to_global`, `look_at`, reparent manteniendo transform | `Transform2D` ahora tiene composicion padre/hijo, conversion de puntos, translate, rotate, scale y look_at en grados |
| `CanvasItemEditor` | Herramientas separadas de la UI, grid offset, pixel snap, smart snapping y guias | `SceneView2D` ahora calcula snap puro con grid offset, pixel snap, guias y conversion world/screen |
| `TileMapLayerEditor` + `TileMapPattern` | Seleccion, clipboard, patrones, rotacion/flip y pegado de celdas | `TilemapEditor2D` ahora tiene seleccion rectangular, clipboard, `TilePattern2D`, rotate right, flip H/V y paste |
| `EditorPlugin` | Extension points para toolbar, canvas, inspector, bottom panel, import/export y overlays | `PluginManifest2D` declara `extension_points`, input/overlay forwarding y slots serializables |
| `CanvasItemEditor::draw_over_viewport` | Overlays desacoplados del input del canvas | `SceneView2D::overlay_commands` genera seleccion, guias, colliders, pivots y labels como comandos de dibujo |
| `SceneTreeEditor` | Warnings pasivos de configuracion, incluyendo root transformado | `WorldOutliner2D` genera warnings por root transformado, parent faltante, ciclos y child visible con parent oculto |
| `SpriteFramesEditor` | Duplicar animaciones, mover frames y editar duracion de seleccion | `SpriteFrames2D` ahora duplica animaciones, mueve frames, cambia duracion batch, togglea loop y samplea frames |
| `ClassDB` / `Object` | Registro de clases, propiedades y categorias para crear objetos desde editor | `ComponentRegistry` ahora expone descriptors, busqueda y submenus por categoria |
| `ResourceLoader` | Estados de carga, cache modes, type hints y progreso | `ResourceManager` ahora tiene cola de carga, cache, `ResourceLoadStatus` y `ResourceCacheMode` |
| `Object` signals | Conexiones por nombre y emision de senales entre sistemas | `EventBus` ahora permite `connect`, `disconnect`, `emit_signal` y consultar subscribers |
| `EditorData` | Estado de editor por escenas/documentos y plugins | `EditorTabSession2D` agrupa pestanas por scenes/scripts/assets/output sin cambiar la apertura directa |
| Xcode tooling | Apple workflows externos al runtime del motor | `XcodeBuildPlan` genera planes `xcodebuild`/`open` opcionales para macOS/iOS |

## Siguiente tanda recomendada

1. **TileSet atlas source**
   - Godot separa atlas, alternative tiles, scene tiles, patterns y previews.
   - MiniForge podria dividir `Tileset2D` en fuentes: atlas sprites, scene/prefab tiles y pattern library.

2. **Resource dependency graph**
   - Godot resuelve dependencias y renames desde loaders.
   - MiniForge puede extender `ResourceManager` para graficar dependencias y avisar assets que romperian escenas.

3. **Apple packaging real**
   - El plan Xcode actual es declarativo.
   - Siguiente paso: generar `.xcodeproj`/`.xcworkspace` minimo desde manifest, manteniendo Rust/cargo como fuente principal.

## Reglas para seguir extrayendo

- Preferir contratos pequenos y JSON-friendly.
- Preservar interacciones de un solo click: las mejoras deben ser estado, comandos o validaciones pasivas, no pasos extra obligatorios.
- Mantener Godot como referencia de UX y edge cases, no como dependencia.
- Agregar tests a cada patron importado antes de conectarlo al UI.
- Si algun dia se copia codigo literal MIT, incluir el aviso de copyright/licencia en el archivo afectado.
