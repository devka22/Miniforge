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
    pub interval: Duration,
    pub last_save: Instant,
}

impl AutosaveManager {
    pub fn new(project_path: impl AsRef<Path>, interval_seconds: u64) -> Self {
        Self {
            path: project_path
                .as_ref()
                .join("saves")
                .join("autosave")
                .join("autosave.scene"),
            interval: Duration::from_secs(interval_seconds),
            last_save: Instant::now(),
        }
    }

    pub fn autosave_exists(&self) -> bool {
        self.path.exists()
    }

    pub fn save(&mut self, entities: &mut [GameObject]) -> io::Result<()> {
        let data = serde_json::json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "scene_name": "autosave",
            "entities": entities.iter_mut().map(GameObject::serialize).collect::<Vec<_>>(),
        });
        write_json_atomic(&self.path, &data)?;
        self.last_save = Instant::now();
        Ok(())
    }

    pub fn recover_entities(&self) -> io::Result<Vec<GameObject>> {
        let raw = AssetTools::read_json(&self.path)?;
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
}
