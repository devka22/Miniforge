use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::systems::physics_system::PhysicsSystem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Physics2DSettings {
    pub gravity_x: f64,
    pub gravity_y: f64,
    pub solver_iterations: usize,
    pub debug_draw: bool,
    #[serde(default)]
    pub layers: Vec<PhysicsLayer2D>,
    #[serde(default)]
    pub layer_matrix: Vec<PhysicsLayerPair2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicsLayer2D {
    pub name: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicsLayerPair2D {
    pub first: String,
    pub second: String,
    pub collides: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rigidbody2DSpec {
    pub body_type: String,
    pub mass: f64,
    pub use_gravity: bool,
    pub drag: f64,
    pub freeze_rotation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Collider2DSpec {
    pub shape: String,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub is_trigger: bool,
    pub layer: String,
    pub mask: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Raycast2DQuery {
    pub origin: (f64, f64),
    pub direction: (f64, f64),
    pub max_distance: f64,
    pub include_triggers: bool,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoxCast2DQuery {
    pub origin: (f64, f64),
    pub half_extents: (f64, f64),
    pub direction: (f64, f64),
    pub max_distance: f64,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CircleCast2DQuery {
    pub origin: (f64, f64),
    pub radius: f64,
    pub direction: (f64, f64),
    pub max_distance: f64,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlapArea2DQuery {
    pub center: (f64, f64),
    pub half_extents: (f64, f64),
    #[serde(default)]
    pub layers: Vec<String>,
}

impl Default for Physics2DSettings {
    fn default() -> Self {
        Self {
            gravity_x: 0.0,
            gravity_y: 18.0,
            solver_iterations: 2,
            debug_draw: false,
            layers: vec![
                PhysicsLayer2D {
                    name: "Default".to_string(),
                    index: 0,
                },
                PhysicsLayer2D {
                    name: "Pawn".to_string(),
                    index: 1,
                },
                PhysicsLayer2D {
                    name: "WorldStatic".to_string(),
                    index: 2,
                },
                PhysicsLayer2D {
                    name: "Trigger".to_string(),
                    index: 3,
                },
            ],
            layer_matrix: vec![PhysicsLayerPair2D {
                first: "Pawn".to_string(),
                second: "Trigger".to_string(),
                collides: true,
            }],
        }
    }
}

impl Physics2DSettings {
    pub fn apply_to_system(&self, physics: &mut PhysicsSystem) {
        physics.set_gravity(self.gravity_x, self.gravity_y);
        physics.solver_iterations = self.solver_iterations.max(1);
        for pair in &self.layer_matrix {
            physics.set_layer_collision(&pair.first, &pair.second, pair.collides);
        }
    }

    pub fn to_value(&self) -> Value {
        json!(self)
    }
}

pub fn minimal_physics_config() -> Value {
    json!({
        "settings": Physics2DSettings::default(),
        "rigidbody": {
            "body_type": "dynamic",
            "mass": 1.0,
            "use_gravity": true,
            "drag": 0.05,
            "freeze_rotation": true
        },
        "static_body": {"body_type": "static", "mass": 0.0, "use_gravity": false, "drag": 0.0, "freeze_rotation": true},
        "kinematic_body": {"body_type": "kinematic", "mass": 1.0, "use_gravity": false, "drag": 0.0, "freeze_rotation": true},
        "collider": {
            "shape": "rect",
            "width": 1.0,
            "height": 1.0,
            "radius": 0.5,
            "is_trigger": false,
            "layer": "Pawn",
            "mask": ["WorldStatic", "Trigger"]
        },
        "raycast": {
            "origin": [0.0, 0.0],
            "direction": [1.0, 0.0],
            "max_distance": 8.0,
            "include_triggers": false,
            "layers": ["WorldStatic"]
        },
        "boxcast": {"origin": [0.0, 0.0], "half_extents": [0.5, 0.5], "direction": [1.0, 0.0], "max_distance": 4.0, "layers": ["WorldStatic"]},
        "circlecast": {"origin": [0.0, 0.0], "radius": 0.5, "direction": [1.0, 0.0], "max_distance": 4.0, "layers": ["WorldStatic"]},
        "overlap_area": {"center": [0.0, 0.0], "half_extents": [1.0, 1.0], "layers": ["Pawn", "Trigger"]},
        "events": ["OnCollisionEnter", "OnCollisionStay", "OnCollisionExit", "OnTriggerEnter", "OnTriggerExit"]
    })
}
