use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::editor_python::{PythonAutomationHost, PythonToolManifest};
use crate::engine::native_library::{NativeLibraryCategory, NativeLibraryManager};
use crate::engine::plugin_manager::{PluginLoadPlan, PluginManager};
use crate::engine::script_host_2d::{ScriptBackend2D, ScriptHost2D, ScriptLanguage2D};
use crate::engine::version::ENGINE_VERSION;
use crate::render::backend::{
    GraphicsApi, RenderBackendConfig, RenderBackendSelection, RenderDeviceCaps,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationBridgeReport {
    pub project_path: PathBuf,
    pub languages: Vec<LanguageBridgeSummary>,
    pub python_tools: Vec<PythonToolManifest>,
    pub plugins: PluginLoadPlan,
    pub native_manifests: Vec<NativeBridgeSummary>,
    pub render: RenderBridgeSummary,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageBridgeSummary {
    pub language: ScriptLanguage2D,
    pub state: String,
    pub adapter: String,
    pub runtime_safe: bool,
    pub editor_only: bool,
    pub hot_reload: bool,
    pub file_extensions: Vec<String>,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeBridgeSummary {
    pub id: String,
    pub library: PathBuf,
    pub enabled: bool,
    pub required: bool,
    pub category: NativeLibraryCategory,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderBridgeSummary {
    pub config: RenderBackendConfig,
    pub selection: RenderBackendSelection,
    pub caps: RenderDeviceCaps,
    #[serde(default)]
    pub plugin_capabilities: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CSharpPluginScaffold {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub project_file: PathBuf,
    pub program_file: PathBuf,
    pub readme_file: PathBuf,
    pub files_written: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct AutomationBridge;

impl AutomationBridge {
    pub fn inspect_project(project_path: impl AsRef<Path>) -> io::Result<AutomationBridgeReport> {
        let project_path = project_path.as_ref();
        let python = PythonAutomationHost::new(project_path);
        let python_tools = python.discover().unwrap_or_default();

        let mut plugins = PluginManager::new(project_path);
        let plugin_plan = plugins.load_plan()?;

        let native_manifests = NativeLibraryManager::new(project_path)
            .discover_manifests()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, manifest)| NativeBridgeSummary {
                id: manifest.id,
                library: manifest.library,
                enabled: manifest.enabled,
                required: manifest.required,
                category: manifest.category,
                services: manifest.services,
            })
            .collect::<Vec<_>>();

        let render_config = read_render_config(project_path);
        let render_selection = RenderBackendSelection::choose(&render_config);
        let render_caps = caps_for(render_selection.selected);

        let languages = ScriptHost2D::foundation()
            .language_matrix()
            .into_iter()
            .map(language_summary)
            .collect::<Vec<_>>();

        let mut recommendations = Vec::new();
        if python_tools.is_empty() {
            recommendations.push(
                "Instala las tools Python del motor para automatizar imports, atlas, reportes y builds."
                    .to_string(),
            );
        }
        if !plugin_plan.capabilities.languages.contains_key("csharp") {
            recommendations.push(
                "Genera un plugin C# editor-only para panels, diagnósticos de render o comandos externos."
                    .to_string(),
            );
        }
        if matches!(render_selection.selected, GraphicsApi::Macroquad)
            && render_config.prefer_metal_on_macos
        {
            recommendations.push(
                "Metal está preferido en config; activa experimental_wgpu sólo cuando el render graph tenga paridad con Macroquad."
                    .to_string(),
            );
        }
        if render_config.opengl_compatibility {
            recommendations.push(
                "OpenGL compatibility está modelado para plugins/herramientas; úsalo como fallback, no como ruta principal de shipping."
                    .to_string(),
            );
        }
        recommendations.sort();
        recommendations.dedup();

        Ok(AutomationBridgeReport {
            project_path: project_path.to_path_buf(),
            languages,
            python_tools,
            plugins: plugin_plan.clone(),
            native_manifests,
            render: RenderBridgeSummary {
                config: render_config,
                selection: render_selection,
                caps: render_caps,
                plugin_capabilities: plugin_plan.capabilities.render_backends,
            },
            recommendations,
        })
    }

    pub fn install_python_tools(project_path: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
        PythonAutomationHost::new(project_path).install_builtin_tools()
    }

    pub fn scaffold_csharp_plugin(
        project_path: impl AsRef<Path>,
        plugin_name: &str,
    ) -> io::Result<CSharpPluginScaffold> {
        let project_path = project_path.as_ref();
        let plugin_id = safe_identifier(plugin_name, "CSharpEditorPlugin");
        let root = project_path.join("plugins").join(&plugin_id);
        let src = root.join("src");
        fs::create_dir_all(&src)?;

        let manifest_path = root.join("plugin.json");
        let project_file = src.join(format!("{plugin_id}.csproj"));
        let program_file = src.join("Program.cs");
        let readme_file = root.join("README.md");
        let mut files_written = Vec::new();

        let manifest = json!({
            "name": plugin_id,
            "version": "0.1.0",
            "author": "MiniForge",
            "enabled": true,
            "description": "C# editor plugin scaffold for MiniForge panels, automation and render diagnostics.",
            "min_engine_version": ENGINE_VERSION,
            "language": "csharp",
            "languages": ["csharp"],
            "systems": ["editor_tools"],
            "editor_panels": ["automation_dashboard", "render_diagnostics"],
            "automation_tools": ["csharp_command_bridge"],
            "render_backends": ["opengl_compatibility", "metal_diagnostics"],
            "services": ["editor_plugin"],
            "hooks": {
                "editor.startup": "dotnet run --project src"
            },
            "entrypoints": {
                "project": format!("src/{plugin_id}.csproj"),
                "command": "dotnet run --project src",
                "protocol": "miniforge-plugin-command-v1"
            },
            "runtime_policy": {
                "editor_only": true,
                "export": "exclude"
            }
        });
        write_json_if_missing(&manifest_path, &manifest, &mut files_written)?;
        write_text_if_missing(&project_file, &csharp_project_file(), &mut files_written)?;
        write_text_if_missing(
            &program_file,
            &csharp_program_file(&plugin_id),
            &mut files_written,
        )?;
        write_text_if_missing(
            &readme_file,
            &csharp_plugin_readme(&plugin_id),
            &mut files_written,
        )?;

        Ok(CSharpPluginScaffold {
            root,
            manifest_path,
            project_file,
            program_file,
            readme_file,
            files_written,
        })
    }
}

fn language_summary(backend: ScriptBackend2D) -> LanguageBridgeSummary {
    let (editor_only, extensions) = match backend.language {
        ScriptLanguage2D::Luau => (false, vec![".luau", ".lua"]),
        ScriptLanguage2D::Blueprint => (false, vec![".mfgraph"]),
        ScriptLanguage2D::Python => (true, vec![".py", ".mftool.json"]),
        ScriptLanguage2D::CSharp => (true, vec![".cs", ".csproj"]),
    };
    LanguageBridgeSummary {
        language: backend.language,
        state: format!("{:?}", backend.state),
        adapter: backend.adapter,
        runtime_safe: backend.language.runtime_safe(),
        editor_only,
        hot_reload: backend.hot_reload,
        file_extensions: extensions.into_iter().map(str::to_string).collect(),
        purpose: backend.purpose,
    }
}

fn read_render_config(project_path: &Path) -> RenderBackendConfig {
    let mut config = RenderBackendConfig::default();
    let engine_config = AssetTools::read_json(project_path.join("engine_config.json"))
        .unwrap_or_else(|_| json!({}));
    let rendering = engine_config
        .get("rendering")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    set_string(&mut config.backend, &rendering, "backend");
    set_bool(
        &mut config.experimental_wgpu,
        &rendering,
        "experimental_wgpu",
    );
    set_bool(
        &mut config.prefer_metal_on_macos,
        &rendering,
        "prefer_metal_on_macos",
    );
    set_bool(&mut config.vsync, &rendering, "vsync");
    set_bool(&mut config.pixel_perfect, &rendering, "pixel_perfect");
    set_f32(&mut config.render_scale, &rendering, "render_scale");
    set_u32(&mut config.max_texture_size, &rendering, "max_texture_size");
    set_bool(&mut config.sprite_batching, &rendering, "sprite_batching");
    set_bool(
        &mut config.tilemap_chunk_batching,
        &rendering,
        "tilemap_chunk_batching",
    );
    set_bool(
        &mut config.view_frustum_culling,
        &rendering,
        "view_frustum_culling",
    );
    set_bool(
        &mut config.occlusion_culling,
        &rendering,
        "occlusion_culling",
    );
    set_bool(&mut config.lod_enabled, &rendering, "lod_enabled");
    set_bool(&mut config.backface_culling, &rendering, "backface_culling");
    set_bool(
        &mut config.backface_culling,
        &rendering,
        "backface_culling_3d",
    );
    set_bool(&mut config.gpu_particles, &rendering, "gpu_particles");
    set_bool(&mut config.post_processing, &rendering, "post_processing");
    set_bool(
        &mut config.opengl_compatibility,
        &rendering,
        "opengl_compatibility",
    );
    set_bool(
        &mut config.shader_hot_reload,
        &rendering,
        "shader_hot_reload",
    );
    if let Some(metal) = rendering.get("metal").and_then(Value::as_object) {
        set_bool(
            &mut config.metal.prefer_metal_on_macos,
            metal,
            "prefer_metal_on_macos",
        );
        set_bool(
            &mut config.metal.use_memoryless_targets,
            metal,
            "use_memoryless_targets",
        );
        set_bool(
            &mut config.metal.prefer_low_power_gpu,
            metal,
            "prefer_low_power_gpu",
        );
        set_bool(
            &mut config.metal.use_frame_capture_labels,
            metal,
            "use_frame_capture_labels",
        );
        set_bool(
            &mut config.metal.triple_buffering,
            metal,
            "triple_buffering",
        );
        set_bool(
            &mut config.metal.use_argument_buffers_future,
            metal,
            "use_argument_buffers_future",
        );
        set_bool(
            &mut config.metal.allow_compute_particles,
            metal,
            "allow_compute_particles",
        );
        set_bool(
            &mut config.metal.allow_compute_tile_visibility,
            metal,
            "allow_compute_tile_visibility",
        );
        set_bool(
            &mut config.metal.allow_compute_flow_fields,
            metal,
            "allow_compute_flow_fields",
        );
    }
    config
}

fn caps_for(api: GraphicsApi) -> RenderDeviceCaps {
    match api {
        GraphicsApi::Macroquad => RenderDeviceCaps::macroquad(),
        GraphicsApi::OpenGl => RenderDeviceCaps::opengl_compatibility(),
        GraphicsApi::WgpuMetal => RenderDeviceCaps::simulated_wgpu_metal(),
        GraphicsApi::WgpuVulkan => RenderDeviceCaps {
            api,
            device_name: "wgpu-vulkan-experimental".to_string(),
            max_texture_size: 8192,
            supports_compute: true,
            supports_storage_buffers: true,
            supports_timestamp_queries: false,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "rgba8unorm_srgb".to_string(),
        },
        GraphicsApi::WgpuDx12 => RenderDeviceCaps {
            api,
            device_name: "wgpu-dx12-experimental".to_string(),
            max_texture_size: 8192,
            supports_compute: true,
            supports_storage_buffers: true,
            supports_timestamp_queries: true,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "rgba8unorm_srgb".to_string(),
        },
        GraphicsApi::WgpuWebGpu => RenderDeviceCaps {
            api,
            device_name: "webgpu-future".to_string(),
            max_texture_size: 4096,
            supports_compute: false,
            supports_storage_buffers: true,
            supports_timestamp_queries: false,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "rgba8unorm_srgb".to_string(),
        },
    }
}

fn safe_identifier(input: &str, fallback: &str) -> String {
    let mut output = String::new();
    for part in input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
    {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            let rest = chars.collect::<String>();
            if rest.chars().any(|character| character.is_ascii_uppercase()) {
                output.push_str(&rest);
            } else {
                output.extend(rest.chars().map(|character| character.to_ascii_lowercase()));
            }
        }
    }
    if output.is_empty() {
        output = fallback.to_string();
    }
    if output
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        output.insert_str(0, "Plugin");
    }
    output
}

fn write_json_if_missing(
    path: &Path,
    value: &Value,
    files_written: &mut Vec<PathBuf>,
) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    AssetTools::write_json(path, value)?;
    files_written.push(path.to_path_buf());
    Ok(())
}

fn write_text_if_missing(
    path: &Path,
    contents: &str,
    files_written: &mut Vec<PathBuf>,
) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    files_written.push(path.to_path_buf());
    Ok(())
}

fn csharp_project_file() -> String {
    r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>
"#
    .to_string()
}

fn csharp_program_file(plugin_id: &str) -> String {
    const TEMPLATE: &str = r#"using System.Text.Json;
using System.Text.Json.Nodes;

var input = Console.In.ReadToEnd();
JsonNode? request = null;

try
{
    request = string.IsNullOrWhiteSpace(input) ? null : JsonNode.Parse(input);
}
catch (JsonException)
{
    request = null;
}

var projectRoot = request?["context"]?["project_root"]?.GetValue<string>() ?? Environment.CurrentDirectory;
var result = new JsonObject
{
    ["success"] = true,
    ["message"] = "__PLUGIN_ID__ C# plugin bridge online",
    ["operations"] = new JsonArray
    {
        new JsonObject
        {
            ["operation"] = "log",
            ["value"] = $"__PLUGIN_ID__ inspected {projectRoot}"
        }
    },
    ["generated_files"] = new JsonArray()
};

Console.WriteLine(result.ToJsonString(new JsonSerializerOptions { WriteIndented = false }));
"#;
    TEMPLATE.replace("__PLUGIN_ID__", plugin_id)
}

fn csharp_plugin_readme(plugin_id: &str) -> String {
    format!(
        "# {plugin_id}\n\n\
        C# editor-only plugin scaffold for MiniForge.\n\n\
        - Protocol: `miniforge-plugin-command-v1`\n\
        - Intended use: editor panels, automation commands, OpenGL compatibility tooling and Metal diagnostics.\n\
        - Export policy: `exclude`; this plugin is not copied into runtime builds.\n\n\
        Run locally with:\n\n\
        ```bash\n\
        dotnet run --project src/{plugin_id}.csproj\n\
        ```\n"
    )
}

fn set_string(target: &mut String, object: &serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get(key).and_then(Value::as_str) {
        *target = value.to_string();
    }
}

fn set_bool(target: &mut bool, object: &serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get(key).and_then(Value::as_bool) {
        *target = value;
    }
}

fn set_f32(target: &mut f32, object: &serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get(key).and_then(Value::as_f64) {
        *target = value as f32;
    }
}

fn set_u32(target: &mut u32, object: &serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get(key).and_then(Value::as_u64) {
        *target = value.min(u32::MAX as u64) as u32;
    }
}
