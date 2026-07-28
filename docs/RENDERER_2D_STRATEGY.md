# MiniForge 2D renderer strategy

## Decision

MiniForge will evolve from Macroquad to a direct `wgpu` backend. Macroquad remains the production
fallback until the new backend has visual and performance parity.

This is an engine migration, not a switch to Bevy:

- Macroquad is deliberately small and convenient, and remains useful for fast, portable 2D builds.
- `wgpu` gives MiniForge direct access to Metal, Vulkan, DirectX 12 and WebGPU, modern shader and
  storage-buffer APIs, compute pipelines, timestamps and explicit render passes.
- Bevy also renders through `wgpu`, but adopting it would duplicate MiniForge's ECS, scene format,
  editor bridge, UI authoring and runtime scheduler. Direct `wgpu` preserves those public systems.

Primary references:

- <https://macroquad.rs/>
- <https://wgpu.rs/>
- <https://wgpu.rs/doc/wgpu/>
- <https://docs.rs/bevy/latest/bevy/render/>
- <https://docs.rs/bevy/latest/bevy/ui/>

## Current implementation

`WgpuBackend` now creates a real physical adapter, device and queue, compiles a WGSL sprite
pipeline, uploads RGBA8 textures, renders rotated/tinted sprite quads and supports deterministic GPU
readback. It supports both an off-screen target and owned window surfaces with sRGB selection,
vsync, resize/reconfigure, occlusion handling and presentation. Atlas UV regions and pixel-space
scissor clipping share the same sprite path. Hardware coverage includes an ignored test that renders
colored and atlas-textured geometry and checks the resulting pixels. A generic
`miniforge_wgpu_preview` binary exercises the window-surface path and can load any MiniForge
project through the normal `EngineRuntime`, upload its sprite assets, extract its camera, layered
tilemap and entities, expand CPU particles, draw basic UI panels/progress bars, and drive player
movement with WASD or the arrow keys:

```bash
cargo run --bin miniforge_wgpu_preview --features wgpu_runtime -- /path/to/project
```

The sprite path now keeps a persistent, geometrically growing vertex buffer instead of allocating
one GPU buffer per frame. Contiguous quads that share texture, scissor and blend state become one
draw, without reordering transparent geometry. Alpha, premultiplied-alpha, additive, multiply and
screen pipelines are selected through backend-independent `SpriteDrawOptions`; runtime entities
read the mode from `Material2D`/`SpriteRenderer`, and CPU particle emitters use the same path.
`WgpuFrameDiagnostics` reports logical calls, culled sprites, GPU draws, texture bindings, pipeline
changes, uploaded bytes, buffer capacity/reallocations, presentation and surface recovery.
Backend-independent `TextDrawCommand` areas now use `glyphon`/`cosmic-text` for Unicode shaping,
system-font fallback, word/glyph wrapping, clipping and a persistent GPU glyph atlas. Runtime
`UIElement` labels submit through this path, so HUD text is no longer a preview-only placeholder.
Runtime `UIElement` controls now also draw sliders, checkbox/toggle state, inventory and ability
slots, text-input placeholders and focused carets. Their built-in interaction updates values,
checked state, dropdown selection, focused inputs and selected inventory slots without a game
script.
Outdated and lost window surfaces are reconfigured and retried once; an occluded or
still-unavailable surface skips the frame without poisoning the next one. A Metal surface smoke
with the default 70×30 grid reduced 2,100 logical
sprite calls to one GPU draw and one texture binding while presenting three verified frames.
The device-loss callback now rebuilds the adapter/device pipeline, surface configuration, buffers
and every uploaded texture from CPU backups. The Metal surface smoke also destroys the device
mid-run and verifies that presentation resumes with `device_loss_recoveries=1`.

The shared 2D composer now turns visible tilemap cells, atlas-backed entities, CPU particles and
basic and interactive UI geometry into backend-independent sprite quads with camera transforms,
ordering and screen culling. The main exported runtime still uses Macroquad while canvas image and
nine-slice widgets, minimaps, lighting, render textures and shader materials are completed. Projects
should therefore leave `experimental_wgpu` disabled for exports that need the full production
renderer. Project Settings keeps this migration state visible instead of hiding it in JSON.

## Migration gates

The preview becomes playable only after these gates pass on macOS, Windows and Linux:

1. Window surface, presentation and device-loss lifecycle. Surface configuration, presentation,
   resize, lost/outdated surface recovery and complete GPU resource recreation are done.
2. Sprite atlas regions, camera transforms, clipping, blend modes and stable batching. Atlas
   regions, clipping, full-texture uploads, pixel-space transforms, conservative culling,
   persistent vertex uploads, five blend modes and stable contiguous batching are done.
3. Chunked tilemaps, UI draw lists, text, particles and render textures. Layered tile cells,
   Unicode text, panels/progress bars, sliders, checkbox/toggle controls, inventory/ability slots,
   text-input feedback and CPU-particle quads are done; canvas images/nine-slice, chunk batching,
   GPU particles and render textures remain.
4. WGSL materials, post-processing and hot reload with readable shader diagnostics.
5. Compute paths for particles and tile visibility, each with a CPU fallback.
6. Golden-image parity tests against Macroquad plus GPU timing and repeated recovery stress.

Once all gates pass, `backend: "auto"` may prefer `wgpu`; until then a playable export keeps
Macroquad as the deterministic fallback.
