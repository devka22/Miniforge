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
readback. The same off-screen target is usable by renderer tests, Qt previews and future window
surfaces. Hardware coverage includes an ignored test that renders colored and textured geometry and
checks the resulting pixels.

The main game window still uses Macroquad while surface presentation, tilemap/UI/particle command
geometry and device-loss recovery are completed. Projects should therefore leave
`experimental_wgpu` disabled for exports that need the full production renderer. Project Settings
keeps this migration state visible instead of hiding it in JSON.

## Migration gates

The preview becomes playable only after these gates pass on macOS, Windows and Linux:

1. Window surface, presentation and device-loss lifecycle. Physical adapter/device/queue are done.
2. Sprite atlas regions, camera transforms, clipping, blend modes and stable batching. Full-texture
   uploads plus pixel-space sprite transforms are done.
3. Chunked tilemaps, UI draw lists, text, particles and render textures.
4. WGSL materials, post-processing and hot reload with readable shader diagnostics.
5. Compute paths for particles and tile visibility, each with a CPU fallback.
6. Golden-image parity tests against Macroquad plus GPU timing and device-loss recovery.

Once all gates pass, `backend: "auto"` may prefer `wgpu`; until then a playable export keeps
Macroquad as the deterministic fallback.
