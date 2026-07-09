# MiniForge Qt/QML Guide

## Estructura

- `editor-cpp`: aplicación Qt 6, docking, modelos y wrapper de la ABI.
- `editor-qml`: paneles visuales y componentes reutilizables.
- `src/engine/editor_core.rs`: servicios reales del editor en Rust.
- `src/editor_ffi.rs`: frontera C ABI.

## Paneles

Los paneles QML reciben modelos Qt por context properties:

- `hierarchyModel`
- `inspectorModel`
- `contentModel`
- `commandModel`
- `consoleModel`
- `editorController`

La lógica de motor no vive en QML. QML dispara comandos o selección mediante `editorController`, C++ llama la ABI, y Rust muta el estado real.

## Añadir un Panel

1. Crear un modelo Qt si el panel necesita datos propios.
2. Exponer el modelo en `MainWindow::makeQmlPanel`.
3. Crear el archivo QML en `editor-qml/panels`.
4. Montarlo en un `QDockWidget` desde `MainWindow::createPanels`.

## Temas

El tema inicial está en `editor-qml/themes/DarkTheme.qml`. Los componentes no deben hardcodear colores fuera del tema salvo que representen datos.
