//! Loads `runtime_manifest.json` and `build_info.json` from an exported build folder.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct RuntimeLoadError {
    pub message: String,
}

impl fmt::Display for RuntimeLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeLoadError {}

impl From<io::Error> for RuntimeLoadError {
    fn from(value: io::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildInfo {
    pub engine_version: String,
    pub runtime: String,
    pub profile: String,
    pub copied_files: Option<u64>,
    pub missing_assets: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct LoadedRuntimeManifest {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub build_info_path: PathBuf,
    pub engine_version: String,
    pub profile: String,
    pub runtime: String,
    pub used_assets: Vec<String>,
    pub declared_missing: Vec<String>,
    /// Paths from manifest that do not exist under `root`
    pub validated_missing: Vec<String>,
    pub source_manifest: Value,
    pub build_info: Option<BuildInfo>,
}

#[derive(Debug, Default)]
pub struct RuntimeManifestLoader;

impl RuntimeManifestLoader {
    pub fn load(build_root: impl AsRef<Path>) -> Result<LoadedRuntimeManifest, RuntimeLoadError> {
        let root = build_root.as_ref().to_path_buf();
        let manifest_path = root.join("runtime_manifest.json");
        if !manifest_path.is_file() {
            return Err(RuntimeLoadError {
                message: format!("No se encontró runtime_manifest.json en {}", root.display()),
            });
        }
        let value: Value = AssetTools::read_json(&manifest_path).map_err(|e| RuntimeLoadError {
            message: format!("No se pudo leer runtime_manifest.json: {e}"),
        })?;

        let engine_version = value
            .get("engine_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let profile = value
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("debug")
            .to_string();
        let runtime = value
            .get("runtime")
            .and_then(|v| v.as_str())
            .unwrap_or("rust")
            .to_string();
        let used_assets = value
            .get("used_assets")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(Vec::<String>::new);
        let declared_missing = value
            .get("missing_assets")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(Vec::<String>::new);
        let source_manifest = value
            .get("source_manifest")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let mut validated_missing = Vec::new();
        for rel in &used_assets {
            let p = root.join(rel);
            if !p.exists() {
                validated_missing.push(rel.clone());
            }
        }
        for rel in &declared_missing {
            if !validated_missing.contains(rel) {
                validated_missing.push(rel.clone());
            }
        }
        validated_missing.sort();
        validated_missing.dedup();

        let build_info_path = root.join("build_info.json");
        let build_info = if build_info_path.is_file() {
            match AssetTools::read_json(&build_info_path) {
                Ok(v) => serde_json::from_value(v).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(LoadedRuntimeManifest {
            root,
            manifest_path,
            build_info_path,
            engine_version,
            profile,
            runtime,
            used_assets,
            declared_missing,
            validated_missing,
            source_manifest,
            build_info,
        })
    }

    /// Validates manifest without requiring full game boot.
    pub fn validate_tree(build_root: impl AsRef<Path>) -> Result<Vec<String>, RuntimeLoadError> {
        let loaded = Self::load(build_root)?;
        Ok(loaded.validated_missing)
    }
}

/// Crash-safe atomic write: write temp then rename.
pub fn write_json_atomic(path: impl AsRef<Path>, value: &Value) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(value).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("serialización JSON: {e}"),
        )
    })?;
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
