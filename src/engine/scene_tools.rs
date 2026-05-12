use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Default)]
pub struct SceneTools;

impl SceneTools {
    pub fn backup_scene(scene_path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let scene_path = scene_path.as_ref();
        let backup = scene_path.with_extension("scene.bak");
        fs::copy(scene_path, &backup)?;
        Ok(backup)
    }

    pub fn duplicate_scene(scene_path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let scene_path = scene_path.as_ref();
        let filename = scene_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("scene.scene");
        let target = AssetTools::unique_path(
            scene_path.parent().unwrap_or_else(|| Path::new(".")),
            filename,
        );
        fs::copy(scene_path, &target)?;
        Ok(target)
    }

    pub fn delete_scene(scene_path: impl AsRef<Path>) -> io::Result<()> {
        if scene_path.as_ref().exists() {
            fs::remove_file(scene_path)?;
        }
        Ok(())
    }
}
