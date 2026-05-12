use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::engine::asset_tools::AssetTools;
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
            "version": "0.6.0",
            "scene_name": "autosave",
            "entities": entities.iter_mut().map(GameObject::serialize).collect::<Vec<_>>(),
        });
        AssetTools::write_json(&self.path, &data)?;
        self.last_save = Instant::now();
        Ok(())
    }
}
