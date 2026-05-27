use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sequencer2D {
    pub name: String,
    pub duration: f64,
    pub frame_rate: f64,
    #[serde(default)]
    pub tracks: Vec<SequencerTrack2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SequencerTrack2D {
    pub id: String,
    pub target: Option<String>,
    pub track_type: String,
    #[serde(default)]
    pub keyframes: Vec<SequencerKeyframe2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SequencerKeyframe2D {
    pub time: f64,
    #[serde(default)]
    pub easing: String,
    #[serde(default)]
    pub value: Value,
}

impl Sequencer2D {
    pub fn validate(&self) -> bool {
        self.duration >= 0.0
            && self.frame_rate > 0.0
            && self
                .tracks
                .iter()
                .flat_map(|track| &track.keyframes)
                .all(|key| key.time >= 0.0 && key.time <= self.duration)
    }

    pub fn sample_events(
        &self,
        previous_time: f64,
        current_time: f64,
    ) -> Vec<&SequencerKeyframe2D> {
        self.tracks
            .iter()
            .filter(|track| matches!(track.track_type.as_str(), "event" | "dialogue"))
            .flat_map(|track| &track.keyframes)
            .filter(|key| key.time > previous_time && key.time <= current_time)
            .collect()
    }

    pub fn last_value(&self, track_id: &str, time: f64) -> Option<&Value> {
        self.tracks
            .iter()
            .find(|track| track.id == track_id)?
            .keyframes
            .iter()
            .filter(|key| key.time <= time)
            .max_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|key| &key.value)
    }
}

pub fn supported_track_types() -> Vec<&'static str> {
    vec![
        "transform",
        "camera",
        "sprite_animation",
        "audio",
        "dialogue",
        "event",
        "fade",
        "screen_shake",
        "ui",
        "spawn",
        "destroy",
    ]
}

pub fn minimal_sequencer() -> Sequencer2D {
    Sequencer2D {
        name: "IntroSequence2D".to_string(),
        duration: 6.0,
        frame_rate: 30.0,
        tracks: vec![
            SequencerTrack2D {
                id: "camera_main".to_string(),
                target: Some("CameraActor2D".to_string()),
                track_type: "camera".to_string(),
                keyframes: vec![
                    SequencerKeyframe2D {
                        time: 0.0,
                        easing: "linear".to_string(),
                        value: json!({"x": 0.0, "y": 0.0, "zoom": 1.0}),
                    },
                    SequencerKeyframe2D {
                        time: 4.0,
                        easing: "smooth".to_string(),
                        value: json!({"x": 8.0, "y": 4.0, "zoom": 1.4}),
                    },
                ],
            },
            SequencerTrack2D {
                id: "dialogue_0".to_string(),
                target: None,
                track_type: "dialogue".to_string(),
                keyframes: vec![SequencerKeyframe2D {
                    time: 1.0,
                    easing: "step".to_string(),
                    value: json!({"speaker": "Guide", "text": "Welcome to MiniForge2D."}),
                }],
            },
            SequencerTrack2D {
                id: "audio_music".to_string(),
                target: None,
                track_type: "audio".to_string(),
                keyframes: vec![SequencerKeyframe2D {
                    time: 0.0,
                    easing: "step".to_string(),
                    value: json!({"sound": "assets/audio/intro.wav", "volume": 0.8}),
                }],
            },
            SequencerTrack2D {
                id: "fade_in".to_string(),
                target: None,
                track_type: "fade".to_string(),
                keyframes: vec![SequencerKeyframe2D {
                    time: 0.0,
                    easing: "linear".to_string(),
                    value: json!({"alpha": 1.0}),
                }],
            },
            SequencerTrack2D {
                id: "spawn_pickup".to_string(),
                target: None,
                track_type: "spawn".to_string(),
                keyframes: vec![SequencerKeyframe2D {
                    time: 3.0,
                    easing: "step".to_string(),
                    value: json!({"prefab": "assets/prefabs/Pickup.prefab", "x": 5.0, "y": 2.0}),
                }],
            },
        ],
    }
}
