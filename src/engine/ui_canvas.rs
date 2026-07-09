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

impl UiCanvasElement {
    pub fn id(&self) -> &str {
        match self {
            Self::Panel { id, .. }
            | Self::Button { id, .. }
            | Self::Label { id, .. }
            | Self::Image { id, .. } => id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Panel { .. } => "Panel",
            Self::Button { .. } => "Button",
            Self::Label { .. } => "Label",
            Self::Image { .. } => "Image",
        }
    }

    pub fn rect(&self) -> &UiRect {
        match self {
            Self::Panel { rect, .. }
            | Self::Button { rect, .. }
            | Self::Label { rect, .. }
            | Self::Image { rect, .. } => rect,
        }
    }

    pub fn rect_mut(&mut self) -> &mut UiRect {
        match self {
            Self::Panel { rect, .. }
            | Self::Button { rect, .. }
            | Self::Label { rect, .. }
            | Self::Image { rect, .. } => rect,
        }
    }

    pub fn set_text(&mut self, value: impl Into<String>) -> bool {
        match self {
            Self::Button { label, .. } => {
                *label = value.into();
                true
            }
            Self::Label { text, .. } => {
                *text = value.into();
                true
            }
            _ => false,
        }
    }

    pub fn set_sprite_path(&mut self, value: impl Into<String>) -> bool {
        match self {
            Self::Image { sprite_path, .. } => {
                *sprite_path = value.into();
                true
            }
            _ => false,
        }
    }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UiCanvasEditReport {
    pub element_id: String,
    pub action: String,
    pub changed: bool,
    #[serde(default)]
    pub before: Option<UiRect>,
    #[serde(default)]
    pub after: Option<UiRect>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiCanvasGizmoHandleKind {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl UiCanvasGizmoHandleKind {
    pub fn cursor(self) -> &'static str {
        match self {
            Self::TopLeft | Self::BottomRight => "nwse-resize",
            Self::TopRight | Self::BottomLeft => "nesw-resize",
            Self::Top | Self::Bottom => "ns-resize",
            Self::Left | Self::Right => "ew-resize",
        }
    }

    fn adjusts_left(self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft | Self::Left)
    }

    fn adjusts_right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight | Self::Right)
    }

    fn adjusts_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight | Self::Top)
    }

    fn adjusts_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight | Self::Bottom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCanvasGizmoHandle {
    pub element_id: String,
    pub kind: UiCanvasGizmoHandleKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub cursor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCanvasGizmo {
    pub element_id: String,
    pub element_kind: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub handles: Vec<UiCanvasGizmoHandle>,
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

    pub fn element_ids(&self) -> Vec<String> {
        self.elements
            .iter()
            .map(|element| element.id().to_string())
            .collect()
    }

    pub fn find_element(&self, id: &str) -> Option<&UiCanvasElement> {
        self.elements.iter().find(|element| element.id() == id)
    }

    pub fn find_element_mut(&mut self, id: &str) -> Option<&mut UiCanvasElement> {
        self.elements.iter_mut().find(|element| element.id() == id)
    }

    pub fn hit_test_element(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        pointer: (f32, f32),
    ) -> Option<&UiCanvasElement> {
        self.elements.iter().rev().find(|element| {
            let rect = element.rect();
            let (x, y, width, height) = layout_element_pixels(self, rect, viewport_w, viewport_h);
            pointer.0 >= x && pointer.1 >= y && pointer.0 <= x + width && pointer.1 <= y + height
        })
    }

    pub fn gizmo_for_element(
        &self,
        id: &str,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<UiCanvasGizmo> {
        let element = self.find_element(id)?;
        let (x, y, width, height) =
            layout_element_pixels(self, element.rect(), viewport_w, viewport_h);
        Some(UiCanvasGizmo {
            element_id: id.to_string(),
            element_kind: element.kind().to_string(),
            x,
            y,
            width,
            height,
            handles: gizmo_handles(id, x, y, width, height),
        })
    }

    pub fn hit_test_gizmo_handle(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        pointer: (f32, f32),
    ) -> Option<UiCanvasGizmoHandle> {
        self.elements.iter().rev().find_map(|element| {
            let gizmo = self.gizmo_for_element(element.id(), viewport_w, viewport_h)?;
            gizmo
                .handles
                .into_iter()
                .rev()
                .find(|handle| point_in_handle(pointer, handle))
        })
    }

    pub fn move_element(
        &mut self,
        id: &str,
        dx: f32,
        dy: f32,
        snap_grid: Option<f32>,
    ) -> UiCanvasEditReport {
        let Some(element) = self.find_element_mut(id) else {
            return edit_report(id, "move", false, None, None);
        };
        let before = element.rect().clone();
        let rect = element.rect_mut();
        rect.offset_x += dx;
        rect.offset_y += dy;
        if let Some(grid) = snap_grid.filter(|grid| *grid > 0.0) {
            rect.offset_x = snap_f32(rect.offset_x, grid);
            rect.offset_y = snap_f32(rect.offset_y, grid);
        }
        let after = element.rect().clone();
        edit_report(id, "move", before != after, Some(before), Some(after))
    }

    pub fn resize_element(
        &mut self,
        id: &str,
        width: f32,
        height: f32,
        snap_grid: Option<f32>,
    ) -> UiCanvasEditReport {
        let Some(element) = self.find_element_mut(id) else {
            return edit_report(id, "resize", false, None, None);
        };
        let before = element.rect().clone();
        let rect = element.rect_mut();
        rect.width = width.max(1.0);
        rect.height = height.max(1.0);
        if let Some(grid) = snap_grid.filter(|grid| *grid > 0.0) {
            rect.width = snap_f32(rect.width, grid).max(1.0);
            rect.height = snap_f32(rect.height, grid).max(1.0);
        }
        let after = element.rect().clone();
        edit_report(id, "resize", before != after, Some(before), Some(after))
    }

    pub fn resize_element_from_handle(
        &mut self,
        id: &str,
        handle: UiCanvasGizmoHandleKind,
        dx: f32,
        dy: f32,
        snap_grid: Option<f32>,
    ) -> UiCanvasEditReport {
        let ref_width = self.reference_width.max(1.0);
        let ref_height = self.reference_height.max(1.0);
        let Some(element) = self.find_element_mut(id) else {
            return edit_report(id, "resize_handle", false, None, None);
        };
        let before = element.rect().clone();
        let rect = element.rect_mut();
        let (mut left, mut top, width, height) =
            layout_element_reference(rect, ref_width, ref_height);
        let mut right = left + width;
        let mut bottom = top + height;

        if handle.adjusts_left() {
            left += dx;
        }
        if handle.adjusts_right() {
            right += dx;
        }
        if handle.adjusts_top() {
            top += dy;
        }
        if handle.adjusts_bottom() {
            bottom += dy;
        }

        if let Some(grid) = snap_grid.filter(|grid| *grid > 0.0) {
            left = snap_f32(left, grid);
            right = snap_f32(right, grid);
            top = snap_f32(top, grid);
            bottom = snap_f32(bottom, grid);
        }

        enforce_min_bounds(&mut left, &mut right, handle.adjusts_left());
        enforce_min_bounds(&mut top, &mut bottom, handle.adjusts_top());

        rect.width = (right - left).max(1.0);
        rect.height = (bottom - top).max(1.0);
        rect.offset_x = left + rect.pivot_x * rect.width - rect.anchor.min_x * ref_width;
        rect.offset_y = top + rect.pivot_y * rect.height - rect.anchor.min_y * ref_height;
        let after = element.rect().clone();
        edit_report(
            id,
            "resize_handle",
            before != after,
            Some(before),
            Some(after),
        )
    }

    pub fn set_element_text(&mut self, id: &str, text: impl Into<String>) -> bool {
        self.find_element_mut(id)
            .map(|element| element.set_text(text))
            .unwrap_or(false)
    }

    pub fn set_element_sprite(&mut self, id: &str, sprite_path: impl Into<String>) -> bool {
        self.find_element_mut(id)
            .map(|element| element.set_sprite_path(sprite_path))
            .unwrap_or(false)
    }
}

fn edit_report(
    element_id: &str,
    action: &str,
    changed: bool,
    before: Option<UiRect>,
    after: Option<UiRect>,
) -> UiCanvasEditReport {
    UiCanvasEditReport {
        element_id: element_id.to_string(),
        action: action.to_string(),
        changed,
        before,
        after,
    }
}

fn snap_f32(value: f32, grid: f32) -> f32 {
    (value / grid).round() * grid
}

fn layout_element_reference(
    element_rect: &UiRect,
    reference_w: f32,
    reference_h: f32,
) -> (f32, f32, f32, f32) {
    let ax = element_rect.anchor.min_x * reference_w;
    let ay = element_rect.anchor.min_y * reference_h;
    let px = ax + element_rect.offset_x - element_rect.pivot_x * element_rect.width;
    let py = ay + element_rect.offset_y - element_rect.pivot_y * element_rect.height;
    (px, py, element_rect.width, element_rect.height)
}

fn enforce_min_bounds(start: &mut f32, end: &mut f32, anchor_end: bool) {
    const MIN_SIZE: f32 = 1.0;
    if *end - *start >= MIN_SIZE {
        return;
    }
    if anchor_end {
        *start = *end - MIN_SIZE;
    } else {
        *end = *start + MIN_SIZE;
    }
}

fn gizmo_handles(
    element_id: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Vec<UiCanvasGizmoHandle> {
    let size = 10.0;
    let half = size * 0.5;
    let cx = x + width * 0.5;
    let cy = y + height * 0.5;
    let right = x + width;
    let bottom = y + height;
    [
        (UiCanvasGizmoHandleKind::TopLeft, x, y),
        (UiCanvasGizmoHandleKind::Top, cx, y),
        (UiCanvasGizmoHandleKind::TopRight, right, y),
        (UiCanvasGizmoHandleKind::Right, right, cy),
        (UiCanvasGizmoHandleKind::BottomRight, right, bottom),
        (UiCanvasGizmoHandleKind::Bottom, cx, bottom),
        (UiCanvasGizmoHandleKind::BottomLeft, x, bottom),
        (UiCanvasGizmoHandleKind::Left, x, cy),
    ]
    .into_iter()
    .map(|(kind, center_x, center_y)| UiCanvasGizmoHandle {
        element_id: element_id.to_string(),
        kind,
        x: center_x - half,
        y: center_y - half,
        width: size,
        height: size,
        cursor: kind.cursor().to_string(),
    })
    .collect()
}

fn point_in_handle(pointer: (f32, f32), handle: &UiCanvasGizmoHandle) -> bool {
    pointer.0 >= handle.x
        && pointer.1 >= handle.y
        && pointer.0 <= handle.x + handle.width
        && pointer.1 <= handle.y + handle.height
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
    (
        px * sx,
        py * sy,
        element_rect.width * sx,
        element_rect.height * sy,
    )
}

pub fn ui_canvases_from_value(value: &Value) -> Vec<UiCanvasRoot> {
    value
        .as_array()
        .map(|arr| arr.iter().filter_map(UiCanvasRoot::from_value).collect())
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
