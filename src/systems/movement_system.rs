use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct MovementSystem;

impl MovementSystem {
    pub fn update_entities(&self, entities: &mut [GameObject], dt: f64) {
        for entity in entities {
            entity.update_movement(dt);
            entity.sync_to_components();
        }
    }
}
