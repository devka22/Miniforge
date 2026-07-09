use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use crate::engine::audio_mixer::AudioMixer;
use crate::entities::game_object::GameObject;
use kira::sound::static_sound::StaticSoundData;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Easing, Tween};

#[derive(Debug, Clone)]
pub struct AudioVoice {
    pub entity_id: u64,
    pub audio_name: String,
    pub bus: String,
    pub volume: f64,
    pub looped: bool,
    pub playing: bool,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioCommandKind {
    Music,
    Sfx,
    Volume,
    Fade,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioCommand {
    pub kind: AudioCommandKind,
    pub name: String,
    pub bus: String,
    pub volume: f64,
    pub decibels: f32,
    pub fade_seconds: f64,
    pub looped: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AudioSystem {
    pub voices: BTreeMap<u64, AudioVoice>,
    pub stats: BTreeMap<String, usize>,
    pub command_log: Vec<AudioCommand>,
    pub music: Option<String>,
    pub bus_volumes: BTreeMap<String, f64>,
    pub last_tween: Option<Tween>,
    started_sources: BTreeSet<u64>,
}

impl AudioSystem {
    pub fn update_entities(&mut self, entities: &mut [GameObject], mixer: &AudioMixer, mode: &str) {
        if mode != "PLAY" {
            self.stats.insert("voices".to_string(), self.voices.len());
            return;
        }

        let mut live_voice_ids = BTreeSet::new();
        for entity in entities {
            let entity_id = entity.id;
            let Some(source) = entity.get_component_mut("AudioSource") else {
                continue;
            };
            live_voice_ids.insert(entity_id);
            let Some(audio_name) = source
                .get("audio_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            if source.get_bool("play_on_start", false) && !self.started_sources.contains(&entity_id)
            {
                let bus = source.get_string("bus", "SFX");
                let bus_volume = mixer.buses.get(&bus).map(|bus| bus.volume).unwrap_or(1.0);
                self.voices.insert(
                    entity_id,
                    AudioVoice {
                        entity_id,
                        audio_name: audio_name.clone(),
                        bus,
                        volume: source.get_f64("volume", 1.0) * mixer.master_volume * bus_volume,
                        looped: source.get_bool("loop", false),
                        playing: true,
                        paused: false,
                    },
                );
                self.started_sources.insert(entity_id);
            }
            if source.get_bool("stop_requested", false) {
                self.voices.remove(&entity_id);
                source.set("stop_requested", serde_json::json!(false));
                continue;
            }
            if let Some(voice) = self.voices.get_mut(&entity_id) {
                let bus_volume = mixer
                    .buses
                    .get(&voice.bus)
                    .map(|bus| bus.volume)
                    .unwrap_or(1.0);
                voice.audio_name = audio_name;
                voice.volume = source.get_f64("volume", 1.0)
                    * mixer.master_volume
                    * bus_volume
                    * self.bus_volumes.get(&voice.bus).copied().unwrap_or(1.0);
                voice.paused = source.get_bool("paused", false);
                voice.playing = !voice.paused;
                voice.looped = source.get_bool("loop", voice.looped);
            }
        }

        self.voices.retain(|entity_id, voice| {
            live_voice_ids.contains(entity_id) && (voice.playing || voice.paused)
        });
        self.started_sources
            .retain(|entity_id| live_voice_ids.contains(entity_id));
        self.stats.insert("voices".to_string(), self.voices.len());
        self.stats
            .insert("audio_commands".to_string(), self.command_log.len());
    }

    pub fn stop(&mut self, entity_id: u64) {
        self.voices.remove(&entity_id);
    }

    pub fn play_music(&mut self, name: &str, volume: f64, fade_seconds: f64) {
        self.music = Some(name.to_string());
        self.queue_command(AudioCommand {
            kind: AudioCommandKind::Music,
            name: name.to_string(),
            bus: "Music".to_string(),
            volume: volume.clamp(0.0, 1.0),
            decibels: Self::volume_to_decibels(volume).0,
            fade_seconds: fade_seconds.max(0.0),
            looped: true,
        });
    }

    pub fn play_sfx(&mut self, name: &str, volume: f64) {
        self.queue_command(AudioCommand {
            kind: AudioCommandKind::Sfx,
            name: name.to_string(),
            bus: "SFX".to_string(),
            volume: volume.clamp(0.0, 1.0),
            decibels: Self::volume_to_decibels(volume).0,
            fade_seconds: 0.0,
            looped: false,
        });
    }

    pub fn set_volume(&mut self, bus: &str, volume: f64) {
        let volume = volume.clamp(0.0, 1.0);
        self.bus_volumes.insert(bus.to_string(), volume);
        self.queue_command(AudioCommand {
            kind: AudioCommandKind::Volume,
            name: bus.to_string(),
            bus: bus.to_string(),
            volume,
            decibels: Self::volume_to_decibels(volume).0,
            fade_seconds: 0.0,
            looped: false,
        });
    }

    pub fn fade(&mut self, bus: &str, target_volume: f64, seconds: f64) -> Tween {
        let volume = target_volume.clamp(0.0, 1.0);
        self.bus_volumes.insert(bus.to_string(), volume);
        let tween = Self::kira_tween(seconds);
        self.last_tween = Some(tween);
        self.queue_command(AudioCommand {
            kind: AudioCommandKind::Fade,
            name: bus.to_string(),
            bus: bus.to_string(),
            volume,
            decibels: Self::volume_to_decibels(volume).0,
            fade_seconds: seconds.max(0.0),
            looped: false,
        });
        tween
    }

    pub fn stop_music(&mut self, fade_seconds: f64) {
        let name = self.music.take().unwrap_or_else(|| "Music".to_string());
        self.queue_command(AudioCommand {
            kind: AudioCommandKind::Stop,
            name,
            bus: "Music".to_string(),
            volume: 0.0,
            decibels: Decibels::SILENCE.0,
            fade_seconds: fade_seconds.max(0.0),
            looped: false,
        });
    }

    pub fn clear_finished_commands(&mut self) {
        self.command_log.clear();
    }

    pub fn try_create_kira_manager() -> Result<AudioManager<DefaultBackend>, String> {
        AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|error| format!("{error:?}"))
    }

    pub fn play_file_with_kira(
        manager: &mut AudioManager<DefaultBackend>,
        path: impl AsRef<Path>,
        volume: f64,
        fade_seconds: f64,
    ) -> Result<(), String> {
        let mut data = StaticSoundData::from_file(path).map_err(|error| format!("{error:?}"))?;
        data = data.volume(Self::volume_to_decibels(volume));
        if fade_seconds > 0.0 {
            data = data.fade_in_tween(Some(Self::kira_tween(fade_seconds)));
        }
        manager
            .play(data)
            .map(|_| ())
            .map_err(|error| format!("{error:?}"))
    }

    pub fn kira_tween(seconds: f64) -> Tween {
        Tween {
            duration: Duration::from_secs_f64(seconds.max(0.0)),
            easing: Easing::Linear,
            ..Default::default()
        }
    }

    pub fn volume_to_decibels(volume: f64) -> Decibels {
        let volume = volume.clamp(0.0, 1.0) as f32;
        if volume <= 0.0001 {
            Decibels::SILENCE
        } else {
            Decibels(20.0 * volume.log10())
        }
    }

    fn queue_command(&mut self, command: AudioCommand) {
        const MAX_COMMAND_HISTORY: usize = 256;
        if self.command_log.len() >= MAX_COMMAND_HISTORY {
            let overflow = self.command_log.len() + 1 - MAX_COMMAND_HISTORY;
            self.command_log.drain(..overflow);
            *self
                .stats
                .entry("dropped_audio_commands".to_string())
                .or_default() += overflow;
        }
        self.command_log.push(command);
        self.stats
            .insert("audio_commands".to_string(), self.command_log.len());
    }
}
