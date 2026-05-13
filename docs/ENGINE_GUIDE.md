# MiniForge Rust Engine Guide

MiniForge es un motor/editor 2D en Rust orientado a prototipos jugables, juegos top-down, plataformas 2D y RTS. El editor abre un proyecto, muestra jerarquia, inspector, escena, navegador de contenido, consola, prefabs, profiler y un modo Play con snapshot seguro.

## Ejecutar

```bash
cargo run --bin miniforge_editor -- --project projects/DefaultProject --no-launcher
cargo run --bin miniforge -- --project projects/DefaultProject --no-launcher
cargo run --bin miniforge_runtime -- --build projects/DefaultProject/build/debug/DefaultProject
cargo run --bin miniforge -- --headless-once
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
- `GameAPI`: API de gameplay para crear entidades, mover, setear posicion, spawn/destroy, cargar escenas, UI text, agregar audio, spawnear sprites, recursos, squads, cooldowns, blackboard y guardado de estado.
- `ArchetypeLibrary`: biblioteca de entidades listas como `rts_worker`, `rts_soldier`, `rts_command_center`, `topdown_hero` y `platformer_player`.
- `AssetDatabase`: escanea sprites, sonidos, prefabs, escenas, materiales, graphs y datos con metadatos/import settings.
- `AssetPreview`: resume GUID, path, labels, settings, dependencias, reverse dependencies y warnings para el panel de preview.
- `FileBrowser`: backend para explorar, crear carpetas, renombrar, mover, duplicar, importar y crear scripts Rhai, prefabs, enemigos, UI, audio events, sprite imports/sound cues/materiales.
- `EditorCommand`: snapshots y Command Pattern para undo/redo de operaciones del editor.
- `TileBrush`: pencil, eraser, fill, rectangle y collision brush sobre `TilemapLayers`.
- `RuntimeExporter`: empaqueta proyecto en `build/debug` o `build/release` con manifest runtime.
- `AnimationEditor`: timeline, clips, keyframes, preview y transiciones de Animator.
- `ParticleSystem`: emitters con burst/loop, velocity, lifetime, size y preview en editor.
- `MaterialLibrary`: materiales 2D editables, shaders builtin y soporte lighting/fog.
- `UiRuntime`: layout responsive, hover/click y comandos para botones.
- `ScriptDebugger`: errores Rhai, scripts activos, trazas de funciones y reload.
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
- `assets/audio`: sonidos, `.sound.json` y `.audio.json`.
- `assets/data`: JSON, CSV, materiales y datos.
- `assets/prefabs`: prefabs serializados.
- `scripts`: scripts de entidad `.rhai` con hot reload.
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

`runtime_manifest.json` incluye engine version, perfil, assets usados, assets faltantes y manifest fuente. El exporter omite `target/`, `build/`, `builds/`, `exports/` y caches locales para evitar builds recursivos.

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

## Mapa de capacidades (motor)

| Area | Modulos / entrada principal |
|------|-------------------------------|
| Runtime/editor Rust | `src/editor_app.rs`, `src/core/game.rs`, binarios `miniforge`, `miniforge_editor`, `miniforge_runtime` |
| Play mode + escena viva | `PlayModeManager`, `Game::enter_play_mode` / `exit_play_mode`, F5/F11, barra de estado `PLAY#frames` |
| RTS (pathfinding, squads, fog, influence) | `RTSSystem`, `map::pathfinding`, `flow_field`, componentes RTS en `engine::component` |
| Arquitectura por componentes | `GameObject`, `Component`, `ComponentRegistry`, `InspectorEditor` |
| Prefabs + overrides | `prefab_manager`, `prefab_overrides`, `advanced_prefabs` |
| Scripting Rhai | `rhai_scripting`, scripts `.rhai`, eventos `on_start/on_update/on_key_down/on_collision_enter/on_destroy` |
| Debug scripts | `script_debugger`, errores runtime, trazas por linea, reload y scripts activos |
| Animation Editor | `animation_editor`, `animation_graph`, clips, keyframes, timeline, preview y transitions |
| Particulas | `ParticleSystem`, `ParticleEmitter`, burst/loop, velocity/lifetime/size y preview |
| Visual scripting | `visual_scripting`, assets `.mfgraph`, contadores en Profiler |
| Fisica 2D | `PhysicsSystem`, rect/circle/polygon colliders, triggers, layers, raycasts, rigidbodies, gravedad, friccion y rebote |
| Escenas | `SceneManager`, load/unload/restart, aditivas, transiciones, DontDestroyOnLoad y scene stack |
| Audio | `AudioSystem`, `AudioMixer`, comandos Kira para music/sfx/volume/fade |
| Materiales/shaders | `MaterialLibrary`, `.material.json`, `.shader.json`, lighting/fog/fallbacks |
| UI Runtime | `UiRuntime`, `UiCanvasRoot`, anchors responsive, hover/click events |
| Asset browser + GUID | `AssetDatabase`, `AssetPreview`, `FileBrowser`, panel Assets del editor |
| Profiler y debug | `Profiler`, `Diagnostics`, `DeveloperConsole`, pestaña Profiler |
| Jerarquia + inspector | `draw_hierarchy`, `draw_inspector`, `SceneViewTools` |
| Render Macroquad | `macroquad` en `editor_app` / `main`, `render::renderer`, `draw_scene` |
| IA / navegacion | `AIController`, `NavAgent`, `MovementSystem`, `GameplaySystem` |
| Tilemap + herramientas | `TilemapLayers`, `TileBrush`, herramienta Paint, capas |
