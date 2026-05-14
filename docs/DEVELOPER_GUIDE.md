# MiniForge Developer Guide

MiniForge 0.8.0 esta orientado a desarrollar juegos 2D y RTS completos desde el editor Rust.

## Flujo Base

1. Crear o abrir proyecto desde launcher/editor.
2. Crear escena en `saves/scenes/`.
3. Crear entidades desde toolbar, inspector, prefabs o templates.
4. Crear scripts `.rhai` o graphs `.mfgraph` desde Content Browser/Programming.
5. Abrir el asset en el panel `Programming`, editarlo y guardar.
6. Entrar a Play Mode, revisar consola/profiler y volver a editar.
7. Exportar runtime en debug/release.

## Estructura De Proyecto

```text
Project/
├─ assets/
│  ├─ sprites/
│  ├─ audio/
│  ├─ data/
│  └─ prefabs/
├─ scripts/
│  └─ visual_graphs/
├─ saves/scenes/
├─ settings/
├─ logs/
├─ builds/
├─ project.json
└─ engine_config.json
```

MiniForge recrea carpetas base al abrir el proyecto. `engine_config.json` se migra, respalda y recupera automaticamente si se corrompe.

## Scripting Rhai

Asigna scripts con `entity.script = "PlayerController.rhai"` o en `scripts`:

```json
{"runtime": "rhai", "path": "scripts/PlayerController.rhai"}
```

Eventos disponibles: `on_start()`, `on_update(dt)`, `on_key_down(key)`, `on_collision_enter(other)`, `on_destroy()`.

API disponible: `move`, `set_position`, `spawn`, `destroy`, `play_sound`, `load_scene`, `input_pressed`, `ui_text`, `set_ui_text`.

Los errores de compilacion/runtime se muestran en consola y en `logs/miniforge.log`; no deben cerrar el editor.

## Visual Graphs

Los `.mfgraph` viven en `scripts/visual_graphs/`. Deben incluir `runtime = "rust_visual_graph"` y un array `nodes`. Nodos desconocidos o referencias `next` rotas se reportan como errores recuperables.

Desde el panel `Programming`, un `.mfgraph` se abre como canvas de nodos. Cada nodo tiene pin de entrada y salida; arrastra desde el pin de salida de un nodo al pin de entrada de otro para actualizar la conexion `next`. Tambien puedes mover nodos y guardar el layout dentro del propio asset.

Los scripts `.rhai` se abren en el mismo panel como editor de codigo. `Ctrl+S` guarda y notifica al hot reload.

## Escenas Y Prefabs

Las escenas se guardan en `saves/scenes/*.scene` con backups `.scene.bak`. Los prefabs se guardan en `assets/prefabs/*.prefab` con backups `.prefab.bak`.

Validar proyecto revisa JSON, escenas, prefabs, referencias rotas, scripts Rhai, graphs y build settings.

## Assets, Input, Audio, Physics Y UI

- Assets: usar Content Browser con Sources, busqueda, filtros por tipo, grid visual, preview, labels, import settings y dependency graph.
- Input: editar `settings/input_map.json` desde el panel Programming/Input Map.
- Audio: usar `AudioSource`, audio events, mixer Master/Music/SFX y preview cuando exista asset.
- Physics: `Rigidbody2D`, `Collider2D`, triggers, layers, raycasts, friccion y rebote.
- UI: canvas/panel/button/label con anchors, layout responsive y eventos hover/click.

## Export

`RuntimeExporter` crea `build/<profile>/<project>/` con `runtime_manifest.json` y `build_info.json`. Release activa metadata de optimizacion y validacion de assets faltantes.

## Troubleshooting

- Config corrupta: revisar `engine_config.json.corrupt` y `engine_config.json.bak`.
- Escena corrupta: MiniForge intenta `.scene.bak`; validar proyecto muestra el archivo afectado.
- Prefab corrupto: se intenta `.prefab.bak`.
- Script Rhai falla: abrir el archivo en Programming, usar `Check`, revisar consola/logs.
- Asset faltante en export: revisar warnings de Runtime Export y dependency graph.
- Play Mode cambia la escena: al salir restaura snapshot del editor.
