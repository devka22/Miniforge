use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Anchor2D {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UiRect2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCallback2D {
    pub event: String,
    pub graph: Option<String>,
    pub function: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiWidget2D {
    pub id: String,
    pub widget_type: String,
    pub rect: UiRect2D,
    pub anchors: Anchor2D,
    #[serde(default)]
    pub children: Vec<UiWidget2D>,
    #[serde(default)]
    pub callbacks: Vec<UiCallback2D>,
    #[serde(default)]
    pub properties: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCanvas2D {
    pub name: String,
    pub viewport_width: f32,
    pub viewport_height: f32,
    #[serde(default)]
    pub widgets: Vec<UiWidget2D>,
}

impl Anchor2D {
    pub const TOP_LEFT: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
    };
    pub const FILL: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 1.0,
    };
}

impl UiCanvas2D {
    pub fn validate_widget_ids(&self) -> bool {
        let mut ids = std::collections::BTreeSet::new();
        self.widgets
            .iter()
            .all(|widget| collect_widget_id(widget, &mut ids))
    }

    pub fn find_widget(&self, id: &str) -> Option<&UiWidget2D> {
        self.widgets
            .iter()
            .find_map(|widget| find_widget(widget, id))
    }

    pub fn flatten_widgets(&self) -> Vec<&UiWidget2D> {
        let mut widgets = Vec::new();
        for widget in &self.widgets {
            flatten_widget(widget, &mut widgets);
        }
        widgets
    }
}

pub fn supported_widget_types() -> Vec<&'static str> {
    vec![
        "Canvas",
        "Panel",
        "Text",
        "Button",
        "Image",
        "ProgressBar",
        "Slider",
        "Checkbox",
        "InputField",
        "VerticalBox",
        "HorizontalBox",
        "GridPanel",
        "ScrollBox",
        "Spacer",
        "Border",
        "IconButton",
        "MiniMap",
        "DialogueBox",
        "InventorySlot",
    ]
}

pub fn supported_ui_events() -> Vec<&'static str> {
    vec![
        "OnClick",
        "OnHover",
        "OnPressed",
        "OnReleased",
        "OnValueChanged",
        "OnTextChanged",
        "OnFocus",
        "OnUnfocus",
    ]
}

pub fn minimal_ui_canvas() -> UiCanvas2D {
    UiCanvas2D {
        name: "HUD_Main".to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        widgets: vec![UiWidget2D {
            id: "RootCanvas".to_string(),
            widget_type: "Canvas".to_string(),
            rect: UiRect2D {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            },
            anchors: Anchor2D::FILL,
            callbacks: Vec::new(),
            properties: json!({}),
            children: vec![
                UiWidget2D {
                    id: "HealthPanel".to_string(),
                    widget_type: "Panel".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 24.0,
                        width: 260.0,
                        height: 56.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    callbacks: Vec::new(),
                    properties: json!({"background": [24, 28, 34, 220]}),
                    children: vec![UiWidget2D {
                        id: "HealthBar".to_string(),
                        widget_type: "ProgressBar".to_string(),
                        rect: UiRect2D {
                            x: 16.0,
                            y: 16.0,
                            width: 228.0,
                            height: 20.0,
                        },
                        anchors: Anchor2D::TOP_LEFT,
                        children: Vec::new(),
                        callbacks: Vec::new(),
                        properties: json!({"value": 1.0, "fill": [220, 60, 72, 255]}),
                    }],
                },
                UiWidget2D {
                    id: "StartButton".to_string(),
                    widget_type: "Button".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 96.0,
                        width: 160.0,
                        height: 40.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    children: Vec::new(),
                    callbacks: vec![UiCallback2D {
                        event: "click".to_string(),
                        graph: Some("scripts/visual_graphs/StartGame.mfgraph".to_string()),
                        function: None,
                        payload: json!({"command": "start"}),
                    }],
                    properties: json!({"text": "Start"}),
                },
                UiWidget2D {
                    id: "CoinText".to_string(),
                    widget_type: "Text".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 148.0,
                        width: 160.0,
                        height: 32.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    children: Vec::new(),
                    callbacks: Vec::new(),
                    properties: json!({"text": "Coins: 0", "binding": "player.coins"}),
                },
            ],
        }],
    }
}

fn collect_widget_id(widget: &UiWidget2D, ids: &mut std::collections::BTreeSet<String>) -> bool {
    if !ids.insert(widget.id.clone()) {
        return false;
    }
    widget
        .children
        .iter()
        .all(|child| collect_widget_id(child, ids))
}

fn find_widget<'a>(widget: &'a UiWidget2D, id: &str) -> Option<&'a UiWidget2D> {
    if widget.id == id {
        return Some(widget);
    }
    widget
        .children
        .iter()
        .find_map(|child| find_widget(child, id))
}

fn flatten_widget<'a>(widget: &'a UiWidget2D, widgets: &mut Vec<&'a UiWidget2D>) {
    widgets.push(widget);
    for child in &widget.children {
        flatten_widget(child, widgets);
    }
}
