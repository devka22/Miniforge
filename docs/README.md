# Documentación de MiniForge

Este directorio describe el estado real del motor MiniForge en la línea `0.9.3.4`. El crate de
Cargo conserva la versión `0.9.3`; la versión visible del motor está en
`package.metadata.miniforge.engine-version` y en `src/engine/version.rs`.

El editor visual soportado es Qt 6/C++/QML. Macroquad continúa siendo el backend gráfico del
runtime de los juegos, pero ya no existe un editor visual Rust/egui.

## Por Dónde Empezar

| Necesidad | Documento |
|---|---|
| Instalar y ejecutar el motor | [README principal](../README.md#inicio-rápido) |
| Aprender el editor y crear un juego | [Editor y flujo de uso](EDITOR_Y_FLUJO_DE_USO.md) |
| Entender el runtime y sus sistemas | [Arquitectura y runtime](ARQUITECTURA_Y_RUNTIME.md) |
| Programar con Luau o consumir APIs | [Datos, scripting y APIs](DATOS_SCRIPTING_Y_APIS.md) |
| Compilar, probar, exportar o extender | [Desarrollo, build y extensión](DESARROLLO_BUILD_Y_EXTENSION.md) |
| Revisar la retirada del editor Rust | [Migración definitiva al editor Qt](QT_EDITOR_MIGRATION.md) |

## Rutas De Lectura

### Usuario del editor

1. Sigue el [inicio rápido](../README.md#inicio-rápido).
2. Aprende los [workspaces y paneles](EDITOR_Y_FLUJO_DE_USO.md#workbench-workspaces-y-layout).
3. Recorre el [flujo base](EDITOR_Y_FLUJO_DE_USO.md#flujo-base).
4. Prueba con un template TopDown, Platformer o RTS.
5. Revisa [Play Mode, Safe Mode y recuperación](EDITOR_Y_FLUJO_DE_USO.md#play-mode).
6. Exporta con [Project Operations](EDITOR_Y_FLUJO_DE_USO.md#build-export-y-ejecución-externa).

### Programador Luau

1. Lee el [modelo de callbacks](DATOS_SCRIPTING_Y_APIS.md#luau).
2. Usa `types/miniforge.luau` como contrato de tipos.
3. Consulta la [tabla de namespaces](DATOS_SCRIPTING_Y_APIS.md#api-luau-por-namespace).
4. Trabaja desde [Luau Studio](DATOS_SCRIPTING_Y_APIS.md#luau-studio).
5. Usa el [debugger callback-level](DATOS_SCRIPTING_Y_APIS.md#debugger-luau).
6. Revisa límites, scheduler y consultas espaciales antes de construir mundos grandes.

### Desarrollador del motor

1. Lee la [separación de capas](ARQUITECTURA_Y_RUNTIME.md#mapa-de-arquitectura).
2. Conserva el límite entre `EngineRuntime`, `EditorCore` y `editor_ffi`.
3. Consulta el [loop de frame](ARQUITECTURA_Y_RUNTIME.md#loop-de-frame) y el
   [scheduler](ARQUITECTURA_Y_RUNTIME.md#scheduler-de-sistemas-y-presupuestos).
4. Ejecuta los [gates de validación](DESARROLLO_BUILD_Y_EXTENSION.md#matriz-de-validación).
5. Versiona cualquier formato persistente y agrega fixtures de migración.

### Desarrollador de extensiones

1. Elige entre TypeScript editor-only, C# tooling, Python automation o librería nativa.
2. Declara capabilities y dependencias en el manifest correspondiente.
3. Respeta Safe Mode y no asumas que un plugin de editor es runtime-safe.
4. Valida el proyecto y el paquete sin depender de una sesión visual abierta.

## Documentos Canónicos

| Documento | Alcance | Fuente de verdad relacionada |
|---|---|---|
| `README.md` | Presentación, novedades e inicio rápido | `Cargo.toml`, scripts y CLI |
| `EDITOR_Y_FLUJO_DE_USO.md` | Operación completa del workbench Qt | `editor-cpp`, `editor-qml`, `MfBridge` |
| `ARQUITECTURA_Y_RUNTIME.md` | Capas, mundo, loop, estabilidad y sistemas | `src/runtime`, `src/core`, `src/engine`, `src/systems` |
| `DATOS_SCRIPTING_Y_APIS.md` | Schemas, Luau, graphs, ABI y APIs públicas | serializadores, `types/miniforge.luau`, header C |
| `DESARROLLO_BUILD_Y_EXTENSION.md` | Toolchain, pruebas, export y extensiones | `Cargo.toml`, `scripts`, `tools`, plugins |
| `QT_EDITOR_MIGRATION.md` | Decisión arquitectónica y gates de Qt | gate anti-editor-Rust y tests Qt |

Los archivos `MINIFORGE_MCP_FEEDBACK*.md` y `MFORGE_MCP_FEEDBACK*.md` son feedback y backlog
histórico. No deben interpretarse como documentación de capacidades actuales: una recomendación
puede estar ya implementada, reemplazada o todavía pendiente.

## Contratos Y Terminología

- **Crate version:** versión semántica del paquete Rust, actualmente `0.9.3`.
- **Engine version:** versión de documentos/runtime, actualmente `0.9.3.4`.
- **Editor:** aplicación Qt 6 construida desde `editor-cpp` y `editor-qml`.
- **EditorCore:** backend Rust frontend-neutral de autoría.
- **Runtime:** `EngineRuntime` y los binarios gráfico/headless, sin UI de editor.
- **Play Mode:** simulación desde el contexto de autoría con snapshot restaurable.
- **External Play:** ejecución de un artefacto mediante `miniforge_runtime` en otro proceso.
- **Visual Graph:** scripting visual serializado como `.mfgraph` y ejecutado por Rust.
- **Luau Studio:** editor Qt de código; Luau runtime sigue viviendo en Rust/mlua.

## Política De Precisión

Al documentar una función nueva:

1. Verifica el nombre de comando, flag o símbolo contra el código.
2. Distingue una capacidad estable de una foundation experimental.
3. Indica si una herramienta es editor-only, runtime-safe o futura.
4. Documenta dónde persiste sus datos y cómo se recuperan.
5. Añade los límites conocidos; no presentes un prototipo como pipeline completo.
6. Actualiza enlaces cruzados y ejemplos ejecutables.

Fuentes preferidas para resolver discrepancias:

1. schemas, tipos públicos y tests;
2. implementación del backend y bridge;
3. QML/C++ que expone la operación;
4. documentación canónica;
5. feedback histórico.

## Convenciones De Comandos

Todos los comandos se ejecutan desde la raíz del repositorio. Los ejemplos con
`projects/DefaultProject` pueden sustituirse por la ruta de cualquier proyecto válido.

```bash
# Editor Qt
scripts/run-editor projects/DefaultProject
scripts/run-editor --launcher

# Un paso headless usando el mismo CLI desktop
scripts/run-editor --project projects/DefaultProject --headless-once

# Runtime gráfico de un export
cargo run --no-default-features --features runtime --bin miniforge_runtime \
  -- --build path/to/export

# Validación headless
cargo run --no-default-features --features runtime --bin miniforge_headless \
  -- path/to/project 120

# Validación integral del editor
scripts/test-editor
scripts/check-qt-backend-contract
```

## Mantenimiento De La Documentación

Antes de publicar:

- revisa la versión de crate y engine;
- comprueba que el editor se abre con el comando documentado;
- valida que todos los enlaces Markdown locales existan;
- elimina referencias al antiguo target `miniforge_editor` como comando vigente;
- actualiza contratos Luau cuando cambie `types/miniforge.luau`;
- actualiza el ABI cuando cambie `include/miniforge_editor_bridge.h`;
- ejecuta el check de contrato cuando cambie una llamada QML/C++;
- registra formatos nuevos, migraciones y rechazo de versiones futuras;
- ejecuta `git diff --check -- README.md docs`.
