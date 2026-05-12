use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::version::ENGINE_VERSION;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub path: PathBuf,
    pub data: Value,
}

impl EngineConfig {
    pub fn new(project_path: impl AsRef<Path>) -> io::Result<Self> {
        let path = project_path.as_ref().join("engine_config.json");
        if !path.exists() {
            AssetTools::write_json(
                &path,
                &json!({
                    "engine_name": "MiniForge",
                    "engine_alt_name": "Mini Forte",
                    "engine_version": ENGINE_VERSION,
                    "project_name": project_path.as_ref().file_name().and_then(|v| v.to_str()).unwrap_or("Project"),
                    "start_scene": "main.scene",
                    "autosave": true,
                    "autosave_interval_seconds": 60,
                    "safe_mode": true,
                }),
            )?;
        }
        let data = AssetTools::read_json(&path)?;
        Ok(Self { path, data })
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) -> io::Result<()> {
        if let Some(map) = self.data.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        AssetTools::write_json(&self.path, &self.data)
    }
}
