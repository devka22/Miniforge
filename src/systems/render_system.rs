use crate::engine::camera::Camera;
use crate::entities::game_object::GameObject;
use crate::render::renderer::{FrameRenderStats, Renderer};
use crate::systems::spatial_index::{EntitySpatialIndex, entity_aabb};

#[derive(Debug, Clone, Default)]
pub struct RenderSystem {
    pub renderer: Renderer,
    visible_scratch: Vec<usize>,
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
        let (min, max) = camera_bounds(camera);
        self.visible_scratch.clear();
        self.visible_scratch.reserve(entities.len());
        for (index, entity) in entities.iter().enumerate() {
            if !entity.enabled || !entity.active || !entity.visible {
                continue;
            }
            let (entity_min, entity_max) = entity_aabb(entity);
            if aabb_intersects(entity_min, entity_max, min, max) {
                self.visible_scratch.push(index);
            }
        }
        self.finish_camera_draw(entities.len())
    }

    /// Uses a shared broad phase for multi-camera or multi-system frames.
    pub fn draw_camera_with_index(
        &mut self,
        entities: &[GameObject],
        camera: &Camera,
        index: &EntitySpatialIndex,
    ) -> FrameRenderStats {
        let (min, max) = camera_bounds(camera);
        index.query_aabb_into(min, max, &mut self.visible_scratch);
        self.visible_scratch
            .retain(|&index| entities.get(index).is_some_and(|entity| entity.visible));
        self.visible_scratch.sort_unstable();
        self.finish_camera_draw(entities.len())
    }

    fn finish_camera_draw(&mut self, submitted_entities: usize) -> FrameRenderStats {
        for _ in &self.visible_scratch {
            self.renderer.draw();
        }
        self.renderer.last_visible_entities = self.visible_scratch.len();
        self.renderer.last_frame = FrameRenderStats {
            submitted_entities,
            visible_entities: self.visible_scratch.len(),
            culled_entities: submitted_entities.saturating_sub(self.visible_scratch.len()),
            draw_calls: self.renderer.draw_calls,
        };
        self.renderer.frame_stats()
    }

    pub fn frame_stats(&self) -> FrameRenderStats {
        self.renderer.frame_stats()
    }
}

fn camera_bounds(camera: &Camera) -> ([f64; 2], [f64; 2]) {
    let width = camera.viewport.2.max(0.0) / camera.zoom.max(0.1);
    let height = camera.viewport.3.max(0.0) / camera.zoom.max(0.1);
    ([camera.x, camera.y], [camera.x + width, camera.y + height])
}

fn aabb_intersects(
    first_min: [f64; 2],
    first_max: [f64; 2],
    second_min: [f64; 2],
    second_max: [f64; 2],
) -> bool {
    first_max[0] >= second_min[0]
        && first_min[0] <= second_max[0]
        && first_max[1] >= second_min[1]
        && first_min[1] <= second_max[1]
}

#[cfg(test)]
mod tests {
    use super::RenderSystem;
    use crate::engine::camera::Camera;
    use crate::entities::game_object::GameObject;
    use crate::systems::spatial_index::EntitySpatialIndex;

    #[test]
    fn camera_culling_matches_reused_spatial_index_path() {
        let mut hidden = GameObject::new(4.0, 4.0, Some("Hidden".to_string()));
        hidden.visible = false;
        let entities = vec![
            GameObject::new(2.0, 2.0, Some("Visible".to_string())),
            GameObject::new(200.0, 200.0, Some("Far".to_string())),
            hidden,
        ];
        let mut camera = Camera::default();
        camera.set_viewport((0.0, 0.0, 20.0, 20.0));
        let index = EntitySpatialIndex::from_entities(&entities);
        let mut render = RenderSystem::default();

        render.begin_frame();
        let direct = render.draw_camera(&entities, &camera);
        render.begin_frame();
        let shared = render.draw_camera_with_index(&entities, &camera, &index);

        assert_eq!(direct.visible_entities, 1);
        assert_eq!(direct, shared);
    }
}
