# MiniForge - Editor y flujo de uso

Esta guía explica la operación completa del editor Qt, desde abrir un proyecto hasta probarlo,
recuperarlo y exportarlo. Para contratos de datos consulta
[Datos, scripting y APIs](DATOS_SCRIPTING_Y_APIS.md); para compilar el editor consulta
[Desarrollo, build y extensión](DESARROLLO_BUILD_Y_EXTENSION.md).

## Abrir El Motor

El editor soportado es exclusivamente Qt/C++/QML. Desde la raiz:

```bash
scripts/run-editor projects/DefaultProject
```

`scripts/run-editor` configura y recompila incrementalmente antes de abrir. El
ejecutable nativo acepta flujos de arranque utiles para produccion y recuperacion:

```bash
scripts/run-editor --project projects/DefaultProject --workspace 2D
scripts/run-editor --project projects/DefaultProject --workspace Scripting
scripts/run-editor --project projects/DefaultProject --safe-mode
scripts/run-editor --launcher
scripts/run-editor --project projects/DefaultProject --runtime
scripts/run-editor --project projects/DefaultProject --headless-once
scripts/run-editor --create-project projects/MiJuego --template Platformer
scripts/run-editor --create-project projects/MiJuego --template RTS --force
scripts/run-editor --reset-layout projects/DefaultProject
```

| Opción | Uso |
|---|---|
| ruta posicional o `--project <ruta>` | Abrir un proyecto existente |
| `--workspace <nombre>` | Entrar en un workspace concreto |
| `--create-project <ruta>` | Crear y abrir un proyecto nuevo |
| `--template <nombre>` | Con `--create-project`: Empty, TopDown, Platformer o RTS |
| `--safe-mode` | Bloquear scripts, graphs y plugins durante la recuperación |
| `--launcher` / `--no-launcher` | Mostrar Project Launcher o abrir el proyecto directamente |
| `--runtime` | Entrar en Play Mode inmediatamente después de abrir |
| `--headless-once` | Simular un paso determinista sin ventana y terminar |
| `--force` / `--overwrite` | Permitir plantilla sobre un directorio ya existente |
| `--reset-layout` | Descartar la geometría y los docks guardados |
| `--screenshot <png>` | Capturar el workbench inicial y terminar |
| `--help` / `--version` | Mostrar ayuda o versión |

En macOS se genera `build/editor-qt/MiniForge.app`; en Linux y Windows se usa
`build/editor-qt/miniforge_qt_editor`. `--safe-mode` abre el proyecto con scripts,
graphs y plugins desactivados y esa condicion se muestra en Runtime Health.
`--screenshot <ruta.png>` captura el workbench inicial y termina, lo que permite
QA visual automatizado aun con `QT_QPA_PLATFORM=offscreen`.

El runtime exportado se ejecuta con:

```bash
cargo run --no-default-features --features runtime --bin miniforge_runtime \
  -- --build path/to/export
```

Para tareas sin ventana:

```bash
cargo run --no-default-features --features runtime --bin miniforge_headless \
  -- projects/DefaultProject 120
cargo dev -- doctor
cargo dev -- project projects/DefaultProject
```

## Entradas Del Editor

El editor Qt/QML es la unica superficie visual. El antiguo target Rust/macroquad/egui fue retirado. Blueprint, animation, tilemaps, UI Designer, settings, prefabs, launcher, Luau recovery/debugger, viewport multi-edit y los docks Asset/Profiler/Dependency tienen flujos Qt validados. Rust sigue siendo dueño del backend no visual mediante `EditorCore` y `editor_ffi`.

La ruta Qt vive en:

- `editor-cpp`: ventana, bridge C++ y modelos Qt.
- `editor-qml`: UI QML con tema oscuro, paneles y componentes.
- `include/miniforge_editor_bridge.h`: ABI C publica.
- `src/editor_ffi.rs`: implementacion Rust del ABI.
- `src/engine/editor_core.rs`: API de alto nivel usada por el bridge.

La ventana Qt actual expone menu, workspaces, hierarchy, command palette, viewport nativo, inspector, content browser, consola, readiness, salud runtime, Luau Studio, Sprite Studio, Prefab Studio, Project Settings, Project Launcher, Visual Graph, Animation Timeline, Tilemap/Terrain, UI Designer, ForgeAI y build/export.

La paridad y los gates para retirar piezas historicas estan documentados en `docs/QT_EDITOR_MIGRATION.md`.

## Workbench, Workspaces Y Layout

La ventana es un `QMainWindow` con un viewport central y paneles `QDockWidget`. Un panel se puede
mover, tabificar, flotar, cerrar y recuperar desde `View > Panels`. MiniForge guarda por usuario
la geometría de la ventana, el último workspace y un layout independiente para cada workspace.
`View > Reset Current Workspace` reinicia solo el preset activo; `--reset-layout` reinicia todos
antes del arranque.

| Workspace | Enfoque |
|---|---|
| 2D | Scene/Game View, jerarquía, inspector y contenido |
| Scripting | Luau Studio, Visual Graph, consola y problemas |
| Animation | Timeline, Sprite Studio y preview |
| World | Tilemap, terrain, escenas y herramientas espaciales |
| UI | UI Designer y propiedades de canvas/widgets |
| Prefab | Prefab Studio, jerarquía e inspector |
| Project | Settings, Launcher, Operations y readiness |
| Assets | Content Browser, Asset Management y dependencias |
| Profiler | Tiempos, budgets, counters y Runtime Health |
| Automation | Python tools y flujos declarativos |
| AI | ForgeAI doctor, planes, validación y tests |
| Build | Perfiles, validación y export |
| Debug | Console, Luau debugger y salud runtime |
| Minimal | Viewport y controles esenciales |

Los primeros diez workspaces se abren con `Ctrl+1` a `Ctrl+0`. La barra superior añade acceso
rápido a Save, Add, Content, selector de workspace, herramientas Q/W/E/R, Guides, HUD, Play,
Stop y Panels.

### Atajos Globales

| Atajo | Acción |
|---|---|
| `Ctrl/Cmd+O` | Abrir proyecto |
| `Ctrl/Cmd+S` | Guardar escena |
| `Ctrl/Cmd+Shift+S` | Save All/proyecto |
| `Ctrl/Cmd+Z` / `Ctrl/Cmd+Shift+Z` | Undo / Redo |
| `Ctrl/Cmd+Shift+P` | Command Palette |
| `Ctrl/Cmd+Shift+O` | Scene Browser |
| `Ctrl/Cmd+,` | Project Settings |
| `Ctrl/Cmd+D` | Duplicar selección |
| `F2` | Renombrar selección |
| `Delete` | Eliminar selección o asset según contexto |
| `Esc` | Limpiar selección |
| `Ctrl/Cmd+Shift+G` / `Ctrl/Cmd+Shift+U` | Agrupar / desagrupar |
| `F5` / `Shift+F5` | Entrar / salir de Play Mode |
| `Ctrl/Cmd+Alt+G` | Guides del viewport |
| `Ctrl/Cmd+H` | HUD del viewport |
| `F11` | Pantalla completa |

Con Scene View enfocado, `Q`, `W`, `E` y `R` seleccionan Select, Move, Rotate y Scale; `F`
enfoca la selección, `Home` restablece la cámara y `Space` activa pan temporal.

Los assets del Content Browser se pueden arrastrar al Scene View. Una textura crea un objeto en
la posición de drop cuando no hay un objetivo; un sprite, material, script Luau o Visual Graph se
asigna al objeto bajo el cursor o a la selección activa. Con `Collision Overlay` habilitado,
`Alt+click` añade o arrastra vértices de colisión con snap y `Alt+Shift+click` elimina el vértice
bajo el cursor; cada gesto se registra como un único paso de undo.

## Conceptos Del Editor

Un proyecto MiniForge contiene:

- `project.json`: metadata del proyecto.
- `engine_config.json`: configuracion del motor/editor.
- `manifest.json`: resumen de runtime/assets/scripts.
- `assets/`: sprites, audio, data y prefabs.
- `scripts/`: Luau y visual graphs.
- `saves/scenes/`: escenas `.scene`.
- `settings/`: runtime, input, tags, layers, build settings y build profiles.
- `logs/`: logs del motor.
- `plugins/`: plugins de editor/proyecto.
- `builds/` y `exports/`: salidas de build/export.

`AssetTools::ensure_project_folders` crea la estructura base y archivos faltantes. Tambien limpia temporales viejos de escritura atomica.

## Flujo Base

1. Crear o abrir proyecto.
2. Verificar que `engine_config.json.start_scene` apunte a una escena existente.
3. Refrescar asset database.
4. Crear o abrir escena.
5. Crear entidades, sprites, prefabs, UI o tilemaps.
6. Agregar componentes desde el registry.
7. Adjuntar Luau o visual graph si aplica.
8. Usar Play Mode para probar.
9. Revisar consola, readiness y project validator.
10. Guardar escena/proyecto.
11. Exportar o empaquetar.

## Paneles Y Herramientas

Paneles principales:

- Hierarchy / World Outliner: muestra entidades, padres, hijos, seleccion, visibilidad y bloqueo.
- Inspector / Details: edita campos base y datos de componentes.
- Content Browser: lista assets con GUID, tipo, labels y dependencias.
- Asset Management: renombra, duplica, mueve, importa archivos externos con sidecar y envia borrados confirmados al trash recuperable del proyecto. Las rutas se validan dentro de carpetas administradas y cada operacion refresca metadata/GUIDs.
- Asset Dependency Graph: muestra assets, aristas dependency-to-consumer, reverse counts, orden de build, ciclos y referencias no resueltas. `Rebuild` vuelve a escanear archivos antes de publicar el snapshot.
- Profiler: ordena tiempos reales de sistemas, calcula porcentaje de frame, uso del presupuesto, FPS, counters y metricas del runtime.
- Project Operations: exporta/importa paquetes `.mfpkg.zip`, crea distributables, configura/ejecuta/recupera autosave, controla checkpoints de sesion y prepara Play/Build externo.
- Scene View: viewport 2D con grid, zoom, pan, snap, seleccion, gizmos y overlays.
- Command Palette: busqueda fuzzy de comandos del editor.
- Console: logs por canal y severidad.
- Readiness: score de preparacion por sistema, gaps y acciones.
- Script Editor: documentos Luau, diagnostics, symbols, completions, minimap, search y code actions.
- Sprite Studio: pixel art, slicing, palette/ramp, atlas y export de spriteframes.
- Animation Timeline: tracks editables, acciones, undo/redo y persistencia de secuencia.
- Problems Panel: issues de validacion 2D.
- UI Designer: jerarquia, canvas, propiedades, acciones, undo/redo, validacion y persistencia.

Otros paneles especializados son Scene Browser, Object Studio, Luau Studio, Visual Graph,
Tilemap Editor, Prefab Studio, Project Launcher, Project Settings, Runtime Health, Build & Export,
Python Tools y ForgeAI. Cada workspace muestra un subconjunto, pero ningún panel se pierde al
cambiar de workspace: se puede recuperar desde el menú Panels.

El Play externo prepara primero un export o distributable validado. Rust publica un plan con ejecutable, carpeta, argumentos y warnings; Qt lo ejecuta en un `QProcess` separado con `--build <artifact>` y permite detenerlo sin cerrar ni bloquear el editor.

El `EditorShell` coordina comandos de panel/document/runtime/plugin mediante un `EditorCommandBus`. Un panel puede pedir abrir, guardar, cerrar documento, mostrar/ocultar panel, reportar problemas o solicitar quit. La solicitud de quit solo se acepta desde `Shell`.

## Comandos De Editor

La capa Qt llama comandos por ID mediante `editorController.executeCommand(commandId)`. Ejemplos visibles en QML:

- `project.save`
- `scene.save`
- `scene.audit_tree`
- `scene.pack_selected`
- `edit.undo`
- `edit.redo`
- `project.audit`
- `assets.refresh`
- `luau.validate_scripts`
- `render.write_2d_profile`
- `sprite.new_pixel_art`
- `sprite.create_hero_template`
- `sprite.export_frames`
- `sprite.export_atlas_pages`
- `sprite.optimize_palette`
- `luau.new_controller`
- `object.create_node2d`
- `object.create_sprite_actor`
- `object.create_camera`
- `object.create_camera_texture2d`
- `object.create_post_process_volume2d`
- `object.create_survival_actor2d`
- `object.create_survival_environment2d`
- `object.create_hybrid_world2d3d`
- `object.create_hybrid_billboard3d`
- `object.create_area2d`
- `object.create_character_body2d`
- `object.create_ui_text`

`EditorCore` tambien expone datos paginados para entidades, assets, comandos, consola, readiness y snapshots de viewport.

`object.create_camera_texture2d` crea una cámara WGPU, su `RenderTexture2D` sampleable y el sprite
que muestra el resultado en una sola acción. El objeto se reconoce por el glifo `RT` del viewport;
Inspector expone resolución y modo de actualización (`always`, `once`, `manual`). El enlace
`render-target://...` se persiste dentro de la escena: no requiere un script del juego ni volver a
conectar la textura después de guardar. En Authoring Hub, el preset **Camera to Texture 2D** añade
un switch **Include UI**: al activarlo la cámara captura también `UIElement`, Canvas/UI Designer y
texto retained con clipping adaptado a la resolución del target.

`object.create_post_process_volume2d` crea un volumen global sin sprite ni colisión. El glifo `FX`
identifica el objeto y el Inspector controla el compositor WGPU físico. Los presets Cinematic,
Horror, Pixel y Damage ofrecen puntos de partida sin escribir WGSL.

El menú **Survival** crea actores con vida, necesidades, lesiones, inventario, equipamiento,
fabricación, control, guardado, animación y ancla híbrida; también crea ambientes globales con
temperatura, viento, lluvia y exposición. El menú **Hybrid 2D + 3D** crea el mundo de presentación
o actores billboard sincronizados. Los glifos `WX`, `2½D` y `BB` muestran sus parámetros esenciales
directamente en el viewport. Véanse [Componentes de supervivencia](SURVIVAL_COMPONENTS.md) y
[Escenas híbridas 2D + 3D](HYBRID_2D_3D.md).

La Command Palette fusiona comandos del backend y acciones propias del shell Qt. La búsqueda es
fuzzy, se navega con teclado y respeta si un comando requiere proyecto o selección. Es la vía más
rápida para acceder a acciones poco frecuentes sin recorrer menús.

## Proyectos Y Templates

`ProjectTemplates` soporta:

- `empty`: escena vacia.
- `rts`: mapa RTS, scripts, graphs, economy/production data y prefabs base.
- `topdown`: controller, enemy brain, graph, bindings y escena topdown.
- `platformer`: motor, jump controller, graphs y escena.
- `demo` / `complete_demo` / `playable_demo`: menu, escena jugable, scripts, visual graph, particulas, shaders, materiales, audio, prefabs y save data.
- `action_rpg`: combate, enemigo, quests, loot, player/enemy/NPC prefabs.
- `survival`: day/night, crafting, recursos, campfire, recipes y biomas.

La recomendacion es empezar con `topdown`, `platformer` o `rts` si se quiere validar gameplay rapido.

## Crear Entidades

`GameObject::new` crea un objeto con:

- `Transform`
- `Selectable`
- `SpriteRenderer`
- `Collider2D`

Los comandos modernos de `EditorCore` para escena agregan ademas `Node2D` y `SceneTreeNode`, de modo que el objeto entra al indice de rutas/grupos/senales sin romper compatibilidad con escenas antiguas.

`GameObject::new_unit` crea una unidad con:

- `Transform`
- `Selectable`
- `RTSMovement`
- `SpriteRenderer`
- `Collider2D`

Luego se pueden agregar componentes desde `ComponentRegistry`. Para un jugador topdown completo, el motor ya tiene helpers de foundation que agregan componentes como `Health`, `Stats`, `Rigidbody2D`, `CharacterController2D`, `CameraFollow`, `Inventory`, `Equipment`, `Ability`, `QuestLog`, `Saveable`, `Cooldown`, `StatusEffects`, `Interaction`, `CombatTarget`, `NavAgent` y `VisualScript`.

## Escenas

Las escenas viven en `saves/scenes/*.scene`. El formato actual es:

- `format`: `miniforge.scene`
- `schema_version`: `1`
- `engine_version`
- `scene_name`
- `mode`
- `active_tool`
- `camera`
- `grid`
- `tiles` / `tilemap_layers`
- `settings`
- `entities`
- `ui_canvases`

Al cargar una escena, el editor:

- instancia `GameObject` con IDs preservados.
- asigna `scene_name`.
- sincroniza componentes.
- limpia seleccion.
- aplica grid, tilemaps, camara, herramientas y UI.
- reconstruye el mundo y toma snapshots de history.

Scene Browser separa las operaciones sobre el archivo de las operaciones sobre el runtime:

| Acción | Resultado |
|---|---|
| New | Crea una escena versionada |
| Duplicate | Copia la escena con un nombre nuevo |
| Save | Persiste la escena activa |
| Restart | Recarga la escena activa desde su estado guardado |
| Load | Sustituye la escena principal |
| Add | Carga una escena de forma aditiva |
| Push | Agrega la escena al stack de navegación |
| Pop | Vuelve a la escena anterior del stack |
| Unload | Retira una escena aditiva |

Antes de Restart, Load o cambio de proyecto, guarda los documentos dirty. Las transiciones de
escena del runtime respetan el `start_scene` configurado y los documentos aditivos activos.

El serializador migra escenas legacy con `objects` hacia `entities`. Versiones futuras se rechazan para evitar perdida silenciosa.

### SceneTree Y NodePath

El editor puede construir un `SceneTreeIndex` desde las entidades abiertas:

- rutas absolutas: `/World/Player/Camera`.
- rutas relativas: `../HUD`, `./WeaponSocket`.
- grupos: `GroupMembership.groups`, `editor_group`, `tag:*` y `layer:*`.
- auditoria: `scene.audit_tree` escribe resumen y warnings en consola.

Las claves `node_path`, `target_path`, `root_path`, `owner_path` y `parent_path` se validan al guardar. Si una conexion `SignalEmitter` apunta a un target inexistente o no define metodo, la validacion lo reporta antes de exportar.

### PackedScene2D

`scene.pack_selected` toma la entidad seleccionada como root, empaqueta root+hijos en `assets/packed_scenes/*.mpscene.json`, preserva la jerarquia y deja el asset index listo para usarlo desde content browser. Al instanciar, `PackedScene2D` remapea IDs y puede aplicar offset de posicion y prefijo de nombre.

## Prefabs

Los prefabs viven en `assets/prefabs/*.prefab`. El formato actual es:

- `format`: `miniforge.prefab`
- `schema_version`: `2`
- `guid`
- `prefab_name`
- `entity`
- `scripts.required`
- `settings.required`
- `dependencies`
- `metadata`

Al guardar un prefab:

- se preserva GUID si el archivo ya existe.
- se calcula `scripts.required` desde `script`, `scripts` o componentes de scripting.
- se agregan settings base: input map, tags, layers y runtime config.
- se escribe con `ProjectStorage::write_json_atomic_with_backup`.
- la entidad queda marcada como instancia con `prefab_source`, `prefab_guid` e `is_prefab_instance`.

Al instanciar un prefab, se carga el documento, se migra si aplica, se valida, se genera entidad con nuevo ID y se coloca en la escena.

## Assets

El `AssetDatabase` escanea:

- `assets/`
- `scripts/`
- `saves/scenes/`
- `settings/`
- `assets/prefabs/`

Cada asset obtiene:

- GUID estable.
- ruta relativa.
- nombre.
- tipo.
- tamano.
- fecha modificada.
- hash de contenido.
- import settings.
- labels.
- compatibilidad.
- dependencias.

Si un archivo se movio fuera del editor, el hash permite reconciliarlo y conservar su GUID. Las referencias deben usar GUID siempre que sea posible.

### Content Browser

Content Browser es el explorador diario del proyecto. Tiene árbol lateral, breadcrumbs, botón
Up, búsqueda, filtro por tipo, orden y modos grid/lista. El grid muestra miniaturas de imágenes
cuando puede generarlas; la lista prioriza nombre, tipo, tamaño y fecha. La búsqueda puede
limitarse a la carpeta actual o recorrer las raíces administradas.

La selección soporta un elemento, toggle con `Ctrl/Cmd` y rango con `Shift`. Las operaciones de
rename, duplicate, move y trash aceptan esa selección y refrescan después el Asset Database. Un
drag & drop sobre una carpeta administrada mueve el asset mediante el backend, no mediante una
mutación QML directa.

Desde `New` se pueden crear:

| Tipo | Destino/resultado habitual |
|---|---|
| Folder | Subcarpeta en la raíz administrada actual |
| Luau | `scripts/*.luau` con plantilla válida |
| Scene | Documento de escena versionado |
| Prefab | `assets/prefabs/*.prefab` |
| JSON | Documento de datos genérico |
| Resource Config | Configuración versionable del proyecto |
| Material | Asset de material 2D |
| Shader | Fuente de shader |
| Visual Graph | `scripts/visual_graphs/*.mfgraph` |
| UI | Canvas/UI document |
| Tilemap | Documento de tilemap |
| SoundCue | Configuración de audio |

Los archivos de texto compatibles se pueden editar en el panel integrado y guardar de forma
atómica, o abrir en el editor configurado por el sistema. `Ctrl/Cmd+S`, `Ctrl/Cmd+D`, `F2` y
`Delete` operan sobre el contexto activo. El menú contextual añade import, preview/open,
duplicate, move, rename y trash.

Las mutaciones se confinan a `assets`, `scripts`, `scenes`, `saves`, `settings`, `components`,
`systems`, `plugins` y `templates`. El backend normaliza rutas, rechaza traversal/symlinks que
escapen del proyecto, limita lecturas de texto y evita sobrescrituras silenciosas. El borrado
envía el contenido a trash recuperable; no equivale a un `remove` irreversible.

### Asset Management Y Dependencias

Asset Management está orientado a operaciones masivas y a importación externa con sidecars
`.mfimport.json`. Dependency Graph muestra relaciones dependency-to-consumer, reverse counts,
ciclos, referencias sin resolver y un orden de build. Usa `Rebuild` después de cambios externos
o antes de diagnosticar por qué un recurso no entra en el export.

## Sprite Studio Y Pipeline 2D

El pipeline de assets 2D soporta:

- perfiles de importacion: Pixel Art, Smooth Sprite, UI Texture, Audio Event y Copy.
- sidecars `.mfimport.json`.
- fingerprints de fuente.
- jobs de reimport por fuente nueva, fuente cambiada, importer cambiado, generado faltante, dependencia cambiada o manual.
- sprite sheets con slices.
- atlas y paginas.
- waveform preview para WAV PCM simple.

Herramientas disponibles en el pipeline:

- slicing de spritesheet.
- export de `.spritesheet.json` y `.spriteframes`.
- atlas pages.
- palette ramp.
- previews de audio.

El widget nativo actual permite crear lienzos 16x16 o 32x32, dibujar con colores primario y
secundario, alternar grid, ajustar zoom/fit, deshacer, rehacer y limpiar. Las transformaciones
incluyen flip horizontal/vertical, rotación de 90 grados, crop y outline. El resultado se guarda
como PNG.

Para spritesheets se configuran ancho/alto de frame y FPS. El overlay enseña los cortes, el
scrubber cambia de frame y Play/Pause reproduce la secuencia sin salir del editor. Los comandos
de pipeline pueden generar `.spriteframes`, páginas de atlas y palette ramps.

## Animation Timeline

Animation Timeline crea y edita secuencias con tracks, keys y cursor temporal. Incluye playback,
selección de keys, tangentes/curvas, undo/redo y persistencia. Si la fuente es WAV PCM compatible,
puede mostrar la waveform para sincronizar eventos o poses. Conviene guardar antes de cambiar de
documento y validar que los targets de cada track sigan existiendo en la escena.

## Tilemaps Y Scene View

MiniForge tiene dos capas complementarias:

- `TilemapLayers`: grid runtime/editor de capas.
- `miniforge_2d::tilemap_editor2d`: seleccion, patrones, brushes, terrain rules, stamp brushes y object brushes.

Componentes relacionados:

- `Tilemap2D`
- `TilemapRenderer2D`
- `TilemapChunk2D`
- `Tileset2D`
- `TilemapCollider`

La capa `Collision` de tilemap alimenta el `Grid` para colisiones runtime 2D.

El editor de tilemap administra capas y strokes con pencil, line, rectangle, fill, selección y
copy/paste. Los terrain sets asocian reglas y variantes probabilísticas para auto-tiling; undo
revierte el stroke completo, no celda por celda.

Scene View y Game View comparten snapshot del backend, pero tienen roles distintos: Scene View
permite editar y Game View prioriza la salida de juego. Scene View soporta selección simple,
aditiva y por caja; pan/zoom; focus/reset; smart snap; grid/guides; camera frame; HUD y collision
overlay. Los gizmos aplican transformaciones batch con un único paso de undo. Las acciones de
arrange alinean o distribuyen la selección, y Groups & Layers administra agrupación, layer,
visibilidad y bloqueo.

## UI

Hay dos modelos UI:

- Legacy por entidad: componente `UIElement`.
- Scene-level canvas: `ui_canvases` con `UiCanvasRoot`.
- MiniForge2D UI: `UiCanvas2D`, `ScreenManager2D`, widgets y focus navigation.

`UiRuntime` soporta layout, hover, click, focus, comandos, rueda y hit testing para canvas y
`UIElement`. `InventoryGrid` y `AbilityBar` calculan sus filas, recortan el contenido, virtualizan
los slots fuera de vista y desplazan con la rueda sin script. `ScrollBox` usa `content_height`,
`scroll_y` y `scroll_step`; el inspector expone esos campos junto con `show_scrollbar`.

El modelo de `UiCanvasRoot` incluye `Panel`, `Button`, `Label` e `Image`. Cada elemento tiene `UiRect` con anchors, pivots, offsets y tamano.

UI Designer ofrece palette, jerarquía, canvas y propiedades. Se pueden crear widgets, cambiar
padre, mover/redimensionar, editar anchors/pivot/offsets, declarar bindings y callbacks, validar
el documento y guardarlo. Undo/redo opera sobre acciones de sesión. Como aún coexisten el modelo
legacy `UIElement` y los canvas modernos, conviene no mezclar ambos en una misma pantalla salvo
que se esté migrando contenido existente.

## Scripting En El Editor

Los scripts Luau se editan en `scripts/*.luau` y se adjuntan por:

- `GameObject.script`
- `GameObject.scripts`
- `ScriptComponent`
- componentes con paths de script.

Visual graphs viven normalmente en `scripts/visual_graphs/*.mfgraph` y se adjuntan con `VisualScript` o `VisualGraphComponent`.

El editor valida Luau con `LuauScriptRuntime::validate_source_diagnostics`, mostrando linea/columna cuando Luau las reporta, y visual graphs con `VisualGraphSerializer::try_migrate`. El autocompletado entiende miembros con prefijo (`Entity.ne`, `Component.`, `Physics2D.`), ofrece callbacks y snippets compatibles con el runtime, y toma como contrato de tipos `types/miniforge.luau`.

El editor Qt conserva tabs Luau, buffers dirty, documento activo, breakpoints y watches en `.miniforge/qt_workspace.json` mediante escritura atomica. Al reabrir el proyecto recupera los buffers sin sustituirlos silenciosamente por el disco. El debugger lateral permite pause/resume/step callback-level y muestra el frame/watches reales entregados por el runtime.

El flujo recomendado en Luau Studio es:

1. Crear o abrir un `.luau` desde Content Browser.
2. Insertar un callback/snippet y completar contra `types/miniforge.luau`.
3. Validar; abrir el diagnostic para ir a su línea/columna.
4. Guardar y adjuntar el path a la entidad o `ScriptComponent`.
5. Entrar en Play Mode y revisar Console.
6. Si hace falta, crear un breakpoint en la declaración del callback y agregar watches como
   `self.speed`, `entity.name` o `event.payload.quest`.

Los breakpoints son por callback. `step` ejecuta el callback pausado y solicita detenerse antes
del siguiente callback elegible; no recorre cada instrucción Luau. Los watches solo aceptan rutas
de identificadores y no evalúan código arbitrario.

`VisualGraphPanel.qml` ofrece lista de graphs, creacion, palette, canvas con nodos movibles, links `next`, inspector JSON por nodo, validacion local de ids/links y validacion final del schema Rust antes de guardar. El Content Browser puede crear y abrir `VisualGraph` en `scripts/visual_graphs`.

## Play Mode

Play Mode usa los mismos sistemas que runtime, pero dentro de `Game`. El orden de sistemas respeta safe mode:

- scripts y graphs pueden desactivarse.
- plugins nativos pueden desactivarse.
- el mundo se sincroniza y reindexa despues de modificaciones.

`PlayModeManager` y `EditorHistory` guardan snapshots para volver al estado editor.

## Autosave Y Recovery

El editor tiene:

- `AutosaveManager` con intervalo por proyecto.
- `SceneSaveManager` para bootstrap y guardado.
- `SessionRecoveryManager` para sesiones del editor.
- `ProjectStorage` con backups rotativos.

Si existe `saves/autosave/autosave.scene`, el editor lo reporta al abrir el proyecto.

Project Operations permite configurar el autosave, forzarlo, recuperar una copia, crear un
checkpoint de sesión, restaurarlo o limpiarlo. Luau Studio mantiene además recuperación de
buffers sin guardar en `.miniforge/qt_workspace.json`. Si un proyecto causa fallos al abrir,
combina `--safe-mode` con `--reset-layout`; después valida y habilita sistemas de uno en uno.

## Safe Mode

Safe Mode puede bloquear:

- scripts Luau.
- visual graphs.
- plugins nativos.

Se usa para abrir proyectos con fallos, evitar ejecucion insegura y aislar problemas.

## Readiness Y Auditoria

`ProjectValidator` revisa:

- carpetas base.
- `project.json`, `manifest.json`, `engine_config.json`.
- JSON invalido.
- Luau invalido.
- visual graphs incompatibles.
- escenas y prefabs legacy/futuros/corruptos.
- documentos 2D.
- plugins.
- GUIDs duplicados.
- referencias de assets.
- build settings.

`SystemAudit` produce score por area, fortalezas, gaps y acciones. La exportacion runtime incluye readiness score y acciones recomendadas.

## Build, Export Y Ejecución Externa

Build & Export ofrece perfiles Debug, Release y Shipping. Antes de copiar el proyecto, el backend
ejecuta `ProjectValidator`, determina assets usados/faltantes, calcula el plan de backend y
escribe `runtime_manifest.json` y `build_info.json`. Los errores bloquean; los warnings y acciones
de readiness quedan en el reporte.

Project Operations cubre cuatro flujos relacionados:

- export runtime a una carpeta validada;
- paquete de proyecto `.mfpkg.zip` para mover/restaurar fuentes;
- distributable con runtime y plan de instalador cuando están disponibles;
- External Play/Build.

External Play no ejecuta el juego dentro de Qt. Rust prepara un `ExternalLaunchPlanDto` con
ejecutable, artifact, argumentos y warnings; `MfBridge` inicia `miniforge_runtime --build
<artifact>` mediante `QProcess`. Stop termina ese proceso sin cerrar el editor. Esta separación
permite que un crash del juego no derribe el workbench y evita confundir el estado de Play Mode
con un build exportado.

## Automatizacion

El editor incluye:

- `AutomationBridge`: inspeccion de lenguajes, Python tools, plugins, render backend y recomendaciones.
- Python editor automation mediante `.mftool.json`.
- C# plugin scaffolding.
- MCP server en `mcp/miniforge`.
- ForgeAI para planificacion y ejecucion controlada por permisos.

## Flujos Recomendados

Topdown:

1. Crear template `topdown`.
2. Abrir escena `TopDown_Level`.
3. Confirmar player con `CharacterController2D`, `Rigidbody2D` y `CameraFollow`.
4. Agregar `EnemyBrain.luau` o visual graph.
5. Probar movimiento, colisiones, camara y UI.

Platformer:

1. Crear template `platformer`.
2. Configurar `CharacterController2D.mode = platformer`.
3. Activar gravedad en `Rigidbody2D`.
4. Agregar tiles de colision.
5. Probar coyote time, jump buffer, dash y respawn.

RTS:

1. Crear template `rts`.
2. Revisar `RTSController`, `Commandable`, `ProductionQueue`, `ResourceNode`, `Worker`.
3. Probar gather, production, rally point, fog y tactical combat.
4. Ajustar balance en `assets/data/RTSBalance.json`.

Demo completa:

1. Crear template `demo`.
2. Probar `Demo_Menu` y `Demo_Game`.
3. Validar scripts, particulas, audio, UI, save data y escena.

## Troubleshooting

- Si no abre una escena: revisar `engine_config.json.start_scene` y existencia en `saves/scenes`.
- Si un prefab no instancia: ejecutar `ProjectValidator`; revisar `scripts.required` y `settings.required`.
- Si un script no corre: revisar Safe Mode, errores de sintaxis Luau y que la entidad este activa.
- Si el editor no refleja assets: ejecutar `cargo dev -- assets <proyecto>`.
- Si Content Browser rechaza una ruta: mover el archivo a una raíz administrada y evitar symlinks
  que salgan del proyecto.
- Si un buffer Luau reaparece tras un cierre: revisar `.miniforge/qt_workspace.json` antes de
  descartarlo.
- Si el layout queda inutilizable: usar `View > Reset Current Workspace` o arrancar con
  `--reset-layout`.
- Si el proyecto falla al abrir: arrancar con `--safe-mode`, ejecutar auditoría y reactivar
  scripts/graphs/plugins de forma gradual.
- Si la camara no sigue: verificar `CameraFollow.target_id` y viewport settings.
- Si hay colisiones raras: revisar `Collider2D`, `Rigidbody2D`, `collision_layer`, `collision_mask`, one-way y tilemap `Collision`.
- Si el export falla: corregir errores de `ProjectValidator` antes de empaquetar.

Consulta también el [índice de documentación](README.md) y la guía de
[desarrollo/build](DESARROLLO_BUILD_Y_EXTENSION.md).
