# MiniForge Rust/C++ Editor Bridge

## Contrato

La ABI pública del editor está en `include/miniforge_editor_bridge.h`. El contrato evita exponer memoria interna de Rust y usa:

- `MfEditorHandle*` como handle opaco del editor.
- `MfEntityId`, `MfAssetId`, `MfSceneId` y `MfCommandId` como enteros opacos.
- `MfStatus` para el resultado de cada llamada.
- `MfError` con código y mensaje copiado a buffer fijo.
- Structs versionados con `abi_version` y `struct_size`.

## Reglas de Memoria

- Rust crea `MfEditorHandle` con `mf_editor_create`.
- C++ destruye el handle con `mf_editor_destroy`.
- C++ asigna todos los buffers de salida.
- Rust copia strings e imágenes a buffers del caller.
- Rust no devuelve punteros internos a entidades, assets, strings, arrays ni imágenes.

## Errores

Cada función que puede fallar acepta `MfError*`. Los errores de proyecto no abierto, argumento inválido, no encontrado, IO y comando fallido se traducen a `MfStatus`.

## Lectura por Lotes

La ABI conserva funciones fila-a-fila por compatibilidad, pero el editor Qt debe preferir las funciones batch:

- `mf_editor_entity_rows`
- `mf_editor_inspector_fields`
- `mf_editor_asset_rows`
- `mf_editor_command_descriptors`
- `mf_editor_console_entries`

El caller asigna el array y recibe `out_written` y `out_total`. Esto evita devolver slices internos de Rust y reduce llamadas FFI desde los modelos Qt.

## Edición de Inspector

`mf_editor_set_inspector_value_json` acepta `entity_id`, `target`, `key` y un valor JSON. La edición se aplica en Rust, refresca los snapshots ligeros del `EditorCore` y el editor Qt vuelve a leer los campos desde la ABI. C++/QML no mutan structs internos ni mantienen punteros a campos del inspector.

## Viewport

`mf_editor_viewport_snapshot_rgba` genera un snapshot RGBA desde datos reales de escena. V1 prioriza integración y compilabilidad; el plan de fase siguiente es integrar el renderer Rust con Qt mediante textura compartida o render target externo.

## Verificación

`editor-cpp/tests/bridge_smoke.cpp` prueba la ABI desde C++ con el mismo header público que usa el editor Qt. Cubre creación/destrucción del handle, errores antes de abrir proyecto, apertura de `projects/DefaultProject`, lectura batch de entidades/assets/comandos/consola, selección, inspector y snapshot RGBA.

## Evolución

La ABI puede crecer agregando funciones y structs nuevos. No se deben cambiar campos existentes sin incrementar versión y mantener compatibilidad.
