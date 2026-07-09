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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneGuideAxis2D {
    X,
    Y,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneGuide2D {
    pub axis: SceneGuideAxis2D,
    pub value: f64,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneSnapTarget2D {
    None,
    Grid,
    Pixel,
    GuideX,
    GuideY,
    GuideBoth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SceneSnapResult2D {
    pub point: (f64, f64),
    pub target: SceneSnapTarget2D,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneOverlayKind2D {
    SelectionRect,
    GuideLine,
    ColliderOutline,
    Pivot,
    Label,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneOverlayCommand2D {
    pub kind: SceneOverlayKind2D,
    pub label: String,
    #[serde(default)]
    pub entity_id: Option<u64>,
    #[serde(default)]
    pub axis: Option<SceneGuideAxis2D>,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub rect: Option<(f64, f64, f64, f64)>,
    #[serde(default)]
    pub point: Option<(f64, f64)>,
    pub color: [u8; 4],
    pub thickness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneView2D {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub grid: bool,
    pub snap: bool,
    pub snap_size: f32,
    #[serde(default)]
    pub grid_offset_x: f32,
    #[serde(default)]
    pub grid_offset_y: f32,
    #[serde(default)]
    pub pixel_snap: bool,
    #[serde(default)]
    pub smart_snap: bool,
    #[serde(default = "default_smart_snap_tolerance")]
    pub smart_snap_tolerance: f32,
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
    #[serde(default)]
    pub guides: Vec<SceneGuide2D>,
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
            grid_offset_x: 0.0,
            grid_offset_y: 0.0,
            pixel_snap: false,
            smart_snap: false,
            smart_snap_tolerance: default_smart_snap_tolerance(),
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
            guides: Vec::new(),
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

    pub fn world_to_screen(&self, point: (f64, f64), viewport_origin: (f64, f64)) -> (f64, f64) {
        (
            viewport_origin.0 + (point.0 - f64::from(self.pan_x)) * f64::from(self.zoom),
            viewport_origin.1 + (point.1 - f64::from(self.pan_y)) * f64::from(self.zoom),
        )
    }

    pub fn screen_to_world(&self, point: (f64, f64), viewport_origin: (f64, f64)) -> (f64, f64) {
        let zoom = f64::from(self.zoom.max(0.001));
        (
            (point.0 - viewport_origin.0) / zoom + f64::from(self.pan_x),
            (point.1 - viewport_origin.1) / zoom + f64::from(self.pan_y),
        )
    }

    pub fn snap_point(&self, point: (f64, f64)) -> SceneSnapResult2D {
        let mut snapped = point;
        let mut target = SceneSnapTarget2D::None;

        if self.snap && self.snap_size > f32::EPSILON {
            let step = f64::from(self.snap_size);
            let offset_x = f64::from(self.grid_offset_x);
            let offset_y = f64::from(self.grid_offset_y);
            snapped.0 = ((snapped.0 - offset_x) / step).round() * step + offset_x;
            snapped.1 = ((snapped.1 - offset_y) / step).round() * step + offset_y;
            target = SceneSnapTarget2D::Grid;
        }

        if self.pixel_snap {
            snapped.0 = snapped.0.round();
            snapped.1 = snapped.1.round();
            target = SceneSnapTarget2D::Pixel;
        }

        if self.smart_snap {
            let guide_target = self.snap_to_guides(&mut snapped);
            if guide_target != SceneSnapTarget2D::None {
                target = guide_target;
            }
        }

        SceneSnapResult2D {
            point: snapped,
            target,
        }
    }

    pub fn rebuild_guides_from_entities(&mut self, entities: &[GameObject]) {
        self.guides = entities
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
            .flat_map(|entity| {
                [
                    SceneGuide2D {
                        axis: SceneGuideAxis2D::X,
                        value: entity.x,
                        label: entity.name.clone(),
                    },
                    SceneGuide2D {
                        axis: SceneGuideAxis2D::Y,
                        value: entity.y,
                        label: entity.name.clone(),
                    },
                ]
            })
            .collect();
    }

    pub fn overlay_commands(&self, entities: &[GameObject]) -> Vec<SceneOverlayCommand2D> {
        let mut commands = Vec::new();
        if let Some(rect) = self.box_selection {
            commands.push(SceneOverlayCommand2D {
                kind: SceneOverlayKind2D::SelectionRect,
                label: "Box Selection".to_string(),
                entity_id: None,
                axis: None,
                value: None,
                rect: Some(normalized_rect(rect)),
                point: None,
                color: [82, 151, 255, 96],
                thickness: 1.0,
            });
        }
        if self.smart_snap {
            for guide in &self.guides {
                commands.push(SceneOverlayCommand2D {
                    kind: SceneOverlayKind2D::GuideLine,
                    label: guide.label.clone(),
                    entity_id: None,
                    axis: Some(guide.axis),
                    value: Some(guide.value),
                    rect: None,
                    point: None,
                    color: [255, 208, 88, 144],
                    thickness: 1.0,
                });
            }
        }
        for entity in entities.iter().filter(|entity| {
            entity.enabled
                && entity.visible
                && (self.selected_ids.contains(&entity.id) || self.show_colliders)
        }) {
            if self.show_colliders {
                let half_w = (entity.width * entity.scale_x).abs() * 0.5;
                let half_h = (entity.height * entity.scale_y).abs() * 0.5;
                commands.push(SceneOverlayCommand2D {
                    kind: SceneOverlayKind2D::ColliderOutline,
                    label: entity.name.clone(),
                    entity_id: Some(entity.id),
                    axis: None,
                    value: None,
                    rect: Some((
                        entity.x - half_w,
                        entity.y - half_h,
                        half_w * 2.0,
                        half_h * 2.0,
                    )),
                    point: None,
                    color: [52, 211, 153, 180],
                    thickness: 1.0,
                });
            }
            if self.show_pivots && self.selected_ids.contains(&entity.id) {
                commands.push(SceneOverlayCommand2D {
                    kind: SceneOverlayKind2D::Pivot,
                    label: entity.name.clone(),
                    entity_id: Some(entity.id),
                    axis: None,
                    value: None,
                    rect: None,
                    point: Some((entity.x, entity.y)),
                    color: [255, 255, 255, 220],
                    thickness: 1.0,
                });
            }
            if self.show_names && self.selected_ids.contains(&entity.id) {
                commands.push(SceneOverlayCommand2D {
                    kind: SceneOverlayKind2D::Label,
                    label: entity.name.clone(),
                    entity_id: Some(entity.id),
                    axis: None,
                    value: None,
                    rect: None,
                    point: Some((entity.x, entity.y)),
                    color: [229, 231, 235, 255],
                    thickness: 1.0,
                });
            }
        }
        commands
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

    fn snap_to_guides(&self, point: &mut (f64, f64)) -> SceneSnapTarget2D {
        let tolerance = f64::from(self.smart_snap_tolerance.max(0.0));
        if tolerance <= f64::EPSILON {
            return SceneSnapTarget2D::None;
        }
        let mut snapped_x = false;
        let mut snapped_y = false;
        let mut best_x = tolerance;
        let mut best_y = tolerance;

        for guide in &self.guides {
            match guide.axis {
                SceneGuideAxis2D::X => {
                    let distance = (point.0 - guide.value).abs();
                    if distance <= best_x {
                        point.0 = guide.value;
                        best_x = distance;
                        snapped_x = true;
                    }
                }
                SceneGuideAxis2D::Y => {
                    let distance = (point.1 - guide.value).abs();
                    if distance <= best_y {
                        point.1 = guide.value;
                        best_y = distance;
                        snapped_y = true;
                    }
                }
            }
        }

        match (snapped_x, snapped_y) {
            (true, true) => SceneSnapTarget2D::GuideBoth,
            (true, false) => SceneSnapTarget2D::GuideX,
            (false, true) => SceneSnapTarget2D::GuideY,
            (false, false) => SceneSnapTarget2D::None,
        }
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

fn default_smart_snap_tolerance() -> f32 {
    6.0
}
