use serde::{Deserialize, Serialize};

use crate::engine::miniforge_2d::ui_framework::{UiCanvas2D, UiWidget2D, minimal_ui_canvas};
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiDesignerTool2D {
    Select,
    Move,
    Scale,
    AddWidget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiDesigner2D {
    pub document_path: String,
    pub canvas: UiCanvas2D,
    pub selected_widget: Option<String>,
    pub active_tool: UiDesignerTool2D,
    pub snap: bool,
    pub guides: bool,
    pub preview_resolution: (u32, u32),
    pub simulated_hover: Option<String>,
    pub simulated_click: Option<String>,
}

impl Default for UiDesigner2D {
    fn default() -> Self {
        Self {
            document_path: "assets/ui/hud.mfui".to_string(),
            canvas: minimal_ui_canvas(),
            selected_widget: None,
            active_tool: UiDesignerTool2D::Select,
            snap: true,
            guides: true,
            preview_resolution: (1280, 720),
            simulated_hover: None,
            simulated_click: None,
        }
    }
}

impl UiDesigner2D {
    pub fn select(&mut self, widget_id: &str) -> bool {
        if self.canvas.find_widget(widget_id).is_some() {
            self.selected_widget = Some(widget_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn add_widget_to_root(&mut self, widget: UiWidget2D) {
        if let Some(root) = self.canvas.widgets.first_mut() {
            root.children.push(widget);
        } else {
            self.canvas.widgets.push(widget);
        }
    }

    pub fn binding_candidates(&self) -> Vec<String> {
        vec![
            "player.health_percent".to_string(),
            "player.mana_percent".to_string(),
            "player.coins".to_string(),
            "quest.active_title".to_string(),
            "rts.selected_count".to_string(),
        ]
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if !self.canvas.validate_widget_ids() {
            report.error(
                "duplicate_widget_id",
                self.document_path.clone(),
                "UI document tiene widget IDs duplicados.",
            );
        }
        for widget in self.canvas.flatten_widgets() {
            if widget.widget_type.trim().is_empty() {
                report.error("widget_type_empty", widget.id.clone(), "Widget sin tipo.");
            }
        }
        report
    }
}
