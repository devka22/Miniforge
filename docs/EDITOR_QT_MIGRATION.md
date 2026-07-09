# MiniForge Qt Editor Migration

Fecha: 2026-06-27

## Estado Encontrado

- El editor legacy entra por `src/main.rs` y `src/bin/miniforge_editor.rs`, ambos sobre `src/editor_app.rs`.
- La ventana y el frame loop actuales usan Macroquad; los paneles de editor usan egui y `egui_dock`.
- El runtime exportado ya tiene composition root separado en `src/runtime/engine_runtime.rs`.
- El estado del editor todavía vive principalmente en `Game` y `editor_app.rs`, por lo que Qt necesita una fachada Rust antes de tocar datos reales.
- En esta máquina no hay `cmake`, `qmake6` ni `qt-cmake` en PATH; los scripts detectan ese caso y no instalan dependencias automáticamente.

## Arquitectura Agregada

```text
Rust engine/editor core
  -> src/engine/editor_core.rs
  -> src/editor_ffi.rs
  -> include/miniforge_editor_bridge.h
  -> editor-cpp Qt shell
  -> editor-qml panels
```

`EditorCore` envuelve servicios reales de `Game`: apertura de proyecto, entidades, selección, inspector, assets, comandos, consola y snapshot RGBA del viewport. La ABI C sólo cruza handles opacos, structs planos y buffers propiedad del caller.

## Ownership y Threading

- Rust crea y destruye `MfEditorHandle` con `mf_editor_create` y `mf_editor_destroy`.
- Qt/C++ nunca recibe referencias internas a `Game`, entidades, assets ni strings Rust.
- Las filas de modelos se copian a DTOs C y después a modelos Qt.
- Los buffers de strings e imágenes son proporcionados por C++.
- V1 asume llamadas desde el main thread del editor Qt; workers de assets y render dedicado quedan para fases posteriores.

## Vertical Slice

- `editor-cpp` crea un `QMainWindow` con `QDockWidget`.
- `Hierarchy`, `Inspector`, `Content Browser`, `Console` y `Command Palette` se renderizan con QML y modelos Qt.
- `ViewportWidget` muestra un `QImage` RGBA generado desde la escena real en Rust.
- El editor legacy Macroquad/egui permanece intacto.

## Limitaciones de V1

- La jerarquía Qt es plana y conserva `parentId`/`childCount`; el árbol incremental completo queda pendiente.
- El viewport usa copia CPU RGBA, no textura compartida ni zero-copy GPU.
- Los modelos se refrescan con reset completo después de comandos; `beginInsertRows`/`dataChanged` granular queda para el siguiente corte.
- Plugins TypeScript quedan documentados como dirección, no implementados todavía.

## Comandos

```bash
scripts/configure-editor
scripts/build-editor
scripts/run-editor projects/DefaultProject
scripts/test-editor
```

Si CMake o Qt 6 no están instalados, `scripts/configure-editor` termina con código 2 y un mensaje accionable.
