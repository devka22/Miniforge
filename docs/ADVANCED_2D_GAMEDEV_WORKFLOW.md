# Advanced 2D Game Development Workflow

MiniForge ahora tiene mas apoyo para juegos 2D grandes y complejos:

- Editor de scripts con outline de funciones, diagnosticos, busqueda, snippets `on_start` / `on_update`, duplicado de linea, comentario rapido, indentacion y formateo JSON para `.mfgraph`, `.scene` y prefabs.
- Panel lateral de programacion con mapa del script, estadisticas del archivo, diagnosticos y salto rapido a funciones o nodos.
- Templates visuales nuevos:
  - `Massive2DSpawnDirector`: oleadas con presupuesto de entidades para mapas grandes.
  - `GrandStrategyMonthlyTick`: tick mensual para economia, research y sistemas tipo grand strategy.
- Componentes para juegos 2D masivos y simulaciones:
  - `WorldPartition2D`, `StreamingChunk2D`, `RuntimeBudget2D`, `ObjectPool2D`, `SpawnDirector2D`, `SaveShard2D`.
  - `Province2D`, `Nation2D`, `PopulationPops2D`, `Market2D`, `Factory2D`, `Diplomacy2D`, `ResearchTree2D`, `ArmyStack2D`, `WarGoal2D`, `TradeRoute2D`.
- Validador de proyecto mas estricto para `engine_config.start_scene`, batching 2D y escenas con miles de entidades.

## Flujo Recomendado

1. Define `RuntimeBudget2D` en un objeto global de escena.
2. Usa `WorldPartition2D` y `StreamingChunk2D` si la escena supera miles de entidades.
3. Usa `ObjectPool2D` para proyectiles, pickups, enemigos repetidos y efectos.
4. Usa `SpawnDirector2D` o el template `Massive2DSpawnDirector` para controlar oleadas sin saturar el runtime.
5. Separa scripts Luau largos por sistema: player, combat, UI, economy, AI y save.
6. Pasa loops visuales reutilizables a `.mfgraph` para que otros devs los ajusten sin tocar Rust.
7. Ejecuta `Validate project` antes de exportar.

## Atajos Del Editor De Scripts

- `Cmd/Ctrl+S`: guardar.
- `Cmd/Ctrl+D`: duplicar linea.
- `Cmd/Ctrl+/`: comentar o descomentar linea.
- `Cmd/Ctrl+Backspace`: borrar linea.
- `Tab`: indentar.
- `Shift+Tab`: quitar indentacion.

