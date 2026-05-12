use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Default)]
pub struct RuntimeExporter;

impl RuntimeExporter {
    pub fn export(
        project_path: impl AsRef<Path>,
        output_root: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        let project_path = project_path.as_ref();
        let name = project_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("MiniForgeGame");
        let output = output_root.as_ref().join(name);
        if output.exists() {
            fs::remove_dir_all(&output)?;
        }
        copy_dir(project_path, &output)?;
        AssetTools::write_json(
            output.join("build_info.json"),
            &json!({"engine_version": crate::engine::version::ENGINE_VERSION, "runtime": "rust"}),
        )?;
        Ok(output)
    }
}

fn copy_dir(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if should_skip_export(&path) {
            continue;
        }
        let target_path = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target_path)?;
        } else {
            fs::copy(path, target_path)?;
        }
    }
    Ok(())
}

fn should_skip_export(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if matches!(
        name,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | "target" | "builds"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("pyc") | Some("pyo")
    )
}
