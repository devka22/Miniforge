use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct BuildSettings {
    pub path: PathBuf,
    pub data: Value,
}

impl BuildSettings {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = if path.exists() {
            AssetTools::read_json(&path)?
        } else {
            json!({
                "game_name": "MiniForgeGame",
                "start_scene": "main.scene",
                "target_fps": 60,
                "window_width": 1100,
                "window_height": 740,
                "fullscreen": false,
                "debug_mode": true,
                "export_folder": "builds",
            })
        };
        AssetTools::write_json(&path, &data)?;
        Ok(Self { path, data })
    }

    pub fn get(&self, key: &str, default: Value) -> Value {
        self.data.get(key).cloned().unwrap_or(default)
    }

    pub fn set(&mut self, key: &str, value: Value) -> io::Result<()> {
        if let Some(map) = self.data.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        AssetTools::write_json(&self.path, &self.data)
    }
}
