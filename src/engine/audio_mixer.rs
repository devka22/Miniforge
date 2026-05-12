use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioBus {
    pub name: String,
    pub volume: f64,
    pub muted: bool,
    pub solo: bool,
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
            })
            .volume = volume.clamp(0.0, 1.0);
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
