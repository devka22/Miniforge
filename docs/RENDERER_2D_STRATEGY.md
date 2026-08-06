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
tilemap and entities, expand CPU particles, draw interactive UI, drive player movement with WASD or
the arrow keys, and route pointer clicks and wheel input through the scriptless UI runtime:

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
script. Scene-level responsive UI Canvas data now renders panels, buttons, labels and image
textures, and `UIElement` supports both plain images and nine-slice frames. The preview discovers
and uploads these UI textures automatically. The runtime minimap centers on the player, draws nearby
road tiles and priority-sorted objectives, threats, NPCs, resources and vehicles under bounded tile
and marker budgets. Ambient `Light2D` applies a multiply pass, directional lights add a bounded
screen tint, and point lights use a built-in radial texture with additive blending. Point lights can
project rotated shadows from nearby `ShadowCaster2D` bounds under per-frame light/caster budgets.
The built-in texture is restored automatically with the rest of the GPU resources after device loss.
`Material2D.shader` can select built-in grayscale, sepia, invert and hit-flash WGSL effects with a
per-sprite strength. Effects travel as vertex data, so sprites with different effects still share
the same stable texture/blend batch. `NormalMap2D` and lit `Material2D` sprites now bind a second
linear texture view, select the most influential point or directional `Light2D`, rotate that light
into sprite tangent space and apply per-pixel diffuse lighting in WGSL. Color textures retain an
sRGB view while the same upload registry exposes normal maps without sRGB decoding. Normal texture
changes participate in stable batching and diagnostics. The Create menu, Inspector workflow,
viewport glyph and `lit_sprite` authoring preset provide the complete path without game code.
`GpuParticles2D` also runs persistent storage-buffer simulation and instanced soft-particle
rendering through WGPU compute, with a synchronized `ParticleEmitter` CPU fallback.
`WgpuBackend` now owns sampleable RGBA8-sRGB render targets with independent dimensions, clear
colors, target-specific pipelines and GPU readback. Off-screen sprite passes are encoded before the
window pass and can immediately be sampled as a color or normal texture. Target descriptors survive
physical device recovery, resource IDs occupy a scene-target namespace that cannot collide with
sequential asset uploads, self-feedback is rejected, and `render_target_passes` is exposed in frame
diagnostics.

The complete no-code camera path is available from **Create > Core 2D > Camera to Texture 2D**. The
command creates one ordinary scene entity with `Camera2D`, `RenderTexture2D` and a display
`SpriteRenderer`; the persistent `render-target://CameraTexture_<entity>` binding makes the result
save/reload/export safe. `miniforge_wgpu_preview` discovers those components, creates their GPU
attachments and renders the world from the authored camera before composing the main scene. Update
mode can be `always`, `once` or `manual`, lighting can be included independently, and device recovery
invalidates `once` targets so they are rebuilt. The target pass currently includes grid, tile layers,
sprites, atlas regions, normal maps and 2D lighting/shadows. The Authoring Hub **Include UI** switch
adds legacy UI, scene canvases, retained widgets and Unicode text through target-local clipping and
one glyph atlas per camera target. Multiple writes to the same target in one frame are rejected;
persistent compute particles inside the off-screen pass remain explicitly unsupported.

`PostProcessVolume2D` now drives a physical fullscreen WGSL composite instead of a preview-only
marker. The backend renders sprites, particles, retained UI and Unicode text into an intermediate
RGBA scene texture and then composites it to the window with exposure, contrast, saturation,
gamma, bloom, vignette, chromatic aberration, pixelation, scanlines, tint, damage flash and fog.
Multiple enabled volumes blend deterministically by priority and weight; legacy `Bloom2D`, `Fog2D`,
`DamageEffect2D`, `PixelArtShader2D` and `Distortion2D` components feed the same command for
compatibility. The scene target, bind groups and pipelines are recreated on resize or device loss.
`post_process_passes` and `post_process_effects` expose the actual work in frame diagnostics.

The complete no-code path is available from **Create > Effects & Audio > Post Process Volume 2D**
and from the cinematic, horror, pixel and damage presets in the Authoring Hub. Hardware coverage
renders the composite on Metal and reads back pixels to verify tint, vignette, UI inclusion and
diagnostics rather than only checking shader compilation.

Outdated and lost window surfaces are reconfigured and retried once; an occluded or
still-unavailable surface skips the frame without poisoning the next one. A Metal surface smoke
with the default 70×30 grid reduced 2,100 logical
sprite calls to one GPU draw and one texture binding while presenting three verified frames.
The device-loss callback now rebuilds the adapter/device pipeline, surface configuration, buffers
and every uploaded texture from CPU backups. The Metal surface smoke also destroys the device
mid-run and verifies that presentation resumes with `device_loss_recoveries=1`.

The shared 2D composer now turns visible tilemap cells, atlas-backed entities, CPU particles and
interactive UI geometry into backend-independent sprite quads with camera transforms, ordering and
screen culling. The main exported runtime still uses Macroquad while retained-canvas hierarchy
clipping, normal-mapped sprites, camera-to-texture world passes and fullscreen postprocess are
available in the WGPU preview; target-aware retained UI/text is available, while higher-fidelity
soft/cone shadows and custom hot-reloaded shader materials remain migration work.
Projects should therefore leave `experimental_wgpu` disabled for exports that need the full
production renderer. Project Settings keeps this migration state visible instead of hiding it in
JSON.

## Migration gates

The preview becomes playable only after these gates pass on macOS, Windows and Linux:

1. Window surface, presentation and device-loss lifecycle. Surface configuration, presentation,
   resize, lost/outdated surface recovery and complete GPU resource recreation are done.
2. Sprite atlas regions, camera transforms, clipping, blend modes and stable batching. Atlas
   regions, clipping, full-texture uploads, pixel-space transforms, conservative culling,
   persistent vertex uploads, five blend modes and stable contiguous batching are done.
3. Chunked tilemaps, UI draw lists, text, particles and render textures. Layered tile cells,
   Unicode text, responsive Canvas panels/buttons/labels/images, nine-slice frames, sliders,
   checkbox/toggle controls, inventory/ability slots, text-input feedback, a bounded runtime minimap
   and CPU-particle quads are done. Additive point-light emission is done. Runtime inventory and
   ability grids now use clipped, virtualized rows with scriptless wheel scrolling, and ScrollBox
   text uses the same scissor path. Ambient/directional light and bounded geometric point-light
   shadows are done. Tangent-space normal maps, persistent compute particles with CPU fallback and
   sampleable camera render targets for the sprite-expanded world are done. Target-aware legacy,
   canvas and retained UI/text are done. Target-aware compute particles, broader retained-canvas virtualization, chunk batching and
   higher-fidelity soft/cone shadows remain.
4. WGSL materials, post-processing and hot reload with readable shader diagnostics. Four built-in
   per-sprite WGSL effects and the physical fullscreen postprocess compositor are done; custom
   material compilation and hot reload remain.
5. Compute paths for particles and tile visibility, each with a CPU fallback. Persistent compute
   particles and their synchronized CPU fallback are done; tile visibility remains.
6. Golden-image parity tests against Macroquad plus GPU timing and repeated recovery stress.

Once all gates pass, `backend: "auto"` may prefer `wgpu`; until then a playable export keeps
Macroquad as the deterministic fallback.
