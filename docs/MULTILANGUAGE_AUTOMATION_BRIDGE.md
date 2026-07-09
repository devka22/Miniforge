# MiniForge Multilanguage Automation Bridge

MiniForge modela los lenguajes del motor como capas separadas, no como un único
runtime mezclado:

- **Luau**: gameplay runtime seguro, hot reload y API de juego.
- **Visual Graphs (`.mfgraph`)**: lógica visual compilable a datos runtime.
- **Python**: automatización confiable de editor/proyecto mediante
  `miniforge-editor-tool-v1`; nunca entra en builds exportados.
- **C#**: plugins editor-only por manifiesto, pensados para paneles,
  diagnósticos, comandos externos y tooling de render.

## CLI

Inspeccionar capacidades multilenguaje:

```bash
cargo run --bin miniforge_dev -- automation projects/MCP_LoveStoryLab --json
```

Instalar las herramientas Python incorporadas en un proyecto:

```bash
cargo run --bin miniforge_dev -- automation projects/MCP_LoveStoryLab --install-python
```

Crear un plugin C# editor-only:

```bash
cargo run --bin miniforge_dev -- scaffold-csharp-plugin projects/MCP_LoveStoryLab RenderDiagnostics
```

El scaffold crea:

- `plugins/RenderDiagnostics/plugin.json`
- `plugins/RenderDiagnostics/src/RenderDiagnostics.csproj`
- `plugins/RenderDiagnostics/src/Program.cs`

El manifiesto declara `runtime_policy.export = "exclude"`, y el exportador de
runtime omite `tools/`, `plugins/` y `native/` por defecto.

## Render

La capa `render::backend` reconoce rutas de selección:

- `macroquad`: backend estable actual.
- `opengl`: compatibilidad para tooling/plugins heredados.
- `wgpu` + `prefer_metal_on_macos`: ruta experimental para Metal.

La configuración de Metal modela memoria temporal, etiquetas de frame capture,
triple buffering y compute futuro. Esto permite que el editor, plugins C# y
diagnósticos sepan qué capacidades existen aunque la ruta de shipping siga
protegida por Macroquad hasta tener paridad.

## Python automation

La tool nueva `Project Health Matrix` resume escenas, scripts, visual graphs,
plugins, prefabs, sprites y render config. Sirve como ejemplo pequeño de cómo
automatizar tareas de producción desde Python sin darle acceso directo al
runtime exportado.
