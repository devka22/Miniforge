use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct BrowserAsset {
    pub path: PathBuf,
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
}

#[derive(Debug, Clone)]
pub struct FileBrowser {
    pub project_path: PathBuf,
    pub selected_asset: Option<BrowserAsset>,
    pub pending_delete: Option<PathBuf>,
    pub tree_view: bool,
}

impl FileBrowser {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            selected_asset: None,
            pending_delete: None,
            tree_view: false,
        }
    }

    pub fn select_asset_by_path(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        let relative_path = path
            .strip_prefix(&self.project_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        self.selected_asset = Some(BrowserAsset {
            name: path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("asset")
                .to_string(),
            asset_type: asset_type(&path),
            path,
            relative_path,
        });
        self.pending_delete = None;
    }

    pub fn delete_selected_asset(&mut self) -> io::Result<bool> {
        let Some(asset) = &self.selected_asset else {
            return Ok(false);
        };
        if self.pending_delete.as_ref() != Some(&asset.path) {
            self.pending_delete = Some(asset.path.clone());
            return Ok(false);
        }
        if asset.path.is_dir() {
            fs::remove_dir_all(&asset.path)?;
        } else if asset.path.exists() {
            fs::remove_file(&asset.path)?;
        }
        self.pending_delete = None;
        Ok(true)
    }

    pub fn duplicate_selected_asset(&mut self) -> io::Result<Option<PathBuf>> {
        let Some(asset) = &self.selected_asset else {
            return Ok(None);
        };
        let filename = asset
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Asset");
        let target = AssetTools::unique_path(
            asset.path.parent().unwrap_or_else(|| Path::new(".")),
            filename,
        );
        if asset.path.is_dir() {
            copy_dir(&asset.path, &target)?;
        } else {
            fs::copy(&asset.path, &target)?;
        }
        Ok(Some(target))
    }

    pub fn toggle_tree_view(&mut self) -> bool {
        self.tree_view = !self.tree_view;
        self.tree_view
    }

    pub fn create_script(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_script(&self.project_path, name)
    }

    pub fn create_visual_graph(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_visual_graph(&self.project_path, name)
    }

    pub fn create_component(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_component(&self.project_path, name)
    }

    pub fn create_system(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_system(&self.project_path, name)
    }

    pub fn create_json(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_json(&self.project_path, None, name)
    }

    pub fn create_txt(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_txt(&self.project_path, None, name)
    }

    pub fn create_scene(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_scene(&self.project_path, name)
    }

    pub fn create_prefab(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_prefab(&self.project_path, name)
    }

    pub fn selected_asset_value(&self) -> serde_json::Value {
        if let Some(asset) = &self.selected_asset {
            json!({
                "path": asset.path,
                "relative_path": asset.relative_path,
                "name": asset.name,
                "type": asset.asset_type,
            })
        } else {
            serde_json::Value::Null
        }
    }
}

fn asset_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "aseprite" => "Sprite",
        "wav" | "mp3" | "ogg" | "flac" => "Audio",
        "prefab" => "Prefab",
        "scene" => "Scene",
        "mfgraph" => "VisualGraph",
        "py" => "LegacyScript",
        "json" | "txt" | "csv" | "ron" | "toml" => "Data",
        "glsl" | "wgsl" => "Shader",
        "ttf" | "otf" => "Font",
        "tmx" | "tsx" => "Tilemap",
        _ => "Asset",
    }
    .to_string()
}

fn copy_dir(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target_path = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target_path)?;
        } else {
            fs::copy(path, target_path)?;
        }
    }
    Ok(())
}
