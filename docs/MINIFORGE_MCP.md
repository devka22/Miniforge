# MiniForge MCP

Este MCP permite que Codex use MiniForge como motor local: puede leer la version actual del motor, crear un juego demo, validar el runtime en modo headless y apuntar problemas o recomendaciones en `docs/MINIFORGE_MCP_FEEDBACK.md`.

## Herramientas

- `engine_status`: muestra version del crate, version interna del motor, stream y comandos recomendados.
- `list_projects`: lista proyectos MiniForge dentro de `projects/`.
- `create_game`: crea un proyecto jugable. Templates actuales: `astro_harvester`, `grand_strategy_rts` y `love_story_lab`.
- `validate_game`: ejecuta el motor una vez en modo runtime/headless para validar el proyecto.
- `record_feedback`: agrega notas al archivo de feedback para pasarlas luego a Codex.

## Configuracion Codex

La configuracion local apunta a:

```text
mcp/miniforge/src/index.js
```

con `MINIFORGE_ENGINE_ROOT` apuntando a la raiz de este repo.

## Uso Manual

```bash
cd mcp/miniforge
npm run self-test
npm run create-demo
npm run create-love-story-lab
node src/index.js --create-grand-strategy MCP_IronTreaty_1836
node src/index.js --create-love-story-lab MCP_LoveStoryLab
node src/index.js --validate-game ../../projects/MCP_IronTreaty_1836
```

Abrir el juego demo:

```bash
cargo run --bin miniforge_editor -- --project projects/MCP_AstroHarvester --no-launcher
```

Validarlo sin ventana:

```bash
cargo run --bin miniforge -- --project projects/MCP_AstroHarvester --runtime --no-launcher --headless-once
```

## CLI Nativa Del Motor

El motor tambien puede crear proyectos base sin pasar por el MCP:

```bash
cargo run --bin miniforge -- --create-project projects/NewRTS --template rts --force
```

## Template Grand Strategy RTS

`grand_strategy_rts` crea `projects/MCP_IronTreaty_1836`, un sandbox 2D inspirado en gran estrategia historica con provincias, poblacion, mercado, fabricas, diplomacia, research, ejercitos y rutas comerciales. Usa la escena inicial `campaign_1836.scene`, lo que valida que el motor respete `engine_config.start_scene`.

## Template Love Story Lab

`love_story_lab` crea `projects/MCP_LoveStoryLab`, un laboratorio narrativo 2D llamado `Letters Under Rain`. El proyecto esta pensado para probar juegos de historia dentro del motor: movimiento top-down, `Dialogue`, `QuestLog`, `Interaction`, `VisualScript`, `UIElement`, `AudioSource`, `ParticleEmitter`, `Trigger2D`, `Checkpoint`, `Saveable`, `Sequencer2D` y `tilemap_layers`.

Las plantillas del MCP escriben `settings/runtime_config.json` con `performance_class: "auto"`, `worker_threads: "auto"`, `parallel_asset_scan: true` y `prefer_metal_on_macos: true` para que el backend pueda adaptar workers al perfil de hardware detectado.

Abrir el laboratorio:

```bash
cargo run --bin miniforge_editor -- --project projects/MCP_LoveStoryLab --no-launcher
```

Validarlo sin ventana:

```bash
cargo run --bin miniforge -- --project projects/MCP_LoveStoryLab --runtime --no-launcher --headless-once
```
