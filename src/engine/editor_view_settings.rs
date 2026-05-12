use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct EditorViewSettings {
    pub path: PathBuf,
    pub data: BTreeMap<String, Value>,
}

impl EditorViewSettings {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let defaults = BTreeMap::from([
            ("show_grid".to_string(), json!(true)),
            ("show_chunks".to_string(), json!(false)),
            ("show_coordinates".to_string(), json!(false)),
            ("show_brush_preview".to_string(), json!(true)),
        ]);
        let mut settings = Self {
            path,
            data: defaults,
        };
        settings.load()?;
        Ok(settings)
    }

    pub fn load(&mut self) -> io::Result<()> {
        if self.path.exists() {
            if let Ok(Value::Object(map)) = AssetTools::read_json(&self.path) {
                for (key, value) in map {
                    self.data.insert(key, value);
                }
            }
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        let mut map = serde_json::Map::new();
        for (key, value) in &self.data {
            map.insert(key.clone(), value.clone());
        }
        AssetTools::write_json(&self.path, &Value::Object(map))
    }

    pub fn set(&mut self, key: &str, value: Value) -> io::Result<()> {
        self.data.insert(key.to_string(), value);
        self.save()
    }

    pub fn toggle(&mut self, key: &str) -> io::Result<bool> {
        let next = !self.data.get(key).and_then(Value::as_bool).unwrap_or(false);
        self.set(key, json!(next))?;
        Ok(next)
    }
}
