//! Scene-level UI Canvas data model (serialized under `ui_canvases` in `.scene` files).
//! Runtime hit-testing for legacy `UIElement` components remains on `UICanvas::hit_test`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UiAnchor {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Default for UiAnchor {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiRect {
    pub anchor: UiAnchor,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for UiRect {
    fn default() -> Self {
        Self {
            anchor: UiAnchor::default(),
            pivot_x: 0.5,
            pivot_y: 0.5,
            offset_x: 0.0,
            offset_y: 0.0,
            width: 160.0,
            height: 48.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiCanvasElement {
    Panel {
        id: String,
        name: String,
        rect: UiRect,
        #[serde(default)]
        color: [u8; 4],
    },
    Button {
        id: String,
        label: String,
        rect: UiRect,
    },
    Label {
        id: String,
        text: String,
        rect: UiRect,
        #[serde(default = "default_font_size")]
        font_size: f32,
    },
    Image {
        id: String,
        #[serde(default)]
        sprite_path: String,
        rect: UiRect,
    },
}

fn default_font_size() -> f32 {
    18.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCanvasRoot {
    pub id: String,
    pub name: String,
    #[serde(default = "default_ref_w")]
    pub reference_width: f32,
    #[serde(default = "default_ref_h")]
    pub reference_height: f32,
    pub elements: Vec<UiCanvasElement>,
}

fn default_ref_w() -> f32 {
    1920.0
}

fn default_ref_h() -> f32 {
    1080.0
}

impl UiCanvasRoot {
    pub fn default_hud() -> Self {
        Self {
            id: "hud_main".into(),
            name: "HUD".into(),
            reference_width: 1920.0,
            reference_height: 1080.0,
            elements: vec![
                UiCanvasElement::Panel {
                    id: "panel_status".into(),
                    name: "Status".into(),
                    rect: UiRect {
                        anchor: UiAnchor {
                            min_x: 0.0,
                            min_y: 1.0,
                            max_x: 1.0,
                            max_y: 1.0,
                        },
                        pivot_x: 0.5,
                        pivot_y: 1.0,
                        offset_x: 0.0,
                        offset_y: -40.0,
                        width: 400.0,
                        height: 56.0,
                    },
                    color: [24, 28, 36, 220],
                },
                UiCanvasElement::Label {
                    id: "label_hint".into(),
                    text: "MiniForge UI Canvas".into(),
                    rect: UiRect {
                        anchor: UiAnchor {
                            min_x: 0.0,
                            min_y: 0.0,
                            max_x: 0.0,
                            max_y: 0.0,
                        },
                        pivot_x: 0.0,
                        pivot_y: 0.0,
                        offset_x: 24.0,
                        offset_y: 24.0,
                        width: 420.0,
                        height: 36.0,
                    },
                    font_size: 20.0,
                },
            ],
        }
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}

/// Resolve layout in reference space → pixel rect for responsive preview.
pub fn layout_element_pixels(
    root: &UiCanvasRoot,
    element_rect: &UiRect,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32, f32, f32) {
    let sx = viewport_w / root.reference_width.max(1.0);
    let sy = viewport_h / root.reference_height.max(1.0);
    let ax = element_rect.anchor.min_x * root.reference_width;
    let ay = element_rect.anchor.min_y * root.reference_height;
    let px = ax + element_rect.offset_x - element_rect.pivot_x * element_rect.width;
    let py = ay + element_rect.offset_y - element_rect.pivot_y * element_rect.height;
    (px * sx, py * sy, element_rect.width * sx, element_rect.height * sy)
}

pub fn ui_canvases_from_value(value: &Value) -> Vec<UiCanvasRoot> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(UiCanvasRoot::from_value)
                .collect()
        })
        .unwrap_or_default()
}

pub fn push_canvas(canvases: &mut Value, canvas: UiCanvasRoot) {
    if !canvases.is_array() {
        *canvases = json!([]);
    }
    if let Some(arr) = canvases.as_array_mut() {
        arr.push(canvas.to_value());
    }
}

#[derive(Debug, Clone, Default)]
pub struct UICanvas {
    pub elements: usize,
}

impl UICanvas {
    pub fn hit_test<'a>(
        &self,
        entities: &'a [GameObject],
        point: (f64, f64),
    ) -> Option<(&'a GameObject, &'a crate::engine::component::Component)> {
        let (px, py) = point;
        let mut hits = Vec::new();
        for entity in entities {
            if let Some(ui) = entity.get_component("UIElement") {
                let x = ui.get_f64("x", 0.0);
                let y = ui.get_f64("y", 0.0);
                let width = ui.get_f64("width", 0.0);
                let height = ui.get_f64("height", 0.0);
                if px >= x && py >= y && px <= x + width && py <= y + height {
                    hits.push((ui.get_i64("sorting_order", 0), entity, ui));
                }
            }
        }
        hits.sort_by_key(|(order, _, _)| *order);
        hits.pop().map(|(_, entity, ui)| (entity, ui))
    }
}
