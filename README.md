<div align="center">

# MiniForge

### A lightweight, extensible 2D game engine built with Rust

MiniForge combines a visual editor, a standalone runtime, Luau scripting, 2D physics and production-oriented tools in a single native project.

**Current version: 0.9.3**

[Features](#features) · [Getting started](#getting-started) · [Architecture](#architecture) · [Roadmap](#roadmap)· [MMForgeweb](https://miniforge-web.vercel.app) 

</div>

---

## About MiniForge

MiniForge is an experimental open-source 2D game engine, runtime and editor written primarily in Rust.

The project is designed for developers who want a native and extensible environment for creating 2D games without depending on a large general-purpose engine.

MiniForge provides:

- A visual scene editor.
- A standalone game runtime.
- A headless runtime for automated validation and simulation.
- Luau scripting.
- Visual scripting.
- Rapier2D physics.
- Sprite and animation tools.
- Tilemap editing.
- Prefabs and reusable components.
- Runtime export and packaging tools.
- RTS, navigation and large-world foundations.

> MiniForge is under active development. APIs, project formats and editor workflows may change between releases.

---

## Features

### Visual editor

MiniForge includes a native editor built with Macroquad and egui.

The editor architecture contains tools for:

- Scene hierarchy.
- Entity selection.
- Component inspector.
- Content and file browsing.
- Dockable panels.
- Project launcher.
- Project settings.
- Command palette.
- Editor history.
- Undo and redo workflows.
- Play mode.
- Scene validation.
- Session recovery.
- Autosave.
- Asset previews.
- Sprite editing.
- Animation editing.
- Script editing and debugging.
- Tilemap brushes.
- Spatial editing tools.
- Visual input mapping.
- Xcode integration.

---

### Scene and entity system

Projects are structured around scenes, entities and reusable components.

MiniForge includes systems for:

- Scene creation and loading.
- Scene serialization.
- Scene validation.
- Hierarchical entities.
- Stable entity identifiers.
- Tags and layers.
- Layer visibility.
- Component registration.
- Component validation.
- Prefab serialization.
- Prefab inheritance and overrides.
- Advanced prefab workflows.
- Archetype libraries.

---

### Luau scripting

MiniForge uses Luau as its embedded scripting language.

Luau scripts can be used to implement game logic without recompiling the complete Rust engine.

The scripting architecture includes:

- Script loading and management.
- Runtime script execution.
- Script hosts for 2D entities.
- Editor integration.
- Script debugging tools.
- Native engine API bindings.
- Runtime error reporting.

Rhai was used by previous versions of MiniForge but has been replaced by Luau.

Example script:

```lua
local speed = 180

function on_start()
    print("Entity started")
end

function on_update(delta)
    local direction = input.get_axis("move_left", "move_right")
    transform.translate(direction * speed * delta, 0)
end
```

The available scripting API may vary while the engine is being developed.

---

### Visual scripting

MiniForge contains a node-based visual scripting system for creating game logic without writing every behavior manually.

The engine includes support for:

- Serializable visual graphs.
- Execution flow.
- Variables and conditions.
- Input events.
- Runtime graph execution.
- Visual input editing.
- Connections between visual graphs and engine systems.

---

### 2D physics

Physics simulation is powered by Rapier2D.

The engine architecture supports:

- Rigid bodies.
- Static and dynamic colliders.
- Collision detection.
- Physics-based movement.
- Runtime physics simulation.
- Scene-integrated physics components.
- Spatial queries and indexing.

---

### Rendering

MiniForge uses Macroquad as its current rendering and windowing foundation.

Rendering-related systems include:

- Sprite rendering.
- 2D cameras.
- Materials.
- Vector-based 2D drawing.
- UI rendering.
- Layered tilemaps.
- Animation graphs.
- Sprite animations.
- Asset previews.
- Experimental 3D foundations.

MiniForge is currently focused primarily on 2D development.

---

### Animation

The animation system includes:

- Sprite animation.
- Animation graphs.
- Animation playback.
- Editor animation tools.
- Runtime animation updates.
- Entity animation components.

---

### Tilemaps and world tools

MiniForge includes tools intended for tile-based and larger 2D projects:

- Tilemap layers.
- Tile brushes.
- Scene spatial indexing.
- World partition foundations.
- Streaming-oriented components.
- Runtime budgets.
- Object pooling.
- Spawn directors.
- Save shards.

---

### UI system

MiniForge contains its own runtime UI architecture.

Available foundations include:

- UI Canvas.
- Runtime UI elements.
- Advanced UI components.
- Vector canvas rendering.
- Input-connected UI.
- Serializable UI layouts.

The UI workflow is still evolving and should be considered experimental.

---

### Audio

Audio playback and mixing are implemented with Kira.

The audio architecture includes:

- Runtime audio voices.
- Audio playback.
- Audio mixing.
- Audio resources.
- Integration with scenes and game systems.

---

### Navigation and strategy-game systems

MiniForge contains systems aimed at RTS, simulation and strategy games.

These include:

- A* and pathfinding foundations.
- Navigation grids.
- Spatial indexing.
- Unit formations.
- RTS command queues.
- Building placement.
- Game clocks.
- Event buses.
- Runtime schedulers.
- Influence and large-world foundations.
- Production-oriented entity systems.

---

### Asset pipeline

The asset system includes:

- Asset database.
- Asset importers.
- Asset references.
- Dependency resolution.
- Asset operations.
- Asset previews.
- 2D asset pipeline.
- Resource management.
- File watching.
- Project-relative resource loading.

Common supported asset types are intended to include images, sprites, audio, scenes, scripts, materials and project data.

---

### Runtime and export

MiniForge provides separate editor and runtime executables.

The project currently defines:

| Executable | Purpose |
|---|---|
| `miniforge` | Default MiniForge editor |
| `miniforge_editor` | Explicit editor executable |
| `miniforge_runtime` | Graphical exported-project runtime |
| `miniforge_headless` | Command-line simulation and validation |
| `miniforge_dev` | Development-oriented editor executable |

Runtime tools include:

- Runtime manifests.
- Project validation.
- Build profiles.
- Build reports.
- Packaging.
- Runtime configuration.
- Exported game player.
- Headless execution.
- Crash reporting.
- Safe mode.

---

## Technology

MiniForge is built using the following main technologies:

| Technology | Purpose |
|---|---|
| Rust | Engine, editor and runtime |
| Macroquad | Rendering, windowing and input |
| egui | Editor user interface |
| Rapier2D | 2D physics |
| Luau / mlua | Game scripting |
| Kira | Audio |
| Serde | Data serialization |
| Rayon | Parallel processing |
| pathfinding | Navigation algorithms |
| petgraph | Graph-based systems |
| notify | File watching |
| image | Image processing |
| lyon | Vector geometry |
| resvg / usvg | SVG processing |

---

## Requirements

Before compiling MiniForge, install:

- Git.
- Rust.
- Cargo.
- A supported native development toolchain.

The project currently declares:

```text
Rust 1.95 or newer
Rust edition 2024
```

### macOS

Install the Xcode Command Line Tools:

```bash
xcode-select --install
```

### Linux

Depending on your distribution, native windowing, audio and graphics development packages may also be required.

### Windows

Install the Rust MSVC toolchain and the Visual Studio C++ Build Tools.

---

## Getting started

Clone the repository:

```bash
git clone https://github.com/devka22/Miniforge.git
cd Miniforge
```

Compile the project:

```bash
cargo build
```

Run the editor:

```bash
cargo run
```

You can also explicitly run the editor binary:

```bash
cargo run --bin miniforge_editor
```

For optimized editor builds:

```bash
cargo run --release
```

---

## Runtime-only build

MiniForge separates editor functionality from the game runtime through Cargo features.

Compile the runtime without editor dependencies:

```bash
cargo build \
  --no-default-features \
  --features runtime \
  --bin miniforge_runtime
```

Run the runtime:

```bash
cargo run \
  --no-default-features \
  --features runtime \
  --bin miniforge_runtime
```

The graphical runtime expects a valid exported MiniForge project or runtime manifest.

---

## Headless runtime

The headless runtime executes a project without opening the graphical editor.

```bash
cargo run \
  --no-default-features \
  --features runtime \
  --bin miniforge_headless \
  -- path/to/project 60
```

Arguments:

```text
miniforge_headless <project> [steps]
```

Example:

```bash
cargo run \
  --no-default-features \
  --features runtime \
  --bin miniforge_headless \
  -- ./examples/my_project 120
```

The command returns a JSON report containing information such as:

- Simulated steps.
- Simulated time.
- Entity count.
- World validation status.
- Executed Luau scripts.
- Script errors.
- Executed visual graphs.
- Animated entities.
- Active audio voices.

The process exits with a failure status when the world is invalid or Luau errors are detected, making it useful for automated testing and CI.

---

## Cargo features

| Feature | Description |
|---|---|
| `editor` | Enables the visual editor and its UI dependencies |
| `runtime` | Enables the standalone engine runtime |
| `editor_ffi` | Enables the editor FFI bridge |
| Default | Enables `editor` |

Example runtime-only compilation:

```bash
cargo check --no-default-features --features runtime
```

Example editor compilation:

```bash
cargo check --features editor
```

---

## Distribution build

MiniForge includes a dedicated `ship` compilation profile for distributable runtimes.

```bash
cargo build \
  --profile ship \
  --no-default-features \
  --features runtime \
  --bin miniforge_runtime
```

The `ship` profile enables:

- Thin link-time optimization.
- A single code generation unit.
- Symbol stripping.
- Aborting on panic.

This profile is more compact but slower to compile than the standard development profiles.

---

## Development checks

Format the source code:

```bash
cargo fmt --all
```

Check the project:

```bash
cargo check --all-targets
```

Run Clippy:

```bash
cargo clippy --all-targets --all-features
```

Run tests:

```bash
cargo test --all-features
```

Recommended complete validation:

```bash
cargo fmt --all --check &&
cargo check --all-targets --all-features &&
cargo clippy --all-targets --all-features -- -D warnings &&
cargo test --all-features
```

---

## Architecture

The high-level source organization is:

```text
src/
├── main.rs                 # Default editor entry point
├── lib.rs                  # Public engine library
│
├── bin/
│   ├── miniforge_editor.rs
│   ├── miniforge_runtime.rs
│   ├── miniforge_headless.rs
│   └── miniforge_dev.rs
│
├── core/                   # Core application and game structures
├── editor_app/             # Editor startup and window configuration
├── engine/                 # Engine services and editor systems
├── entities/               # Entity implementations
├── input/                  # Input processing
├── map/                    # Maps and tile-related systems
├── pathfinding/            # Navigation algorithms
├── render/                 # Rendering foundations
├── runtime/                # Standalone game runtime
└── systems/                # Runtime game systems
```

The crate exposes the main runtime types:

```rust
use miniforge::{EngineRuntime, RuntimeWorld};
```

The editor is compiled only when the `editor` feature is enabled, allowing runtime builds to avoid editor-only dependencies.

---

## Project goals

MiniForge aims to become:

- A focused engine for 2D games.
- A native alternative for developers who prefer Rust.
- A visual editor with a fast development workflow.
- A scriptable engine using Luau.
- A foundation for platformers, top-down games, RTS projects and simulations.
- An engine whose editor and runtime can evolve independently.
- A practical environment for experimenting with engine architecture.

---

## Current status

MiniForge is a personal and experimental engine project.

Several systems already exist in the repository, but not every system has reached the same level of completeness or editor integration.

Expect:

- Breaking changes.
- Incomplete documentation.
- Experimental APIs.
- Editor workflows that may change.
- Features that exist as foundations but still require additional integration.
- Project-format changes between versions.

MiniForge is not yet recommended for critical production projects.

It is suitable for:

- Engine development experiments.
- Learning Rust game-engine architecture.
- Prototyping 2D games.
- Testing Luau integration.
- Building custom editor workflows.
- Exploring RTS and simulation systems.

---

## Roadmap

Areas currently suitable for continued development include:

- Editor usability and visual consistency.
- Canvas UI workflow.
- Complete sprite import workflow.
- Improved animation tooling.
- Luau API documentation.
- Runtime performance profiling.
- Render batching.
- Physics editor improvements.
- Project templates.
- Export presets.
- Asset dependency visualization.
- Plugin documentation.
- Automated tests.
- Example games.
- Cross-platform release builds.
- Stable project and scene formats.
- Complete end-user documentation.

---

## Contributing

Contributions, experiments and technical feedback are welcome.

A recommended contribution workflow is:

1. Fork the repository.
2. Create a development branch.
3. Make focused changes.
4. Run formatting, checks and tests.
5. Open a pull request explaining the change.

```bash
git checkout -b feature/my-improvement
cargo fmt --all
cargo check --all-targets --all-features
cargo test --all-features
```

When contributing, avoid mixing unrelated engine, editor and formatting changes in the same pull request.

---

## License

MiniForge is distributed under the MIT License.

See the `LICENSE` file for the complete license text.

---

## Author

Created and maintained by [devka22](https://github.com/devka22).

Repository:

```text
https://github.com/devka22/Miniforge
```

---

<div align="center">

**MiniForge — forge your own 2D worlds.**

</div>
