use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct TagsLayersManager {
    pub settings_path: PathBuf,
    pub tags: Vec<String>,
    pub layers: Vec<String>,
}

impl TagsLayersManager {
    pub fn new(settings_path: impl AsRef<Path>) -> io::Result<Self> {
        let settings_path = settings_path.as_ref().to_path_buf();
        let mut manager = Self {
            settings_path,
            tags: vec![
                "Untagged".to_string(),
                "Player".to_string(),
                "Enemy".to_string(),
                "Resource".to_string(),
                "Building".to_string(),
                "Projectile".to_string(),
                "Neutral".to_string(),
            ],
            layers: vec![
                "Default".to_string(),
                "Ground".to_string(),
                "Units".to_string(),
                "Buildings".to_string(),
                "UI".to_string(),
                "Effects".to_string(),
                "IgnoreSelection".to_string(),
                "EditorOnly".to_string(),
            ],
        };
        manager.load()?;
        Ok(manager)
    }

    fn read_items(path: &Path, defaults: &[String]) -> io::Result<Vec<String>> {
        if !path.exists() {
            AssetTools::write_json(path, &json!({"items": defaults}))?;
            return Ok(defaults.to_vec());
        }
        let value = AssetTools::read_json(path)?;
        Ok(value
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_else(|| defaults.to_vec()))
    }

    pub fn load(&mut self) -> io::Result<()> {
        self.tags = Self::read_items(&self.settings_path.join("tags.json"), &self.tags)?;
        self.layers = Self::read_items(&self.settings_path.join("layers.json"), &self.layers)?;
        Ok(())
    }

    pub fn add_tag(&mut self, name: &str) -> io::Result<()> {
        if !self.tags.iter().any(|tag| tag == name) {
            self.tags.push(name.to_string());
        }
        AssetTools::write_json(
            self.settings_path.join("tags.json"),
            &json!({"items": self.tags}),
        )
    }

    pub fn add_layer(&mut self, name: &str) -> io::Result<()> {
        if !self.layers.iter().any(|layer| layer == name) {
            self.layers.push(name.to_string());
        }
        AssetTools::write_json(
            self.settings_path.join("layers.json"),
            &json!({"items": self.layers}),
        )
    }
}
