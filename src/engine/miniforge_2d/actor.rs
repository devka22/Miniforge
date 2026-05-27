use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorClass2D {
    Actor,
    Pawn,
    PlayerController,
    AIController,
    GameMode,
    Widget,
    Camera,
    Custom(String),
}

impl ActorClass2D {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Actor => "Actor2D",
            Self::Pawn => "Pawn2D",
            Self::PlayerController => "PlayerController2D",
            Self::AIController => "AIController2D",
            Self::GameMode => "GameMode2D",
            Self::Widget => "WidgetActor2D",
            Self::Camera => "CameraActor2D",
            Self::Custom(value) => value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorDescriptor2D {
    pub id: u64,
    pub name: String,
    pub class: ActorClass2D,
    pub enabled: bool,
    pub visible: bool,
    pub transform: Transform2D,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub components: Vec<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Transform2D {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl ActorDescriptor2D {
    pub fn from_entity(entity: &GameObject) -> Self {
        let class = match entity.entity_type.as_str() {
            "Pawn2D" | "Unit" => ActorClass2D::Pawn,
            "PlayerController2D" => ActorClass2D::PlayerController,
            "AIController2D" => ActorClass2D::AIController,
            "GameMode2D" => ActorClass2D::GameMode,
            "CameraActor2D" => ActorClass2D::Camera,
            other if other != "GameObject" => ActorClass2D::Custom(other.to_string()),
            _ => ActorClass2D::Actor,
        };
        Self {
            id: entity.id,
            name: entity.name.clone(),
            class,
            enabled: entity.enabled,
            visible: entity.visible,
            transform: Transform2D {
                x: entity.x,
                y: entity.y,
                rotation: entity.rotation,
                scale_x: entity.scale_x,
                scale_y: entity.scale_y,
            },
            tag: entity.tag.clone(),
            layer: entity.layer.clone(),
            components: entity
                .components
                .iter()
                .map(Component::serialize)
                .collect::<Vec<_>>(),
        }
    }

    pub fn to_entity(&self) -> GameObject {
        let mut entity =
            GameObject::new(self.transform.x, self.transform.y, Some(self.name.clone()));
        entity.id = self.id;
        entity.entity_type = self.class.as_str().to_string();
        entity.enabled = self.enabled;
        entity.active = self.enabled;
        entity.visible = self.visible;
        entity.rotation = self.transform.rotation;
        entity.scale_x = self.transform.scale_x;
        entity.scale_y = self.transform.scale_y;
        entity.tag = if self.tag.is_empty() {
            "Untagged".to_string()
        } else {
            self.tag.clone()
        };
        entity.layer = if self.layer.is_empty() {
            "Default".to_string()
        } else {
            self.layer.clone()
        };
        for component in &self.components {
            if let Some(component) = crate::engine::component::component_from_data(component) {
                entity.add_component(component);
            }
        }
        entity.sync_to_components();
        entity
    }
}

#[derive(Debug, Clone, Default)]
pub struct Actor2DFactory;

impl Actor2DFactory {
    pub fn actor(name: impl Into<String>, x: f64, y: f64) -> GameObject {
        let mut entity = GameObject::new(x, y, Some(name.into()));
        entity.entity_type = "Actor2D".to_string();
        ensure_component(&mut entity, "Actor2D");
        entity
    }

    pub fn pawn(name: impl Into<String>, x: f64, y: f64) -> GameObject {
        let mut entity = GameObject::new_unit(x, y, Some(name.into()));
        entity.entity_type = "Pawn2D".to_string();
        entity.tag = "Player".to_string();
        ensure_component(&mut entity, "Actor2D");
        ensure_component(&mut entity, "Pawn2D");
        ensure_component(&mut entity, "CharacterController2D");
        ensure_component(&mut entity, "Rigidbody2D");
        entity
    }

    pub fn player_controller(name: impl Into<String>) -> GameObject {
        let mut entity = GameObject::new(0.0, 0.0, Some(name.into()));
        entity.entity_type = "PlayerController2D".to_string();
        entity.visible = false;
        ensure_component(&mut entity, "Actor2D");
        ensure_component(&mut entity, "Controller2D");
        ensure_component(&mut entity, "PlayerController2D");
        entity
    }

    pub fn ai_controller(name: impl Into<String>) -> GameObject {
        let mut entity = GameObject::new(0.0, 0.0, Some(name.into()));
        entity.entity_type = "AIController2D".to_string();
        entity.visible = false;
        ensure_component(&mut entity, "Actor2D");
        ensure_component(&mut entity, "Controller2D");
        ensure_component(&mut entity, "AIController2D");
        ensure_component(&mut entity, "Blackboard");
        ensure_component(&mut entity, "BehaviorTree2D");
        entity
    }

    pub fn game_mode(name: impl Into<String>, default_pawn: impl Into<String>) -> GameObject {
        let mut entity = GameObject::new(0.0, 0.0, Some(name.into()));
        entity.entity_type = "GameMode2D".to_string();
        entity.visible = false;
        ensure_component(&mut entity, "Actor2D");
        ensure_component(&mut entity, "GameMode2D");
        if let Some(game_mode) = entity.get_component_mut("GameMode2D") {
            game_mode.set("default_pawn", json!(default_pawn.into()));
        }
        entity
    }
}

pub fn ensure_component(entity: &mut GameObject, component_type: &str) -> bool {
    if entity.get_component(component_type).is_some() {
        return false;
    }
    let component = default_component(component_type).unwrap_or_else(|| Component {
        component_type: component_type.to_string(),
        enabled: true,
        data: Default::default(),
    });
    entity.add_component(component);
    true
}
