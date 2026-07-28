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
            if !ui.enabled || !ui.get_bool("visible", true) {
                continue;
            }
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
            let widget_type = ui.get_string("element_type", "Label");
            let is_implicit_control = is_entity_control_type(&widget_type);
            if clicked {
                let is_text_input = matches!(widget_type.as_str(), "InputField" | "TextInput");
                ui.set("focused", serde_json::json!(is_hovered && is_text_input));
            }
            if is_hovered && clicked && (ui.get_bool("interactable", false) || is_implicit_control)
            {
                apply_entity_control_click(ui, &widget_type, pointer);
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

fn is_entity_control_type(widget_type: &str) -> bool {
    matches!(
        widget_type,
        "Button"
            | "MenuButton"
            | "IconButton"
            | "Slider"
            | "Checkbox"
            | "Toggle"
            | "Dropdown"
            | "InputField"
            | "TextInput"
            | "InventoryGrid"
            | "AbilityBar"
    )
}

fn apply_entity_control_click(
    ui: &mut crate::engine::component::Component,
    widget_type: &str,
    pointer: (f64, f64),
) {
    match widget_type {
        "Checkbox" | "Toggle" => {
            ui.set("checked", serde_json::json!(!ui.get_bool("checked", false)));
        }
        "Slider" => {
            let min = finite_or(ui.get_f64("min", 0.0), 0.0);
            let max = valid_range_max(min, ui.get_f64("max", 1.0));
            let x = finite_or(ui.get_f64("x", 0.0), 0.0);
            let width = finite_or(ui.get_f64("width", 1.0), 1.0).max(1.0);
            let normalized = ((pointer.0 - x) / width).clamp(0.0, 1.0);
            ui.set("value", serde_json::json!(min + (max - min) * normalized));
        }
        "Dropdown" => {
            let option_count = ui
                .get("options")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            if option_count > 0 {
                let next = (ui.get_i64("selected", 0).max(0) as usize + 1) % option_count;
                ui.set("selected", serde_json::json!(next));
            }
        }
        "InventoryGrid" | "AbilityBar" => {
            if let Some(slot) = clicked_slot(ui, pointer) {
                ui.set("selected_slot", serde_json::json!(slot));
            }
        }
        _ => {}
    }
}

fn clicked_slot(ui: &crate::engine::component::Component, pointer: (f64, f64)) -> Option<usize> {
    let columns = ui.get_i64("columns", 4).clamp(1, 16) as usize;
    let slot_count = ui
        .get("slot_count")
        .or_else(|| ui.get("slots"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(columns as i64)
        .clamp(1, 256) as usize;
    let x = finite_or(ui.get_f64("x", 0.0), 0.0) + 4.0;
    let y = finite_or(ui.get_f64("y", 0.0), 0.0) + 4.0;
    let gap = 2.0;
    let slot_size = finite_or(ui.get_f64("slot_size", 32.0), 32.0).clamp(8.0, 128.0);
    let local_x = pointer.0 - x;
    let local_y = pointer.1 - y;
    if local_x < 0.0 || local_y < 0.0 {
        return None;
    }
    let stride = slot_size + gap;
    let column = (local_x / stride).floor() as usize;
    let row = (local_y / stride).floor() as usize;
    if column >= columns || local_x % stride > slot_size || local_y % stride > slot_size {
        return None;
    }
    let slot = row.saturating_mul(columns).saturating_add(column);
    (slot < slot_count).then_some(slot)
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn valid_range_max(min: f64, max: f64) -> f64 {
    if max.is_finite() && max > min {
        max
    } else {
        min + 1.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;
    use serde_json::json;

    fn ui_entity(name: &str, widget_type: &str, x: f64) -> GameObject {
        let mut entity = GameObject::new(0.0, 0.0, Some(name.to_string()));
        let mut ui = default_component("UIElement").unwrap();
        ui.set("element_type", json!(widget_type));
        ui.set("x", json!(x));
        ui.set("y", json!(0.0));
        ui.set("width", json!(100.0));
        ui.set("height", json!(40.0));
        entity.add_component(ui);
        entity
    }

    #[test]
    fn entity_controls_update_state_without_game_scripts() {
        let mut checkbox = ui_entity("Checkbox", "Checkbox", 0.0);
        checkbox
            .get_component_mut("UIElement")
            .unwrap()
            .set("checked", json!(false));
        let checkbox_id = checkbox.id;
        let mut slider = ui_entity("Slider", "Slider", 120.0);
        let slider_id = slider.id;
        {
            let ui = slider.get_component_mut("UIElement").unwrap();
            ui.set("min", json!(10.0));
            ui.set("max", json!(20.0));
        }
        let input = ui_entity("Name", "TextInput", 240.0);
        let input_id = input.id;
        let mut entities = vec![checkbox, slider, input];
        let mut runtime = UiRuntime::default();

        let events = runtime.update_entity_interaction(&mut entities, (10.0, 10.0), true);
        assert!(events.iter().any(|event| {
            event.kind == UiEventKind::Click && event.entity_id == Some(checkbox_id)
        }));
        assert!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_bool("checked", false)
        );

        runtime.update_entity_interaction(&mut entities, (195.0, 10.0), true);
        assert_eq!(
            entities[1]
                .get_component("UIElement")
                .unwrap()
                .get_f64("value", 0.0),
            17.5
        );
        assert_eq!(entities[1].id, slider_id);

        runtime.update_entity_interaction(&mut entities, (250.0, 10.0), true);
        assert!(
            entities[2]
                .get_component("UIElement")
                .unwrap()
                .get_bool("focused", false)
        );
        assert_eq!(entities[2].id, input_id);
    }

    #[test]
    fn inventory_click_selects_a_slot_and_ignores_gaps() {
        let mut inventory = ui_entity("Bag", "InventoryGrid", 0.0);
        {
            let ui = inventory.get_component_mut("UIElement").unwrap();
            ui.set("columns", json!(3));
            ui.set("slot_count", json!(6));
            ui.set("slot_size", json!(20.0));
        }
        let mut entities = vec![inventory];
        let mut runtime = UiRuntime::default();

        runtime.update_entity_interaction(&mut entities, (28.0, 28.0), true);
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_i64("selected_slot", -1),
            4
        );
        runtime.update_entity_interaction(&mut entities, (25.0, 10.0), true);
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_i64("selected_slot", -1),
            4
        );
    }
}
