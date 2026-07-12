# MiniForge MCP Feedback

Este archivo lo usa el MCP de MiniForge para apuntar problemas, recomendaciones y observaciones que luego puedes pasar a Codex para mejorar el motor.

## 2026-05-28 - RECOMMENDATION - Unificar version visible del motor

Fuente: configuracion inicial del MCP

`Cargo.toml` declara `0.9.2`, mientras que `src/engine/version.rs` mantiene `ENGINE_VERSION = "0.9.1.1"` y `ENGINE_STREAM_VERSION = "0.9.2"`. Es util, pero convendria mostrar ambas etiquetas claramente en launcher, documentacion y manifests para evitar confusion al crear proyectos desde herramientas externas.

## 2026-05-28 - RECOMMENDATION - Exponer generacion de proyectos como API/CLI nativa

Fuente: validacion del juego demo MCP_AstroHarvester

El MCP puede crear proyectos funcionales, pero actualmente replica parte del esquema JSON de escenas/componentes en JavaScript. Para evitar drift, convendria exponer un comando nativo del motor, por ejemplo `miniforge --create-project <path> --template <name>`, reutilizando `ProjectTemplates` y `AssetTools` desde Rust.

## 2026-05-28 - RECOMMENDATION - Respetar start_scene en el loader

Fuente: validacion del juego demo MCP_AstroHarvester

`SceneManager::new` inicia siempre con `main.scene`, por eso el proyecto generado usa ese nombre para ser compatible. Seria mejor que el arranque lea `engine_config.json` o `project.json` y use `start_scene`, asi las herramientas externas pueden crear proyectos con escenas iniciales personalizadas.

Estado 2026-05-28: corregido en el motor. `Game::from_project` ahora pasa `engine_config.start_scene` a `SceneManager::new_with_start_scene`, e Iron Treaty valida con `campaign_1836.scene`.

## 2026-05-28 - RECOMMENDATION - Agregar sistema mensual de gran estrategia

Fuente: template MCP_IronTreaty_1836

El motor ya puede representar provincias, pops, mercados, fabricas, diplomacia, research, ejercitos y rutas comerciales como componentes editables. El siguiente salto seria un `GrandStrategySystem` Rust que procese ticks mensuales: necesidades de pops, precios por oferta/demanda, produccion de fabricas, investigacion, militancia, supply de ejercitos y warscore.

## 2026-05-28 - NOTE - CLI nativa de proyectos agregada

Fuente: validacion de motor

Se agrego `miniforge --create-project <path> --template <name> [--force]` y se valido con `target/tmp/miniforge_cli_test`. Esto reduce la dependencia de herramientas externas cuando solo se quiere crear una base de proyecto.

## 2026-06-18T03:37:27.471Z - RECOMMENDATION - Move generated game templates to declarative manifests

Fuente: MCP_LoveStoryLab template implementation

The MCP now creates astro_harvester, grand_strategy_rts and love_story_lab projects, but the larger templates duplicate scene and asset schemas directly in JavaScript. A better next step is a templates/ folder with declarative JSON manifests plus shared helper builders, or a native Rust template API that the MCP can call. That would make future 2D story labs easier to maintain and safer against engine schema drift.
