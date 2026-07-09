use crate::engine::camera::Camera;
use crate::entities::game_object::GameObject;
use crate::render::renderer::{FrameRenderStats, Renderer};
use crate::systems::spatial_index::EntitySpatialIndex;

#[derive(Debug, Clone, Default)]
pub struct RenderSystem {
    pub renderer: Renderer,
}

impl RenderSystem {
    pub fn begin_frame(&mut self) {
        self.renderer.begin_frame();
    }

    pub fn draw(&mut self) {
        self.renderer.draw();
    }

    pub fn draw_entities(&mut self, entities: &[GameObject]) {
        self.renderer.draw_entities(entities);
    }

    pub fn draw_camera(&mut self, entities: &[GameObject], camera: &Camera) -> FrameRenderStats {
        let index = EntitySpatialIndex::from_entities(entities);
        let width = camera.viewport.2.max(0.0) / camera.zoom.max(0.1);
        let height = camera.viewport.3.max(0.0) / camera.zoom.max(0.1);
        let mut visible =
            index.query_aabb([camera.x, camera.y], [camera.x + width, camera.y + height]);
        visible.sort_unstable();
        visible.retain(|&index| entities[index].visible);
        for _ in &visible {
            self.renderer.draw();
        }
        self.renderer.last_visible_entities = visible.len();
        self.renderer.last_frame = FrameRenderStats {
            submitted_entities: entities.len(),
            visible_entities: visible.len(),
            culled_entities: entities.len().saturating_sub(visible.len()),
            draw_calls: self.renderer.draw_calls,
        };
        self.renderer.frame_stats()
    }

    pub fn frame_stats(&self) -> FrameRenderStats {
        self.renderer.frame_stats()
    }
}
