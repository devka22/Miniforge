use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ProjectBrowser {
    pub recent_projects: Vec<PathBuf>,
}

impl ProjectBrowser {
    pub fn scan(root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
        let mut projects = Vec::new();
        if root.as_ref().exists() {
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                if path.join("project.json").exists() {
                    projects.push(path);
                }
            }
        }
        Ok(projects)
    }
}
