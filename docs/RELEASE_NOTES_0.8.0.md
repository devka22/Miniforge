# MiniForge 0.8.0 Developer Stability Update

## Enfoque

Esta release no agrega demos grandes. Estabiliza el flujo para crear juegos 2D/RTS desde cero usando el editor actual, scripting Rhai, visual graphs, escenas, prefabs, assets, input, audio, physics, UI y export runtime.

## Cambios Principales

- Nombre oficial unificado: MiniForge.
- `engine_config.json` con defaults, migracion, backup y recuperacion.
- Logs con niveles y archivo `logs/miniforge.log`.
- Editor interno de archivos para scripts, graphs, escenas, prefabs y JSON.
- Content Browser puede abrir assets editables sin reiniciar el motor.
- Content Browser con Sources, busqueda, filtros, asset grid y detalles inspirado en flujos de editores profesionales.
- Visual Graph Editor con nodos, pines, conexiones y layout persistente en `.mfgraph`.
- Guardado atomico de JSON/escenas/prefabs con backups.
- Validacion reforzada de escenas, prefabs, referencias, scripts y graphs.
- Visual Scripting reporta nodos invalidos sin romper Play Mode.
- Rhai hot reload se marca al guardar desde el editor.
- GitHub Actions para fmt, check, clippy y test.
- Tests de regresion para config, logs, edicion interna, Rhai, visual graphs, escenas/prefabs y export.

## Compatibilidad

Los proyectos existentes se migran al abrir. Las escenas legacy con `objects` se convierten a `entities`. Los backups se conservan como archivos `.bak`.

## Riesgo Residual

El editor de texto interno es intencionalmente simple en 0.8.0. Sirve para editar scripts/graphs y desbloquear el flujo sin salir del motor; futuras versiones pueden mejorar seleccion, scroll y autocompletado.
