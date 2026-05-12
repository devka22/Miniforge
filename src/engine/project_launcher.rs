use std::path::PathBuf;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Default)]
pub struct ProjectLauncher;

impl ProjectLauncher {
    pub fn run(&self) -> PathBuf {
        AssetTools::default_project_path()
    }
}
