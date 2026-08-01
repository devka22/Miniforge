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
    #[serde(default)]
    pub clip_rect: Option<(f32, f32, f32, f32)>,
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
                clip_rect: None,
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

    /// Scrolls the top-most scrollable `UIElement` below the pointer.
    ///
    /// Inventory grids, ability bars and scroll boxes work without a game
    /// script. `wheel_lines` follows the usual wheel convention: positive
    /// values move the visible content toward its beginning.
    pub fn scroll_entity_under_pointer(
        &mut self,
        entities: &mut [GameObject],
        pointer: (f64, f64),
        wheel_lines: f64,
    ) -> Option<u64> {
        if !wheel_lines.is_finite() || wheel_lines == 0.0 {
            return None;
        }
        let entity_id = entities
            .iter()
            .filter_map(|entity| {
                let ui = entity.get_component("UIElement")?;
                let inside = entity.enabled
                    && entity.visible
                    && ui.enabled
                    && ui.get_bool("visible", true)
                    && point_in_entity_ui(pointer, ui);
                (inside && entity_ui_scroll_range(ui).is_some_and(|range| range > 0.0))
                    .then_some((ui.get_i64("sorting_order", 0), entity.id))
            })
            .max()
            .map(|(_, entity_id)| entity_id)?;
        let entity = entities.iter_mut().find(|entity| entity.id == entity_id)?;
        let ui = entity.get_component_mut("UIElement")?;
        let max_scroll = entity_ui_scroll_range(ui)?;
        let automatic_step = default_entity_scroll_step(ui);
        let authored_step = ui.get_f64("scroll_step", 0.0);
        let step = if authored_step.is_finite() && authored_step > 0.0 {
            authored_step
        } else {
            automatic_step
        }
        .clamp(1.0, 512.0);
        let current = finite_or(ui.get_f64("scroll_y", 0.0), 0.0);
        let next = (current - wheel_lines * step).clamp(0.0, max_scroll);
        ui.set("scroll_y", serde_json::json!(next));
        Some(entity_id)
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
        if let Some(item) = pointer.and_then(|point| {
            layout
                .iter()
                .rev()
                .find(|item| item.interactive && point_in_layout_box(point, item))
        }) {
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

    /// Applies the built-in behavior of a retained-mode control without a
    /// gameplay script. Buttons still emit commands, while common stateful
    /// controls mutate their authored properties directly.
    pub fn activate_miniforge_widget(
        &mut self,
        canvas: &mut UiCanvas2D,
        widget_id: &str,
        viewport: (f32, f32),
        pointer: (f32, f32),
    ) -> bool {
        let Some(resolved) = canvas
            .resolve_layout(viewport)
            .into_iter()
            .find(|widget| widget.id == widget_id && point_in_resolved_widget(pointer, widget))
        else {
            return false;
        };
        let Some(widget) = canvas.find_widget_mut(widget_id) else {
            return false;
        };
        match widget.widget_type.as_str() {
            "Checkbox" | "Toggle" => {
                let checked = widget
                    .properties
                    .get("checked")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                ensure_json_object(&mut widget.properties)
                    .insert("checked".to_string(), serde_json::json!(!checked));
            }
            "Slider" => {
                let min = finite_json_f32(&widget.properties, "min");
                let authored_max = finite_json_f32(&widget.properties, "max");
                let max = if widget.properties.get("max").is_some() && authored_max > min {
                    authored_max
                } else {
                    min + 1.0
                };
                let normalized =
                    ((pointer.0 - resolved.rect.x) / resolved.rect.width).clamp(0.0, 1.0);
                ensure_json_object(&mut widget.properties).insert(
                    "value".to_string(),
                    serde_json::json!(min + (max - min) * normalized),
                );
            }
            "Dropdown" => {
                let option_count = widget
                    .properties
                    .get("options")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, Vec::len);
                if option_count > 0 {
                    let selected = widget
                        .properties
                        .get("selected")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0) as usize;
                    ensure_json_object(&mut widget.properties).insert(
                        "selected".to_string(),
                        serde_json::json!((selected + 1) % option_count),
                    );
                }
            }
            "InventoryGrid" | "AbilityBar" => {
                if let Some(slot) = clicked_miniforge_slot(widget, resolved.rect, pointer) {
                    ensure_json_object(&mut widget.properties)
                        .insert("selected_slot".to_string(), serde_json::json!(slot));
                }
            }
            _ => {}
        }
        true
    }

    /// Scrolls the top-most scriptless `ScrollBox` below the pointer.
    ///
    /// The authored `content_height` and `scroll_step` properties are
    /// optional. When omitted, the runtime derives a useful range from the
    /// widget tree and uses a 32 pixel step.
    pub fn scroll_miniforge_canvas_under_pointer(
        &mut self,
        canvas: &mut UiCanvas2D,
        viewport: (f32, f32),
        pointer: (f32, f32),
        wheel_lines: f32,
    ) -> Option<String> {
        if !wheel_lines.is_finite() || wheel_lines == 0.0 {
            return None;
        }
        let layout = canvas.resolve_layout(viewport);
        let target_id = layout.iter().rev().find_map(|item| {
            let widget = canvas.find_widget(&item.id)?;
            let scrollable = widget.widget_type.eq_ignore_ascii_case("ScrollBox")
                || widget
                    .properties
                    .get("scrollable")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            let max_scroll = scrollable
                .then(|| miniforge_scroll_range(widget))
                .flatten()?;
            (max_scroll > 0.0 && point_in_resolved_widget(pointer, item)).then_some(item.id.clone())
        })?;
        let widget = canvas.find_widget_mut(&target_id)?;
        let max_scroll = miniforge_scroll_range(widget)?;
        let authored_step = finite_json_f32(&widget.properties, "scroll_step");
        let step = if authored_step > 0.0 {
            authored_step
        } else {
            32.0
        }
        .clamp(1.0, 512.0);
        let current = finite_json_f32(&widget.properties, "scroll_y").max(0.0);
        let next = (current - wheel_lines * step).clamp(0.0, max_scroll);
        ensure_json_object(&mut widget.properties)
            .insert("scroll_y".to_string(), serde_json::json!(next));
        Some(target_id)
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
                clip_rect: widget
                    .clip_rect
                    .map(|rect| (rect.x, rect.y, rect.width, rect.height)),
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
    let scroll_y = finite_or(ui.get_f64("scroll_y", 0.0), 0.0).max(0.0);
    let local_y = pointer.1 - y + scroll_y;
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

fn point_in_entity_ui(pointer: (f64, f64), ui: &crate::engine::component::Component) -> bool {
    let x = finite_or(ui.get_f64("x", 0.0), 0.0);
    let y = finite_or(ui.get_f64("y", 0.0), 0.0);
    let width = finite_or(ui.get_f64("width", 0.0), 0.0).max(0.0);
    let height = finite_or(ui.get_f64("height", 0.0), 0.0).max(0.0);
    pointer.0 >= x && pointer.1 >= y && pointer.0 <= x + width && pointer.1 <= y + height
}

fn entity_ui_scroll_range(ui: &crate::engine::component::Component) -> Option<f64> {
    let widget_type = ui.get_string("element_type", "Label");
    let implicitly_scrollable = matches!(
        widget_type.as_str(),
        "InventoryGrid" | "AbilityBar" | "ScrollBox"
    );
    if !implicitly_scrollable && !ui.get_bool("scrollable", false) {
        return None;
    }
    let viewport_height = finite_or(ui.get_f64("height", 0.0), 0.0).max(0.0);
    let content_height = if matches!(widget_type.as_str(), "InventoryGrid" | "AbilityBar") {
        let columns = ui.get_i64("columns", 4).clamp(1, 16) as usize;
        let slot_count = ui
            .get("slot_count")
            .or_else(|| ui.get("slots"))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(columns as i64)
            .clamp(1, 256) as usize;
        let rows = slot_count.div_ceil(columns);
        let width = finite_or(ui.get_f64("width", 160.0), 160.0).max(0.0);
        let max_slot_width =
            ((width - 8.0 - 2.0 * columns.saturating_sub(1) as f64) / columns as f64).max(1.0);
        let slot_size = finite_or(ui.get_f64("slot_size", 32.0), 32.0)
            .clamp(8.0, 128.0)
            .min(max_slot_width);
        8.0 + rows as f64 * (slot_size + 2.0) - 2.0
    } else {
        finite_or(
            ui.get_f64("content_height", viewport_height),
            viewport_height,
        )
        .max(0.0)
    };
    Some((content_height - viewport_height).max(0.0))
}

fn default_entity_scroll_step(ui: &crate::engine::component::Component) -> f64 {
    if matches!(
        ui.get_string("element_type", "Label").as_str(),
        "InventoryGrid" | "AbilityBar"
    ) {
        finite_or(ui.get_f64("slot_size", 32.0), 32.0).clamp(8.0, 128.0) + 2.0
    } else {
        32.0
    }
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

fn point_in_layout_box(point: (f32, f32), layout: &UiLayoutBox) -> bool {
    point_in_rect(point, layout.rect)
        && layout
            .clip_rect
            .is_none_or(|clip_rect| point_in_rect(point, clip_rect))
}

fn point_in_resolved_widget(
    point: (f32, f32),
    widget: &crate::engine::miniforge_2d::ui_framework::UiResolvedWidget2D,
) -> bool {
    point_in_rect(
        point,
        (
            widget.rect.x,
            widget.rect.y,
            widget.rect.width,
            widget.rect.height,
        ),
    ) && widget
        .clip_rect
        .is_none_or(|clip| point_in_rect(point, (clip.x, clip.y, clip.width, clip.height)))
}

fn miniforge_scroll_range(
    widget: &crate::engine::miniforge_2d::ui_framework::UiWidget2D,
) -> Option<f32> {
    let viewport_height = widget.rect.height.max(0.0);
    if viewport_height <= 0.0 {
        return None;
    }
    let authored_height = finite_json_f32(&widget.properties, "content_height");
    let derived_height = widget
        .children
        .iter()
        .map(|child| child.rect.y.max(0.0) + child.rect.height.max(0.0))
        .fold(viewport_height, f32::max);
    let content_height = if authored_height > 0.0 {
        authored_height
    } else {
        derived_height
    };
    Some((content_height - viewport_height).max(0.0))
}

fn clicked_miniforge_slot(
    widget: &crate::engine::miniforge_2d::ui_framework::UiWidget2D,
    rect: crate::engine::miniforge_2d::ui_framework::UiRect2D,
    pointer: (f32, f32),
) -> Option<usize> {
    let columns = widget
        .properties
        .get("columns")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(4)
        .clamp(1, 16) as usize;
    let slot_count = ["slot_count", "slots"]
        .into_iter()
        .find_map(|key| {
            widget
                .properties
                .get(key)
                .and_then(serde_json::Value::as_u64)
        })
        .unwrap_or(columns as u64)
        .clamp(1, 256) as usize;
    let gap = 2.0;
    let max_slot_width =
        ((rect.width - 8.0 - gap * columns.saturating_sub(1) as f32) / columns as f32).max(1.0);
    let authored_slot_size = finite_json_f32(&widget.properties, "slot_size");
    let slot_size = if authored_slot_size > 0.0 {
        authored_slot_size
    } else {
        32.0
    }
    .clamp(8.0, 128.0)
    .min(max_slot_width);
    let local_x = pointer.0 - rect.x - 4.0;
    let local_y =
        pointer.1 - rect.y - 4.0 + finite_json_f32(&widget.properties, "scroll_y").max(0.0);
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

fn finite_json_f32(value: &serde_json::Value, key: &str) -> f32 {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0) as f32
}

fn ensure_json_object(
    value: &mut serde_json::Value,
) -> &mut serde_json::Map<String, serde_json::Value> {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
    value.as_object_mut().expect("object assigned above")
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
    use crate::engine::miniforge_2d::ui_framework::{
        Anchor2D, UiNavigation2D, UiRect2D, UiStyle2D, UiTheme2D, UiWidget2D,
    };
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

    fn retained_widget(id: &str, widget_type: &str, rect: UiRect2D) -> UiWidget2D {
        UiWidget2D {
            id: id.to_string(),
            widget_type: widget_type.to_string(),
            rect,
            anchors: Anchor2D::TOP_LEFT,
            children: Vec::new(),
            callbacks: Vec::new(),
            properties: json!({}),
            style: UiStyle2D::default(),
            bindings: Vec::new(),
            navigation: UiNavigation2D::default(),
        }
    }

    fn retained_canvas(widgets: Vec<UiWidget2D>) -> UiCanvas2D {
        UiCanvas2D {
            name: "Runtime UI".to_string(),
            viewport_width: 320.0,
            viewport_height: 180.0,
            widgets,
            theme: UiTheme2D::default(),
            animations: Vec::new(),
        }
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

    #[test]
    fn inventory_scroll_is_clamped_and_clicks_use_scrolled_content() {
        let mut inventory = ui_entity("LargeBag", "InventoryGrid", 0.0);
        let inventory_id = inventory.id;
        {
            let ui = inventory.get_component_mut("UIElement").unwrap();
            ui.set("columns", json!(2));
            ui.set("slot_count", json!(20));
            ui.set("slot_size", json!(20.0));
            ui.set("height", json!(52.0));
        }
        let mut entities = vec![inventory];
        let mut runtime = UiRuntime::default();

        assert_eq!(
            runtime.scroll_entity_under_pointer(&mut entities, (10.0, 10.0), -2.0),
            Some(inventory_id)
        );
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_f64("scroll_y", 0.0),
            44.0
        );

        runtime.update_entity_interaction(&mut entities, (10.0, 10.0), true);
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_i64("selected_slot", -1),
            4
        );

        runtime.scroll_entity_under_pointer(&mut entities, (10.0, 10.0), -100.0);
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_f64("scroll_y", 0.0),
            174.0
        );
        runtime.scroll_entity_under_pointer(&mut entities, (10.0, 10.0), 100.0);
        assert_eq!(
            entities[0]
                .get_component("UIElement")
                .unwrap()
                .get_f64("scroll_y", -1.0),
            0.0
        );
    }

    #[test]
    fn retained_scrollbox_scrolls_without_scripts_and_clamps() {
        let mut scroll = retained_widget(
            "feed",
            "ScrollBox",
            UiRect2D {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 60.0,
            },
        );
        scroll.properties = json!({"content_height": 180.0, "scroll_step": 24.0});
        let mut canvas = retained_canvas(vec![scroll]);
        let mut runtime = UiRuntime::default();

        assert_eq!(
            runtime.scroll_miniforge_canvas_under_pointer(
                &mut canvas,
                (320.0, 180.0),
                (20.0, 20.0),
                -2.0,
            ),
            Some("feed".to_string())
        );
        assert_eq!(
            canvas
                .find_widget("feed")
                .unwrap()
                .properties
                .get("scroll_y")
                .and_then(serde_json::Value::as_f64),
            Some(48.0)
        );
        runtime.scroll_miniforge_canvas_under_pointer(
            &mut canvas,
            (320.0, 180.0),
            (20.0, 20.0),
            -100.0,
        );
        assert_eq!(
            canvas
                .find_widget("feed")
                .unwrap()
                .properties
                .get("scroll_y")
                .and_then(serde_json::Value::as_f64),
            Some(120.0)
        );
    }

    #[test]
    fn retained_controls_mutate_common_state_without_scripts() {
        let mut toggle = retained_widget(
            "mute",
            "Checkbox",
            UiRect2D {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 30.0,
            },
        );
        toggle.properties = json!({"checked": false});
        let mut slider = retained_widget(
            "volume",
            "Slider",
            UiRect2D {
                x: 10.0,
                y: 50.0,
                width: 100.0,
                height: 20.0,
            },
        );
        slider.properties = json!({"min": 0.0, "max": 100.0, "value": 0.0});
        let mut canvas = retained_canvas(vec![toggle, slider]);
        let mut runtime = UiRuntime::default();

        assert!(runtime.activate_miniforge_widget(
            &mut canvas,
            "mute",
            (320.0, 180.0),
            (20.0, 20.0),
        ));
        assert_eq!(
            canvas.find_widget("mute").unwrap().properties["checked"],
            json!(true)
        );
        assert!(runtime.activate_miniforge_widget(
            &mut canvas,
            "volume",
            (320.0, 180.0),
            (85.0, 60.0),
        ));
        assert_eq!(
            canvas.find_widget("volume").unwrap().properties["value"],
            json!(75.0)
        );
    }
}
