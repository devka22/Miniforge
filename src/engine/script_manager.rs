use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Default)]
pub struct ScriptManager {
    /// Stable, project-relative script keys such as `player.luau` or
    /// `ai/enemy/player.luau`. Keeping the extension prevents Luau and visual
    /// graph assets with the same stem from overwriting one another.
    pub script_paths: BTreeMap<String, PathBuf>,
    pub mtimes: BTreeMap<String, SystemTime>,
    scripts_root: Option<PathBuf>,
}

impl ScriptManager {
    pub fn scan_scripts(&mut self, project_path: impl AsRef<Path>) -> io::Result<usize> {
        let root = project_path.as_ref().join("scripts");
        let (script_paths, mtimes) = scan_root(&root)?;
        self.script_paths = script_paths;
        self.mtimes = mtimes;
        self.scripts_root = Some(root);
        Ok(self.script_paths.len())
    }

    /// Resolves either a stable relative key or a unique legacy file stem.
    /// Ambiguous stems intentionally return `None` instead of choosing an
    /// arbitrary script.
    pub fn resolve(&self, key_or_stem: &str) -> Option<&Path> {
        if let Some(path) = self.script_paths.get(key_or_stem) {
            return Some(path.as_path());
        }
        let mut matches = self.script_paths.values().filter(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|stem| stem == key_or_stem)
        });
        let path = matches.next()?;
        matches.next().is_none().then_some(path.as_path())
    }

    /// Refreshes the registry and returns every script that was added,
    /// removed, moved, or modified since the previous snapshot.
    pub fn reload_changed_scripts(&mut self) -> Vec<String> {
        let Some(root) = self.scripts_root.clone() else {
            return Vec::new();
        };
        let Ok((next_paths, next_mtimes)) = scan_root(&root) else {
            return Vec::new();
        };
        let keys = self
            .script_paths
            .keys()
            .chain(next_paths.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let changed = keys
            .into_iter()
            .filter(|key| {
                self.script_paths.get(key) != next_paths.get(key)
                    || self.mtimes.get(key) != next_mtimes.get(key)
            })
            .collect::<Vec<_>>();
        self.script_paths = next_paths;
        self.mtimes = next_mtimes;
        changed
    }

    /// Compatibility helper for callers that only need a dirty flag. The
    /// snapshot is advanced when a change is observed, so one edit produces
    /// exactly one `true` result instead of retriggering forever.
    pub fn reload_if_changed(&mut self) -> bool {
        !self.reload_changed_scripts().is_empty()
    }
}

fn scan_root(root: &Path) -> io::Result<(BTreeMap<String, PathBuf>, BTreeMap<String, SystemTime>)> {
    let mut script_paths = BTreeMap::new();
    let mut mtimes = BTreeMap::new();
    if !root.exists() {
        return Ok((script_paths, mtimes));
    }
    for path in walk_files(root)? {
        if !is_script_path(&path) {
            continue;
        }
        let key = script_key(root, &path);
        if let Ok(modified) = fs::metadata(&path).and_then(|metadata| metadata.modified()) {
            mtimes.insert(key.clone(), modified);
        }
        script_paths.insert(key, path);
    }
    Ok((script_paths, mtimes))
}

fn is_script_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mfgraph" | "luau" | "lua"
            )
        })
}

fn script_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut files = Vec::new();
    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            files.extend(walk_files(&path)?);
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn nested_scripts_with_the_same_stem_do_not_collide() {
        let root = temp_project("unique_keys");
        write(&root.join("scripts/player.luau"), "return {}");
        write(
            &root.join("scripts/enemies/player.luau"),
            "return { enemy = true }",
        );
        write(&root.join("scripts/visual_graphs/player.mfgraph"), "{}");
        let mut manager = ScriptManager::default();

        assert_eq!(manager.scan_scripts(&root).unwrap(), 3);
        assert!(manager.script_paths.contains_key("player.luau"));
        assert!(manager.script_paths.contains_key("enemies/player.luau"));
        assert!(
            manager
                .script_paths
                .contains_key("visual_graphs/player.mfgraph")
        );
        assert!(manager.resolve("player").is_none());
        assert!(manager.resolve("enemies/player.luau").is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn modified_script_triggers_reload_only_once() {
        let root = temp_project("mtime");
        write(&root.join("scripts/player.luau"), "return {}");
        let mut manager = ScriptManager::default();
        manager.scan_scripts(&root).unwrap();
        manager.mtimes.insert("player.luau".to_string(), UNIX_EPOCH);

        assert_eq!(manager.reload_changed_scripts(), vec!["player.luau"]);
        assert!(!manager.reload_if_changed());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn added_and_deleted_scripts_refresh_the_registry() {
        let root = temp_project("add_remove");
        let first = root.join("scripts/first.luau");
        let second = root.join("scripts/second.lua");
        write(&first, "return {}");
        let mut manager = ScriptManager::default();
        manager.scan_scripts(&root).unwrap();

        write(&second, "return {}");
        assert_eq!(manager.reload_changed_scripts(), vec!["second.lua"]);
        assert!(manager.script_paths.contains_key("second.lua"));

        fs::remove_file(first).unwrap();
        assert_eq!(manager.reload_changed_scripts(), vec!["first.luau"]);
        assert!(!manager.script_paths.contains_key("first.luau"));

        let _ = fs::remove_dir_all(root);
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn temp_project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("miniforge_script_manager_{name}_{stamp}"))
    }
}
