use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct ProjectManager {
    pub path: PathBuf,
    pub data: Value,
}

impl ProjectManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            path: project_path.as_ref().join("project.json"),
            data: json!({}),
        }
    }

    pub fn load_project(&mut self) -> io::Result<()> {
        if self.path.exists() {
            self.data = AssetTools::read_json(&self.path)?;
        }
        Ok(())
    }

    pub fn save_project(&self) -> io::Result<()> {
        AssetTools::write_json(&self.path, &self.data)
    }
}
