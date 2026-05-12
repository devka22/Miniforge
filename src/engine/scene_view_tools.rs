use crate::entities::game_object::GameObject;

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
        for entity in selected {
            match tool {
                "Move" => {
                    entity.x += world_dx;
                    entity.y += world_dy;
                    if self.grid_snapping {
                        entity.x = (entity.x / self.snap_size).round() * self.snap_size;
                        entity.y = (entity.y / self.snap_size).round() * self.snap_size;
                    }
                }
                "Rotate" => entity.rotation += dx * 0.2,
                "Scale" => {
                    let delta = (dx + dy) * 0.01;
                    entity.scale_x = (entity.scale_x + delta).max(0.05);
                    entity.scale_y = (entity.scale_y + delta).max(0.05);
                }
                _ => {}
            }
            entity.sync_to_components();
        }
    }
}
