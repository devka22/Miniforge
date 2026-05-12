use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct Renderer {
    pub draw_calls: usize,
    pub last_visible_entities: usize,
}

impl Renderer {
    pub fn draw(&mut self) {
        self.draw_calls += 1;
    }

    pub fn draw_entities(&mut self, entities: &[GameObject]) {
        self.last_visible_entities = entities
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
            .count();
        self.draw_calls += self.last_visible_entities;
    }
}
