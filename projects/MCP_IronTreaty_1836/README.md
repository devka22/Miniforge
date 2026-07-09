# Iron Treaty 1836

Grand strategy RTS sandbox generado por el MCP de MiniForge usando MiniForge 0.9.1.1 / stream 0.9.2.

No copia Victoria 2; usa una idea parecida: provincias, pops, mercado, industrias, diplomacia, investigacion y ejercitos en un mapa 2D editable.

## Ejecutar editor

```bash
cargo run --bin miniforge_editor -- --project projects/MCP_IronTreaty_1836 --no-launcher
```

## Validar sin ventana

```bash
cargo run --bin miniforge -- --project projects/MCP_IronTreaty_1836 --runtime --no-launcher --headless-once
```

## Que mirar en el Inspector

- Provincias: `Province2D`, `PopulationPops2D`, `Market2D`, `Factory2D`.
- Naciones: `Nation2D`, `Diplomacy2D`, `ResearchTree2D`.
- Ejercitos: `ArmyStack2D`, `ThreatSource`, `Commandable`.
- Ruta comercial: `TradeRoute2D`.

## Archivos de diseno

- `assets/data/WorldMap.json`
- `assets/data/Nations.json`
- `assets/data/GoodsMarket.json`
- `assets/data/TechTree.json`
- `assets/data/Decisions.json`
