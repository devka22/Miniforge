use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::component::{
    Component, advanced_component_category, advanced_component_types, default_component,
};

#[derive(Debug, Clone)]
pub struct ComponentRegistry {
    pub categories: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentClassDescriptor {
    pub name: String,
    pub category: String,
    pub creatable: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentSubMenu {
    pub category: String,
    pub component_types: Vec<String>,
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

    pub fn category_for(&self, component_type: &str) -> Option<&str> {
        self.categories.get(component_type).map(String::as_str)
    }

    pub fn descriptor(&self, component_type: &str) -> Option<ComponentClassDescriptor> {
        let category = self.category_for(component_type)?.to_string();
        let component = default_component(component_type);
        Some(ComponentClassDescriptor {
            name: component_type.to_string(),
            category,
            creatable: component.is_some(),
            properties: component
                .map(|component| component.data.into_iter().collect())
                .unwrap_or_default(),
        })
    }

    pub fn descriptors(&self) -> Vec<ComponentClassDescriptor> {
        self.names()
            .into_iter()
            .filter_map(|name| self.descriptor(&name))
            .collect()
    }

    pub fn submenu_model(&self) -> Vec<ComponentSubMenu> {
        let mut by_category = BTreeMap::<String, Vec<String>>::new();
        for (component_type, category) in &self.categories {
            by_category
                .entry(category.clone())
                .or_default()
                .push(component_type.clone());
        }
        by_category
            .into_iter()
            .map(|(category, mut component_types)| {
                component_types.sort();
                ComponentSubMenu {
                    category,
                    component_types,
                }
            })
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<ComponentClassDescriptor> {
        let query = query.to_lowercase();
        self.descriptors()
            .into_iter()
            .filter(|descriptor| {
                query.is_empty()
                    || descriptor.name.to_lowercase().contains(&query)
                    || descriptor.category.to_lowercase().contains(&query)
            })
            .collect()
    }
}
