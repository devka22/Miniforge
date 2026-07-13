#![cfg(feature = "editor_core")]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::automation_bridge::AutomationBridge;
use miniforge::engine::runtime_exporter::{ExportProfile, RuntimeExporter};
use miniforge::engine::script_host_2d::ScriptLanguage2D;
use miniforge::render::backend::GraphicsApi;
use serde_json::json;

fn temp_project(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-{label}-{stamp}"));
    AssetTools::ensure_project_folders(&path).unwrap();
    path
}

#[test]
fn automation_bridge_reports_python_csharp_and_render_capabilities() {
    let project = temp_project("automation-bridge");
    AssetTools::write_json(
        project.join("engine_config.json"),
        &json!({
            "name": "AutomationBridgeLab",
            "rendering": {
                "backend": "opengl",
                "prefer_metal_on_macos": true,
                "opengl_compatibility": true,
                "metal": {
                    "use_memoryless_targets": true,
                    "use_frame_capture_labels": true,
                    "triple_buffering": true
                }
            }
        }),
    )
    .unwrap();

    AutomationBridge::install_python_tools(&project).unwrap();
    let scaffold =
        AutomationBridge::scaffold_csharp_plugin(&project, "render diagnostics").unwrap();
    assert!(scaffold.program_file.exists());
    assert!(scaffold.project_file.exists());

    let report = AutomationBridge::inspect_project(&project).unwrap();
    assert!(
        report
            .languages
            .iter()
            .any(|language| language.language == ScriptLanguage2D::Python && language.editor_only)
    );
    assert!(
        report
            .languages
            .iter()
            .any(|language| language.language == ScriptLanguage2D::CSharp && language.editor_only)
    );
    assert!(
        report
            .python_tools
            .iter()
            .any(|tool| tool.id == "project_health_matrix")
    );
    assert_eq!(report.render.selection.selected, GraphicsApi::OpenGl);
    assert!(
        report
            .plugins
            .capabilities
            .languages
            .get("csharp")
            .is_some_and(|plugins| plugins.contains(&"RenderDiagnostics".to_string()))
    );

    fs::remove_dir_all(project).unwrap();
}

#[test]
fn runtime_export_excludes_editor_only_tools_and_plugins() {
    let project = temp_project("automation-export");
    AutomationBridge::install_python_tools(&project).unwrap();
    AutomationBridge::scaffold_csharp_plugin(&project, "EditorOnlyCSharp").unwrap();

    let output_root = std::env::temp_dir().join("miniforge-automation-export-output");
    if output_root.exists() {
        fs::remove_dir_all(&output_root).unwrap();
    }
    let report =
        RuntimeExporter::export_with_profile(&project, &output_root, ExportProfile::Release)
            .unwrap();

    assert!(!report.output_path.join("tools").exists());
    assert!(!report.output_path.join("plugins").exists());

    fs::remove_dir_all(project).unwrap();
    fs::remove_dir_all(output_root).unwrap();
}
