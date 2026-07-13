use std::collections::{BTreeMap, HashSet};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::Component;
use crate::entities::game_object::GameObject;

const MAX_PARTICLES_PER_EMITTER: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleEmitterConfig {
    pub looped: bool,
    pub rate: f64,
    pub burst_count: usize,
    pub lifetime: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub spread: f64,
    pub start_size: f64,
    pub end_size: f64,
    pub color: [u8; 4],
    pub max_particles: usize,
}

impl Default for ParticleEmitterConfig {
    fn default() -> Self {
        Self {
            looped: true,
            rate: 16.0,
            burst_count: 0,
            lifetime: 1.0,
            velocity_x: 0.0,
            velocity_y: -40.0,
            spread: 18.0,
            start_size: 8.0,
            end_size: 0.0,
            color: [255, 210, 120, 220],
            max_particles: 128,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub age: f64,
    pub lifetime: f64,
    pub size: f64,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParticleEmitterState {
    pub particles: Vec<Particle>,
    pub emit_accumulator: f64,
    pub burst_emitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParticlePreview {
    pub entity_id: u64,
    pub particle_count: usize,
    pub bounds: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Default)]
pub struct ParticleSystem {
    pub emitters: BTreeMap<u64, ParticleEmitterState>,
    pub previews: BTreeMap<u64, ParticlePreview>,
    pub stats: BTreeMap<String, usize>,
    live_scratch: HashSet<u64>,
}

impl ParticleSystem {
    pub fn update_entities(&mut self, entities: &[GameObject], dt: f64, mode: &str) {
        if mode == "EDITOR" {
            self.update_previews(entities, 1.0 / 30.0);
        } else {
            self.update_previews(entities, dt);
        }
    }

    pub fn update_previews(&mut self, entities: &[GameObject], dt: f64) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        self.live_scratch.clear();
        self.live_scratch.reserve(entities.len());
        for entity in entities {
            let Some(component) = entity.get_component("ParticleEmitter") else {
                continue;
            };
            if !component.enabled {
                continue;
            }
            let config = config_from_component(component);
            let state = self.emitters.entry(entity.id).or_default();
            update_emitter(entity, &config, state, dt);
            self.live_scratch.insert(entity.id);
            self.previews.insert(
                entity.id,
                ParticlePreview {
                    entity_id: entity.id,
                    particle_count: state.particles.len(),
                    bounds: particle_bounds(&state.particles),
                },
            );
        }
        self.emitters.retain(|id, _| self.live_scratch.contains(id));
        self.previews.retain(|id, _| self.live_scratch.contains(id));
        let particles = self
            .emitters
            .values()
            .map(|state| state.particles.len())
            .sum();
        set_stat(&mut self.stats, "emitters", self.emitters.len());
        set_stat(&mut self.stats, "particles", particles);
    }

    pub fn preview(&self, entity_id: u64) -> Option<&ParticlePreview> {
        self.previews.get(&entity_id)
    }

    pub fn burst(&mut self, entity: &GameObject, count: usize) {
        let Some(component) = entity.get_component("ParticleEmitter") else {
            return;
        };
        let mut config = config_from_component(component);
        config.burst_count = count;
        let state = self.emitters.entry(entity.id).or_default();
        emit(entity, &config, state, count);
    }
}

pub fn default_particle_emitter() -> Value {
    json!({
        "looped": true,
        "rate": 16.0,
        "burst_count": 8,
        "lifetime": 1.0,
        "velocity_x": 0.0,
        "velocity_y": -40.0,
        "spread": 18.0,
        "start_size": 8.0,
        "end_size": 0.0,
        "color": [255, 210, 120, 220],
        "max_particles": 128,
        "editor_preview": true,
    })
}

fn update_emitter(
    entity: &GameObject,
    config: &ParticleEmitterConfig,
    state: &mut ParticleEmitterState,
    dt: f64,
) {
    let update_particle = |particle: &mut Particle| {
        particle.age += dt;
        particle.x += particle.velocity_x * dt;
        particle.y += particle.velocity_y * dt;
        let t = (particle.age / particle.lifetime.max(0.0001)).clamp(0.0, 1.0);
        particle.size = config.start_size + (config.end_size - config.start_size) * t;
        particle.color[3] = ((config.color[3] as f64) * (1.0 - t)).round() as u8;
    };
    if state.particles.len() >= 256 {
        state.particles.par_iter_mut().for_each(update_particle);
    } else {
        state.particles.iter_mut().for_each(update_particle);
    }
    state
        .particles
        .retain(|particle| particle.age < particle.lifetime);

    if config.burst_count > 0 && !state.burst_emitted {
        emit(entity, config, state, config.burst_count);
        state.burst_emitted = true;
    }
    if config.looped && config.rate > 0.0 {
        state.emit_accumulator += config.rate * dt.max(0.0);
        let count = state.emit_accumulator.floor() as usize;
        if count > 0 {
            state.emit_accumulator -= count as f64;
            emit(entity, config, state, count);
        }
    }
}

fn emit(
    entity: &GameObject,
    config: &ParticleEmitterConfig,
    state: &mut ParticleEmitterState,
    count: usize,
) {
    let available = config.max_particles.saturating_sub(state.particles.len());
    let emit_count = count.min(available);
    let first_index = state.particles.len();
    state.particles.reserve(emit_count);
    for index in 0..emit_count {
        let jitter = deterministic_jitter(entity.id, first_index + index);
        state.particles.push(Particle {
            x: entity.x,
            y: entity.y,
            velocity_x: config.velocity_x + config.spread * jitter,
            velocity_y: config.velocity_y - config.spread * jitter.abs(),
            age: 0.0,
            lifetime: config.lifetime.max(0.01),
            size: config.start_size.max(0.0),
            color: config.color,
        });
    }
}

fn deterministic_jitter(entity_id: u64, index: usize) -> f64 {
    let seed = entity_id
        .wrapping_mul(6364136223846793005)
        .wrapping_add((index as u64).wrapping_mul(1442695040888963407));
    ((seed % 2000) as f64 / 1000.0) - 1.0
}

fn particle_bounds(particles: &[Particle]) -> Option<(f64, f64, f64, f64)> {
    let first = particles.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for particle in particles {
        min_x = min_x.min(particle.x);
        min_y = min_y.min(particle.y);
        max_x = max_x.max(particle.x);
        max_y = max_y.max(particle.y);
    }
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn config_from_component(component: &Component) -> ParticleEmitterConfig {
    let color = component
        .get("color")
        .and_then(Value::as_array)
        .map(|items| {
            let mut color = [255, 210, 120, 220];
            for (index, item) in items.iter().take(4).enumerate() {
                color[index] = item.as_u64().unwrap_or(color[index] as u64).min(255) as u8;
            }
            color
        })
        .unwrap_or([255, 210, 120, 220]);
    ParticleEmitterConfig {
        looped: component.get_bool("looped", true),
        rate: finite_or(component.get_f64("rate", 16.0), 16.0).clamp(0.0, 1_000_000.0),
        burst_count: component
            .get_usize("burst_count", 0)
            .min(MAX_PARTICLES_PER_EMITTER),
        lifetime: finite_or(component.get_f64("lifetime", 1.0), 1.0).clamp(0.01, 3_600.0),
        velocity_x: finite_or(component.get_f64("velocity_x", 0.0), 0.0),
        velocity_y: finite_or(component.get_f64("velocity_y", -40.0), -40.0),
        spread: finite_or(component.get_f64("spread", 18.0), 18.0).clamp(0.0, 1_000_000.0),
        start_size: finite_or(component.get_f64("start_size", 8.0), 8.0).max(0.0),
        end_size: finite_or(component.get_f64("end_size", 0.0), 0.0).max(0.0),
        color,
        max_particles: component
            .get_usize("max_particles", 128)
            .min(MAX_PARTICLES_PER_EMITTER),
    }
}

fn set_stat(stats: &mut BTreeMap<String, usize>, key: &str, value: usize) {
    if let Some(existing) = stats.get_mut(key) {
        *existing = value;
    } else {
        stats.insert(key.to_string(), value);
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PARTICLES_PER_EMITTER, ParticleSystem, config_from_component};
    use crate::engine::component::default_component;
    use crate::entities::game_object::GameObject;
    use serde_json::json;

    #[test]
    fn emitter_configuration_is_bounded_and_dead_state_is_cleaned() {
        let mut extreme = default_component("ParticleEmitter").expect("emitter");
        extreme.set("rate", json!(2_000_000.0));
        extreme.set("burst_count", json!(u64::MAX));
        extreme.set("max_particles", json!(u64::MAX));
        let config = config_from_component(&extreme);
        assert_eq!(config.rate, 1_000_000.0);
        assert_eq!(config.burst_count, MAX_PARTICLES_PER_EMITTER);
        assert_eq!(config.max_particles, MAX_PARTICLES_PER_EMITTER);

        let mut entity = GameObject::new(0.0, 0.0, Some("Emitter".to_string()));
        entity.add_component(default_component("ParticleEmitter").expect("emitter"));
        let mut system = ParticleSystem::default();
        system.update_previews(&[entity], 0.0);
        assert_eq!(system.emitters.len(), 1);
        system.update_previews(&[], 0.0);
        assert!(system.emitters.is_empty());
        assert!(system.previews.is_empty());
    }
}
