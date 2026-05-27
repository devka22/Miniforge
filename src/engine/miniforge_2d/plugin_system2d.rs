use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest2D {
    pub name: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub description: String,
    pub systems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginScaffold2D {
    pub root: PathBuf,
    pub manifest: PluginManifest2D,
    pub folders: Vec<PathBuf>,
}

impl PluginManifest2D {
    pub fn rts_tools_demo() -> Self {
        Self {
            name: "RTS Tools".to_string(),
            version: "1.0.0".to_string(),
            author: "MiniForge".to_string(),
            enabled: true,
            description: "Adds RTS creation tools.".to_string(),
            systems: vec![
                "AI".to_string(),
                "Navigation".to_string(),
                "Commands".to_string(),
            ],
        }
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if self.name.trim().is_empty() {
            report.error("plugin_name", "plugin.json", "Plugin sin name.");
        }
        if self.version.trim().is_empty() {
            report.error("plugin_version", "plugin.json", "Plugin sin version.");
        }
        if self.systems.is_empty() {
            report.warning(
                "plugin_systems",
                "plugin.json",
                "Plugin sin systems declarados.",
            );
        }
        report
    }
}

pub fn scaffold_plugin(
    project_root: impl AsRef<Path>,
    manifest: PluginManifest2D,
) -> io::Result<PluginScaffold2D> {
    let root = project_root
        .as_ref()
        .join("plugins")
        .join(manifest.name.replace(' ', "_"));
    let folders = ["docs", "assets", "scripts", "graphs"]
        .iter()
        .map(|folder| root.join(folder))
        .collect::<Vec<_>>();
    std::fs::create_dir_all(&root)?;
    for folder in &folders {
        std::fs::create_dir_all(folder)?;
    }
    AssetTools::write_json(
        root.join("plugin.json"),
        &serde_json::to_value(&manifest).unwrap(),
    )?;
    Ok(PluginScaffold2D {
        root,
        manifest,
        folders,
    })
}
