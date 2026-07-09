# Astro Harvester

Juego demo generado por el MCP de MiniForge usando MiniForge 0.9.1.1 / stream 0.9.2.

## Ejecutar editor

```bash
cargo run --bin miniforge_editor -- --project projects/MCP_AstroHarvester --no-launcher
```

## Validar sin ventana

```bash
cargo run --bin miniforge -- --project projects/MCP_AstroHarvester --runtime --no-launcher --headless-once
```

## Controles

- WASD: mover piloto.
- Space: pulso defensivo.
- E: interactuar cuando el flujo del motor lo conecte.

## Que prueba

- Escena jugable con HUD, scripts Rhai y componentes avanzados.
- Loop hibrido top-down/RTS con recursos, base, drones enemigos y produccion.
- Datos de balance en `assets/data/AstroBalance.json`.
