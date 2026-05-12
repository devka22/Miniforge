use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct BuildReport;

impl BuildReport {
    pub fn generate(build_path: impl AsRef<Path>) -> io::Result<Value> {
        let build_path = build_path.as_ref();
        let files = walk_files(build_path)?;
        let bytes: u64 = files
            .iter()
            .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
            .sum();
        let report = json!({
            "summary": {
                "files": files.len(),
                "bytes": bytes,
            },
            "files": files.iter().map(|path| {
                path.strip_prefix(build_path).unwrap_or(path).to_string_lossy().to_string()
            }).collect::<Vec<_>>(),
        });
        AssetTools::write_json(build_path.join("build_report.json"), &report)?;
        Ok(report)
    }
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
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
