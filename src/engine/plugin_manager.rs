use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub manifest: Value,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    pub project_path: PathBuf,
    pub plugins: Vec<PluginInfo>,
}

impl PluginManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            plugins: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> io::Result<usize> {
        self.plugins.clear();
        for root_name in ["plugins", "packages"] {
            let root = self.project_path.join(root_name);
            if !root.exists() {
                continue;
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let manifest_path = path.join("plugin.json");
                if !manifest_path.exists() {
                    continue;
                }
                let manifest = AssetTools::read_json(&manifest_path).unwrap_or(Value::Null);
                let enabled = manifest
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let name = manifest
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        path.file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("plugin")
                            .to_string()
                    });
                self.plugins.push(PluginInfo {
                    name,
                    path,
                    enabled,
                    manifest,
                });
            }
        }
        Ok(self.plugins.len())
    }

    pub fn emit_hook(&mut self, hook_name: &str) -> io::Result<usize> {
        self.scan()?;
        let mut count = 0;
        for plugin in &self.plugins {
            if !plugin.enabled {
                continue;
            }
            let plugin_py = plugin.path.join("plugin.py");
            if !plugin_py.exists() {
                continue;
            }
            let source = fs::read_to_string(plugin_py).unwrap_or_default();
            if source.contains(&format!("def {hook_name}(")) {
                count += 1;
            }
        }
        Ok(count)
    }
}
