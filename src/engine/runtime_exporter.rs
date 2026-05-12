use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::manifest_builder::ManifestBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportProfile {
    Debug,
    Release,
}

impl ExportProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeExportReport {
    pub output_path: PathBuf,
    pub profile: ExportProfile,
    pub copied_files: usize,
    pub used_assets: Vec<String>,
    pub missing_assets: Vec<String>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeExporter;

impl RuntimeExporter {
    pub fn export(
        project_path: impl AsRef<Path>,
        output_root: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        Ok(Self::export_with_profile(project_path, output_root, ExportProfile::Debug)?.output_path)
    }

    pub fn export_with_profile(
        project_path: impl AsRef<Path>,
        output_root: impl AsRef<Path>,
        profile: ExportProfile,
    ) -> io::Result<RuntimeExportReport> {
        let project_path = project_path.as_ref();
        let name = project_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("MiniForgeGame");
        let output = output_root.as_ref().join(profile.label()).join(name);
        if output.exists() {
            fs::remove_dir_all(&output)?;
        }
        let copied_files = copy_dir(project_path, &output)?;
        let manifest = ManifestBuilder::build_manifest(project_path).unwrap_or_else(|_| json!({}));
        let used_assets = collect_used_assets(&manifest);
        let missing_assets = detect_missing_assets(project_path, &used_assets);
        let manifest_path = output.join("runtime_manifest.json");
        AssetTools::write_json(
            &manifest_path,
            &json!({
                "engine_version": crate::engine::version::ENGINE_VERSION,
                "runtime": "rust",
                "profile": profile.label(),
                "used_assets": used_assets.clone(),
                "missing_assets": missing_assets.clone(),
                "source_manifest": manifest,
            }),
        )?;
        AssetTools::write_json(
            output.join("build_info.json"),
            &json!({
                "engine_version": crate::engine::version::ENGINE_VERSION,
                "runtime": "rust",
                "profile": profile.label(),
                "copied_files": copied_files,
                "missing_assets": missing_assets.len(),
            }),
        )?;
        Ok(RuntimeExportReport {
            output_path: output,
            profile,
            copied_files,
            used_assets,
            missing_assets,
            manifest_path,
        })
    }
}

fn copy_dir(source: &Path, target: &Path) -> io::Result<usize> {
    fs::create_dir_all(target)?;
    let mut copied = 0;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if should_skip_export(&path) {
            continue;
        }
        let target_path = target.join(entry.file_name());
        if path.is_dir() {
            copied += copy_dir(&path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, target_path)?;
            copied += 1;
        }
    }
    Ok(copied)
}

fn should_skip_export(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if matches!(
        name,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | "target" | "builds" | "build" | "exports"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("pyc") | Some("pyo")
    )
}

fn collect_used_assets(manifest: &Value) -> Vec<String> {
    let mut assets = Vec::new();
    for key in ["assets", "scenes", "settings", "prefabs"] {
        if let Some(values) = manifest.get(key).and_then(Value::as_array) {
            for value in values {
                if let Some(path) = value.as_str() {
                    assets.push(path.to_string());
                }
            }
        }
    }
    assets.sort();
    assets.dedup();
    assets
}

fn detect_missing_assets(project_path: &Path, used_assets: &[String]) -> Vec<String> {
    used_assets
        .iter()
        .filter(|relative| !project_path.join(relative).exists())
        .cloned()
        .collect()
}
