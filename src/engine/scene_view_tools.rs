use crate::engine::ui_canvas::{UiCanvasEditReport, UiCanvasGizmoHandleKind, UiCanvasRoot};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneGizmoMode {
    Select,
    Move,
    Rotate,
    Scale,
}

impl SceneGizmoMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn center(self) -> (f64, f64) {
        (
            self.min_x + self.width() * 0.5,
            self.min_y + self.height() * 0.5,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SceneViewTools {
    pub grid_snapping: bool,
    pub snap_size: f64,
    pub tile_size: f64,
    pub camera_zoom: f64,
}

impl Default for SceneViewTools {
    fn default() -> Self {
        Self {
            grid_snapping: true,
            snap_size: 1.0,
            tile_size: 32.0,
            camera_zoom: 1.0,
        }
    }
}

impl SceneViewTools {
    pub fn apply_screen_drag(&self, selected: &mut [GameObject], dx: f64, dy: f64, tool: &str) {
        let world_dx = dx / self.tile_size / self.camera_zoom.max(0.0001);
        let world_dy = dy / self.tile_size / self.camera_zoom.max(0.0001);
        let mode = match tool {
            "Move" => SceneGizmoMode::Move,
            "Rotate" => SceneGizmoMode::Rotate,
            "Scale" => SceneGizmoMode::Scale,
            _ => SceneGizmoMode::Select,
        };
        self.apply_world_drag(selected, world_dx, world_dy, mode);
    }

    pub fn apply_world_drag(
        &self,
        selected: &mut [GameObject],
        world_dx: f64,
        world_dy: f64,
        mode: SceneGizmoMode,
    ) {
        for entity in selected {
            match mode {
                SceneGizmoMode::Move => {
                    entity.x += world_dx;
                    entity.y += world_dy;
                    (entity.x, entity.y) = self.snap_point(entity.x, entity.y);
                }
                SceneGizmoMode::Rotate => entity.rotation += world_dx * 18.0,
                SceneGizmoMode::Scale => {
                    let delta = (world_dx + world_dy) * 0.25;
                    entity.scale_x = (entity.scale_x + delta).max(0.05);
                    entity.scale_y = (entity.scale_y + delta).max(0.05);
                    entity.width = (entity.width * entity.scale_x).max(0.05);
                    entity.height = (entity.height * entity.scale_y).max(0.05);
                }
                SceneGizmoMode::Select => {}
            }
            entity.sync_to_components();
        }
    }

    pub fn set_world_position(&self, entity: &mut GameObject, x: f64, y: f64) {
        let (x, y) = self.snap_point(x, y);
        entity.x = x;
        entity.y = y;
        entity.path.clear();
        entity.sync_to_components();
    }

    pub fn snap_point(&self, x: f64, y: f64) -> (f64, f64) {
        if !self.grid_snapping {
            return (x, y);
        }
        let size = self.snap_size.max(0.0001);
        ((x / size).round() * size, (y / size).round() * size)
    }

    pub fn bounding_box(entity: &GameObject) -> BoundingBox {
        let half_w = (entity.width * entity.scale_x).abs().max(0.05) * 0.5;
        let half_h = (entity.height * entity.scale_y).abs().max(0.05) * 0.5;
        BoundingBox {
            min_x: entity.x - half_w,
            min_y: entity.y - half_h,
            max_x: entity.x + half_w,
            max_y: entity.y + half_h,
        }
    }

    pub fn select_ui_element_at(
        &self,
        root: &UiCanvasRoot,
        viewport_w: f32,
        viewport_h: f32,
        pointer: (f32, f32),
    ) -> Option<String> {
        root.hit_test_element(viewport_w, viewport_h, pointer)
            .map(|element| element.id().to_string())
    }

    pub fn drag_ui_element(
        &self,
        root: &mut UiCanvasRoot,
        element_id: &str,
        viewport_w: f32,
        viewport_h: f32,
        screen_dx: f32,
        screen_dy: f32,
    ) -> UiCanvasEditReport {
        let ref_dx = screen_dx * root.reference_width / viewport_w.max(1.0);
        let ref_dy = screen_dy * root.reference_height / viewport_h.max(1.0);
        let snap = self.grid_snapping.then_some(self.snap_size.max(1.0) as f32);
        root.move_element(element_id, ref_dx, ref_dy, snap)
    }

    pub fn resize_ui_element(
        &self,
        root: &mut UiCanvasRoot,
        element_id: &str,
        width: f32,
        height: f32,
    ) -> UiCanvasEditReport {
        let snap = self.grid_snapping.then_some(self.snap_size.max(1.0) as f32);
        root.resize_element(element_id, width, height, snap)
    }

    #[allow(clippy::too_many_arguments, reason = "UI resize command boundary")]
    pub fn resize_ui_element_from_handle(
        &self,
        root: &mut UiCanvasRoot,
        element_id: &str,
        handle: UiCanvasGizmoHandleKind,
        viewport_w: f32,
        viewport_h: f32,
        screen_dx: f32,
        screen_dy: f32,
    ) -> UiCanvasEditReport {
        let ref_dx = screen_dx * root.reference_width / viewport_w.max(1.0);
        let ref_dy = screen_dy * root.reference_height / viewport_h.max(1.0);
        let snap = self.grid_snapping.then_some(self.snap_size.max(1.0) as f32);
        root.resize_element_from_handle(element_id, handle, ref_dx, ref_dy, snap)
    }
}
