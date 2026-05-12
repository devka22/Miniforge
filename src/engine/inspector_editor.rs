use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct InspectorEditor;

impl InspectorEditor {
    pub fn set_name(entity: &mut GameObject, name: &str) {
        entity.name = name.to_string();
    }

    pub fn set_position(entity: &mut GameObject, x: f64, y: f64) {
        entity.x = x;
        entity.y = y;
        entity.sync_to_components();
    }
}
