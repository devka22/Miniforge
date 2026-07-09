# Building the Qt Editor

## Requirements

- Rust 1.95 as declared by `rust-toolchain.toml`.
- CMake 3.24 or newer.
- Qt 6.5 or newer with Widgets, Quick, QuickWidgets and QML tooling.

The scripts do not install system dependencies.

If CMake or Qt are installed outside `PATH`, point the scripts at them explicitly:

```bash
MINIFORGE_QT_CMAKE=/path/to/cmake scripts/configure-editor
CMAKE_PREFIX_PATH=/path/to/Qt/6.5/macos scripts/configure-editor
```

`MINIFORGE_QT_CMAKE` may also point at `qt-cmake`.

## Commands

```bash
scripts/configure-editor
scripts/build-editor
scripts/run-editor projects/DefaultProject
scripts/package-editor
```

`scripts/build-editor` builds the Rust `editor_ffi` library through Cargo and then links the Qt shell against it.
`scripts/package-editor` runs the same configure/build path and then invokes CPack.

## Full Test Pass

```bash
scripts/test-editor
```

This runs Rust formatting/check/tests, runtime-only check, CMake build, CTest and QML lint when `qmllint` is available.
By default it uses `CARGO_TARGET_DIR=/tmp/miniforge-codex-target` so editor checks stay isolated from local build artifacts.

If TypeScript plugin dependencies are installed under `editor-plugins/typescript/node_modules`, `scripts/test-editor` also runs the plugin API typecheck. It never installs npm packages automatically.

CTest includes `miniforge_editor_bridge_smoke`, a C++ executable that links the Rust editor dylib through the public C ABI, opens `projects/DefaultProject`, reads hierarchy/inspector/assets/commands/console data and requests a viewport RGBA snapshot.

To run only that smoke test after configuring:

```bash
ctest --test-dir build/editor-qt --output-on-failure
```
