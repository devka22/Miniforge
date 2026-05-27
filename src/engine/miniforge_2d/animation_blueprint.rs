use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::animation_graph::{
    AnimationClip, AnimationGraphLibrary, AnimationTransition, AnimatorControllerAsset,
    AnimatorState,
};
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationBlueprint2D {
    pub name: String,
    pub default_state: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, AnimationParameter2D>,
    #[serde(default)]
    pub states: BTreeMap<String, AnimationState2D>,
    #[serde(default)]
    pub transitions: Vec<AnimationTransition2D>,
    #[serde(default)]
    pub frame_events: Vec<AnimationFrameEvent2D>,
    #[serde(default)]
    pub hitbox_frames: Vec<AnimationBoxFrame2D>,
    #[serde(default)]
    pub hurtbox_frames: Vec<AnimationBoxFrame2D>,
    #[serde(default)]
    pub preview: AnimationPreview2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationParameter2D {
    pub value_type: String,
    #[serde(default)]
    pub default_value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationState2D {
    pub clip: String,
    pub speed: f64,
    pub looped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationTransition2D {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub parameter: Option<String>,
    #[serde(default)]
    pub equals: Value,
    #[serde(default)]
    pub exit_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationFrameEvent2D {
    pub state: String,
    pub frame: usize,
    pub event: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AnimationPreview2D {
    pub enabled: bool,
    pub state: String,
    pub normalized_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationBoxFrame2D {
    pub state: String,
    pub frame: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AnimationBlueprint2D {
    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if !self.states.contains_key(&self.default_state) {
            report.error(
                "missing_default_state",
                "default_state",
                format!("Estado inicial inexistente: {}", self.default_state),
            );
        }
        let state_names = self.states.keys().cloned().collect::<BTreeSet<_>>();
        for transition in &self.transitions {
            if !state_names.contains(&transition.from) {
                report.error(
                    "transition_from_missing",
                    transition.from.clone(),
                    format!("Transition desde estado inexistente: {}", transition.from),
                );
            }
            if !state_names.contains(&transition.to) {
                report.error(
                    "transition_to_missing",
                    transition.to.clone(),
                    format!("Transition hacia estado inexistente: {}", transition.to),
                );
            }
            if let Some(parameter) = &transition.parameter
                && !self.parameters.contains_key(parameter)
            {
                report.error(
                    "transition_parameter_missing",
                    parameter.clone(),
                    format!("Parametro de transicion inexistente: {parameter}"),
                );
            }
        }
        for event in &self.frame_events {
            if !state_names.contains(&event.state) {
                report.warning(
                    "frame_event_state_missing",
                    event.state.clone(),
                    format!(
                        "Evento de frame apunta a estado inexistente: {}",
                        event.state
                    ),
                );
            }
        }
        report
    }

    pub fn to_animation_graph_library(
        &self,
        clips: BTreeMap<String, AnimationClip>,
    ) -> AnimationGraphLibrary {
        let states = self
            .states
            .iter()
            .map(|(name, state)| {
                (
                    name.clone(),
                    AnimatorState {
                        name: name.clone(),
                        clip: state.clip.clone(),
                        speed: state.speed,
                        looped: state.looped,
                        preview_time: 0.0,
                    },
                )
            })
            .collect();
        let transitions = self
            .transitions
            .iter()
            .map(|transition| AnimationTransition {
                from: transition.from.clone(),
                to: transition.to.clone(),
                parameter: transition.parameter.clone(),
                equals: transition.equals.clone(),
                exit_time: transition.exit_time,
                duration: 0.0,
            })
            .collect();
        AnimationGraphLibrary {
            controllers: BTreeMap::from([(
                self.name.clone(),
                AnimatorControllerAsset {
                    name: self.name.clone(),
                    states,
                    clips,
                    transitions,
                    default_state: self.default_state.clone(),
                },
            )]),
        }
    }
}

pub fn minimal_animation_blueprint() -> AnimationBlueprint2D {
    AnimationBlueprint2D {
        name: "ABP_Player2D".to_string(),
        default_state: "Idle".to_string(),
        parameters: BTreeMap::from([
            (
                "Speed".to_string(),
                AnimationParameter2D {
                    value_type: "float".to_string(),
                    default_value: json!(0.0),
                },
            ),
            (
                "Attacking".to_string(),
                AnimationParameter2D {
                    value_type: "bool".to_string(),
                    default_value: json!(false),
                },
            ),
        ]),
        states: [
            "Idle", "Walk", "Run", "Jump", "Fall", "Attack", "Hit", "Death",
        ]
        .iter()
        .map(|state| {
            (
                (*state).to_string(),
                AnimationState2D {
                    clip: (*state).to_string(),
                    speed: 1.0,
                    looped: !matches!(*state, "Attack" | "Hit" | "Death"),
                },
            )
        })
        .collect(),
        transitions: vec![
            AnimationTransition2D {
                from: "Idle".to_string(),
                to: "Run".to_string(),
                parameter: Some("Speed".to_string()),
                equals: json!(1.0),
                exit_time: None,
            },
            AnimationTransition2D {
                from: "Run".to_string(),
                to: "Idle".to_string(),
                parameter: Some("Speed".to_string()),
                equals: json!(0.0),
                exit_time: None,
            },
        ],
        frame_events: vec![AnimationFrameEvent2D {
            state: "Run".to_string(),
            frame: 2,
            event: "footstep".to_string(),
            payload: json!({"sound": "assets/audio/step.wav"}),
        }],
        hitbox_frames: vec![AnimationBoxFrame2D {
            state: "Attack".to_string(),
            frame: 3,
            x: 0.5,
            y: -0.2,
            width: 0.8,
            height: 0.4,
        }],
        hurtbox_frames: vec![AnimationBoxFrame2D {
            state: "Idle".to_string(),
            frame: 0,
            x: -0.35,
            y: -0.5,
            width: 0.7,
            height: 1.0,
        }],
        preview: AnimationPreview2D {
            enabled: true,
            state: "Idle".to_string(),
            normalized_time: 0.0,
        },
    }
}
