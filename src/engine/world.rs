use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct World {
    pub entities: Vec<GameObject>,
}
