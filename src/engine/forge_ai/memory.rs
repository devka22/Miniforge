use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::{AiError, AiErrorKind, AiResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiProjectMemory {
    pub schema_version: u32,
    pub project_summary: String,
    pub naming_conventions: Vec<String>,
    pub approved_decisions: Vec<AiDecisionRecord>,
    pub known_systems: Vec<String>,
    pub developer_preferences: Vec<String>,
    pub fixed_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiDecisionRecord {
    pub id: String,
    pub summary: String,
    pub approved_by_user: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiMemoryStore {
    pub root: PathBuf,
}

impl AiMemoryStore {
    pub fn for_project(project_path: impl AsRef<Path>) -> Self {
        Self {
            root: project_path.as_ref().join(".miniforge").join("ai"),
        }
    }

    pub fn load(&self) -> AiResult<AiProjectMemory> {
        let path = self.root.join("project_summary.json");
        if !path.exists() {
            return Ok(AiProjectMemory {
                schema_version: 1,
                ..AiProjectMemory::default()
            });
        }
        let source = fs::read_to_string(&path)?;
        serde_json::from_str(&source).map_err(Into::into)
    }

    pub fn save(&self, memory: &AiProjectMemory) -> AiResult<()> {
        if self.root.starts_with(std::env::temp_dir()) || self.root.components().count() > 2 {
            fs::create_dir_all(&self.root)?;
            let data = serde_json::to_string_pretty(memory)?;
            fs::write(self.root.join("project_summary.json"), data)?;
            return Ok(());
        }
        Err(AiError::new(
            AiErrorKind::Permission,
            "refusing to write AI memory outside a project .miniforge/ai folder",
        ))
    }

    pub fn conversations_dir(&self) -> PathBuf {
        self.root.join("conversations")
    }

    pub fn task_history_dir(&self) -> PathBuf {
        self.root.join("task_history")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.root.join("indexes")
    }
}
