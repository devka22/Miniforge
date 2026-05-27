use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorLayout2D {
    pub theme: String,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub bottom_panel_height: f32,
    pub active_bottom_tab: String,
    pub scene_view_grid: bool,
    pub show_console: bool,
    pub show_problems: bool,
    pub panels: Vec<EditorPanel2D>,
    pub bottom_tabs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorPanel2D {
    pub id: String,
    pub title: String,
    pub region: String,
    pub visible: bool,
    pub dockable: bool,
    pub resizable: bool,
}

impl Default for EditorLayout2D {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            left_panel_width: 280.0,
            right_panel_width: 340.0,
            bottom_panel_height: 260.0,
            active_bottom_tab: "Content Browser".to_string(),
            scene_view_grid: true,
            show_console: true,
            show_problems: true,
            panels: vec![
                panel("menu_bar", "Menu Bar", "top", true, false, false),
                panel("toolbar", "Toolbar", "top", true, false, false),
                panel("world_outliner", "World Outliner", "left", true, true, true),
                panel("scene_view", "Scene View 2D", "center", true, false, true),
                panel("inspector", "Details", "right", true, true, true),
                panel(
                    "content_browser",
                    "Content Browser",
                    "bottom",
                    true,
                    true,
                    true,
                ),
                panel("console", "Console", "bottom", true, true, true),
                panel("problems", "Problems", "bottom", true, true, true),
                panel("output", "Output", "bottom", true, true, true),
            ],
            bottom_tabs: vec![
                "Content Browser".to_string(),
                "Console".to_string(),
                "Problems".to_string(),
                "Output".to_string(),
            ],
        }
    }
}

impl EditorLayout2D {
    pub fn load_or_default(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let layout = Self::default();
            layout.save(path)?;
            return Ok(layout);
        }
        let value = AssetTools::read_json(path)?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        AssetTools::write_json(path, &serde_json::to_value(self).unwrap_or_default())
    }

    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        for required in [
            "menu_bar",
            "toolbar",
            "world_outliner",
            "scene_view",
            "inspector",
        ] {
            if !self.panels.iter().any(|panel| panel.id == required) {
                report.error(
                    "layout_missing_panel",
                    required,
                    format!("Falta panel obligatorio de editor: {required}"),
                );
            }
        }
        if self.theme != "dark" {
            report.warning(
                "layout_theme",
                "theme",
                "La guia pide tema oscuro por defecto.",
            );
        }
        report
    }
}

fn panel(
    id: &str,
    title: &str,
    region: &str,
    visible: bool,
    dockable: bool,
    resizable: bool,
) -> EditorPanel2D {
    EditorPanel2D {
        id: id.to_string(),
        title: title.to_string(),
        region: region.to_string(),
        visible,
        dockable,
        resizable,
    }
}
