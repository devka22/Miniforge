# Neon Sombra

Juego 2D cenital de mundo abierto creado con MiniForge 0.9.3.4.

## Estado

- Ciudad amplia: Santa Aurelia, 128x80 tiles, 217 entidades, playa, marina, Pier 21, yates y costa neon.
- Historia: modo historia por actos con Mara, Ivo, Valeria, Luz y Rojas.
- IA urbana: peatones con rutas/panico, trafico con rutas y bocina, policias con patrulla, busqueda, persecucion y arresto.
- Vehiculos: autos civiles y patrullas conducibles con faros dinamicos por noche, lluvia y niebla.
- Clima/dia-noche: CityDirector publica fase, clima, lluvia, niebla, trafico y faros para cualquier script.
- Guardado: puntos seguros reutilizables con `Game.save_slot("autosave")`.
- Iluminacion: `Light2D` visible en runtime con abanicos raycast contra `ShadowCaster2D`.
- UI/menu: HUD compacto con minimapa, estrellas de busqueda, clima, dinero, vida/chaleco, radio, dialogo y menu noir de pausa.
- Arte: backdrop pixel-art 2.5D de ciudad/playa y atlas para personajes, vehiculos, yates, props y bloqueos.
- Optimizacion: perfiles graficos con F1, culling por camara, scheduler Luau con `ScriptSchedule`, presupuestos de entidades/minimapa, prioridad de marcadores cercanos, recorte de casters por luz y limite de luces con sombra.

## Controles

- WASD/flechas: mover o conducir.
- Shift: correr o acelerar vehiculo.
- Mouse izquierdo/Ctrl: disparar o bocina/sirena en vehiculo.
- E/Enter: hablar o recoger.
- F: entrar/salir de vehiculo cercano.
- F1: cambiar perfil grafico runtime (`low`, `medium`, `high`, `ultra`).
- Escape: alternar menu del juego.
- F10: cerrar la ventana runtime.

## Archivos clave

- `saves/scenes/main.scene`: ciudad principal.
- `scripts/PlayerController.luau`: jugador, crimen, vehiculos y HUD.
- `scripts/TrafficBrain.luau`: trafico y conduccion.
- `scripts/PoliceBrain.luau`: patrulla/persecucion/arresto.
- `scripts/CityDirector.luau`: historia, busqueda, reloj, clima, dia/noche, radio e iluminacion global.
- `scripts/SavePointBrain.luau`: puntos seguros y autosave.
- `tools/generate_neon_sombra_world.py`: generador reproducible de ciudad/datos.
- `tools/generate_neon_sombra_art.py`: generador reproducible de pixel art.
- `assets/sprites/neon_sombra_city_backdrop.png`: arte visual principal del mapa.
- `assets/sprites/neon_sombra_atlas.png`: atlas de sprites runtime.
- `assets/data/GraphicsProfiles.json`: perfiles de rendimiento.
- `assets/data/WeatherProfiles.json`: clima/dia-noche consumido por scripts.
- `settings/runtime_config.json`: `graphics` y `script_scheduler` regulan recursos por equipo.

## Probar

Desde la raiz del motor:

```bash
cargo run --bin miniforge_headless --features runtime -- projects/NewProject_5 10
cargo test --test neon_sombra_world_sim
cargo test --test neon_sombra_performance_budget
cargo run --bin miniforge_dev -- export projects/NewProject_5 projects/NewProject_5/builds debug
cargo run --bin miniforge_runtime --features runtime -- projects/NewProject_5/builds/debug/NewProject_5
```

Para forzar rendimiento bajo desde terminal:

```bash
MINIFORGE_GRAPHICS_QUALITY=low cargo run --bin miniforge_runtime --features runtime -- projects/NewProject_5/builds/debug/NewProject_5
```
