use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::version::ENGINE_VERSION;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub path: PathBuf,
    pub data: Value,
    pub status: EngineConfigStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineConfigStatus {
    pub used_defaults: bool,
    pub recovered_from_backup: bool,
    pub migrated: bool,
    pub backup_path: Option<PathBuf>,
    pub corrupt_backup_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}

impl EngineConfig {
    pub fn new(project_path: impl AsRef<Path>) -> io::Result<Self> {
        let path = project_path.as_ref().join("engine_config.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut status = EngineConfigStatus::default();
        let defaults = default_config(project_path.as_ref());
        let mut data = if path.exists() {
            match AssetTools::read_json(&path) {
                Ok(data) => data,
                Err(error) => {
                    status.warnings.push(format!(
                        "engine_config.json corrupto; se intentará recuperar: {error}"
                    ));
                    let corrupt_path = path.with_file_name("engine_config.json.corrupt");
                    if fs::copy(&path, &corrupt_path).is_ok() {
                        status.corrupt_backup_path = Some(corrupt_path);
                    }
                    let backup_path = backup_path_for(&path);
                    match AssetTools::read_json(&backup_path) {
                        Ok(data) => {
                            status.recovered_from_backup = true;
                            status.backup_path = Some(backup_path);
                            data
                        }
                        Err(_) => {
                            status.used_defaults = true;
                            status.warnings.push(
                                "No se pudo usar engine_config.json.bak; aplicando defaults seguros."
                                    .to_string(),
                            );
                            defaults.clone()
                        }
                    }
                }
            }
        } else {
            status.used_defaults = true;
            defaults.clone()
        };

        if migrate_config(&mut data, &defaults, &mut status) {
            status.migrated = true;
        }
        if status.used_defaults || status.recovered_from_backup || status.migrated || !path.exists()
        {
            if status.used_defaults || status.recovered_from_backup {
                AssetTools::write_json(&path, &data)?;
            } else {
                write_with_backup(&path, &data)?;
            }
        }
        Ok(Self { path, data, status })
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) -> io::Result<()> {
        if let Some(map) = self.data.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        self.save()
    }

    pub fn save(&self) -> io::Result<()> {
        write_with_backup(&self.path, &self.data)
    }

    pub fn backup_path(&self) -> PathBuf {
        backup_path_for(&self.path)
    }

    pub fn recover_from_backup(&mut self) -> io::Result<bool> {
        let backup = self.backup_path();
        if !backup.exists() {
            return Ok(false);
        }
        self.data = AssetTools::read_json(&backup)?;
        self.status.recovered_from_backup = true;
        self.status.backup_path = Some(backup);
        self.save()?;
        Ok(true)
    }
}

fn default_config(project_path: &Path) -> Value {
    json!({
        "config_version": 2,
        "engine_name": "MiniForge",
        "engine_alt_name": "MiniForge",
        "engine_version": ENGINE_VERSION,
        "project_name": project_path.file_name().and_then(|v| v.to_str()).unwrap_or("Project"),
        "start_scene": "main.scene",
        "autosave": true,
        "autosave_interval_seconds": 60,
        "safe_mode": true,
        "recover_corrupt_config": true,
        "logs": {
            "level": "info",
            "file": "logs/miniforge.log",
            "engine": "logs/engine.log",
            "error": "logs/error.log"
        },
        "editor": {
            "open_created_assets": true,
            "script_hot_reload": true,
            "fallback_assets": true
        }
    })
}

fn migrate_config(data: &mut Value, defaults: &Value, status: &mut EngineConfigStatus) -> bool {
    if !data.is_object() {
        *data = defaults.clone();
        status.used_defaults = true;
        return true;
    }
    let mut migrated = false;
    if merge_missing(data, defaults) {
        migrated = true;
    }
    let Some(map) = data.as_object_mut() else {
        return true;
    };
    if map
        .get("engine_alt_name")
        .and_then(Value::as_str)
        .is_some_and(|name| name != "MiniForge")
    {
        map.insert("engine_alt_name".to_string(), json!("MiniForge"));
        status
            .warnings
            .push("engine_alt_name migrado al nombre oficial MiniForge.".to_string());
        migrated = true;
    }
    if map.get("engine_name").and_then(Value::as_str) != Some("MiniForge") {
        map.insert("engine_name".to_string(), json!("MiniForge"));
        migrated = true;
    }
    if map
        .get("config_version")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < 2
    {
        map.insert("config_version".to_string(), json!(2));
        migrated = true;
    }
    map.insert("engine_version".to_string(), json!(ENGINE_VERSION));
    migrated
}

fn write_with_backup(path: &Path, data: &Value) -> io::Result<()> {
    if path.exists() {
        let backup = backup_path_for(path);
        let _ = fs::copy(path, backup);
    }
    AssetTools::write_json(path, data)
}

fn backup_path_for(path: &Path) -> PathBuf {
    path.with_file_name("engine_config.json.bak")
}

fn merge_missing(data: &mut Value, defaults: &Value) -> bool {
    let (Some(map), Some(default_map)) = (data.as_object_mut(), defaults.as_object()) else {
        return false;
    };
    let mut changed = false;
    for (key, value) in default_map {
        if let Some(existing) = map.get_mut(key) {
            if existing.is_object() && value.is_object() && merge_missing(existing, value) {
                changed = true;
            }
        } else {
            map.insert(key.clone(), value.clone());
            changed = true;
        }
    }
    changed
}
