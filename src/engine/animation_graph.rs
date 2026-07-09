use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationKeyframe {
    pub time: f64,
    #[serde(default)]
    pub sprite_name: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub rotation: Option<f64>,
    #[serde(default)]
    pub scale_x: Option<f64>,
    #[serde(default)]
    pub scale_y: Option<f64>,
    #[serde(default)]
    pub tint: Option<(u8, u8, u8)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AnimationSample {
    pub time: f64,
    pub sprite_name: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub rotation: Option<f64>,
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    pub tint: Option<(u8, u8, u8)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f64,
    pub tint_from: (u8, u8, u8),
    pub tint_to: (u8, u8, u8),
    #[serde(default)]
    pub keyframes: Vec<AnimationKeyframe>,
    #[serde(default)]
    pub events: Vec<AnimationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationEvent {
    pub time: f64,
    pub name: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationTransition {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub parameter: Option<String>,
    #[serde(default)]
    pub equals: serde_json::Value,
    #[serde(default)]
    pub exit_time: Option<f64>,
    #[serde(default)]
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimatorState {
    pub name: String,
    pub clip: String,
    pub speed: f64,
    pub looped: bool,
    #[serde(default)]
    pub preview_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimatorControllerAsset {
    pub name: String,
    pub states: BTreeMap<String, AnimatorState>,
    pub clips: BTreeMap<String, AnimationClip>,
    #[serde(default)]
    pub transitions: Vec<AnimationTransition>,
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
                keyframes: vec![
                    AnimationKeyframe {
                        time: 0.0,
                        sprite_name: None,
                        x: None,
                        y: None,
                        rotation: Some(0.0),
                        scale_x: Some(1.0),
                        scale_y: Some(1.0),
                        tint: Some((255, 255, 255)),
                    },
                    AnimationKeyframe {
                        time: 1.0,
                        sprite_name: None,
                        x: None,
                        y: None,
                        rotation: Some(0.0),
                        scale_x: Some(1.0),
                        scale_y: Some(1.0),
                        tint: Some((140, 210, 255)),
                    },
                ],
                events: Vec::new(),
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
                preview_time: 0.0,
            },
        );
        let mut controllers = BTreeMap::new();
        controllers.insert(
            "Default".to_string(),
            AnimatorControllerAsset {
                name: "Default".to_string(),
                states,
                clips,
                transitions: Vec::new(),
                default_state: "Idle".to_string(),
            },
        );
        Self { controllers }
    }
}

impl AnimationClip {
    pub fn sample(&self, time: f64) -> AnimationSample {
        let duration = self.duration.max(0.0001);
        let time = time.clamp(0.0, duration);
        if self.keyframes.is_empty() {
            let t = (time / duration).clamp(0.0, 1.0);
            return AnimationSample {
                time,
                tint: Some(lerp_color(self.tint_from, self.tint_to, t)),
                ..Default::default()
            };
        }

        // Find the bracketing keys without cloning and sorting the complete clip
        // for every animated entity on every frame.
        let first = self
            .keyframes
            .iter()
            .min_by(|a, b| a.time.total_cmp(&b.time))
            .expect("checked non-empty");
        if time <= first.time {
            return sample_from_key(first, time);
        }
        let last = self
            .keyframes
            .iter()
            .max_by(|a, b| a.time.total_cmp(&b.time))
            .expect("checked non-empty");
        if time >= last.time {
            return sample_from_key(last, time);
        }

        let mut a = first;
        let mut b = last;
        for key in &self.keyframes {
            if key.time <= time && key.time >= a.time {
                a = key;
            }
            if key.time >= time && key.time <= b.time {
                b = key;
            }
        }
        let span = (b.time - a.time).max(0.0001);
        let t = ((time - a.time) / span).clamp(0.0, 1.0);
        AnimationSample {
            time,
            sprite_name: b.sprite_name.clone().or_else(|| a.sprite_name.clone()),
            x: lerp_opt(a.x, b.x, t),
            y: lerp_opt(a.y, b.y, t),
            rotation: lerp_opt(a.rotation, b.rotation, t),
            scale_x: lerp_opt(a.scale_x, b.scale_x, t),
            scale_y: lerp_opt(a.scale_y, b.scale_y, t),
            tint: match (a.tint, b.tint) {
                (Some(from), Some(to)) => Some(lerp_color(from, to, t)),
                (None, Some(to)) => Some(to),
                (Some(from), None) => Some(from),
                _ => None,
            },
        }
    }
}

impl AnimatorControllerAsset {
    pub fn state(&self, name: &str) -> Option<&AnimatorState> {
        self.states
            .get(name)
            .or_else(|| self.states.get(&self.default_state))
    }

    pub fn clip_for_state(&self, state_name: &str) -> Option<&AnimationClip> {
        let state = self.state(state_name)?;
        self.clips.get(&state.clip)
    }

    pub fn add_transition(&mut self, transition: AnimationTransition) {
        self.transitions.push(transition);
    }

    pub fn resolve_transition(
        &self,
        current_state: &str,
        normalized_time: f64,
        parameters: &serde_json::Value,
    ) -> Option<&AnimationTransition> {
        self.transitions.iter().find(|transition| {
            transition.from == current_state
                && transition
                    .exit_time
                    .is_none_or(|exit| normalized_time >= exit.clamp(0.0, 1.0))
                && transition
                    .parameter
                    .as_ref()
                    .is_none_or(|parameter| parameters.get(parameter) == Some(&transition.equals))
        })
    }
}

fn sample_from_key(key: &AnimationKeyframe, time: f64) -> AnimationSample {
    AnimationSample {
        time,
        sprite_name: key.sprite_name.clone(),
        x: key.x,
        y: key.y,
        rotation: key.rotation,
        scale_x: key.scale_x,
        scale_y: key.scale_y,
        tint: key.tint,
    }
}

fn lerp_opt(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + (b - a) * t),
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        _ => None,
    }
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    (lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
}
