use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct LayoutManager {
    pub path: PathBuf,
    pub data: Value,
}

impl LayoutManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            path: project_path
                .as_ref()
                .join("project")
                .join("editor_layout.json"),
            data: json!({"panels": {}}),
        }
    }

    pub fn load_layout(&mut self) -> io::Result<()> {
        if self.path.exists() {
            self.data = AssetTools::read_json(&self.path)?;
        }
        Ok(())
    }

    pub fn save_layout(&self) -> io::Result<()> {
        AssetTools::write_json(&self.path, &self.data)
    }

    pub fn reset_layout(&mut self) {
        self.data = json!({"panels": {}});
    }
}
