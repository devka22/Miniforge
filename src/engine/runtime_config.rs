use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub path: PathBuf,
    pub data: Value,
}

impl RuntimeConfig {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = if path.exists() {
            AssetTools::read_json(&path)?
        } else {
            json!({
                "game_name": "MiniForgeGame",
                "start_scene": "main.scene",
                "window_width": 1100,
                "window_height": 740,
                "fullscreen": false,
                "target_fps": 60,
                "debug": true,
            })
        };
        AssetTools::write_json(&path, &data)?;
        Ok(Self { path, data })
    }

    pub fn get(&self, key: &str, default: Value) -> Value {
        self.data.get(key).cloned().unwrap_or(default)
    }
}
