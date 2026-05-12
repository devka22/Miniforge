use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct ProjectSettings {
    pub path: PathBuf,
    pub data: Value,
}

impl ProjectSettings {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = if path.exists() {
            AssetTools::read_json(&path)?
        } else {
            json!({})
        };
        Ok(Self { path, data })
    }

    pub fn load(&mut self) -> io::Result<()> {
        if self.path.exists() {
            self.data = AssetTools::read_json(&self.path)?;
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        AssetTools::write_json(&self.path, &self.data)
    }

    pub fn get(&self, key: &str, default: Value) -> Value {
        self.data.get(key).cloned().unwrap_or(default)
    }

    pub fn set(&mut self, key: &str, value: Value) -> io::Result<()> {
        if let Some(map) = self.data.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        self.save()
    }
}
