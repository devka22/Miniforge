use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ScriptManager {
    pub script_paths: BTreeMap<String, PathBuf>,
    pub mtimes: BTreeMap<String, std::time::SystemTime>,
}

impl ScriptManager {
    pub fn scan_scripts(&mut self, project_path: impl AsRef<Path>) -> io::Result<usize> {
        self.script_paths.clear();
        for root in [
            project_path.as_ref().join("scripts"),
            project_path.as_ref().join("systems"),
        ] {
            if !root.exists() {
                continue;
            }
            for path in walk_files(&root)? {
                if path.extension().and_then(|value| value.to_str()) != Some("py") {
                    continue;
                }
                let name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("script")
                    .to_string();
                if let Ok(meta) = fs::metadata(&path).and_then(|m| m.modified()) {
                    self.mtimes.insert(name.clone(), meta);
                }
                self.script_paths.insert(name, path);
            }
        }
        Ok(self.script_paths.len())
    }

    pub fn reload_if_changed(&mut self) -> bool {
        let mut changed = false;
        for (name, path) in &self.script_paths {
            if let Ok(mtime) = fs::metadata(path).and_then(|meta| meta.modified())
                && self.mtimes.get(name).is_some_and(|old| old < &mtime)
            {
                changed = true;
            }
        }
        changed
    }
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
