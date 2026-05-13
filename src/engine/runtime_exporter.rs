use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::manifest_builder::ManifestBuilder;
use crate::engine::project_validator::ProjectValidator;

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
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
    pub release_optimized: bool,
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
        let mut validator = ProjectValidator::default();
        validator.validate(project_path);
        let copied_files = copy_dir(project_path, &output)?;
        let manifest = ManifestBuilder::build_manifest(project_path).unwrap_or_else(|_| json!({}));
        let used_assets = collect_used_assets(&manifest);
        let mut missing_assets = detect_missing_assets(project_path, &used_assets);
        missing_assets.extend(detect_missing_dependencies(project_path));
        missing_assets.sort();
        missing_assets.dedup();
        let manifest_path = output.join("runtime_manifest.json");
        AssetTools::write_json(
            &manifest_path,
            &json!({
                "engine_version": crate::engine::version::ENGINE_VERSION,
                "runtime": "rust",
                "profile": profile.label(),
                "release_optimized": profile == ExportProfile::Release,
                "used_assets": used_assets.clone(),
                "missing_assets": missing_assets.clone(),
                "validation": {
                    "errors": validator.errors.clone(),
                    "warnings": validator.warnings.clone(),
                },
                "source_manifest": manifest,
            }),
        )?;
        AssetTools::write_json(
            output.join("build_info.json"),
            &json!({
                "engine_version": crate::engine::version::ENGINE_VERSION,
                "runtime": "rust",
                "profile": profile.label(),
                "optimization": match profile {
                    ExportProfile::Debug => "debug-symbols",
                    ExportProfile::Release => "release-optimized",
                },
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
            validation_errors: validator.errors,
            validation_warnings: validator.warnings,
            release_optimized: profile == ExportProfile::Release,
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
    if matches!(name, ".cache" | "target" | "builds" | "build" | "exports") {
        return true;
    }
    false
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

fn detect_missing_dependencies(project_path: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    for path in walk_files(project_path) {
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("scene" | "prefab" | "json")
        ) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for token in text.split(['"', '\'', ' ', '\n', '\r', '\t', ',', ':', '[', ']']) {
            if !looks_like_project_asset(token) {
                continue;
            }
            if !project_path.join(token).exists() {
                missing.push(token.to_string());
            }
        }
    }
    missing
}

fn looks_like_project_asset(token: &str) -> bool {
    (token.starts_with("assets/") || token.starts_with("scripts/") || token.starts_with("saves/"))
        && token.contains('.')
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if should_skip_export(&path) {
            continue;
        }
        if path.is_dir() {
            files.extend(walk_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}
