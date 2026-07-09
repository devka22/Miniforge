use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct Renderer {
    pub draw_calls: usize,
    pub last_visible_entities: usize,
    pub last_frame: FrameRenderStats,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameRenderStats {
    pub submitted_entities: usize,
    pub visible_entities: usize,
    pub culled_entities: usize,
    pub draw_calls: usize,
}

impl Renderer {
    pub fn begin_frame(&mut self) {
        self.draw_calls = 0;
        self.last_visible_entities = 0;
        self.last_frame = FrameRenderStats::default();
    }

    pub fn draw(&mut self) {
        self.draw_calls += 1;
        self.last_frame.draw_calls = self.draw_calls;
    }

    pub fn draw_entities(&mut self, entities: &[GameObject]) {
        let visible = entities
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
            .count();
        self.last_visible_entities = visible;
        self.draw_calls += self.last_visible_entities;
        self.last_frame = FrameRenderStats {
            submitted_entities: entities.len(),
            visible_entities: visible,
            culled_entities: entities.len().saturating_sub(visible),
            draw_calls: self.draw_calls,
        };
    }

    pub fn frame_stats(&self) -> FrameRenderStats {
        self.last_frame
    }

    pub fn visibility_ratio(&self) -> f64 {
        if self.last_frame.submitted_entities == 0 {
            return 1.0;
        }
        self.last_frame.visible_entities as f64 / self.last_frame.submitted_entities as f64
    }
}
