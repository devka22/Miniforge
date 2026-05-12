use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub struct PrefabManager {
    pub prefabs_path: PathBuf,
}

impl PrefabManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            prefabs_path: AssetTools::get_project_paths(project_path).prefabs,
        }
    }

    pub fn safe_filename(name: &str) -> String {
        let clean = name
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            .collect::<String>();
        let clean = clean.trim().to_lowercase();
        if clean.is_empty() {
            "unit".to_string()
        } else {
            clean
        }
    }

    pub fn save_prefab(
        &self,
        entity: &mut GameObject,
        filename: Option<&str>,
    ) -> io::Result<PathBuf> {
        let mut filename = filename
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{}.prefab", Self::safe_filename(&entity.name)));
        if !filename.ends_with(".prefab") {
            filename.push_str(".prefab");
        }
        let path = self.prefabs_path.join(filename);
        AssetTools::write_json(
            &path,
            &json!({
                "version": crate::engine::version::ENGINE_VERSION,
                "prefab_name": path.file_name().and_then(|value| value.to_str()).unwrap_or("prefab"),
                "entity": entity.serialize(),
            }),
        )?;
        Ok(path)
    }

    pub fn entity_from_data(entity_data: &Value, preserve_id: bool) -> Option<GameObject> {
        if entity_data.is_null() {
            return None;
        }
        let mut entity = GameObject::from_data(entity_data, preserve_id);
        if !preserve_id {
            let source_name = entity.name.clone();
            entity.name = format!("{source_name}_Instance");
        }
        Some(entity)
    }

    pub fn load_prefab(&self, path: impl AsRef<Path>) -> io::Result<Option<GameObject>> {
        if !path.as_ref().exists() {
            return Ok(None);
        }
        let data = AssetTools::read_json(&path)?;
        let Some(entity_data) = data.get("entity") else {
            return Ok(None);
        };
        let mut entity = Self::entity_from_data(entity_data, false);
        if let Some(entity) = &mut entity {
            entity.prefab_source = Some(path.as_ref().to_string_lossy().to_string());
            entity.is_prefab_instance = true;
        }
        Ok(entity)
    }

    pub fn instantiate_prefab(
        &self,
        entities: &mut Vec<GameObject>,
        path: impl AsRef<Path>,
        x: f64,
        y: f64,
    ) -> io::Result<Option<u64>> {
        let Some(mut entity) = self.load_prefab(path)? else {
            return Ok(None);
        };
        entity.x = x;
        entity.y = y;
        entity.sync_to_components();
        let id = entity.id;
        entities.push(entity);
        Ok(Some(id))
    }
}
