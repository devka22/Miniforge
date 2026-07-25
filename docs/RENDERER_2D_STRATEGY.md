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
one GPU buffer per frame. Contiguous quads that share texture and scissor state become one draw,
without reordering transparent geometry. `WgpuFrameDiagnostics` reports logical calls, culled
sprites, GPU draws, texture bindings, uploaded bytes, buffer capacity/reallocations, presentation
and surface recovery. A Metal surface smoke with the default 70×30 grid reduced 2,100 logical
sprite calls to one GPU draw and one texture binding while presenting three verified frames.

The shared 2D composer now turns visible tilemap cells, atlas-backed entities, CPU particles and
basic UI geometry into backend-independent sprite quads with camera transforms, ordering and screen
culling. The main exported runtime still uses Macroquad while text/minimap widgets, lighting,
render textures, batching and full device-loss recreation are completed. Projects should therefore
leave `experimental_wgpu` disabled for exports that need the full production renderer. Project
Settings keeps this migration state visible instead of hiding it in JSON.

## Migration gates

The preview becomes playable only after these gates pass on macOS, Windows and Linux:

1. Window surface, presentation and device-loss lifecycle. Surface configuration, presentation,
   resize and recoverable reconfiguration are done; complete device-loss recreation remains.
2. Sprite atlas regions, camera transforms, clipping, blend modes and stable batching. Atlas
   regions, clipping, full-texture uploads, pixel-space transforms, conservative culling,
   persistent vertex uploads and stable contiguous batching are done. Additional blend modes
   remain.
3. Chunked tilemaps, UI draw lists, text, particles and render textures. Layered tile cells, basic
   panels/progress bars and CPU-particle quads are done; text, advanced widgets, chunk batching,
   GPU particles and render textures remain.
4. WGSL materials, post-processing and hot reload with readable shader diagnostics.
5. Compute paths for particles and tile visibility, each with a CPU fallback.
6. Golden-image parity tests against Macroquad plus GPU timing and device-loss recovery.

Once all gates pass, `backend: "auto"` may prefer `wgpu`; until then a playable export keeps
Macroquad as the deterministic fallback.
