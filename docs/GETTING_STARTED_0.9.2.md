# MiniForge 0.9.2 Getting Started

Esta guia resume el flujo recomendado para aprender MiniForge y crear juegos 2D o RTS usando el editor, scripts Rhai y blueprints `.mfgraph`.

## 1. Abrir El Motor

```bash
cargo run --bin miniforge_editor -- --project projects/DefaultProject --no-launcher
```

Si quieres validar sin ventana:

```bash
cargo run -- --headless-once --no-launcher
```

El launcher permite crear proyectos, elegir ubicacion libre, abrir recientes y revisar notas del parche antes de entrar al editor.

## 2. Conocer El Editor

- `Hierarchy`: entidades de la escena. Click derecho permite seleccionar, mover, parentar, limpiar parent y eliminar.
- `Inspector`: transform y componentes del seleccionado.
- `Scene`: edicion visual con herramientas `Select`, `Move`, `Rotate`, `Scale` y `Paint`.
- `Browser`: assets, scripts, prefabs, escenas, materiales y graphs.
- `Programming`: scripts Rhai, blueprints, templates, validador y tabs.
- `Prefabs`, `Scenes`, `Sprites`, `Console` y `Profiler`: flujo de contenido, escena, sprites, errores y rendimiento.
- `Ctrl+P`: paleta de comandos con busqueda difusa. Prueba `inventory`, `quest`, `rts`, `sprite`, `package`.

## 3. Crear Una Escena Jugable

1. Usa `Top2D`, `Plat2D` o `RTS Demo` desde la barra superior.
2. Selecciona el jugador o base en `Hierarchy`.
3. Usa `Ctrl+P` y ejecuta `Attach Graph InventoryEconomyLoop`, `Attach Graph QuestAbilityLoop` o `Attach Graph RTSProductionEconomy`.
4. Pulsa `Play` para probar sin ensuciar la escena; al detener, el editor restaura el snapshot.

## 4. Blueprints 0.9.2

Los blueprints viven en `scripts/visual_graphs/` y se editan como nodos conectables. Puedes abrirlos en ventana flotante desde `Programming` o `Blueprint Picker`.

Templates principales:

- `InventoryEconomyLoop`: agrega oro, items, comprueba recursos, gasta, equipa y consume items.
- `QuestAbilityLoop`: crea quest, actualiza objetivo, dispara habilidad y recarga cargas.
- `RTSProductionEconomy`: configura recursos, recetas y cola de produccion para una base RTS.

Nodos utiles:

- Inventario: `InventoryAdd`, `InventoryRemove`, `BranchItem`, `EquipItem`.
- Economia: `EconomyAdd`, `EconomySpend`, `BranchResource`.
- RTS: `AddProductionRecipe`, `SetPreferredRecipe`, `QueuePreferredRecipe`.
- Quests/habilidades: `AddQuest`, `QuestProgress`, `TriggerAbility`, `RechargeAbility`.
- Flujo: `Sequence`, `Gate`, `DoOnce`, `FlipFlop`, `BranchVariable`, `BranchHealth`.

## 5. Scripts Rhai

Los scripts `.rhai` se guardan en `scripts/`. Eventos disponibles:

```rhai
fn on_start() {
    set_position(3.0, 4.0);
}

fn on_update(dt) {
    move(2.0 * dt, 0.0);
}
```

Usa Rhai para comportamiento especifico y blueprints para flujos visuales de gameplay.

## 6. Inventarios, Economia Y RTS

La API interna `GameAPI` permite construir sistemas complejos:

```rust
GameAPI::add_item(player, "potion", 3);
GameAPI::equip_item(player, "weapon", "iron_sword", serde_json::json!({"attack": 4.0}));
GameAPI::add_resources(base, &serde_json::json!({"Gold": 180.0, "Wood": 60.0}));
GameAPI::spend_cost(base, &serde_json::json!({"Gold": 50.0}));
GameAPI::add_production_recipe(base, "Worker", "Worker", 3.0, serde_json::json!({"Gold": 50.0}));
GameAPI::enqueue_preferred_recipe(base);
```

Para RTS, combina `EconomyWallet`, `ProductionRecipeBook`, `ProductionQueue`, `Worker`, `ResourceNode`, `Commandable`, `Team`, `Vision` y `NavAgent`.

## 7. Sprites, Prefabs Y Escenas

- `Sprites`: crea canvas, pinta pixeles, usa paleta, flip, rotacion y guarda PNG.
- `Prefabs`: guarda seleccion, crea variants, instancia, aplica cambios, revierte o despega una instancia.
- `Scenes`: carga escenas, additive load, push stack y restart.
- `File > Export Project Zip`: genera paquete `.mfpkg.zip`.
- `File > Import Project Zip`: importa un paquete desde `builds/import.mfpkg.zip`.

## 8. Debug

- La consola separa info, warning y error por canal.
- Los errores Rhai y VisualScript no cierran el editor; quedan en consola y panel Programming.
- `Validate project` revisa estructura, assets, escenas, prefabs y graphs.
- `Profiler` muestra costos por sistemas como Gameplay, VisualGraph, Rhai, RTS y Physics.

## 9. Ruta Recomendada Para Aprender

1. Crea una escena `Top2D`.
2. Adjunta `InventoryEconomyLoop` al jugador.
3. Crea un sprite simple en `Sprites` y arrastralo a escena.
4. Guarda el jugador como prefab.
5. Crea una escena `RTS Demo`.
6. Adjunta `RTSProductionEconomy` al CommandCenter.
7. Exporta runtime debug y revisa `build/debug/<ProjectName>/runtime_manifest.json`.
