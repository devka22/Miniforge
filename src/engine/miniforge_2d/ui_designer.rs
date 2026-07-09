use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::engine::miniforge_2d::ui_framework::{
    Anchor2D, UiAnimation2D, UiCanvas2D, UiRect2D, UiResolvedWidget2D, UiStyle2D, UiWidget2D,
    main_menu_canvas, minimal_ui_canvas, pause_menu_canvas, settings_menu_canvas,
};
use crate::engine::miniforge_2d::validation::ValidationReport2D;
use crate::engine::ui_advanced::{
    UiAccessibilityReport2D, UiAdvancedInterface2D, UiBindingContext2D, UiPreparedInterface2D,
};

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
    #[serde(default)]
    pub responsive_presets: Vec<(u32, u32)>,
    #[serde(default)]
    pub active_state: UiPreviewState2D,
    #[serde(default)]
    pub animation_timeline: Vec<UiAnimation2D>,
    #[serde(default)]
    pub palette: Vec<UiPaletteItem2D>,
    #[serde(default)]
    pub snap_grid: f32,
    #[serde(default)]
    pub show_safe_area: bool,
    #[serde(default)]
    pub show_hierarchy: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiPreviewState2D {
    #[default]
    Normal,
    Hovered,
    Pressed,
    Disabled,
    Focused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiPaletteItem2D {
    pub widget_type: String,
    pub category: String,
    pub icon: String,
    pub description: String,
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
            responsive_presets: vec![(1280, 720), (1920, 1080), (800, 600), (390, 844)],
            active_state: UiPreviewState2D::Normal,
            animation_timeline: minimal_ui_canvas().animations,
            palette: default_ui_palette(),
            snap_grid: 8.0,
            show_safe_area: true,
            show_hierarchy: true,
        }
    }
}

impl UiDesigner2D {
    pub fn main_menu(game_title: &str) -> Self {
        let canvas = main_menu_canvas(game_title);
        Self {
            document_path: "assets/ui/main_menu.mfui".to_string(),
            animation_timeline: canvas.animations.clone(),
            canvas,
            ..Default::default()
        }
    }

    pub fn pause_menu() -> Self {
        let canvas = pause_menu_canvas();
        Self {
            document_path: "assets/ui/pause_menu.mfui".to_string(),
            animation_timeline: canvas.animations.clone(),
            canvas,
            ..Default::default()
        }
    }

    pub fn settings_menu() -> Self {
        let canvas = settings_menu_canvas();
        Self {
            document_path: "assets/ui/settings_menu.mfui".to_string(),
            animation_timeline: canvas.animations.clone(),
            canvas,
            ..Default::default()
        }
    }

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

    pub fn add_widget_to_selected_or_root(&mut self, widget: UiWidget2D) -> bool {
        if let Some(parent_id) = self.selected_widget.clone()
            && let Some(parent) = find_widget_mut_in_canvas(&mut self.canvas, &parent_id)
        {
            parent.children.push(widget);
            return true;
        }
        self.add_widget_to_root(widget);
        true
    }

    pub fn move_selected(&mut self, dx: f32, dy: f32) -> bool {
        let Some(id) = self.selected_widget.clone() else {
            return false;
        };
        let Some(widget) = find_widget_mut_in_canvas(&mut self.canvas, &id) else {
            return false;
        };
        let step = if self.snap {
            self.snap_grid.max(1.0)
        } else {
            1.0
        };
        widget.rect.x = snap_value(widget.rect.x + dx, step);
        widget.rect.y = snap_value(widget.rect.y + dy, step);
        true
    }

    pub fn resize_selected(&mut self, width: f32, height: f32) -> bool {
        let Some(id) = self.selected_widget.clone() else {
            return false;
        };
        let Some(widget) = find_widget_mut_in_canvas(&mut self.canvas, &id) else {
            return false;
        };
        widget.rect.width = width.max(1.0);
        widget.rect.height = height.max(1.0);
        true
    }

    pub fn set_selected_property(&mut self, key: &str, value: Value) -> bool {
        let Some(id) = self.selected_widget.clone() else {
            return false;
        };
        let Some(widget) = find_widget_mut_in_canvas(&mut self.canvas, &id) else {
            return false;
        };
        if !widget.properties.is_object() {
            widget.properties = serde_json::json!({});
        }
        if let Some(map) = widget.properties.as_object_mut() {
            map.insert(key.to_string(), value);
            return true;
        }
        false
    }

    pub fn create_widget_from_palette(
        &mut self,
        widget_type: &str,
        id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        if !self
            .palette
            .iter()
            .any(|item| item.widget_type == widget_type)
        {
            return false;
        }
        let widget = UiWidget2D {
            id: id.to_string(),
            widget_type: widget_type.to_string(),
            rect: default_rect_for_widget(widget_type, x, y),
            anchors: Anchor2D::TOP_LEFT,
            children: Vec::new(),
            callbacks: Vec::new(),
            properties: default_properties_for_widget(widget_type),
            style: UiStyle2D {
                style_id: Some(default_style_for_widget(widget_type).to_string()),
                ..Default::default()
            },
            bindings: Vec::new(),
            navigation: Default::default(),
        };
        self.add_widget_to_selected_or_root(widget)
    }

    pub fn duplicate_selected(&mut self, new_id: &str) -> bool {
        let Some(selected) = self.selected_widget.clone() else {
            return false;
        };
        let Some(source) = self.canvas.find_widget(&selected).cloned() else {
            return false;
        };
        let mut clone = source;
        clone.id = new_id.to_string();
        clone.rect.x += self.snap_grid.max(8.0);
        clone.rect.y += self.snap_grid.max(8.0);
        self.add_widget_to_root(clone);
        self.selected_widget = Some(new_id.to_string());
        true
    }

    pub fn align_selected(&mut self, alignment: &str) -> bool {
        let Some(id) = self.selected_widget.clone() else {
            return false;
        };
        let viewport_width = self.canvas.viewport_width;
        let viewport_height = self.canvas.viewport_height;
        let Some(widget) = find_widget_mut_in_canvas(&mut self.canvas, &id) else {
            return false;
        };
        match alignment {
            "center_x" => widget.rect.x = (viewport_width - widget.rect.width) * 0.5,
            "center_y" => widget.rect.y = (viewport_height - widget.rect.height) * 0.5,
            "top" => widget.rect.y = 0.0,
            "bottom" => widget.rect.y = viewport_height - widget.rect.height,
            "left" => widget.rect.x = 0.0,
            "right" => widget.rect.x = viewport_width - widget.rect.width,
            _ => return false,
        }
        true
    }

    pub fn hierarchy_rows(&self) -> Vec<(String, String, usize)> {
        self.canvas
            .resolve_layout((self.canvas.viewport_width, self.canvas.viewport_height))
            .into_iter()
            .map(|widget| (widget.id, widget.widget_type, widget.depth))
            .collect()
    }

    pub fn palette_search(&self, query: &str) -> Vec<&UiPaletteItem2D> {
        let query = query.to_lowercase();
        self.palette
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.widget_type.to_lowercase().contains(&query)
                    || item.category.to_lowercase().contains(&query)
                    || item.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn preview_layout(&self) -> Vec<UiResolvedWidget2D> {
        self.canvas.resolve_layout((
            self.preview_resolution.0 as f32,
            self.preview_resolution.1 as f32,
        ))
    }

    pub fn advanced_preview(&self, bindings: UiBindingContext2D) -> UiPreparedInterface2D {
        let interface = UiAdvancedInterface2D {
            bindings,
            ..Default::default()
        };
        interface.prepare(
            &self.canvas,
            (
                self.preview_resolution.0 as f32,
                self.preview_resolution.1 as f32,
            ),
        )
    }

    pub fn accessibility_report(&self) -> UiAccessibilityReport2D {
        self.advanced_preview(UiBindingContext2D::default())
            .accessibility
    }

    pub fn select_at_preview_point(&mut self, x: f32, y: f32) -> Option<String> {
        let hit = self.preview_layout().into_iter().rev().find(|widget| {
            x >= widget.rect.x
                && y >= widget.rect.y
                && x <= widget.rect.x + widget.rect.width
                && y <= widget.rect.y + widget.rect.height
        })?;
        self.selected_widget = Some(hit.id.clone());
        Some(hit.id)
    }

    pub fn scene_editing_actions(&self) -> Vec<&'static str> {
        vec![
            "select_widget_in_scene",
            "move_widget_gizmo",
            "resize_widget_gizmo",
            "snap_to_safe_area",
            "bind_to_runtime_value",
            "preview_hover_pressed_disabled",
            "promote_to_screen_manager",
            "preview_responsive_breakpoint",
            "audit_accessibility",
            "preview_localized_bindings",
        ]
    }

    pub fn responsive_preview_count(&self) -> usize {
        self.responsive_presets.len()
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
            let stretch_x = widget.anchors.max_x > widget.anchors.min_x;
            let stretch_y = widget.anchors.max_y > widget.anchors.min_y;
            if (!stretch_x && widget.rect.width <= 0.0) || (!stretch_y && widget.rect.height <= 0.0)
            {
                report.error(
                    "widget_size_invalid",
                    widget.id.clone(),
                    "Widget con ancho/alto invalido.",
                );
            }
        }
        for issue in self.canvas.validate_navigation_links() {
            report.warning("navigation_link_missing", issue, "Navegacion UI rota.");
        }
        report
    }
}

pub fn default_ui_palette() -> Vec<UiPaletteItem2D> {
    [
        (
            "Panel",
            "Layout",
            "panel",
            "Container with background style.",
        ),
        (
            "SafeArea",
            "Layout",
            "safe",
            "Responsive safe area for menus.",
        ),
        (
            "VerticalBox",
            "Layout",
            "vbox",
            "Stacks child widgets vertically.",
        ),
        (
            "HorizontalBox",
            "Layout",
            "hbox",
            "Stacks child widgets horizontally.",
        ),
        (
            "GridPanel",
            "Layout",
            "grid",
            "Grid container for inventory and menus.",
        ),
        ("Text", "Common", "text", "Static or bound text label."),
        ("Label", "Common", "label", "Alias for quick HUD labels."),
        (
            "RichText",
            "Common",
            "rich",
            "Styled text for dialogue and logs.",
        ),
        ("Button", "Input", "button", "Clickable game UI button."),
        (
            "MenuButton",
            "Input",
            "menu",
            "Game menu button with navigation.",
        ),
        ("IconButton", "Input", "icon", "Compact icon action button."),
        ("Image", "Visual", "image", "Sprite or texture widget."),
        ("NineSlice", "Visual", "slice", "Resizable framed image."),
        (
            "ProgressBar",
            "Gameplay",
            "bar",
            "Health, mana or loading progress.",
        ),
        ("Slider", "Input", "slider", "Numeric option control."),
        ("Checkbox", "Input", "check", "Boolean option control."),
        ("Dropdown", "Input", "drop", "Option list control."),
        ("InputField", "Input", "input", "Text entry control."),
        (
            "TextInput",
            "Input",
            "text-input",
            "Alias for text entry control.",
        ),
        ("InventoryGrid", "Gameplay", "bag", "Inventory slot grid."),
        (
            "AbilityBar",
            "Gameplay",
            "spell",
            "Action bar for abilities.",
        ),
        (
            "DialogueBox",
            "Narrative",
            "dialogue",
            "Conversation text box.",
        ),
        ("Tooltip", "Feedback", "tip", "Contextual hover panel."),
        ("RadialMenu", "Input", "radial", "Radial command menu."),
    ]
    .into_iter()
    .map(
        |(widget_type, category, icon, description)| UiPaletteItem2D {
            widget_type: widget_type.to_string(),
            category: category.to_string(),
            icon: icon.to_string(),
            description: description.to_string(),
        },
    )
    .collect()
}

fn default_rect_for_widget(widget_type: &str, x: f32, y: f32) -> UiRect2D {
    let (width, height) = match widget_type {
        "Text" | "Label" | "RichText" => (220.0, 32.0),
        "Button" | "MenuButton" => (220.0, 44.0),
        "ProgressBar" | "Slider" => (240.0, 24.0),
        "Checkbox" => (180.0, 28.0),
        "InputField" | "TextInput" | "Dropdown" => (260.0, 36.0),
        "InventoryGrid" => (320.0, 240.0),
        "AbilityBar" => (360.0, 56.0),
        "DialogueBox" => (520.0, 140.0),
        "RadialMenu" => (220.0, 220.0),
        "SafeArea" | "Panel" | "MenuStack" => (320.0, 220.0),
        _ => (128.0, 64.0),
    };
    UiRect2D {
        x,
        y,
        width,
        height,
    }
}

fn default_properties_for_widget(widget_type: &str) -> Value {
    match widget_type {
        "Text" | "Label" | "RichText" => serde_json::json!({"text": "Text"}),
        "Button" | "MenuButton" => serde_json::json!({"text": "Button"}),
        "ProgressBar" => serde_json::json!({"value": 1.0, "max": 1.0}),
        "Slider" => serde_json::json!({"value": 0.5, "min": 0.0, "max": 1.0}),
        "Checkbox" => serde_json::json!({"checked": false, "text": "Option"}),
        "InputField" | "TextInput" => {
            serde_json::json!({"text": "", "placeholder": "Enter text"})
        }
        "Dropdown" => serde_json::json!({"selected": 0, "options": ["Option A", "Option B"]}),
        "InventoryGrid" => serde_json::json!({"columns": 5, "slot_size": 48}),
        "AbilityBar" => serde_json::json!({"slots": 6}),
        "DialogueBox" => serde_json::json!({"speaker": "", "text": ""}),
        _ => serde_json::json!({}),
    }
}

fn default_style_for_widget(widget_type: &str) -> &'static str {
    match widget_type {
        "Button" | "MenuButton" | "IconButton" => "button",
        "Panel" | "SafeArea" | "MenuStack" | "DialogueBox" | "Tooltip" => "panel",
        _ => "default",
    }
}

fn find_widget_mut_in_canvas<'a>(
    canvas: &'a mut UiCanvas2D,
    id: &str,
) -> Option<&'a mut UiWidget2D> {
    for widget in &mut canvas.widgets {
        if let Some(found) = find_widget_mut(widget, id) {
            return Some(found);
        }
    }
    None
}

fn find_widget_mut<'a>(widget: &'a mut UiWidget2D, id: &str) -> Option<&'a mut UiWidget2D> {
    if widget.id == id {
        return Some(widget);
    }
    for child in &mut widget.children {
        if let Some(found) = find_widget_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn snap_value(value: f32, step: f32) -> f32 {
    if step <= 1.0 {
        value
    } else {
        (value / step).round() * step
    }
}
