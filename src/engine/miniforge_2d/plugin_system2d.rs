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
    #[serde(default)]
    pub extension_points: Vec<PluginExtensionPoint2D>,
    #[serde(default)]
    pub canvas_input_forwarding: bool,
    #[serde(default)]
    pub canvas_overlay_forwarding: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginExtensionSlot2D {
    Toolbar,
    CanvasMenu,
    CanvasSideLeft,
    CanvasSideRight,
    CanvasBottom,
    InspectorBottom,
    ProjectSettingsLeft,
    ProjectSettingsRight,
    BottomPanel,
    AssetImporter,
    SceneOverlay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginExtensionPoint2D {
    pub id: String,
    pub slot: PluginExtensionSlot2D,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub icon: String,
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
            extension_points: vec![
                PluginExtensionPoint2D::new(
                    "rts_toolbar",
                    PluginExtensionSlot2D::Toolbar,
                    "RTS Tools",
                    "rts.open_tools",
                    "mouse-pointer-2",
                ),
                PluginExtensionPoint2D::new(
                    "rts_overlay",
                    PluginExtensionSlot2D::SceneOverlay,
                    "RTS Overlay",
                    "rts.draw_overlay",
                    "layers",
                ),
                PluginExtensionPoint2D::new(
                    "rts_bottom_panel",
                    PluginExtensionSlot2D::BottomPanel,
                    "RTS Debug",
                    "rts.open_debug_panel",
                    "terminal",
                ),
            ],
            canvas_input_forwarding: true,
            canvas_overlay_forwarding: true,
        }
    }

    pub fn extension_points_for(
        &self,
        slot: PluginExtensionSlot2D,
    ) -> Vec<&PluginExtensionPoint2D> {
        self.extension_points
            .iter()
            .filter(|extension| extension.slot == slot)
            .collect()
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
        let mut ids = std::collections::BTreeSet::new();
        for extension in &self.extension_points {
            if extension.id.trim().is_empty() {
                report.error(
                    "plugin_extension_id",
                    "plugin.json",
                    "Extension point sin id.",
                );
            } else if !ids.insert(extension.id.clone()) {
                report.error(
                    "plugin_extension_duplicate",
                    &extension.id,
                    format!("Extension point duplicado: {}", extension.id),
                );
            }
            if extension.label.trim().is_empty() {
                report.warning(
                    "plugin_extension_label",
                    &extension.id,
                    "Extension point sin label visible.",
                );
            }
            if extension.command.trim().is_empty() {
                report.error(
                    "plugin_extension_command",
                    &extension.id,
                    "Extension point sin command.",
                );
            }
        }
        report
    }
}

impl PluginExtensionPoint2D {
    pub fn new(
        id: impl Into<String>,
        slot: PluginExtensionSlot2D,
        label: impl Into<String>,
        command: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            slot,
            label: label.into(),
            command: command.into(),
            icon: icon.into(),
        }
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
