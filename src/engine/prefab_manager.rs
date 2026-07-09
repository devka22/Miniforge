use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_database::new_asset_guid;
use crate::engine::asset_tools::AssetTools;
use crate::engine::prefab_serializer::PrefabSerializer;
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
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
        let backup = path.with_extension("prefab.bak");
        let guid = if path.is_file() {
            AssetTools::read_json(&path)
                .ok()
                .and_then(|data| {
                    data.get("guid")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| new_asset_guid(&path.to_string_lossy()))
        } else {
            new_asset_guid(&path.to_string_lossy())
        };
        let data = PrefabSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "guid": guid,
            "prefab_name": path.file_name().and_then(|value| value.to_str()).unwrap_or("prefab"),
            "entity": entity.serialize(),
        }))
        .map_err(io::Error::from)?;
        ProjectStorage::write_json_atomic_with_backup(
            &path,
            &data,
            BackupPolicy::new(backup, DEFAULT_BACKUP_GENERATIONS),
        )
        .map_err(io::Error::from)?;
        entity.prefab_source = Some(path.to_string_lossy().to_string());
        entity.prefab_guid = Some(guid);
        entity.is_prefab_instance = true;
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
        let data = load_prefab_document_with_backup(path.as_ref())?;
        let prefab_guid = data
            .get("guid")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let Some(entity_data) = data.get("entity") else {
            return Ok(None);
        };
        let mut entity = Self::entity_from_data(entity_data, false);
        if let Some(entity) = &mut entity {
            entity.prefab_source = Some(path.as_ref().to_string_lossy().to_string());
            entity.prefab_guid = prefab_guid;
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

    pub fn validate_prefab_data(data: &Value) -> io::Result<()> {
        PrefabSerializer::try_migrate(data.clone())
            .map(|_| ())
            .map_err(io::Error::from)
    }
}

fn load_prefab_document_with_backup(path: &Path) -> io::Result<Value> {
    let primary_error = match AssetTools::read_json(path) {
        Ok(data) => match PrefabSerializer::try_migrate(data) {
            Ok(report) => return Ok(report.data),
            Err(error) if error.is_future_version() => return Err(io::Error::from(error)),
            Err(error) => error.to_string(),
        },
        Err(error) => error.to_string(),
    };
    let backup = path.with_extension("prefab.bak");
    if !backup.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Prefab invalido: {} | {primary_error}", path.display()),
        ));
    }
    let backup_data = AssetTools::read_json(&backup).map_err(|backup_error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Prefab invalido y backup ilegible: {} | {primary_error}; backup: {backup_error}",
                path.display()
            ),
        )
    })?;
    PrefabSerializer::try_migrate(backup_data)
        .map(|report| report.data)
        .map_err(|backup_error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Prefab invalido y backup incompatible: {} | {primary_error}; backup: {backup_error}",
                    path.display()
                ),
            )
        })
}
