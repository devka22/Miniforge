use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::build_settings::BuildSettings;

#[derive(Debug, Clone)]
pub struct BuildProfiles {
    pub path: PathBuf,
    pub active: String,
    pub profiles: BTreeMap<String, Value>,
}

impl BuildProfiles {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut profiles = Self {
            path,
            active: "Development".to_string(),
            profiles: BTreeMap::from([
                (
                    "Development".to_string(),
                    json!({"debug_mode": true, "target_fps": 60}),
                ),
                (
                    "Release".to_string(),
                    json!({"debug_mode": false, "target_fps": 60}),
                ),
                (
                    "Web".to_string(),
                    json!({"debug_mode": false, "target_fps": 30}),
                ),
            ]),
        };
        profiles.load()?;
        Ok(profiles)
    }

    pub fn load(&mut self) -> io::Result<()> {
        if self.path.exists() {
            let value = AssetTools::read_json(&self.path)?;
            self.active = value
                .get("active")
                .and_then(Value::as_str)
                .unwrap_or(&self.active)
                .to_string();
            if let Some(items) = value.get("profiles").and_then(Value::as_object) {
                self.profiles = items.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            }
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        AssetTools::write_json(
            &self.path,
            &json!({"active": self.active, "profiles": self.profiles}),
        )
    }

    pub fn cycle(&mut self) -> io::Result<String> {
        let names: Vec<String> = self.profiles.keys().cloned().collect();
        let index = names
            .iter()
            .position(|name| name == &self.active)
            .unwrap_or(0);
        self.active = names[(index + 1) % names.len()].clone();
        self.save()?;
        Ok(self.active.clone())
    }

    pub fn apply_to(&self, settings: &mut BuildSettings) -> io::Result<()> {
        if let Some(Value::Object(profile)) = self.profiles.get(&self.active) {
            for (key, value) in profile {
                settings.set(key, value.clone())?;
            }
        }
        Ok(())
    }
}
