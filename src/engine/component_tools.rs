use serde_json::Value;

use crate::engine::component::{Component, component_from_data};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct ComponentTools {
    pub clipboard: Option<Value>,
}

impl ComponentTools {
    pub fn copy(&mut self, entity: &GameObject, component_type: &str) -> bool {
        self.clipboard = entity
            .get_component(component_type)
            .map(Component::serialize);
        self.clipboard.is_some()
    }

    pub fn paste(&self, entity: &mut GameObject) -> bool {
        let Some(data) = &self.clipboard else {
            return false;
        };
        let Some(component) = component_from_data(data) else {
            return false;
        };
        entity.add_component(component);
        true
    }

    pub fn reset(entity: &mut GameObject, component_type: &str) -> bool {
        let Some(component) = crate::engine::component::default_component(component_type) else {
            return false;
        };
        entity.remove_component(component_type);
        entity.add_component(component);
        true
    }
}
