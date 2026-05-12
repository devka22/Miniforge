use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f64,
    pub tint_from: (u8, u8, u8),
    pub tint_to: (u8, u8, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimatorState {
    pub name: String,
    pub clip: String,
    pub speed: f64,
    pub looped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimatorControllerAsset {
    pub name: String,
    pub states: BTreeMap<String, AnimatorState>,
    pub clips: BTreeMap<String, AnimationClip>,
    pub default_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationGraphLibrary {
    pub controllers: BTreeMap<String, AnimatorControllerAsset>,
}

impl Default for AnimationGraphLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationGraphLibrary {
    pub fn new() -> Self {
        let mut clips = BTreeMap::new();
        clips.insert(
            "Idle".to_string(),
            AnimationClip {
                name: "Idle".to_string(),
                duration: 1.0,
                tint_from: (255, 255, 255),
                tint_to: (140, 210, 255),
            },
        );
        let mut states = BTreeMap::new();
        states.insert(
            "Idle".to_string(),
            AnimatorState {
                name: "Idle".to_string(),
                clip: "Idle".to_string(),
                speed: 1.0,
                looped: true,
            },
        );
        let mut controllers = BTreeMap::new();
        controllers.insert(
            "Default".to_string(),
            AnimatorControllerAsset {
                name: "Default".to_string(),
                states,
                clips,
                default_state: "Idle".to_string(),
            },
        );
        Self { controllers }
    }
}
