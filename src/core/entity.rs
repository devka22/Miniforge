use serde_json::Value;

use crate::entities::game_object::GameObject;

pub fn entity_from_json(data: &Value, preserve_id: bool) -> GameObject {
    GameObject::from_data(data, preserve_id)
}
