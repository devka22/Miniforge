# MiniForge 0.9.3.4 — 2D Workflow Foundations

Estado: **release final**. La versión pública, paquetes, manifests y proyectos nuevos se identifican como `0.9.3.4`. `DEVELOPMENT_VERSION` queda como alias de compatibilidad, `Engine0934FoundationPlan::current()` marca `Released` y `launch_allowed = true`.

## Referencias estudiadas

- Godot: repositorio principal, flujo de importación, recursos, scripting entre lenguajes y GDExtension.
- Unreal Engine: Paper 2D, Sprite Editor, colisiones, sockets, flipbooks, Tile Sets/Tile Maps y edición múltiple desde Content Browser/Property Matrix.

Las ideas se reimplementan como APIs Rust propias; no se copia código fuente de ninguno de los motores.

## Bases incluidas

### Pipeline de assets 2D

`engine::asset_pipeline_2d` separa el archivo fuente del artista y el asset generado para runtime:

- huella estable del contenido para detectar cambios;
- perfiles `Pixel Art 2D`, `Smooth Sprite 2D`, `UI Texture` y `Game Audio`;
- opciones editables por lote;
- dependencias y cálculo de impacto;
- plan de reimportación priorizado;
- archivos generados borrables y reconstruibles;
- manifiesto JSON y ruta estándar de sidecar `.mfimport.json`.

Esto permite conectar después un Import Dock, reimportación automática y caché bajo `.miniforge/imported` sin convertir archivos generados en la fuente de verdad.

### Workflow del editor

`engine::editor_workflow_2d` define una capa de acciones independiente de egui:

- acciones sensibles al contexto (Scene, Sprite, Tilemap, Animation, UI, Script y Play Mode);
- búsqueda aproximada para Command Palette y Quick Open;
- herramientas Select/Move/Rotate/Scale/Pivot/Measure/Paint/Erase;
- selección múltiple;
- transacciones de edición por lote con undo/redo;
- acciones base para pixel snap, colisiones, extracción de sprites, sockets, patrones de tiles y preview de animación.

La interfaz puede renderizar estas acciones como menú, toolbar, shortcut o palette sin duplicar la lógica.

### Rust + otros lenguajes

`engine::script_host_2d` mantiene Rust como dueño del motor y usa un contrato único de llamadas con valores serializables:

| Backend | Estado 0.9.3.4 | Uso |
|---|---|---|
| Luau | integrado | gameplay y hot reload |
| Visual Graph | integrado | gameplay visual |
| Rust nativo | disponible | sistemas de alto rendimiento compilados con el proyecto |
| WebAssembly | contrato preparado | extensiones portables y aisladas |
| Lua | contrato preparado | scripts pequeños embebidos |
| Python | disponible, solo editor | automatización e importación de assets mediante proceso aislado y protocolo JSON |

Lua y WebAssembly no se marcan como ejecutables todavía. Cada módulo declara versión de API, funciones y capacidades (scene, physics, input, UI, filesystem, network), evitando introducir ejecución nativa insegura antes de tener sandbox, límites y diagnóstico. Python se limita a scripts confiables dentro de `project/tools`, nunca se incluye en gameplay exportado y entrega operaciones validadas al editor mediante `miniforge-editor-tool-v1`.

### Interfaz espacial y geometría Lyon

El editor visible continúa usando Macroquad; egui permanece disponible para docking/modelos auxiliares y wgpu sigue como backend experimental. `engine::vector_canvas_2d` integra Lyon como teselador independiente del backend, produciendo buffers de vértices/índices que hoy consume Macroquad y que podrán subirse a wgpu después.

- paths con líneas, Bézier cuadráticas y cúbicas;
- rellenos y trazos con joins/caps suaves;
- polígonos, círculos y rectángulos redondeados;
- hit testing por triángulos teselados;
- cables Bézier en Visual Graph;
- contornos de selección, flechas y anillos de gizmo;
- paths de colisión dibujados sobre el viewport.

### Herramientas 2D conectadas

- smart snapping contra centros y bordes, además de grid/pixel snap;
- guías amarillas visibles durante el arrastre;
- selección múltiple con Shift/Cmd y movimiento conjunto;
- grupos persistentes en escena y selección del grupo completo;
- alineación y distribución horizontal/vertical desde Command Palette;
- capas con visibilidad, bloqueo y cambio de capa por lote;
- bloqueo respetado por hit testing y gizmos;
- edición visual de pivote con herramienta dedicada;
- edición visual de vértices de `Collider2D`, Alt+click para agregar;
- Pencil, Eraser, Fill, Rectangle, Line y Collision para tiles;
- marco de cámara 16:9 con safe area;
- zoom interpolado y navegación con shortcuts;
- indicadores de selección, zoom, capa y snap en el HUD del viewport.

Atajos principales: `1..7` cambia herramienta, `Shift/Cmd+click` multiselecciona, `F` encuadra la selección, `Space+drag` o botón medio desplaza, `Cmd+wheel` hace zoom suave, `G` alterna grid snap, `B` recorre brushes y `L` recorre capas de tilemap. `Cmd+Shift+G` agrupa y `Cmd+Shift+U` desagrupa.

Reglas de autoría:

1. Un objeto o capa bloqueada no participa en hit testing, drag, alineación ni snapping.
2. Un objeto oculto no se puede seleccionar; ocultar una capa limpia su selección activa.
3. Smart snap aplica primero grid opcional y después el ajuste más cercano de borde/centro dentro de la tolerancia visual.
4. La primera entidad seleccionada es la referencia de alineación; distribución conserva los extremos.
5. Cambios de grupo, capa, pivote y colisión generan una transacción undo/redo.
6. Los colliders poligonales conservan al menos tres vértices.
7. Herramientas Python deben estar explícitamente marcadas como confiables, vivir bajo `tools/`, respetar timeout y devolver solo operaciones permitidas.

## Próximas conexiones

1. Renderizar Import Dock y estado de reimportación dentro del Content Browser.
2. Subir los mismos buffers Lyon al backend wgpu cuando deje de ser experimental.
3. Ampliar el Sprite Editor con edición por lote de polígonos, sockets y render geometry.
4. Elegir y prototipar primero WebAssembly o Lua; Python queda restringido a herramientas.
5. Añadir caché atómica, workers de importación y preview incremental.

## Criterio para lanzar

No cambiar `FoundationReleaseState` a `Released` ni `launch_allowed` a `true` hasta completar integración visual, migración de proyecto, pruebas de recuperación de importación y una prueba de exportación limpia. Esta base no modifica el paquete público ni los manifests de release.
