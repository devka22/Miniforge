# MiniForge Rust Engine Guide

MiniForge es un motor/editor 2D en Rust orientado a prototipos jugables, juegos top-down, plataformas 2D y RTS. El editor abre un proyecto, muestra jerarquia, inspector, escena, navegador de contenido, consola, prefabs, profiler y un modo Play con snapshot seguro.

## Ejecutar

```bash
cargo run --bin miniforge_editor -- --project projects/DefaultProject --no-launcher
cargo run --bin miniforge -- --project projects/DefaultProject --no-launcher
cargo run --bin miniforge_runtime -- --build projects/DefaultProject/build/debug/DefaultProject
cargo run --bin miniforge -- --headless-once
python3 main.py
```

- `miniforge_editor` / `miniforge`: editor completo.
- `miniforge_runtime`: solo lectura de build exportado (sin paneles de editor).

## Flujo Basico

1. Usa `Top2D`, `Plat2D` o `RTS Demo` para crear una escena base.
2. Selecciona entidades en la jerarquia o escena.
3. Usa el inspector para editar transform, stats, AI, RTS, dialogue, quest, tweens y componentes avanzados.
4. Guarda con `Save` o `File > Save Project`.
5. Entra a `Play` para probar sin destruir la escena: al salir se restaura el snapshot.

## Sistemas Principales

- `GameObject`: entidad serializable con transform, tag, layer, componentes, comandos y path.
- `Component`: datos extensibles con helpers para vida, stats, inventario, economia, cooldown, nav, tween, estado y combate.
- `GameAPI`: API de gameplay para crear entidades, mover en X/Y, escalar, rotar, mirar hacia un punto, agregar audio, spawnear sprites, recursos, squads, cooldowns, blackboard y guardado de estado.
- `ArchetypeLibrary`: biblioteca de entidades listas como `rts_worker`, `rts_soldier`, `rts_command_center`, `topdown_hero` y `platformer_player`.
- `AssetDatabase`: escanea sprites, sonidos, prefabs, escenas, materiales, graphs y datos con metadatos/import settings.
- `AssetPreview`: resume GUID, path, labels, settings, dependencias, reverse dependencies y warnings para el panel de preview.
- `FileBrowser`: backend para explorar, crear carpetas, renombrar, mover, duplicar, importar y crear sprite imports/sound cues/materiales.
- `EditorCommand`: snapshots y Command Pattern para undo/redo de operaciones del editor.
- `TileBrush`: pencil, eraser, fill, rectangle y collision brush sobre `TilemapLayers`.
- `RuntimeExporter`: empaqueta proyecto en `build/debug` o `build/release` con manifest runtime.
- `RTSSystem`: economia, produccion, construccion, fog of war, combate tactico, auto-queue por recetas y destruccion.
- `GameplaySystem`: AI, spawners, timers, tweens, estado, status effects, interacciones y NavAgent.

## Editor 0.7

- `1..5`: Select, Move, Rotate, Scale y Paint.
- `G`: alterna snap to grid.
- `B` en Paint cambia brush.
- `L` cambia capa de tilemap.
- `Cmd/Ctrl+Z`: undo. `Cmd/Ctrl+Y` o `Shift+Cmd/Ctrl+Z`: redo.
- El Inspector confirma campos de texto con Enter y cancela con Escape.
- El Content Browser permite arrastrar sprites, prefabs, materiales, sonidos y visual graphs hacia Scene.
- `Asset Preview` permite reconstruir dependencias y alternar `include_in_build`.
- `Build D` y `Build R` exportan runtime debug/release.

## Assets

Carpetas reconocidas:

- `assets/sprites`: imagenes y `.sprite.json`.
- `assets/audio`: sonidos y `.sound.json`.
- `assets/data`: JSON, CSV, materiales y datos.
- `assets/prefabs`: prefabs serializados.
- `scripts/visual_graphs`: graphs visuales `.mfgraph`.
- `saves/scenes`: escenas del proyecto.

Los import settings se guardan en `project/asset_metadata.json` y no deben versionarse como fuente canonica.

## Export Runtime

El exporter escribe:

```text
build/
├─ debug/<ProjectName>/
│  ├─ runtime_manifest.json
│  └─ build_info.json
└─ release/<ProjectName>/
```

`runtime_manifest.json` incluye engine version, perfil, assets usados, assets faltantes y manifest fuente. El exporter omite `target/`, `build/`, `builds/`, `exports/` y caches Python para evitar builds recursivos.

## Play Mode

Play Mode crea un snapshot de entidades antes de entrar a juego. Durante Play se ejecutan sistemas runtime y al detener vuelve al estado anterior. Esto permite probar combate, produccion, IA y movimiento sin ensuciar la escena.

## RTS

El flujo RTS usa:

- `Team`, `Commandable`, `SquadMember`, `RtsBrain`.
- `ProductionQueue` + `ProductionRecipeBook`.
- `Worker`, `ResourceNode`, `EconomyWallet`.
- `Vision`, `FogOfWar`, `ThreatSource`, `InfluenceSource`.
- `CombatTarget`, `DamageDealer`, `NavAgent`.

Las rutas usan A*, flow fields, line-of-sight, influence maps y rutas threat-aware.
