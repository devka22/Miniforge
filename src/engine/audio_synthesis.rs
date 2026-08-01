//! Small deterministic sound synthesizer for prototype and built-in 2D audio.
//!
//! Games can request ready-to-use WAV bytes without shipping a bespoke audio
//! generator. Imported recordings remain the preferred production path, while
//! these presets make common feedback sounds available immediately.

use std::f32::consts::TAU;

pub const DEFAULT_SAMPLE_RATE: u32 = 22_050;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform2D {
    Sine,
    Triangle,
    Square,
    Noise,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthVoice2D {
    pub waveform: Waveform2D,
    pub start_frequency_hz: f32,
    pub end_frequency_hz: f32,
    pub volume: f32,
    pub attack_seconds: f32,
    pub release_seconds: f32,
    /// `0` is raw noise; values near `1` produce smoother wind-like noise.
    pub noise_smoothing: f32,
    pub seed: u32,
}

impl SynthVoice2D {
    pub const fn tone(waveform: Waveform2D, frequency_hz: f32, volume: f32) -> Self {
        Self {
            waveform,
            start_frequency_hz: frequency_hz,
            end_frequency_hz: frequency_hz,
            volume,
            attack_seconds: 0.004,
            release_seconds: 0.04,
            noise_smoothing: 0.0,
            seed: 0xA5A5_2D2D,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinSound2D {
    Footstep,
    Hit,
    Pickup,
    Interact,
    Door,
    WindLoop,
    RainLoop,
}

pub fn synthesize_builtin_wav(sound: BuiltinSound2D) -> Vec<u8> {
    let (duration, voices) = builtin_recipe(sound);
    synthesize_wav(duration, DEFAULT_SAMPLE_RATE, &voices)
}

pub fn synthesize_wav(duration_seconds: f32, sample_rate: u32, voices: &[SynthVoice2D]) -> Vec<u8> {
    let sample_rate = sample_rate.clamp(8_000, 96_000);
    let duration_seconds = duration_seconds.clamp(0.01, 30.0);
    let sample_count = (duration_seconds * sample_rate as f32).round() as usize;
    let mut phases = vec![0.0_f32; voices.len()];
    let mut random_states = voices.iter().map(|voice| voice.seed).collect::<Vec<_>>();
    let mut smoothed_noise = vec![0.0_f32; voices.len()];
    let mut pcm = Vec::with_capacity(sample_count * 2);

    for sample_index in 0..sample_count {
        let time = sample_index as f32 / sample_rate as f32;
        let progress = sample_index as f32 / sample_count.max(1) as f32;
        let mut mixed = 0.0;
        for (voice_index, voice) in voices.iter().enumerate() {
            let frequency = voice.start_frequency_hz
                + (voice.end_frequency_hz - voice.start_frequency_hz) * progress;
            phases[voice_index] =
                (phases[voice_index] + frequency.max(0.0) / sample_rate as f32).fract();
            let raw = match voice.waveform {
                Waveform2D::Sine => (phases[voice_index] * TAU).sin(),
                Waveform2D::Triangle => 1.0 - 4.0 * (phases[voice_index] - 0.5).abs(),
                Waveform2D::Square => {
                    if phases[voice_index] < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Waveform2D::Noise => {
                    let state = &mut random_states[voice_index];
                    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (*state as f32 / u32::MAX as f32) * 2.0 - 1.0
                }
            };
            let smoothing = voice.noise_smoothing.clamp(0.0, 0.999);
            smoothed_noise[voice_index] =
                smoothed_noise[voice_index] * smoothing + raw * (1.0 - smoothing);
            let signal = if voice.waveform == Waveform2D::Noise {
                smoothed_noise[voice_index]
            } else {
                raw
            };
            let attack = if voice.attack_seconds <= 0.0 {
                1.0
            } else {
                (time / voice.attack_seconds).clamp(0.0, 1.0)
            };
            let remaining = duration_seconds - time;
            let release = if voice.release_seconds <= 0.0 {
                1.0
            } else {
                (remaining / voice.release_seconds).clamp(0.0, 1.0)
            };
            mixed += signal * voice.volume.clamp(0.0, 1.0) * attack * release;
        }
        let compressed = mixed / (1.0 + mixed.abs());
        let sample = (compressed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    encode_mono_pcm16_wav(sample_rate, &pcm)
}

fn builtin_recipe(sound: BuiltinSound2D) -> (f32, Vec<SynthVoice2D>) {
    let mut noise = SynthVoice2D::tone(Waveform2D::Noise, 0.0, 0.35);
    noise.seed = 0x51A7_200D;
    match sound {
        BuiltinSound2D::Footstep => {
            noise.release_seconds = 0.09;
            let mut body = SynthVoice2D::tone(Waveform2D::Triangle, 92.0, 0.52);
            body.end_frequency_hz = 54.0;
            body.release_seconds = 0.11;
            (0.14, vec![noise, body])
        }
        BuiltinSound2D::Hit => {
            noise.volume = 0.62;
            noise.release_seconds = 0.13;
            let mut body = SynthVoice2D::tone(Waveform2D::Sine, 105.0, 0.7);
            body.end_frequency_hz = 42.0;
            body.release_seconds = 0.16;
            (0.18, vec![noise, body])
        }
        BuiltinSound2D::Pickup => {
            let mut low = SynthVoice2D::tone(Waveform2D::Sine, 560.0, 0.42);
            low.end_frequency_hz = 920.0;
            low.release_seconds = 0.08;
            let mut high = SynthVoice2D::tone(Waveform2D::Triangle, 840.0, 0.28);
            high.end_frequency_hz = 1_380.0;
            high.attack_seconds = 0.045;
            high.release_seconds = 0.06;
            (0.2, vec![low, high])
        }
        BuiltinSound2D::Interact => {
            let mut tone = SynthVoice2D::tone(Waveform2D::Triangle, 310.0, 0.42);
            tone.end_frequency_hz = 390.0;
            tone.release_seconds = 0.07;
            (0.11, vec![tone])
        }
        BuiltinSound2D::Door => {
            noise.volume = 0.28;
            noise.noise_smoothing = 0.82;
            noise.release_seconds = 0.24;
            let mut creak = SynthVoice2D::tone(Waveform2D::Triangle, 128.0, 0.5);
            creak.end_frequency_hz = 61.0;
            creak.release_seconds = 0.22;
            (0.32, vec![noise, creak])
        }
        BuiltinSound2D::WindLoop => {
            noise.volume = 0.36;
            noise.noise_smoothing = 0.985;
            noise.attack_seconds = 0.35;
            noise.release_seconds = 0.35;
            let mut gust = SynthVoice2D::tone(Waveform2D::Sine, 0.42, 0.12);
            gust.attack_seconds = 0.4;
            gust.release_seconds = 0.4;
            (4.0, vec![noise, gust])
        }
        BuiltinSound2D::RainLoop => {
            noise.volume = 0.42;
            noise.noise_smoothing = 0.72;
            noise.attack_seconds = 0.3;
            noise.release_seconds = 0.3;
            (4.0, vec![noise])
        }
    }
}

fn encode_mono_pcm16_wav(sample_rate: u32, pcm_bytes: &[u8]) -> Vec<u8> {
    let data_size = pcm_bytes.len() as u32;
    let mut wav = Vec::with_capacity(44 + pcm_bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm_bytes);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_valid_non_silent_pcm_wav_files() {
        for sound in [
            BuiltinSound2D::Footstep,
            BuiltinSound2D::Hit,
            BuiltinSound2D::Pickup,
            BuiltinSound2D::Interact,
            BuiltinSound2D::Door,
            BuiltinSound2D::WindLoop,
            BuiltinSound2D::RainLoop,
        ] {
            let bytes = synthesize_builtin_wav(sound);
            assert_eq!(&bytes[0..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");
            assert!(bytes.len() > 44);
            assert!(bytes[44..].iter().any(|byte| *byte != 0));
        }
    }
}
