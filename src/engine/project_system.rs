use std::env;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::{AssetTools, ProjectPaths};

#[derive(Debug, Clone)]
pub struct ProjectSystem {
    pub project_path: PathBuf,
    pub paths: ProjectPaths,
}

impl ProjectSystem {
    pub fn new() -> Self {
        let project_path = AssetTools::default_project_path();
        let paths = AssetTools::get_project_paths(&project_path);
        Self {
            project_path,
            paths,
        }
    }

    pub fn open_project(&mut self, project_path: impl AsRef<Path>) -> io::Result<()> {
        self.project_path = project_path.as_ref().to_path_buf();
        self.paths = AssetTools::ensure_project_folders(&self.project_path)?;
        Ok(())
    }

    pub fn apply_project_as_working_directory(&self) -> io::Result<()> {
        env::set_current_dir(&self.project_path)
    }
}

impl Default for ProjectSystem {
    fn default() -> Self {
        Self::new()
    }
}
