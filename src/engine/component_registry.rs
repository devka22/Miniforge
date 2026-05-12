use std::collections::BTreeMap;

use crate::engine::component::{
    Component, advanced_component_category, advanced_component_types, default_component,
};

#[derive(Debug, Clone)]
pub struct ComponentRegistry {
    pub categories: BTreeMap<String, String>,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    pub fn new() -> Self {
        let mut categories = BTreeMap::from([
            ("Transform".to_string(), "Core".to_string()),
            ("SpriteRenderer".to_string(), "Rendering".to_string()),
            ("RTSMovement".to_string(), "RTS".to_string()),
            ("Selectable".to_string(), "Editor".to_string()),
            ("MovementComponent".to_string(), "Gameplay".to_string()),
            ("AudioSource".to_string(), "Audio".to_string()),
            ("Rigidbody2D".to_string(), "Physics".to_string()),
            ("Animator".to_string(), "Animation".to_string()),
            ("VisualScript".to_string(), "Scripting".to_string()),
            ("UIElement".to_string(), "UI".to_string()),
            ("Collider2D".to_string(), "Physics".to_string()),
            ("Health".to_string(), "Gameplay".to_string()),
            ("Team".to_string(), "RTS".to_string()),
            ("ResourceNode".to_string(), "RTS".to_string()),
            ("Worker".to_string(), "RTS".to_string()),
        ]);
        for component_type in advanced_component_types() {
            categories.insert(
                component_type.to_string(),
                advanced_component_category(component_type)
                    .unwrap_or("Advanced")
                    .to_string(),
            );
        }
        Self { categories }
    }

    pub fn create(&self, component_type: &str) -> Option<Component> {
        default_component(component_type)
    }

    pub fn names(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }
}
