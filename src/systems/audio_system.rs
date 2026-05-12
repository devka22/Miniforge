use std::collections::BTreeMap;

use crate::engine::audio_mixer::AudioMixer;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub struct AudioVoice {
    pub entity_id: u64,
    pub audio_name: String,
    pub bus: String,
    pub volume: f64,
    pub looped: bool,
    pub playing: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AudioSystem {
    pub voices: BTreeMap<u64, AudioVoice>,
    pub stats: BTreeMap<String, usize>,
}

impl AudioSystem {
    pub fn update_entities(&mut self, entities: &mut [GameObject], mixer: &AudioMixer, mode: &str) {
        if mode != "PLAY" {
            self.stats.insert("voices".to_string(), self.voices.len());
            return;
        }

        for entity in entities {
            let entity_id = entity.id;
            let Some(source) = entity.get_component_mut("AudioSource") else {
                continue;
            };
            let Some(audio_name) = source.get("audio_name").and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            if source.get_bool("play_on_start", false) && !source.get_bool("_started", false) {
                let bus = source.get_string("bus", "SFX");
                let bus_volume = mixer.buses.get(&bus).map(|bus| bus.volume).unwrap_or(1.0);
                self.voices.insert(
                    entity_id,
                    AudioVoice {
                        entity_id,
                        audio_name: audio_name.to_string(),
                        bus,
                        volume: source.get_f64("volume", 1.0) * mixer.master_volume * bus_volume,
                        looped: source.get_bool("loop", false),
                        playing: true,
                    },
                );
                source.set("_started", serde_json::json!(true));
            }
        }

        self.voices.retain(|_, voice| voice.playing || voice.looped);
        self.stats.insert("voices".to_string(), self.voices.len());
    }

    pub fn stop(&mut self, entity_id: u64) {
        if let Some(voice) = self.voices.get_mut(&entity_id) {
            voice.playing = false;
        }
    }
}
