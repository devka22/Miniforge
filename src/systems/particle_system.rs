use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::entities::game_object::GameObject;

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
        let mut live = BTreeMap::new();
        for entity in entities {
            let Some(component) = entity.get_component("ParticleEmitter") else {
                continue;
            };
            if !component.enabled {
                continue;
            }
            let config = config_from_value(&component.serialize());
            let state = self.emitters.entry(entity.id).or_default();
            update_emitter(entity, &config, state, dt);
            live.insert(entity.id, ());
            self.previews.insert(
                entity.id,
                ParticlePreview {
                    entity_id: entity.id,
                    particle_count: state.particles.len(),
                    bounds: particle_bounds(&state.particles),
                },
            );
        }
        self.emitters.retain(|id, _| live.contains_key(id));
        self.previews.retain(|id, _| live.contains_key(id));
        let particles = self
            .emitters
            .values()
            .map(|state| state.particles.len())
            .sum();
        self.stats
            .insert("emitters".to_string(), self.emitters.len());
        self.stats.insert("particles".to_string(), particles);
    }

    pub fn preview(&self, entity_id: u64) -> Option<&ParticlePreview> {
        self.previews.get(&entity_id)
    }

    pub fn burst(&mut self, entity: &GameObject, count: usize) {
        let Some(component) = entity.get_component("ParticleEmitter") else {
            return;
        };
        let mut config = config_from_value(&component.serialize());
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
    for particle in &mut state.particles {
        particle.age += dt;
        particle.x += particle.velocity_x * dt;
        particle.y += particle.velocity_y * dt;
        let t = (particle.age / particle.lifetime.max(0.0001)).clamp(0.0, 1.0);
        particle.size = config.start_size + (config.end_size - config.start_size) * t;
        particle.color[3] = ((config.color[3] as f64) * (1.0 - t)).round() as u8;
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
    for index in 0..count.min(available) {
        let jitter = deterministic_jitter(entity.id, state.particles.len() + index);
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

fn config_from_value(value: &Value) -> ParticleEmitterConfig {
    let color = value
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
        looped: value.get("looped").and_then(Value::as_bool).unwrap_or(true),
        rate: value.get("rate").and_then(Value::as_f64).unwrap_or(16.0),
        burst_count: value
            .get("burst_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        lifetime: value.get("lifetime").and_then(Value::as_f64).unwrap_or(1.0),
        velocity_x: value
            .get("velocity_x")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        velocity_y: value
            .get("velocity_y")
            .and_then(Value::as_f64)
            .unwrap_or(-40.0),
        spread: value.get("spread").and_then(Value::as_f64).unwrap_or(18.0),
        start_size: value
            .get("start_size")
            .and_then(Value::as_f64)
            .unwrap_or(8.0),
        end_size: value.get("end_size").and_then(Value::as_f64).unwrap_or(0.0),
        color,
        max_particles: value
            .get("max_particles")
            .and_then(Value::as_u64)
            .unwrap_or(128) as usize,
    }
}
