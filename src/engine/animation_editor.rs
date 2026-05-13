use serde::{Deserialize, Serialize};

use crate::engine::animation_graph::{
    AnimationClip, AnimationSample, AnimationTransition, AnimatorControllerAsset,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationTimeline {
    pub clip: String,
    pub cursor: f64,
    pub zoom: f64,
    pub playing: bool,
    pub looped: bool,
}

impl Default for AnimationTimeline {
    fn default() -> Self {
        Self {
            clip: "Idle".to_string(),
            cursor: 0.0,
            zoom: 1.0,
            playing: false,
            looped: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationPreview {
    pub state: String,
    pub clip: String,
    pub sample: AnimationSample,
    pub progress: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationEditor {
    pub controller: AnimatorControllerAsset,
    pub selected_state: String,
    pub timeline: AnimationTimeline,
    pub preview_enabled: bool,
    pub warnings: Vec<String>,
}

impl AnimationEditor {
    pub fn new(controller: AnimatorControllerAsset) -> Self {
        let selected_state = controller.default_state.clone();
        let clip = controller
            .state(&selected_state)
            .map(|state| state.clip.clone())
            .unwrap_or_else(|| "Idle".to_string());
        Self {
            controller,
            selected_state,
            timeline: AnimationTimeline {
                clip,
                ..Default::default()
            },
            preview_enabled: true,
            warnings: Vec::new(),
        }
    }

    pub fn select_state(&mut self, state: &str) -> bool {
        let Some(next) = self.controller.state(state) else {
            self.warnings
                .push(format!("Animator state not found: {state}"));
            return false;
        };
        self.selected_state = next.name.clone();
        self.timeline.clip = next.clip.clone();
        self.timeline.cursor = 0.0;
        true
    }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.timeline.clip = clip.name.clone();
        self.controller.clips.insert(clip.name.clone(), clip);
    }

    pub fn add_transition(&mut self, transition: AnimationTransition) {
        self.controller.add_transition(transition);
    }

    pub fn tick(&mut self, dt: f64) -> AnimationPreview {
        let clip = self
            .controller
            .clips
            .get(&self.timeline.clip)
            .or_else(|| self.controller.clip_for_state(&self.selected_state));
        let Some(clip) = clip else {
            self.warnings
                .push(format!("Animation clip not found: {}", self.timeline.clip));
            return AnimationPreview {
                state: self.selected_state.clone(),
                clip: self.timeline.clip.clone(),
                sample: AnimationSample::default(),
                progress: 0.0,
                warnings: self.warnings.clone(),
            };
        };
        if self.timeline.playing {
            self.timeline.cursor += dt.max(0.0);
            if self.timeline.cursor > clip.duration {
                self.timeline.cursor = if self.timeline.looped {
                    self.timeline.cursor % clip.duration.max(0.0001)
                } else {
                    clip.duration
                };
            }
        }
        self.preview_at(self.timeline.cursor)
    }

    pub fn preview_at(&mut self, time: f64) -> AnimationPreview {
        self.timeline.cursor = time.max(0.0);
        let clip = self
            .controller
            .clips
            .get(&self.timeline.clip)
            .or_else(|| self.controller.clip_for_state(&self.selected_state));
        let Some(clip) = clip else {
            self.warnings
                .push(format!("Animation clip not found: {}", self.timeline.clip));
            return AnimationPreview {
                state: self.selected_state.clone(),
                clip: self.timeline.clip.clone(),
                sample: AnimationSample::default(),
                progress: 0.0,
                warnings: self.warnings.clone(),
            };
        };
        AnimationPreview {
            state: self.selected_state.clone(),
            clip: clip.name.clone(),
            sample: clip.sample(self.timeline.cursor),
            progress: (self.timeline.cursor / clip.duration.max(0.0001)).clamp(0.0, 1.0),
            warnings: self.warnings.clone(),
        }
    }

    pub fn validate(&mut self) -> bool {
        self.warnings.clear();
        if self.controller.states.is_empty() {
            self.warnings
                .push("Animator controller has no states".to_string());
        }
        for state in self.controller.states.values() {
            if !self.controller.clips.contains_key(&state.clip) {
                self.warnings.push(format!(
                    "State {} references missing clip {}",
                    state.name, state.clip
                ));
            }
        }
        for transition in &self.controller.transitions {
            if !self.controller.states.contains_key(&transition.from) {
                self.warnings
                    .push(format!("Transition from missing state {}", transition.from));
            }
            if !self.controller.states.contains_key(&transition.to) {
                self.warnings
                    .push(format!("Transition to missing state {}", transition.to));
            }
        }
        self.warnings.is_empty()
    }
}
