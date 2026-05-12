use crate::entities::game_object::GameObject;
use crate::render::renderer::Renderer;

#[derive(Debug, Clone, Default)]
pub struct RenderSystem {
    pub renderer: Renderer,
}

impl RenderSystem {
    pub fn draw(&mut self) {
        self.renderer.draw();
    }

    pub fn draw_entities(&mut self, entities: &[GameObject]) {
        self.renderer.draw_entities(entities);
    }
}
