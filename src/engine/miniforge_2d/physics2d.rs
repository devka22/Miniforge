use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::miniforge_2d::authoring_catalog::PhysicsWorldProfile2D;
use crate::engine::miniforge_2d::validation::ValidationReport2D;
use crate::systems::physics_system::PhysicsSystem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Physics2DSettings {
    pub gravity_x: f64,
    pub gravity_y: f64,
    pub solver_iterations: usize,
    #[serde(default = "default_fixed_hz")]
    pub fixed_hz: u32,
    #[serde(default = "default_max_substeps")]
    pub max_substeps: usize,
    #[serde(default = "default_continuous_collision")]
    pub continuous_collision: bool,
    #[serde(default = "default_sleeping")]
    pub sleeping: bool,
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
pub struct PhysicsRuntimeTuning2D {
    pub fixed_delta_seconds: f64,
    pub max_substeps: usize,
    pub solver_iterations: usize,
    pub continuous_collision: bool,
    pub sleeping: bool,
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
            fixed_hz: default_fixed_hz(),
            max_substeps: default_max_substeps(),
            continuous_collision: default_continuous_collision(),
            sleeping: default_sleeping(),
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
    pub fn from_world_profile(profile: &PhysicsWorldProfile2D) -> Self {
        let mut settings = Self::default();
        settings.apply_world_profile(profile);
        settings
    }

    pub fn apply_world_profile(&mut self, profile: &PhysicsWorldProfile2D) {
        self.gravity_x = profile.gravity[0];
        self.gravity_y = profile.gravity[1];
        self.solver_iterations = profile.solver_iterations;
        self.fixed_hz = profile.fixed_hz;
        self.max_substeps = profile.max_substeps;
        self.continuous_collision = profile.continuous_collision;
        self.sleeping = profile.sleeping;
        self.normalize();
    }

    pub fn runtime_tuning(&self) -> PhysicsRuntimeTuning2D {
        let mut settings = self.clone();
        settings.normalize();
        PhysicsRuntimeTuning2D {
            fixed_delta_seconds: 1.0 / f64::from(settings.fixed_hz),
            max_substeps: settings.max_substeps,
            solver_iterations: settings.solver_iterations,
            continuous_collision: settings.continuous_collision,
            sleeping: settings.sleeping,
        }
    }

    pub fn normalize(&mut self) {
        if !self.gravity_x.is_finite() {
            self.gravity_x = 0.0;
        }
        if !self.gravity_y.is_finite() {
            self.gravity_y = 18.0;
        }
        self.solver_iterations = self.solver_iterations.clamp(1, 64);
        self.fixed_hz = self.fixed_hz.clamp(15, 480);
        self.max_substeps = self.max_substeps.clamp(1, 16);

        let mut names = std::collections::BTreeSet::new();
        let mut indexes = std::collections::BTreeSet::new();
        self.layers.retain(|layer| {
            !layer.name.trim().is_empty()
                && names.insert(layer.name.clone())
                && indexes.insert(layer.index)
        });
        if self.layers.is_empty() {
            self.layers = Self::default().layers;
        }
        let known = self
            .layers
            .iter()
            .map(|layer| layer.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        self.layer_matrix.retain(|pair| {
            known.contains(pair.first.as_str()) && known.contains(pair.second.as_str())
        });
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if !self.gravity_x.is_finite() || !self.gravity_y.is_finite() {
            report.error(
                "physics_non_finite_gravity",
                "physics2d.gravity",
                "Physics gravity must contain finite numbers.",
            );
        } else if self.gravity_x.hypot(self.gravity_y) > 1_000.0 {
            report.warning(
                "physics_extreme_gravity",
                "physics2d.gravity",
                "Gravity exceeds 1000 units/s²; reduce it or increase the fixed-step rate.",
            );
        }
        if self.solver_iterations == 0 {
            report.error(
                "physics_zero_solver_iterations",
                "physics2d.solver_iterations",
                "At least one solver iteration is required.",
            );
        } else if self.solver_iterations > 64 {
            report.warning(
                "physics_expensive_solver",
                "physics2d.solver_iterations",
                "More than 64 solver iterations is rarely useful and can heavily reduce performance.",
            );
        }
        if self.fixed_hz == 0 {
            report.error(
                "physics_zero_fixed_hz",
                "physics2d.fixed_hz",
                "Fixed physics frequency must be greater than zero.",
            );
        } else if !(30..=240).contains(&self.fixed_hz) {
            report.warning(
                "physics_unusual_fixed_hz",
                "physics2d.fixed_hz",
                "Use 30–240 Hz for predictable cross-platform physics.",
            );
        }
        if self.max_substeps == 0 {
            report.error(
                "physics_zero_substeps",
                "physics2d.max_substeps",
                "At least one physics substep is required.",
            );
        } else if self.max_substeps > 16 {
            report.warning(
                "physics_expensive_substeps",
                "physics2d.max_substeps",
                "More than 16 physics substeps can make frame spikes worse.",
            );
        }

        let mut names = std::collections::BTreeSet::new();
        let mut indexes = std::collections::BTreeSet::new();
        for (index, layer) in self.layers.iter().enumerate() {
            if layer.name.trim().is_empty() {
                report.error(
                    "physics_empty_layer_name",
                    format!("physics2d.layers[{index}].name"),
                    "Physics layer names cannot be empty.",
                );
            }
            if !names.insert(layer.name.as_str()) {
                report.error(
                    "physics_duplicate_layer_name",
                    format!("physics2d.layers[{index}].name"),
                    format!("Duplicate physics layer name: {}", layer.name),
                );
            }
            if !indexes.insert(layer.index) {
                report.error(
                    "physics_duplicate_layer_index",
                    format!("physics2d.layers[{index}].index"),
                    format!("Duplicate physics layer index: {}", layer.index),
                );
            }
            if layer.index > 31 {
                report.warning(
                    "physics_large_layer_index",
                    format!("physics2d.layers[{index}].index"),
                    "Layer indexes above 31 may not be portable to every backend.",
                );
            }
        }
        if self.layers.is_empty() {
            report.error(
                "physics_no_layers",
                "physics2d.layers",
                "Define at least one physics layer.",
            );
        }
        for (index, pair) in self.layer_matrix.iter().enumerate() {
            if !names.contains(pair.first.as_str()) || !names.contains(pair.second.as_str()) {
                report.error(
                    "physics_unknown_layer_pair",
                    format!("physics2d.layer_matrix[{index}]"),
                    format!(
                        "Collision pair references an unknown layer: {} ↔ {}",
                        pair.first, pair.second
                    ),
                );
            }
        }
        report
    }

    pub fn apply_to_system(&self, physics: &mut PhysicsSystem) {
        let mut settings = self.clone();
        settings.normalize();
        physics.set_gravity(settings.gravity_x, settings.gravity_y);
        physics.solver_iterations = settings.solver_iterations;
        physics.fixed_delta = 1.0 / f64::from(settings.fixed_hz);
        physics.max_substeps = settings.max_substeps;
        physics.continuous_collision = settings.continuous_collision;
        physics.sleeping_enabled = settings.sleeping;
        for pair in &settings.layer_matrix {
            physics.set_layer_collision(&pair.first, &pair.second, pair.collides);
        }
    }

    pub fn to_value(&self) -> Value {
        json!(self)
    }
}

const fn default_fixed_hz() -> u32 {
    60
}

const fn default_max_substeps() -> usize {
    4
}

const fn default_continuous_collision() -> bool {
    true
}

const fn default_sleeping() -> bool {
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_profile_configures_runtime_and_survives_serialization() {
        let profile = PhysicsWorldProfile2D {
            gravity: [0.0, 32.0],
            solver_iterations: 8,
            fixed_hz: 120,
            max_substeps: 6,
            continuous_collision: true,
            sleeping: false,
        };
        let settings = Physics2DSettings::from_world_profile(&profile);
        let tuning = settings.runtime_tuning();
        assert_eq!(tuning.fixed_delta_seconds, 1.0 / 120.0);
        assert_eq!(tuning.max_substeps, 6);
        assert!(!tuning.sleeping);
        assert!(settings.validate().is_valid());

        let encoded = serde_json::to_value(&settings).unwrap();
        let decoded: Physics2DSettings = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn diagnostics_find_invalid_layers_and_runtime_values() {
        let mut settings = Physics2DSettings {
            fixed_hz: 0,
            max_substeps: 0,
            ..Physics2DSettings::default()
        };
        settings.layers.push(PhysicsLayer2D {
            name: "Pawn".to_string(),
            index: 17,
        });
        settings.layer_matrix.push(PhysicsLayerPair2D {
            first: "Missing".to_string(),
            second: "Pawn".to_string(),
            collides: true,
        });
        let report = settings.validate();
        assert!(!report.is_valid());
        assert!(report.error_count() >= 4);

        settings.normalize();
        assert!(settings.validate().is_valid());
    }
}
