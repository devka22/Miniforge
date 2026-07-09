# Python Automation and 2D Effects

MiniForge exposes trusted, editor-only Python tools through `View > Python Tools`.
The window discovers `tools/*.mftool.json`, launches each script in isolated
Python mode and applies validated operations through the Rust editor services.
Python is never embedded in exported gameplay builds.

## Built-in production tools

- Batch import: copies a complete `import_drop/` tree into `assets/imported/`.
- Sprite conversion: converts JPEG, WebP and BMP sources to PNG.
- Atlas generation: packs sprites into as many 4096px atlas pages as necessary,
  writes PNG pages and `.atlas.json` metadata, and extrudes sprite borders.
- Bulk properties: updates the current selection as one undoable scene action.
- Procedural level: creates an editable deterministic level with lighting, water,
  fog and fire anchors.
- Project data export: writes scene, manifest and asset-index JSON files.
- Automated build: validates, rebuilds the manifest and exports a debug runtime.
- Animation processing: validates indexed animation assets and writes a report.
- Documentation generation: creates Markdown for assets, scenes, Python tools and
  the 2D rendering suite.
- Project health matrix: summarizes scenes, scripts, visual graphs, plugins,
  prefabs, sprites and render backend settings for production triage.

Use `Install/Refresh` once per project to copy the built-ins into its `tools/`
folder. Those copies are ordinary project files and can be adapted into custom
tools.

## Custom tool contract

A tool consists of a Python entry point and a `.mftool.json` manifest. It reads
one `miniforge-editor-tool-v1` JSON request from stdin and prints one JSON result
as its final non-empty stdout line. `tools/miniforge_editor_api.py` provides the
request and result helpers. Only trusted manifests and allow-listed editor
operations are accepted; entries must remain under the project's `tools/`
folder.

## Sprite Studio 2D

`View > Sprite Editor Window` opens the sprite editor as a movable, independent
workspace. It retains the docked version, pixel painting, palette, crop, outline,
flip/rotate, sprite-sheet slicing and animation timeline preview. Use `Dock` to
return it to the lower editor panel.

## Rendering suite

The component picker now includes `Light2D`, `ShadowCaster2D`, `NormalMap2D`,
`Water2D`, `Distortion2D`, `Fire2D`, `Fog2D`, `Outline2D`, `Bloom2D`,
`GpuParticles2D`, `DamageEffect2D` and `PixelArtShader2D`. Each component has a
production preset, shader identity, parameters and an explicit GPU/CPU fallback.
The post-process stack includes configured bloom, fog, outline, damage flash,
pixelation, underwater refraction and heat distortion defaults.

## Multilanguage bridge

`miniforge_dev automation <project>` reports the active language matrix:
Luau and `.mfgraph` for gameplay, Python for editor automation and C# for
editor/plugin tooling. `scaffold-csharp-plugin` creates an editor-only C# plugin
manifest and `dotnet` project that can grow into custom panels, render
diagnostics or OpenGL/Metal tooling. Runtime export excludes these editor-only
folders by default.
