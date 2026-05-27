use serde::{Deserialize, Serialize};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneViewMode2D {
    Editor,
    Game,
    Camera,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneTool2D {
    Select,
    Move,
    Rotate,
    Scale,
    Paint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneView2D {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub grid: bool,
    pub snap: bool,
    pub snap_size: f32,
    pub active_tool: SceneTool2D,
    pub view_mode: SceneViewMode2D,
    pub show_colliders: bool,
    pub show_pivots: bool,
    pub show_names: bool,
    pub show_camera_icons: bool,
    pub show_light_icons: bool,
    pub show_audio_icons: bool,
    pub show_spawn_icons: bool,
    pub show_trigger_icons: bool,
    pub debug_draw: bool,
    pub selected_ids: Vec<u64>,
    pub box_selection: Option<(f64, f64, f64, f64)>,
}

impl Default for SceneView2D {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            grid: true,
            snap: false,
            snap_size: 1.0,
            active_tool: SceneTool2D::Select,
            view_mode: SceneViewMode2D::Editor,
            show_colliders: false,
            show_pivots: true,
            show_names: true,
            show_camera_icons: true,
            show_light_icons: true,
            show_audio_icons: true,
            show_spawn_icons: true,
            show_trigger_icons: true,
            debug_draw: false,
            selected_ids: Vec::new(),
            box_selection: None,
        }
    }
}

impl SceneView2D {
    pub fn apply_shortcut(&mut self, shortcut: &str) -> bool {
        match shortcut {
            "W" => self.active_tool = SceneTool2D::Move,
            "E" => self.active_tool = SceneTool2D::Rotate,
            "R" => self.active_tool = SceneTool2D::Scale,
            "G" => self.grid = !self.grid,
            _ => return false,
        }
        true
    }

    pub fn select_at(
        &mut self,
        entities: &[GameObject],
        point: (f64, f64),
        multi: bool,
    ) -> Option<u64> {
        let hit = entities
            .iter()
            .rev()
            .find(|entity| entity.enabled && point_hits_entity(entity, point))
            .map(|entity| entity.id);
        if let Some(id) = hit {
            if !multi {
                self.selected_ids.clear();
            }
            if !self.selected_ids.contains(&id) {
                self.selected_ids.push(id);
            }
        } else if !multi {
            self.selected_ids.clear();
        }
        hit
    }

    pub fn box_select(&mut self, entities: &[GameObject], rect: (f64, f64, f64, f64)) -> Vec<u64> {
        self.box_selection = Some(rect);
        let (x, y, w, h) = normalized_rect(rect);
        self.selected_ids = entities
            .iter()
            .filter(|entity| {
                entity.x >= x && entity.y >= y && entity.x <= x + w && entity.y <= y + h
            })
            .map(|entity| entity.id)
            .collect();
        self.selected_ids.clone()
    }

    pub fn focus_selected(&mut self, entities: &[GameObject]) -> bool {
        let selected = entities
            .iter()
            .find(|entity| self.selected_ids.contains(&entity.id));
        let Some(entity) = selected else {
            return false;
        };
        self.pan_x = entity.x as f32;
        self.pan_y = entity.y as f32;
        true
    }
}

fn point_hits_entity(entity: &GameObject, point: (f64, f64)) -> bool {
    let half_w = (entity.width * entity.scale_x).abs() * 0.5;
    let half_h = (entity.height * entity.scale_y).abs() * 0.5;
    point.0 >= entity.x - half_w
        && point.0 <= entity.x + half_w
        && point.1 >= entity.y - half_h
        && point.1 <= entity.y + half_h
}

fn normalized_rect(rect: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let x = rect.0.min(rect.0 + rect.2);
    let y = rect.1.min(rect.1 + rect.3);
    (x, y, rect.2.abs(), rect.3.abs())
}
