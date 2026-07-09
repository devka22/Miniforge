use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::engine::asset_tools::AssetTools;
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutosaveDomain {
    Scenes,
    Scripts,
    Graphs,
    Ui,
    Layouts,
    Configs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub domain: AutosaveDomain,
    pub path: PathBuf,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDecision {
    Restore,
    Ignore,
    Compare,
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
        let data = SceneSerializer::stamp(serde_json::json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "scene_name": "autosave",
            "entities": entities.iter_mut().map(GameObject::serialize).collect::<Vec<_>>(),
        }))
        .map_err(io::Error::from)?;
        match ProjectStorage::write_json_atomic_with_backup(
            &self.path,
            &data,
            BackupPolicy::new(&self.backup_path, DEFAULT_BACKUP_GENERATIONS),
        ) {
            Ok(_) => {
                self.last_save = Instant::now();
                self.last_error = None;
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(io::Error::from(error))
            }
        }
    }

    pub fn recover_entities(&self) -> io::Result<Vec<GameObject>> {
        let raw = AssetTools::read_json(&self.path)
            .or_else(|_| AssetTools::read_json(&self.backup_path))?;
        let data = SceneSerializer::try_migrate(raw)
            .map_err(io::Error::from)?
            .data;
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

    pub fn autosave_root(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("autosave"))
    }

    pub fn domain_path(&self, domain: AutosaveDomain, name: &str) -> PathBuf {
        self.autosave_root()
            .join(domain.folder())
            .join(sanitize_recovery_name(name))
    }

    pub fn save_text(
        &mut self,
        domain: AutosaveDomain,
        name: &str,
        text: &str,
    ) -> io::Result<PathBuf> {
        let path = self.domain_path(domain, name);
        let backup = path.with_extension("bak");
        match ProjectStorage::write_atomic_with_backup(
            &path,
            text.as_bytes(),
            BackupPolicy::new(backup, DEFAULT_BACKUP_GENERATIONS),
        ) {
            Ok(_) => {
                self.last_save = Instant::now();
                self.last_error = None;
                Ok(path)
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(io::Error::from(error))
            }
        }
    }

    pub fn recover_text(&self, domain: AutosaveDomain, name: &str) -> io::Result<String> {
        let path = self.domain_path(domain, name);
        std::fs::read_to_string(&path)
            .or_else(|_| std::fs::read_to_string(path.with_extension("bak")))
    }

    pub fn available_recoveries(&self) -> Vec<RecoveryEntry> {
        let mut entries = Vec::new();
        for domain in AutosaveDomain::ALL {
            let root = self.autosave_root().join(domain.folder());
            if !root.exists() {
                continue;
            }
            let Ok(read_dir) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().and_then(|value| value.to_str()) != Some("bak")
                {
                    let backup = path.with_extension("bak");
                    entries.push(RecoveryEntry {
                        domain,
                        path,
                        backup_path: backup.exists().then_some(backup),
                    });
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }
}

impl AutosaveDomain {
    pub const ALL: [AutosaveDomain; 6] = [
        AutosaveDomain::Scenes,
        AutosaveDomain::Scripts,
        AutosaveDomain::Graphs,
        AutosaveDomain::Ui,
        AutosaveDomain::Layouts,
        AutosaveDomain::Configs,
    ];

    pub fn folder(self) -> &'static str {
        match self {
            Self::Scenes => "scenes",
            Self::Scripts => "scripts",
            Self::Graphs => "graphs",
            Self::Ui => "ui",
            Self::Layouts => "layouts",
            Self::Configs => "configs",
        }
    }
}

fn sanitize_recovery_name(name: &str) -> String {
    let mut value = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.trim_matches('_').is_empty() {
        value = "document.txt".to_string();
    }
    value
}
