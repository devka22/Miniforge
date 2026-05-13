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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileBrowserStats {
    pub folders: usize,
    pub files: usize,
    pub sprites: usize,
    pub audio: usize,
    pub scenes: usize,
    pub prefabs: usize,
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

    pub fn scan_entries(&self) -> io::Result<Vec<BrowserAsset>> {
        let mut entries = Vec::new();
        for path in walk_project(&self.project_path)? {
            let relative_path = path
                .strip_prefix(&self.project_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            entries.push(BrowserAsset {
                name: path
                    .file_stem()
                    .or_else(|| path.file_name())
                    .and_then(|value| value.to_str())
                    .unwrap_or("asset")
                    .to_string(),
                asset_type: if path.is_dir() {
                    "Folder".to_string()
                } else {
                    asset_type(&path)
                },
                path,
                relative_path,
            });
        }
        entries.sort_by(|a, b| {
            (a.asset_type != "Folder")
                .cmp(&(b.asset_type != "Folder"))
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
        Ok(entries)
    }

    pub fn stats(&self) -> io::Result<FileBrowserStats> {
        let mut stats = FileBrowserStats::default();
        for entry in self.scan_entries()? {
            match entry.asset_type.as_str() {
                "Folder" => stats.folders += 1,
                "Sprite" => {
                    stats.files += 1;
                    stats.sprites += 1;
                }
                "Audio" => {
                    stats.files += 1;
                    stats.audio += 1;
                }
                "Scene" => {
                    stats.files += 1;
                    stats.scenes += 1;
                }
                "Prefab" => {
                    stats.files += 1;
                    stats.prefabs += 1;
                }
                _ => stats.files += 1,
            }
        }
        Ok(stats)
    }

    pub fn create_folder(&self, parent: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let safe = AssetTools::safe_name(name, "NewFolder");
        let parent = self.resolve_project_path(parent);
        let path = AssetTools::unique_path(parent, &safe);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn rename_selected_asset(&mut self, new_name: &str) -> io::Result<Option<PathBuf>> {
        let Some(asset) = &self.selected_asset else {
            return Ok(None);
        };
        let safe = AssetTools::safe_name(new_name, "Asset");
        let old_name = asset
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let extension = if old_name.ends_with(".sprite.json") {
            ".sprite.json".to_string()
        } else if old_name.ends_with(".sound.json") {
            ".sound.json".to_string()
        } else if old_name.ends_with(".material.json") {
            ".material.json".to_string()
        } else {
            asset
                .path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| format!(".{value}"))
                .unwrap_or_default()
        };
        let filename = if asset.path.is_dir() || safe.ends_with(&extension) {
            safe
        } else {
            format!("{safe}{extension}")
        };
        let parent = asset.path.parent().unwrap_or(&self.project_path);
        let target = AssetTools::unique_path(parent, &filename);
        fs::rename(&asset.path, &target)?;
        self.select_asset_by_path(&target);
        Ok(Some(target))
    }

    pub fn move_selected_asset(
        &mut self,
        target_folder: impl AsRef<Path>,
    ) -> io::Result<Option<PathBuf>> {
        let Some(asset) = &self.selected_asset else {
            return Ok(None);
        };
        let target_folder = self.resolve_project_path(target_folder);
        fs::create_dir_all(&target_folder)?;
        let filename = asset
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Asset");
        let target = AssetTools::unique_path(target_folder, filename);
        fs::rename(&asset.path, &target)?;
        self.select_asset_by_path(&target);
        Ok(Some(target))
    }

    pub fn import_external_asset(
        &mut self,
        source_path: impl AsRef<Path>,
        folder_type: &str,
    ) -> io::Result<PathBuf> {
        let target_folder = AssetTools::create_special_folder(&self.project_path, folder_type)?;
        let imported = AssetTools::safe_copy_to_folder(source_path, target_folder)?;
        self.select_asset_by_path(&imported);
        Ok(imported)
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
        AssetTools::create_rhai_script(&self.project_path, name)
    }

    pub fn create_rhai_script(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_rhai_script(&self.project_path, name)
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

    pub fn create_enemy(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_enemy_prefab(&self.project_path, name)
    }

    pub fn create_ui(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_ui_asset(&self.project_path, name)
    }

    pub fn create_sprite_import(&self, name: &str, source_path: &str) -> io::Result<PathBuf> {
        AssetTools::create_sprite_import(&self.project_path, name, source_path)
    }

    pub fn create_sound_cue(&self, name: &str, source_path: &str) -> io::Result<PathBuf> {
        AssetTools::create_sound_cue(&self.project_path, name, source_path)
    }

    pub fn create_audio_event(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_audio_event(&self.project_path, name)
    }

    pub fn create_material(&self, name: &str) -> io::Result<PathBuf> {
        AssetTools::create_material(&self.project_path, name)
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

    fn resolve_project_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_path.join(path)
        }
    }
}

fn asset_type(path: &Path) -> String {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    if filename.ends_with(".sprite.json") {
        return "Sprite".to_string();
    }
    if filename.ends_with(".sound.json") {
        return "Audio".to_string();
    }
    if filename.ends_with(".audio.json") {
        return "AudioEvent".to_string();
    }
    if filename.ends_with(".material.json") {
        return "Material".to_string();
    }
    if filename.ends_with(".ui.prefab") {
        return "UI".to_string();
    }
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
        "rhai" => "RhaiScript",
        "mfgraph" => "VisualGraph",
        "json" | "txt" | "csv" | "ron" | "toml" => "Data",
        "glsl" | "wgsl" => "Shader",
        "ttf" | "otf" => "Font",
        "tmx" | "tsx" => "Tilemap",
        _ => "Asset",
    }
    .to_string()
}

fn walk_project(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if !root.exists() {
        return Ok(paths);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if ignored_path(&path) {
            continue;
        }
        paths.push(path.clone());
        if path.is_dir() {
            paths.extend(walk_project(&path)?);
        }
    }
    Ok(paths)
}

fn ignored_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(
        name,
        ".git" | "target" | ".DS_Store" | "asset_metadata.json"
    )
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
