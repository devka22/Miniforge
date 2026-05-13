use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::engine::asset_tools::AssetTools;
use crate::engine::runtime_manifest_loader::write_json_atomic;
use crate::engine::scene_serializer::SceneSerializer;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub struct AutosaveManager {
    pub path: PathBuf,
    pub backup_path: PathBuf,
    pub interval: Duration,
    pub last_save: Instant,
    pub last_error: Option<String>,
}

impl AutosaveManager {
    pub fn new(project_path: impl AsRef<Path>, interval_seconds: u64) -> Self {
        Self {
            path: project_path
                .as_ref()
                .join("saves")
                .join("autosave")
                .join("autosave.scene"),
            backup_path: project_path
                .as_ref()
                .join("saves")
                .join("autosave")
                .join("autosave.scene.bak"),
            interval: Duration::from_secs(interval_seconds),
            last_save: Instant::now(),
            last_error: None,
        }
    }

    pub fn autosave_exists(&self) -> bool {
        self.path.exists()
    }

    pub fn save(&mut self, entities: &mut [GameObject]) -> io::Result<()> {
        if self.path.exists() {
            let _ = std::fs::copy(&self.path, &self.backup_path);
        }
        let data = serde_json::json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "scene_name": "autosave",
            "entities": entities.iter_mut().map(GameObject::serialize).collect::<Vec<_>>(),
        });
        match write_json_atomic(&self.path, &data) {
            Ok(()) => {
                self.last_save = Instant::now();
                self.last_error = None;
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    pub fn recover_entities(&self) -> io::Result<Vec<GameObject>> {
        let raw = AssetTools::read_json(&self.path)
            .or_else(|_| AssetTools::read_json(&self.backup_path))?;
        let data = SceneSerializer::migrate(raw);
        let entities = data
            .get("entities")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(|item| GameObject::from_data(item, true))
                    .collect()
            })
            .unwrap_or_default();
        Ok(entities)
    }

    pub fn should_save(&self) -> bool {
        self.last_save.elapsed() >= self.interval
    }

    pub fn health(&self) -> &'static str {
        if self.last_error.is_some() {
            "error"
        } else if self.autosave_exists() {
            "ready"
        } else {
            "empty"
        }
    }
}
