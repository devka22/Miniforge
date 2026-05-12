use serde::{Deserialize, Serialize};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    pub scene_name: String,
    pub entities: Vec<GameObject>,
}
