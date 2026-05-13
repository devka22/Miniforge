use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioBus {
    pub name: String,
    pub volume: f64,
    pub muted: bool,
    pub solo: bool,
    #[serde(default = "default_volume")]
    pub target_volume: f64,
    #[serde(default)]
    pub fade_seconds: f64,
    #[serde(default)]
    pub preview_cue: Option<String>,
}

fn default_volume() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioMixer {
    pub master_volume: f64,
    pub buses: BTreeMap<String, AudioBus>,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMixer {
    pub fn new() -> Self {
        let mut buses = BTreeMap::new();
        for name in ["Master", "SFX", "Music", "UI"] {
            buses.insert(
                name.to_string(),
                AudioBus {
                    name: name.to_string(),
                    volume: 1.0,
                    muted: false,
                    solo: false,
                    target_volume: 1.0,
                    fade_seconds: 0.0,
                    preview_cue: None,
                },
            );
        }
        Self {
            master_volume: 1.0,
            buses,
        }
    }

    pub fn set_bus_volume(&mut self, bus: &str, volume: f64) {
        self.buses
            .entry(bus.to_string())
            .or_insert(AudioBus {
                name: bus.to_string(),
                volume: 1.0,
                muted: false,
                solo: false,
                target_volume: 1.0,
                fade_seconds: 0.0,
                preview_cue: None,
            })
            .volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_bus_fade(&mut self, bus: &str, target_volume: f64, seconds: f64) {
        let entry = self.buses.entry(bus.to_string()).or_insert(AudioBus {
            name: bus.to_string(),
            volume: 1.0,
            muted: false,
            solo: false,
            target_volume: 1.0,
            fade_seconds: 0.0,
            preview_cue: None,
        });
        entry.target_volume = target_volume.clamp(0.0, 1.0);
        entry.fade_seconds = seconds.max(0.0);
    }

    pub fn tick_fades(&mut self, dt: f64) {
        for bus in self.buses.values_mut() {
            if bus.fade_seconds <= 0.0 {
                bus.volume = bus.target_volume.clamp(0.0, 1.0);
                continue;
            }
            let step = (dt.max(0.0) / bus.fade_seconds).clamp(0.0, 1.0);
            bus.volume += (bus.target_volume - bus.volume) * step;
            if (bus.volume - bus.target_volume).abs() < 0.001 {
                bus.volume = bus.target_volume;
                bus.fade_seconds = 0.0;
            }
        }
    }

    pub fn slider_rows(&self) -> Vec<(String, f64, bool)> {
        self.buses
            .values()
            .map(|bus| (bus.name.clone(), bus.volume, bus.muted))
            .collect()
    }

    pub fn set_preview_cue(&mut self, bus: &str, cue: impl Into<String>) {
        self.buses
            .entry(bus.to_string())
            .or_insert(AudioBus {
                name: bus.to_string(),
                volume: 1.0,
                muted: false,
                solo: false,
                target_volume: 1.0,
                fade_seconds: 0.0,
                preview_cue: None,
            })
            .preview_cue = Some(cue.into());
    }

    pub fn preview_cue(&self, bus: &str) -> Option<&str> {
        self.buses.get(bus)?.preview_cue.as_deref()
    }

    pub fn serialize(&self) -> Value {
        json!(self)
    }

    pub fn deserialize(&mut self, data: &Value) {
        if let Ok(next) = serde_json::from_value::<AudioMixer>(data.clone()) {
            *self = next;
        }
    }
}
