# MiniForge

MiniForge es un motor 2D en Rust con editor, runtime exportable, scripting Luau, visual graphs, fisica 2D, audio, pipeline de assets y tooling de proyecto. La version actual documentada es `0.9.3.4 - 2D Workflow Foundations`.

Esta rama suma una capa de escena inspirada por buenas ideas de Godot, manteniendo el nucleo Rust y el modelo `GameObject`/`Component` existente:

- `NodePath` y `SceneTreeIndex` para resolver rutas absolutas/relativas, hijos, raices y grupos.
- `PackedScene2D` para empaquetar una rama de entidades e instanciarla con IDs nuevos.
- `SceneSignalBus` para conexiones por `target_id` o `target_path`, validacion y despachos deterministas.
- Componentes nuevos: `Node2D`, `SceneTreeNode`, `GroupMembership`, `SignalEmitter`, `PackedSceneInstance` y `ResourceReference`.
- Comandos de editor: `object.create_node2d`, `object.create_area2d`, `object.create_character_body2d`, `scene.audit_tree` y `scene.pack_selected`.

Godot es MIT y una referencia enorme de arquitectura open source; MiniForge toma patrones de escena/editor y los reimplementa de forma nativa en Rust, sin copiar el motor C++.

## Uso Rapido

```bash
cargo run --bin miniforge_editor --features editor
cargo run --bin miniforge_runtime --features runtime
cargo run --bin miniforge_dev --features editor -- doctor
```

Checks recomendados:

```bash
cargo fmt
cargo check --features editor
cargo check --no-default-features --features runtime
cargo test --features editor --lib
```

## Documentacion

- [Arquitectura y runtime](docs/ARQUITECTURA_Y_RUNTIME.md)
- [Editor y flujo de uso](docs/EDITOR_Y_FLUJO_DE_USO.md)
- [Desarrollo, build y extension](docs/DESARROLLO_BUILD_Y_EXTENSION.md)
- [Datos, scripting y APIs](docs/DATOS_SCRIPTING_Y_APIS.md)

## Licencia

MiniForge usa licencia MIT. La inspiracion arquitectonica de Godot se referencia desde <https://github.com/godotengine/godot>, tambien MIT.
