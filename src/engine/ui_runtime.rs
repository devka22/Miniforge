use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::engine::miniforge_2d::ui_framework::{
    UiCanvas2D, UiResolvedWidget2D, is_interactive_widget_type,
};
use crate::engine::ui_canvas::{UICanvas, UiCanvasElement, UiCanvasRoot, layout_element_pixels};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiEventKind {
    HoverEnter,
    HoverExit,
    Click,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiRuntimeEvent {
    pub kind: UiEventKind,
    pub element_id: String,
    pub entity_id: Option<u64>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UiLayoutBox {
    pub id: String,
    pub rect: (f32, f32, f32, f32),
    pub interactive: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UiRuntime {
    hovered: BTreeSet<String>,
    focused: Option<String>,
    pub last_layout: BTreeMap<String, UiLayoutBox>,
    pub events: Vec<UiRuntimeEvent>,
}

impl UiRuntime {
    pub fn layout_canvas(
        &mut self,
        canvas: &UiCanvasRoot,
        viewport: (f32, f32),
    ) -> Vec<UiLayoutBox> {
        let mut boxes = Vec::new();
        for element in &canvas.elements {
            let rect = element_rect(element);
            let id = element_id(element);
            let layout = UiLayoutBox {
                id: id.clone(),
                rect: layout_element_pixels(canvas, rect, viewport.0, viewport.1),
                interactive: element_interactive(element),
            };
            self.last_layout.insert(id, layout.clone());
            boxes.push(layout);
        }
        boxes
    }

    pub fn update_canvas_interaction(
        &mut self,
        canvas: &UiCanvasRoot,
        viewport: (f32, f32),
        pointer: Option<(f32, f32)>,
        clicked: bool,
    ) -> Vec<UiRuntimeEvent> {
        self.events.clear();
        let layout = self.layout_canvas(canvas, viewport);
        let mut now_hovered = BTreeSet::new();
        if let Some(point) = pointer {
            for item in layout.iter().filter(|item| item.interactive) {
                if point_in_rect(point, item.rect) {
                    now_hovered.insert(item.id.clone());
                    if !self.hovered.contains(&item.id) {
                        self.events.push(UiRuntimeEvent {
                            kind: UiEventKind::HoverEnter,
                            element_id: item.id.clone(),
                            entity_id: None,
                            command: None,
                        });
                    }
                    if clicked {
                        self.events.push(UiRuntimeEvent {
                            kind: UiEventKind::Click,
                            element_id: item.id.clone(),
                            entity_id: None,
                            command: element_command(canvas, &item.id),
                        });
                    }
                }
            }
        }
        for previous in self.hovered.difference(&now_hovered) {
            self.events.push(UiRuntimeEvent {
                kind: UiEventKind::HoverExit,
                element_id: previous.clone(),
                entity_id: None,
                command: None,
            });
        }
        self.hovered = now_hovered;
        self.events.clone()
    }

    pub fn update_entity_interaction(
        &mut self,
        entities: &mut [GameObject],
        pointer: (f64, f64),
        clicked: bool,
    ) -> Vec<UiRuntimeEvent> {
        let mut events = Vec::new();
        let hovered = UICanvas::default()
            .hit_test(entities, pointer)
            .map(|(entity, _)| entity.id);
        for entity in entities {
            let entity_id = entity.id;
            let Some(ui) = entity.get_component_mut("UIElement") else {
                continue;
            };
            let element_id = entity_id.to_string();
            let is_hovered = hovered == Some(entity_id);
            let was_hovered = ui.get_bool("_hovered", false);
            if is_hovered != was_hovered {
                ui.set("_hovered", serde_json::json!(is_hovered));
                events.push(UiRuntimeEvent {
                    kind: if is_hovered {
                        UiEventKind::HoverEnter
                    } else {
                        UiEventKind::HoverExit
                    },
                    element_id: element_id.clone(),
                    entity_id: Some(entity_id),
                    command: None,
                });
            }
            if is_hovered && clicked && ui.get_bool("interactable", false) {
                events.push(UiRuntimeEvent {
                    kind: UiEventKind::Click,
                    element_id,
                    entity_id: Some(entity_id),
                    command: ui
                        .get("on_click")
                        .or_else(|| ui.get("on_click_graph"))
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                });
            }
        }
        self.events = events.clone();
        events
    }

    pub fn layout_miniforge_canvas(
        &mut self,
        canvas: &UiCanvas2D,
        viewport: (f32, f32),
    ) -> Vec<UiLayoutBox> {
        let resolved = canvas.resolve_layout(viewport);
        self.record_resolved_layout(&resolved)
    }

    pub fn update_miniforge_canvas_interaction(
        &mut self,
        canvas: &UiCanvas2D,
        viewport: (f32, f32),
        pointer: Option<(f32, f32)>,
        clicked: bool,
    ) -> Vec<UiRuntimeEvent> {
        self.events.clear();
        let layout = self.layout_miniforge_canvas(canvas, viewport);
        let mut now_hovered = BTreeSet::new();
        if let Some(point) = pointer {
            for item in layout.iter().filter(|item| item.interactive) {
                if point_in_rect(point, item.rect) {
                    now_hovered.insert(item.id.clone());
                    if !self.hovered.contains(&item.id) {
                        self.events.push(UiRuntimeEvent {
                            kind: UiEventKind::HoverEnter,
                            element_id: item.id.clone(),
                            entity_id: None,
                            command: None,
                        });
                    }
                    if clicked {
                        self.focused = Some(item.id.clone());
                        self.events.push(UiRuntimeEvent {
                            kind: UiEventKind::Click,
                            element_id: item.id.clone(),
                            entity_id: None,
                            command: canvas
                                .command_for_widget(&item.id)
                                .or_else(|| Some(item.id.clone())),
                        });
                    }
                }
            }
        }
        for previous in self.hovered.difference(&now_hovered) {
            self.events.push(UiRuntimeEvent {
                kind: UiEventKind::HoverExit,
                element_id: previous.clone(),
                entity_id: None,
                command: None,
            });
        }
        self.hovered = now_hovered;
        self.events.clone()
    }

    pub fn move_focus(&mut self, canvas: &UiCanvas2D, direction: &str) -> Option<String> {
        let current = self
            .focused
            .clone()
            .or_else(|| first_interactive_widget(canvas).map(ToString::to_string))?;
        let next = canvas
            .focused_neighbor(&current, direction)
            .map(ToString::to_string)
            .unwrap_or(current);
        self.focused = Some(next.clone());
        Some(next)
    }

    pub fn focused_widget(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    fn record_resolved_layout(&mut self, resolved: &[UiResolvedWidget2D]) -> Vec<UiLayoutBox> {
        let mut boxes = Vec::new();
        for widget in resolved {
            let layout = UiLayoutBox {
                id: widget.id.clone(),
                rect: (
                    widget.rect.x,
                    widget.rect.y,
                    widget.rect.width,
                    widget.rect.height,
                ),
                interactive: widget.interactive,
            };
            self.last_layout.insert(widget.id.clone(), layout.clone());
            boxes.push(layout);
        }
        boxes
    }
}

fn element_id(element: &UiCanvasElement) -> String {
    match element {
        UiCanvasElement::Panel { id, .. }
        | UiCanvasElement::Button { id, .. }
        | UiCanvasElement::Label { id, .. }
        | UiCanvasElement::Image { id, .. } => id.clone(),
    }
}

fn element_rect(element: &UiCanvasElement) -> &crate::engine::ui_canvas::UiRect {
    match element {
        UiCanvasElement::Panel { rect, .. }
        | UiCanvasElement::Button { rect, .. }
        | UiCanvasElement::Label { rect, .. }
        | UiCanvasElement::Image { rect, .. } => rect,
    }
}

fn element_interactive(element: &UiCanvasElement) -> bool {
    matches!(element, UiCanvasElement::Button { .. })
}

fn element_command(canvas: &UiCanvasRoot, id: &str) -> Option<String> {
    canvas
        .elements
        .iter()
        .find(|element| element_id(element) == id)
        .and_then(|element| match element {
            UiCanvasElement::Button { label, .. } => Some(label.clone()),
            _ => None,
        })
}

fn point_in_rect(point: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    point.0 >= rect.0
        && point.1 >= rect.1
        && point.0 <= rect.0 + rect.2
        && point.1 <= rect.1 + rect.3
}

fn first_interactive_widget(canvas: &UiCanvas2D) -> Option<&str> {
    canvas
        .flatten_widgets()
        .into_iter()
        .find(|widget| {
            is_interactive_widget_type(&widget.widget_type) || !widget.callbacks.is_empty()
        })
        .map(|widget| widget.id.as_str())
}
