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

## Current safety rule

`WgpuBackend` is currently a compatibility scaffold and does not yet render production frames.
Projects must leave `experimental_wgpu` disabled to receive the Macroquad fallback. Project
Settings exposes the migration target and its explicit preview switch so this limitation is visible
instead of hidden in JSON.

## Migration gates

The preview becomes playable only after these gates pass on macOS, Windows and Linux:

1. Real adapter, device, surface and swap-chain lifecycle.
2. Sprite atlas uploads, camera transforms, clipping, blend modes and stable batching.
3. Chunked tilemaps, UI draw lists, text, particles and render textures.
4. WGSL materials, post-processing and hot reload with readable shader diagnostics.
5. Compute paths for particles and tile visibility, each with a CPU fallback.
6. Golden-image parity tests against Macroquad plus GPU timing and device-loss recovery.

Once all gates pass, `backend: "auto"` may prefer `wgpu`; until then a playable export keeps
Macroquad as the deterministic fallback.
