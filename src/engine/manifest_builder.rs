use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Default)]
pub struct ManifestBuilder;

impl ManifestBuilder {
    pub fn build_manifest(project_path: impl AsRef<Path>) -> io::Result<serde_json::Value> {
        let project_path = project_path.as_ref();
        let paths = AssetTools::get_project_paths(project_path);
        let assets = walk_files(&paths.assets)?;
        let script_files = walk_files(&paths.scripts)?;
        let scripts = script_files
            .iter()
            .filter(|path| {
                matches!(
                    path.extension().and_then(|value| value.to_str()),
                    Some("mfgraph" | "rhai")
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let components = walk_files(&paths.components)?;
        let systems = walk_files(&paths.systems)?;
        let scenes = walk_files(&paths.scenes)?;
        let manifest = json!({
            "engine_version": crate::engine::version::ENGINE_VERSION,
            "runtime": "rust",
            "assets": rels(project_path, &assets),
            "scripts": rels(project_path, &scripts),
            "components": rels(project_path, &components),
            "systems": rels(project_path, &systems),
            "scenes": rels(project_path, &scenes),
        });
        AssetTools::write_json(project_path.join("manifest.json"), &manifest)?;
        Ok(manifest)
    }
}

fn rels(root: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string()
        })
        .collect()
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if ignored_dir(&path) {
                continue;
            }
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn ignored_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(name, ".git" | "target" | "builds" | ".cache")
}
